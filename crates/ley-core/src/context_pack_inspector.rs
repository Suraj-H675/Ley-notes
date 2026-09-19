use crate::{
    AgentEgressTarget, CompiledContextPack, ContextAdmissionBasis, ContextAuthority,
    ContextCompileCoverage, ContextEgressCoverage, ContextEgressExclusion, ContextExclusion,
    ContextFollowUp, ContextGap, ContextPremiseAdjudication, GraphCitation,
    MountedReferenceCoverage, MountedReferenceExclusion, MountedReferenceScope,
    ProjectMemoryConflict, ProjectMemoryResultKind, ProjectMemorySearchRetrieval,
    ProjectRevisionFreshness, RevisionApplicability, SharedKnowledgeCoverage,
    SharedKnowledgeExclusion, SharedKnowledgeScope, SpecificationCompileCoverage,
    SpecificationCompileExclusion,
};
use serde::Serialize;

pub const CONTEXT_PACK_INSPECTOR_SCHEMA_VERSION: u32 = 2;

const INSPECTION_BASIS: &str = "current-recompiled-context-pack-manifest";
const SOURCE_BOUNDARY: &str = "derived-context-pack-inspection";
const INSTRUCTION_WARNING: &str = "This Inspector manifest explains one compiled Ley context pack. It does not make stored text authoritative, does not prove live source is unchanged, and does not reconstruct an older pack when the expected contextPackId differs.";
const PRIVACY_NOTICE: &str = "The Inspector manifest omits included active-project, Specification, mounted-reference, and shared-scope text/excerpts. It exposes only diagnostic metadata already represented by the compiled pack: stable IDs, project-relative citations/paths, mount/scope/source identities, authority/admission reasoning, exclusions, conflicts, retrieval/revision signals, budget composition, coverage, and follow-up handles.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextPackRecordSource {
    Specification,
    ActiveProjectMemory,
    MountedReference,
    SharedKnowledgeReference,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPackIncludedRecord {
    pub source: ContextPackRecordSource,
    pub entity_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ProjectMemoryResultKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specification_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<GraphCitation>,
    pub authority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission_basis: Option<ContextAdmissionBasis>,
    pub inclusion_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance_score: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_match: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trusted_for_reuse: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_applicability: Option<RevisionApplicability>,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPackBudgetBreakdown {
    pub max_tokens: usize,
    pub estimated_tokens: usize,
    pub specification_tokens: usize,
    pub active_project_item_tokens: usize,
    pub mounted_reference_tokens: usize,
    pub shared_knowledge_reference_tokens: usize,
    pub diagnostic_and_overhead_tokens: usize,
    pub omitted_or_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPackInspection {
    pub schema_version: u32,
    pub context_pack_id: String,
    pub recompiled_at_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_context_pack_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches_expected_context_pack: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mismatch_warning: Option<String>,
    pub inspection_basis: &'static str,
    pub persisted: bool,
    pub project_id: String,
    pub project_name: String,
    pub artifact_snapshot_id: String,
    pub graph_snapshot_id: String,
    pub task: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_target: Option<AgentEgressTarget>,
    pub authority_precedence: &'static str,
    pub reference_precedence: &'static str,
    pub shared_knowledge_precedence: &'static str,
    pub included_records: Vec<ContextPackIncludedRecord>,
    pub specification_exclusions: Vec<SpecificationCompileExclusion>,
    pub active_project_exclusions: Vec<ContextExclusion>,
    pub mounted_reference_exclusions: Vec<MountedReferenceExclusion>,
    pub shared_knowledge_exclusions: Vec<SharedKnowledgeExclusion>,
    pub egress_exclusions: Vec<ContextEgressExclusion>,
    pub premise_adjudication: ContextPremiseAdjudication,
    pub conflicts: Vec<ProjectMemoryConflict>,
    pub gaps: Vec<ContextGap>,
    pub retrieval: ProjectMemorySearchRetrieval,
    pub revision_freshness: ProjectRevisionFreshness,
    pub budget: ContextPackBudgetBreakdown,
    pub coverage: ContextCompileCoverage,
    pub specification_coverage: SpecificationCompileCoverage,
    pub mounted_reference_coverage: MountedReferenceCoverage,
    pub shared_knowledge_coverage: SharedKnowledgeCoverage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_coverage: Option<ContextEgressCoverage>,
    pub mounted_reference_scopes: Vec<MountedReferenceScope>,
    pub shared_knowledge_scopes: Vec<SharedKnowledgeScope>,
    pub follow_ups: Vec<ContextFollowUp>,
    pub live_source_checked: bool,
    pub source_boundary: &'static str,
    pub instruction_warning: &'static str,
    pub privacy_notice: &'static str,
}

pub fn inspect_context_pack(
    pack: &CompiledContextPack,
    expected_context_pack_id: Option<&str>,
) -> ContextPackInspection {
    let mut included_records = Vec::with_capacity(
        pack.specifications
            .len()
            .saturating_add(pack.items.len())
            .saturating_add(pack.mounted_references.len())
            .saturating_add(pack.shared_knowledge_references.len()),
    );
    for specification in &pack.specifications {
        included_records.push(ContextPackIncludedRecord {
            source: ContextPackRecordSource::Specification,
            entity_id: specification.specification_id.clone(),
            kind: None,
            specification_id: Some(specification.specification_id.clone()),
            relative_path: Some(specification.relative_path.clone()),
            content_hash: Some(specification.content_hash.clone()),
            mount_id: None,
            scope_id: None,
            source_project_id: None,
            session_id: None,
            learning_id: None,
            citation: None,
            authority: specification.authority.to_owned(),
            source_authority: None,
            admission_basis: None,
            inclusion_reason: if specification.exact_match {
                "approved-current-specification-exact-task-match".to_owned()
            } else {
                "approved-current-specification-task-relevant".to_owned()
            },
            relevance_score: Some(specification.relevance_score),
            exact_match: Some(specification.exact_match),
            trusted_for_reuse: None,
            revision_applicability: None,
            estimated_tokens: specification.estimated_tokens,
        });
    }
    for item in &pack.items {
        included_records.push(ContextPackIncludedRecord {
            source: ContextPackRecordSource::ActiveProjectMemory,
            entity_id: item.entity_id.clone(),
            kind: Some(item.kind),
            specification_id: None,
            relative_path: None,
            content_hash: None,
            mount_id: None,
            scope_id: None,
            source_project_id: None,
            session_id: item.session_id.clone(),
            learning_id: item.learning_id.clone(),
            citation: item.citation.clone(),
            authority: authority_label(item.authority).to_owned(),
            source_authority: None,
            admission_basis: Some(item.admission_basis),
            inclusion_reason: format!(
                "admitted-{}-via-{}",
                authority_label(item.authority),
                admission_basis_label(item.admission_basis)
            ),
            relevance_score: None,
            exact_match: None,
            trusted_for_reuse: Some(item.trusted_for_reuse),
            revision_applicability: item.revision_applicability.clone(),
            estimated_tokens: item.estimated_tokens,
        });
    }
    for item in &pack.mounted_references {
        included_records.push(ContextPackIncludedRecord {
            source: ContextPackRecordSource::MountedReference,
            entity_id: item.entity_id.clone(),
            kind: Some(item.kind),
            specification_id: None,
            relative_path: None,
            content_hash: None,
            mount_id: Some(item.mount_id.clone()),
            scope_id: None,
            source_project_id: Some(item.source_project_id.clone()),
            session_id: item.session_id.clone(),
            learning_id: item.learning_id.clone(),
            citation: item.citation.clone(),
            authority: item.authority.to_owned(),
            source_authority: Some(authority_label(item.source_authority).to_owned()),
            admission_basis: Some(item.admission_basis),
            inclusion_reason: format!(
                "admitted-read-only-mounted-reference-{}-via-{}",
                authority_label(item.source_authority),
                admission_basis_label(item.admission_basis)
            ),
            relevance_score: None,
            exact_match: None,
            trusted_for_reuse: Some(item.trusted_for_reuse),
            revision_applicability: item.revision_applicability.clone(),
            estimated_tokens: item.estimated_tokens,
        });
    }
    for item in &pack.shared_knowledge_references {
        included_records.push(ContextPackIncludedRecord {
            source: ContextPackRecordSource::SharedKnowledgeReference,
            entity_id: item.entity_id.clone(),
            kind: Some(item.kind),
            specification_id: None,
            relative_path: None,
            content_hash: None,
            mount_id: None,
            scope_id: Some(item.scope_id.clone()),
            source_project_id: Some(item.source_project_id.clone()),
            session_id: item.session_id.clone(),
            learning_id: item.learning_id.clone(),
            citation: item.citation.clone(),
            authority: item.authority.to_owned(),
            source_authority: Some(authority_label(item.source_authority).to_owned()),
            admission_basis: Some(item.admission_basis),
            inclusion_reason: format!(
                "admitted-read-only-shared-knowledge-{}-via-{}",
                authority_label(item.source_authority),
                admission_basis_label(item.admission_basis)
            ),
            relevance_score: None,
            exact_match: None,
            trusted_for_reuse: Some(item.trusted_for_reuse),
            revision_applicability: item.revision_applicability.clone(),
            estimated_tokens: item.estimated_tokens,
        });
    }

    let specification_tokens = pack
        .specifications
        .iter()
        .map(|item| item.estimated_tokens)
        .sum::<usize>();
    let active_project_item_tokens = pack
        .items
        .iter()
        .map(|item| item.estimated_tokens)
        .sum::<usize>();
    let mounted_reference_tokens = pack
        .mounted_references
        .iter()
        .map(|item| item.estimated_tokens)
        .sum::<usize>();
    let shared_knowledge_reference_tokens = pack
        .shared_knowledge_references
        .iter()
        .map(|item| item.estimated_tokens)
        .sum::<usize>();
    let known_item_tokens = specification_tokens
        .saturating_add(active_project_item_tokens)
        .saturating_add(mounted_reference_tokens)
        .saturating_add(shared_knowledge_reference_tokens);
    let omitted_or_truncated = pack.coverage.search_truncated
        || pack.coverage.source_truncated
        || pack.coverage.omitted_conflicts > 0
        || pack.coverage.omitted_exclusions > 0
        || pack.coverage.omitted_gaps > 0
        || pack.coverage.omitted_follow_ups > 0
        || pack.specification_coverage.omitted_specifications > 0
        || pack.specification_coverage.omitted_exclusions > 0
        || pack.mounted_reference_coverage.omitted_scopes > 0
        || pack.mounted_reference_coverage.omitted_exclusions > 0
        || pack.mounted_reference_coverage.omitted_by_result_limit > 0
        || pack.mounted_reference_coverage.omitted_by_token_budget > 0
        || pack.shared_knowledge_coverage.omitted_scopes > 0
        || pack.shared_knowledge_coverage.omitted_exclusions > 0
        || pack.shared_knowledge_coverage.omitted_by_result_limit > 0
        || pack.shared_knowledge_coverage.omitted_by_token_budget > 0
        || pack
            .egress_coverage
            .as_ref()
            .is_some_and(|coverage| coverage.omitted_exclusions > 0);

    let expected_context_pack_id = expected_context_pack_id.map(str::to_owned);
    let matches_expected_context_pack = expected_context_pack_id
        .as_deref()
        .map(|expected| expected == pack.context_pack_id);
    let mismatch_warning = match matches_expected_context_pack {
        Some(false) => Some(
            "Expected contextPackId does not match the current recompiled pack. This manifest describes the current pack only and does not claim to reconstruct the older supplied pack."
                .to_owned(),
        ),
        _ => None,
    };

    ContextPackInspection {
        schema_version: CONTEXT_PACK_INSPECTOR_SCHEMA_VERSION,
        context_pack_id: pack.context_pack_id.clone(),
        recompiled_at_unix_ms: pack.created_at_unix_ms,
        expected_context_pack_id,
        matches_expected_context_pack,
        mismatch_warning,
        inspection_basis: INSPECTION_BASIS,
        persisted: false,
        project_id: pack.project_id.clone(),
        project_name: pack.project_name.clone(),
        artifact_snapshot_id: pack.artifact_snapshot_id.clone(),
        graph_snapshot_id: pack.graph_snapshot_id.clone(),
        task: pack.task.clone(),
        egress_target: pack.egress_target,
        authority_precedence: pack.authority_precedence,
        reference_precedence: pack.reference_precedence,
        shared_knowledge_precedence: pack.shared_knowledge_precedence,
        included_records,
        specification_exclusions: pack.specification_exclusions.clone(),
        active_project_exclusions: pack.exclusions.clone(),
        mounted_reference_exclusions: pack.mounted_reference_exclusions.clone(),
        shared_knowledge_exclusions: pack.shared_knowledge_exclusions.clone(),
        egress_exclusions: pack.egress_exclusions.clone(),
        premise_adjudication: pack.premise_adjudication.clone(),
        conflicts: pack.conflicts.clone(),
        gaps: pack.gaps.clone(),
        retrieval: pack.retrieval.clone(),
        revision_freshness: pack.revision_freshness.clone(),
        budget: ContextPackBudgetBreakdown {
            max_tokens: pack.max_tokens,
            estimated_tokens: pack.estimated_tokens,
            specification_tokens,
            active_project_item_tokens,
            mounted_reference_tokens,
            shared_knowledge_reference_tokens,
            diagnostic_and_overhead_tokens: pack.estimated_tokens.saturating_sub(known_item_tokens),
            omitted_or_truncated,
        },
        coverage: pack.coverage.clone(),
        specification_coverage: pack.specification_coverage.clone(),
        mounted_reference_coverage: pack.mounted_reference_coverage.clone(),
        shared_knowledge_coverage: pack.shared_knowledge_coverage.clone(),
        egress_coverage: pack.egress_coverage.clone(),
        mounted_reference_scopes: pack.mounted_reference_scopes.clone(),
        shared_knowledge_scopes: pack.shared_knowledge_scopes.clone(),
        follow_ups: pack.follow_ups.clone(),
        live_source_checked: pack.live_source_checked,
        source_boundary: SOURCE_BOUNDARY,
        instruction_warning: INSTRUCTION_WARNING,
        privacy_notice: PRIVACY_NOTICE,
    }
}

fn authority_label(authority: ContextAuthority) -> &'static str {
    match authority {
        ContextAuthority::DirectEvidence => "direct-evidence",
        ContextAuthority::TrustedReviewedKnowledge => "trusted-reviewed-knowledge",
        ContextAuthority::HistoricalProjectMemory => "historical-project-memory",
    }
}

fn admission_basis_label(basis: ContextAdmissionBasis) -> &'static str {
    match basis {
        ContextAdmissionBasis::Lexical => "lexical",
        ContextAdmissionBasis::Semantic => "semantic",
        ContextAdmissionBasis::LexicalAndSemantic => "lexical-and-semantic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compile_project_context_with_registries, ingest_project, initialize_project, CaptureMode,
        ContextCompileLimits, ContextMountRegistry, SpecificationRegistry,
    };
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn inspector_explains_exact_compiled_pack_without_copying_context_text() {
        let root = tempdir().unwrap();
        let project = root.path().join("project");
        let vault = root.path().join("vault");
        let config = root.path().join("config");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(
            project.join("auth.rs"),
            "pub fn auth_guard() { /* inspector_private_body_marker_7d91 */ }\n",
        )
        .unwrap();
        initialize_project(&project, Some("Inspector fixture"), CaptureMode::Structured).unwrap();
        ingest_project(&project, &vault).unwrap();
        let specifications = SpecificationRegistry::at(config.join("specifications-v1.json"));
        let mounts = ContextMountRegistry::at(config.join("context-mounts-v1.json"));
        let pack = compile_project_context_with_registries(
            &project,
            &vault,
            "auth guard",
            ContextCompileLimits {
                max_results: 8,
                max_tokens: 1_500,
            },
            &specifications,
            &mounts,
        )
        .unwrap();

        assert!(pack.context_pack_id.starts_with("cpk_"));
        assert!(pack.created_at_unix_ms > 0);
        assert!(pack
            .items
            .iter()
            .any(|item| { item.excerpt.contains("inspector_private_body_marker_7d91") }));

        let inspection = inspect_context_pack(&pack, Some(&pack.context_pack_id));
        assert_eq!(
            inspection.schema_version,
            CONTEXT_PACK_INSPECTOR_SCHEMA_VERSION
        );
        assert_eq!(inspection.context_pack_id, pack.context_pack_id);
        assert_eq!(inspection.recompiled_at_unix_ms, pack.created_at_unix_ms);
        assert_eq!(inspection.matches_expected_context_pack, Some(true));
        assert!(inspection.mismatch_warning.is_none());
        assert!(!inspection.persisted);
        assert_eq!(inspection.task, "auth guard");
        assert!(inspection
            .included_records
            .iter()
            .any(
                |record| record.source == ContextPackRecordSource::ActiveProjectMemory
                    && record.kind == Some(ProjectMemoryResultKind::Artifact)
                    && record.citation.is_some()
                    && record.inclusion_reason.contains("admitted-direct-evidence")
            ));
        assert_eq!(inspection.budget.max_tokens, pack.max_tokens);
        assert_eq!(inspection.budget.estimated_tokens, pack.estimated_tokens);
        assert!(!inspection.live_source_checked);

        let serialized = serde_json::to_string(&inspection).unwrap();
        assert!(!serialized.contains("inspector_private_body_marker_7d91"));
        assert!(!serialized.contains(project.to_str().unwrap()));
        assert!(!serialized.contains(vault.to_str().unwrap()));
    }

    #[test]
    fn inspector_reports_expected_pack_mismatch_after_source_state_changes() {
        let root = tempdir().unwrap();
        let project = root.path().join("project");
        let vault = root.path().join("vault");
        let config = root.path().join("config");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(project.join("state.txt"), "inspector_state_v1 auth guard\n").unwrap();
        initialize_project(
            &project,
            Some("Inspector mismatch"),
            CaptureMode::Structured,
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let specifications = SpecificationRegistry::at(config.join("specifications-v1.json"));
        let mounts = ContextMountRegistry::at(config.join("context-mounts-v1.json"));
        let limits = ContextCompileLimits {
            max_results: 8,
            max_tokens: 1_500,
        };
        let first = compile_project_context_with_registries(
            &project,
            &vault,
            "auth guard",
            limits,
            &specifications,
            &mounts,
        )
        .unwrap();

        fs::write(project.join("state.txt"), "inspector_state_v2 auth guard\n").unwrap();
        ingest_project(&project, &vault).unwrap();
        let second = compile_project_context_with_registries(
            &project,
            &vault,
            "auth guard",
            limits,
            &specifications,
            &mounts,
        )
        .unwrap();
        assert_ne!(first.artifact_snapshot_id, second.artifact_snapshot_id);
        assert_ne!(first.context_pack_id, second.context_pack_id);

        let inspection = inspect_context_pack(&second, Some(&first.context_pack_id));
        assert_eq!(
            inspection.expected_context_pack_id,
            Some(first.context_pack_id)
        );
        assert_eq!(inspection.matches_expected_context_pack, Some(false));
        assert!(inspection
            .mismatch_warning
            .as_deref()
            .is_some_and(|warning| warning.contains("does not match")));
        assert_eq!(inspection.context_pack_id, second.context_pack_id);
    }
}
