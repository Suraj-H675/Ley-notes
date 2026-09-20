use crate::project_memory_search::lexical_score;
use crate::specification::{specification_task_terms, validate_specification_id};
use crate::{
    canonical_directory, default_binding_registry_path, diagnose_project, evaluate_agent_egress,
    initialize_project_uncoordinated, validate_project_id, AgentEgressBlockReason,
    AgentEgressPolicy, AgentEgressTarget, BindingRegistry, CaptureMode, ContextCompileLimits,
    EgressPolicyRegistry, LeyCoreError, ProjectCatalog, ProjectInitialization,
    SpecificationRegistry, BINDING_REGISTRY_FILE, MAX_CONTEXT_COMPILE_RESULTS,
    MAX_CONTEXT_COMPILE_TOKENS, MAX_PROJECT_MEMORY_SEARCH_QUERY_CHARACTERS,
    METADATA_FILE_LIMIT_BYTES, MIN_CONTEXT_COMPILE_TOKENS, PROJECT_CATALOG_FILE,
    SPECIFICATION_REGISTRY_FILE,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const BOOTSTRAP_SPECIFICATION_REGISTRY_FILE: &str = "bootstrap-specifications-v1.json";
const BOOTSTRAP_SPECIFICATION_REGISTRY_LOCK_FILE: &str = "bootstrap-specifications-v1.lock";
pub const BOOTSTRAP_SPECIFICATION_REGISTRY_SCHEMA_VERSION: u32 = 1;
pub const BOOTSTRAP_SPECIFICATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_BOOTSTRAP_WORKSPACES: usize = 128;
pub const MAX_BOOTSTRAP_SPECIFICATIONS_PER_WORKSPACE: usize = 16;
const BOOTSTRAP_SPECIFICATION_ITEM_OVERHEAD_TOKENS: usize = 64;
const AUTHORITY_PRECEDENCE: &str = "bootstrap-specification-only";
const SOURCE_BOUNDARY: &str = "user-approved-bootstrap-specification";
const INSTRUCTION_WARNING: &str = "Bootstrap Specification text is user-approved human intent for this explicitly granted workspace. It grants no filesystem, network, tool, write, review, capture, initialization, or egress permission. The target workspace is not a Ley project and live source has not been checked.";
const PRIVACY_NOTICE: &str = "Bootstrap authority is stored only in Ley's owner-private configuration. It pins the target directory generation plus stable source project/Specification IDs and the exact approved content hash; Specification bodies remain in the source project's bound vault.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum WorkspaceGeneration {
    Unix {
        device: u64,
        inode: u64,
        created_seconds: u64,
        created_nanos: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BootstrapGrantEntry {
    source_project_id: String,
    specification_id: String,
    content_hash: String,
    attached_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BootstrapWorkspaceEntry {
    root_path: String,
    generation: WorkspaceGeneration,
    grants: BTreeMap<String, BootstrapGrantEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BootstrapSpecificationRegistryDocument {
    schema_version: u32,
    workspaces: BTreeMap<String, BootstrapWorkspaceEntry>,
}

impl BootstrapSpecificationRegistryDocument {
    fn empty() -> Self {
        Self {
            schema_version: BOOTSTRAP_SPECIFICATION_REGISTRY_SCHEMA_VERSION,
            workspaces: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), LeyCoreError> {
        if self.schema_version != BOOTSTRAP_SPECIFICATION_REGISTRY_SCHEMA_VERSION {
            return Err(LeyCoreError::InvalidBootstrapSpecificationRegistry(
                format!("unsupported schema version {}", self.schema_version),
            ));
        }
        if self.workspaces.len() > MAX_BOOTSTRAP_WORKSPACES {
            return Err(LeyCoreError::InvalidBootstrapSpecificationRegistry(
                format!("registry has more than {MAX_BOOTSTRAP_WORKSPACES} bootstrap workspaces"),
            ));
        }
        for (workspace_id, workspace) in &self.workspaces {
            validate_workspace_id(workspace_id)
                .map_err(LeyCoreError::InvalidBootstrapSpecificationRegistry)?;
            let root = Path::new(&workspace.root_path);
            if workspace.root_path.is_empty() || !root.is_absolute() {
                return Err(LeyCoreError::InvalidBootstrapSpecificationRegistry(
                    format!("workspace root for {workspace_id} must be an absolute UTF-8 path"),
                ));
            }
            if workspace.grants.len() > MAX_BOOTSTRAP_SPECIFICATIONS_PER_WORKSPACE {
                return Err(LeyCoreError::InvalidBootstrapSpecificationRegistry(format!(
                    "workspace {workspace_id} has more than {MAX_BOOTSTRAP_SPECIFICATIONS_PER_WORKSPACE} bootstrap Specifications"
                )));
            }
            for (grant_id, grant) in &workspace.grants {
                validate_grant_id(grant_id)
                    .map_err(LeyCoreError::InvalidBootstrapSpecificationRegistry)?;
                validate_project_id(&grant.source_project_id).map_err(|error| {
                    LeyCoreError::InvalidBootstrapSpecificationRegistry(error.to_string())
                })?;
                validate_specification_id(&grant.specification_id)
                    .map_err(LeyCoreError::InvalidBootstrapSpecificationRegistry)?;
                validate_content_hash(&grant.content_hash)
                    .map_err(LeyCoreError::InvalidBootstrapSpecificationRegistry)?;
                if grant.attached_at_unix_ms == 0 {
                    return Err(LeyCoreError::InvalidBootstrapSpecificationRegistry(
                        format!("grant {grant_id} attachment time must be non-zero"),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSpecificationGrant {
    pub workspace_id: String,
    pub grant_id: String,
    pub source_project_id: String,
    pub specification_id: String,
    pub content_hash: String,
    pub attached_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSpecificationMutation {
    pub grant: BootstrapSpecificationGrant,
    pub created: bool,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSpecificationList {
    pub workspace_id: String,
    pub grants: Vec<BootstrapSpecificationGrant>,
    pub total_grants: usize,
    pub target_initialized: bool,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BootstrapCompileExclusionReason {
    SourceProjectUnavailable,
    SourceIdentityChanged,
    SourceVaultUnavailable,
    SpecificationNotApproved,
    SpecificationUnavailable,
    SpecificationRevisionChanged,
    EgressBlocked,
    LowRelevance,
    ResultLimit,
    TokenBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapCompileExclusion {
    pub grant_id: String,
    pub source_project_id: String,
    pub specification_id: String,
    pub reason: BootstrapCompileExclusionReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<AgentEgressPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<AgentEgressBlockReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapCompiledSpecification {
    pub grant_id: String,
    pub source_project_id: String,
    pub source_project_name: String,
    pub specification_id: String,
    pub relative_path: String,
    pub content_hash: String,
    pub approved_at_unix_ms: u64,
    pub source: String,
    pub relevance_score: u32,
    pub exact_match: bool,
    pub authority: &'static str,
    pub source_boundary: &'static str,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapCompileCoverage {
    pub attached_grants: usize,
    pub resolved_sources: usize,
    pub unavailable_sources: usize,
    pub stale_specifications: usize,
    pub egress_blocked: usize,
    pub low_relevance: usize,
    pub relevant_candidates: usize,
    pub returned_specifications: usize,
    pub omitted_by_result_limit: usize,
    pub omitted_by_token_budget: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapSpecificationContext {
    pub schema_version: u32,
    pub workspace_id: String,
    pub task: String,
    pub egress_target: AgentEgressTarget,
    pub max_results: usize,
    pub max_tokens: usize,
    pub estimated_tokens: usize,
    pub specifications: Vec<BootstrapCompiledSpecification>,
    pub exclusions: Vec<BootstrapCompileExclusion>,
    pub coverage: BootstrapCompileCoverage,
    pub authority_precedence: &'static str,
    pub target_initialized: bool,
    pub project_memory_available: bool,
    pub live_source_checked: bool,
    pub persisted: bool,
    pub automatic_write_allowed: bool,
    pub instruction_warning: &'static str,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone)]
pub struct BootstrapSpecificationRegistry {
    path: PathBuf,
    project_catalog: ProjectCatalog,
    binding_registry: BindingRegistry,
    specification_registry: SpecificationRegistry,
}

impl BootstrapSpecificationRegistry {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        Ok(Self::at(
            default_binding_registry_path()?.with_file_name(BOOTSTRAP_SPECIFICATION_REGISTRY_FILE),
        ))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            project_catalog: ProjectCatalog::at(path.with_file_name(PROJECT_CATALOG_FILE)),
            binding_registry: BindingRegistry::at(path.with_file_name(BINDING_REGISTRY_FILE)),
            specification_registry: SpecificationRegistry::at(
                path.with_file_name(SPECIFICATION_REGISTRY_FILE),
            ),
            path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn authority_file_present(&self) -> Result<bool, LeyCoreError> {
        match fs::symlink_metadata(&self.path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(source) => Err(LeyCoreError::Io {
                path: self.path.clone(),
                source,
            }),
        }
    }

    pub fn attach(
        &self,
        workspace: impl AsRef<Path>,
        source_project: impl AsRef<Path>,
        specification_id: &str,
    ) -> Result<BootstrapSpecificationMutation, LeyCoreError> {
        validate_specification_id(specification_id)
            .map_err(LeyCoreError::InvalidBootstrapSpecificationRequest)?;
        let workspace_path = workspace.as_ref().to_path_buf();
        let source_project = source_project.as_ref().to_path_buf();
        let requested_specification_id = specification_id.to_owned();
        let (
            workspace,
            grant_id,
            source_project_id,
            specification_id,
            content_hash,
            created,
            attached_at_unix_ms,
        ) = with_target_directory_lock(&workspace_path, |locked_root| {
            self.mutate(|document| {
                let workspace = bootstrap_workspace_identity(locked_root)?;
                ensure_target_uninitialized(&workspace.root)?;
                let source_diagnostic = diagnose_project(&source_project)?;
                let source = self.binding_registry.with_resolved_observed_locked(
                    &source_diagnostic,
                    |binding| {
                        self.specification_registry.read_approved_source(
                            &source_diagnostic.root,
                            &binding.vault_path,
                            &requested_specification_id,
                        )
                    },
                )?;
                let grant_id = grant_id(
                    &workspace.workspace_id,
                    &source.project_id,
                    &source.specification_id,
                );
                let now = unix_time_ms();
                let source_project_id = source.project_id.clone();
                let specification_id = source.specification_id.clone();
                let content_hash = source.content_hash.clone();
                let root_string = path_string(&workspace.root)?;
                let generation = workspace.generation.clone();
                document.workspaces.retain(|id, entry| {
                    id == &workspace.workspace_id || entry.root_path != root_string
                });
                if !document.workspaces.contains_key(&workspace.workspace_id)
                    && document.workspaces.len() >= MAX_BOOTSTRAP_WORKSPACES
                {
                    return Err(LeyCoreError::InvalidBootstrapSpecificationRequest(format!(
                        "Ley can retain bootstrap authority for at most {MAX_BOOTSTRAP_WORKSPACES} workspaces"
                    )));
                }
                let target = document
                    .workspaces
                    .entry(workspace.workspace_id.clone())
                    .or_insert_with(|| BootstrapWorkspaceEntry {
                        root_path: root_string.clone(),
                        generation: generation.clone(),
                        grants: BTreeMap::new(),
                    });
                if target.root_path != root_string || target.generation != generation {
                    return Err(LeyCoreError::InvalidBootstrapSpecificationRequest(
                        "bootstrap workspace identity changed; attach again for the current directory generation"
                            .to_owned(),
                    ));
                }
                if let Some(existing) = target.grants.get(&grant_id) {
                    if existing.content_hash == content_hash {
                        return Ok((
                            workspace,
                            grant_id,
                            source_project_id,
                            specification_id,
                            content_hash,
                            false,
                            existing.attached_at_unix_ms,
                        ));
                    }
                }
                if !target.grants.contains_key(&grant_id)
                    && target.grants.len() >= MAX_BOOTSTRAP_SPECIFICATIONS_PER_WORKSPACE
                {
                    return Err(LeyCoreError::InvalidBootstrapSpecificationRequest(format!(
                        "a bootstrap workspace may have at most {MAX_BOOTSTRAP_SPECIFICATIONS_PER_WORKSPACE} Specifications"
                    )));
                }
                target.grants.insert(
                    grant_id.clone(),
                    BootstrapGrantEntry {
                        source_project_id: source_project_id.clone(),
                        specification_id: specification_id.clone(),
                        content_hash: content_hash.clone(),
                        attached_at_unix_ms: now,
                    },
                );
                Ok((
                    workspace,
                    grant_id,
                    source_project_id,
                    specification_id,
                    content_hash,
                    true,
                    now,
                ))
            })
        })?;
        Ok(BootstrapSpecificationMutation {
            grant: BootstrapSpecificationGrant {
                workspace_id: workspace.workspace_id,
                grant_id,
                source_project_id,
                specification_id,
                content_hash,
                attached_at_unix_ms,
            },
            created,
            privacy_notice: PRIVACY_NOTICE,
        })
    }

    pub fn list(
        &self,
        workspace: impl AsRef<Path>,
    ) -> Result<BootstrapSpecificationList, LeyCoreError> {
        let workspace_path = workspace.as_ref().to_path_buf();
        if !self.authority_file_present()? {
            let workspace = bootstrap_workspace_identity(&workspace_path)?;
            return Ok(BootstrapSpecificationList {
                workspace_id: workspace.workspace_id,
                grants: Vec::new(),
                total_grants: 0,
                target_initialized: diagnose_project(&workspace.root).is_ok(),
                privacy_notice: PRIVACY_NOTICE,
            });
        }
        let lock = self.acquire_lock()?;
        let result = (|| {
            let workspace = bootstrap_workspace_identity(&workspace_path)?;
            let initialized = diagnose_project(&workspace.root).is_ok();
            let document = self.read_document()?;
            let grants = document
                .workspaces
                .get(&workspace.workspace_id)
                .filter(|entry| {
                    entry.root_path == path_string(&workspace.root).unwrap_or_default()
                        && entry.generation == workspace.generation
                })
                .map(|entry| {
                    entry
                        .grants
                        .iter()
                        .map(|(grant_id, grant)| BootstrapSpecificationGrant {
                            workspace_id: workspace.workspace_id.clone(),
                            grant_id: grant_id.clone(),
                            source_project_id: grant.source_project_id.clone(),
                            specification_id: grant.specification_id.clone(),
                            content_hash: grant.content_hash.clone(),
                            attached_at_unix_ms: grant.attached_at_unix_ms,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Ok(BootstrapSpecificationList {
                workspace_id: workspace.workspace_id,
                total_grants: grants.len(),
                grants,
                target_initialized: initialized,
                privacy_notice: PRIVACY_NOTICE,
            })
        })();
        unlock_bootstrap_result(lock, self.lock_path(), result)
    }

    pub fn detach(
        &self,
        workspace: impl AsRef<Path>,
        grant_id: &str,
    ) -> Result<Option<BootstrapSpecificationGrant>, LeyCoreError> {
        validate_grant_id(grant_id).map_err(LeyCoreError::InvalidBootstrapSpecificationRequest)?;
        let workspace_path = workspace.as_ref().to_path_buf();
        let removed = self.mutate(|document| {
            let workspace = bootstrap_workspace_identity(&workspace_path)?;
            let Some(entry) = document.workspaces.get_mut(&workspace.workspace_id) else {
                return Ok(None);
            };
            if entry.generation != workspace.generation {
                return Ok(None);
            }
            let removed = entry.grants.remove(grant_id);
            if entry.grants.is_empty() {
                document.workspaces.remove(&workspace.workspace_id);
            }
            Ok(removed.map(|grant| (workspace.workspace_id.clone(), grant)))
        })?;
        Ok(
            removed.map(|(workspace_id, grant)| BootstrapSpecificationGrant {
                workspace_id,
                grant_id: grant_id.to_owned(),
                source_project_id: grant.source_project_id,
                specification_id: grant.specification_id,
                content_hash: grant.content_hash,
                attached_at_unix_ms: grant.attached_at_unix_ms,
            }),
        )
    }

    pub fn retire(&self, workspace: impl AsRef<Path>) -> Result<usize, LeyCoreError> {
        let workspace_path = workspace.as_ref().to_path_buf();
        self.mutate(|document| {
            let workspace = bootstrap_workspace_identity(&workspace_path)?;
            Ok(document
                .workspaces
                .remove(&workspace.workspace_id)
                .map_or(0, |entry| entry.grants.len()))
        })
    }

    fn with_current_workspace_locked<T>(
        &self,
        workspace: &Path,
        operation: impl FnOnce(
            &BootstrapWorkspaceIdentity,
            &BootstrapWorkspaceEntry,
        ) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        with_target_directory_lock(workspace, |locked_root| {
            let lock = self.acquire_lock()?;
            let result = (|| {
                let workspace = bootstrap_workspace_identity(locked_root)?;
                ensure_target_uninitialized(&workspace.root)?;
                let document = self.read_document()?;
                let entry = document
                    .workspaces
                    .get(&workspace.workspace_id)
                    .ok_or_else(|| {
                        LeyCoreError::InvalidBootstrapSpecificationRequest(
                            "no bootstrap Specifications are attached to this workspace".to_owned(),
                        )
                    })?;
                if entry.generation != workspace.generation
                    || entry.root_path != path_string(&workspace.root)?
                {
                    return Err(LeyCoreError::InvalidBootstrapSpecificationRequest(
                        "bootstrap workspace directory generation changed; attach Specifications again"
                            .to_owned(),
                    ));
                }
                let value = operation(&workspace, entry)?;
                let current = bootstrap_workspace_identity(&workspace.root)?;
                ensure_target_uninitialized(&current.root)?;
                if current.workspace_id != workspace.workspace_id
                    || current.generation != workspace.generation
                    || current.root != workspace.root
                {
                    return Err(LeyCoreError::InvalidBootstrapSpecificationRequest(
                        "bootstrap workspace directory generation changed during context compilation; retry after reviewing current authority"
                            .to_owned(),
                    ));
                }
                Ok(value)
            })();
            unlock_bootstrap_result(lock, self.lock_path(), result)
        })
    }

    fn initialize_retiring(
        &self,
        workspace: &Path,
        requested_name: Option<&str>,
        mode: CaptureMode,
    ) -> Result<ProjectInitialization, LeyCoreError> {
        self.initialize_retiring_with(workspace, |root| {
            initialize_project_uncoordinated(root, requested_name, mode)
        })
    }

    fn initialize_retiring_with(
        &self,
        workspace: &Path,
        initialize: impl FnOnce(&Path) -> Result<ProjectInitialization, LeyCoreError>,
    ) -> Result<ProjectInitialization, LeyCoreError> {
        with_target_directory_lock(workspace, |locked_root| {
            if !self.authority_file_present()? {
                return initialize(locked_root);
            }
            let identity = match bootstrap_workspace_identity(locked_root) {
                Ok(identity) => identity,
                Err(LeyCoreError::BootstrapWorkspaceGenerationUnavailable) => {
                    return initialize(locked_root)
                }
                Err(error) => return Err(error),
            };
            let lock = self.acquire_lock()?;
            let result = (|| {
                let mut document = self.read_document()?;
                let removed = document.workspaces.remove(&identity.workspace_id);
                if removed.is_some() {
                    document.validate()?;
                    self.write_document(&document)?;
                }
                match initialize(&identity.root) {
                    Ok(initialized) => Ok(initialized),
                    Err(error) => {
                        if let Some(entry) = removed {
                            document
                                .workspaces
                                .insert(identity.workspace_id.clone(), entry);
                            if self.write_document(&document).is_err() {
                                return Err(LeyCoreError::BootstrapSpecificationRestorationFailed);
                            }
                        }
                        Err(error)
                    }
                }
            })();
            unlock_bootstrap_result(lock, self.lock_path(), result)
        })
    }

    fn resolve_source_identity(
        &self,
        source_project_id: &str,
    ) -> Result<(crate::ProjectDiagnostic, String), BootstrapCompileExclusionReason> {
        let observed = self
            .project_catalog
            .get(source_project_id)
            .map_err(|_| BootstrapCompileExclusionReason::SourceProjectUnavailable)?
            .ok_or(BootstrapCompileExclusionReason::SourceProjectUnavailable)?;
        let diagnostic = diagnose_project(&observed.root_path)
            .map_err(|_| BootstrapCompileExclusionReason::SourceProjectUnavailable)?;
        if diagnostic.root != observed.root_path
            || diagnostic.identity.project_id != source_project_id
        {
            return Err(BootstrapCompileExclusionReason::SourceIdentityChanged);
        }
        let source_name = diagnostic.identity.name.clone();
        Ok((diagnostic, source_name))
    }

    fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut BootstrapSpecificationRegistryDocument) -> Result<T, LeyCoreError>,
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
        let mut options = OpenOptions::new();
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
            LeyCoreError::InvalidBootstrapSpecificationRegistry(
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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(LeyCoreError::InvalidBootstrapSpecificationRegistry(
                    "bootstrap authority requires an owner-private configuration directory"
                        .to_owned(),
                ));
            }
        }
        Ok(())
    }

    fn read_document(&self) -> Result<BootstrapSpecificationRegistryDocument, LeyCoreError> {
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BootstrapSpecificationRegistryDocument::empty())
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
        let document: BootstrapSpecificationRegistryDocument = serde_json::from_slice(&bytes)
            .map_err(|source| LeyCoreError::Json {
                path: self.path.clone(),
                source,
            })?;
        document.validate()?;
        Ok(document)
    }

    fn write_document(
        &self,
        document: &BootstrapSpecificationRegistryDocument,
    ) -> Result<(), LeyCoreError> {
        reject_non_regular_if_present(&self.path)?;
        let mut body = serde_json::to_vec_pretty(document)
            .expect("validated bootstrap Specification registry is serializable");
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
        self.path
            .with_file_name(BOOTSTRAP_SPECIFICATION_REGISTRY_LOCK_FILE)
    }
}

#[derive(Debug, Clone)]
struct BootstrapWorkspaceIdentity {
    workspace_id: String,
    root: PathBuf,
    generation: WorkspaceGeneration,
}

#[derive(Debug, Clone)]
struct BootstrapCandidate {
    grant_id: String,
    source_project_id: String,
    source_project_name: String,
    specification_id: String,
    relative_path: String,
    content_hash: String,
    approved_at_unix_ms: u64,
    source: String,
    relevance_score: u32,
    exact_match: bool,
    estimated_tokens: usize,
}

pub fn compile_bootstrap_specifications(
    workspace: impl AsRef<Path>,
    task: &str,
    limits: ContextCompileLimits,
    target: AgentEgressTarget,
) -> Result<BootstrapSpecificationContext, LeyCoreError> {
    let registry = BootstrapSpecificationRegistry::system_default()?;
    let egress = EgressPolicyRegistry::system_default()?;
    compile_bootstrap_specifications_with_registries(
        workspace.as_ref(),
        task,
        limits,
        target,
        &registry,
        &egress,
    )
}

pub fn compile_bootstrap_specifications_with_registries(
    workspace: &Path,
    task: &str,
    limits: ContextCompileLimits,
    target: AgentEgressTarget,
    registry: &BootstrapSpecificationRegistry,
    egress: &EgressPolicyRegistry,
) -> Result<BootstrapSpecificationContext, LeyCoreError> {
    validate_compile_limits(task, limits)?;
    let normalized_task = task.trim().to_lowercase();
    let terms = specification_task_terms(&normalized_task);
    registry.with_current_workspace_locked(workspace, |workspace, entry| {
        let mut coverage = BootstrapCompileCoverage {
            attached_grants: entry.grants.len(),
            ..Default::default()
        };
        let mut candidates = Vec::new();
        let mut exclusions = Vec::new();
        for (grant_id, grant) in &entry.grants {
            let (source_diagnostic, source_project_name) =
                match registry.resolve_source_identity(&grant.source_project_id) {
                    Ok(source) => source,
                    Err(reason) => {
                        coverage.unavailable_sources += 1;
                        exclusions.push(exclusion(grant_id, grant, reason, None));
                        continue;
                    }
                };
            coverage.resolved_sources += 1;
            let approved = registry.binding_registry.with_resolved_observed_locked(
                &source_diagnostic,
                |binding| {
                    egress.with_snapshot_locked(|policies| {
                        let project_decision = evaluate_agent_egress(
                            policies.project_policy(&grant.source_project_id),
                            target,
                        );
                        if !project_decision.allowed {
                            return Ok(Err((
                                project_decision.policy,
                                project_decision
                                    .block_reason
                                    .expect("blocked project egress has a reason"),
                            )));
                        }
                        let specification_decision = evaluate_agent_egress(
                            policies.specification_policy(
                                &grant.source_project_id,
                                &grant.specification_id,
                            ),
                            target,
                        );
                        if !specification_decision.allowed {
                            return Ok(Err((
                                specification_decision.policy,
                                specification_decision
                                    .block_reason
                                    .expect("blocked Specification egress has a reason"),
                            )));
                        }
                        Ok(Ok(registry
                            .specification_registry
                            .read_approved_source_for_expected_project(
                                &source_diagnostic.root,
                                &binding.vault_path,
                                &grant.source_project_id,
                                &grant.specification_id,
                            )))
                    })
                },
            );
            let approved = match approved {
                Ok(approved) => approved,
                Err(LeyCoreError::VaultNotBound(_))
                | Err(LeyCoreError::BoundVaultUnavailable { .. }) => {
                    coverage.unavailable_sources += 1;
                    exclusions.push(exclusion(
                        grant_id,
                        grant,
                        BootstrapCompileExclusionReason::SourceVaultUnavailable,
                        None,
                    ));
                    continue;
                }
                Err(LeyCoreError::ProjectNotFound(_))
                | Err(LeyCoreError::InvalidProjectIdentity(_)) => {
                    coverage.unavailable_sources += 1;
                    exclusions.push(exclusion(
                        grant_id,
                        grant,
                        BootstrapCompileExclusionReason::SourceIdentityChanged,
                        None,
                    ));
                    continue;
                }
                Err(error) => return Err(error),
            };
            let approved = match approved {
                Err((policy, block_reason)) => {
                    coverage.egress_blocked += 1;
                    exclusions.push(exclusion(
                        grant_id,
                        grant,
                        BootstrapCompileExclusionReason::EgressBlocked,
                        Some((policy, block_reason)),
                    ));
                    continue;
                }
                Ok(Ok(approved)) => approved,
                Ok(Err(LeyCoreError::SpecificationNotApproved(_))) => {
                    coverage.stale_specifications += 1;
                    exclusions.push(exclusion(
                        grant_id,
                        grant,
                        BootstrapCompileExclusionReason::SpecificationNotApproved,
                        None,
                    ));
                    continue;
                }
                Ok(Err(LeyCoreError::Io { source, .. }))
                    if source.kind() == std::io::ErrorKind::NotFound =>
                {
                    coverage.stale_specifications += 1;
                    exclusions.push(exclusion(
                        grant_id,
                        grant,
                        BootstrapCompileExclusionReason::SpecificationUnavailable,
                        None,
                    ));
                    continue;
                }
                Ok(Err(LeyCoreError::SpecificationApprovalStale { .. })) => {
                    coverage.stale_specifications += 1;
                    exclusions.push(exclusion(
                        grant_id,
                        grant,
                        BootstrapCompileExclusionReason::SpecificationRevisionChanged,
                        None,
                    ));
                    continue;
                }
                Ok(Err(LeyCoreError::InvalidProjectIdentity(_)))
                | Ok(Err(LeyCoreError::ProjectNotFound(_))) => {
                    coverage.unavailable_sources += 1;
                    exclusions.push(exclusion(
                        grant_id,
                        grant,
                        BootstrapCompileExclusionReason::SourceIdentityChanged,
                        None,
                    ));
                    continue;
                }
                Ok(Err(error)) => return Err(error),
            };
            if approved.content_hash != grant.content_hash {
                coverage.stale_specifications += 1;
                exclusions.push(exclusion(
                    grant_id,
                    grant,
                    BootstrapCompileExclusionReason::SpecificationRevisionChanged,
                    None,
                ));
                continue;
            }
            let searchable = format!("{}\n{}", approved.relative_path, approved.source);
            let (score, exact_match) = lexical_score(&searchable, &normalized_task, &terms);
            if score == 0 {
                coverage.low_relevance += 1;
                exclusions.push(exclusion(
                    grant_id,
                    grant,
                    BootstrapCompileExclusionReason::LowRelevance,
                    None,
                ));
                continue;
            }
            coverage.relevant_candidates += 1;
            let estimated_tokens =
                estimate_specification_tokens(&approved.relative_path, &approved.source);
            candidates.push(BootstrapCandidate {
                grant_id: grant_id.clone(),
                source_project_id: approved.project_id,
                source_project_name,
                specification_id: approved.specification_id,
                relative_path: approved.relative_path,
                content_hash: approved.content_hash,
                approved_at_unix_ms: approved.approved_at_unix_ms,
                source: approved.source,
                relevance_score: score,
                exact_match,
                estimated_tokens,
            });
        }
        candidates.sort_by(|left, right| {
            right
                .exact_match
                .cmp(&left.exact_match)
                .then_with(|| right.relevance_score.cmp(&left.relevance_score))
                .then_with(|| right.approved_at_unix_ms.cmp(&left.approved_at_unix_ms))
                .then_with(|| left.grant_id.cmp(&right.grant_id))
        });
        let mut specifications = Vec::new();
        let mut estimated_tokens = 0usize;
        for candidate in candidates {
            if specifications.len() >= limits.max_results {
                coverage.omitted_by_result_limit += 1;
                exclusions.push(candidate_exclusion(
                    &candidate,
                    BootstrapCompileExclusionReason::ResultLimit,
                ));
                continue;
            }
            if estimated_tokens.saturating_add(candidate.estimated_tokens) > limits.max_tokens {
                coverage.omitted_by_token_budget += 1;
                exclusions.push(candidate_exclusion(
                    &candidate,
                    BootstrapCompileExclusionReason::TokenBudget,
                ));
                continue;
            }
            estimated_tokens += candidate.estimated_tokens;
            specifications.push(BootstrapCompiledSpecification {
                grant_id: candidate.grant_id,
                source_project_id: candidate.source_project_id,
                source_project_name: candidate.source_project_name,
                specification_id: candidate.specification_id,
                relative_path: candidate.relative_path,
                content_hash: candidate.content_hash,
                approved_at_unix_ms: candidate.approved_at_unix_ms,
                source: candidate.source,
                relevance_score: candidate.relevance_score,
                exact_match: candidate.exact_match,
                authority: "human-intent",
                source_boundary: SOURCE_BOUNDARY,
                estimated_tokens: candidate.estimated_tokens,
            });
        }
        coverage.returned_specifications = specifications.len();
        Ok(BootstrapSpecificationContext {
            schema_version: BOOTSTRAP_SPECIFICATION_SCHEMA_VERSION,
            workspace_id: workspace.workspace_id.clone(),
            task: task.to_owned(),
            egress_target: target,
            max_results: limits.max_results,
            max_tokens: limits.max_tokens,
            estimated_tokens,
            specifications,
            exclusions,
            coverage,
            authority_precedence: AUTHORITY_PRECEDENCE,
            target_initialized: false,
            project_memory_available: false,
            live_source_checked: false,
            persisted: false,
            automatic_write_allowed: false,
            instruction_warning: INSTRUCTION_WARNING,
            privacy_notice: PRIVACY_NOTICE,
        })
    })
}

pub fn initialize_project_retiring_bootstrap(
    root: impl AsRef<Path>,
    requested_name: Option<&str>,
    mode: CaptureMode,
) -> Result<ProjectInitialization, LeyCoreError> {
    match BootstrapSpecificationRegistry::system_default() {
        Ok(registry) => registry.initialize_retiring(root.as_ref(), requested_name, mode),
        Err(LeyCoreError::ConfigDirectoryUnavailable) => {
            initialize_project_uncoordinated(root.as_ref(), requested_name, mode)
        }
        Err(error) => Err(error),
    }
}

fn bootstrap_workspace_identity(path: &Path) -> Result<BootstrapWorkspaceIdentity, LeyCoreError> {
    let root = canonical_directory(path)?;
    let metadata = fs::metadata(&root).map_err(|source| LeyCoreError::Io {
        path: root.clone(),
        source,
    })?;
    let generation = workspace_generation(&metadata)?;
    let root_string = path_string(&root)?;
    let generation_json =
        serde_json::to_vec(&generation).expect("bootstrap workspace generation is serializable");
    let mut digest = Sha256::new();
    digest.update(root_string.as_bytes());
    digest.update([0]);
    digest.update(generation_json);
    Ok(BootstrapWorkspaceIdentity {
        workspace_id: format!("bsw_{:x}", digest.finalize()),
        root,
        generation,
    })
}

#[cfg(unix)]
fn workspace_generation(metadata: &fs::Metadata) -> Result<WorkspaceGeneration, LeyCoreError> {
    use std::os::unix::fs::MetadataExt;
    let created = metadata
        .created()
        .map_err(|_| LeyCoreError::BootstrapWorkspaceGenerationUnavailable)?;
    let created = created
        .duration_since(UNIX_EPOCH)
        .map_err(|_| LeyCoreError::BootstrapWorkspaceGenerationUnavailable)?;
    Ok(WorkspaceGeneration::Unix {
        device: metadata.dev(),
        inode: metadata.ino(),
        created_seconds: created.as_secs(),
        created_nanos: created.subsec_nanos(),
    })
}

#[cfg(not(unix))]
fn workspace_generation(_metadata: &fs::Metadata) -> Result<WorkspaceGeneration, LeyCoreError> {
    Err(LeyCoreError::BootstrapWorkspaceGenerationUnavailable)
}

fn ensure_target_uninitialized(root: &Path) -> Result<(), LeyCoreError> {
    match diagnose_project(root) {
        Err(LeyCoreError::ProjectNotFound(_)) => Ok(()),
        Ok(_) => Err(LeyCoreError::InvalidBootstrapSpecificationRequest(
            "bootstrap Specifications apply only to an uninitialized workspace; use normal Ley project context here"
                .to_owned(),
        )),
        Err(error) => Err(error),
    }
}

fn grant_id(workspace_id: &str, source_project_id: &str, specification_id: &str) -> String {
    let mut digest = Sha256::new();
    for value in [workspace_id, source_project_id, specification_id] {
        digest.update(value.as_bytes());
        digest.update([0]);
    }
    format!("bsg_{:x}", digest.finalize())
}

fn exclusion(
    grant_id: &str,
    grant: &BootstrapGrantEntry,
    reason: BootstrapCompileExclusionReason,
    egress: Option<(AgentEgressPolicy, AgentEgressBlockReason)>,
) -> BootstrapCompileExclusion {
    BootstrapCompileExclusion {
        grant_id: grant_id.to_owned(),
        source_project_id: grant.source_project_id.clone(),
        specification_id: grant.specification_id.clone(),
        reason,
        policy: egress.map(|value| value.0),
        block_reason: egress.map(|value| value.1),
    }
}

fn candidate_exclusion(
    candidate: &BootstrapCandidate,
    reason: BootstrapCompileExclusionReason,
) -> BootstrapCompileExclusion {
    BootstrapCompileExclusion {
        grant_id: candidate.grant_id.clone(),
        source_project_id: candidate.source_project_id.clone(),
        specification_id: candidate.specification_id.clone(),
        reason,
        policy: None,
        block_reason: None,
    }
}

fn estimate_specification_tokens(relative_path: &str, source: &str) -> usize {
    BOOTSTRAP_SPECIFICATION_ITEM_OVERHEAD_TOKENS.saturating_add(
        relative_path
            .chars()
            .count()
            .saturating_add(source.chars().count())
            .div_ceil(4),
    )
}

fn validate_compile_limits(task: &str, limits: ContextCompileLimits) -> Result<(), LeyCoreError> {
    if task.trim().is_empty() {
        return Err(LeyCoreError::InvalidBootstrapSpecificationRequest(
            "bootstrap compiler task must contain visible characters".to_owned(),
        ));
    }
    if task.chars().count() > MAX_PROJECT_MEMORY_SEARCH_QUERY_CHARACTERS {
        return Err(LeyCoreError::InvalidBootstrapSpecificationRequest(format!(
            "bootstrap compiler task cannot exceed {MAX_PROJECT_MEMORY_SEARCH_QUERY_CHARACTERS} characters"
        )));
    }
    if !(1..=MAX_CONTEXT_COMPILE_RESULTS).contains(&limits.max_results) {
        return Err(LeyCoreError::InvalidBootstrapSpecificationRequest(format!(
            "maxResults must be between 1 and {MAX_CONTEXT_COMPILE_RESULTS}"
        )));
    }
    if !(MIN_CONTEXT_COMPILE_TOKENS..=MAX_CONTEXT_COMPILE_TOKENS).contains(&limits.max_tokens) {
        return Err(LeyCoreError::InvalidBootstrapSpecificationRequest(format!(
            "maxTokens must be between {MIN_CONTEXT_COMPILE_TOKENS} and {MAX_CONTEXT_COMPILE_TOKENS}"
        )));
    }
    Ok(())
}

fn path_string(path: &Path) -> Result<String, LeyCoreError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| LeyCoreError::NonUtf8Path(path.to_path_buf()))
}

fn validate_workspace_id(value: &str) -> Result<(), String> {
    validate_sha256_id(value, "bsw_", "workspaceId")
}

fn validate_grant_id(value: &str) -> Result<(), String> {
    validate_sha256_id(value, "bsg_", "grantId")
}

fn validate_sha256_id(value: &str, prefix: &str, label: &str) -> Result<(), String> {
    let Some(hash) = value.strip_prefix(prefix) else {
        return Err(format!("{label} must start with {prefix}"));
    };
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!("{label} must be a lowercase SHA-256 identifier"));
    }
    Ok(())
}

fn validate_content_hash(value: &str) -> Result<(), String> {
    validate_sha256_id(value, "sha256:", "contentHash")
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
                return Err(LeyCoreError::InvalidBootstrapSpecificationRegistry(
                    format!(
                        "private file permissions required for {}; use mode 600",
                        path.display()
                    ),
                ));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        Ok(_) => Ok(()),
    }
}

#[cfg(unix)]
fn with_target_directory_lock<T>(
    path: &Path,
    operation: impl FnOnce(&Path) -> Result<T, LeyCoreError>,
) -> Result<T, LeyCoreError> {
    let root = canonical_directory(path)?;
    let directory = File::open(&root).map_err(|source| LeyCoreError::Io {
        path: root.clone(),
        source,
    })?;
    directory.lock().map_err(|source| LeyCoreError::Io {
        path: root.clone(),
        source,
    })?;
    let result = operation(&root);
    let unlock_result = File::unlock(&directory);
    match (result, unlock_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(source)) => Err(LeyCoreError::Io { path: root, source }),
    }
}

#[cfg(not(unix))]
fn with_target_directory_lock<T>(
    path: &Path,
    operation: impl FnOnce(&Path) -> Result<T, LeyCoreError>,
) -> Result<T, LeyCoreError> {
    let root = canonical_directory(path)?;
    operation(&root)
}

fn unlock_bootstrap_result<T>(
    lock: File,
    lock_path: PathBuf,
    result: Result<T, LeyCoreError>,
) -> Result<T, LeyCoreError> {
    let unlock_result = File::unlock(&lock);
    match (result, unlock_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(source)) => Err(LeyCoreError::Io {
            path: lock_path,
            source,
        }),
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::{generate_specification_id, initialize_project, AgentEgressPolicy};
    use std::sync::mpsc;
    use std::time::Duration;
    use tempfile::tempdir;

    struct Fixture {
        _temporary: tempfile::TempDir,
        target: PathBuf,
        source: PathBuf,
        vault: PathBuf,
        bootstrap: BootstrapSpecificationRegistry,
        specifications: SpecificationRegistry,
        egress: EgressPolicyRegistry,
        specification_id: String,
    }

    fn fixture(source_body: &str) -> Fixture {
        let temporary = tempdir().unwrap();
        let target = temporary.path().join("target");
        let source = temporary.path().join("source");
        let vault = temporary.path().join("vault");
        let config = temporary.path().join("config");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(vault.join("Specs")).unwrap();
        fs::create_dir_all(&config).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config, fs::Permissions::from_mode(0o700)).unwrap();
        }
        initialize_project(&source, Some("Bootstrap source"), CaptureMode::Structured).unwrap();

        let bootstrap =
            BootstrapSpecificationRegistry::at(config.join(BOOTSTRAP_SPECIFICATION_REGISTRY_FILE));
        let bindings = BindingRegistry::at(config.join(BINDING_REGISTRY_FILE));
        bindings.bind(&source, &vault).unwrap();
        let specifications = SpecificationRegistry::at(config.join(SPECIFICATION_REGISTRY_FILE));
        let specification_id = generate_specification_id();
        fs::write(vault.join("Specs/Product.md"), source_body).unwrap();
        specifications
            .approve(&source, &vault, &specification_id, "Specs/Product.md")
            .unwrap();
        let egress = EgressPolicyRegistry::at(config.join(crate::EGRESS_POLICY_REGISTRY_FILE));

        Fixture {
            _temporary: temporary,
            target,
            source,
            vault,
            bootstrap,
            specifications,
            egress,
            specification_id,
        }
    }

    fn compile(fixture: &Fixture, task: &str, max_tokens: usize) -> BootstrapSpecificationContext {
        compile_bootstrap_specifications_with_registries(
            &fixture.target,
            task,
            ContextCompileLimits {
                max_results: 8,
                max_tokens,
            },
            AgentEgressTarget::Cloud,
            &fixture.bootstrap,
            &fixture.egress,
        )
        .unwrap()
    }

    #[test]
    fn explicit_bootstrap_grant_is_non_mutating_and_compiles_exact_approved_specification() {
        let fixture = fixture(
            "# Offline product\n\nThe bootstrap_offline_marker app must work fully offline.\n",
        );
        let before = fs::read_dir(&fixture.target).unwrap().count();

        let attached = fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();
        assert!(attached.created);
        assert!(!fixture.target.join(".ley").exists());
        assert_eq!(fs::read_dir(&fixture.target).unwrap().count(), before);

        let context = compile(&fixture, "implement bootstrap_offline_marker", 1_500);
        assert_eq!(context.specifications.len(), 1);
        assert_eq!(
            context.specifications[0].source,
            "# Offline product\n\nThe bootstrap_offline_marker app must work fully offline.\n"
        );
        assert_eq!(context.specifications[0].authority, "human-intent");
        assert_eq!(
            context.specifications[0].source_boundary,
            "user-approved-bootstrap-specification"
        );
        assert!(!context.project_memory_available);
        assert!(!context.live_source_checked);
        assert!(!context.persisted);
        assert!(!context.automatic_write_allowed);
        assert!(!fixture.target.join(".ley").exists());
    }

    #[test]
    fn recreated_directory_at_same_path_does_not_inherit_bootstrap_authority() {
        let fixture = fixture("# Product\n\ngeneration_marker requirement\n");
        let attached = fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();
        assert_eq!(
            fixture
                .bootstrap
                .list(&fixture.target)
                .unwrap()
                .total_grants,
            1
        );

        fs::remove_dir_all(&fixture.target).unwrap();
        std::thread::sleep(Duration::from_millis(2));
        fs::create_dir(&fixture.target).unwrap();
        let listed = fixture.bootstrap.list(&fixture.target).unwrap();
        assert_eq!(listed.total_grants, 0);
        assert_ne!(listed.workspace_id, attached.grant.workspace_id);
        assert!(matches!(
            compile_bootstrap_specifications_with_registries(
                &fixture.target,
                "generation_marker",
                ContextCompileLimits::default(),
                AgentEgressTarget::Cloud,
                &fixture.bootstrap,
                &fixture.egress,
            ),
            Err(LeyCoreError::InvalidBootstrapSpecificationRequest(_))
        ));
    }

    #[test]
    fn attach_and_initialization_share_the_target_transition_lock() {
        let fixture = fixture("# Product\n\nattach_race_marker requirement\n");
        let lock = fixture.bootstrap.acquire_lock().unwrap();
        let bootstrap = fixture.bootstrap.clone();
        let target = fixture.target.clone();
        let source = fixture.source.clone();
        let specification_id = fixture.specification_id.clone();
        let worker =
            std::thread::spawn(move || bootstrap.attach(&target, &source, &specification_id));

        let probe = File::open(&fixture.target).unwrap();
        for _ in 0..100 {
            match probe.try_lock() {
                Ok(()) => {
                    File::unlock(&probe).unwrap();
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(std::fs::TryLockError::WouldBlock) => break,
                Err(std::fs::TryLockError::Error(error)) => {
                    panic!("could not probe target transition lock: {error}")
                }
            }
        }
        assert!(matches!(
            probe.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));

        let init_registry = fixture.bootstrap.clone();
        let init_target = fixture.target.clone();
        let (init_tx, init_rx) = mpsc::channel();
        let initializer = std::thread::spawn(move || {
            let result = init_registry.initialize_retiring(
                &init_target,
                Some("Race target"),
                CaptureMode::Structured,
            );
            init_tx.send(()).unwrap();
            result
        });
        assert!(init_rx.recv_timeout(Duration::from_millis(50)).is_err());
        File::unlock(&lock).unwrap();
        assert!(worker.join().unwrap().unwrap().created);
        init_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(initializer.join().unwrap().unwrap().created);
        let listed = fixture.bootstrap.list(&fixture.target).unwrap();
        assert!(listed.target_initialized);
        assert_eq!(listed.total_grants, 0);
        fs::remove_dir_all(fixture.target.join(".ley")).unwrap();
        assert_eq!(
            fixture
                .bootstrap
                .list(&fixture.target)
                .unwrap()
                .total_grants,
            0
        );
    }

    #[test]
    fn compile_and_initialization_share_the_target_transition_lock() {
        let fixture = fixture("# Product\n\ncompile_race_marker requirement\n");
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();
        let lock = fixture.bootstrap.acquire_lock().unwrap();
        let bootstrap = fixture.bootstrap.clone();
        let egress = fixture.egress.clone();
        let target = fixture.target.clone();
        let worker = std::thread::spawn(move || {
            compile_bootstrap_specifications_with_registries(
                &target,
                "compile_race_marker",
                ContextCompileLimits::default(),
                AgentEgressTarget::Cloud,
                &bootstrap,
                &egress,
            )
        });

        let probe = File::open(&fixture.target).unwrap();
        for _ in 0..100 {
            match probe.try_lock() {
                Ok(()) => {
                    File::unlock(&probe).unwrap();
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(std::fs::TryLockError::WouldBlock) => break,
                Err(std::fs::TryLockError::Error(error)) => {
                    panic!("could not probe target transition lock: {error}")
                }
            }
        }
        assert!(matches!(
            probe.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));

        let init_registry = fixture.bootstrap.clone();
        let init_target = fixture.target.clone();
        let (init_tx, init_rx) = mpsc::channel();
        let initializer = std::thread::spawn(move || {
            let result = init_registry.initialize_retiring(
                &init_target,
                Some("Compile race target"),
                CaptureMode::Structured,
            );
            init_tx.send(()).unwrap();
            result
        });
        assert!(init_rx.recv_timeout(Duration::from_millis(50)).is_err());
        File::unlock(&lock).unwrap();
        let compiled = worker.join().unwrap().unwrap();
        assert_eq!(compiled.specifications.len(), 1);
        init_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(initializer.join().unwrap().unwrap().created);
        let listed = fixture.bootstrap.list(&fixture.target).unwrap();
        assert!(listed.target_initialized);
        assert_eq!(listed.total_grants, 0);
    }

    #[test]
    fn source_revision_and_egress_changes_fail_closed_before_returning_text() {
        let fixture = fixture("# Product\n\nsecret_bootstrap_marker original requirement\n");
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();

        fixture
            .egress
            .set_specification_policy(
                &fixture.source,
                &fixture.specification_id,
                AgentEgressPolicy::NeverSend,
            )
            .unwrap();
        let blocked = compile(&fixture, "secret_bootstrap_marker", 1_500);
        assert!(blocked.specifications.is_empty());
        assert_eq!(blocked.coverage.egress_blocked, 1);
        assert!(blocked.exclusions.iter().any(|item| {
            item.reason == BootstrapCompileExclusionReason::EgressBlocked
                && item.policy == Some(AgentEgressPolicy::NeverSend)
        }));
        assert!(!serde_json::to_string(&blocked)
            .unwrap()
            .contains("original requirement"));

        fixture
            .egress
            .set_specification_policy(
                &fixture.source,
                &fixture.specification_id,
                AgentEgressPolicy::AgentOk,
            )
            .unwrap();
        fs::write(
            fixture.vault.join("Specs/Product.md"),
            "# Product\n\nsecret_bootstrap_marker changed requirement\n",
        )
        .unwrap();
        let stale = compile(&fixture, "secret_bootstrap_marker", 1_500);
        assert!(stale.specifications.is_empty());
        assert_eq!(stale.coverage.stale_specifications, 1);
        assert!(stale.exclusions.iter().any(|item| {
            item.reason == BootstrapCompileExclusionReason::SpecificationRevisionChanged
        }));
        let serialized = serde_json::to_string(&stale).unwrap();
        assert!(!serialized.contains("changed requirement"));
        assert!(!serialized.contains(fixture.vault.to_str().unwrap()));
    }

    #[test]
    fn missing_approved_specification_is_excluded_without_aborting_other_grants() {
        let fixture = fixture(
            "# Missing requirement\n\nshared_missing_marker vanished requirement must not leak.\n",
        );
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();

        let surviving_id = generate_specification_id();
        fs::write(
            fixture.vault.join("Specs/Surviving.md"),
            "# Surviving requirement\n\nshared_missing_marker surviving requirement stays available.\n",
        )
        .unwrap();
        fixture
            .specifications
            .approve(
                &fixture.source,
                &fixture.vault,
                &surviving_id,
                "Specs/Surviving.md",
            )
            .unwrap();
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &surviving_id)
            .unwrap();

        fs::remove_file(fixture.vault.join("Specs/Product.md")).unwrap();
        let context = compile(&fixture, "shared_missing_marker", 1_500);
        assert_eq!(context.specifications.len(), 1);
        assert_eq!(context.specifications[0].specification_id, surviving_id);
        assert!(context.specifications[0]
            .source
            .contains("surviving requirement stays available"));
        assert_eq!(context.coverage.stale_specifications, 1);
        assert!(context.exclusions.iter().any(|item| {
            item.specification_id == fixture.specification_id
                && item.reason == BootstrapCompileExclusionReason::SpecificationUnavailable
        }));
        assert!(!serde_json::to_string(&context)
            .unwrap()
            .contains("vanished requirement must not leak"));
    }

    #[test]
    fn unavailable_source_identity_or_vault_is_disclosed_without_path_leakage() {
        let fixture = fixture("# Product\n\nsource_availability_marker requirement\n");
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();

        let moved_source = fixture.source.with_extension("moved");
        fs::rename(&fixture.source, &moved_source).unwrap();
        let missing_source = compile(&fixture, "source_availability_marker", 1_500);
        assert!(missing_source.specifications.is_empty());
        assert_eq!(missing_source.coverage.unavailable_sources, 1);
        assert!(missing_source.exclusions.iter().any(|item| {
            item.reason == BootstrapCompileExclusionReason::SourceProjectUnavailable
        }));
        let serialized = serde_json::to_string(&missing_source).unwrap();
        assert!(!serialized.contains(moved_source.to_str().unwrap()));
        assert!(!serialized.contains(fixture.vault.to_str().unwrap()));

        fs::rename(&moved_source, &fixture.source).unwrap();
        let moved_vault = fixture.vault.with_extension("moved");
        fs::rename(&fixture.vault, &moved_vault).unwrap();
        let missing_vault = compile(&fixture, "source_availability_marker", 1_500);
        assert!(missing_vault.specifications.is_empty());
        assert_eq!(missing_vault.coverage.unavailable_sources, 1);
        assert!(missing_vault.exclusions.iter().any(|item| {
            item.reason == BootstrapCompileExclusionReason::SourceVaultUnavailable
        }));
        let serialized = serde_json::to_string(&missing_vault).unwrap();
        assert!(!serialized.contains(moved_vault.to_str().unwrap()));
    }

    #[test]
    fn compilation_uses_the_current_locked_source_binding_not_a_cached_vault() {
        let fixture = fixture("# Product\n\ncurrent_binding_marker original requirement\n");
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();

        let replacement_vault = fixture._temporary.path().join("replacement-vault");
        fs::create_dir_all(replacement_vault.join("Specs")).unwrap();
        fs::write(
            replacement_vault.join("Specs/Product.md"),
            "# Product\n\ncurrent_binding_marker replacement vault requirement\n",
        )
        .unwrap();
        fixture
            .bootstrap
            .binding_registry
            .bind(&fixture.source, &replacement_vault)
            .unwrap();

        let rebound = compile(&fixture, "current_binding_marker", 1_500);
        assert!(rebound.specifications.is_empty());
        assert_eq!(rebound.coverage.stale_specifications, 1);
        assert!(!serde_json::to_string(&rebound)
            .unwrap()
            .contains("replacement vault requirement"));

        fixture
            .bootstrap
            .binding_registry
            .bind(&fixture.source, &fixture.vault)
            .unwrap();
        let restored = compile(&fixture, "current_binding_marker", 1_500);
        assert_eq!(restored.specifications.len(), 1);
        assert!(restored.specifications[0]
            .source
            .contains("original requirement"));
    }

    #[test]
    fn source_identity_replacement_after_egress_subject_resolution_fails_closed() {
        use std::os::unix::fs::OpenOptionsExt;

        let fixture =
            fixture("# Product\n\nsource_identity_race_marker original approved requirement.\n");
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();

        let egress_lock_path = fixture.egress.path().with_file_name("agent-egress-v1.lock");
        let egress_lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(&egress_lock_path)
            .unwrap();
        egress_lock.lock().unwrap();

        let bootstrap = fixture.bootstrap.clone();
        let egress = fixture.egress.clone();
        let target = fixture.target.clone();
        let worker = std::thread::spawn(move || {
            compile_bootstrap_specifications_with_registries(
                &target,
                "source_identity_race_marker",
                ContextCompileLimits::default(),
                AgentEgressTarget::Cloud,
                &bootstrap,
                &egress,
            )
        });

        let binding_lock_path = fixture
            .bootstrap
            .binding_registry
            .path()
            .with_file_name("bindings-v1.lock");
        let mut observed_binding_lock = false;
        for _ in 0..200 {
            if let Ok(probe) = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&binding_lock_path)
            {
                match probe.try_lock() {
                    Ok(()) => {
                        File::unlock(&probe).unwrap();
                    }
                    Err(std::fs::TryLockError::WouldBlock) => {
                        observed_binding_lock = true;
                        break;
                    }
                    Err(std::fs::TryLockError::Error(error)) => {
                        panic!("could not probe source binding lock: {error}")
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(
            observed_binding_lock,
            "compile never acquired source binding lock"
        );

        let old_source = fixture.source.with_extension("old-generation");
        fs::rename(&fixture.source, &old_source).unwrap();
        fs::create_dir(&fixture.source).unwrap();
        initialize_project(
            &fixture.source,
            Some("Replacement source"),
            CaptureMode::Structured,
        )
        .unwrap();
        fs::write(
            fixture.source.join("replacement.txt"),
            "source_identity_race_marker replacement project must never authorize old context\n",
        )
        .unwrap();

        File::unlock(&egress_lock).unwrap();
        let context = worker.join().unwrap().unwrap();
        assert!(context.specifications.is_empty());
        assert_eq!(context.coverage.unavailable_sources, 1);
        assert!(context
            .exclusions
            .iter()
            .any(|item| { item.reason == BootstrapCompileExclusionReason::SourceIdentityChanged }));
        let serialized = serde_json::to_string(&context).unwrap();
        assert!(!serialized.contains("original approved requirement"));
        assert!(!serialized.contains("replacement project must never authorize old context"));
    }

    #[test]
    fn reapproval_of_changed_source_does_not_update_an_existing_bootstrap_pin() {
        let fixture = fixture("# Product\n\npinned_revision_marker original requirement\n");
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();
        fs::write(
            fixture.vault.join("Specs/Product.md"),
            "# Product\n\npinned_revision_marker replacement requirement\n",
        )
        .unwrap();
        fixture
            .specifications
            .approve(
                &fixture.source,
                &fixture.vault,
                &fixture.specification_id,
                "Specs/Product.md",
            )
            .unwrap();

        let context = compile(&fixture, "pinned_revision_marker", 1_500);
        assert!(context.specifications.is_empty());
        assert_eq!(context.coverage.stale_specifications, 1);

        let refreshed = fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();
        assert!(refreshed.created);
        let context = compile(&fixture, "pinned_revision_marker", 1_500);
        assert_eq!(context.specifications.len(), 1);
        assert!(context.specifications[0]
            .source
            .contains("replacement requirement"));
    }

    #[test]
    fn specification_is_omitted_whole_when_it_cannot_fit_the_context_budget() {
        let fixture = fixture(&format!(
            "# Large product\n\nwhole_document_marker {}\n",
            "bounded requirement ".repeat(180)
        ));
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();
        let context = compile(&fixture, "whole_document_marker", 500);
        assert!(context.specifications.is_empty());
        assert_eq!(context.coverage.omitted_by_token_budget, 1);
        assert!(context
            .exclusions
            .iter()
            .any(|item| { item.reason == BootstrapCompileExclusionReason::TokenBudget }));
        assert!(!serde_json::to_string(&context)
            .unwrap()
            .contains("bounded requirement"));
    }

    #[test]
    fn initialization_retires_bootstrap_authority_and_failed_initialization_restores_it() {
        let fixture = fixture("# Product\n\nretire_bootstrap_marker requirement\n");
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();

        assert!(matches!(
            fixture.bootstrap.initialize_retiring(
                &fixture.target,
                Some(""),
                CaptureMode::Structured,
            ),
            Err(LeyCoreError::InvalidProjectName)
        ));
        assert_eq!(
            fixture
                .bootstrap
                .list(&fixture.target)
                .unwrap()
                .total_grants,
            1
        );

        let initialized = fixture
            .bootstrap
            .initialize_retiring(
                &fixture.target,
                Some("Bootstrapped target"),
                CaptureMode::Structured,
            )
            .unwrap();
        assert!(initialized.created);
        let listed = fixture.bootstrap.list(&fixture.target).unwrap();
        assert!(listed.target_initialized);
        assert_eq!(listed.total_grants, 0);
        assert!(matches!(
            compile_bootstrap_specifications_with_registries(
                &fixture.target,
                "retire_bootstrap_marker",
                ContextCompileLimits::default(),
                AgentEgressTarget::Cloud,
                &fixture.bootstrap,
                &fixture.egress,
            ),
            Err(LeyCoreError::InvalidBootstrapSpecificationRequest(_))
        ));

        fs::remove_dir_all(fixture.target.join(".ley")).unwrap();
        let listed = fixture.bootstrap.list(&fixture.target).unwrap();
        assert!(!listed.target_initialized);
        assert_eq!(listed.total_grants, 0);
    }

    #[test]
    fn restoration_failure_is_explicit_instead_of_silently_losing_authority() {
        let fixture = fixture("# Product\n\nrollback_failure_marker requirement\n");
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();
        let registry_path = fixture.bootstrap.path().to_path_buf();
        let result = fixture
            .bootstrap
            .initialize_retiring_with(&fixture.target, |_root| {
                fs::remove_file(&registry_path).unwrap();
                fs::create_dir(&registry_path).unwrap();
                Err(LeyCoreError::InvalidProjectName)
            });
        assert!(matches!(
            result,
            Err(LeyCoreError::BootstrapSpecificationRestorationFailed)
        ));
    }

    #[test]
    fn absent_bootstrap_registry_read_does_not_create_private_config_state() {
        let temporary = tempdir().unwrap();
        let target = temporary.path().join("target");
        let config = temporary.path().join("missing-config");
        fs::create_dir(&target).unwrap();
        let registry =
            BootstrapSpecificationRegistry::at(config.join(BOOTSTRAP_SPECIFICATION_REGISTRY_FILE));
        assert!(!config.exists());
        assert!(!registry.authority_file_present().unwrap());
        let listed = registry.list(&target).unwrap();
        assert_eq!(listed.total_grants, 0);
        assert!(!config.exists());
    }

    #[cfg(unix)]
    #[test]
    fn bootstrap_registry_rejects_group_or_world_readable_authority_files() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = fixture("# Product\n\nprivate_registry_marker requirement\n");
        fixture
            .bootstrap
            .attach(&fixture.target, &fixture.source, &fixture.specification_id)
            .unwrap();
        fs::set_permissions(fixture.bootstrap.path(), fs::Permissions::from_mode(0o644)).unwrap();

        assert!(matches!(
            fixture.bootstrap.list(&fixture.target),
            Err(LeyCoreError::InvalidBootstrapSpecificationRegistry(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn bootstrap_registry_rejects_group_or_world_accessible_parent_directory() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = fixture("# Product\n\nprivate_parent_marker requirement\n");
        let parent = fixture.bootstrap.path().parent().unwrap();
        fs::set_permissions(parent, fs::Permissions::from_mode(0o775)).unwrap();
        assert!(matches!(
            fixture
                .bootstrap
                .attach(&fixture.target, &fixture.source, &fixture.specification_id,),
            Err(LeyCoreError::InvalidBootstrapSpecificationRegistry(_))
        ));
    }
}

#[cfg(all(test, not(unix)))]
mod non_unix_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn bootstrap_workspace_identity_fails_closed_without_supported_unix_generation() {
        let temporary = tempdir().unwrap();
        assert!(matches!(
            bootstrap_workspace_identity(temporary.path()),
            Err(LeyCoreError::BootstrapWorkspaceGenerationUnavailable)
        ));
    }

    #[test]
    fn unsupported_bootstrap_generation_does_not_block_normal_initialization() {
        let temporary = tempdir().unwrap();
        let target = temporary.path().join("target");
        let config = temporary.path().join("config");
        fs::create_dir(&target).unwrap();
        fs::create_dir(&config).unwrap();
        let registry =
            BootstrapSpecificationRegistry::at(config.join(BOOTSTRAP_SPECIFICATION_REGISTRY_FILE));
        registry
            .write_document(&BootstrapSpecificationRegistryDocument::empty())
            .unwrap();

        let initialized = registry
            .initialize_retiring(&target, Some("Normal project"), CaptureMode::Structured)
            .unwrap();
        assert!(initialized.created);
        assert!(target.join(".ley").is_dir());
    }
}
