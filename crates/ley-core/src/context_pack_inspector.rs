use crate::{
    AgentEgressTarget, CompiledContextPack, ContextAdmissionBasis, ContextAuthority,
    ContextCompileCoverage, ContextEgressCoverage, ContextEgressExclusion, ContextExclusion,
    ContextFollowUp, ContextGap, ContextPremiseAdjudication, GraphCitation,
    MountedReferenceCoverage, MountedReferenceExclusion, MountedReferenceScope,
    PolicyBundleCompileCoverage, PolicyBundleCompileExclusion, PolicyBundleContext,
    ProjectMemoryConflict, ProjectMemoryResultKind, ProjectMemorySearchRetrieval,
    ProjectRevisionFreshness, RevisionApplicability, SharedKnowledgeCoverage,
    SharedKnowledgeExclusion, SharedKnowledgeScope, SpecificationCompileCoverage,
    SpecificationCompileExclusion,
};
use serde::Serialize;

pub const CONTEXT_PACK_INSPECTOR_SCHEMA_VERSION: u32 = 3;

const INSPECTION_BASIS: &str = "current-recompiled-context-pack-manifest";
const SOURCE_BOUNDARY: &str = "derived-context-pack-inspection";
const INSTRUCTION_WARNING: &str = "This Inspector manifest explains one compiled Ley context pack. It does not make stored text authoritative, does not prove live source is unchanged, and does not reconstruct an older pack when the expected contextPackId differs.";
const PRIVACY_NOTICE: &str = "The Inspector manifest omits included active-project, Specification, Policy Bundle, mounted-reference, and shared-scope text/excerpts. It exposes only diagnostic metadata already represented by the compiled pack: stable IDs, project-relative citations/paths, bundle/mount/scope/source identities, authority/admission reasoning, exclusions, conflicts, retrieval/revision signals, budget composition, coverage, and follow-up handles.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextPackRecordSource {
    Specification,
    PolicyBundleSpecification,
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
    pub bundle_id: Option<String>,
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
    pub policy_bundle_tokens: usize,
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
    pub policy_bundle_precedence: &'static str,
    pub reference_precedence: &'static str,
    pub shared_knowledge_precedence: &'static str,
    pub included_records: Vec<ContextPackIncludedRecord>,
    pub specification_exclusions: Vec<SpecificationCompileExclusion>,
    pub policy_bundle_exclusions: Vec<PolicyBundleCompileExclusion>,
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
    pub policy_bundle_coverage: PolicyBundleCompileCoverage,
    pub mounted_reference_coverage: MountedReferenceCoverage,
    pub shared_knowledge_coverage: SharedKnowledgeCoverage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_coverage: Option<ContextEgressCoverage>,
    pub mounted_reference_scopes: Vec<MountedReferenceScope>,
    pub policy_bundles: Vec<PolicyBundleContext>,
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
            .saturating_add(pack.policy_bundle_policies.len())
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
            bundle_id: None,
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
    for policy in &pack.policy_bundle_policies {
        included_records.push(ContextPackIncludedRecord {
            source: ContextPackRecordSource::PolicyBundleSpecification,
            entity_id: policy.specification_id.clone(),
            kind: None,
            specification_id: Some(policy.specification_id.clone()),
            relative_path: Some(policy.relative_path.clone()),
            content_hash: Some(policy.content_hash.clone()),
            bundle_id: Some(policy.bundle_id.clone()),
            mount_id: None,
            scope_id: Some(policy.scope_id.clone()),
            source_project_id: Some(policy.source_project_id.clone()),
            session_id: None,
            learning_id: None,
            citation: None,
            authority: policy.authority.to_owned(),
            source_authority: None,
            admission_basis: None,
            inclusion_reason: if policy.exact_match {
                "approved-policy-bundle-specification-exact-task-match".to_owned()
            } else {
                "approved-policy-bundle-specification-task-relevant".to_owned()
            },
            relevance_score: Some(policy.relevance_score),
            exact_match: Some(policy.exact_match),
            trusted_for_reuse: None,
            revision_applicability: None,
            estimated_tokens: policy.estimated_tokens,
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
            bundle_id: None,
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
            bundle_id: None,
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
            bundle_id: None,
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
    let policy_bundle_tokens = pack
        .policy_bundle_policies
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
        .saturating_add(policy_bundle_tokens)
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
        || pack.policy_bundle_coverage.omitted_bundles > 0
        || pack.policy_bundle_coverage.omitted_exclusions > 0
        || pack.policy_bundle_coverage.omitted_by_result_limit > 0
        || pack.policy_bundle_coverage.omitted_by_token_budget > 0
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
        policy_bundle_precedence: pack.policy_bundle_precedence,
        reference_precedence: pack.reference_precedence,
        shared_knowledge_precedence: pack.shared_knowledge_precedence,
        included_records,
        specification_exclusions: pack.specification_exclusions.clone(),
        policy_bundle_exclusions: pack.policy_bundle_exclusions.clone(),
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
            policy_bundle_tokens,
            active_project_item_tokens,
            mounted_reference_tokens,
            shared_knowledge_reference_tokens,
            diagnostic_and_overhead_tokens: pack.estimated_tokens.saturating_sub(known_item_tokens),
            omitted_or_truncated,
        },
        coverage: pack.coverage.clone(),
        specification_coverage: pack.specification_coverage.clone(),
        policy_bundle_coverage: pack.policy_bundle_coverage.clone(),
        mounted_reference_coverage: pack.mounted_reference_coverage.clone(),
        shared_knowledge_coverage: pack.shared_knowledge_coverage.clone(),
        egress_coverage: pack.egress_coverage.clone(),
        mounted_reference_scopes: pack.mounted_reference_scopes.clone(),
        policy_bundles: pack.policy_bundles.clone(),
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
        compile_project_context_for_agent_with_registries, compile_project_context_with_registries,
        ingest_project, initialize_project, AgentContextAuthorities, AgentEgressTarget,
        BindingRegistry, CaptureMode, ContextCompileLimits, ContextMountRegistry,
        EgressPolicyRegistry, KnowledgeScopeKind, KnowledgeScopeRegistry, PolicyBundleRegistry,
        PolicyBundleSourceInput, SpecificationRegistry, BINDING_REGISTRY_FILE,
        CONTEXT_MOUNT_REGISTRY_FILE, KNOWLEDGE_SCOPE_REGISTRY_FILE,
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
    fn inspector_attributes_policy_bundle_without_copying_policy_body_or_private_paths() {
        let root = tempdir().unwrap();
        let config = root.path().join("config");
        let active = root.path().join("active");
        let active_vault = root.path().join("active-vault");
        let source = root.path().join("source");
        let source_vault = root.path().join("source-vault");
        for path in [&config, &active, &active_vault, &source, &source_vault] {
            fs::create_dir_all(path).unwrap();
        }
        initialize_project(&active, Some("Active"), CaptureMode::Structured).unwrap();
        initialize_project(&source, Some("Policy source"), CaptureMode::Structured).unwrap();
        fs::write(active.join("README.md"), "inspector policy baseline\n").unwrap();
        let bindings = BindingRegistry::at(config.join(BINDING_REGISTRY_FILE));
        bindings.bind(&active, &active_vault).unwrap();
        bindings.bind(&source, &source_vault).unwrap();
        ingest_project(&active, &active_vault).unwrap();
        fs::create_dir_all(source_vault.join("Specs")).unwrap();
        let private_policy_marker = "inspector_policy_body_marker_49f1";
        fs::write(
            source_vault.join("Specs/Release.md"),
            format!("# Release policy\n\n{private_policy_marker} signed release process\n"),
        )
        .unwrap();

        let specifications = SpecificationRegistry::at(config.join("specifications-v1.json"));
        let source_specification_id = crate::generate_specification_id();
        specifications
            .approve(
                &source,
                &source_vault,
                &source_specification_id,
                "Specs/Release.md",
            )
            .unwrap();
        let mounts = ContextMountRegistry::at(config.join(CONTEXT_MOUNT_REGISTRY_FILE));
        let scopes = KnowledgeScopeRegistry::at(config.join(KNOWLEDGE_SCOPE_REGISTRY_FILE));
        let policy_bundles = PolicyBundleRegistry::at(config.join("policy-bundles-v1.json"));
        let egress = EgressPolicyRegistry::at(config.join("agent-egress-v1.json"));
        let scope = scopes
            .create(
                KnowledgeScopeKind::Team,
                "Release team",
                std::slice::from_ref(&source),
            )
            .unwrap();
        scopes.attach(&active, &scope.scope.scope_id).unwrap();
        let bundle = policy_bundles
            .create(
                &scope.scope.scope_id,
                "Release policy",
                &[PolicyBundleSourceInput {
                    source_project: source.clone(),
                    specification_id: source_specification_id.clone(),
                }],
                &scopes,
                &specifications,
            )
            .unwrap();
        policy_bundles
            .attach(&active, &bundle.bundle.bundle_id, &scopes)
            .unwrap();

        let pack = compile_project_context_for_agent_with_registries(
            &active,
            &active_vault,
            "signed release process",
            ContextCompileLimits {
                max_results: 8,
                max_tokens: 2_000,
            },
            AgentContextAuthorities {
                specifications: &specifications,
                mounts: &mounts,
                knowledge_scopes: &scopes,
                policy_bundles: &policy_bundles,
                egress: &egress,
            },
            AgentEgressTarget::Cloud,
        )
        .unwrap();
        assert_eq!(pack.policy_bundle_policies.len(), 1);
        assert!(pack.policy_bundle_policies[0]
            .source
            .contains(private_policy_marker));

        let inspection = inspect_context_pack(&pack, Some(&pack.context_pack_id));
        assert_eq!(inspection.schema_version, 3);
        assert_eq!(inspection.policy_bundles.len(), 1);
        assert_eq!(inspection.policy_bundle_coverage.returned_policies, 1);
        assert!(inspection.included_records.iter().any(|record| {
            record.source == ContextPackRecordSource::PolicyBundleSpecification
                && record.bundle_id.as_deref() == Some(bundle.bundle.bundle_id.as_str())
                && record.scope_id.as_deref() == Some(scope.scope.scope_id.as_str())
                && record.source_project_id.as_deref()
                    == Some(bundle.bundle.sources[0].source_project_id.as_str())
                && record.specification_id.as_deref() == Some(source_specification_id.as_str())
                && record.relative_path.as_deref() == Some("Specs/Release.md")
                && record.content_hash.as_deref()
                    == Some(bundle.bundle.sources[0].content_hash.as_str())
                && record.authority == "human-intent"
        }));
        assert!(inspection.budget.policy_bundle_tokens > 0);

        let serialized = serde_json::to_string(&inspection).unwrap();
        assert!(!serialized.contains(private_policy_marker));
        assert!(!serialized.contains(active.to_str().unwrap()));
        assert!(!serialized.contains(source.to_str().unwrap()));
        assert!(!serialized.contains(active_vault.to_str().unwrap()));
        assert!(!serialized.contains(source_vault.to_str().unwrap()));
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
