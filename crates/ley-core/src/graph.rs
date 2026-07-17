use crate::ingestion::{ArtifactKind, ArtifactRecord};
use crate::{validate_project_id, LeyCoreError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, StreamingIterator};

pub const PROJECT_GRAPH_SCHEMA_VERSION: u32 = 1;
pub const PROJECT_GRAPH_LIMIT_BYTES: u64 = 67_108_864;
const GIT_OUTPUT_LIMIT_BYTES: u64 = 8_388_608;

#[derive(Debug, Clone)]
pub(crate) struct GraphSource {
    pub artifact: ArtifactRecord,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphNodeKind {
    Project,
    File,
    Symbol,
    Dependency,
    ExternalSymbol,
    ExternalModule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphEdgeKind {
    Contains,
    Defines,
    Imports,
    Calls,
    Inherits,
    Implements,
    References,
    DependsOn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FactProvenance {
    Deterministic,
    UserAuthored,
    AgentAuthored,
    Inferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphCitation {
    pub artifact_path: String,
    pub start_line: u64,
    pub start_column: u64,
    pub end_line: u64,
    pub end_column: u64,
    pub content_hash: String,
    pub artifact_snapshot_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphNode {
    pub id: String,
    pub kind: GraphNodeKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<GraphCitation>,
    pub provenance: FactProvenance,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphEdge {
    pub id: String,
    pub kind: GraphEdgeKind,
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<GraphCitation>,
    pub provenance: FactProvenance,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphDiagnostic {
    pub artifact_path: String,
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitChange {
    pub status: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    pub ahead: u64,
    pub behind: u64,
    pub changes: Vec<GitChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectGraph {
    pub schema_version: u32,
    pub project_id: String,
    pub project_name: String,
    pub artifact_snapshot_id: String,
    pub graph_snapshot_id: String,
    pub generated_at_unix_ms: u64,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub diagnostics: Vec<GraphDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphIdentity<'a> {
    schema_version: u32,
    project_id: &'a str,
    project_name: &'a str,
    artifact_snapshot_id: &'a str,
    nodes: &'a [GraphNode],
    edges: &'a [GraphEdge],
    diagnostics: &'a [GraphDiagnostic],
    git: &'a Option<GitState>,
}

#[derive(Debug)]
struct Definition {
    node_id: String,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Debug)]
struct TaggedFact {
    role: String,
    name: String,
    start_byte: usize,
    end_byte: usize,
    citation_node: tree_sitter::Range,
}

pub(crate) fn build_project_graph(
    project_root: &Path,
    project_id: &str,
    project_name: &str,
    artifact_snapshot_id: &str,
    sources: &[GraphSource],
    generated_at_unix_ms: u64,
) -> Result<ProjectGraph, LeyCoreError> {
    validate_project_id(project_id)?;
    let project_node_id = stable_id("prj", &[project_id]);
    let mut nodes = vec![GraphNode {
        id: project_node_id.clone(),
        kind: GraphNodeKind::Project,
        name: project_name.to_owned(),
        path: None,
        language: None,
        symbol_kind: None,
        package_manager: None,
        citation: None,
        provenance: FactProvenance::UserAuthored,
        confidence: 1.0,
    }];
    let mut edges = Vec::new();
    let mut diagnostics = Vec::new();
    let mut external_nodes = BTreeMap::<(GraphNodeKind, String), String>::new();
    let mut dependencies = BTreeMap::<(String, String), String>::new();

    for source in sources {
        let file_id = stable_id("fil", &[project_id, &source.artifact.path]);
        nodes.push(GraphNode {
            id: file_id.clone(),
            kind: GraphNodeKind::File,
            name: file_name(&source.artifact.path),
            path: Some(source.artifact.path.clone()),
            language: source.artifact.language.clone(),
            symbol_kind: None,
            package_manager: None,
            citation: Some(full_file_citation(source, artifact_snapshot_id)),
            provenance: FactProvenance::Deterministic,
            confidence: 1.0,
        });
        edges.push(edge(
            GraphEdgeKind::Contains,
            &project_node_id,
            &file_id,
            None,
            None,
            FactProvenance::Deterministic,
            1.0,
        ));

        if source.artifact.kind == ArtifactKind::Manifest {
            for dependency in extract_dependencies(source, artifact_snapshot_id, &mut diagnostics) {
                let key = (dependency.manager.clone(), dependency.name.clone());
                let dependency_id = dependencies
                    .entry(key)
                    .or_insert_with(|| {
                        stable_id("dep", &[project_id, &dependency.manager, &dependency.name])
                    })
                    .clone();
                if !nodes.iter().any(|node| node.id == dependency_id) {
                    nodes.push(GraphNode {
                        id: dependency_id.clone(),
                        kind: GraphNodeKind::Dependency,
                        name: dependency.name.clone(),
                        path: None,
                        language: None,
                        symbol_kind: None,
                        package_manager: Some(dependency.manager.clone()),
                        citation: Some(dependency.citation.clone()),
                        provenance: FactProvenance::Deterministic,
                        confidence: 1.0,
                    });
                }
                edges.push(edge(
                    GraphEdgeKind::DependsOn,
                    &project_node_id,
                    &dependency_id,
                    dependency.requirement,
                    Some(dependency.citation),
                    FactProvenance::Deterministic,
                    1.0,
                ));
            }
        }

        let Some(language_name) = source.artifact.language.as_deref() else {
            continue;
        };
        let Some((language, tags_query)) = language_spec(language_name, &source.artifact.path)
        else {
            continue;
        };
        let mut parser = Parser::new();
        parser.set_language(&language).map_err(|error| {
            LeyCoreError::InvalidProjectGraph(format!(
                "could not configure parser for {}: {error}",
                source.artifact.path
            ))
        })?;
        let Some(tree) = parser.parse(source.text.as_bytes(), None) else {
            diagnostics.push(GraphDiagnostic {
                artifact_path: source.artifact.path.clone(),
                kind: "parse-failed".to_owned(),
                message: "Tree-sitter did not return a syntax tree".to_owned(),
            });
            continue;
        };
        if tree.root_node().has_error() {
            diagnostics.push(GraphDiagnostic {
                artifact_path: source.artifact.path.clone(),
                kind: "syntax-error".to_owned(),
                message: format!(
                    "parsed with {} syntax error node(s); valid regions were still indexed",
                    count_error_nodes(tree.root_node())
                ),
            });
        }
        let facts = query_tags(
            &language,
            &tags_query,
            tree.root_node(),
            source.text.as_bytes(),
            &source.artifact.path,
        )?;
        let definitions = add_tagged_facts(
            project_id,
            artifact_snapshot_id,
            source,
            &file_id,
            facts,
            &mut nodes,
            &mut edges,
            &mut external_nodes,
        );
        add_import_facts(
            project_id,
            artifact_snapshot_id,
            source,
            &file_id,
            tree.root_node(),
            &definitions,
            &mut nodes,
            &mut edges,
            &mut external_nodes,
        );
        add_inheritance_facts(
            project_id,
            artifact_snapshot_id,
            source,
            &file_id,
            tree.root_node(),
            &definitions,
            &mut nodes,
            &mut edges,
            &mut external_nodes,
        );
    }

    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    nodes.dedup_by(|left, right| left.id == right.id);
    edges.sort_by(|left, right| left.id.cmp(&right.id));
    edges.dedup_by(|left, right| left.id == right.id);
    diagnostics.sort_by(|left, right| {
        (&left.artifact_path, &left.kind, &left.message).cmp(&(
            &right.artifact_path,
            &right.kind,
            &right.message,
        ))
    });
    diagnostics.dedup();
    let allowed_paths = sources
        .iter()
        .map(|source| source.artifact.path.as_str())
        .collect::<BTreeSet<_>>();
    let git = capture_git_state(project_root)?.map(|mut git| {
        git.changes.retain(|change| {
            allowed_paths.contains(change.path.as_str())
                || change
                    .original_path
                    .as_deref()
                    .is_some_and(|path| allowed_paths.contains(path))
        });
        git
    });
    let mut graph = ProjectGraph {
        schema_version: PROJECT_GRAPH_SCHEMA_VERSION,
        project_id: project_id.to_owned(),
        project_name: project_name.to_owned(),
        artifact_snapshot_id: artifact_snapshot_id.to_owned(),
        graph_snapshot_id: String::new(),
        generated_at_unix_ms,
        nodes,
        edges,
        diagnostics,
        git,
    };
    graph.graph_snapshot_id = graph_snapshot_id(&graph);
    validate_project_graph(&graph, project_id)?;
    Ok(graph)
}

fn language_spec(language: &str, path: &str) -> Option<(Language, Cow<'static, str>)> {
    match language {
        "rust" => Some((
            tree_sitter_rust::LANGUAGE.into(),
            Cow::Borrowed(tree_sitter_rust::TAGS_QUERY),
        )),
        "javascript" => Some((
            tree_sitter_javascript::LANGUAGE.into(),
            Cow::Borrowed(tree_sitter_javascript::TAGS_QUERY),
        )),
        "typescript" if path.ends_with(".tsx") => Some((
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            Cow::Owned(format!(
                "{}\n{}",
                tree_sitter_javascript::TAGS_QUERY,
                tree_sitter_typescript::TAGS_QUERY
            )),
        )),
        "typescript" => Some((
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Cow::Owned(format!(
                "{}\n{}",
                tree_sitter_javascript::TAGS_QUERY,
                tree_sitter_typescript::TAGS_QUERY
            )),
        )),
        "python" => Some((
            tree_sitter_python::LANGUAGE.into(),
            Cow::Borrowed(tree_sitter_python::TAGS_QUERY),
        )),
        _ => None,
    }
}

fn query_tags(
    language: &Language,
    tags_query: &str,
    root: Node<'_>,
    source: &[u8],
    path: &str,
) -> Result<Vec<TaggedFact>, LeyCoreError> {
    let query = Query::new(language, tags_query).map_err(|error| {
        LeyCoreError::InvalidProjectGraph(format!("invalid bundled tag query for {path}: {error}"))
    })?;
    let capture_names = query.capture_names();
    let mut cursor = QueryCursor::new();
    cursor.set_match_limit(65_536);
    let mut matches = cursor.matches(&query, root, source);
    let mut facts = Vec::new();
    while let Some(matched) = matches.next() {
        let name_capture = matched
            .captures
            .iter()
            .find(|capture| capture_names[capture.index as usize] == "name");
        let Some(name_capture) = name_capture else {
            continue;
        };
        let Some(role_capture) = matched.captures.iter().find(|capture| {
            let capture_name = capture_names[capture.index as usize];
            capture_name.starts_with("definition.") || capture_name.starts_with("reference.")
        }) else {
            continue;
        };
        let name = name_capture
            .node
            .utf8_text(source)
            .unwrap_or_default()
            .trim()
            .to_owned();
        if name.is_empty() || name.chars().count() > 512 {
            continue;
        }
        let role_node = role_capture.node;
        let mut citation_node = role_node.range();
        if name_capture.node.start_byte() < citation_node.start_byte {
            citation_node.start_byte = name_capture.node.start_byte();
            citation_node.start_point = name_capture.node.start_position();
        }
        if name_capture.node.end_byte() > citation_node.end_byte {
            citation_node.end_byte = name_capture.node.end_byte();
            citation_node.end_point = name_capture.node.end_position();
        }
        facts.push(TaggedFact {
            role: capture_names[role_capture.index as usize].to_owned(),
            name,
            start_byte: name_capture.node.start_byte(),
            end_byte: name_capture.node.end_byte(),
            citation_node,
        });
    }
    if cursor.did_exceed_match_limit() {
        return Err(LeyCoreError::InvalidProjectGraph(format!(
            "tag query exceeded its bounded match limit for {path}"
        )));
    }
    facts.sort_by(|left, right| {
        (left.start_byte, role_priority(&left.role), &left.name).cmp(&(
            right.start_byte,
            role_priority(&right.role),
            &right.name,
        ))
    });
    let mut seen_definition = BTreeSet::new();
    facts.retain(|fact| {
        if !fact.role.starts_with("definition.") {
            return true;
        }
        seen_definition.insert((fact.start_byte, fact.end_byte, fact.name.clone()))
    });
    Ok(facts)
}

fn role_priority(role: &str) -> u8 {
    match role {
        "definition.method" => 0,
        "definition.function" => 1,
        _ if role.starts_with("definition.") => 2,
        _ => 3,
    }
}

#[allow(clippy::too_many_arguments)]
fn add_tagged_facts(
    project_id: &str,
    artifact_snapshot_id: &str,
    source: &GraphSource,
    file_id: &str,
    facts: Vec<TaggedFact>,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
    external_nodes: &mut BTreeMap<(GraphNodeKind, String), String>,
) -> Vec<Definition> {
    let mut definitions = Vec::new();
    let mut ordinals = BTreeMap::<(String, String), usize>::new();
    for fact in facts
        .iter()
        .filter(|fact| fact.role.starts_with("definition."))
    {
        let symbol_kind = fact
            .role
            .strip_prefix("definition.")
            .expect("filtered definition role");
        let ordinal = ordinals
            .entry((symbol_kind.to_owned(), fact.name.clone()))
            .and_modify(|value| *value += 1)
            .or_insert(1);
        let ordinal_text = ordinal.to_string();
        let node_id = stable_id(
            "sym",
            &[
                project_id,
                &source.artifact.path,
                symbol_kind,
                &fact.name,
                &ordinal_text,
            ],
        );
        let citation = citation(source, artifact_snapshot_id, fact.citation_node);
        nodes.push(GraphNode {
            id: node_id.clone(),
            kind: GraphNodeKind::Symbol,
            name: fact.name.clone(),
            path: Some(source.artifact.path.clone()),
            language: source.artifact.language.clone(),
            symbol_kind: Some(symbol_kind.to_owned()),
            package_manager: None,
            citation: Some(citation.clone()),
            provenance: FactProvenance::Deterministic,
            confidence: 1.0,
        });
        edges.push(edge(
            GraphEdgeKind::Defines,
            file_id,
            &node_id,
            Some(symbol_kind.to_owned()),
            Some(citation),
            FactProvenance::Deterministic,
            1.0,
        ));
        definitions.push(Definition {
            node_id,
            start_byte: fact.citation_node.start_byte,
            end_byte: fact.citation_node.end_byte,
        });
    }

    for fact in facts
        .iter()
        .filter(|fact| fact.role.starts_with("reference."))
    {
        let reference_kind = fact
            .role
            .strip_prefix("reference.")
            .expect("filtered reference role");
        let (node_kind, edge_kind) = match reference_kind {
            "call" => (GraphNodeKind::ExternalSymbol, GraphEdgeKind::Calls),
            "implementation" | "class" | "type" => {
                (GraphNodeKind::ExternalSymbol, GraphEdgeKind::References)
            }
            _ => continue,
        };
        let target_id = external_node(project_id, node_kind, &fact.name, nodes, external_nodes);
        let owner = containing_definition(&definitions, fact.start_byte)
            .map(|definition| definition.node_id.as_str())
            .unwrap_or(file_id);
        edges.push(edge(
            edge_kind,
            owner,
            &target_id,
            Some(fact.name.clone()),
            Some(citation(source, artifact_snapshot_id, fact.citation_node)),
            FactProvenance::Deterministic,
            1.0,
        ));
    }
    definitions
}

#[allow(clippy::too_many_arguments)]
fn add_import_facts(
    project_id: &str,
    artifact_snapshot_id: &str,
    source: &GraphSource,
    file_id: &str,
    root: Node<'_>,
    _definitions: &[Definition],
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
    external_nodes: &mut BTreeMap<(GraphNodeKind, String), String>,
) {
    visit_named_nodes(root, &mut |node| {
        let is_import = matches!(
            (source.artifact.language.as_deref(), node.kind()),
            (Some("rust"), "use_declaration")
                | (Some("javascript" | "typescript"), "import_statement")
                | (
                    Some("python"),
                    "import_statement" | "import_from_statement" | "future_import_statement"
                )
        );
        if !is_import {
            return;
        }
        let statement = node
            .utf8_text(source.text.as_bytes())
            .unwrap_or_default()
            .trim();
        for target in import_targets(
            source.artifact.language.as_deref().unwrap_or_default(),
            statement,
        ) {
            let target_id = external_node(
                project_id,
                GraphNodeKind::ExternalModule,
                &target,
                nodes,
                external_nodes,
            );
            edges.push(edge(
                GraphEdgeKind::Imports,
                file_id,
                &target_id,
                Some(target),
                Some(citation(source, artifact_snapshot_id, node.range())),
                FactProvenance::Deterministic,
                1.0,
            ));
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn add_inheritance_facts(
    project_id: &str,
    artifact_snapshot_id: &str,
    source: &GraphSource,
    file_id: &str,
    root: Node<'_>,
    definitions: &[Definition],
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
    external_nodes: &mut BTreeMap<(GraphNodeKind, String), String>,
) {
    visit_named_nodes(root, &mut |node| {
        let edge_kind = match (source.artifact.language.as_deref(), node.kind()) {
            (Some("javascript"), "class_heritage") | (Some("typescript"), "extends_clause") => {
                Some(GraphEdgeKind::Inherits)
            }
            (Some("typescript"), "implements_clause") => Some(GraphEdgeKind::Implements),
            _ => None,
        };
        if let Some(edge_kind) = edge_kind {
            let owner = containing_definition(definitions, node.start_byte())
                .map(|definition| definition.node_id.as_str())
                .unwrap_or(file_id);
            let raw = node.utf8_text(source.text.as_bytes()).unwrap_or_default();
            add_relation_targets(
                project_id,
                artifact_snapshot_id,
                source,
                node,
                owner,
                edge_kind,
                inheritance_targets(raw),
                nodes,
                edges,
                external_nodes,
            );
            return;
        }

        if source.artifact.language.as_deref() == Some("python")
            && node.kind() == "class_definition"
        {
            let Some(superclasses) = node.child_by_field_name("superclasses") else {
                return;
            };
            let owner = containing_definition(definitions, node.start_byte())
                .map(|definition| definition.node_id.as_str())
                .unwrap_or(file_id);
            let raw = superclasses
                .utf8_text(source.text.as_bytes())
                .unwrap_or_default()
                .trim_matches(['(', ')']);
            add_relation_targets(
                project_id,
                artifact_snapshot_id,
                source,
                superclasses,
                owner,
                GraphEdgeKind::Inherits,
                inheritance_targets(raw),
                nodes,
                edges,
                external_nodes,
            );
            return;
        }

        if source.artifact.language.as_deref() == Some("rust") && node.kind() == "impl_item" {
            let (Some(implementor), Some(trait_node)) = (
                node.child_by_field_name("type"),
                node.child_by_field_name("trait"),
            ) else {
                return;
            };
            let implementor_name = implementor
                .utf8_text(source.text.as_bytes())
                .unwrap_or_default()
                .trim();
            let trait_name = trait_node
                .utf8_text(source.text.as_bytes())
                .unwrap_or_default()
                .trim();
            if implementor_name.is_empty() || trait_name.is_empty() {
                return;
            }
            let owner = external_node(
                project_id,
                GraphNodeKind::ExternalSymbol,
                implementor_name,
                nodes,
                external_nodes,
            );
            add_relation_targets(
                project_id,
                artifact_snapshot_id,
                source,
                trait_node,
                &owner,
                GraphEdgeKind::Implements,
                vec![trait_name.to_owned()],
                nodes,
                edges,
                external_nodes,
            );
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn add_relation_targets(
    project_id: &str,
    artifact_snapshot_id: &str,
    source: &GraphSource,
    citation_node: Node<'_>,
    owner: &str,
    edge_kind: GraphEdgeKind,
    targets: Vec<String>,
    nodes: &mut Vec<GraphNode>,
    edges: &mut Vec<GraphEdge>,
    external_nodes: &mut BTreeMap<(GraphNodeKind, String), String>,
) {
    for target in targets {
        let target_id = external_node(
            project_id,
            GraphNodeKind::ExternalSymbol,
            &target,
            nodes,
            external_nodes,
        );
        edges.push(edge(
            edge_kind,
            owner,
            &target_id,
            Some(target),
            Some(citation(
                source,
                artifact_snapshot_id,
                citation_node.range(),
            )),
            FactProvenance::Deterministic,
            1.0,
        ));
    }
}

fn visit_named_nodes(root: Node<'_>, visitor: &mut impl FnMut(Node<'_>)) {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        visitor(node);
        for index in (0..node.named_child_count()).rev() {
            let index = u32::try_from(index).expect("Tree-sitter child indexes fit in u32");
            if let Some(child) = node.named_child(index) {
                pending.push(child);
            }
        }
    }
}

fn import_targets(language: &str, statement: &str) -> Vec<String> {
    let mut targets = Vec::new();
    match language {
        "javascript" | "typescript" => {
            let bytes = statement.as_bytes();
            let mut index = 0;
            while index < bytes.len() {
                if bytes[index] == b'\'' || bytes[index] == b'"' {
                    let quote = bytes[index];
                    let start = index + 1;
                    index = start;
                    while index < bytes.len() && bytes[index] != quote {
                        if bytes[index] == b'\\' {
                            index = index.saturating_add(1);
                        }
                        index = index.saturating_add(1);
                    }
                    if let Some(value) = statement.get(start..index) {
                        if !value.trim().is_empty() {
                            targets.push(value.trim().to_owned());
                        }
                    }
                }
                index = index.saturating_add(1);
            }
            targets.truncate(1);
        }
        "python" => {
            let value = statement
                .strip_prefix("from ")
                .and_then(|rest| rest.split_whitespace().next())
                .or_else(|| statement.strip_prefix("import "))
                .unwrap_or_default();
            targets.extend(
                value
                    .split(',')
                    .filter_map(|part| part.split_whitespace().next())
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned),
            );
        }
        "rust" => {
            let value = statement
                .strip_prefix("use ")
                .unwrap_or(statement)
                .trim_end_matches(';')
                .trim();
            let root = value
                .trim_start_matches("::")
                .split("::")
                .next()
                .unwrap_or_default()
                .trim();
            if !root.is_empty() {
                targets.push(root.to_owned());
            }
        }
        _ => {}
    }
    targets.sort();
    targets.dedup();
    targets
}

fn inheritance_targets(raw: &str) -> Vec<String> {
    let body = raw
        .trim()
        .strip_prefix("extends")
        .or_else(|| raw.trim().strip_prefix("implements"))
        .unwrap_or(raw)
        .trim();
    let mut targets = body
        .split(',')
        .filter_map(|part| {
            let identifier = part
                .trim()
                .split(|character: char| {
                    character.is_whitespace()
                        || matches!(character, '<' | '>' | '(' | ')' | '{' | '}')
                })
                .find(|value| !value.is_empty() && *value != "extends" && *value != "implements")?;
            Some(identifier.trim_end_matches('.').to_owned())
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    targets
}

fn containing_definition(definitions: &[Definition], byte: usize) -> Option<&Definition> {
    definitions
        .iter()
        .filter(|definition| definition.start_byte <= byte && byte <= definition.end_byte)
        .min_by_key(|definition| definition.end_byte - definition.start_byte)
}

fn external_node(
    project_id: &str,
    kind: GraphNodeKind,
    name: &str,
    nodes: &mut Vec<GraphNode>,
    external_nodes: &mut BTreeMap<(GraphNodeKind, String), String>,
) -> String {
    let key = (kind, name.to_owned());
    if let Some(node_id) = external_nodes.get(&key) {
        return node_id.clone();
    }
    let kind_name = match kind {
        GraphNodeKind::ExternalModule => "module",
        _ => "symbol",
    };
    let node_id = stable_id("ext", &[project_id, kind_name, name]);
    nodes.push(GraphNode {
        id: node_id.clone(),
        kind,
        name: name.to_owned(),
        path: None,
        language: None,
        symbol_kind: None,
        package_manager: None,
        citation: None,
        provenance: FactProvenance::Deterministic,
        confidence: 1.0,
    });
    external_nodes.insert(key, node_id.clone());
    node_id
}

fn edge(
    kind: GraphEdgeKind,
    source: &str,
    target: &str,
    label: Option<String>,
    citation: Option<GraphCitation>,
    provenance: FactProvenance,
    confidence: f32,
) -> GraphEdge {
    let kind_name = serde_json::to_string(&kind).expect("edge kind is serializable");
    let citation_key = citation
        .as_ref()
        .map(|value| {
            format!(
                "{}:{}:{}:{}:{}",
                value.artifact_path,
                value.start_line,
                value.start_column,
                value.end_line,
                value.end_column
            )
        })
        .unwrap_or_default();
    let id = stable_id(
        "edg",
        &[
            &kind_name,
            source,
            target,
            label.as_deref().unwrap_or_default(),
            &citation_key,
        ],
    );
    GraphEdge {
        id,
        kind,
        source: source.to_owned(),
        target: target.to_owned(),
        label,
        citation,
        provenance,
        confidence,
    }
}

fn citation(
    source: &GraphSource,
    artifact_snapshot_id: &str,
    range: tree_sitter::Range,
) -> GraphCitation {
    GraphCitation {
        artifact_path: source.artifact.path.clone(),
        start_line: range.start_point.row as u64 + 1,
        start_column: range.start_point.column as u64 + 1,
        end_line: range.end_point.row as u64 + 1,
        end_column: range.end_point.column as u64 + 1,
        content_hash: source.artifact.content_hash.clone(),
        artifact_snapshot_id: artifact_snapshot_id.to_owned(),
    }
}

fn full_file_citation(source: &GraphSource, artifact_snapshot_id: &str) -> GraphCitation {
    let (start_line, start_column, end_line, end_column) = if source.text.is_empty() {
        (0, 0, 0, 0)
    } else {
        let newline_count = source.text.bytes().filter(|byte| *byte == b'\n').count() as u64;
        (
            1,
            1,
            newline_count + 1,
            source
                .text
                .rsplit_once('\n')
                .map_or(source.text.len(), |(_, tail)| tail.len()) as u64
                + 1,
        )
    };
    GraphCitation {
        artifact_path: source.artifact.path.clone(),
        start_line,
        start_column,
        end_line,
        end_column,
        content_hash: source.artifact.content_hash.clone(),
        artifact_snapshot_id: artifact_snapshot_id.to_owned(),
    }
}

#[derive(Debug)]
struct DependencyFact {
    manager: String,
    name: String,
    requirement: Option<String>,
    citation: GraphCitation,
}

fn extract_dependencies(
    source: &GraphSource,
    artifact_snapshot_id: &str,
    diagnostics: &mut Vec<GraphDiagnostic>,
) -> Vec<DependencyFact> {
    let name = source
        .artifact
        .path
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let parsed = match name.as_str() {
        "package.json" => package_json_dependencies(source, artifact_snapshot_id),
        "cargo.toml" => toml_dependencies(source, artifact_snapshot_id, "cargo"),
        "pyproject.toml" => pyproject_dependencies(source, artifact_snapshot_id),
        "requirements.txt" => requirements_dependencies(source, artifact_snapshot_id),
        _ => return Vec::new(),
    };
    match parsed {
        Ok(mut dependencies) => {
            dependencies.sort_by(|left, right| {
                (&left.manager, &left.name, &left.requirement).cmp(&(
                    &right.manager,
                    &right.name,
                    &right.requirement,
                ))
            });
            dependencies.dedup_by(|left, right| {
                left.manager == right.manager
                    && left.name == right.name
                    && left.requirement == right.requirement
            });
            dependencies
        }
        Err(message) => {
            diagnostics.push(GraphDiagnostic {
                artifact_path: source.artifact.path.clone(),
                kind: "manifest-parse-error".to_owned(),
                message,
            });
            Vec::new()
        }
    }
}

fn package_json_dependencies(
    source: &GraphSource,
    snapshot: &str,
) -> Result<Vec<DependencyFact>, String> {
    let value: serde_json::Value =
        serde_json::from_str(&source.text).map_err(|error| error.to_string())?;
    let mut dependencies = Vec::new();
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        let Some(table) = value.get(section).and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (name, requirement) in table {
            dependencies.push(DependencyFact {
                manager: "npm".to_owned(),
                name: name.clone(),
                requirement: requirement.as_str().map(str::to_owned),
                citation: line_citation(source, snapshot, name),
            });
        }
    }
    Ok(dependencies)
}

fn toml_dependencies(
    source: &GraphSource,
    snapshot: &str,
    manager: &str,
) -> Result<Vec<DependencyFact>, String> {
    let value: toml::Value = toml::from_str(&source.text).map_err(|error| error.to_string())?;
    let mut dependencies = Vec::new();
    collect_toml_dependency_tables(&value, "", manager, source, snapshot, &mut dependencies);
    Ok(dependencies)
}

fn collect_toml_dependency_tables(
    value: &toml::Value,
    root_path: &str,
    manager: &str,
    source: &GraphSource,
    snapshot: &str,
    dependencies: &mut Vec<DependencyFact>,
) {
    let mut pending = vec![(value, root_path.to_owned())];
    while let Some((value, path)) = pending.pop() {
        let Some(table) = value.as_table() else {
            continue;
        };
        if path.ends_with("dependencies")
            || path.ends_with("dev-dependencies")
            || path.ends_with("build-dependencies")
        {
            for (name, requirement) in table {
                dependencies.push(DependencyFact {
                    manager: manager.to_owned(),
                    name: name.clone(),
                    requirement: toml_requirement(requirement),
                    citation: line_citation(source, snapshot, name),
                });
            }
            continue;
        }
        for (name, child) in table {
            let child_path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            pending.push((child, child_path));
        }
    }
}

fn toml_requirement(value: &toml::Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.to_owned());
    }
    value
        .as_table()
        .and_then(|table| table.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
}

fn pyproject_dependencies(
    source: &GraphSource,
    snapshot: &str,
) -> Result<Vec<DependencyFact>, String> {
    let value: toml::Value = toml::from_str(&source.text).map_err(|error| error.to_string())?;
    let mut dependencies = Vec::new();
    if let Some(items) = value
        .get("project")
        .and_then(|project| project.get("dependencies"))
        .and_then(toml::Value::as_array)
    {
        for item in items.iter().filter_map(toml::Value::as_str) {
            push_python_dependency(item, source, snapshot, &mut dependencies);
        }
    }
    if let Some(groups) = value
        .get("project")
        .and_then(|project| project.get("optional-dependencies"))
        .and_then(toml::Value::as_table)
    {
        for items in groups.values().filter_map(toml::Value::as_array) {
            for item in items.iter().filter_map(toml::Value::as_str) {
                push_python_dependency(item, source, snapshot, &mut dependencies);
            }
        }
    }
    if let Some(groups) = value
        .get("dependency-groups")
        .and_then(toml::Value::as_table)
    {
        for items in groups.values().filter_map(toml::Value::as_array) {
            for item in items.iter().filter_map(toml::Value::as_str) {
                push_python_dependency(item, source, snapshot, &mut dependencies);
            }
        }
    }
    if let Some(poetry) = value.get("tool").and_then(|tool| tool.get("poetry")) {
        collect_poetry_dependencies(poetry, source, snapshot, &mut dependencies);
    }
    Ok(dependencies)
}

fn push_python_dependency(
    value: &str,
    source: &GraphSource,
    snapshot: &str,
    dependencies: &mut Vec<DependencyFact>,
) {
    let (name, requirement) = split_python_requirement(value);
    if !name.is_empty() {
        dependencies.push(DependencyFact {
            manager: "python".to_owned(),
            name: name.clone(),
            requirement,
            citation: line_citation(source, snapshot, &name),
        });
    }
}

fn collect_poetry_dependencies(
    value: &toml::Value,
    source: &GraphSource,
    snapshot: &str,
    dependencies: &mut Vec<DependencyFact>,
) {
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        let Some(table) = value.as_table() else {
            continue;
        };
        for (key, child) in table {
            if key == "dependencies" {
                if let Some(table) = child.as_table() {
                    for (name, requirement) in table {
                        if name.eq_ignore_ascii_case("python") {
                            continue;
                        }
                        dependencies.push(DependencyFact {
                            manager: "python".to_owned(),
                            name: name.clone(),
                            requirement: toml_requirement(requirement),
                            citation: line_citation(source, snapshot, name),
                        });
                    }
                }
            } else if key != "name" && key != "version" {
                pending.push(child);
            }
        }
    }
}

fn requirements_dependencies(
    source: &GraphSource,
    snapshot: &str,
) -> Result<Vec<DependencyFact>, String> {
    let mut dependencies = Vec::new();
    for line in source.text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') || line.contains("://")
        {
            continue;
        }
        let (name, requirement) = split_python_requirement(line);
        if !name.is_empty() {
            dependencies.push(DependencyFact {
                manager: "python".to_owned(),
                name: name.clone(),
                requirement,
                citation: line_citation(source, snapshot, &name),
            });
        }
    }
    Ok(dependencies)
}

fn split_python_requirement(value: &str) -> (String, Option<String>) {
    let value = value.split(';').next().unwrap_or(value).trim();
    let end = value
        .char_indices()
        .find(|(_, character)| {
            matches!(character, '<' | '>' | '=' | '!' | '~' | '[' | '@')
                || character.is_whitespace()
        })
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    let name = value[..end].trim().to_owned();
    let requirement = value[end..]
        .trim()
        .strip_prefix('@')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            let value = value[end..].trim();
            (!value.is_empty()).then(|| value.to_owned())
        });
    (name, requirement)
}

fn line_citation(source: &GraphSource, snapshot: &str, needle: &str) -> GraphCitation {
    let line = source
        .text
        .lines()
        .position(|line| line.contains(needle))
        .map(|index| index as u64 + 1)
        .unwrap_or(1);
    GraphCitation {
        artifact_path: source.artifact.path.clone(),
        start_line: line,
        start_column: 1,
        end_line: line,
        end_column: 1,
        content_hash: source.artifact.content_hash.clone(),
        artifact_snapshot_id: snapshot.to_owned(),
    }
}

fn capture_git_state(project_root: &Path) -> Result<Option<GitState>, LeyCoreError> {
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg("core.fsmonitor=false")
        .arg("-c")
        .arg("core.untrackedCache=false")
        .arg("-C")
        .arg(project_root)
        .args([
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=no",
            "--",
            ".",
        ])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for variable in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_NAMESPACE",
        "GIT_CONFIG",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CEILING_DIRECTORIES",
        "GIT_DISCOVERY_ACROSS_FILESYSTEM",
        "GIT_TRACE",
        "GIT_TRACE2",
        "GIT_TRACE2_EVENT",
        "GIT_TRACE2_PERF",
        "GIT_TRACE_PERFORMANCE",
        "GIT_TRACE_SETUP",
        "GIT_TRACE_PACKET",
        "GIT_TRACE_CURL",
        "GIT_REDIRECT_STDERR",
    ] {
        command.env_remove(variable);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(LeyCoreError::Io {
                path: project_root.to_path_buf(),
                source,
            })
        }
    };
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .expect("piped Git stdout is available")
        .take(GIT_OUTPUT_LIMIT_BYTES + 1)
        .read_to_end(&mut output)
        .map_err(|source| LeyCoreError::Io {
            path: project_root.to_path_buf(),
            source,
        })?;
    if output.len() as u64 > GIT_OUTPUT_LIMIT_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        return Err(LeyCoreError::InvalidProjectGraph(format!(
            "Git status exceeds the {GIT_OUTPUT_LIMIT_BYTES}-byte safety limit"
        )));
    }
    let status = child.wait().map_err(|source| LeyCoreError::Io {
        path: project_root.to_path_buf(),
        source,
    })?;
    if !status.success() {
        return Ok(None);
    }
    parse_git_status(&output)
        .map(Some)
        .map_err(LeyCoreError::InvalidProjectGraph)
}

fn parse_git_status(output: &[u8]) -> Result<GitState, String> {
    let mut state = GitState {
        head: None,
        branch: None,
        upstream: None,
        ahead: 0,
        behind: 0,
        changes: Vec::new(),
    };
    let records = output.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        if record.is_empty() {
            index += 1;
            continue;
        }
        let text = std::str::from_utf8(record)
            .map_err(|_| "Git status contains a non-UTF-8 path".to_owned())?;
        if let Some(value) = text.strip_prefix("# branch.oid ") {
            if value != "(initial)" {
                state.head = Some(value.to_owned());
            }
        } else if let Some(value) = text.strip_prefix("# branch.head ") {
            if value != "(detached)" {
                state.branch = Some(value.to_owned());
            }
        } else if let Some(value) = text.strip_prefix("# branch.upstream ") {
            state.upstream = Some(value.to_owned());
        } else if let Some(value) = text.strip_prefix("# branch.ab ") {
            for part in value.split_whitespace() {
                if let Some(value) = part.strip_prefix('+') {
                    state.ahead = value.parse().unwrap_or(0);
                } else if let Some(value) = part.strip_prefix('-') {
                    state.behind = value.parse().unwrap_or(0);
                }
            }
        } else if text.starts_with("1 ") {
            let parts = text.splitn(9, ' ').collect::<Vec<_>>();
            if parts.len() == 9 {
                state.changes.push(GitChange {
                    status: parts[1].to_owned(),
                    path: parts[8].to_owned(),
                    original_path: None,
                });
            }
        } else if text.starts_with("2 ") {
            let parts = text.splitn(10, ' ').collect::<Vec<_>>();
            if parts.len() == 10 {
                let original_path = records
                    .get(index + 1)
                    .and_then(|value| std::str::from_utf8(value).ok())
                    .map(str::to_owned);
                state.changes.push(GitChange {
                    status: parts[1].to_owned(),
                    path: parts[9].to_owned(),
                    original_path,
                });
                index += 1;
            }
        } else if text.starts_with("u ") {
            let parts = text.splitn(11, ' ').collect::<Vec<_>>();
            if parts.len() == 11 {
                state.changes.push(GitChange {
                    status: parts[1].to_owned(),
                    path: parts[10].to_owned(),
                    original_path: None,
                });
            }
        }
        index += 1;
    }
    state.changes.sort_by(|left, right| {
        (&left.path, &left.original_path, &left.status).cmp(&(
            &right.path,
            &right.original_path,
            &right.status,
        ))
    });
    Ok(state)
}

pub(crate) fn graph_body(graph: &ProjectGraph) -> Result<Vec<u8>, LeyCoreError> {
    validate_project_graph(graph, &graph.project_id)?;
    let mut body = serde_json::to_vec_pretty(graph)
        .map_err(|error| LeyCoreError::InvalidProjectGraph(error.to_string()))?;
    body.push(b'\n');
    if body.len() as u64 > PROJECT_GRAPH_LIMIT_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: "graph-v1.json".into(),
            limit_bytes: PROJECT_GRAPH_LIMIT_BYTES,
        });
    }
    Ok(body)
}

pub(crate) fn validate_project_graph(
    graph: &ProjectGraph,
    expected_project_id: &str,
) -> Result<(), LeyCoreError> {
    if graph.schema_version != PROJECT_GRAPH_SCHEMA_VERSION {
        return Err(LeyCoreError::InvalidProjectGraph(format!(
            "unsupported graph schema version {}",
            graph.schema_version
        )));
    }
    validate_project_id(&graph.project_id)?;
    if graph.project_id != expected_project_id {
        return Err(LeyCoreError::InvalidProjectGraph(
            "graph project ID does not match the initialized project".to_owned(),
        ));
    }
    if graph.project_name.trim().is_empty()
        || graph.project_name.chars().count() > 128
        || graph.project_name.chars().any(char::is_control)
    {
        return Err(LeyCoreError::InvalidProjectGraph(
            "graph project name is invalid".to_owned(),
        ));
    }
    if !valid_snapshot_id(&graph.artifact_snapshot_id, "snp_")
        || !valid_snapshot_id(&graph.graph_snapshot_id, "grf_")
        || graph.graph_snapshot_id != graph_snapshot_id(graph)
    {
        return Err(LeyCoreError::InvalidProjectGraph(
            "graph snapshot identity is invalid".to_owned(),
        ));
    }
    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    if node_ids.len() != graph.nodes.len()
        || graph
            .nodes
            .windows(2)
            .any(|nodes| nodes[0].id >= nodes[1].id)
    {
        return Err(LeyCoreError::InvalidProjectGraph(
            "graph nodes must be sorted by unique ID".to_owned(),
        ));
    }
    if graph
        .nodes
        .iter()
        .filter(|node| node.kind == GraphNodeKind::Project)
        .count()
        != 1
    {
        return Err(LeyCoreError::InvalidProjectGraph(
            "graph must contain exactly one project node".to_owned(),
        ));
    }
    for node in &graph.nodes {
        if node.name.is_empty() || node.name.chars().count() > 1024 {
            return Err(LeyCoreError::InvalidProjectGraph(format!(
                "graph node {} has an invalid name",
                node.id
            )));
        }
        if let Some(path) = &node.path {
            validate_graph_path(path)?;
        }
        if let Some(citation) = &node.citation {
            validate_citation(citation, &graph.artifact_snapshot_id)?;
        }
    }
    if graph
        .edges
        .windows(2)
        .any(|edges| edges[0].id >= edges[1].id)
    {
        return Err(LeyCoreError::InvalidProjectGraph(
            "graph edges must be sorted by unique ID".to_owned(),
        ));
    }
    for edge in &graph.edges {
        if !node_ids.contains(edge.source.as_str()) || !node_ids.contains(edge.target.as_str()) {
            return Err(LeyCoreError::InvalidProjectGraph(format!(
                "graph edge {} refers to a missing node",
                edge.id
            )));
        }
        if edge
            .label
            .as_ref()
            .is_some_and(|label| label.chars().count() > 4096)
        {
            return Err(LeyCoreError::InvalidProjectGraph(format!(
                "graph edge {} has an oversized label",
                edge.id
            )));
        }
        if let Some(citation) = &edge.citation {
            validate_citation(citation, &graph.artifact_snapshot_id)?;
        }
    }
    for confidence in graph
        .nodes
        .iter()
        .map(|node| node.confidence)
        .chain(graph.edges.iter().map(|edge| edge.confidence))
    {
        if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
            return Err(LeyCoreError::InvalidProjectGraph(
                "graph confidence must be between zero and one".to_owned(),
            ));
        }
    }
    for diagnostic in &graph.diagnostics {
        validate_graph_path(&diagnostic.artifact_path)?;
        if diagnostic.kind.is_empty()
            || diagnostic.kind.chars().count() > 128
            || diagnostic.message.is_empty()
            || diagnostic.message.chars().count() > 2048
        {
            return Err(LeyCoreError::InvalidProjectGraph(
                "graph diagnostic is invalid".to_owned(),
            ));
        }
    }
    if let Some(git) = &graph.git {
        if git.head.as_ref().is_some_and(|head| {
            !matches!(head.len(), 40 | 64) || !head.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) || git
            .branch
            .as_ref()
            .is_some_and(|branch| branch.is_empty() || branch.chars().count() > 1024)
            || git
                .upstream
                .as_ref()
                .is_some_and(|upstream| upstream.is_empty() || upstream.chars().count() > 1024)
        {
            return Err(LeyCoreError::InvalidProjectGraph(
                "graph Git identity is invalid".to_owned(),
            ));
        }
        for change in &git.changes {
            validate_graph_path(&change.path)?;
            if let Some(path) = &change.original_path {
                validate_graph_path(path)?;
            }
            if change.status.is_empty() || change.status.len() > 8 {
                return Err(LeyCoreError::InvalidProjectGraph(
                    "graph Git change status is invalid".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_citation(
    citation: &GraphCitation,
    expected_snapshot: &str,
) -> Result<(), LeyCoreError> {
    validate_graph_path(&citation.artifact_path)?;
    if citation.artifact_snapshot_id != expected_snapshot
        || !citation
            .content_hash
            .strip_prefix("sha256:")
            .is_some_and(|hash| {
                hash.len() == 64
                    && hash
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        || citation.end_line < citation.start_line
        || (citation.start_line == citation.end_line && citation.end_column < citation.start_column)
    {
        return Err(LeyCoreError::InvalidProjectGraph(
            "graph citation is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_graph_path(value: &str) -> Result<(), LeyCoreError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            !matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return Err(LeyCoreError::InvalidProjectGraph(format!(
            "unsafe graph artifact path {value}"
        )));
    }
    Ok(())
}

fn graph_snapshot_id(graph: &ProjectGraph) -> String {
    let identity = GraphIdentity {
        schema_version: graph.schema_version,
        project_id: &graph.project_id,
        project_name: &graph.project_name,
        artifact_snapshot_id: &graph.artifact_snapshot_id,
        nodes: &graph.nodes,
        edges: &graph.edges,
        diagnostics: &graph.diagnostics,
        git: &graph.git,
    };
    let bytes = serde_json::to_vec(&identity).expect("graph identity is serializable");
    stable_id("grf", &[&String::from_utf8_lossy(&bytes)])
}

fn valid_snapshot_id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{prefix}_{:x}", hasher.finalize())
}

fn file_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_owned()
}

fn count_error_nodes(root: Node<'_>) -> usize {
    let mut count = 0;
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        count += usize::from(node.is_error() || node.is_missing());
        for index in (0..node.child_count()).rev() {
            let index = u32::try_from(index).expect("Tree-sitter child indexes fit in u32");
            if let Some(child) = node.child(index) {
                pending.push(child);
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(path: &str, language: &str, text: &str, kind: ArtifactKind) -> GraphSource {
        GraphSource {
            artifact: ArtifactRecord {
                path: path.to_owned(),
                kind,
                language: Some(language.to_owned()),
                source_bytes: text.len() as u64,
                stored_bytes: text.len() as u64,
                line_count: text.lines().count() as u64,
                content_hash: format!("sha256:{:x}", Sha256::digest(text)),
                content_blob: None,
                redactions: Vec::new(),
            },
            text: text.to_owned(),
        }
    }

    #[test]
    fn graph_extracts_symbols_calls_imports_inheritance_and_dependencies() {
        let root = tempfile::tempdir().unwrap();
        let sources = vec![
            source(
                "src/main.ts",
                "typescript",
                "import { Base } from '@ley/core';\nclass Memory extends Base {\n  recall() { helper(); }\n}\nfunction helper() {}\n",
                ArtifactKind::Source,
            ),
            source(
                "package.json",
                "json",
                r#"{"dependencies":{"@ley/core":"^1.2.3"}}"#,
                ArtifactKind::Manifest,
            ),
            source(
                "src/lib.rs",
                "rust",
                "trait Recall { fn recall(&self); }\nstruct Store;\nimpl Recall for Store { fn recall(&self) { helper(); } }\nfn helper() {}\n",
                ArtifactKind::Source,
            ),
            source(
                "memory.py",
                "python",
                "from ley.core import Brain\nclass Memory(Brain):\n    def recall(self):\n        helper()\n",
                ArtifactKind::Source,
            ),
        ];
        let graph = build_project_graph(
            root.path(),
            "prj_0123456789abcdef0123456789abcdef",
            "Graph test",
            &format!("snp_{}", "a".repeat(64)),
            &sources,
            1,
        )
        .unwrap();

        assert!(graph.nodes.iter().any(|node| {
            node.kind == GraphNodeKind::Symbol
                && node.name == "Memory"
                && node.symbol_kind.as_deref() == Some("class")
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == GraphNodeKind::Dependency
                && node.name == "@ley/core"
                && node.package_manager.as_deref() == Some("npm")
        }));
        assert!(graph.edges.iter().any(
            |edge| edge.kind == GraphEdgeKind::Calls && edge.label.as_deref() == Some("helper")
        ));
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == GraphEdgeKind::Imports && edge.label.as_deref() == Some("@ley/core")
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == GraphEdgeKind::Inherits && edge.label.as_deref() == Some("Base")
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == GraphEdgeKind::Inherits && edge.label.as_deref() == Some("Brain")
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == GraphEdgeKind::Implements && edge.label.as_deref() == Some("Recall")
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == GraphEdgeKind::References && edge.label.as_deref() == Some("Recall")
        }));
    }

    #[test]
    fn git_porcelain_v2_parser_preserves_branch_and_renames() {
        let body = b"# branch.oid aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\x32 R. N... 100644 100644 100644 a b R100 new.rs\0old.rs\0";
        let state = parse_git_status(body).unwrap();
        assert_eq!(
            state.head.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(state.branch.as_deref(), Some("main"));
        assert_eq!(state.ahead, 2);
        assert_eq!(state.behind, 1);
        assert_eq!(state.changes[0].path, "new.rs");
        assert_eq!(state.changes[0].original_path.as_deref(), Some("old.rs"));
    }

    #[test]
    fn graph_identity_ignores_generation_time_but_tracks_git_state() {
        let root = tempfile::tempdir().unwrap();
        let sources = vec![source(
            "main.py",
            "python",
            "def recall():\n    helper()\n",
            ArtifactKind::Source,
        )];
        let first = build_project_graph(
            root.path(),
            "prj_0123456789abcdef0123456789abcdef",
            "Graph test",
            &format!("snp_{}", "b".repeat(64)),
            &sources,
            1,
        )
        .unwrap();
        let second = build_project_graph(
            root.path(),
            "prj_0123456789abcdef0123456789abcdef",
            "Graph test",
            &format!("snp_{}", "b".repeat(64)),
            &sources,
            2,
        )
        .unwrap();
        assert_eq!(first.graph_snapshot_id, second.graph_snapshot_id);
    }
}
