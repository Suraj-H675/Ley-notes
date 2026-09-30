use crate::continuity_store::{
    ContinuityApprovedSourceIssueReason, ContinuityApprovedSourceIssueRecord,
};
use crate::continuity_store::{ContinuityApprovedSourceKind, ContinuityApprovedSourceRecord};
use crate::project_memory_search::lexical_score;
use crate::specification::{
    specification_task_terms, validate_specification_context_limits, ApprovedSpecificationSource,
    ProjectSpecificationsContext, SpecificationContextExclusion,
    SpecificationContextExclusionReason, SpecificationContextItem, SpecificationContextLimits,
    SpecificationEgressCoverage, SpecificationEgressExclusion, TaskSpecificationCandidate,
    TaskSpecificationExclusion, TaskSpecificationExclusionReason, TaskSpecificationScan,
    SPECIFICATION_INSTRUCTION_WARNING,
};
use crate::{
    diagnose_project, evaluate_agent_egress, generate_specification_id, specification_content_hash,
    AgentEgressTarget, ContinuityStore, EgressPolicyRegistry, EgressPolicySnapshot, LeyCoreError,
    SpecificationRegistry,
};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub const MAX_APPROVED_SOURCE_BYTES: u64 = 1_048_576;
pub const MAX_APPROVED_SOURCE_PATH_CHARACTERS: usize = 1_024;
const APPROVED_SOURCE_PRIVACY_NOTICE: &str = "Approved-source authority is stored in Ley's owner-private continuity database. Project-file approvals retain only a project-relative path, exact content hash, and approval time; imported snapshots retain the exact explicitly approved bytes as a bounded private content-addressed snapshot. Repository text cannot grant itself authority.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovedSourceKind {
    ProjectFile,
    ImportedSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovedSourceApproval {
    pub project_id: String,
    pub source_id: String,
    pub source_kind: ApprovedSourceKind,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_relative_path: Option<String>,
    pub content_hash: String,
    pub approved_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovedSourceContent {
    #[serde(flatten)]
    pub approval: ApprovedSourceApproval,
    pub source: String,
    pub source_boundary: &'static str,
    pub authority: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovedSourceState {
    Current,
    Changed,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovedSourceAuthority {
    pub approval: ApprovedSourceApproval,
    pub state: ApprovedSourceState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_content_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovedSourceLegacyIssueReason {
    Changed,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovedSourceLegacyIssue {
    pub project_id: String,
    pub source_id: String,
    pub display_name: String,
    pub approved_content_hash: String,
    pub approved_at_unix_ms: u64,
    pub reason: ApprovedSourceLegacyIssueReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovedSourceAuthorityList {
    pub project_id: String,
    pub sources: Vec<ApprovedSourceAuthority>,
    pub current: usize,
    pub changed: usize,
    pub missing: usize,
    pub legacy_issues: Vec<ApprovedSourceLegacyIssue>,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone)]
pub struct ApprovedSourceRegistry {
    store: ContinuityStore,
}

impl ApprovedSourceRegistry {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        Ok(Self {
            store: ContinuityStore::system_default()?,
        })
    }

    pub fn at(store: ContinuityStore) -> Self {
        Self { store }
    }

    pub(crate) fn store(&self) -> &ContinuityStore {
        &self.store
    }

    pub fn approve_project_file(
        &self,
        project_start: impl AsRef<Path>,
        relative_path: &str,
    ) -> Result<ApprovedSourceApproval, LeyCoreError> {
        approve_project_file_source_with_store(project_start, relative_path, &self.store)
    }

    pub fn reapprove_project_file(
        &self,
        project_start: impl AsRef<Path>,
        source_id: &str,
    ) -> Result<ApprovedSourceApproval, LeyCoreError> {
        reapprove_project_file_source_with_store(project_start, source_id, &self.store)
    }

    pub fn authority(
        &self,
        project_start: impl AsRef<Path>,
    ) -> Result<ApprovedSourceAuthorityList, LeyCoreError> {
        approved_source_authority_with_store(project_start, &self.store)
    }

    pub fn authority_ready(&self, project_start: impl AsRef<Path>) -> Result<bool, LeyCoreError> {
        let project_start = project_start.as_ref();
        self.store.with_approved_source_authority_lock(|| {
            let diagnostic = diagnose_project(project_start)?;
            self.store.register_project(&diagnostic.identity)?;
            self.store
                .approved_source_authority_ready(&diagnostic.identity.project_id)
        })
    }

    pub(crate) fn authority_ready_for_expected_project(
        &self,
        project_start: impl AsRef<Path>,
        expected_project_id: &str,
    ) -> Result<bool, LeyCoreError> {
        let project_start = project_start.as_ref();
        self.store.with_approved_source_authority_lock(|| {
            let diagnostic = diagnose_project(project_start)?;
            if diagnostic.identity.project_id != expected_project_id {
                return Err(LeyCoreError::InvalidProjectIdentity(
                    "source project identity changed during approved-source authority check"
                        .to_owned(),
                ));
            }
            self.store.register_project(&diagnostic.identity)?;
            self.store
                .approved_source_authority_ready(&diagnostic.identity.project_id)
        })
    }

    pub fn migrate_legacy_specifications(
        &self,
        project_start: impl AsRef<Path>,
        legacy_vault: impl AsRef<Path>,
        legacy_registry: &SpecificationRegistry,
    ) -> Result<crate::LegacyApprovedSourceImportSummary, LeyCoreError> {
        crate::import_legacy_approved_sources(
            project_start,
            legacy_vault,
            legacy_registry,
            &self.store,
        )
    }

    pub(crate) fn migrate_legacy_specifications_for_expected_project(
        &self,
        project_start: impl AsRef<Path>,
        legacy_vault: impl AsRef<Path>,
        legacy_registry: &SpecificationRegistry,
        expected_project_id: &str,
    ) -> Result<crate::LegacyApprovedSourceImportSummary, LeyCoreError> {
        crate::continuity_import::import_legacy_approved_sources_for_expected_project(
            project_start.as_ref(),
            legacy_vault.as_ref(),
            legacy_registry,
            &self.store,
            expected_project_id,
        )
    }

    pub fn read(
        &self,
        project_start: impl AsRef<Path>,
        source_id: &str,
    ) -> Result<ApprovedSourceContent, LeyCoreError> {
        read_approved_source_with_store(project_start, source_id, &self.store)
    }

    pub(crate) fn read_specification_compat_for_expected_project(
        &self,
        project_start: impl AsRef<Path>,
        expected_project_id: &str,
        source_id: &str,
    ) -> Result<ApprovedSpecificationSource, LeyCoreError> {
        let project_start = project_start.as_ref();
        self.store.with_approved_source_authority_lock(|| {
            let diagnostic = diagnose_project(project_start)?;
            if diagnostic.identity.project_id != expected_project_id {
                return Err(LeyCoreError::InvalidProjectIdentity(
                    "source project identity changed during approved-source read".to_owned(),
                ));
            }
            let content = read_approved_source_unlocked(&diagnostic, source_id, &self.store)?;
            Ok(compatibility_specification_source(content))
        })
    }

    pub fn revoke(
        &self,
        project_start: impl AsRef<Path>,
        source_id: &str,
    ) -> Result<bool, LeyCoreError> {
        revoke_approved_source_with_store(project_start, source_id, &self.store)
    }

    pub fn context_for_agent_transition(
        &self,
        project_start: impl AsRef<Path>,
        legacy_vault: impl AsRef<Path>,
        legacy_registry: &SpecificationRegistry,
        limits: SpecificationContextLimits,
        egress_registry: &EgressPolicyRegistry,
        target: AgentEgressTarget,
    ) -> Result<ProjectSpecificationsContext, LeyCoreError> {
        validate_specification_context_limits(limits)?;
        let project_start = project_start.as_ref();
        let legacy_vault = legacy_vault.as_ref();
        crate::import_legacy_approved_sources(
            project_start,
            legacy_vault,
            legacy_registry,
            &self.store,
        )?;
        let project_id = diagnose_project(project_start)?.identity.project_id;
        egress_registry.with_transition_snapshot_locked(&self.store, |policies| {
            let project_decision =
                evaluate_agent_egress(policies.project_policy(&project_id), target);
            if !project_decision.allowed {
                return Err(LeyCoreError::AgentEgressDenied {
                    policy: project_decision.policy.to_string(),
                    target: target.to_string(),
                });
            }
            self.store.with_approved_source_authority_lock(|| {
                self.context_for_agent_with_policies(
                    project_start,
                    &project_id,
                    limits,
                    policies,
                    target,
                )
            })
        })
    }

    fn context_for_agent_with_policies(
        &self,
        project_start: &Path,
        project_id: &str,
        limits: SpecificationContextLimits,
        policies: &EgressPolicySnapshot,
        target: AgentEgressTarget,
    ) -> Result<ProjectSpecificationsContext, LeyCoreError> {
        let diagnostic = diagnose_project(project_start)?;
        if diagnostic.identity.project_id != project_id {
            return Err(LeyCoreError::InvalidProjectIdentity(
                "project identity changed during approved-source context read".to_owned(),
            ));
        }
        let records = self.store.approved_sources(project_id)?;
        let total_approved = records.len();
        let mut specifications = Vec::new();
        let mut exclusions = Vec::new();
        let mut egress_exclusions = Vec::new();
        let mut omitted_exclusions = 0usize;
        let mut omitted_egress_exclusions = 0usize;
        let mut text_characters = 0usize;
        let mut current_approved = 0usize;
        let mut changed_approved = 0usize;
        let mut missing_approved = 0usize;

        for record in records {
            let decision = evaluate_agent_egress(
                policies.specification_policy(project_id, &record.source_id),
                target,
            );
            if !decision.allowed {
                if egress_exclusions.len() < 40 {
                    egress_exclusions.push(SpecificationEgressExclusion {
                        specification_id: record.source_id,
                        policy: decision.policy,
                        block_reason: decision
                            .block_reason
                            .expect("blocked egress decision has a reason"),
                    });
                } else {
                    omitted_egress_exclusions += 1;
                }
                continue;
            }

            let display_path = display_path(&record);
            let content =
                match read_approved_source_unlocked(&diagnostic, &record.source_id, &self.store) {
                    Ok(content) => content,
                    Err(LeyCoreError::ApprovedSourceStale { .. }) => {
                        changed_approved += 1;
                        push_compatibility_exclusion(
                            &mut exclusions,
                            &mut omitted_exclusions,
                            SpecificationContextExclusion {
                                specification_id: record.source_id,
                                relative_path: display_path,
                                reason: SpecificationContextExclusionReason::Changed,
                            },
                        );
                        continue;
                    }
                    Err(LeyCoreError::Io { source, .. })
                        if source.kind() == std::io::ErrorKind::NotFound =>
                    {
                        missing_approved += 1;
                        push_compatibility_exclusion(
                            &mut exclusions,
                            &mut omitted_exclusions,
                            SpecificationContextExclusion {
                                specification_id: record.source_id,
                                relative_path: display_path,
                                reason: SpecificationContextExclusionReason::Missing,
                            },
                        );
                        continue;
                    }
                    Err(error) => return Err(error),
                };
            current_approved += 1;
            if specifications.len() >= limits.max_results {
                push_compatibility_exclusion(
                    &mut exclusions,
                    &mut omitted_exclusions,
                    SpecificationContextExclusion {
                        specification_id: content.approval.source_id,
                        relative_path: content
                            .approval
                            .project_relative_path
                            .unwrap_or(content.approval.display_name),
                        reason: SpecificationContextExclusionReason::ResultLimit,
                    },
                );
                continue;
            }
            let characters = content.source.chars().count();
            let relative_path = content
                .approval
                .project_relative_path
                .clone()
                .unwrap_or_else(|| content.approval.display_name.clone());
            if text_characters.saturating_add(characters) > limits.max_characters {
                push_compatibility_exclusion(
                    &mut exclusions,
                    &mut omitted_exclusions,
                    SpecificationContextExclusion {
                        specification_id: content.approval.source_id,
                        relative_path,
                        reason: SpecificationContextExclusionReason::CharacterBudget,
                    },
                );
                continue;
            }
            text_characters += characters;
            specifications.push(SpecificationContextItem {
                specification_id: content.approval.source_id,
                relative_path,
                content_hash: content.approval.content_hash,
                approved_at_unix_ms: content.approval.approved_at_unix_ms,
                source: content.source,
                characters,
                authority: content.authority,
                source_boundary: content.source_boundary,
            });
        }

        let omitted_specifications = total_approved.saturating_sub(specifications.len());
        let blocked_specifications = egress_exclusions.len() + omitted_egress_exclusions;
        Ok(ProjectSpecificationsContext {
            project_id: project_id.to_owned(),
            max_characters: limits.max_characters,
            text_characters,
            specifications,
            exclusions,
            total_approved,
            current_approved,
            changed_approved,
            missing_approved,
            omitted_specifications,
            omitted_exclusions,
            truncated: omitted_specifications > 0
                || omitted_exclusions > 0
                || omitted_egress_exclusions > 0,
            egress_target: Some(target),
            egress_exclusions,
            egress_coverage: Some(SpecificationEgressCoverage {
                target,
                blocked_specifications,
                returned_exclusions: blocked_specifications
                    .saturating_sub(omitted_egress_exclusions),
                omitted_exclusions: omitted_egress_exclusions,
            }),
            project_live_source_checked: false,
            specification_source_revision_checked: true,
            authority: "human-intent",
            source_boundary: "user-approved-specification",
            instruction_warning: SPECIFICATION_INSTRUCTION_WARNING,
            privacy_notice: APPROVED_SOURCE_PRIVACY_NOTICE,
        })
    }

    pub(crate) fn with_task_scan_for_agent_locked<T>(
        &self,
        project_start: impl AsRef<Path>,
        task: &str,
        policies: &EgressPolicySnapshot,
        target: AgentEgressTarget,
        operation: impl FnOnce(
            TaskSpecificationScan,
            Vec<SpecificationEgressExclusion>,
            &ApprovedSourceAuthoritySnapshot<'_>,
        ) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        self.with_task_scan_policy_locked(
            project_start.as_ref(),
            task,
            Some((policies, target)),
            operation,
        )
    }

    pub(crate) fn with_task_scan_locked<T>(
        &self,
        project_start: impl AsRef<Path>,
        task: &str,
        operation: impl FnOnce(TaskSpecificationScan) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        self.with_task_scan_policy_locked(project_start.as_ref(), task, None, |scan, _, _| {
            operation(scan)
        })
    }

    fn with_task_scan_policy_locked<T>(
        &self,
        project_start: &Path,
        task: &str,
        egress: Option<(&EgressPolicySnapshot, AgentEgressTarget)>,
        operation: impl FnOnce(
            TaskSpecificationScan,
            Vec<SpecificationEgressExclusion>,
            &ApprovedSourceAuthoritySnapshot<'_>,
        ) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let normalized_task = task.trim().to_lowercase();
        let terms = specification_task_terms(&normalized_task);
        self.store.with_approved_source_authority_lock(|| {
            let diagnostic = diagnose_project(project_start)?;
            let project_id = diagnostic.identity.project_id.clone();
            let records = self.store.approved_sources(&project_id)?;
            let mut total_approved = 0usize;
            let mut candidates = Vec::new();
            let mut exclusions = Vec::new();
            let mut egress_exclusions = Vec::new();
            let mut current_approved = 0usize;
            let mut changed_approved = 0usize;
            let mut missing_approved = 0usize;
            let mut low_relevance_approved = 0usize;
            let snapshot = ApprovedSourceAuthoritySnapshot { store: &self.store };

            for record in records {
                if let Some((policies, target)) = egress {
                    let decision = evaluate_agent_egress(
                        policies.specification_policy(&project_id, &record.source_id),
                        target,
                    );
                    if !decision.allowed {
                        egress_exclusions.push(SpecificationEgressExclusion {
                            specification_id: record.source_id,
                            policy: decision.policy,
                            block_reason: decision
                                .block_reason
                                .expect("blocked egress decision has a reason"),
                        });
                        continue;
                    }
                }
                total_approved += 1;
                let relative_path = display_path(&record);
                let approved = match snapshot.read_approved_source(
                    &diagnostic.root,
                    &project_id,
                    &record.source_id,
                ) {
                    Ok(approved) => approved,
                    Err(LeyCoreError::ApprovedSourceStale { .. }) => {
                        changed_approved += 1;
                        exclusions.push(TaskSpecificationExclusion {
                            specification_id: record.source_id,
                            relative_path,
                            reason: TaskSpecificationExclusionReason::Changed,
                        });
                        continue;
                    }
                    Err(LeyCoreError::Io { source, .. })
                        if source.kind() == std::io::ErrorKind::NotFound =>
                    {
                        missing_approved += 1;
                        exclusions.push(TaskSpecificationExclusion {
                            specification_id: record.source_id,
                            relative_path,
                            reason: TaskSpecificationExclusionReason::Missing,
                        });
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                current_approved += 1;
                let searchable = format!("{}\n{}", approved.relative_path, approved.source);
                let (score, exact_match) = lexical_score(&searchable, &normalized_task, &terms);
                if score == 0 {
                    low_relevance_approved += 1;
                    exclusions.push(TaskSpecificationExclusion {
                        specification_id: approved.specification_id,
                        relative_path: approved.relative_path,
                        reason: TaskSpecificationExclusionReason::LowRelevance,
                    });
                    continue;
                }
                candidates.push(TaskSpecificationCandidate {
                    source: approved,
                    lexical_score: score,
                    exact_match,
                });
            }

            candidates.sort_by(|left, right| {
                right
                    .exact_match
                    .cmp(&left.exact_match)
                    .then_with(|| right.lexical_score.cmp(&left.lexical_score))
                    .then_with(|| {
                        right
                            .source
                            .approved_at_unix_ms
                            .cmp(&left.source.approved_at_unix_ms)
                    })
                    .then_with(|| {
                        left.source
                            .specification_id
                            .cmp(&right.source.specification_id)
                    })
            });
            operation(
                TaskSpecificationScan {
                    project_id,
                    candidates,
                    exclusions,
                    total_approved,
                    current_approved,
                    changed_approved,
                    missing_approved,
                    low_relevance_approved,
                },
                egress_exclusions,
                &snapshot,
            )
        })
    }
}

pub(crate) struct ApprovedSourceAuthoritySnapshot<'a> {
    store: &'a ContinuityStore,
}

impl ApprovedSourceAuthoritySnapshot<'_> {
    pub(crate) fn store(&self) -> &ContinuityStore {
        self.store
    }

    pub(crate) fn read_approved_source(
        &self,
        project_start: impl AsRef<Path>,
        expected_project_id: &str,
        source_id: &str,
    ) -> Result<ApprovedSpecificationSource, LeyCoreError> {
        let diagnostic = diagnose_project(project_start)?;
        if diagnostic.identity.project_id != expected_project_id {
            return Err(LeyCoreError::InvalidProjectIdentity(
                "source project identity changed during approved-source read".to_owned(),
            ));
        }
        let content = read_approved_source_unlocked(&diagnostic, source_id, self.store)?;
        Ok(compatibility_specification_source(content))
    }
}

pub fn approve_project_file_source(
    project_start: impl AsRef<Path>,
    relative_path: &str,
) -> Result<ApprovedSourceApproval, LeyCoreError> {
    let store = ContinuityStore::system_default()?;
    approve_project_file_source_with_store(project_start, relative_path, &store)
}

pub fn reapprove_project_file_source(
    project_start: impl AsRef<Path>,
    source_id: &str,
) -> Result<ApprovedSourceApproval, LeyCoreError> {
    let store = ContinuityStore::system_default()?;
    reapprove_project_file_source_with_store(project_start, source_id, &store)
}

pub fn list_approved_sources(
    project_start: impl AsRef<Path>,
) -> Result<Vec<ApprovedSourceApproval>, LeyCoreError> {
    let store = ContinuityStore::system_default()?;
    list_approved_sources_with_store(project_start, &store)
}

pub fn approved_source_authority(
    project_start: impl AsRef<Path>,
) -> Result<ApprovedSourceAuthorityList, LeyCoreError> {
    let store = ContinuityStore::system_default()?;
    approved_source_authority_with_store(project_start, &store)
}

pub fn read_approved_source(
    project_start: impl AsRef<Path>,
    source_id: &str,
) -> Result<ApprovedSourceContent, LeyCoreError> {
    let store = ContinuityStore::system_default()?;
    read_approved_source_with_store(project_start, source_id, &store)
}

pub fn revoke_approved_source(
    project_start: impl AsRef<Path>,
    source_id: &str,
) -> Result<bool, LeyCoreError> {
    let store = ContinuityStore::system_default()?;
    revoke_approved_source_with_store(project_start, source_id, &store)
}

pub(crate) fn approve_project_file_source_with_store(
    project_start: impl AsRef<Path>,
    relative_path: &str,
    store: &ContinuityStore,
) -> Result<ApprovedSourceApproval, LeyCoreError> {
    let project_start = project_start.as_ref();
    let relative_path = validate_and_normalize_project_relative_path(relative_path)?;
    store.with_approved_source_authority_lock(|| {
        let diagnostic = diagnose_project(project_start)?;
        store.register_project(&diagnostic.identity)?;
        let bytes = read_stable_project_source_bytes(&diagnostic.root, &relative_path)?;
        std::str::from_utf8(&bytes).map_err(|_| {
            LeyCoreError::InvalidApprovedSourceRequest(format!(
                "approved source {relative_path:?} must be UTF-8 text"
            ))
        })?;
        let content_hash = specification_content_hash(&bytes);
        let source_id = store
            .approved_project_file_for_path(&diagnostic.identity.project_id, &relative_path)?
            .map(|record| record.source_id)
            .unwrap_or_else(generate_specification_id);
        let record = store.approve_project_file_source_unlocked(
            &diagnostic.identity.project_id,
            &source_id,
            &relative_path,
            &relative_path,
            &content_hash,
            crate::unix_time_ms(),
        )?;
        Ok(approval_from_record(record))
    })
}

pub(crate) fn reapprove_project_file_source_with_store(
    project_start: impl AsRef<Path>,
    source_id: &str,
    store: &ContinuityStore,
) -> Result<ApprovedSourceApproval, LeyCoreError> {
    let project_start = project_start.as_ref();
    store.with_approved_source_authority_lock(|| {
        let diagnostic = diagnose_project(project_start)?;
        let record = store
            .approved_source(&diagnostic.identity.project_id, source_id)?
            .ok_or_else(|| LeyCoreError::ApprovedSourceNotFound(source_id.to_owned()))?;
        if record.source_kind != ContinuityApprovedSourceKind::ProjectFile {
            return Err(LeyCoreError::InvalidApprovedSourceRequest(format!(
                "approved source {source_id} is an immutable imported snapshot"
            )));
        }
        let relative_path = record.project_relative_path.as_deref().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(format!(
                "project-file approved source {source_id} has no project-relative path"
            ))
        })?;
        let bytes = read_stable_project_source_bytes(&diagnostic.root, relative_path)?;
        std::str::from_utf8(&bytes).map_err(|_| {
            LeyCoreError::InvalidApprovedSourceRequest(format!(
                "approved source {relative_path:?} must be UTF-8 text"
            ))
        })?;
        let updated = store.approve_project_file_source_unlocked(
            &diagnostic.identity.project_id,
            source_id,
            &record.display_name,
            relative_path,
            &specification_content_hash(&bytes),
            crate::unix_time_ms(),
        )?;
        Ok(approval_from_record(updated))
    })
}

pub(crate) fn list_approved_sources_with_store(
    project_start: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<Vec<ApprovedSourceApproval>, LeyCoreError> {
    let project_start = project_start.as_ref();
    store.with_approved_source_authority_lock(|| {
        let diagnostic = diagnose_project(project_start)?;
        store
            .approved_sources(&diagnostic.identity.project_id)?
            .into_iter()
            .map(|record| Ok(approval_from_record(record)))
            .collect()
    })
}

pub(crate) fn approved_source_authority_with_store(
    project_start: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<ApprovedSourceAuthorityList, LeyCoreError> {
    let project_start = project_start.as_ref();
    store.with_approved_source_authority_lock(|| {
        let diagnostic = diagnose_project(project_start)?;
        let records = store.approved_sources(&diagnostic.identity.project_id)?;
        let mut sources = Vec::with_capacity(records.len());
        let mut current = 0usize;
        let mut changed = 0usize;
        let mut missing = 0usize;
        for record in records {
            let (state, current_content_hash) = match record.source_kind {
                ContinuityApprovedSourceKind::ImportedSnapshot => {
                    let bytes = store
                        .approved_snapshot_bytes(
                            &diagnostic.identity.project_id,
                            &record.source_id,
                        )?
                        .ok_or_else(|| {
                            LeyCoreError::InvalidContinuityStore(format!(
                                "imported approved source {} has no snapshot bytes",
                                record.source_id
                            ))
                        })?;
                    let hash = specification_content_hash(&bytes);
                    if hash != record.content_hash || std::str::from_utf8(&bytes).is_err() {
                        return Err(LeyCoreError::InvalidContinuityStore(format!(
                            "imported approved source {} failed snapshot validation",
                            record.source_id
                        )));
                    }
                    current += 1;
                    (ApprovedSourceState::Current, Some(hash))
                }
                ContinuityApprovedSourceKind::ProjectFile => {
                    let relative_path =
                        record.project_relative_path.as_deref().ok_or_else(|| {
                            LeyCoreError::InvalidContinuityStore(format!(
                                "project-file approved source {} has no project-relative path",
                                record.source_id
                            ))
                        })?;
                    match read_stable_project_source_bytes(&diagnostic.root, relative_path) {
                        Ok(bytes) => {
                            let hash = specification_content_hash(&bytes);
                            if hash == record.content_hash {
                                current += 1;
                                (ApprovedSourceState::Current, Some(hash))
                            } else {
                                changed += 1;
                                (ApprovedSourceState::Changed, Some(hash))
                            }
                        }
                        Err(LeyCoreError::Io { source, .. })
                            if source.kind() == std::io::ErrorKind::NotFound =>
                        {
                            missing += 1;
                            (ApprovedSourceState::Missing, None)
                        }
                        Err(error) => return Err(error),
                    }
                }
            };
            sources.push(ApprovedSourceAuthority {
                approval: approval_from_record(record),
                state,
                current_content_hash,
            });
        }
        let legacy_issues = store
            .approved_source_issues(&diagnostic.identity.project_id)?
            .into_iter()
            .map(legacy_issue_from_record)
            .collect();
        Ok(ApprovedSourceAuthorityList {
            project_id: diagnostic.identity.project_id.clone(),
            sources,
            current,
            changed,
            missing,
            legacy_issues,
            privacy_notice: APPROVED_SOURCE_PRIVACY_NOTICE,
        })
    })
}

pub(crate) fn read_approved_source_with_store(
    project_start: impl AsRef<Path>,
    source_id: &str,
    store: &ContinuityStore,
) -> Result<ApprovedSourceContent, LeyCoreError> {
    let project_start = project_start.as_ref();
    store.with_approved_source_authority_lock(|| {
        let diagnostic = diagnose_project(project_start)?;
        read_approved_source_unlocked(&diagnostic, source_id, store)
    })
}

fn read_approved_source_unlocked(
    diagnostic: &crate::ProjectDiagnostic,
    source_id: &str,
    store: &ContinuityStore,
) -> Result<ApprovedSourceContent, LeyCoreError> {
    let record = store
        .approved_source(&diagnostic.identity.project_id, source_id)?
        .ok_or_else(|| LeyCoreError::ApprovedSourceNotFound(source_id.to_owned()))?;
    let (bytes, source_boundary) = match record.source_kind {
        ContinuityApprovedSourceKind::ProjectFile => {
            let relative_path = record.project_relative_path.as_deref().ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "project-file approved source {source_id} has no project-relative path"
                ))
            })?;
            let bytes = read_stable_project_source_bytes(&diagnostic.root, relative_path)?;
            let current_hash = specification_content_hash(&bytes);
            if current_hash != record.content_hash {
                return Err(LeyCoreError::ApprovedSourceStale {
                    source_id: source_id.to_owned(),
                    path: relative_path.to_owned(),
                });
            }
            (bytes, "approved-project-file")
        }
        ContinuityApprovedSourceKind::ImportedSnapshot => {
            let bytes = store
                .approved_snapshot_bytes(&diagnostic.identity.project_id, source_id)?
                .ok_or_else(|| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "imported approved source {source_id} has no snapshot bytes"
                    ))
                })?;
            if specification_content_hash(&bytes) != record.content_hash {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "imported approved source {source_id} snapshot hash mismatch"
                )));
            }
            (bytes, "imported-approved-snapshot")
        }
    };
    let source = String::from_utf8(bytes).map_err(|_| {
        LeyCoreError::InvalidContinuityStore(format!(
            "approved source {source_id} contains non-UTF-8 bytes"
        ))
    })?;
    Ok(ApprovedSourceContent {
        approval: approval_from_record(record),
        source,
        source_boundary,
        authority: "human-intent",
    })
}

pub(crate) fn revoke_approved_source_with_store(
    project_start: impl AsRef<Path>,
    source_id: &str,
    store: &ContinuityStore,
) -> Result<bool, LeyCoreError> {
    let project_start = project_start.as_ref();
    store.with_approved_source_authority_lock(|| {
        let diagnostic = diagnose_project(project_start)?;
        store.revoke_approved_source_unlocked(&diagnostic.identity.project_id, source_id)
    })
}

fn approval_from_record(record: ContinuityApprovedSourceRecord) -> ApprovedSourceApproval {
    ApprovedSourceApproval {
        project_id: record.project_id,
        source_id: record.source_id,
        source_kind: match record.source_kind {
            ContinuityApprovedSourceKind::ProjectFile => ApprovedSourceKind::ProjectFile,
            ContinuityApprovedSourceKind::ImportedSnapshot => ApprovedSourceKind::ImportedSnapshot,
        },
        display_name: record.display_name,
        project_relative_path: record.project_relative_path,
        content_hash: record.content_hash,
        approved_at_unix_ms: record.approved_at_unix_ms,
    }
}

fn compatibility_specification_source(
    content: ApprovedSourceContent,
) -> ApprovedSpecificationSource {
    ApprovedSpecificationSource {
        project_id: content.approval.project_id,
        specification_id: content.approval.source_id,
        relative_path: content
            .approval
            .project_relative_path
            .unwrap_or(content.approval.display_name),
        content_hash: content.approval.content_hash,
        approved_at_unix_ms: content.approval.approved_at_unix_ms,
        source: content.source,
        source_boundary: content.source_boundary,
        authority: content.authority,
    }
}

fn display_path(record: &ContinuityApprovedSourceRecord) -> String {
    record
        .project_relative_path
        .clone()
        .unwrap_or_else(|| record.display_name.clone())
}

fn push_compatibility_exclusion(
    exclusions: &mut Vec<SpecificationContextExclusion>,
    omitted_exclusions: &mut usize,
    exclusion: SpecificationContextExclusion,
) {
    if exclusions.len() < 40 {
        exclusions.push(exclusion);
    } else {
        *omitted_exclusions += 1;
    }
}

fn legacy_issue_from_record(
    record: ContinuityApprovedSourceIssueRecord,
) -> ApprovedSourceLegacyIssue {
    ApprovedSourceLegacyIssue {
        project_id: record.project_id,
        source_id: record.source_id,
        display_name: record.display_name,
        approved_content_hash: record.approved_content_hash,
        approved_at_unix_ms: record.approved_at_unix_ms,
        reason: match record.reason {
            ContinuityApprovedSourceIssueReason::Changed => {
                ApprovedSourceLegacyIssueReason::Changed
            }
            ContinuityApprovedSourceIssueReason::Missing => {
                ApprovedSourceLegacyIssueReason::Missing
            }
            ContinuityApprovedSourceIssueReason::Invalid => {
                ApprovedSourceLegacyIssueReason::Invalid
            }
        },
    }
}

pub(crate) fn validate_and_normalize_project_relative_path(
    value: &str,
) -> Result<String, LeyCoreError> {
    if value.is_empty()
        || value.chars().count() > MAX_APPROVED_SOURCE_PATH_CHARACTERS
        || value.contains('\\')
    {
        return Err(LeyCoreError::InvalidApprovedSourceRequest(format!(
            "project-relative approved-source path must contain 1 to {MAX_APPROVED_SOURCE_PATH_CHARACTERS} portable characters"
        )));
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(LeyCoreError::InvalidApprovedSourceRequest(
            "approved-source path must be project-relative".to_owned(),
        ));
    }
    let mut segments = Vec::new();
    for component in path.components() {
        let Component::Normal(segment) = component else {
            return Err(LeyCoreError::InvalidApprovedSourceRequest(
                "approved-source path contains an unsafe segment".to_owned(),
            ));
        };
        let segment = segment.to_str().ok_or_else(|| {
            LeyCoreError::InvalidApprovedSourceRequest(
                "approved-source path must be valid UTF-8".to_owned(),
            )
        })?;
        if segment.is_empty() || segment.chars().any(char::is_control) {
            return Err(LeyCoreError::InvalidApprovedSourceRequest(
                "approved-source path contains an invalid segment".to_owned(),
            ));
        }
        if segment == ".ley" || segment == ".git" {
            return Err(LeyCoreError::InvalidApprovedSourceRequest(
                "approved-source path must not point inside .ley or .git".to_owned(),
            ));
        }
        segments.push(segment);
    }
    if segments.is_empty() {
        return Err(LeyCoreError::InvalidApprovedSourceRequest(
            "approved-source path is empty".to_owned(),
        ));
    }
    Ok(segments.join("/"))
}

fn read_stable_project_source_bytes(
    project_root: &Path,
    relative_path: &str,
) -> Result<Vec<u8>, LeyCoreError> {
    let first = read_project_source_bytes_once(project_root, relative_path)?;
    let second = read_project_source_bytes_once(project_root, relative_path)?;
    if first != second {
        return Err(LeyCoreError::InvalidApprovedSourceRequest(format!(
            "approved source {relative_path:?} changed while Ley was reading it; retry"
        )));
    }
    Ok(first)
}

fn read_project_source_bytes_once(
    project_root: &Path,
    relative_path: &str,
) -> Result<Vec<u8>, LeyCoreError> {
    let root = Dir::open_ambient_dir(project_root, ambient_authority()).map_err(|source| {
        LeyCoreError::Io {
            path: project_root.to_path_buf(),
            source,
        }
    })?;
    let components = Path::new(relative_path)
        .components()
        .map(|component| match component {
            Component::Normal(segment) => Ok(segment.to_os_string()),
            _ => Err(LeyCoreError::InvalidApprovedSourceRequest(
                "approved-source path contains an unsafe segment".to_owned(),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (file_name, directories) = components.split_last().ok_or_else(|| {
        LeyCoreError::InvalidApprovedSourceRequest("approved-source path is empty".to_owned())
    })?;
    let mut directory = root;
    let mut traversed = PathBuf::new();
    for segment in directories {
        traversed.push(segment);
        directory = directory
            .open_dir_nofollow(segment)
            .map_err(|source| LeyCoreError::Io {
                path: traversed.clone(),
                source,
            })?;
    }
    traversed.push(file_name);
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = directory
        .open_with(file_name, &options)
        .map_err(|source| LeyCoreError::Io {
            path: traversed.clone(),
            source,
        })?;
    let metadata = file.metadata().map_err(|source| LeyCoreError::Io {
        path: traversed.clone(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(LeyCoreError::UnsafeProjectLayout(traversed));
    }
    if metadata.len() > MAX_APPROVED_SOURCE_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: traversed,
            limit_bytes: MAX_APPROVED_SOURCE_BYTES,
        });
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(MAX_APPROVED_SOURCE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| LeyCoreError::Io {
            path: PathBuf::from(relative_path),
            source,
        })?;
    if bytes.len() as u64 > MAX_APPROVED_SOURCE_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: PathBuf::from(relative_path),
            limit_bytes: MAX_APPROVED_SOURCE_BYTES,
        });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{initialize_project, CaptureMode};
    use std::fs;
    use tempfile::tempdir;

    fn ready_store(project: &Path) -> ContinuityStore {
        let base = project.parent().unwrap();
        let private = base.join("private");
        fs::create_dir_all(&private).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let store = ContinuityStore::at(private.join("continuity.sqlite3"));
        let diagnostic = diagnose_project(project).unwrap();
        store.register_project(&diagnostic.identity).unwrap();
        store
            .import_legacy_approved_source_authority(&diagnostic.identity.project_id, &[], &[])
            .unwrap();
        store
    }

    #[test]
    fn project_file_approval_is_exact_reapprovable_and_revocable() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        fs::create_dir(&project).unwrap();
        initialize_project(&project, Some("Approved source"), CaptureMode::Structured).unwrap();
        fs::write(project.join("AGENTS.md"), "# Rules\nUse exact source.\n").unwrap();
        let store = ready_store(&project);

        let approved =
            approve_project_file_source_with_store(&project, "AGENTS.md", &store).unwrap();
        assert_eq!(approved.source_kind, ApprovedSourceKind::ProjectFile);
        assert_eq!(approved.project_relative_path.as_deref(), Some("AGENTS.md"));
        let first_id = approved.source_id.clone();
        let read = read_approved_source_with_store(&project, &first_id, &store).unwrap();
        assert_eq!(read.source, "# Rules\nUse exact source.\n");
        assert_eq!(read.source_boundary, "approved-project-file");

        fs::write(project.join("AGENTS.md"), "# Rules\nUse changed source.\n").unwrap();
        assert!(matches!(
            read_approved_source_with_store(&project, &first_id, &store),
            Err(LeyCoreError::ApprovedSourceStale { source_id, path })
                if source_id == first_id && path == "AGENTS.md"
        ));
        let reapproved =
            reapprove_project_file_source_with_store(&project, &first_id, &store).unwrap();
        assert_eq!(reapproved.source_id, first_id);
        assert_ne!(reapproved.content_hash, approved.content_hash);
        assert_eq!(
            read_approved_source_with_store(&project, &first_id, &store)
                .unwrap()
                .source,
            "# Rules\nUse changed source.\n"
        );

        let replay = approve_project_file_source_with_store(&project, "AGENTS.md", &store).unwrap();
        assert_eq!(replay.source_id, first_id);
        assert_eq!(
            list_approved_sources_with_store(&project, &store)
                .unwrap()
                .len(),
            1
        );
        assert!(revoke_approved_source_with_store(&project, &first_id, &store).unwrap());
        assert!(!revoke_approved_source_with_store(&project, &first_id, &store).unwrap());
        assert!(matches!(
            read_approved_source_with_store(&project, &first_id, &store),
            Err(LeyCoreError::ApprovedSourceNotFound(id)) if id == first_id
        ));
    }

    #[cfg(unix)]
    #[test]
    fn project_file_approval_rejects_escape_internal_paths_and_symlinks() {
        use std::os::unix::fs::symlink;

        let base = tempdir().unwrap();
        let project = base.path().join("project");
        fs::create_dir(&project).unwrap();
        initialize_project(
            &project,
            Some("Approved source security"),
            CaptureMode::Structured,
        )
        .unwrap();
        let store = ready_store(&project);
        fs::write(base.path().join("outside.md"), "outside\n").unwrap();
        symlink(base.path().join("outside.md"), project.join("linked.md")).unwrap();
        fs::create_dir(project.join("nested")).unwrap();
        symlink(base.path(), project.join("nested/link")).unwrap();

        for path in [
            "../outside.md",
            ".ley/project.json",
            ".git/config",
            "linked.md",
            "nested/link/outside.md",
        ] {
            assert!(
                approve_project_file_source_with_store(&project, path, &store).is_err(),
                "{path}"
            );
        }
        assert!(list_approved_sources_with_store(&project, &store)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn native_approval_fails_closed_before_legacy_authority_migration() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        fs::create_dir(&project).unwrap();
        let initialized =
            initialize_project(&project, Some("Pending source"), CaptureMode::Structured).unwrap();
        fs::write(project.join("AGENTS.md"), "# pending\n").unwrap();
        let private = base.path().join("private");
        fs::create_dir(&private).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let store = ContinuityStore::at(private.join("continuity.sqlite3"));
        store.register_project(&initialized.identity).unwrap();
        assert!(matches!(
            approve_project_file_source_with_store(&project, "AGENTS.md", &store),
            Err(LeyCoreError::ApprovedSourceAuthorityMigrationPending { project_id })
                if project_id == initialized.identity.project_id
        ));
    }

    #[test]
    fn imported_snapshot_is_immutable_authority_and_does_not_relink_to_project_file() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        fs::create_dir(&project).unwrap();
        let initialized = initialize_project(
            &project,
            Some("Imported approved source"),
            CaptureMode::Structured,
        )
        .unwrap();
        let private = base.path().join("private");
        fs::create_dir(&private).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let store = ContinuityStore::at(private.join("continuity.sqlite3"));
        store.register_project(&initialized.identity).unwrap();

        let source_id = "spec_55555555555555555555555555555555";
        let bytes = b"# Migrated\nlegacy approved bytes\n".to_vec();
        let content_hash = specification_content_hash(&bytes);
        store
            .import_legacy_approved_source_authority(
                &initialized.identity.project_id,
                &[
                    crate::continuity_store::ContinuityApprovedSourceSnapshotInput {
                        source_id: source_id.to_owned(),
                        display_name: "Legacy/Intent.md".to_owned(),
                        content_hash: content_hash.clone(),
                        approved_at_unix_ms: 123,
                        content_bytes: bytes,
                    },
                ],
                &[],
            )
            .unwrap();

        let read = read_approved_source_with_store(&project, source_id, &store).unwrap();
        assert_eq!(
            read.approval.source_kind,
            ApprovedSourceKind::ImportedSnapshot
        );
        assert_eq!(read.approval.content_hash, content_hash);
        assert_eq!(read.source, "# Migrated\nlegacy approved bytes\n");
        assert_eq!(read.source_boundary, "imported-approved-snapshot");

        fs::write(project.join("AGENTS.md"), "# replacement\n").unwrap();
        assert!(matches!(
            store.approve_project_file_source(
                &initialized.identity.project_id,
                source_id,
                "AGENTS.md",
                "AGENTS.md",
                &specification_content_hash(b"# replacement\n"),
                124,
            ),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("cannot be silently relinked")
        ));
        assert!(matches!(
            reapprove_project_file_source_with_store(&project, source_id, &store),
            Err(LeyCoreError::InvalidApprovedSourceRequest(message))
                if message.contains("immutable imported snapshot")
        ));
    }
}
