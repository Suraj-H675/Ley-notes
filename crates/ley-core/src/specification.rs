use crate::egress_policy::EgressPolicySnapshot;
use crate::project_memory_search::lexical_score;
use crate::{
    default_binding_registry_path, diagnose_project, evaluate_agent_egress, validate_project_id,
    AgentEgressBlockReason, AgentEgressPolicy, AgentEgressTarget, EgressPolicyRegistry,
    LeyCoreError, METADATA_FILE_LIMIT_BYTES,
};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions as StdOpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const SPECIFICATION_REGISTRY_FILE: &str = "specifications-v1.json";
const SPECIFICATION_REGISTRY_LOCK_FILE: &str = "specifications-v1.lock";
pub const SPECIFICATION_REGISTRY_SCHEMA_VERSION: u32 = 1;
pub const MAX_SPECIFICATION_BYTES: u64 = 1_048_576;
pub const MAX_SPECIFICATION_PATH_CHARACTERS: usize = 1_024;
pub const MAX_SPECIFICATION_APPROVALS_PER_PROJECT: usize = 64;
pub const MAX_SPECIFICATION_ACCEPTANCE_CRITERIA: usize = 64;

const PRIVACY_NOTICE: &str = "Specification approval stores only project/specification identity, a vault-relative Markdown path, an exact content hash, and approval time. Specification text remains in the user's ordinary vault note.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpecificationApprovalState {
    Current,
    Changed,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpecificationApproval {
    pub project_id: String,
    pub specification_id: String,
    pub relative_path: String,
    pub content_hash: String,
    pub approved_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificationAuthority {
    pub approval: SpecificationApproval,
    pub state: SpecificationApprovalState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificationAuthorityList {
    pub project_id: String,
    pub specifications: Vec<SpecificationAuthority>,
    pub current: usize,
    pub changed: usize,
    pub missing: usize,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedSpecificationSource {
    pub project_id: String,
    pub specification_id: String,
    pub relative_path: String,
    pub content_hash: String,
    pub approved_at_unix_ms: u64,
    pub source: String,
    pub source_boundary: &'static str,
    pub authority: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpecificationAcceptanceCriteriaState {
    Available,
    Empty,
    Absent,
    Ambiguous,
    OmittedLimit,
    OmittedBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificationAcceptanceCriterion {
    pub criterion_id: String,
    pub text: String,
    pub start_line: u64,
    pub end_line: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificationAcceptanceCriteria {
    pub state: SpecificationAcceptanceCriteriaState,
    pub total_criteria: usize,
    pub returned_criteria: usize,
    pub omitted_criteria: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_line: Option<u64>,
    pub criteria: Vec<SpecificationAcceptanceCriterion>,
    pub source_revision_bound: bool,
    pub status_interpreted: bool,
    pub persisted: bool,
    pub authority: &'static str,
    pub source_boundary: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskSpecificationCandidate {
    pub source: ApprovedSpecificationSource,
    pub lexical_score: u32,
    pub exact_match: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSpecificationExclusionReason {
    Changed,
    Missing,
    LowRelevance,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskSpecificationExclusion {
    pub specification_id: String,
    pub relative_path: String,
    pub reason: TaskSpecificationExclusionReason,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskSpecificationScan {
    pub project_id: String,
    pub candidates: Vec<TaskSpecificationCandidate>,
    pub exclusions: Vec<TaskSpecificationExclusion>,
    pub total_approved: usize,
    pub current_approved: usize,
    pub changed_approved: usize,
    pub missing_approved: usize,
    pub low_relevance_approved: usize,
}

pub(crate) struct SpecificationAuthoritySnapshot<'a> {
    approvals: &'a BTreeMap<String, BTreeMap<String, SpecificationApprovalEntry>>,
}

impl SpecificationAuthoritySnapshot<'_> {
    pub(crate) fn read_approved_source(
        &self,
        project_id: &str,
        vault: &Path,
        specification_id: &str,
    ) -> Result<ApprovedSpecificationSource, LeyCoreError> {
        validate_project_id(project_id)
            .map_err(|error| LeyCoreError::InvalidSpecificationRequest(error.to_string()))?;
        validate_specification_id(specification_id)
            .map_err(LeyCoreError::InvalidSpecificationRequest)?;
        let entry = self
            .approvals
            .get(project_id)
            .and_then(|approvals| approvals.get(specification_id))
            .ok_or_else(|| LeyCoreError::SpecificationNotApproved(specification_id.to_owned()))?;
        let source = read_stable_specification_bytes(vault, &entry.relative_path)?;
        let content_hash = specification_content_hash(&source);
        if content_hash != entry.content_hash {
            return Err(LeyCoreError::SpecificationApprovalStale {
                specification_id: specification_id.to_owned(),
                path: entry.relative_path.clone(),
            });
        }
        let source = String::from_utf8(source).map_err(|_| {
            LeyCoreError::InvalidSpecificationRequest(
                "approved Specification Markdown must be valid UTF-8".to_owned(),
            )
        })?;
        Ok(ApprovedSpecificationSource {
            project_id: project_id.to_owned(),
            specification_id: specification_id.to_owned(),
            relative_path: entry.relative_path.clone(),
            content_hash,
            approved_at_unix_ms: entry.approved_at_unix_ms,
            source,
            source_boundary: "user-approved-specification",
            authority: "human-intent",
        })
    }
}

pub const DEFAULT_SPECIFICATION_CONTEXT_RESULTS: usize = 8;
pub const MAX_SPECIFICATION_CONTEXT_RESULTS: usize = 20;
pub const DEFAULT_SPECIFICATION_CONTEXT_CHARACTERS: usize = 16_000;
pub const MIN_SPECIFICATION_CONTEXT_CHARACTERS: usize = 1_000;
pub const MAX_SPECIFICATION_CONTEXT_CHARACTERS: usize = 64_000;
const MAX_SPECIFICATION_CONTEXT_EXCLUSIONS: usize = 40;
const MAX_SPECIFICATION_EGRESS_EXCLUSIONS: usize = 40;

const SPECIFICATION_INSTRUCTION_WARNING: &str = "Approved Specifications express user-authorized human intent for the exact approved Markdown revision. They are not historical evidence, and they do not grant filesystem, network, tool, review, or write permissions. Changed or missing revisions are excluded until the user approves them again.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpecificationContextLimits {
    pub max_results: usize,
    pub max_characters: usize,
}

impl Default for SpecificationContextLimits {
    fn default() -> Self {
        Self {
            max_results: DEFAULT_SPECIFICATION_CONTEXT_RESULTS,
            max_characters: DEFAULT_SPECIFICATION_CONTEXT_CHARACTERS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpecificationContextExclusionReason {
    Changed,
    Missing,
    ResultLimit,
    CharacterBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificationContextItem {
    pub specification_id: String,
    pub relative_path: String,
    pub content_hash: String,
    pub approved_at_unix_ms: u64,
    pub source: String,
    pub characters: usize,
    pub acceptance_criteria_characters: usize,
    pub acceptance_criteria: SpecificationAcceptanceCriteria,
    pub authority: &'static str,
    pub source_boundary: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificationContextExclusion {
    pub specification_id: String,
    pub relative_path: String,
    pub reason: SpecificationContextExclusionReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificationEgressExclusion {
    pub specification_id: String,
    pub policy: AgentEgressPolicy,
    pub block_reason: AgentEgressBlockReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecificationEgressCoverage {
    pub target: AgentEgressTarget,
    pub blocked_specifications: usize,
    pub returned_exclusions: usize,
    pub omitted_exclusions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSpecificationsContext {
    pub project_id: String,
    pub max_characters: usize,
    pub text_characters: usize,
    pub acceptance_criteria_characters: usize,
    pub specifications: Vec<SpecificationContextItem>,
    pub exclusions: Vec<SpecificationContextExclusion>,
    pub total_approved: usize,
    pub current_approved: usize,
    pub changed_approved: usize,
    pub missing_approved: usize,
    pub omitted_specifications: usize,
    pub omitted_exclusions: usize,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_target: Option<AgentEgressTarget>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub egress_exclusions: Vec<SpecificationEgressExclusion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_coverage: Option<SpecificationEgressCoverage>,
    pub project_live_source_checked: bool,
    pub specification_source_revision_checked: bool,
    pub authority: &'static str,
    pub source_boundary: &'static str,
    pub instruction_warning: &'static str,
    pub privacy_notice: &'static str,
}

pub fn project_specifications_context(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    limits: SpecificationContextLimits,
) -> Result<ProjectSpecificationsContext, LeyCoreError> {
    SpecificationRegistry::system_default()?.context(project_start, vault, limits)
}

fn push_context_exclusion(
    exclusions: &mut Vec<SpecificationContextExclusion>,
    omitted_exclusions: &mut usize,
    exclusion: SpecificationContextExclusion,
) {
    if exclusions.len() < MAX_SPECIFICATION_CONTEXT_EXCLUSIONS {
        exclusions.push(exclusion);
    } else {
        *omitted_exclusions += 1;
    }
}

fn fit_specification_acceptance_criteria_characters(
    specifications: &mut [SpecificationContextItem],
    mut remaining_characters: usize,
) -> usize {
    let mut used = 0usize;
    for specification in specifications {
        let required =
            acceptance_criteria_projection_characters(&specification.acceptance_criteria);
        if required == 0 {
            specification.acceptance_criteria_characters = 0;
            continue;
        }
        if required <= remaining_characters {
            specification.acceptance_criteria_characters = required;
            remaining_characters -= required;
            used = used.saturating_add(required);
        } else {
            omit_acceptance_criteria_for_budget(&mut specification.acceptance_criteria);
            specification.acceptance_criteria_characters = 0;
        }
    }
    used
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpecificationRegistryDocument {
    schema_version: u32,
    approvals: BTreeMap<String, BTreeMap<String, SpecificationApprovalEntry>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SpecificationApprovalEntry {
    relative_path: String,
    content_hash: String,
    approved_at_unix_ms: u64,
}

impl SpecificationRegistryDocument {
    fn empty() -> Self {
        Self {
            schema_version: SPECIFICATION_REGISTRY_SCHEMA_VERSION,
            approvals: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), LeyCoreError> {
        if self.schema_version != SPECIFICATION_REGISTRY_SCHEMA_VERSION {
            return Err(LeyCoreError::InvalidSpecificationRegistry(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        for (project_id, approvals) in &self.approvals {
            validate_project_id(project_id).map_err(|error| {
                LeyCoreError::InvalidSpecificationRegistry(format!(
                    "invalid project ID key '{project_id}': {error}"
                ))
            })?;
            if approvals.len() > MAX_SPECIFICATION_APPROVALS_PER_PROJECT {
                return Err(LeyCoreError::InvalidSpecificationRegistry(format!(
                    "project {project_id} has more than {MAX_SPECIFICATION_APPROVALS_PER_PROJECT} Specification approvals"
                )));
            }
            let mut paths = HashSet::new();
            for (specification_id, approval) in approvals {
                validate_specification_id(specification_id)
                    .map_err(LeyCoreError::InvalidSpecificationRegistry)?;
                validate_specification_relative_path(&approval.relative_path)
                    .map_err(LeyCoreError::InvalidSpecificationRegistry)?;
                validate_content_hash(&approval.content_hash)
                    .map_err(LeyCoreError::InvalidSpecificationRegistry)?;
                if approval.approved_at_unix_ms == 0 {
                    return Err(LeyCoreError::InvalidSpecificationRegistry(
                        "specification approval time must be non-zero".to_owned(),
                    ));
                }
                if !paths.insert(approval.relative_path.as_str()) {
                    return Err(LeyCoreError::InvalidSpecificationRegistry(format!(
                        "multiple specification IDs claim {} in project {project_id}",
                        approval.relative_path
                    )));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SpecificationRegistry {
    path: PathBuf,
}

impl SpecificationRegistry {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        Ok(Self::at(
            default_binding_registry_path()?.with_file_name(SPECIFICATION_REGISTRY_FILE),
        ))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn contains_approval(
        &self,
        project_start: impl AsRef<Path>,
        specification_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_specification_id(specification_id)
            .map_err(LeyCoreError::InvalidSpecificationRequest)?;
        let project_id = diagnose_project(project_start)?.identity.project_id;
        self.with_locked_document(|document| {
            Ok(document
                .approvals
                .get(&project_id)
                .is_some_and(|approvals| approvals.contains_key(specification_id)))
        })
    }

    pub fn approve(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
        specification_id: &str,
        relative_path: &str,
    ) -> Result<SpecificationApproval, LeyCoreError> {
        validate_specification_id(specification_id)
            .map_err(LeyCoreError::InvalidSpecificationRequest)?;
        validate_specification_relative_path(relative_path)
            .map_err(LeyCoreError::InvalidSpecificationRequest)?;
        let diagnostic = diagnose_project(project_start)?;
        let source = read_stable_specification_bytes(vault.as_ref(), relative_path)?;
        let content_hash = specification_content_hash(&source);
        let approval = SpecificationApproval {
            project_id: diagnostic.identity.project_id.clone(),
            specification_id: specification_id.to_owned(),
            relative_path: relative_path.to_owned(),
            content_hash: content_hash.clone(),
            approved_at_unix_ms: unix_time_ms(),
        };
        let project_id = approval.project_id.clone();
        let specification_id = approval.specification_id.clone();
        let relative_path = approval.relative_path.clone();
        let approved_at_unix_ms = approval.approved_at_unix_ms;
        self.mutate(|document| {
            let approvals = document.approvals.entry(project_id.clone()).or_default();
            if !approvals.contains_key(&specification_id)
                && approvals.len() >= MAX_SPECIFICATION_APPROVALS_PER_PROJECT
            {
                return Err(LeyCoreError::InvalidSpecificationRequest(format!(
                    "a project can approve at most {MAX_SPECIFICATION_APPROVALS_PER_PROJECT} Specifications"
                )));
            }
            if approvals.iter().any(|(existing_id, existing)| {
                existing_id != &specification_id && existing.relative_path == relative_path
            }) {
                return Err(LeyCoreError::InvalidSpecificationRequest(format!(
                    "{relative_path} is already approved under another specification ID; revoke that approval first"
                )));
            }
            approvals.insert(
                specification_id.clone(),
                SpecificationApprovalEntry {
                    relative_path: relative_path.clone(),
                    content_hash: content_hash.clone(),
                    approved_at_unix_ms,
                },
            );
            Ok(())
        })?;
        Ok(approval)
    }

    pub fn revoke(
        &self,
        project_start: impl AsRef<Path>,
        specification_id: &str,
    ) -> Result<Option<SpecificationApproval>, LeyCoreError> {
        validate_specification_id(specification_id)
            .map_err(LeyCoreError::InvalidSpecificationRequest)?;
        let diagnostic = diagnose_project(project_start)?;
        let project_id = diagnostic.identity.project_id;
        self.mutate(|document| {
            let Some(approvals) = document.approvals.get_mut(&project_id) else {
                return Ok(None);
            };
            let removed = approvals.remove(specification_id);
            if approvals.is_empty() {
                document.approvals.remove(&project_id);
            }
            Ok(removed.map(|entry| SpecificationApproval {
                project_id: project_id.clone(),
                specification_id: specification_id.to_owned(),
                relative_path: entry.relative_path,
                content_hash: entry.content_hash,
                approved_at_unix_ms: entry.approved_at_unix_ms,
            }))
        })
    }

    pub fn list(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
    ) -> Result<SpecificationAuthorityList, LeyCoreError> {
        let diagnostic = diagnose_project(project_start)?;
        let project_id = diagnostic.identity.project_id;
        self.with_locked_document(|document| {
            let approvals = document
                .approvals
                .get(&project_id)
                .cloned()
                .unwrap_or_default();
            let mut specifications = Vec::with_capacity(approvals.len());
            let mut current = 0;
            let mut changed = 0;
            let mut missing = 0;
            for (specification_id, entry) in approvals {
                let current_hash =
                    match read_stable_specification_bytes(vault.as_ref(), &entry.relative_path) {
                        Ok(source) => Some(specification_content_hash(&source)),
                        Err(LeyCoreError::Io { source, .. })
                            if source.kind() == std::io::ErrorKind::NotFound =>
                        {
                            None
                        }
                        Err(error) => return Err(error),
                    };
                let state = match current_hash.as_deref() {
                    Some(hash) if hash == entry.content_hash => {
                        current += 1;
                        SpecificationApprovalState::Current
                    }
                    Some(_) => {
                        changed += 1;
                        SpecificationApprovalState::Changed
                    }
                    None => {
                        missing += 1;
                        SpecificationApprovalState::Missing
                    }
                };
                specifications.push(SpecificationAuthority {
                    approval: SpecificationApproval {
                        project_id: project_id.clone(),
                        specification_id,
                        relative_path: entry.relative_path,
                        content_hash: entry.content_hash,
                        approved_at_unix_ms: entry.approved_at_unix_ms,
                    },
                    state,
                    current_content_hash: current_hash,
                });
            }
            Ok(SpecificationAuthorityList {
                project_id: project_id.clone(),
                specifications,
                current,
                changed,
                missing,
                privacy_notice: PRIVACY_NOTICE,
            })
        })
    }

    pub fn read_approved_source(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
        specification_id: &str,
    ) -> Result<ApprovedSpecificationSource, LeyCoreError> {
        validate_specification_id(specification_id)
            .map_err(LeyCoreError::InvalidSpecificationRequest)?;
        let diagnostic = diagnose_project(project_start)?;
        self.read_approved_source_for_project_id(
            &diagnostic.identity.project_id,
            vault.as_ref(),
            specification_id,
        )
    }

    pub(crate) fn read_approved_source_for_expected_project(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
        expected_project_id: &str,
        specification_id: &str,
    ) -> Result<ApprovedSpecificationSource, LeyCoreError> {
        validate_project_id(expected_project_id)
            .map_err(|error| LeyCoreError::InvalidSpecificationRequest(error.to_string()))?;
        validate_specification_id(specification_id)
            .map_err(LeyCoreError::InvalidSpecificationRequest)?;
        let diagnostic = diagnose_project(project_start)?;
        if diagnostic.identity.project_id != expected_project_id {
            return Err(LeyCoreError::InvalidProjectIdentity(
                "source project identity changed during approved Specification read".to_owned(),
            ));
        }
        self.read_approved_source_for_project_id(
            expected_project_id,
            vault.as_ref(),
            specification_id,
        )
    }

    fn read_approved_source_for_project_id(
        &self,
        project_id: &str,
        vault: &Path,
        specification_id: &str,
    ) -> Result<ApprovedSpecificationSource, LeyCoreError> {
        self.with_locked_document(|document| {
            SpecificationAuthoritySnapshot {
                approvals: &document.approvals,
            }
            .read_approved_source(project_id, vault, specification_id)
        })
    }

    #[cfg(test)]
    pub(crate) fn task_scan(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
        task: &str,
    ) -> Result<TaskSpecificationScan, LeyCoreError> {
        self.with_task_scan_locked(project_start, vault, task, Ok)
    }

    pub(crate) fn with_task_scan_locked<T>(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
        task: &str,
        operation: impl FnOnce(TaskSpecificationScan) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        self.with_task_scan_policy_locked(
            project_start.as_ref(),
            vault.as_ref(),
            task,
            None,
            |scan, _, _| operation(scan),
        )
    }

    pub(crate) fn with_task_scan_for_agent_locked<T>(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
        task: &str,
        policies: &EgressPolicySnapshot,
        target: AgentEgressTarget,
        operation: impl FnOnce(
            TaskSpecificationScan,
            Vec<SpecificationEgressExclusion>,
            &SpecificationAuthoritySnapshot<'_>,
        ) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        self.with_task_scan_policy_locked(
            project_start.as_ref(),
            vault.as_ref(),
            task,
            Some((policies, target)),
            operation,
        )
    }

    fn with_task_scan_policy_locked<T>(
        &self,
        project_start: &Path,
        vault: &Path,
        task: &str,
        egress: Option<(&EgressPolicySnapshot, AgentEgressTarget)>,
        operation: impl FnOnce(
            TaskSpecificationScan,
            Vec<SpecificationEgressExclusion>,
            &SpecificationAuthoritySnapshot<'_>,
        ) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let normalized_task = task.trim().to_lowercase();
        let terms = specification_task_terms(&normalized_task);
        let diagnostic = diagnose_project(project_start)?;
        let project_id = diagnostic.identity.project_id;
        self.with_locked_document(|document| {
            let approvals = document
                .approvals
                .get(&project_id)
                .cloned()
                .unwrap_or_default();
            let mut total_approved = 0usize;
            let mut candidates = Vec::new();
            let mut exclusions = Vec::new();
            let mut egress_exclusions = Vec::new();
            let mut current_approved = 0;
            let mut changed_approved = 0;
            let mut missing_approved = 0;
            let mut low_relevance_approved = 0;

            for (specification_id, entry) in approvals {
                if let Some((policies, target)) = egress {
                    let decision = evaluate_agent_egress(
                        policies.specification_policy(&project_id, &specification_id),
                        target,
                    );
                    if !decision.allowed {
                        egress_exclusions.push(SpecificationEgressExclusion {
                            specification_id,
                            policy: decision.policy,
                            block_reason: decision
                                .block_reason
                                .expect("blocked egress decision has a reason"),
                        });
                        continue;
                    }
                }
                total_approved += 1;
                let source = match read_stable_specification_bytes(vault, &entry.relative_path) {
                    Ok(source) => source,
                    Err(LeyCoreError::Io { source, .. })
                        if source.kind() == std::io::ErrorKind::NotFound =>
                    {
                        missing_approved += 1;
                        exclusions.push(TaskSpecificationExclusion {
                            specification_id,
                            relative_path: entry.relative_path,
                            reason: TaskSpecificationExclusionReason::Missing,
                        });
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                let content_hash = specification_content_hash(&source);
                if content_hash != entry.content_hash {
                    changed_approved += 1;
                    exclusions.push(TaskSpecificationExclusion {
                        specification_id,
                        relative_path: entry.relative_path,
                        reason: TaskSpecificationExclusionReason::Changed,
                    });
                    continue;
                }
                current_approved += 1;
                let source = String::from_utf8(source).map_err(|_| {
                    LeyCoreError::InvalidSpecificationRequest(
                        "approved Specification Markdown must be valid UTF-8".to_owned(),
                    )
                })?;
                let searchable = format!("{}\n{}", entry.relative_path, source);
                let (score, exact_match) = lexical_score(&searchable, &normalized_task, &terms);
                if score == 0 {
                    low_relevance_approved += 1;
                    exclusions.push(TaskSpecificationExclusion {
                        specification_id,
                        relative_path: entry.relative_path,
                        reason: TaskSpecificationExclusionReason::LowRelevance,
                    });
                    continue;
                }
                candidates.push(TaskSpecificationCandidate {
                    source: ApprovedSpecificationSource {
                        project_id: project_id.clone(),
                        specification_id,
                        relative_path: entry.relative_path,
                        content_hash,
                        approved_at_unix_ms: entry.approved_at_unix_ms,
                        source,
                        source_boundary: "user-approved-specification",
                        authority: "human-intent",
                    },
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

            let authority_snapshot = SpecificationAuthoritySnapshot {
                approvals: &document.approvals,
            };
            operation(
                TaskSpecificationScan {
                    project_id: project_id.clone(),
                    candidates,
                    exclusions,
                    total_approved,
                    current_approved,
                    changed_approved,
                    missing_approved,
                    low_relevance_approved,
                },
                egress_exclusions,
                &authority_snapshot,
            )
        })
    }

    pub fn context(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
        limits: SpecificationContextLimits,
    ) -> Result<ProjectSpecificationsContext, LeyCoreError> {
        validate_specification_context_limits(limits)?;
        let authority = self.list(project_start.as_ref(), vault.as_ref())?;
        let total_approved = authority.specifications.len();
        let mut specifications = Vec::new();
        let mut exclusions = Vec::new();
        let mut omitted_exclusions = 0;
        let mut text_characters: usize = 0;
        let push_exclusion =
            |exclusions: &mut Vec<SpecificationContextExclusion>,
             omitted_exclusions: &mut usize,
             exclusion: SpecificationContextExclusion| {
                if exclusions.len() < MAX_SPECIFICATION_CONTEXT_EXCLUSIONS {
                    exclusions.push(exclusion);
                } else {
                    *omitted_exclusions += 1;
                }
            };

        for item in &authority.specifications {
            match item.state {
                SpecificationApprovalState::Changed => {
                    push_exclusion(
                        &mut exclusions,
                        &mut omitted_exclusions,
                        SpecificationContextExclusion {
                            specification_id: item.approval.specification_id.clone(),
                            relative_path: item.approval.relative_path.clone(),
                            reason: SpecificationContextExclusionReason::Changed,
                        },
                    );
                    continue;
                }
                SpecificationApprovalState::Missing => {
                    push_exclusion(
                        &mut exclusions,
                        &mut omitted_exclusions,
                        SpecificationContextExclusion {
                            specification_id: item.approval.specification_id.clone(),
                            relative_path: item.approval.relative_path.clone(),
                            reason: SpecificationContextExclusionReason::Missing,
                        },
                    );
                    continue;
                }
                SpecificationApprovalState::Current => {}
            }
            if specifications.len() >= limits.max_results {
                push_exclusion(
                    &mut exclusions,
                    &mut omitted_exclusions,
                    SpecificationContextExclusion {
                        specification_id: item.approval.specification_id.clone(),
                        relative_path: item.approval.relative_path.clone(),
                        reason: SpecificationContextExclusionReason::ResultLimit,
                    },
                );
                continue;
            }
            let approved = self.read_approved_source(
                project_start.as_ref(),
                vault.as_ref(),
                &item.approval.specification_id,
            )?;
            let characters = approved.source.chars().count();
            if text_characters.saturating_add(characters) > limits.max_characters {
                push_exclusion(
                    &mut exclusions,
                    &mut omitted_exclusions,
                    SpecificationContextExclusion {
                        specification_id: approved.specification_id,
                        relative_path: approved.relative_path,
                        reason: SpecificationContextExclusionReason::CharacterBudget,
                    },
                );
                continue;
            }
            let acceptance_criteria = derive_specification_acceptance_criteria(
                &approved.specification_id,
                &approved.content_hash,
                &approved.source,
            );
            text_characters += characters;
            specifications.push(SpecificationContextItem {
                specification_id: approved.specification_id,
                relative_path: approved.relative_path,
                content_hash: approved.content_hash,
                approved_at_unix_ms: approved.approved_at_unix_ms,
                source: approved.source,
                characters,
                acceptance_criteria_characters: 0,
                acceptance_criteria,
                authority: approved.authority,
                source_boundary: approved.source_boundary,
            });
        }
        let acceptance_criteria_characters = fit_specification_acceptance_criteria_characters(
            &mut specifications,
            limits.max_characters.saturating_sub(text_characters),
        );
        let omitted_specifications = total_approved.saturating_sub(specifications.len());
        Ok(ProjectSpecificationsContext {
            project_id: authority.project_id,
            max_characters: limits.max_characters,
            text_characters,
            acceptance_criteria_characters,
            specifications,
            exclusions,
            total_approved,
            current_approved: authority.current,
            changed_approved: authority.changed,
            missing_approved: authority.missing,
            omitted_specifications,
            omitted_exclusions,
            truncated: omitted_specifications > 0 || omitted_exclusions > 0,
            egress_target: None,
            egress_exclusions: Vec::new(),
            egress_coverage: None,
            project_live_source_checked: false,
            specification_source_revision_checked: true,
            authority: "human-intent",
            source_boundary: "user-approved-specification",
            instruction_warning: SPECIFICATION_INSTRUCTION_WARNING,
            privacy_notice: PRIVACY_NOTICE,
        })
    }

    pub fn context_for_agent(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
        limits: SpecificationContextLimits,
        egress_registry: &EgressPolicyRegistry,
        target: AgentEgressTarget,
    ) -> Result<ProjectSpecificationsContext, LeyCoreError> {
        validate_specification_context_limits(limits)?;
        let project_start = project_start.as_ref();
        let vault = vault.as_ref();
        let project_id = diagnose_project(project_start)?.identity.project_id;
        egress_registry.with_snapshot_locked(|policies| {
            let project_decision =
                evaluate_agent_egress(policies.project_policy(&project_id), target);
            if !project_decision.allowed {
                return Err(LeyCoreError::AgentEgressDenied {
                    policy: project_decision.policy.to_string(),
                    target: target.to_string(),
                });
            }
            self.context_for_agent_with_policies(
                project_start,
                vault,
                limits,
                policies,
                target,
                &project_id,
            )
        })
    }

    fn context_for_agent_with_policies(
        &self,
        _project_start: &Path,
        vault: &Path,
        limits: SpecificationContextLimits,
        policies: &EgressPolicySnapshot,
        target: AgentEgressTarget,
        project_id: &str,
    ) -> Result<ProjectSpecificationsContext, LeyCoreError> {
        self.with_locked_document(|document| {
            let approvals = document
                .approvals
                .get(project_id)
                .cloned()
                .unwrap_or_default();
            let mut specifications = Vec::new();
            let mut exclusions = Vec::new();
            let mut omitted_exclusions = 0usize;
            let mut egress_exclusions = Vec::new();
            let mut omitted_egress_exclusions = 0usize;
            let mut blocked_specifications = 0usize;
            let mut total_approved = 0usize;
            let mut current_approved = 0usize;
            let mut changed_approved = 0usize;
            let mut missing_approved = 0usize;
            let mut text_characters = 0usize;

            for (specification_id, entry) in approvals {
                let decision = evaluate_agent_egress(
                    policies.specification_policy(project_id, &specification_id),
                    target,
                );
                if !decision.allowed {
                    blocked_specifications += 1;
                    let exclusion = SpecificationEgressExclusion {
                        specification_id,
                        policy: decision.policy,
                        block_reason: decision
                            .block_reason
                            .expect("blocked egress decision has a reason"),
                    };
                    if egress_exclusions.len() < MAX_SPECIFICATION_EGRESS_EXCLUSIONS {
                        egress_exclusions.push(exclusion);
                    } else {
                        omitted_egress_exclusions += 1;
                    }
                    continue;
                }
                total_approved += 1;

                let source = match read_stable_specification_bytes(vault, &entry.relative_path) {
                    Ok(source) => source,
                    Err(LeyCoreError::Io { source, .. })
                        if source.kind() == std::io::ErrorKind::NotFound =>
                    {
                        missing_approved += 1;
                        push_context_exclusion(
                            &mut exclusions,
                            &mut omitted_exclusions,
                            SpecificationContextExclusion {
                                specification_id,
                                relative_path: entry.relative_path,
                                reason: SpecificationContextExclusionReason::Missing,
                            },
                        );
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                let content_hash = specification_content_hash(&source);
                if content_hash != entry.content_hash {
                    changed_approved += 1;
                    push_context_exclusion(
                        &mut exclusions,
                        &mut omitted_exclusions,
                        SpecificationContextExclusion {
                            specification_id,
                            relative_path: entry.relative_path,
                            reason: SpecificationContextExclusionReason::Changed,
                        },
                    );
                    continue;
                }
                current_approved += 1;
                if specifications.len() >= limits.max_results {
                    push_context_exclusion(
                        &mut exclusions,
                        &mut omitted_exclusions,
                        SpecificationContextExclusion {
                            specification_id,
                            relative_path: entry.relative_path,
                            reason: SpecificationContextExclusionReason::ResultLimit,
                        },
                    );
                    continue;
                }
                let source = String::from_utf8(source).map_err(|_| {
                    LeyCoreError::InvalidSpecificationRequest(
                        "approved Specification Markdown must be valid UTF-8".to_owned(),
                    )
                })?;
                let characters = source.chars().count();
                if text_characters.saturating_add(characters) > limits.max_characters {
                    push_context_exclusion(
                        &mut exclusions,
                        &mut omitted_exclusions,
                        SpecificationContextExclusion {
                            specification_id,
                            relative_path: entry.relative_path,
                            reason: SpecificationContextExclusionReason::CharacterBudget,
                        },
                    );
                    continue;
                }
                let acceptance_criteria = derive_specification_acceptance_criteria(
                    &specification_id,
                    &content_hash,
                    &source,
                );
                text_characters += characters;
                specifications.push(SpecificationContextItem {
                    specification_id,
                    relative_path: entry.relative_path,
                    content_hash,
                    approved_at_unix_ms: entry.approved_at_unix_ms,
                    source,
                    characters,
                    acceptance_criteria_characters: 0,
                    acceptance_criteria,
                    authority: "human-intent",
                    source_boundary: "user-approved-specification",
                });
            }

            let acceptance_criteria_characters = fit_specification_acceptance_criteria_characters(
                &mut specifications,
                limits.max_characters.saturating_sub(text_characters),
            );
            let omitted_specifications = total_approved.saturating_sub(specifications.len());
            Ok(ProjectSpecificationsContext {
                project_id: project_id.to_owned(),
                max_characters: limits.max_characters,
                text_characters,
                acceptance_criteria_characters,
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
                    || blocked_specifications > 0
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
                privacy_notice: PRIVACY_NOTICE,
            })
        })
    }

    fn with_locked_document<T>(
        &self,
        operation: impl FnOnce(&SpecificationRegistryDocument) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let lock = self.acquire_lock()?;
        let result = (|| {
            let document = self.read_document()?;
            operation(&document)
        })();
        let unlock_result = File::unlock(&lock);
        match (result, unlock_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(source)) => Err(LeyCoreError::Io {
                path: self.lock_path(),
                source,
            }),
        }
    }

    fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut SpecificationRegistryDocument) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let lock = self.acquire_lock()?;
        let result = (|| {
            let mut document = self.read_document()?;
            let value = operation(&mut document)?;
            document.validate()?;
            self.write_document(&document)?;
            Ok(value)
        })();
        let unlock_result = File::unlock(&lock);
        match (result, unlock_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(source)) => Err(LeyCoreError::Io {
                path: self.lock_path(),
                source,
            }),
        }
    }

    fn acquire_lock(&self) -> Result<File, LeyCoreError> {
        self.prepare_parent()?;
        let lock_path = self.lock_path();
        reject_non_regular_if_present(&lock_path)?;
        let mut options = StdOpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options
            .open(&lock_path)
            .map_err(|source| LeyCoreError::Io {
                path: lock_path.clone(),
                source,
            })?;
        lock.lock().map_err(|source| LeyCoreError::Io {
            path: lock_path,
            source,
        })?;
        Ok(lock)
    }

    fn prepare_parent(&self) -> Result<(), LeyCoreError> {
        let parent = self.path.parent().ok_or_else(|| {
            LeyCoreError::InvalidSpecificationRegistry(
                "registry path must have a parent directory".to_owned(),
            )
        })?;
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(parent).map_err(|source| LeyCoreError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        let metadata = fs::symlink_metadata(parent).map_err(|source| LeyCoreError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LeyCoreError::UnsafeProjectLayout(parent.to_path_buf()));
        }
        Ok(())
    }

    fn read_document(&self) -> Result<SpecificationRegistryDocument, LeyCoreError> {
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SpecificationRegistryDocument::empty())
            }
            Err(source) => {
                return Err(LeyCoreError::Io {
                    path: self.path.clone(),
                    source,
                })
            }
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(LeyCoreError::UnsafeProjectLayout(self.path.clone()))
            }
            Ok(metadata) if metadata.len() > METADATA_FILE_LIMIT_BYTES => {
                return Err(LeyCoreError::MetadataTooLarge {
                    path: self.path.clone(),
                    limit_bytes: METADATA_FILE_LIMIT_BYTES,
                })
            }
            Ok(_) => {}
        }
        reject_non_regular_if_present(&self.path)?;
        let bytes = fs::read(&self.path).map_err(|source| LeyCoreError::Io {
            path: self.path.clone(),
            source,
        })?;
        let document: SpecificationRegistryDocument =
            serde_json::from_slice(&bytes).map_err(|source| LeyCoreError::Json {
                path: self.path.clone(),
                source,
            })?;
        document.validate()?;
        Ok(document)
    }

    fn write_document(&self, document: &SpecificationRegistryDocument) -> Result<(), LeyCoreError> {
        reject_non_regular_if_present(&self.path)?;
        let mut body = serde_json::to_vec_pretty(document)
            .expect("validated specification registry is serializable");
        body.push(b'\n');
        if body.len() as u64 > METADATA_FILE_LIMIT_BYTES {
            return Err(LeyCoreError::MetadataTooLarge {
                path: self.path.clone(),
                limit_bytes: METADATA_FILE_LIMIT_BYTES,
            });
        }
        let mut options = atomic_write_file::OpenOptions::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&self.path)
            .map_err(|source| LeyCoreError::Io {
                path: self.path.clone(),
                source,
            })?;
        file.write_all(&body).map_err(|source| LeyCoreError::Io {
            path: self.path.clone(),
            source,
        })?;
        file.commit().map_err(|source| LeyCoreError::Io {
            path: self.path.clone(),
            source,
        })
    }

    fn lock_path(&self) -> PathBuf {
        self.path.with_file_name(SPECIFICATION_REGISTRY_LOCK_FILE)
    }
}

fn validate_specification_context_limits(
    limits: SpecificationContextLimits,
) -> Result<(), LeyCoreError> {
    if limits.max_results == 0 || limits.max_results > MAX_SPECIFICATION_CONTEXT_RESULTS {
        return Err(LeyCoreError::InvalidSpecificationRequest(format!(
            "maxResults must be between 1 and {MAX_SPECIFICATION_CONTEXT_RESULTS}"
        )));
    }
    if limits.max_characters < MIN_SPECIFICATION_CONTEXT_CHARACTERS
        || limits.max_characters > MAX_SPECIFICATION_CONTEXT_CHARACTERS
    {
        return Err(LeyCoreError::InvalidSpecificationRequest(format!(
            "maxCharacters must be between {MIN_SPECIFICATION_CONTEXT_CHARACTERS} and {MAX_SPECIFICATION_CONTEXT_CHARACTERS}"
        )));
    }
    Ok(())
}

pub(crate) fn specification_task_terms(query: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "a",
        "add",
        "an",
        "and",
        "are",
        "as",
        "at",
        "be",
        "bug",
        "by",
        "can",
        "change",
        "code",
        "create",
        "do",
        "does",
        "feature",
        "fix",
        "for",
        "from",
        "implement",
        "in",
        "into",
        "is",
        "issue",
        "it",
        "make",
        "of",
        "on",
        "or",
        "should",
        "support",
        "task",
        "that",
        "the",
        "this",
        "to",
        "update",
        "use",
        "with",
        "work",
    ];
    let mut terms = query
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|term| !term.is_empty() && !STOPWORDS.contains(term))
        .take(16)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    terms
}

pub fn specification_content_hash(source: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(source))
}

pub fn derive_specification_acceptance_criteria(
    specification_id: &str,
    content_hash: &str,
    source: &str,
) -> SpecificationAcceptanceCriteria {
    let lines = markdown_source_lines(source);
    let frontmatter_end = markdown_frontmatter_end(&lines);
    let headings = acceptance_criteria_headings(&lines, frontmatter_end);
    if headings.is_empty() {
        return acceptance_criteria_projection(
            SpecificationAcceptanceCriteriaState::Absent,
            None,
            Vec::new(),
            0,
        );
    }
    if headings.len() != 1 {
        return acceptance_criteria_projection(
            SpecificationAcceptanceCriteriaState::Ambiguous,
            None,
            Vec::new(),
            0,
        );
    }

    let (heading_index, heading_level) = headings[0];
    let heading_line = lines[heading_index].number;
    let mut criteria = Vec::new();
    let mut total_criteria = 0usize;
    let mut index = heading_index + 1;
    let mut in_fence: Option<(u8, usize)> = None;
    let mut child_subsection = false;

    while index < lines.len() {
        let line = &lines[index];
        if let Some((marker, minimum)) = in_fence {
            if closes_fence(line.text, marker, minimum) {
                in_fence = None;
            }
            index += 1;
            continue;
        }
        if let Some((marker, minimum)) = opens_fence(line.text) {
            in_fence = Some((marker, minimum));
            index += 1;
            continue;
        }
        if let Some((level, _)) = parse_atx_heading(line.text) {
            if level <= heading_level {
                break;
            }
            child_subsection = true;
            index += 1;
            continue;
        }
        if child_subsection {
            index += 1;
            continue;
        }
        if direct_list_marker_end(line.text).is_none() {
            index += 1;
            continue;
        }

        total_criteria = total_criteria.saturating_add(1);
        let start_index = index;
        let mut end_index = index;
        let mut cursor = index + 1;
        let mut blank_seen = false;
        while cursor < lines.len() {
            let next = &lines[cursor];
            if let Some((level, _)) = parse_atx_heading(next.text) {
                if level <= heading_level {
                    break;
                }
                child_subsection = true;
                break;
            }
            if direct_list_marker_end(next.text).is_some()
                || opens_fence(next.text).is_some()
                || next.text.starts_with('>')
            {
                break;
            }
            if next.text.trim().is_empty() {
                blank_seen = true;
                cursor += 1;
                continue;
            }
            if blank_seen && markdown_leading_spaces(next.text) < 4 {
                break;
            }
            end_index = cursor;
            blank_seen = false;
            cursor += 1;
        }

        if total_criteria <= MAX_SPECIFICATION_ACCEPTANCE_CRITERIA {
            let start = lines[start_index].start;
            let end = lines[end_index].content_end;
            let text = source[start..end].to_owned();
            let criterion_id = specification_acceptance_criterion_id(
                specification_id,
                content_hash,
                heading_line,
                lines[start_index].number,
                lines[end_index].number,
                total_criteria,
                &text,
            );
            criteria.push(SpecificationAcceptanceCriterion {
                criterion_id,
                text,
                start_line: lines[start_index].number,
                end_line: lines[end_index].number,
            });
        }
        index = cursor.max(index + 1);
    }

    if total_criteria > MAX_SPECIFICATION_ACCEPTANCE_CRITERIA {
        return acceptance_criteria_projection(
            SpecificationAcceptanceCriteriaState::OmittedLimit,
            Some(heading_line),
            Vec::new(),
            total_criteria,
        );
    }
    if criteria.is_empty() {
        acceptance_criteria_projection(
            SpecificationAcceptanceCriteriaState::Empty,
            Some(heading_line),
            Vec::new(),
            0,
        )
    } else {
        acceptance_criteria_projection(
            SpecificationAcceptanceCriteriaState::Available,
            Some(heading_line),
            criteria,
            total_criteria,
        )
    }
}

pub(crate) fn acceptance_criteria_projection_characters(
    projection: &SpecificationAcceptanceCriteria,
) -> usize {
    if projection.state != SpecificationAcceptanceCriteriaState::Available {
        return 0;
    }
    projection.criteria.iter().fold(0usize, |total, criterion| {
        total
            .saturating_add(criterion.criterion_id.chars().count())
            .saturating_add(criterion.text.chars().count())
            .saturating_add(32)
    })
}

pub(crate) fn acceptance_criteria_projection_tokens(
    projection: &SpecificationAcceptanceCriteria,
) -> usize {
    let characters = acceptance_criteria_projection_characters(projection);
    if characters == 0 {
        0
    } else {
        12usize.saturating_add(characters.div_ceil(4))
    }
}

pub(crate) fn omit_acceptance_criteria_for_budget(
    projection: &mut SpecificationAcceptanceCriteria,
) {
    if projection.state != SpecificationAcceptanceCriteriaState::Available {
        return;
    }
    projection.state = SpecificationAcceptanceCriteriaState::OmittedBudget;
    projection.returned_criteria = 0;
    projection.omitted_criteria = projection.total_criteria;
    projection.criteria.clear();
}

fn acceptance_criteria_projection(
    state: SpecificationAcceptanceCriteriaState,
    heading_line: Option<u64>,
    criteria: Vec<SpecificationAcceptanceCriterion>,
    total_criteria: usize,
) -> SpecificationAcceptanceCriteria {
    let returned_criteria = criteria.len();
    SpecificationAcceptanceCriteria {
        state,
        total_criteria,
        returned_criteria,
        omitted_criteria: total_criteria.saturating_sub(returned_criteria),
        heading_line,
        criteria,
        source_revision_bound: true,
        status_interpreted: false,
        persisted: false,
        authority: "human-intent",
        source_boundary: "derived-from-approved-specification",
    }
}

fn specification_acceptance_criterion_id(
    specification_id: &str,
    content_hash: &str,
    heading_line: u64,
    start_line: u64,
    end_line: u64,
    ordinal: usize,
    text: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ley-specification-acceptance-criterion-v1\0");
    digest.update(specification_id.as_bytes());
    digest.update(b"\0");
    digest.update(content_hash.as_bytes());
    digest.update(b"\0");
    digest.update(heading_line.to_be_bytes());
    digest.update(start_line.to_be_bytes());
    digest.update(end_line.to_be_bytes());
    digest.update((ordinal as u64).to_be_bytes());
    digest.update(text.as_bytes());
    format!("acr_{:x}", digest.finalize())
}

#[derive(Debug, Clone, Copy)]
struct MarkdownSourceLine<'a> {
    number: u64,
    start: usize,
    content_end: usize,
    text: &'a str,
}

fn markdown_source_lines(source: &str) -> Vec<MarkdownSourceLine<'_>> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    for (index, segment) in source.split_inclusive('\n').enumerate() {
        let end = start + segment.len();
        let content_end = if segment.ends_with("\r\n") {
            end.saturating_sub(2)
        } else if segment.ends_with('\n') {
            end.saturating_sub(1)
        } else {
            end
        };
        lines.push(MarkdownSourceLine {
            number: index as u64 + 1,
            start,
            content_end,
            text: &source[start..content_end],
        });
        start = end;
    }
    lines
}

fn markdown_frontmatter_end(lines: &[MarkdownSourceLine<'_>]) -> Option<usize> {
    let Some(first) = lines.first() else {
        return Some(0);
    };
    if first.text.trim() != "---" {
        return Some(0);
    }
    for (index, line) in lines.iter().enumerate().skip(1) {
        if matches!(line.text.trim(), "---" | "...") {
            return Some(index + 1);
        }
    }
    None
}

fn acceptance_criteria_headings(
    lines: &[MarkdownSourceLine<'_>],
    frontmatter_end: Option<usize>,
) -> Vec<(usize, u8)> {
    let Some(start) = frontmatter_end else {
        return Vec::new();
    };
    let mut headings = Vec::new();
    let mut in_fence: Option<(u8, usize)> = None;
    for (index, line) in lines.iter().enumerate().skip(start) {
        if let Some((marker, minimum)) = in_fence {
            if closes_fence(line.text, marker, minimum) {
                in_fence = None;
            }
            continue;
        }
        if let Some((marker, minimum)) = opens_fence(line.text) {
            in_fence = Some((marker, minimum));
            continue;
        }
        if let Some((level, heading)) = parse_atx_heading(line.text) {
            if heading.eq_ignore_ascii_case("acceptance criteria") {
                headings.push((index, level));
            }
        }
    }
    headings
}

fn parse_atx_heading(line: &str) -> Option<(u8, &str)> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('>') {
        return None;
    }
    let rest = line;
    let hashes = rest.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let after_hashes = rest.get(hashes..)?;
    if !after_hashes.is_empty()
        && !after_hashes
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_whitespace)
    {
        return None;
    }
    let trimmed =
        after_hashes.trim_matches(|character: char| character == ' ' || character == '\t');
    let without_hashes = trimmed.trim_end_matches('#');
    let heading = if without_hashes.len() < trimmed.len()
        && without_hashes
            .chars()
            .last()
            .is_some_and(char::is_whitespace)
    {
        without_hashes.trim_end()
    } else {
        trimmed
    };
    Some((hashes as u8, heading))
}

fn opens_fence(line: &str) -> Option<(u8, usize)> {
    let leading = markdown_leading_spaces(line);
    if leading > 3 {
        return None;
    }
    let rest = line.get(leading..)?;
    let marker = *rest.as_bytes().first()?;
    if marker != 96 && marker != b'~' {
        return None;
    }
    let count = rest.bytes().take_while(|byte| *byte == marker).count();
    (count >= 3).then_some((marker, count))
}

fn closes_fence(line: &str, marker: u8, minimum: usize) -> bool {
    let leading = markdown_leading_spaces(line);
    if leading > 3 {
        return false;
    }
    let Some(rest) = line.get(leading..) else {
        return false;
    };
    let count = rest.bytes().take_while(|byte| *byte == marker).count();
    count >= minimum
        && rest[count..]
            .chars()
            .all(|character| character == ' ' || character == '\t')
}

fn direct_list_marker_end(line: &str) -> Option<usize> {
    if line.is_empty() || line.starts_with(' ') || line.starts_with('\t') || line.starts_with('>') {
        return None;
    }
    let bytes = line.as_bytes();
    if matches!(bytes.first(), Some(b'-' | b'*' | b'+')) {
        return bytes
            .get(1)
            .is_some_and(u8::is_ascii_whitespace)
            .then_some(2);
    }
    let digits = bytes
        .iter()
        .take(9)
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits == 0 || !matches!(bytes.get(digits), Some(b'.' | b')')) {
        return None;
    }
    bytes
        .get(digits + 1)
        .is_some_and(u8::is_ascii_whitespace)
        .then_some(digits + 2)
}

fn markdown_leading_spaces(line: &str) -> usize {
    line.as_bytes()
        .iter()
        .take_while(|byte| **byte == b' ')
        .count()
}

pub fn generate_specification_id() -> String {
    format!("spec_{}", Uuid::new_v4().simple())
}

pub(crate) fn validate_specification_id(value: &str) -> Result<(), String> {
    let Some(uuid) = value.strip_prefix("spec_") else {
        return Err("specificationId must start with spec_".to_owned());
    };
    if uuid.len() != 32
        || !uuid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || Uuid::parse_str(uuid).is_err()
    {
        return Err(
            "specificationId must contain a 32-character lowercase hexadecimal UUID".to_owned(),
        );
    }
    Ok(())
}

fn validate_specification_relative_path(value: &str) -> Result<(), String> {
    if value.is_empty() || value.chars().count() > MAX_SPECIFICATION_PATH_CHARACTERS {
        return Err(format!(
            "Specification path must contain 1 to {MAX_SPECIFICATION_PATH_CHARACTERS} characters"
        ));
    }
    let path = Path::new(value);
    if path.is_absolute() || path.extension().and_then(|part| part.to_str()) != Some("md") {
        return Err("Specification path must be a relative Markdown path".to_owned());
    }
    let mut components = 0;
    for component in path.components() {
        let Component::Normal(segment) = component else {
            return Err("Specification path contains an unsafe segment".to_owned());
        };
        let Some(segment) = segment.to_str() else {
            return Err("Specification path must be valid UTF-8".to_owned());
        };
        if segment.starts_with('.') || segment.chars().any(char::is_control) {
            return Err("Specification path must refer to a visible vault note".to_owned());
        }
        components += 1;
    }
    if components == 0 {
        return Err("Specification path is empty".to_owned());
    }
    Ok(())
}

fn validate_content_hash(value: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err("Specification content hash must use sha256".to_owned());
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("Specification content hash is malformed".to_owned());
    }
    Ok(())
}

fn read_stable_specification_bytes(vault: &Path, relative: &str) -> Result<Vec<u8>, LeyCoreError> {
    validate_specification_relative_path(relative)
        .map_err(LeyCoreError::InvalidSpecificationRequest)?;
    let root_path = fs::canonicalize(vault).map_err(|source| LeyCoreError::Io {
        path: vault.to_path_buf(),
        source,
    })?;
    if !root_path.is_dir() {
        return Err(LeyCoreError::NotDirectory(root_path));
    }
    let first = read_specification_bytes_once(&root_path, relative)?;
    let second = read_specification_bytes_once(&root_path, relative)?;
    if first != second {
        return Err(LeyCoreError::InvalidSpecificationRequest(format!(
            "Specification {relative} changed while Ley was reading it; retry approval"
        )));
    }
    Ok(first)
}

fn read_specification_bytes_once(
    root_path: &Path,
    relative: &str,
) -> Result<Vec<u8>, LeyCoreError> {
    let root = Dir::open_ambient_dir(root_path, ambient_authority()).map_err(|source| {
        LeyCoreError::Io {
            path: root_path.to_path_buf(),
            source,
        }
    })?;
    let path = Path::new(relative);
    let components = path
        .components()
        .map(|component| match component {
            Component::Normal(segment) => Ok(segment.to_os_string()),
            _ => Err(LeyCoreError::InvalidSpecificationRequest(
                "Specification path contains an unsafe segment".to_owned(),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (file_name, directories) = components.split_last().ok_or_else(|| {
        LeyCoreError::InvalidSpecificationRequest("Specification path is empty".to_owned())
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
    if metadata.len() > MAX_SPECIFICATION_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: traversed,
            limit_bytes: MAX_SPECIFICATION_BYTES,
        });
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(MAX_SPECIFICATION_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| LeyCoreError::Io {
            path: PathBuf::from(relative),
            source,
        })?;
    if bytes.len() as u64 > MAX_SPECIFICATION_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: PathBuf::from(relative),
            limit_bytes: MAX_SPECIFICATION_BYTES,
        });
    }
    Ok(bytes)
}

fn reject_non_regular_if_present(path: &Path) -> Result<(), LeyCoreError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(LeyCoreError::UnsafeProjectLayout(path.to_path_buf()))
        }
        #[cfg(unix)]
        Ok(metadata) => {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(LeyCoreError::InvalidSpecificationRegistry(format!(
                    "private file permissions required for {}; use mode 600",
                    path.display()
                )));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        Ok(_) => Ok(()),
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after the Unix epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{initialize_project, CaptureMode};
    use tempfile::tempdir;

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf, SpecificationRegistry) {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Spec project"), CaptureMode::Structured).unwrap();
        let registry = SpecificationRegistry::at(base.path().join("config/specifications.json"));
        (base, project, vault, registry)
    }

    #[test]
    fn acceptance_criteria_projection_preserves_exact_markdown_and_ranges() {
        let specification_id = "spec_11111111111111111111111111111111";
        let source = concat!(
            "---\r\n",
            "title: Example\r\n",
            "---\r\n",
            "# Product\r\n",
            "\r\n",
            "## Acceptance criteria\r\n",
            "\r\n",
            "- [ ] Preserve *raw* [link](https://example.invalid)\r\n",
            "1. Work offline\r\n",
            "    and keep local state.\r\n",
            "    - nested detail stays inside the parent slice\r\n",
            "2) Sync later\r\n",
            "\r\n",
            "## Verification\r\n",
            "- This is not an acceptance criterion.\r\n",
        );
        let content_hash = specification_content_hash(source.as_bytes());
        let first =
            derive_specification_acceptance_criteria(specification_id, &content_hash, source);
        let second =
            derive_specification_acceptance_criteria(specification_id, &content_hash, source);

        assert_eq!(first, second);
        assert_eq!(first.state, SpecificationAcceptanceCriteriaState::Available);
        assert_eq!(first.heading_line, Some(6));
        assert_eq!(first.total_criteria, 3);
        assert_eq!(first.returned_criteria, 3);
        assert_eq!(first.omitted_criteria, 0);
        assert!(!first.status_interpreted);
        assert!(!first.persisted);
        assert_eq!(first.authority, "human-intent");
        assert_eq!(first.source_boundary, "derived-from-approved-specification");
        assert_eq!(
            first.criteria[0].text,
            "- [ ] Preserve *raw* [link](https://example.invalid)"
        );
        assert_eq!(first.criteria[0].start_line, 8);
        assert_eq!(first.criteria[0].end_line, 8);
        assert_eq!(
            first.criteria[1].text,
            "1. Work offline\r\n    and keep local state.\r\n    - nested detail stays inside the parent slice"
        );
        assert_eq!(first.criteria[1].start_line, 9);
        assert_eq!(first.criteria[1].end_line, 11);
        assert_eq!(first.criteria[2].text, "2) Sync later");
        assert_eq!(first.criteria[2].start_line, 12);
        assert_eq!(first.criteria[2].end_line, 12);
        assert!(first
            .criteria
            .iter()
            .all(|criterion| criterion.criterion_id.starts_with("acr_")));

        let changed_hash = specification_content_hash(b"different approved revision");
        let changed =
            derive_specification_acceptance_criteria(specification_id, &changed_hash, source);
        assert_ne!(
            first.criteria[0].criterion_id,
            changed.criteria[0].criterion_id
        );
    }

    #[test]
    fn acceptance_criteria_projection_excludes_non_structural_markdown() {
        let specification_id = "spec_22222222222222222222222222222222";
        let source = concat!(
            "---\n",
            "fake: |\n",
            "  ## Acceptance criteria\n",
            "  - frontmatter fake\n",
            "---\n",
            "# Product\n",
            "\n",
            "~~~\n",
            "## Acceptance criteria\n",
            "- fenced fake\n",
            "~~~\n",
            "\n",
            "> ## Acceptance criteria\n",
            "> - quoted fake\n",
            "\n",
            "Acceptance criteria\n",
            "-------------------\n",
            "- setext fake\n",
            "\n",
            "## Acceptance criteria###\n",
            "- malformed ATX fake\n",
            "\n",
            "- list container\n",
            "  ## Acceptance criteria\n",
            "  - list-continuation fake\n",
        );
        let content_hash = specification_content_hash(source.as_bytes());
        let projection =
            derive_specification_acceptance_criteria(specification_id, &content_hash, source);
        assert_eq!(
            projection.state,
            SpecificationAcceptanceCriteriaState::Absent
        );
        assert!(projection.criteria.is_empty());
        assert_eq!(projection.total_criteria, 0);
    }

    #[test]
    fn acceptance_criteria_projection_fails_closed_on_ambiguous_or_nested_sections() {
        let specification_id = "spec_33333333333333333333333333333333";
        let ambiguous = concat!(
            "# Product\n",
            "## Acceptance criteria\n",
            "- first\n",
            "## Acceptance Criteria\n",
            "- second\n",
        );
        let ambiguous_hash = specification_content_hash(ambiguous.as_bytes());
        let ambiguous_projection =
            derive_specification_acceptance_criteria(specification_id, &ambiguous_hash, ambiguous);
        assert_eq!(
            ambiguous_projection.state,
            SpecificationAcceptanceCriteriaState::Ambiguous
        );
        assert!(ambiguous_projection.criteria.is_empty());

        let nested = concat!(
            "# Product\n",
            "## Acceptance criteria\n",
            "- direct criterion\n",
            "### Detailed examples\n",
            "- child-section item must not become a criterion\n",
            "## Verification\n",
            "- not a criterion\n",
        );
        let nested_hash = specification_content_hash(nested.as_bytes());
        let nested_projection =
            derive_specification_acceptance_criteria(specification_id, &nested_hash, nested);
        assert_eq!(
            nested_projection.state,
            SpecificationAcceptanceCriteriaState::Available
        );
        assert_eq!(nested_projection.total_criteria, 1);
        assert_eq!(nested_projection.criteria[0].text, "- direct criterion");
    }

    #[test]
    fn acceptance_criteria_projection_omits_atomically_when_count_limit_is_exceeded() {
        let specification_id = "spec_44444444444444444444444444444444";
        let mut source = String::from("# Product\n## Acceptance criteria\n");
        for index in 0..=MAX_SPECIFICATION_ACCEPTANCE_CRITERIA {
            source.push_str(&format!("- criterion {index}\n"));
        }
        let content_hash = specification_content_hash(source.as_bytes());
        let projection =
            derive_specification_acceptance_criteria(specification_id, &content_hash, &source);
        assert_eq!(
            projection.state,
            SpecificationAcceptanceCriteriaState::OmittedLimit
        );
        assert_eq!(
            projection.total_criteria,
            MAX_SPECIFICATION_ACCEPTANCE_CRITERIA + 1
        );
        assert_eq!(projection.returned_criteria, 0);
        assert_eq!(
            projection.omitted_criteria,
            MAX_SPECIFICATION_ACCEPTANCE_CRITERIA + 1
        );
        assert!(projection.criteria.is_empty());
    }

    #[test]
    fn direct_context_keeps_whole_specification_when_criteria_projection_exceeds_spare_budget() {
        let (_base, project, vault, registry) = setup();
        let source = format!(
            "# Offline\n\nRequirement body remains authoritative.\n{}\n\n## Acceptance criteria\n\n- This criterion is deliberately long enough to exceed the small spare projection budget while the source itself still fits.\n",
            "x".repeat(720)
        );
        fs::write(vault.join("Spec.md"), &source).unwrap();
        let specification_id = generate_specification_id();
        registry
            .approve(&project, &vault, &specification_id, "Spec.md")
            .unwrap();
        let source_characters = source.chars().count();
        let context = registry
            .context(
                &project,
                &vault,
                SpecificationContextLimits {
                    max_results: 8,
                    max_characters: 1_000,
                },
            )
            .unwrap();
        assert!(source_characters < 1_000);
        assert_eq!(context.specifications.len(), 1);
        assert_eq!(context.text_characters, source_characters);
        assert_eq!(context.acceptance_criteria_characters, 0);
        assert_eq!(
            context.specifications[0].acceptance_criteria.state,
            SpecificationAcceptanceCriteriaState::OmittedBudget
        );
        assert_eq!(
            context.specifications[0].acceptance_criteria.total_criteria,
            1
        );
        assert!(context.specifications[0]
            .acceptance_criteria
            .criteria
            .is_empty());
        assert!(context.specifications[0]
            .source
            .contains("Requirement body remains authoritative."));
    }

    #[test]
    fn approval_is_exact_revision_authority_and_edits_make_it_stale() {
        let (_base, project, vault, registry) = setup();
        fs::create_dir(vault.join("Specs")).unwrap();
        fs::write(
            vault.join("Specs/Offline.md"),
            "---\nley-type: specification\n---\n# Offline\n\nMust work offline.\n",
        )
        .unwrap();
        let specification_id = generate_specification_id();
        let approved = registry
            .approve(&project, &vault, &specification_id, "Specs/Offline.md")
            .unwrap();
        assert_eq!(approved.specification_id, specification_id);
        let listed = registry.list(&project, &vault).unwrap();
        assert_eq!(listed.current, 1);
        let serialized = serde_json::to_value(&listed).unwrap();
        assert_eq!(
            serialized["specifications"][0]["approval"]["specificationId"],
            specification_id
        );
        assert!(serialized["specifications"][0]
            .get("specificationId")
            .is_none());
        let source = registry
            .read_approved_source(&project, &vault, &specification_id)
            .unwrap();
        assert_eq!(source.authority, "human-intent");
        assert!(source.source.contains("Must work offline"));

        fs::write(
            vault.join("Specs/Offline.md"),
            "---\nley-type: specification\n---\n# Offline\n\nMust work offline and sync later.\n",
        )
        .unwrap();
        let list = registry.list(&project, &vault).unwrap();
        assert_eq!(list.current, 0);
        assert_eq!(list.changed, 1);
        assert!(matches!(
            registry.read_approved_source(&project, &vault, &specification_id),
            Err(LeyCoreError::SpecificationApprovalStale { .. })
        ));
    }

    #[test]
    fn approval_is_project_scoped_and_revocation_is_explicit() {
        let (base, project, vault, registry) = setup();
        fs::write(vault.join("Spec.md"), "# Requirement\n").unwrap();
        let specification_id = generate_specification_id();
        registry
            .approve(&project, &vault, &specification_id, "Spec.md")
            .unwrap();

        let other = base.path().join("other-project");
        fs::create_dir(&other).unwrap();
        initialize_project(&other, Some("Other"), CaptureMode::Structured).unwrap();
        assert!(registry
            .list(&other, &vault)
            .unwrap()
            .specifications
            .is_empty());
        assert!(registry
            .revoke(&other, &specification_id)
            .unwrap()
            .is_none());
        assert!(registry
            .revoke(&project, &specification_id)
            .unwrap()
            .is_some());
        assert!(registry
            .list(&project, &vault)
            .unwrap()
            .specifications
            .is_empty());
    }

    #[test]
    fn missing_note_is_disclosed_and_reapproval_updates_revision() {
        let (_base, project, vault, registry) = setup();
        fs::write(vault.join("Spec.md"), "first\n").unwrap();
        let specification_id = generate_specification_id();
        let first = registry
            .approve(&project, &vault, &specification_id, "Spec.md")
            .unwrap();
        fs::remove_file(vault.join("Spec.md")).unwrap();
        assert_eq!(registry.list(&project, &vault).unwrap().missing, 1);
        fs::write(vault.join("Spec.md"), "second\n").unwrap();
        let second = registry
            .approve(&project, &vault, &specification_id, "Spec.md")
            .unwrap();
        assert_ne!(first.content_hash, second.content_hash);
        assert_eq!(registry.list(&project, &vault).unwrap().current, 1);
    }

    #[test]
    fn unsafe_paths_symlinks_and_duplicate_path_authority_are_rejected() {
        let (base, project, vault, registry) = setup();
        fs::write(vault.join("Spec.md"), "safe\n").unwrap();
        let first = generate_specification_id();
        registry
            .approve(&project, &vault, &first, "Spec.md")
            .unwrap();
        let second = generate_specification_id();
        assert!(matches!(
            registry.approve(&project, &vault, &second, "Spec.md"),
            Err(LeyCoreError::InvalidSpecificationRequest(_))
        ));
        assert!(matches!(
            registry.approve(&project, &vault, &second, "../Spec.md"),
            Err(LeyCoreError::InvalidSpecificationRequest(_))
        ));
        assert!(matches!(
            registry.approve(&project, &vault, &second, ".trash/Spec.md"),
            Err(LeyCoreError::InvalidSpecificationRequest(_))
        ));

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let outside = base.path().join("outside.md");
            fs::write(&outside, "outside\n").unwrap();
            symlink(&outside, vault.join("Linked.md")).unwrap();
            assert!(registry
                .approve(&project, &vault, &second, "Linked.md")
                .is_err());
        }
    }

    #[test]
    fn concurrent_approvals_do_not_overwrite_each_other() {
        use std::sync::{Arc, Barrier};

        let (_base, project, vault, registry) = setup();
        let barrier = Arc::new(Barrier::new(8));
        let mut workers = Vec::new();
        for index in 0..8 {
            let relative = format!("Spec-{index}.md");
            fs::write(vault.join(&relative), format!("# Spec {index}\n")).unwrap();
            let worker_registry = registry.clone();
            let worker_project = project.clone();
            let worker_vault = vault.clone();
            let worker_barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                worker_barrier.wait();
                worker_registry
                    .approve(
                        worker_project,
                        worker_vault,
                        &generate_specification_id(),
                        &relative,
                    )
                    .unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        let listed = registry.list(&project, &vault).unwrap();
        assert_eq!(listed.specifications.len(), 8);
        assert_eq!(listed.current, 8);
    }

    #[test]
    fn authority_reads_serialize_with_revocation() {
        use std::sync::mpsc;
        use std::time::Duration;

        let (_base, project, vault, registry) = setup();
        fs::write(vault.join("Spec.md"), "# Requirement\n").unwrap();
        let specification_id = generate_specification_id();
        registry
            .approve(&project, &vault, &specification_id, "Spec.md")
            .unwrap();

        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let reader_registry = registry.clone();
        let reader_project = project.clone();
        let reader_vault = vault.clone();
        let reader = std::thread::spawn(move || {
            reader_registry
                .with_task_scan_locked(reader_project, reader_vault, "requirement", |_scan| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap();
        });
        entered_rx.recv().unwrap();

        let (revoked_tx, revoked_rx) = mpsc::channel();
        let revoke_registry = registry.clone();
        let revoke_project = project.clone();
        let revoke_id = specification_id.clone();
        let revoker = std::thread::spawn(move || {
            revoke_registry.revoke(revoke_project, &revoke_id).unwrap();
            revoked_tx.send(()).unwrap();
        });
        assert!(revoked_rx.recv_timeout(Duration::from_millis(50)).is_err());

        release_tx.send(()).unwrap();
        reader.join().unwrap();
        revoked_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        revoker.join().unwrap();
        assert!(matches!(
            registry.read_approved_source(&project, &vault, &specification_id),
            Err(LeyCoreError::SpecificationNotApproved(_))
        ));
    }

    #[test]
    fn task_scan_ignores_generic_task_words_and_keeps_meaningful_relevance() {
        let (_base, project, vault, registry) = setup();
        fs::write(
            vault.join("Login.md"),
            "# Authentication\n\nLogin must work without network access.\n",
        )
        .unwrap();
        fs::write(
            vault.join("Billing.md"),
            "# Billing\n\nThe system should support monthly invoices.\n",
        )
        .unwrap();
        let login_id = generate_specification_id();
        registry
            .approve(&project, &vault, &login_id, "Login.md")
            .unwrap();
        registry
            .approve(&project, &vault, &generate_specification_id(), "Billing.md")
            .unwrap();

        let scan = registry
            .task_scan(&project, &vault, "fix the login bug")
            .unwrap();
        assert_eq!(scan.candidates.len(), 1);
        assert_eq!(scan.candidates[0].source.specification_id, login_id);
        assert_eq!(scan.low_relevance_approved, 1);
    }

    #[test]
    fn context_returns_only_whole_current_approved_revisions() {
        let (_base, project, vault, registry) = setup();
        fs::write(
            vault.join("Current.md"),
            "# Current\n\nMust remain exact.\n",
        )
        .unwrap();
        fs::write(vault.join("Changed.md"), "# Changed\n\nFirst revision.\n").unwrap();
        let current_id = generate_specification_id();
        let changed_id = generate_specification_id();
        registry
            .approve(&project, &vault, &current_id, "Current.md")
            .unwrap();
        registry
            .approve(&project, &vault, &changed_id, "Changed.md")
            .unwrap();
        fs::write(vault.join("Changed.md"), "# Changed\n\nSecond revision.\n").unwrap();

        let context = registry
            .context(
                &project,
                &vault,
                SpecificationContextLimits {
                    max_results: 8,
                    max_characters: 4_000,
                },
            )
            .unwrap();
        assert_eq!(context.total_approved, 2);
        assert_eq!(context.current_approved, 1);
        assert_eq!(context.changed_approved, 1);
        assert_eq!(context.specifications.len(), 1);
        assert_eq!(context.specifications[0].specification_id, current_id);
        assert!(context.specifications[0]
            .source
            .contains("Must remain exact"));
        assert!(context.exclusions.iter().any(|item| {
            item.specification_id == changed_id
                && item.reason == SpecificationContextExclusionReason::Changed
        }));
        assert!(context.specification_source_revision_checked);
        assert!(!context.project_live_source_checked);
    }

    #[test]
    fn context_budget_omits_an_entire_specification_instead_of_truncating_it() {
        let (_base, project, vault, registry) = setup();
        fs::write(vault.join("Small.md"), "s".repeat(900)).unwrap();
        fs::write(vault.join("Large.md"), "l".repeat(900)).unwrap();
        let small_id = generate_specification_id();
        let large_id = generate_specification_id();
        registry
            .approve(&project, &vault, &small_id, "Small.md")
            .unwrap();
        registry
            .approve(&project, &vault, &large_id, "Large.md")
            .unwrap();
        let context = registry
            .context(
                &project,
                &vault,
                SpecificationContextLimits {
                    max_results: 8,
                    max_characters: 1_000,
                },
            )
            .unwrap();
        assert_eq!(context.specifications.len(), 1);
        assert_eq!(context.specifications[0].source.chars().count(), 900);
        assert!(context
            .exclusions
            .iter()
            .any(|item| { item.reason == SpecificationContextExclusionReason::CharacterBudget }));
        assert_eq!(context.text_characters, 900);
        assert!(context.truncated);
    }

    #[test]
    fn agent_context_checks_specification_egress_before_opening_source() {
        let (base, project, vault, registry) = setup();
        let relative_path = "Private.md";
        let marker = "private_specification_egress_marker";
        fs::write(
            vault.join(relative_path),
            format!("# Private\n\n{marker}\n"),
        )
        .unwrap();
        let specification_id = generate_specification_id();
        registry
            .approve(&project, &vault, &specification_id, relative_path)
            .unwrap();
        let egress = EgressPolicyRegistry::at(base.path().join("config/egress.json"));
        egress
            .set_specification_policy(
                &project,
                &specification_id,
                AgentEgressPolicy::LocalModelOnly,
            )
            .unwrap();
        let limits = SpecificationContextLimits {
            max_results: 8,
            max_characters: 4_000,
        };

        let cloud = registry
            .context_for_agent(&project, &vault, limits, &egress, AgentEgressTarget::Cloud)
            .unwrap();
        assert!(cloud.specifications.is_empty());
        assert!(cloud.exclusions.is_empty());
        assert_eq!(cloud.total_approved, 0);
        assert_eq!(cloud.egress_target, Some(AgentEgressTarget::Cloud));
        assert_eq!(
            cloud
                .egress_coverage
                .as_ref()
                .unwrap()
                .blocked_specifications,
            1
        );
        assert_eq!(cloud.egress_exclusions.len(), 1);
        assert_eq!(
            cloud.egress_exclusions[0].specification_id,
            specification_id
        );
        assert_eq!(
            cloud.egress_exclusions[0].block_reason,
            AgentEgressBlockReason::LocalModelOnly
        );
        let serialized = serde_json::to_string(&cloud).unwrap();
        assert!(!serialized.contains(marker));
        assert!(!serialized.contains(relative_path));

        let local = registry
            .context_for_agent(&project, &vault, limits, &egress, AgentEgressTarget::Local)
            .unwrap();
        assert_eq!(local.specifications.len(), 1);
        assert!(local.specifications[0].source.contains(marker));
        assert!(local.egress_exclusions.is_empty());

        egress
            .set_specification_policy(&project, &specification_id, AgentEgressPolicy::NeverSend)
            .unwrap();
        fs::remove_file(vault.join(relative_path)).unwrap();
        let blocked_missing = registry
            .context_for_agent(&project, &vault, limits, &egress, AgentEgressTarget::Local)
            .unwrap();
        assert!(blocked_missing.specifications.is_empty());
        assert!(blocked_missing.exclusions.is_empty());
        assert_eq!(blocked_missing.missing_approved, 0);
        assert_eq!(
            blocked_missing.egress_exclusions[0].block_reason,
            AgentEgressBlockReason::NeverSend
        );
    }

    #[cfg(unix)]
    #[test]
    fn registry_files_are_private_and_registry_symlinks_are_rejected() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let (base, project, vault, registry) = setup();
        fs::write(vault.join("Spec.md"), "safe\n").unwrap();
        registry
            .approve(&project, &vault, &generate_specification_id(), "Spec.md")
            .unwrap();
        assert_eq!(
            fs::metadata(registry.path()).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(registry.lock_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );

        fs::remove_file(registry.path()).unwrap();
        let outside = base.path().join("outside.json");
        fs::write(&outside, b"do not replace").unwrap();
        symlink(&outside, registry.path()).unwrap();
        assert!(matches!(
            registry.approve(&project, &vault, &generate_specification_id(), "Spec.md"),
            Err(LeyCoreError::UnsafeProjectLayout(_))
        ));
        assert_eq!(fs::read_to_string(outside).unwrap(), "do not replace");
    }
}
