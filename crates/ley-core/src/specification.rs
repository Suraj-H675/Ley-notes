use crate::project_memory_search::lexical_score;
use crate::{
    default_binding_registry_path, diagnose_project, validate_project_id, LeyCoreError,
    METADATA_FILE_LIMIT_BYTES,
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

pub const DEFAULT_SPECIFICATION_CONTEXT_RESULTS: usize = 8;
pub const MAX_SPECIFICATION_CONTEXT_RESULTS: usize = 20;
pub const DEFAULT_SPECIFICATION_CONTEXT_CHARACTERS: usize = 16_000;
pub const MIN_SPECIFICATION_CONTEXT_CHARACTERS: usize = 1_000;
pub const MAX_SPECIFICATION_CONTEXT_CHARACTERS: usize = 64_000;
const MAX_SPECIFICATION_CONTEXT_EXCLUSIONS: usize = 40;

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
pub struct ProjectSpecificationsContext {
    pub project_id: String,
    pub max_characters: usize,
    pub text_characters: usize,
    pub specifications: Vec<SpecificationContextItem>,
    pub exclusions: Vec<SpecificationContextExclusion>,
    pub total_approved: usize,
    pub current_approved: usize,
    pub changed_approved: usize,
    pub missing_approved: usize,
    pub omitted_specifications: usize,
    pub omitted_exclusions: usize,
    pub truncated: bool,
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
        let project_id = diagnostic.identity.project_id;
        self.with_locked_document(|document| {
            let entry = document
                .approvals
                .get(&project_id)
                .and_then(|approvals| approvals.get(specification_id))
                .ok_or_else(|| {
                    LeyCoreError::SpecificationNotApproved(specification_id.to_owned())
                })?;
            let source = read_stable_specification_bytes(vault.as_ref(), &entry.relative_path)?;
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
                project_id: project_id.clone(),
                specification_id: specification_id.to_owned(),
                relative_path: entry.relative_path.clone(),
                content_hash,
                approved_at_unix_ms: entry.approved_at_unix_ms,
                source,
                source_boundary: "user-approved-specification",
                authority: "human-intent",
            })
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
            let total_approved = approvals.len();
            let mut candidates = Vec::new();
            let mut exclusions = Vec::new();
            let mut current_approved = 0;
            let mut changed_approved = 0;
            let mut missing_approved = 0;
            let mut low_relevance_approved = 0;

            for (specification_id, entry) in approvals {
                let source =
                    match read_stable_specification_bytes(vault.as_ref(), &entry.relative_path) {
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

            operation(TaskSpecificationScan {
                project_id: project_id.clone(),
                candidates,
                exclusions,
                total_approved,
                current_approved,
                changed_approved,
                missing_approved,
                low_relevance_approved,
            })
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
            text_characters += characters;
            specifications.push(SpecificationContextItem {
                specification_id: approved.specification_id,
                relative_path: approved.relative_path,
                content_hash: approved.content_hash,
                approved_at_unix_ms: approved.approved_at_unix_ms,
                source: approved.source,
                characters,
                authority: approved.authority,
                source_boundary: approved.source_boundary,
            });
        }
        let omitted_specifications = total_approved.saturating_sub(specifications.len());
        Ok(ProjectSpecificationsContext {
            project_id: authority.project_id,
            max_characters: limits.max_characters,
            text_characters,
            specifications,
            exclusions,
            total_approved,
            current_approved: authority.current,
            changed_approved: authority.changed,
            missing_approved: authority.missing,
            omitted_specifications,
            omitted_exclusions,
            truncated: omitted_specifications > 0 || omitted_exclusions > 0,
            project_live_source_checked: false,
            specification_source_revision_checked: true,
            authority: "human-intent",
            source_boundary: "user-approved-specification",
            instruction_warning: SPECIFICATION_INSTRUCTION_WARNING,
            privacy_notice: PRIVACY_NOTICE,
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

fn specification_task_terms(query: &str) -> Vec<String> {
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

pub fn generate_specification_id() -> String {
    format!("spec_{}", Uuid::new_v4().simple())
}

fn validate_specification_id(value: &str) -> Result<(), String> {
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
