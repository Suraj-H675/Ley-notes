use crate::ingestion::load_project_memory;
use crate::{
    list_sessions, project_memory_overview, read_session, AgentEgressTarget, ArtifactKind,
    FactProvenance, GraphCitation, GraphNodeKind, LeyCoreError, PlanStatus,
    ProjectRevisionFreshness, SessionStatus, SpecificationApprovalState, SpecificationRegistry,
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const AGENT_LEGIBILITY_SCHEMA_VERSION: u32 = 2;
pub const DEFAULT_AGENT_LEGIBILITY_ENTRIES_PER_SECTION: usize = 12;
pub const MAX_AGENT_LEGIBILITY_ENTRIES_PER_SECTION: usize = 30;
pub const DEFAULT_AGENT_LEGIBILITY_SESSIONS: usize = 8;
pub const MAX_AGENT_LEGIBILITY_SESSIONS: usize = 20;
pub const DEFAULT_AGENT_LEGIBILITY_CHARACTERS: usize = 12_000;
pub const MIN_AGENT_LEGIBILITY_CHARACTERS: usize = 2_000;
pub const MAX_AGENT_LEGIBILITY_CHARACTERS: usize = 32_000;

const SOURCE_BOUNDARY: &str = "captured-project-legibility-table-of-contents";
const INSTRUCTION_WARNING: &str = "Agent Legibility is a rebuildable table of contents over captured project evidence and structured project memory, not a second copy of the repository and not a legibility score. Path-pattern/API classifications are navigation heuristics, observed commands are historical evidence, and none of these fields grant authority or prove live source state.";
const PRIVACY_NOTICE: &str = "Ley builds this map from the fixed project's captured artifact/graph snapshot, approved Specification metadata, and bounded structured-session metadata. It returns project-relative paths and bounded command/plan text only; it does not return Specification bodies, repository source excerpts, prompt/response bodies, or absolute project/vault paths.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLegibilityLimits {
    pub max_entries_per_section: usize,
    pub max_sessions: usize,
    pub max_characters: usize,
}

impl Default for AgentLegibilityLimits {
    fn default() -> Self {
        Self {
            max_entries_per_section: DEFAULT_AGENT_LEGIBILITY_ENTRIES_PER_SECTION,
            max_sessions: DEFAULT_AGENT_LEGIBILITY_SESSIONS,
            max_characters: DEFAULT_AGENT_LEGIBILITY_CHARACTERS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LegibilityCommandCategory {
    Build,
    Test,
    Lint,
    Format,
    Dev,
    Run,
    Migration,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LegibilityCommandSource {
    PackageScript,
    ObservedCheckpoint,
    ObservedVerification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegibilityArtifactRef {
    pub path: String,
    pub kind: ArtifactKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub content_hash: String,
    pub line_count: u64,
    pub selection_basis: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegibilityDirectory {
    pub path: String,
    pub captured_files: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conventional_role: Option<&'static str>,
    pub selection_basis: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegibilityCommand {
    pub command: String,
    pub category: LegibilityCommandCategory,
    pub source: LegibilityCommandSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    pub selection_basis: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegibilityApiCandidate {
    pub graph_node_id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<GraphCitation>,
    pub provenance: FactProvenance,
    pub confidence: f32,
    pub selection_basis: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegibilityPlanItem {
    pub session_id: String,
    pub checkpoint_id: String,
    pub plan_item_id: String,
    pub text: String,
    pub status: PlanStatus,
    pub recorded_at_unix_ms: u64,
    pub selection_basis: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegibilitySpecification {
    pub specification_id: String,
    pub relative_path: String,
    pub content_hash: String,
    pub approved_at_unix_ms: u64,
    pub authority: &'static str,
    pub selection_basis: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegibilityGap {
    pub kind: &'static str,
    pub detail: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLegibilityCoverage {
    pub captured_files: usize,
    pub retained_source_files: usize,
    pub sessions_total: usize,
    pub sessions_inspected: usize,
    pub sessions_omitted: usize,
    pub all_sessions_inspected: bool,
    pub architecture_candidates: usize,
    pub policy_candidates: usize,
    pub schema_migration_candidates: usize,
    pub observability_candidates: usize,
    pub api_candidates: usize,
    pub declared_command_candidates: usize,
    pub inspected_session_observed_command_candidates: usize,
    pub inspected_session_current_plan_candidates: usize,
    pub current_specifications: usize,
    pub changed_specifications: usize,
    pub missing_specifications: usize,
    pub omitted_entries: usize,
    pub text_characters: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLegibilityMap {
    pub schema_version: u32,
    pub project_id: String,
    pub project_name: String,
    pub artifact_snapshot_id: String,
    pub graph_snapshot_id: String,
    pub generated_at_unix_ms: u64,
    pub revision_freshness: ProjectRevisionFreshness,
    pub map_fingerprint: String,
    pub projection: &'static str,
    pub persisted: bool,
    pub table_of_contents_not_score: bool,
    pub egress_target: AgentEgressTarget,
    pub architecture_docs: Vec<LegibilityArtifactRef>,
    pub important_directories: Vec<LegibilityDirectory>,
    pub declared_commands: Vec<LegibilityCommand>,
    pub observed_commands: Vec<LegibilityCommand>,
    pub project_policies: Vec<LegibilityArtifactRef>,
    pub schema_migrations: Vec<LegibilityArtifactRef>,
    pub primary_api_candidates: Vec<LegibilityApiCandidate>,
    pub observability_references: Vec<LegibilityArtifactRef>,
    pub current_plans: Vec<LegibilityPlanItem>,
    pub important_specifications: Vec<LegibilitySpecification>,
    pub gaps: Vec<LegibilityGap>,
    pub coverage: AgentLegibilityCoverage,
    pub live_source_checked: bool,
    pub source_boundary: &'static str,
    pub instruction_warning: &'static str,
    pub privacy_notice: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintInput<'a> {
    schema_version: u32,
    project_id: &'a str,
    artifact_snapshot_id: &'a str,
    graph_snapshot_id: &'a str,
    revision_freshness: &'a ProjectRevisionFreshness,
    architecture_docs: &'a [LegibilityArtifactRef],
    important_directories: &'a [LegibilityDirectory],
    declared_commands: &'a [LegibilityCommand],
    observed_commands: &'a [LegibilityCommand],
    project_policies: &'a [LegibilityArtifactRef],
    schema_migrations: &'a [LegibilityArtifactRef],
    primary_api_candidates: &'a [LegibilityApiCandidate],
    observability_references: &'a [LegibilityArtifactRef],
    current_plans: &'a [LegibilityPlanItem],
    important_specifications: &'a [LegibilitySpecification],
    gaps: &'a [LegibilityGap],
    coverage: &'a AgentLegibilityCoverage,
}

pub fn compile_agent_legibility_map(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    limits: AgentLegibilityLimits,
    specification_registry: &SpecificationRegistry,
    egress_target: AgentEgressTarget,
) -> Result<AgentLegibilityMap, LeyCoreError> {
    validate_limits(limits)?;
    let project_start = project_start.as_ref();
    let vault = vault.as_ref();
    let overview = project_memory_overview(project_start, vault)?;
    let memory = load_project_memory(project_start, vault)?;
    let mut budget = TextBudget::new(limits.max_characters);

    let mut architecture_candidates = memory
        .manifest
        .files
        .iter()
        .filter(|artifact| is_architecture_path(&artifact.path))
        .collect::<Vec<_>>();
    architecture_candidates.sort_by(|left, right| left.path.cmp(&right.path));
    let architecture_total = architecture_candidates.len();
    let architecture_docs = architecture_candidates
        .into_iter()
        .take(limits.max_entries_per_section)
        .map(|artifact| artifact_ref(artifact, "captured-path-architecture-pattern"))
        .collect::<Vec<_>>();

    let (important_directories, directory_total) =
        directory_map(&memory.manifest.files, limits.max_entries_per_section);

    let (declared_commands, declared_command_total, declared_source_unavailable) =
        declared_commands(&memory, limits.max_entries_per_section, &mut budget)?;

    let mut policy_candidates = memory
        .manifest
        .files
        .iter()
        .filter(|artifact| is_policy_path(&artifact.path))
        .collect::<Vec<_>>();
    policy_candidates.sort_by(|left, right| left.path.cmp(&right.path));
    let policy_total = policy_candidates.len();
    let project_policies = policy_candidates
        .into_iter()
        .take(limits.max_entries_per_section)
        .map(|artifact| artifact_ref(artifact, "captured-path-policy-pattern"))
        .collect::<Vec<_>>();

    let mut schema_candidates = memory
        .manifest
        .files
        .iter()
        .filter(|artifact| is_schema_migration_path(&artifact.path))
        .collect::<Vec<_>>();
    schema_candidates.sort_by(|left, right| left.path.cmp(&right.path));
    let schema_total = schema_candidates.len();
    let schema_migrations = schema_candidates
        .into_iter()
        .take(limits.max_entries_per_section)
        .map(|artifact| artifact_ref(artifact, "captured-path-schema-migration-pattern"))
        .collect::<Vec<_>>();

    let mut observability_candidates = memory
        .manifest
        .files
        .iter()
        .filter(|artifact| is_observability_path(&artifact.path))
        .collect::<Vec<_>>();
    observability_candidates.sort_by(|left, right| left.path.cmp(&right.path));
    let observability_total = observability_candidates.len();
    let observability_references = observability_candidates
        .into_iter()
        .take(limits.max_entries_per_section)
        .map(|artifact| artifact_ref(artifact, "captured-path-observability-pattern"))
        .collect::<Vec<_>>();

    let mut api_candidates = memory
        .graph
        .nodes
        .iter()
        .filter(|node| {
            node.kind == GraphNodeKind::Symbol
                && node
                    .path
                    .as_deref()
                    .is_some_and(|path| is_api_path(path) || is_api_name(&node.name))
        })
        .collect::<Vec<_>>();
    api_candidates.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    let api_total = api_candidates.len();
    let primary_api_candidates = api_candidates
        .into_iter()
        .take(limits.max_entries_per_section)
        .filter_map(|node| {
            node.path.as_ref().map(|path| LegibilityApiCandidate {
                graph_node_id: node.id.clone(),
                name: node.name.clone(),
                path: path.clone(),
                symbol_kind: node.symbol_kind.clone(),
                citation: node.citation.clone(),
                provenance: node.provenance,
                confidence: node.confidence,
                selection_basis: "captured-symbol-api-path-or-name-pattern",
            })
        })
        .collect::<Vec<_>>();

    let mut sessions = list_sessions(project_start, vault)?;
    sessions.sort_by(|left, right| {
        session_priority(left.status)
            .cmp(&session_priority(right.status))
            .then_with(|| right.updated_at_unix_ms.cmp(&left.updated_at_unix_ms))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    let sessions_total = sessions.len();
    let selected_sessions = sessions
        .into_iter()
        .take(limits.max_sessions)
        .collect::<Vec<_>>();
    let sessions_inspected = selected_sessions.len();
    let mut observed_command_candidates = Vec::new();
    let mut current_plan_candidates = Vec::new();
    for summary in selected_sessions {
        let session = read_session(project_start, vault, &summary.session_id)?;
        let Some(checkpoint) = session.checkpoints.last() else {
            continue;
        };
        for command in &checkpoint.commands {
            observed_command_candidates.push(LegibilityCommand {
                command: command.command.clone(),
                category: classify_command("", &command.command),
                source: LegibilityCommandSource::ObservedCheckpoint,
                declaration_name: None,
                artifact_path: None,
                session_id: Some(session.session_id.clone()),
                checkpoint_id: Some(checkpoint.id.clone()),
                record_id: Some(command.id.clone()),
                selection_basis: "latest-inspected-checkpoint-observed-command",
            });
        }
        for verification in &checkpoint.verification {
            if let Some(command) = verification.command.as_deref() {
                observed_command_candidates.push(LegibilityCommand {
                    command: command.to_owned(),
                    category: classify_command(&verification.kind, command),
                    source: LegibilityCommandSource::ObservedVerification,
                    declaration_name: None,
                    artifact_path: None,
                    session_id: Some(session.session_id.clone()),
                    checkpoint_id: Some(checkpoint.id.clone()),
                    record_id: Some(verification.id.clone()),
                    selection_basis: "latest-inspected-checkpoint-verification-command",
                });
            }
        }
        if matches!(
            session.status,
            SessionStatus::Active | SessionStatus::Paused
        ) {
            for item in checkpoint
                .plan
                .iter()
                .filter(|item| item.status != PlanStatus::Completed)
            {
                current_plan_candidates.push(LegibilityPlanItem {
                    session_id: session.session_id.clone(),
                    checkpoint_id: checkpoint.id.clone(),
                    plan_item_id: item.id.clone(),
                    text: item.text.clone(),
                    status: item.status,
                    recorded_at_unix_ms: checkpoint.recorded_at_unix_ms,
                    selection_basis: "latest-checkpoint-active-or-paused-session-plan",
                });
            }
        }
    }
    dedupe_commands(&mut observed_command_candidates);
    let observed_command_total = observed_command_candidates.len();
    observed_command_candidates.truncate(limits.max_entries_per_section);
    let mut observed_commands = Vec::new();
    for mut command in observed_command_candidates {
        if budget.remaining() == 0 {
            budget.truncated = true;
            break;
        }
        command.command = budget.take(&command.command, 512);
        observed_commands.push(command);
    }

    current_plan_candidates.sort_by(|left, right| {
        right
            .recorded_at_unix_ms
            .cmp(&left.recorded_at_unix_ms)
            .then_with(|| left.session_id.cmp(&right.session_id))
            .then_with(|| left.plan_item_id.cmp(&right.plan_item_id))
    });
    let current_plan_total = current_plan_candidates.len();
    current_plan_candidates.truncate(limits.max_entries_per_section);
    let mut current_plans = Vec::new();
    for mut plan in current_plan_candidates {
        if budget.remaining() == 0 {
            budget.truncated = true;
            break;
        }
        plan.text = budget.take(&plan.text, 768);
        current_plans.push(plan);
    }

    let specification_authority = specification_registry.list(project_start, vault)?;
    let current_spec_total = specification_authority.current;
    let mut important_specifications = specification_authority
        .specifications
        .iter()
        .filter(|item| item.state == SpecificationApprovalState::Current)
        .map(|item| LegibilitySpecification {
            specification_id: item.approval.specification_id.clone(),
            relative_path: item.approval.relative_path.clone(),
            content_hash: item.approval.content_hash.clone(),
            approved_at_unix_ms: item.approval.approved_at_unix_ms,
            authority: "human-intent",
            selection_basis: "current-user-approved-specification-metadata",
        })
        .collect::<Vec<_>>();
    important_specifications.sort_by(|left, right| {
        right
            .approved_at_unix_ms
            .cmp(&left.approved_at_unix_ms)
            .then_with(|| left.specification_id.cmp(&right.specification_id))
    });
    important_specifications.truncate(limits.max_entries_per_section);

    let mut gaps = Vec::new();
    push_gap_if_empty(
        &mut gaps,
        &architecture_docs,
        "architecture-docs-not-detected",
        "No captured path matched the conservative architecture/design/ADR/root-README patterns.",
    );
    if declared_commands.is_empty() {
        gaps.push(LegibilityGap {
            kind: "declared-commands-not-detected",
            detail: if declared_source_unavailable {
                "A package.json manifest was captured without retained source text, so scripts could not be inspected; Ley does not infer commands from package-manager presence alone."
            } else {
                "No captured package.json scripts were found; Ley does not invent build/test/lint commands from framework or package-manager conventions."
            },
        });
    }
    push_gap_if_empty(
        &mut gaps,
        &project_policies,
        "project-policies-not-detected",
        "No captured path matched the conservative AGENTS/CONTRIBUTING/SECURITY/CODEOWNERS/policy patterns.",
    );
    push_gap_if_empty(
        &mut gaps,
        &schema_migrations,
        "schema-migrations-not-detected",
        "No captured path matched the conservative schema/migration patterns.",
    );
    push_gap_if_empty(
        &mut gaps,
        &primary_api_candidates,
        "primary-api-candidates-not-detected",
        "No captured symbol path/name matched the conservative API/route/controller/handler/OpenAPI patterns; Ley does not infer public API status from symbol visibility it has not extracted.",
    );
    push_gap_if_empty(
        &mut gaps,
        &observability_references,
        "observability-references-not-detected",
        "No captured path matched the conservative telemetry/metrics/tracing/logging/Prometheus/Grafana patterns.",
    );
    push_gap_if_empty(
        &mut gaps,
        &current_plans,
        "current-plans-not-detected",
        "No non-completed plan item was found in the latest checkpoint of the bounded active/paused session set.",
    );
    push_gap_if_empty(
        &mut gaps,
        &important_specifications,
        "current-specifications-not-detected",
        "No current user-approved Specification metadata was available in the local authority registry.",
    );

    let returned_counts = architecture_docs.len()
        + important_directories.len()
        + declared_commands.len()
        + observed_commands.len()
        + project_policies.len()
        + schema_migrations.len()
        + primary_api_candidates.len()
        + observability_references.len()
        + current_plans.len()
        + important_specifications.len();
    let candidate_counts = architecture_total
        + directory_total
        + declared_command_total
        + observed_command_total
        + policy_total
        + schema_total
        + api_total
        + observability_total
        + current_plan_total
        + current_spec_total;
    let coverage = AgentLegibilityCoverage {
        captured_files: memory.manifest.files.len(),
        retained_source_files: memory
            .manifest
            .files
            .iter()
            .filter(|artifact| artifact.content_blob.is_some())
            .count(),
        sessions_total,
        sessions_inspected,
        sessions_omitted: sessions_total.saturating_sub(sessions_inspected),
        all_sessions_inspected: sessions_total == sessions_inspected,
        architecture_candidates: architecture_total,
        policy_candidates: policy_total,
        schema_migration_candidates: schema_total,
        observability_candidates: observability_total,
        api_candidates: api_total,
        declared_command_candidates: declared_command_total,
        inspected_session_observed_command_candidates: observed_command_total,
        inspected_session_current_plan_candidates: current_plan_total,
        current_specifications: current_spec_total,
        changed_specifications: specification_authority.changed,
        missing_specifications: specification_authority.missing,
        omitted_entries: candidate_counts.saturating_sub(returned_counts),
        text_characters: budget.used,
        truncated: budget.truncated
            || candidate_counts > returned_counts
            || sessions_total > sessions_inspected,
    };

    let mut map = AgentLegibilityMap {
        schema_version: AGENT_LEGIBILITY_SCHEMA_VERSION,
        project_id: overview.project_id,
        project_name: overview.project_name,
        artifact_snapshot_id: overview.artifact_snapshot_id,
        graph_snapshot_id: overview.graph_snapshot_id,
        generated_at_unix_ms: crate::unix_time_ms(),
        revision_freshness: overview.revision_freshness,
        map_fingerprint: String::new(),
        projection: "on-demand-agent-legibility-map",
        persisted: false,
        table_of_contents_not_score: true,
        egress_target,
        architecture_docs,
        important_directories,
        declared_commands,
        observed_commands,
        project_policies,
        schema_migrations,
        primary_api_candidates,
        observability_references,
        current_plans,
        important_specifications,
        gaps,
        coverage,
        live_source_checked: false,
        source_boundary: SOURCE_BOUNDARY,
        instruction_warning: INSTRUCTION_WARNING,
        privacy_notice: PRIVACY_NOTICE,
    };
    map.map_fingerprint = map_fingerprint(&map);
    Ok(map)
}

fn declared_commands(
    memory: &crate::ingestion::LoadedProjectMemory,
    maximum: usize,
    budget: &mut TextBudget,
) -> Result<(Vec<LegibilityCommand>, usize, bool), LeyCoreError> {
    let mut commands = Vec::new();
    let mut source_unavailable = false;
    for artifact in memory
        .manifest
        .files
        .iter()
        .filter(|artifact| artifact.path.eq_ignore_ascii_case("package.json"))
    {
        let Some(source) = memory.read_artifact_text(artifact)? else {
            source_unavailable = true;
            continue;
        };
        let Ok(document) = serde_json::from_str::<Value>(&source) else {
            continue;
        };
        let Some(scripts) = document.get("scripts").and_then(Value::as_object) else {
            continue;
        };
        let mut ordered = scripts
            .iter()
            .filter_map(|(name, value)| value.as_str().map(|command| (name, command)))
            .collect::<Vec<_>>();
        ordered.sort_by(|left, right| left.0.cmp(right.0));
        for (name, command) in ordered {
            commands.push(LegibilityCommand {
                command: command.to_owned(),
                category: classify_command(name, command),
                source: LegibilityCommandSource::PackageScript,
                declaration_name: Some(name.to_owned()),
                artifact_path: Some(artifact.path.clone()),
                session_id: None,
                checkpoint_id: None,
                record_id: None,
                selection_basis: "captured-package-json-script",
            });
        }
    }
    dedupe_commands(&mut commands);
    let total = commands.len();
    commands.truncate(maximum);
    let mut returned = Vec::new();
    for mut command in commands {
        if budget.remaining() == 0 {
            budget.truncated = true;
            break;
        }
        command.command = budget.take(&command.command, 512);
        returned.push(command);
    }
    Ok((returned, total, source_unavailable))
}

fn directory_map(
    artifacts: &[crate::ArtifactRecord],
    maximum: usize,
) -> (Vec<LegibilityDirectory>, usize) {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for artifact in artifacts {
        let path = Path::new(&artifact.path);
        let mut components = path.components();
        let Some(first) = components.next() else {
            continue;
        };
        if components.next().is_none() {
            continue;
        }
        let name = first.as_os_str().to_string_lossy().into_owned();
        *counts.entry(name).or_default() += 1;
    }
    let mut directories = counts
        .into_iter()
        .map(|(path, captured_files)| LegibilityDirectory {
            conventional_role: directory_role(&path),
            path,
            captured_files,
            selection_basis: "captured-top-level-directory-file-count",
        })
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        right
            .conventional_role
            .is_some()
            .cmp(&left.conventional_role.is_some())
            .then_with(|| right.captured_files.cmp(&left.captured_files))
            .then_with(|| left.path.cmp(&right.path))
    });
    let total = directories.len();
    directories.truncate(maximum);
    (directories, total)
}

fn artifact_ref(
    artifact: &crate::ArtifactRecord,
    selection_basis: &'static str,
) -> LegibilityArtifactRef {
    LegibilityArtifactRef {
        path: artifact.path.clone(),
        kind: artifact.kind,
        language: artifact.language.clone(),
        content_hash: artifact.content_hash.clone(),
        line_count: artifact.line_count,
        selection_basis,
    }
}

fn is_architecture_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower == "readme.md"
        || lower.contains("architecture")
        || lower.contains("system-design")
        || lower.contains("system_design")
        || lower.contains("/adr/")
        || lower.starts_with("docs/adr/")
        || lower.contains("design-doc")
        || lower.contains("design_doc")
        || lower.contains("overview")
}

fn is_policy_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let file = Path::new(&lower)
        .file_name()
        .and_then(|part| part.to_str())
        .unwrap_or("");
    matches!(
        file,
        "agents.md"
            | "contributing.md"
            | "security.md"
            | "code_of_conduct.md"
            | "code-of-conduct.md"
            | "governance.md"
            | "codeowners"
            | ".leyignore"
    ) || lower.contains("/policies/")
        || lower.starts_with("policies/")
}

fn is_schema_migration_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("migration")
        || lower.contains("/schema/")
        || lower.starts_with("schema/")
        || lower.contains("schemas/")
        || lower.ends_with("schema.prisma")
        || lower.contains("alembic")
        || (lower.ends_with(".sql") && (lower.contains("db/") || lower.contains("database/")))
}

fn is_observability_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [
        "observability",
        "telemetry",
        "metrics",
        "tracing",
        "logging",
        "prometheus",
        "grafana",
        "opentelemetry",
        "otel",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_api_path(path: &str) -> bool {
    let lower = format!("/{}", path.to_ascii_lowercase());
    [
        "/api/",
        "/apis/",
        "/routes/",
        "/router/",
        "/controllers/",
        "/handlers/",
        "openapi",
        "swagger",
        "graphql",
        "/rpc/",
        "endpoint",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn is_api_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["handler", "controller", "route", "endpoint", "resolver"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn directory_role(path: &str) -> Option<&'static str> {
    match path.to_ascii_lowercase().as_str() {
        "src" | "app" => Some("primary-source"),
        "test" | "tests" | "spec" | "specs" => Some("tests"),
        "docs" | "doc" => Some("documentation"),
        "api" | "apis" | "routes" => Some("api-surface"),
        "db" | "database" | "migrations" | "schema" => Some("data-schema"),
        "scripts" | "tools" => Some("tooling"),
        "config" | "configs" => Some("configuration"),
        "observability" | "monitoring" | "telemetry" => Some("observability"),
        "crates" | "packages" | "modules" => Some("workspace-modules"),
        ".github" => Some("repository-automation"),
        _ => None,
    }
}

fn classify_command(name: &str, command: &str) -> LegibilityCommandCategory {
    let combined = format!("{} {}", name, command).to_ascii_lowercase();
    if combined.contains("test") || combined.contains("pytest") {
        LegibilityCommandCategory::Test
    } else if combined.contains("lint")
        || combined.contains("clippy")
        || combined.contains("eslint")
        || combined.contains("ruff")
    {
        LegibilityCommandCategory::Lint
    } else if combined.contains("format")
        || combined.contains("prettier")
        || combined.contains("cargo fmt")
        || combined.contains(" fmt")
    {
        LegibilityCommandCategory::Format
    } else if combined.contains("migrat") || combined.contains("alembic") {
        LegibilityCommandCategory::Migration
    } else if combined.contains("build") || combined.contains("compile") || combined.contains("tsc")
    {
        LegibilityCommandCategory::Build
    } else if combined.contains("dev") || combined.contains("serve") || combined.contains("watch") {
        LegibilityCommandCategory::Dev
    } else if combined.contains("start") || combined.contains(" run") {
        LegibilityCommandCategory::Run
    } else {
        LegibilityCommandCategory::Other
    }
}

fn dedupe_commands(commands: &mut Vec<LegibilityCommand>) {
    let mut seen = BTreeSet::new();
    commands.retain(|command| {
        seen.insert((
            command.category,
            command.source,
            command.artifact_path.clone(),
            command.declaration_name.clone(),
            normalize_content(&command.command),
        ))
    });
    commands.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.command.cmp(&right.command))
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.session_id.cmp(&right.session_id))
            .then_with(|| left.record_id.cmp(&right.record_id))
    });
}

fn normalize_content(value: &str) -> String {
    let mut output = String::new();
    let mut pending_space = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if pending_space && !output.is_empty() {
                output.push(' ');
            }
            output.push(character);
            pending_space = false;
        } else {
            pending_space = true;
        }
    }
    output
}

fn push_gap_if_empty<T>(
    gaps: &mut Vec<LegibilityGap>,
    values: &[T],
    kind: &'static str,
    detail: &'static str,
) {
    if values.is_empty() {
        gaps.push(LegibilityGap { kind, detail });
    }
}

fn session_priority(status: SessionStatus) -> u8 {
    match status {
        SessionStatus::Active => 0,
        SessionStatus::Paused => 1,
        SessionStatus::Completed => 2,
        SessionStatus::Abandoned => 3,
    }
}

fn validate_limits(limits: AgentLegibilityLimits) -> Result<(), LeyCoreError> {
    if !(1..=MAX_AGENT_LEGIBILITY_ENTRIES_PER_SECTION).contains(&limits.max_entries_per_section) {
        return Err(LeyCoreError::InvalidRetrievalRequest(format!(
            "agent legibility maxEntriesPerSection must be between 1 and {MAX_AGENT_LEGIBILITY_ENTRIES_PER_SECTION}"
        )));
    }
    if !(1..=MAX_AGENT_LEGIBILITY_SESSIONS).contains(&limits.max_sessions) {
        return Err(LeyCoreError::InvalidRetrievalRequest(format!(
            "agent legibility maxSessions must be between 1 and {MAX_AGENT_LEGIBILITY_SESSIONS}"
        )));
    }
    if !(MIN_AGENT_LEGIBILITY_CHARACTERS..=MAX_AGENT_LEGIBILITY_CHARACTERS)
        .contains(&limits.max_characters)
    {
        return Err(LeyCoreError::InvalidRetrievalRequest(format!(
            "agent legibility maxCharacters must be between {MIN_AGENT_LEGIBILITY_CHARACTERS} and {MAX_AGENT_LEGIBILITY_CHARACTERS}"
        )));
    }
    Ok(())
}

fn map_fingerprint(map: &AgentLegibilityMap) -> String {
    let input = FingerprintInput {
        schema_version: map.schema_version,
        project_id: &map.project_id,
        artifact_snapshot_id: &map.artifact_snapshot_id,
        graph_snapshot_id: &map.graph_snapshot_id,
        revision_freshness: &map.revision_freshness,
        architecture_docs: &map.architecture_docs,
        important_directories: &map.important_directories,
        declared_commands: &map.declared_commands,
        observed_commands: &map.observed_commands,
        project_policies: &map.project_policies,
        schema_migrations: &map.schema_migrations,
        primary_api_candidates: &map.primary_api_candidates,
        observability_references: &map.observability_references,
        current_plans: &map.current_plans,
        important_specifications: &map.important_specifications,
        gaps: &map.gaps,
        coverage: &map.coverage,
    };
    let bytes = serde_json::to_vec(&input).expect("agent legibility map is serializable");
    format!("sha256:{:x}", Sha256::digest(bytes))
}

struct TextBudget {
    remaining: usize,
    used: usize,
    truncated: bool,
}

impl TextBudget {
    fn new(maximum: usize) -> Self {
        Self {
            remaining: maximum,
            used: 0,
            truncated: false,
        }
    }

    fn remaining(&self) -> usize {
        self.remaining
    }

    fn take(&mut self, value: &str, per_field_maximum: usize) -> String {
        let allowed = self.remaining.min(per_field_maximum);
        let mut characters = value.chars();
        let mut output = characters.by_ref().take(allowed).collect::<String>();
        let omitted = characters.next().is_some();
        if omitted && allowed > 0 {
            output.pop();
            output.push('…');
        }
        let used = output.chars().count();
        self.remaining = self.remaining.saturating_sub(used);
        self.used = self.used.saturating_add(used);
        self.truncated |= omitted;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        checkpoint_session, generate_specification_id, ingest_project, initialize_project,
        start_session, CaptureMode, CheckpointInput, CommandInput, PlanItemInput, SessionSource,
        StartSessionInput, VerificationInput, VerificationStatus,
    };
    use std::fs;
    use tempfile::tempdir;

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
    }

    #[test]
    fn map_is_a_source_bound_toc_without_copying_specification_bodies() {
        let temporary = tempdir().unwrap();
        let project = temporary.path().join("project");
        let vault = temporary.path().join("vault");
        let config = temporary.path().join("config");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::create_dir_all(project.join("src/api")).unwrap();
        fs::create_dir_all(project.join("db/migrations")).unwrap();
        fs::create_dir_all(project.join("observability")).unwrap();
        fs::create_dir_all(vault.join("Specs")).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(project.join("README.md"), "# Legibility fixture\n").unwrap();
        fs::write(
            project.join("docs/architecture.md"),
            "# Architecture\nCaptured architecture navigation.\n",
        )
        .unwrap();
        fs::write(project.join("AGENTS.md"), "# Project policy\n").unwrap();
        fs::write(
            project.join("src/api/users.rs"),
            "pub fn user_handler() -> &'static str { \"ok\" }\n",
        )
        .unwrap();
        fs::write(
            project.join("db/migrations/001_init.sql"),
            "create table users(id integer);\n",
        )
        .unwrap();
        fs::write(
            project.join("observability/grafana.md"),
            "# Grafana\nDashboard notes.\n",
        )
        .unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"scripts":{"build":"vite build","lint":"eslint .","test":"vitest run"}}"#,
        )
        .unwrap();
        fs::write(
            vault.join("Specs/Operating.md"),
            "# Operating requirement\nlegibility_private_spec_body_71ac\n",
        )
        .unwrap();

        initialize_project(
            &project,
            Some("Legibility fixture"),
            CaptureMode::Structured,
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let registry = SpecificationRegistry::at(config.join("specifications-v1.json"));
        let specification_id = generate_specification_id();
        registry
            .approve(&project, &vault, &specification_id, "Specs/Operating.md")
            .unwrap();

        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('1'),
                name: "Operate project".to_owned(),
                goal: "Record how to operate the project".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            CheckpointInput {
                request_id: request_id('2'),
                summary: "Recorded operating entry points.".to_owned(),
                plan: vec![PlanItemInput {
                    text: "Finish API observability wiring.".to_owned(),
                    status: PlanStatus::InProgress,
                }],
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: vec!["docs/architecture.md".to_owned()],
                commands: vec![CommandInput {
                    command: "cargo test -p api".to_owned(),
                    exit_code: Some(0),
                    summary: "API package tests passed.".to_owned(),
                }],
                verification: vec![VerificationInput {
                    kind: "lint".to_owned(),
                    status: VerificationStatus::Passed,
                    summary: "Lint passed.".to_owned(),
                    command: Some("npm run lint".to_owned()),
                    evidence_artifact_paths: Vec::new(),
                }],
                unresolved: Vec::new(),
            },
        )
        .unwrap();

        let limits = AgentLegibilityLimits::default();
        let first = compile_agent_legibility_map(
            &project,
            &vault,
            limits,
            &registry,
            AgentEgressTarget::Cloud,
        )
        .unwrap();
        let second = compile_agent_legibility_map(
            &project,
            &vault,
            limits,
            &registry,
            AgentEgressTarget::Cloud,
        )
        .unwrap();

        assert_eq!(first.schema_version, AGENT_LEGIBILITY_SCHEMA_VERSION);
        assert_eq!(first.projection, "on-demand-agent-legibility-map");
        assert!(!first.persisted);
        assert!(first.table_of_contents_not_score);
        assert!(!first.live_source_checked);
        assert_eq!(first.map_fingerprint, second.map_fingerprint);
        assert!(first
            .architecture_docs
            .iter()
            .any(|item| item.path == "docs/architecture.md"));
        assert!(first
            .important_directories
            .iter()
            .any(|item| item.path == "src" && item.conventional_role == Some("primary-source")));
        assert!(first.declared_commands.iter().any(|item| {
            item.declaration_name.as_deref() == Some("test")
                && item.command == "vitest run"
                && item.category == LegibilityCommandCategory::Test
        }));
        assert!(first
            .observed_commands
            .iter()
            .any(|item| item.command == "cargo test -p api"));
        assert!(first
            .project_policies
            .iter()
            .any(|item| item.path == "AGENTS.md"));
        assert!(first
            .schema_migrations
            .iter()
            .any(|item| item.path == "db/migrations/001_init.sql"));
        assert!(first
            .primary_api_candidates
            .iter()
            .any(|item| { item.path == "src/api/users.rs" && item.name.contains("user_handler") }));
        assert!(first
            .observability_references
            .iter()
            .any(|item| item.path == "observability/grafana.md"));
        assert!(first
            .current_plans
            .iter()
            .any(|item| item.text == "Finish API observability wiring."));
        assert!(first.important_specifications.iter().any(|item| {
            item.specification_id == specification_id && item.relative_path == "Specs/Operating.md"
        }));
        assert!(first.coverage.text_characters <= limits.max_characters);
        let serialized = serde_json::to_string(&first).unwrap();
        assert!(!serialized.contains("legibility_private_spec_body_71ac"));
        assert!(!serialized.contains(project.to_str().unwrap()));
        assert!(!serialized.contains(vault.to_str().unwrap()));
        let json = serde_json::to_value(&first).unwrap();
        assert!(json.get("score").is_none());

        fs::write(
            project.join("docs/architecture.md"),
            "# Architecture\nCaptured architecture navigation v2.\n",
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let changed = compile_agent_legibility_map(
            &project,
            &vault,
            limits,
            &registry,
            AgentEgressTarget::Cloud,
        )
        .unwrap();
        assert_ne!(first.artifact_snapshot_id, changed.artifact_snapshot_id);
        assert_ne!(first.map_fingerprint, changed.map_fingerprint);
    }

    #[test]
    fn command_dedupe_preserves_distinct_observed_provenance() {
        let mut commands = vec![
            LegibilityCommand {
                command: "cargo test -p api".to_owned(),
                category: LegibilityCommandCategory::Test,
                source: LegibilityCommandSource::ObservedCheckpoint,
                declaration_name: None,
                artifact_path: None,
                session_id: Some("ses_z_newer".to_owned()),
                checkpoint_id: Some("ckp_newer".to_owned()),
                record_id: Some("cmd_newer".to_owned()),
                selection_basis: "latest-inspected-checkpoint-observed-command",
            },
            LegibilityCommand {
                command: "cargo  test -p api".to_owned(),
                category: LegibilityCommandCategory::Test,
                source: LegibilityCommandSource::ObservedCheckpoint,
                declaration_name: None,
                artifact_path: None,
                session_id: Some("ses_a_older".to_owned()),
                checkpoint_id: Some("ckp_older".to_owned()),
                record_id: Some("cmd_older".to_owned()),
                selection_basis: "latest-inspected-checkpoint-observed-command",
            },
            LegibilityCommand {
                command: "cargo test -p api".to_owned(),
                category: LegibilityCommandCategory::Test,
                source: LegibilityCommandSource::ObservedVerification,
                declaration_name: None,
                artifact_path: None,
                session_id: Some("ses_verification".to_owned()),
                checkpoint_id: Some("ckp_verification".to_owned()),
                record_id: Some("ver_verification".to_owned()),
                selection_basis: "latest-inspected-checkpoint-verification-command",
            },
        ];

        dedupe_commands(&mut commands);

        assert_eq!(commands.len(), 2);
        assert_eq!(
            commands
                .iter()
                .filter(|command| command.source == LegibilityCommandSource::ObservedCheckpoint)
                .count(),
            1
        );
        let checkpoint = commands
            .iter()
            .find(|command| command.source == LegibilityCommandSource::ObservedCheckpoint)
            .unwrap();
        assert_eq!(checkpoint.session_id.as_deref(), Some("ses_z_newer"));
        assert_eq!(checkpoint.checkpoint_id.as_deref(), Some("ckp_newer"));
        assert_eq!(checkpoint.record_id.as_deref(), Some("cmd_newer"));
        assert_eq!(
            commands
                .iter()
                .filter(|command| command.source == LegibilityCommandSource::ObservedVerification)
                .count(),
            1
        );
    }

    #[test]
    fn command_dedupe_preserves_distinct_package_script_declarations() {
        let mut commands = vec![
            LegibilityCommand {
                command: "vitest run".to_owned(),
                category: LegibilityCommandCategory::Test,
                source: LegibilityCommandSource::PackageScript,
                declaration_name: Some("test".to_owned()),
                artifact_path: Some("package.json".to_owned()),
                session_id: None,
                checkpoint_id: None,
                record_id: None,
                selection_basis: "captured-package-json-script",
            },
            LegibilityCommand {
                command: "vitest  run".to_owned(),
                category: LegibilityCommandCategory::Test,
                source: LegibilityCommandSource::PackageScript,
                declaration_name: Some("test:unit".to_owned()),
                artifact_path: Some("package.json".to_owned()),
                session_id: None,
                checkpoint_id: None,
                record_id: None,
                selection_basis: "captured-package-json-script",
            },
        ];

        dedupe_commands(&mut commands);

        assert_eq!(commands.len(), 2);
        assert!(commands
            .iter()
            .any(|command| command.declaration_name.as_deref() == Some("test")));
        assert!(commands
            .iter()
            .any(|command| command.declaration_name.as_deref() == Some("test:unit")));
    }

    #[test]
    fn minimal_capture_reports_declared_command_gap_instead_of_inventing_commands() {
        let temporary = tempdir().unwrap();
        let project = temporary.path().join("project");
        let vault = temporary.path().join("vault");
        let config = temporary.path().join("config");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"scripts":{"build":"secret-build-command","test":"secret-test-command"}}"#,
        )
        .unwrap();
        initialize_project(&project, Some("Minimal legibility"), CaptureMode::Minimal).unwrap();
        ingest_project(&project, &vault).unwrap();
        let registry = SpecificationRegistry::at(config.join("specifications-v1.json"));
        let map = compile_agent_legibility_map(
            &project,
            &vault,
            AgentLegibilityLimits::default(),
            &registry,
            AgentEgressTarget::Cloud,
        )
        .unwrap();
        assert!(map.declared_commands.is_empty());
        assert!(map.gaps.iter().any(|gap| {
            gap.kind == "declared-commands-not-detected"
                && gap.detail.contains("without retained source text")
        }));
        let serialized = serde_json::to_string(&map).unwrap();
        assert!(!serialized.contains("secret-build-command"));
        assert!(!serialized.contains("secret-test-command"));
    }

    #[test]
    fn map_names_session_derived_candidate_counts_as_inspected_subset() {
        let temporary = tempdir().unwrap();
        let project = temporary.path().join("project");
        let vault = temporary.path().join("vault");
        let config = temporary.path().join("config");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(project.join("README.md"), "# Legibility bounds\n").unwrap();
        initialize_project(&project, Some("Legibility bounds"), CaptureMode::Structured).unwrap();
        ingest_project(&project, &vault).unwrap();
        let registry = SpecificationRegistry::at(config.join("specifications-v1.json"));

        for (index, digit) in [('1', '2'), ('3', '4')].into_iter().enumerate() {
            let started = start_session(
                &project,
                &vault,
                StartSessionInput {
                    request_id: request_id(digit.0),
                    name: format!("Active legibility session {index}"),
                    goal: "Exercise bounded session-derived legibility coverage".to_owned(),
                    source: SessionSource::default(),
                },
            )
            .unwrap();
            checkpoint_session(
                &project,
                &vault,
                &started.session.session_id,
                CheckpointInput {
                    request_id: request_id(digit.1),
                    summary: "Recorded bounded operating evidence.".to_owned(),
                    plan: vec![PlanItemInput {
                        text: format!("Finish bounded plan {index}"),
                        status: PlanStatus::InProgress,
                    }],
                    decisions: Vec::new(),
                    tasks: Vec::new(),
                    problems: Vec::new(),
                    touched_artifacts: Vec::new(),
                    commands: vec![CommandInput {
                        command: format!("cargo test bounded_{index}"),
                        exit_code: Some(0),
                        summary: "Historical command evidence.".to_owned(),
                    }],
                    verification: Vec::new(),
                    unresolved: Vec::new(),
                },
            )
            .unwrap();
        }

        let map = compile_agent_legibility_map(
            &project,
            &vault,
            AgentLegibilityLimits {
                max_entries_per_section: 12,
                max_sessions: 1,
                max_characters: 12_000,
            },
            &registry,
            AgentEgressTarget::Cloud,
        )
        .unwrap();
        assert_eq!(map.schema_version, 2);
        assert_eq!(map.coverage.sessions_total, 2);
        assert_eq!(map.coverage.sessions_inspected, 1);
        assert_eq!(map.coverage.sessions_omitted, 1);
        assert!(!map.coverage.all_sessions_inspected);
        assert_eq!(
            map.coverage.inspected_session_observed_command_candidates,
            1
        );
        assert_eq!(map.coverage.inspected_session_current_plan_candidates, 1);
        assert!(map.coverage.truncated);
    }
}
