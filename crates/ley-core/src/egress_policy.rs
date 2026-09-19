use crate::context_mount::validate_mount_id;
use crate::external_connector::validate_connector_id;
use crate::specification::validate_specification_id;
use crate::{
    default_binding_registry_path, diagnose_project, validate_project_id, LeyCoreError,
    METADATA_FILE_LIMIT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const EGRESS_POLICY_REGISTRY_FILE: &str = "agent-egress-v1.json";
const EGRESS_POLICY_REGISTRY_LOCK_FILE: &str = "agent-egress-v1.lock";
pub const EGRESS_POLICY_REGISTRY_SCHEMA_VERSION: u32 = 1;
pub const MAX_EGRESS_SCOPE_OVERRIDES_PER_PROJECT: usize = 256;

const PRIVACY_NOTICE: &str = "Agent egress policy is OS-private authority. It stores stable project/specification/mount/external-connector identities and policy labels only; repository, remembered, or fetched external text cannot grant itself model-sharing permission.";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentEgressPolicy {
    #[default]
    AgentOk,
    ConfirmPerUse,
    LocalModelOnly,
    NeverSend,
}

impl AgentEgressPolicy {
    pub fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "agent-ok" => Ok(Self::AgentOk),
            "confirm-per-use" => Ok(Self::ConfirmPerUse),
            "local-model-only" => Ok(Self::LocalModelOnly),
            "never-send" => Ok(Self::NeverSend),
            _ => Err(LeyCoreError::InvalidEgressPolicyRequest(format!(
                "unsupported egress policy '{value}'; use agent-ok, confirm-per-use, local-model-only, or never-send"
            ))),
        }
    }
}

impl std::fmt::Display for AgentEgressPolicy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::AgentOk => "agent-ok",
            Self::ConfirmPerUse => "confirm-per-use",
            Self::LocalModelOnly => "local-model-only",
            Self::NeverSend => "never-send",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentEgressTarget {
    Cloud,
    Local,
}

impl AgentEgressTarget {
    pub fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "cloud" => Ok(Self::Cloud),
            "local" => Ok(Self::Local),
            _ => Err(LeyCoreError::InvalidEgressPolicyRequest(format!(
                "unsupported egress target '{value}'; use cloud or local"
            ))),
        }
    }
}

impl std::fmt::Display for AgentEgressTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentEgressBlockReason {
    ConfirmationRequired,
    LocalModelOnly,
    NeverSend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEgressDecision {
    pub policy: AgentEgressPolicy,
    pub target: AgentEgressTarget,
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<AgentEgressBlockReason>,
}

pub fn evaluate_agent_egress(
    policy: AgentEgressPolicy,
    target: AgentEgressTarget,
) -> AgentEgressDecision {
    let block_reason = match (policy, target) {
        (AgentEgressPolicy::AgentOk, _) => None,
        (AgentEgressPolicy::LocalModelOnly, AgentEgressTarget::Local) => None,
        (AgentEgressPolicy::LocalModelOnly, AgentEgressTarget::Cloud) => {
            Some(AgentEgressBlockReason::LocalModelOnly)
        }
        (AgentEgressPolicy::ConfirmPerUse, _) => Some(AgentEgressBlockReason::ConfirmationRequired),
        (AgentEgressPolicy::NeverSend, _) => Some(AgentEgressBlockReason::NeverSend),
    };
    AgentEgressDecision {
        policy,
        target,
        allowed: block_reason.is_none(),
        block_reason,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentEgressScopeKind {
    Project,
    Specification,
    ContextMount,
    ExternalConnector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEgressScopePolicy {
    pub scope_kind: AgentEgressScopeKind,
    pub scope_id: String,
    pub policy: AgentEgressPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEgressPolicyMutation {
    pub project_id: String,
    pub scope: AgentEgressScopePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgentEgressPolicy {
    pub project_id: String,
    pub project_policy: AgentEgressPolicy,
    pub specification_overrides: Vec<AgentEgressScopePolicy>,
    pub mount_overrides: Vec<AgentEgressScopePolicy>,
    pub connector_overrides: Vec<AgentEgressScopePolicy>,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectEgressEntry {
    #[serde(default = "agent_ok")]
    project_policy: AgentEgressPolicy,
    #[serde(default)]
    specifications: BTreeMap<String, AgentEgressPolicy>,
    #[serde(default)]
    mounts: BTreeMap<String, AgentEgressPolicy>,
    #[serde(default)]
    connectors: BTreeMap<String, AgentEgressPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EgressPolicyRegistryDocument {
    schema_version: u32,
    projects: BTreeMap<String, ProjectEgressEntry>,
}

impl EgressPolicyRegistryDocument {
    fn empty() -> Self {
        Self {
            schema_version: EGRESS_POLICY_REGISTRY_SCHEMA_VERSION,
            projects: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), LeyCoreError> {
        if self.schema_version != EGRESS_POLICY_REGISTRY_SCHEMA_VERSION {
            return Err(LeyCoreError::InvalidEgressPolicyRegistry(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        for (project_id, entry) in &self.projects {
            validate_project_id(project_id).map_err(|error| {
                LeyCoreError::InvalidEgressPolicyRegistry(format!(
                    "invalid project ID key '{project_id}': {error}"
                ))
            })?;
            if entry.specifications.len() > MAX_EGRESS_SCOPE_OVERRIDES_PER_PROJECT {
                return Err(LeyCoreError::InvalidEgressPolicyRegistry(format!(
                    "project {project_id} has more than {MAX_EGRESS_SCOPE_OVERRIDES_PER_PROJECT} Specification egress overrides"
                )));
            }
            if entry.mounts.len() > MAX_EGRESS_SCOPE_OVERRIDES_PER_PROJECT {
                return Err(LeyCoreError::InvalidEgressPolicyRegistry(format!(
                    "project {project_id} has more than {MAX_EGRESS_SCOPE_OVERRIDES_PER_PROJECT} Context Mount egress overrides"
                )));
            }
            if entry.connectors.len() > MAX_EGRESS_SCOPE_OVERRIDES_PER_PROJECT {
                return Err(LeyCoreError::InvalidEgressPolicyRegistry(format!(
                    "project {project_id} has more than {MAX_EGRESS_SCOPE_OVERRIDES_PER_PROJECT} external connector egress overrides"
                )));
            }
            for (specification_id, policy) in &entry.specifications {
                validate_specification_id(specification_id)
                    .map_err(LeyCoreError::InvalidEgressPolicyRegistry)?;
                if *policy == AgentEgressPolicy::AgentOk {
                    return Err(LeyCoreError::InvalidEgressPolicyRegistry(
                        "agent-ok Specification policy must be represented by absence of an override"
                            .to_owned(),
                    ));
                }
            }
            for (mount_id, policy) in &entry.mounts {
                validate_mount_id(mount_id).map_err(LeyCoreError::InvalidEgressPolicyRegistry)?;
                if *policy == AgentEgressPolicy::AgentOk {
                    return Err(LeyCoreError::InvalidEgressPolicyRegistry(
                        "agent-ok Context Mount policy must be represented by absence of an override"
                            .to_owned(),
                    ));
                }
            }
            for (connector_id, policy) in &entry.connectors {
                validate_connector_id(connector_id)
                    .map_err(LeyCoreError::InvalidEgressPolicyRegistry)?;
                if *policy == AgentEgressPolicy::AgentOk {
                    return Err(LeyCoreError::InvalidEgressPolicyRegistry(
                        "agent-ok external connector policy must be represented by absence of an override"
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn agent_ok() -> AgentEgressPolicy {
    AgentEgressPolicy::AgentOk
}

#[derive(Debug, Clone)]
pub struct EgressPolicySnapshot {
    projects: BTreeMap<String, ProjectEgressEntry>,
}

impl EgressPolicySnapshot {
    pub fn project_policy(&self, project_id: &str) -> AgentEgressPolicy {
        self.projects
            .get(project_id)
            .map_or(AgentEgressPolicy::AgentOk, |entry| entry.project_policy)
    }

    pub fn specification_policy(
        &self,
        project_id: &str,
        specification_id: &str,
    ) -> AgentEgressPolicy {
        self.projects
            .get(project_id)
            .and_then(|entry| entry.specifications.get(specification_id))
            .copied()
            .unwrap_or(AgentEgressPolicy::AgentOk)
    }

    pub fn mount_policy(&self, project_id: &str, mount_id: &str) -> AgentEgressPolicy {
        self.projects
            .get(project_id)
            .and_then(|entry| entry.mounts.get(mount_id))
            .copied()
            .unwrap_or(AgentEgressPolicy::AgentOk)
    }

    pub fn connector_policy(&self, project_id: &str, connector_id: &str) -> AgentEgressPolicy {
        self.projects
            .get(project_id)
            .and_then(|entry| entry.connectors.get(connector_id))
            .copied()
            .unwrap_or(AgentEgressPolicy::AgentOk)
    }

    pub fn has_blocked_fine_grained_source(
        &self,
        project_id: &str,
        target: AgentEgressTarget,
    ) -> bool {
        self.fine_grained_policies(project_id)
            .iter()
            .any(|scope| !evaluate_agent_egress(scope.policy, target).allowed)
    }

    pub fn fine_grained_policies(&self, project_id: &str) -> Vec<AgentEgressScopePolicy> {
        let Some(entry) = self.projects.get(project_id) else {
            return Vec::new();
        };
        entry
            .specifications
            .iter()
            .map(|(scope_id, policy)| AgentEgressScopePolicy {
                scope_kind: AgentEgressScopeKind::Specification,
                scope_id: scope_id.clone(),
                policy: *policy,
            })
            .chain(
                entry
                    .mounts
                    .iter()
                    .map(|(scope_id, policy)| AgentEgressScopePolicy {
                        scope_kind: AgentEgressScopeKind::ContextMount,
                        scope_id: scope_id.clone(),
                        policy: *policy,
                    }),
            )
            .chain(
                entry
                    .connectors
                    .iter()
                    .map(|(scope_id, policy)| AgentEgressScopePolicy {
                        scope_kind: AgentEgressScopeKind::ExternalConnector,
                        scope_id: scope_id.clone(),
                        policy: *policy,
                    }),
            )
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct EgressPolicyRegistry {
    path: PathBuf,
}

impl EgressPolicyRegistry {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        Ok(Self::at(
            default_binding_registry_path()?.with_file_name(EGRESS_POLICY_REGISTRY_FILE),
        ))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn set_project_policy(
        &self,
        project_start: impl AsRef<Path>,
        policy: AgentEgressPolicy,
    ) -> Result<AgentEgressPolicyMutation, LeyCoreError> {
        let project_id = diagnose_project(project_start)?.identity.project_id;
        self.mutate(|document| {
            let entry = document.projects.entry(project_id.clone()).or_default();
            entry.project_policy = policy;
            prune_default_entry(document, &project_id);
            Ok(AgentEgressPolicyMutation {
                project_id: project_id.clone(),
                scope: AgentEgressScopePolicy {
                    scope_kind: AgentEgressScopeKind::Project,
                    scope_id: project_id.clone(),
                    policy,
                },
            })
        })
    }

    pub fn set_specification_policy(
        &self,
        project_start: impl AsRef<Path>,
        specification_id: &str,
        policy: AgentEgressPolicy,
    ) -> Result<AgentEgressPolicyMutation, LeyCoreError> {
        validate_specification_id(specification_id)
            .map_err(LeyCoreError::InvalidEgressPolicyRequest)?;
        let project_id = diagnose_project(project_start)?.identity.project_id;
        self.mutate(|document| {
            let entry = document.projects.entry(project_id.clone()).or_default();
            if policy == AgentEgressPolicy::AgentOk {
                entry.specifications.remove(specification_id);
            } else {
                entry
                    .specifications
                    .insert(specification_id.to_owned(), policy);
            }
            prune_default_entry(document, &project_id);
            Ok(AgentEgressPolicyMutation {
                project_id: project_id.clone(),
                scope: AgentEgressScopePolicy {
                    scope_kind: AgentEgressScopeKind::Specification,
                    scope_id: specification_id.to_owned(),
                    policy,
                },
            })
        })
    }

    pub fn set_mount_policy(
        &self,
        project_start: impl AsRef<Path>,
        mount_id: &str,
        policy: AgentEgressPolicy,
    ) -> Result<AgentEgressPolicyMutation, LeyCoreError> {
        validate_mount_id(mount_id).map_err(LeyCoreError::InvalidEgressPolicyRequest)?;
        let project_id = diagnose_project(project_start)?.identity.project_id;
        self.mutate(|document| {
            let entry = document.projects.entry(project_id.clone()).or_default();
            if policy == AgentEgressPolicy::AgentOk {
                entry.mounts.remove(mount_id);
            } else {
                entry.mounts.insert(mount_id.to_owned(), policy);
            }
            prune_default_entry(document, &project_id);
            Ok(AgentEgressPolicyMutation {
                project_id: project_id.clone(),
                scope: AgentEgressScopePolicy {
                    scope_kind: AgentEgressScopeKind::ContextMount,
                    scope_id: mount_id.to_owned(),
                    policy,
                },
            })
        })
    }

    pub fn set_connector_policy(
        &self,
        project_start: impl AsRef<Path>,
        connector_id: &str,
        policy: AgentEgressPolicy,
    ) -> Result<AgentEgressPolicyMutation, LeyCoreError> {
        validate_connector_id(connector_id).map_err(LeyCoreError::InvalidEgressPolicyRequest)?;
        let project_id = diagnose_project(project_start)?.identity.project_id;
        self.mutate(|document| {
            let entry = document.projects.entry(project_id.clone()).or_default();
            if policy == AgentEgressPolicy::AgentOk {
                entry.connectors.remove(connector_id);
            } else {
                entry.connectors.insert(connector_id.to_owned(), policy);
            }
            prune_default_entry(document, &project_id);
            Ok(AgentEgressPolicyMutation {
                project_id: project_id.clone(),
                scope: AgentEgressScopePolicy {
                    scope_kind: AgentEgressScopeKind::ExternalConnector,
                    scope_id: connector_id.to_owned(),
                    policy,
                },
            })
        })
    }

    pub fn list(
        &self,
        project_start: impl AsRef<Path>,
    ) -> Result<ProjectAgentEgressPolicy, LeyCoreError> {
        let project_id = diagnose_project(project_start)?.identity.project_id;
        let document = self.read_locked()?;
        let entry = document.projects.get(&project_id);
        let project_policy = entry.map_or(AgentEgressPolicy::AgentOk, |entry| entry.project_policy);
        let specification_overrides = entry
            .into_iter()
            .flat_map(|entry| entry.specifications.iter())
            .map(|(scope_id, policy)| AgentEgressScopePolicy {
                scope_kind: AgentEgressScopeKind::Specification,
                scope_id: scope_id.clone(),
                policy: *policy,
            })
            .collect();
        let mount_overrides = entry
            .into_iter()
            .flat_map(|entry| entry.mounts.iter())
            .map(|(scope_id, policy)| AgentEgressScopePolicy {
                scope_kind: AgentEgressScopeKind::ContextMount,
                scope_id: scope_id.clone(),
                policy: *policy,
            })
            .collect();
        let connector_overrides = entry
            .into_iter()
            .flat_map(|entry| entry.connectors.iter())
            .map(|(scope_id, policy)| AgentEgressScopePolicy {
                scope_kind: AgentEgressScopeKind::ExternalConnector,
                scope_id: scope_id.clone(),
                policy: *policy,
            })
            .collect();
        Ok(ProjectAgentEgressPolicy {
            project_id,
            project_policy,
            specification_overrides,
            mount_overrides,
            connector_overrides,
            privacy_notice: PRIVACY_NOTICE,
        })
    }

    pub fn has_specification_override(
        &self,
        project_start: impl AsRef<Path>,
        specification_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_specification_id(specification_id)
            .map_err(LeyCoreError::InvalidEgressPolicyRequest)?;
        let project_id = diagnose_project(project_start)?.identity.project_id;
        let document = self.read_locked()?;
        Ok(document
            .projects
            .get(&project_id)
            .is_some_and(|entry| entry.specifications.contains_key(specification_id)))
    }

    pub fn has_mount_override(
        &self,
        project_start: impl AsRef<Path>,
        mount_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_mount_id(mount_id).map_err(LeyCoreError::InvalidEgressPolicyRequest)?;
        let project_id = diagnose_project(project_start)?.identity.project_id;
        let document = self.read_locked()?;
        Ok(document
            .projects
            .get(&project_id)
            .is_some_and(|entry| entry.mounts.contains_key(mount_id)))
    }

    pub fn has_connector_override(
        &self,
        project_start: impl AsRef<Path>,
        connector_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_connector_id(connector_id).map_err(LeyCoreError::InvalidEgressPolicyRequest)?;
        let project_id = diagnose_project(project_start)?.identity.project_id;
        let document = self.read_locked()?;
        Ok(document
            .projects
            .get(&project_id)
            .is_some_and(|entry| entry.connectors.contains_key(connector_id)))
    }

    pub fn with_snapshot_locked<T>(
        &self,
        operation: impl FnOnce(&EgressPolicySnapshot) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let lock = self.acquire_lock()?;
        let result = (|| {
            let document = self.read_document()?;
            let snapshot = EgressPolicySnapshot {
                projects: document.projects,
            };
            operation(&snapshot)
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

    pub fn with_project_egress_locked<T>(
        &self,
        project_start: impl AsRef<Path>,
        target: AgentEgressTarget,
        operation: impl FnOnce() -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let project_id = diagnose_project(project_start)?.identity.project_id;
        self.with_snapshot_locked(|snapshot| {
            let decision = evaluate_agent_egress(snapshot.project_policy(&project_id), target);
            if !decision.allowed {
                return Err(LeyCoreError::AgentEgressDenied {
                    policy: decision.policy.to_string(),
                    target: target.to_string(),
                });
            }
            operation()
        })
    }

    fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut EgressPolicyRegistryDocument) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let lock = self.acquire_lock()?;
        let result = (|| {
            let mut document = self.read_document()?;
            let value = operation(&mut document)?;
            document.schema_version = EGRESS_POLICY_REGISTRY_SCHEMA_VERSION;
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

    fn read_locked(&self) -> Result<EgressPolicyRegistryDocument, LeyCoreError> {
        let lock = self.acquire_lock()?;
        let result = self.read_document();
        let unlock_result = File::unlock(&lock);
        match (result, unlock_result) {
            (Ok(document), Ok(())) => Ok(document),
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
            LeyCoreError::InvalidEgressPolicyRegistry(
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

    fn read_document(&self) -> Result<EgressPolicyRegistryDocument, LeyCoreError> {
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(EgressPolicyRegistryDocument::empty())
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
        let document: EgressPolicyRegistryDocument =
            serde_json::from_slice(&bytes).map_err(|source| LeyCoreError::Json {
                path: self.path.clone(),
                source,
            })?;
        document.validate()?;
        Ok(document)
    }

    fn write_document(&self, document: &EgressPolicyRegistryDocument) -> Result<(), LeyCoreError> {
        reject_non_regular_if_present(&self.path)?;
        let mut body = serde_json::to_vec_pretty(document)
            .expect("validated agent egress policy registry is serializable");
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
        self.path.with_file_name(EGRESS_POLICY_REGISTRY_LOCK_FILE)
    }
}

fn prune_default_entry(document: &mut EgressPolicyRegistryDocument, project_id: &str) {
    let remove = document.projects.get(project_id).is_some_and(|entry| {
        entry.project_policy == AgentEgressPolicy::AgentOk
            && entry.specifications.is_empty()
            && entry.mounts.is_empty()
            && entry.connectors.is_empty()
    });
    if remove {
        document.projects.remove(project_id);
    }
}

fn reject_non_regular_if_present(path: &Path) -> Result<(), LeyCoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(LeyCoreError::UnsafeProjectLayout(path.to_path_buf()))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{initialize_project, CaptureMode};
    use tempfile::tempdir;

    #[test]
    fn policies_are_private_explicit_and_agent_ok_is_the_compatible_default() {
        let root = tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir(&project).unwrap();
        let initialized =
            initialize_project(&project, Some("Egress policy"), CaptureMode::Structured).unwrap();
        let registry = EgressPolicyRegistry::at(root.path().join("private/egress.json"));

        let before = registry.list(&project).unwrap();
        assert_eq!(before.project_policy, AgentEgressPolicy::AgentOk);
        assert!(before.specification_overrides.is_empty());
        assert!(before.mount_overrides.is_empty());
        assert!(before.connector_overrides.is_empty());
        assert!(!registry.path().exists());

        registry
            .set_project_policy(&project, AgentEgressPolicy::LocalModelOnly)
            .unwrap();
        registry
            .with_snapshot_locked(|snapshot| {
                assert_eq!(
                    snapshot.project_policy(&initialized.identity.project_id),
                    AgentEgressPolicy::LocalModelOnly
                );
                assert!(
                    !evaluate_agent_egress(
                        snapshot.project_policy(&initialized.identity.project_id),
                        AgentEgressTarget::Cloud
                    )
                    .allowed
                );
                assert!(
                    evaluate_agent_egress(
                        snapshot.project_policy(&initialized.identity.project_id),
                        AgentEgressTarget::Local
                    )
                    .allowed
                );
                Ok(())
            })
            .unwrap();

        registry
            .set_project_policy(&project, AgentEgressPolicy::AgentOk)
            .unwrap();
        assert_eq!(
            registry.list(&project).unwrap().project_policy,
            AgentEgressPolicy::AgentOk
        );
        let body = fs::read_to_string(registry.path()).unwrap();
        assert!(!body.contains(&initialized.identity.project_id));
    }

    #[test]
    fn confirm_and_never_send_fail_closed_for_agent_targets() {
        for target in [AgentEgressTarget::Cloud, AgentEgressTarget::Local] {
            let confirm = evaluate_agent_egress(AgentEgressPolicy::ConfirmPerUse, target);
            assert!(!confirm.allowed);
            assert_eq!(
                confirm.block_reason,
                Some(AgentEgressBlockReason::ConfirmationRequired)
            );
            let never = evaluate_agent_egress(AgentEgressPolicy::NeverSend, target);
            assert!(!never.allowed);
            assert_eq!(never.block_reason, Some(AgentEgressBlockReason::NeverSend));
        }
    }

    #[test]
    fn fine_grained_overrides_are_bounded_and_reset_with_agent_ok() {
        let root = tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir(&project).unwrap();
        initialize_project(&project, Some("Egress overrides"), CaptureMode::Structured).unwrap();
        let registry = EgressPolicyRegistry::at(root.path().join("private/egress.json"));
        let specification_id = "spec_11111111111111111111111111111111";
        let mount_id = "mnt_22222222222222222222222222222222";
        let connector_id = "ext_33333333333333333333333333333333";

        registry
            .set_specification_policy(&project, specification_id, AgentEgressPolicy::NeverSend)
            .unwrap();
        registry
            .set_mount_policy(&project, mount_id, AgentEgressPolicy::LocalModelOnly)
            .unwrap();
        registry
            .set_connector_policy(&project, connector_id, AgentEgressPolicy::NeverSend)
            .unwrap();
        let listed = registry.list(&project).unwrap();
        assert_eq!(listed.specification_overrides.len(), 1);
        assert_eq!(listed.mount_overrides.len(), 1);
        assert_eq!(listed.connector_overrides.len(), 1);
        registry
            .with_snapshot_locked(|snapshot| {
                assert_eq!(
                    snapshot.connector_policy(
                        &initialize_project(&project, None, CaptureMode::Structured)?
                            .identity
                            .project_id,
                        connector_id,
                    ),
                    AgentEgressPolicy::NeverSend
                );
                Ok(())
            })
            .unwrap();

        registry
            .set_specification_policy(&project, specification_id, AgentEgressPolicy::AgentOk)
            .unwrap();
        registry
            .set_mount_policy(&project, mount_id, AgentEgressPolicy::AgentOk)
            .unwrap();
        registry
            .set_connector_policy(&project, connector_id, AgentEgressPolicy::AgentOk)
            .unwrap();
        let listed = registry.list(&project).unwrap();
        assert!(listed.specification_overrides.is_empty());
        assert!(listed.mount_overrides.is_empty());
        assert!(listed.connector_overrides.is_empty());
    }

    #[test]
    fn policy_revocation_serializes_with_in_flight_agent_egress() {
        use std::sync::mpsc;
        use std::time::Duration;

        let root = tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir(&project).unwrap();
        initialize_project(&project, Some("Egress lock"), CaptureMode::Structured).unwrap();
        let registry = EgressPolicyRegistry::at(root.path().join("private/egress.json"));
        let reader_registry = registry.clone();
        let reader_project = project.clone();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            reader_registry
                .with_project_egress_locked(&reader_project, AgentEgressTarget::Cloud, || {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap();
        });
        entered_rx.recv().unwrap();

        let writer_registry = registry.clone();
        let writer_project = project.clone();
        let (updated_tx, updated_rx) = mpsc::channel();
        let writer = std::thread::spawn(move || {
            writer_registry
                .set_project_policy(&writer_project, AgentEgressPolicy::NeverSend)
                .unwrap();
            updated_tx.send(()).unwrap();
        });
        assert!(updated_rx.recv_timeout(Duration::from_millis(50)).is_err());

        release_tx.send(()).unwrap();
        reader.join().unwrap();
        updated_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        writer.join().unwrap();
        assert!(matches!(
            registry.with_project_egress_locked(&project, AgentEgressTarget::Cloud, || Ok(())),
            Err(LeyCoreError::AgentEgressDenied { .. })
        ));
    }
}
