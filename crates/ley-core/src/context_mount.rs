use crate::{
    default_binding_registry_path, diagnose_project, validate_project_id, BindingRegistry,
    LeyCoreError, ProjectCatalog, BINDING_REGISTRY_FILE, METADATA_FILE_LIMIT_BYTES,
    PROJECT_CATALOG_FILE,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const CONTEXT_MOUNT_REGISTRY_FILE: &str = "context-mounts-v1.json";
const CONTEXT_MOUNT_REGISTRY_LOCK_FILE: &str = "context-mounts-v1.lock";
pub const CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION: u32 = 3;
const LEGACY_CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION: u32 = 1;
const AGENT_ENABLED_CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION: u32 = 2;
pub const MAX_CONTEXT_MOUNTS_PER_PROJECT: usize = 16;
pub const MAX_CONTEXT_MOUNT_HISTORY_PER_PROJECT: usize = 256;

const PRIVACY_NOTICE: &str = "Context Mount authority stores only active/source project identities, a stable mount ID, creation time, whether the user explicitly enabled bounded agent context, and bounded historical mount-ID/source-project-ID pairs so later mount or source egress restrictions can still constrain unproven derivatives after unmount. Project and vault paths remain owned by Ley's private project catalog and binding registry. Legacy v1 mounts remain agent-disabled until explicitly re-added.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextMountPermission {
    ReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextMountStatus {
    Ready,
    SourceProjectUnavailable,
    SourceIdentityChanged,
    SourceVaultUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMount {
    pub mount_id: String,
    pub active_project_id: String,
    pub source_project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_project_name: Option<String>,
    pub permission: ContextMountPermission,
    pub agent_context_enabled: bool,
    pub status: ContextMountStatus,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMountMutation {
    pub mount: ContextMount,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextMountList {
    pub active_project_id: String,
    pub mounts: Vec<ContextMount>,
    pub ready: usize,
    pub unavailable: usize,
    pub agent_context_enabled: usize,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMountEgressSource {
    pub mount_id: String,
    pub source_project_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMountEgressSources {
    pub active: Vec<ContextMountEgressSource>,
    pub historical: Vec<ContextMountEgressSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextMountEntry {
    source_project_id: String,
    created_at_unix_ms: u64,
    #[serde(default)]
    agent_context_enabled: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextMountRegistryDocument {
    schema_version: u32,
    mounts: BTreeMap<String, BTreeMap<String, ContextMountEntry>>,
    #[serde(default)]
    agent_mount_history: BTreeMap<String, BTreeMap<String, String>>,
}

impl ContextMountRegistryDocument {
    fn empty() -> Self {
        Self {
            schema_version: CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION,
            mounts: BTreeMap::new(),
            agent_mount_history: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), LeyCoreError> {
        if self.schema_version != LEGACY_CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION
            && self.schema_version != AGENT_ENABLED_CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION
            && self.schema_version != CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION
        {
            return Err(LeyCoreError::InvalidContextMountRegistry(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        if self.schema_version != CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION
            && !self.agent_mount_history.is_empty()
        {
            return Err(LeyCoreError::InvalidContextMountRegistry(
                "pre-v3 Context Mount documents cannot contain agent mount history".to_owned(),
            ));
        }
        for (active_project_id, mounts) in &self.mounts {
            validate_project_id(active_project_id).map_err(|error| {
                LeyCoreError::InvalidContextMountRegistry(format!(
                    "invalid active project ID key '{active_project_id}': {error}"
                ))
            })?;
            if mounts.len() > MAX_CONTEXT_MOUNTS_PER_PROJECT {
                return Err(LeyCoreError::InvalidContextMountRegistry(format!(
                    "project {active_project_id} has more than {MAX_CONTEXT_MOUNTS_PER_PROJECT} Context Mounts"
                )));
            }
            let mut sources = HashSet::new();
            for (mount_id, entry) in mounts {
                validate_mount_id(mount_id).map_err(LeyCoreError::InvalidContextMountRegistry)?;
                validate_project_id(&entry.source_project_id).map_err(|error| {
                    LeyCoreError::InvalidContextMountRegistry(format!(
                        "invalid source project ID for {mount_id}: {error}"
                    ))
                })?;
                if entry.source_project_id == *active_project_id {
                    return Err(LeyCoreError::InvalidContextMountRegistry(format!(
                        "Context Mount {mount_id} cannot reference its active project"
                    )));
                }
                if entry.created_at_unix_ms == 0 {
                    return Err(LeyCoreError::InvalidContextMountRegistry(
                        "Context Mount creation time must be non-zero".to_owned(),
                    ));
                }
                if self.schema_version == LEGACY_CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION
                    && entry.agent_context_enabled
                {
                    return Err(LeyCoreError::InvalidContextMountRegistry(
                        "legacy Context Mount documents cannot grant agent context".to_owned(),
                    ));
                }
                if !sources.insert(entry.source_project_id.as_str()) {
                    return Err(LeyCoreError::InvalidContextMountRegistry(format!(
                        "project {active_project_id} contains duplicate source project mounts"
                    )));
                }
            }
        }
        for (active_project_id, mounts) in &self.agent_mount_history {
            validate_project_id(active_project_id).map_err(|error| {
                LeyCoreError::InvalidContextMountRegistry(format!(
                    "invalid mount-history active project ID '{active_project_id}': {error}"
                ))
            })?;
            if mounts.len() > MAX_CONTEXT_MOUNT_HISTORY_PER_PROJECT {
                return Err(LeyCoreError::InvalidContextMountRegistry(format!(
                    "project {active_project_id} has more than {MAX_CONTEXT_MOUNT_HISTORY_PER_PROJECT} historical agent-enabled Context Mounts"
                )));
            }
            for (mount_id, source_project_id) in mounts {
                validate_mount_id(mount_id).map_err(LeyCoreError::InvalidContextMountRegistry)?;
                validate_project_id(source_project_id).map_err(|error| {
                    LeyCoreError::InvalidContextMountRegistry(format!(
                        "invalid historical source project ID for {mount_id}: {error}"
                    ))
                })?;
                if source_project_id == active_project_id {
                    return Err(LeyCoreError::InvalidContextMountRegistry(
                        "Context Mount history cannot reference its active project".to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn upgrade_for_write(&mut self) -> Result<(), LeyCoreError> {
        if self.schema_version == AGENT_ENABLED_CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION {
            let enabled = self
                .mounts
                .iter()
                .flat_map(|(active_project_id, mounts)| {
                    mounts
                        .iter()
                        .filter(|(_, entry)| entry.agent_context_enabled)
                        .map(|(mount_id, entry)| {
                            (
                                active_project_id.clone(),
                                mount_id.clone(),
                                entry.source_project_id.clone(),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            for (active_project_id, mount_id, source_project_id) in enabled {
                self.remember_agent_mount(&active_project_id, &mount_id, &source_project_id)?;
            }
        }
        self.schema_version = CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION;
        Ok(())
    }

    fn remember_agent_mount(
        &mut self,
        active_project_id: &str,
        mount_id: &str,
        source_project_id: &str,
    ) -> Result<(), LeyCoreError> {
        let mounts = self
            .agent_mount_history
            .entry(active_project_id.to_owned())
            .or_default();
        if !mounts.contains_key(mount_id) && mounts.len() >= MAX_CONTEXT_MOUNT_HISTORY_PER_PROJECT {
            return Err(LeyCoreError::InvalidContextMountRequest(format!(
                "an active project may retain at most {MAX_CONTEXT_MOUNT_HISTORY_PER_PROJECT} agent-enabled Context Mount identities for egress inheritance"
            )));
        }
        if let Some(existing_source) = mounts.get(mount_id) {
            if existing_source != source_project_id {
                return Err(LeyCoreError::InvalidContextMountRegistry(format!(
                    "historical Context Mount {mount_id} changed source project identity"
                )));
            }
        } else {
            mounts.insert(mount_id.to_owned(), source_project_id.to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedProjectContextMount {
    pub mount_id: String,
    pub source_project_id: String,
    pub source_project_name: String,
    pub project_root: PathBuf,
    pub vault_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedProjectContextMounts {
    pub active_project_id: String,
    pub ready: Vec<ResolvedProjectContextMount>,
    pub unavailable: Vec<ContextMount>,
    pub historical_mounts: Vec<ContextMountEgressSource>,
}

#[derive(Debug, Clone)]
pub struct ContextMountRegistry {
    path: PathBuf,
    project_catalog: ProjectCatalog,
    binding_registry: BindingRegistry,
}

impl ContextMountRegistry {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        Ok(Self::at(
            default_binding_registry_path()?.with_file_name(CONTEXT_MOUNT_REGISTRY_FILE),
        ))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let parent_binding = path.with_file_name(BINDING_REGISTRY_FILE);
        Self {
            project_catalog: ProjectCatalog::at(path.with_file_name(PROJECT_CATALOG_FILE)),
            binding_registry: BindingRegistry::at(parent_binding),
            path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn contains_mount(
        &self,
        active_project: impl AsRef<Path>,
        mount_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_mount_id(mount_id).map_err(LeyCoreError::InvalidContextMountRequest)?;
        let active_project_id = diagnose_project(active_project)?.identity.project_id;
        let document = self.read_locked()?;
        Ok(document
            .mounts
            .get(&active_project_id)
            .is_some_and(|mounts| mounts.contains_key(mount_id)))
    }

    pub fn contains_mount_or_history(
        &self,
        active_project: impl AsRef<Path>,
        mount_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_mount_id(mount_id).map_err(LeyCoreError::InvalidContextMountRequest)?;
        let active_project_id = diagnose_project(active_project)?.identity.project_id;
        let document = self.read_locked()?;
        Ok(document
            .mounts
            .get(&active_project_id)
            .is_some_and(|mounts| mounts.contains_key(mount_id))
            || document
                .agent_mount_history
                .get(&active_project_id)
                .is_some_and(|mounts| mounts.contains_key(mount_id)))
    }

    pub fn mount_project(
        &self,
        active_project: impl AsRef<Path>,
        source_project: impl AsRef<Path>,
    ) -> Result<ContextMountMutation, LeyCoreError> {
        let active = diagnose_project(active_project)?;
        let source = diagnose_project(source_project)?;
        if active.identity.project_id == source.identity.project_id {
            return Err(LeyCoreError::InvalidContextMountRequest(
                "a project cannot mount itself as reference context".to_owned(),
            ));
        }

        // A mount is useful only when both sides have their durable local scope established.
        // resolve() also refreshes the project catalog through the canonical project identity.
        self.binding_registry.resolve(&active.root, None)?;
        self.binding_registry.resolve(&source.root, None)?;

        let active_project_id = active.identity.project_id;
        let source_project_id = source.identity.project_id;
        let source_project_name = source.identity.name;
        let created_at_unix_ms = unix_time_ms();
        let (mount_id, created, stored_at) = self.mutate(|document| {
            let existing = document
                .mounts
                .get(&active_project_id)
                .and_then(|mounts| {
                    mounts
                        .iter()
                        .find(|(_, entry)| entry.source_project_id == source_project_id)
                        .map(|(mount_id, entry)| (mount_id.clone(), entry.created_at_unix_ms))
                });
            if let Some((mount_id, stored_at)) = existing {
                document.remember_agent_mount(
                    &active_project_id,
                    &mount_id,
                    &source_project_id,
                )?;
                document
                    .mounts
                    .get_mut(&active_project_id)
                    .and_then(|mounts| mounts.get_mut(&mount_id))
                    .expect("existing mount remains present under the registry lock")
                    .agent_context_enabled = true;
                return Ok((mount_id, false, stored_at));
            }
            let mounts_len = document
                .mounts
                .get(&active_project_id)
                .map_or(0, BTreeMap::len);
            if mounts_len >= MAX_CONTEXT_MOUNTS_PER_PROJECT {
                return Err(LeyCoreError::InvalidContextMountRequest(format!(
                    "an active project may have at most {MAX_CONTEXT_MOUNTS_PER_PROJECT} Context Mounts"
                )));
            }
            let mount_id = generate_mount_id();
            document.remember_agent_mount(
                &active_project_id,
                &mount_id,
                &source_project_id,
            )?;
            document
                .mounts
                .entry(active_project_id.clone())
                .or_default()
                .insert(
                mount_id.clone(),
                ContextMountEntry {
                    source_project_id: source_project_id.clone(),
                    created_at_unix_ms,
                    agent_context_enabled: true,
                },
            );
            Ok((mount_id, true, created_at_unix_ms))
        })?;

        Ok(ContextMountMutation {
            mount: ContextMount {
                mount_id,
                active_project_id,
                source_project_id,
                source_project_name: Some(source_project_name),
                permission: ContextMountPermission::ReadOnly,
                agent_context_enabled: true,
                status: ContextMountStatus::Ready,
                created_at_unix_ms: stored_at,
            },
            created,
        })
    }

    pub fn list(&self, active_project: impl AsRef<Path>) -> Result<ContextMountList, LeyCoreError> {
        let active = diagnose_project(active_project)?;
        let active_project_id = active.identity.project_id;
        let document = self.read_locked()?;
        let entries = document
            .mounts
            .get(&active_project_id)
            .cloned()
            .unwrap_or_default();
        let mut mounts = entries
            .into_iter()
            .map(|(mount_id, entry)| self.resolve_mount(&active_project_id, mount_id, entry))
            .collect::<Result<Vec<_>, _>>()?;
        mounts.sort_by(|left, right| {
            right
                .created_at_unix_ms
                .cmp(&left.created_at_unix_ms)
                .then_with(|| left.mount_id.cmp(&right.mount_id))
        });
        let ready = mounts
            .iter()
            .filter(|mount| mount.status == ContextMountStatus::Ready)
            .count();
        let agent_context_enabled = mounts
            .iter()
            .filter(|mount| mount.agent_context_enabled)
            .count();
        Ok(ContextMountList {
            active_project_id,
            unavailable: mounts.len().saturating_sub(ready),
            mounts,
            ready,
            agent_context_enabled,
            privacy_notice: PRIVACY_NOTICE,
        })
    }

    pub fn with_agent_context_sources_locked<T>(
        &self,
        active_project: impl AsRef<Path>,
        operation: impl FnOnce(ContextMountEgressSources) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let active = diagnose_project(active_project)?;
        let active_project_id = active.identity.project_id;
        self.with_locked_document(|document| {
            let mut active_sources = document
                .mounts
                .get(&active_project_id)
                .into_iter()
                .flat_map(|mounts| mounts.iter())
                .filter(|(_, entry)| entry.agent_context_enabled)
                .map(|(mount_id, entry)| ContextMountEgressSource {
                    mount_id: mount_id.clone(),
                    source_project_id: entry.source_project_id.clone(),
                })
                .collect::<Vec<_>>();
            active_sources.sort_by(|left, right| left.mount_id.cmp(&right.mount_id));
            let mut historical = document
                .agent_mount_history
                .get(&active_project_id)
                .cloned()
                .unwrap_or_default();
            for source in &active_sources {
                historical
                    .entry(source.mount_id.clone())
                    .or_insert_with(|| source.source_project_id.clone());
            }
            operation(ContextMountEgressSources {
                active: active_sources,
                historical: historical
                    .into_iter()
                    .map(|(mount_id, source_project_id)| ContextMountEgressSource {
                        mount_id,
                        source_project_id,
                    })
                    .collect(),
            })
        })
    }

    pub(crate) fn with_resolved_project_mounts_locked<T>(
        &self,
        active_project: impl AsRef<Path>,
        operation: impl FnOnce(ResolvedProjectContextMounts) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let active = diagnose_project(active_project)?;
        let active_project_id = active.identity.project_id;
        self.with_locked_document(|document| {
            let entries = document
                .mounts
                .get(&active_project_id)
                .cloned()
                .unwrap_or_default();
            let mut historical = document
                .agent_mount_history
                .get(&active_project_id)
                .cloned()
                .unwrap_or_default();
            for (mount_id, entry) in entries
                .iter()
                .filter(|(_, entry)| entry.agent_context_enabled)
            {
                historical
                    .entry(mount_id.clone())
                    .or_insert_with(|| entry.source_project_id.clone());
            }
            let mut ready = Vec::new();
            let mut unavailable = Vec::new();
            for (mount_id, entry) in entries {
                if !entry.agent_context_enabled {
                    continue;
                }
                let mut public_mount =
                    self.resolve_mount(&active_project_id, mount_id.clone(), entry.clone())?;
                if public_mount.status != ContextMountStatus::Ready {
                    unavailable.push(public_mount);
                    continue;
                }
                let Some(observed) = self.project_catalog.get(&entry.source_project_id)? else {
                    public_mount.status = ContextMountStatus::SourceProjectUnavailable;
                    unavailable.push(public_mount);
                    continue;
                };
                let diagnostic = match diagnose_project(&observed.root_path) {
                    Ok(diagnostic) if diagnostic.identity.project_id == entry.source_project_id => {
                        diagnostic
                    }
                    Ok(_) => {
                        public_mount.status = ContextMountStatus::SourceIdentityChanged;
                        unavailable.push(public_mount);
                        continue;
                    }
                    Err(_) => {
                        public_mount.status = ContextMountStatus::SourceProjectUnavailable;
                        unavailable.push(public_mount);
                        continue;
                    }
                };
                let binding = match self.binding_registry.resolve_observed(&diagnostic) {
                    Ok(binding) => binding,
                    Err(_) => {
                        public_mount.status = ContextMountStatus::SourceVaultUnavailable;
                        unavailable.push(public_mount);
                        continue;
                    }
                };
                ready.push(ResolvedProjectContextMount {
                    mount_id,
                    source_project_id: entry.source_project_id,
                    source_project_name: diagnostic.identity.name,
                    project_root: diagnostic.root,
                    vault_path: binding.vault_path,
                });
            }
            ready.sort_by(|left, right| left.mount_id.cmp(&right.mount_id));
            unavailable.sort_by(|left, right| left.mount_id.cmp(&right.mount_id));
            operation(ResolvedProjectContextMounts {
                active_project_id: active_project_id.clone(),
                ready,
                unavailable,
                historical_mounts: historical
                    .into_iter()
                    .map(|(mount_id, source_project_id)| ContextMountEgressSource {
                        mount_id,
                        source_project_id,
                    })
                    .collect(),
            })
        })
    }

    pub fn unmount(
        &self,
        active_project: impl AsRef<Path>,
        mount_id: &str,
    ) -> Result<Option<ContextMount>, LeyCoreError> {
        validate_mount_id(mount_id).map_err(LeyCoreError::InvalidContextMountRequest)?;
        let active = diagnose_project(active_project)?;
        let active_project_id = active.identity.project_id;
        let removed = self.mutate(|document| {
            let Some(mounts) = document.mounts.get_mut(&active_project_id) else {
                return Ok(None);
            };
            let removed = mounts.remove(mount_id);
            if mounts.is_empty() {
                document.mounts.remove(&active_project_id);
            }
            Ok(removed)
        })?;
        removed
            .map(|entry| self.resolve_mount(&active_project_id, mount_id.to_owned(), entry))
            .transpose()
    }

    fn resolve_mount(
        &self,
        active_project_id: &str,
        mount_id: String,
        entry: ContextMountEntry,
    ) -> Result<ContextMount, LeyCoreError> {
        let mut status = ContextMountStatus::SourceProjectUnavailable;
        let mut source_project_name = None;
        if let Some(observed) = self.project_catalog.get(&entry.source_project_id)? {
            if let Ok(diagnostic) = diagnose_project(&observed.root_path) {
                if diagnostic.identity.project_id != entry.source_project_id {
                    status = ContextMountStatus::SourceIdentityChanged;
                } else {
                    source_project_name = Some(diagnostic.identity.name.clone());
                    status = if self.binding_registry.resolve_observed(&diagnostic).is_ok() {
                        ContextMountStatus::Ready
                    } else {
                        ContextMountStatus::SourceVaultUnavailable
                    };
                }
            }
        }
        Ok(ContextMount {
            mount_id,
            active_project_id: active_project_id.to_owned(),
            source_project_id: entry.source_project_id,
            source_project_name,
            permission: ContextMountPermission::ReadOnly,
            agent_context_enabled: entry.agent_context_enabled,
            status,
            created_at_unix_ms: entry.created_at_unix_ms,
        })
    }

    fn with_locked_document<T>(
        &self,
        operation: impl FnOnce(&ContextMountRegistryDocument) -> Result<T, LeyCoreError>,
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

    fn read_locked(&self) -> Result<ContextMountRegistryDocument, LeyCoreError> {
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

    fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut ContextMountRegistryDocument) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let lock = self.acquire_lock()?;
        let result = (|| {
            let mut document = self.read_document()?;
            document.upgrade_for_write()?;
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
            LeyCoreError::InvalidContextMountRegistry(
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

    fn read_document(&self) -> Result<ContextMountRegistryDocument, LeyCoreError> {
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ContextMountRegistryDocument::empty())
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
        let document: ContextMountRegistryDocument =
            serde_json::from_slice(&bytes).map_err(|source| LeyCoreError::Json {
                path: self.path.clone(),
                source,
            })?;
        document.validate()?;
        Ok(document)
    }

    fn write_document(&self, document: &ContextMountRegistryDocument) -> Result<(), LeyCoreError> {
        reject_non_regular_if_present(&self.path)?;
        let mut body = serde_json::to_vec_pretty(document)
            .expect("validated Context Mount registry is serializable");
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
        self.path.with_file_name(CONTEXT_MOUNT_REGISTRY_LOCK_FILE)
    }
}

fn generate_mount_id() -> String {
    format!("mnt_{}", Uuid::new_v4().simple())
}

pub(crate) fn validate_mount_id(value: &str) -> Result<(), String> {
    let Some(uuid) = value.strip_prefix("mnt_") else {
        return Err("mountId must start with mnt_".to_owned());
    };
    if uuid.len() != 32
        || !uuid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || Uuid::parse_str(uuid).is_err()
    {
        return Err("mountId must contain a 32-character lowercase hexadecimal UUID".to_owned());
    }
    Ok(())
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
                return Err(LeyCoreError::InvalidContextMountRegistry(format!(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{initialize_project, CaptureMode};
    use tempfile::tempdir;

    struct Fixture {
        _base: tempfile::TempDir,
        active: PathBuf,
        active_two: PathBuf,
        source: PathBuf,
        source_two: PathBuf,
        source_vault: PathBuf,
        registry: ContextMountRegistry,
        binding_registry: BindingRegistry,
    }

    fn setup() -> Fixture {
        let base = tempdir().unwrap();
        let config = base.path().join("config");
        let registry_path = config.join(CONTEXT_MOUNT_REGISTRY_FILE);
        let registry = ContextMountRegistry::at(&registry_path);
        let binding_registry = BindingRegistry::at(config.join(BINDING_REGISTRY_FILE));
        let active = base.path().join("active");
        let active_two = base.path().join("active-two");
        let source = base.path().join("source");
        let source_two = base.path().join("source-two");
        let active_vault = base.path().join("active-vault");
        let active_two_vault = base.path().join("active-two-vault");
        let source_vault = base.path().join("source-vault");
        let source_two_vault = base.path().join("source-two-vault");
        for path in [
            &active,
            &active_two,
            &source,
            &source_two,
            &active_vault,
            &active_two_vault,
            &source_vault,
            &source_two_vault,
        ] {
            fs::create_dir_all(path).unwrap();
        }
        initialize_project(&active, Some("Active"), CaptureMode::Structured).unwrap();
        initialize_project(&active_two, Some("Active two"), CaptureMode::Structured).unwrap();
        initialize_project(&source, Some("Reference"), CaptureMode::Structured).unwrap();
        initialize_project(&source_two, Some("Reference two"), CaptureMode::Structured).unwrap();
        binding_registry.bind(&active, &active_vault).unwrap();
        binding_registry
            .bind(&active_two, &active_two_vault)
            .unwrap();
        binding_registry.bind(&source, &source_vault).unwrap();
        binding_registry
            .bind(&source_two, &source_two_vault)
            .unwrap();
        Fixture {
            _base: base,
            active,
            active_two,
            source,
            source_two,
            source_vault,
            registry,
            binding_registry,
        }
    }

    #[test]
    fn project_mount_is_explicit_idempotent_scoped_and_unmountable() {
        let fixture = setup();
        let mounted = fixture
            .registry
            .mount_project(&fixture.active, &fixture.source)
            .unwrap();
        assert!(mounted.created);
        assert_eq!(mounted.mount.permission, ContextMountPermission::ReadOnly);
        assert_eq!(mounted.mount.status, ContextMountStatus::Ready);
        assert_eq!(
            mounted.mount.source_project_name.as_deref(),
            Some("Reference")
        );

        let retry = fixture
            .registry
            .mount_project(&fixture.active, &fixture.source)
            .unwrap();
        assert!(!retry.created);
        assert_eq!(retry.mount.mount_id, mounted.mount.mount_id);
        let list = fixture.registry.list(&fixture.active).unwrap();
        assert_eq!(list.mounts.len(), 1);
        assert_eq!(list.ready, 1);
        let serialized = serde_json::to_string(&list).unwrap();
        assert!(!serialized.contains(fixture.active.to_str().unwrap()));
        assert!(!serialized.contains(fixture.source.to_str().unwrap()));
        assert!(!serialized.contains(fixture.source_vault.to_str().unwrap()));
        assert!(fixture
            .registry
            .list(&fixture.active_two)
            .unwrap()
            .mounts
            .is_empty());

        let removed = fixture
            .registry
            .unmount(&fixture.active, &mounted.mount.mount_id)
            .unwrap()
            .unwrap();
        assert_eq!(removed.source_project_id, mounted.mount.source_project_id);
        assert!(fixture
            .registry
            .list(&fixture.active)
            .unwrap()
            .mounts
            .is_empty());
        assert!(fixture
            .registry
            .unmount(&fixture.active, &mounted.mount.mount_id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn self_mount_is_rejected_and_duplicate_source_does_not_create_authority() {
        let fixture = setup();
        assert!(matches!(
            fixture
                .registry
                .mount_project(&fixture.active, &fixture.active),
            Err(LeyCoreError::InvalidContextMountRequest(_))
        ));
        let first = fixture
            .registry
            .mount_project(&fixture.active, &fixture.source)
            .unwrap();
        let second = fixture
            .registry
            .mount_project(&fixture.active, &fixture.source)
            .unwrap();
        assert_eq!(first.mount.mount_id, second.mount.mount_id);
        assert_eq!(
            fixture.registry.list(&fixture.active).unwrap().mounts.len(),
            1
        );
    }

    #[test]
    fn source_move_survives_reobservation_but_identity_replacement_fails_closed() {
        let fixture = setup();
        fixture
            .registry
            .mount_project(&fixture.active, &fixture.source)
            .unwrap();
        let moved = fixture._base.path().join("source-moved");
        fs::rename(&fixture.source, &moved).unwrap();
        fixture.binding_registry.resolve(&moved, None).unwrap();
        let moved_list = fixture.registry.list(&fixture.active).unwrap();
        assert_eq!(moved_list.mounts[0].status, ContextMountStatus::Ready);

        let original_location = fixture._base.path().join("source-replacement");
        fs::rename(&moved, &original_location).unwrap();
        fs::create_dir_all(&moved).unwrap();
        initialize_project(&moved, Some("Other identity"), CaptureMode::Structured).unwrap();
        let list = fixture.registry.list(&fixture.active).unwrap();
        assert_eq!(
            list.mounts[0].status,
            ContextMountStatus::SourceIdentityChanged
        );
    }

    #[test]
    fn unavailable_source_and_vault_are_disclosed_without_removing_mount() {
        let fixture = setup();
        fixture
            .registry
            .mount_project(&fixture.active, &fixture.source)
            .unwrap();
        fs::remove_dir_all(&fixture.source_vault).unwrap();
        let no_vault = fixture.registry.list(&fixture.active).unwrap();
        assert_eq!(
            no_vault.mounts[0].status,
            ContextMountStatus::SourceVaultUnavailable
        );
        fs::remove_dir_all(&fixture.source).unwrap();
        let no_project = fixture.registry.list(&fixture.active).unwrap();
        assert_eq!(
            no_project.mounts[0].status,
            ContextMountStatus::SourceProjectUnavailable
        );
        assert_eq!(no_project.mounts.len(), 1);
    }

    #[test]
    fn registry_files_are_private_and_symlinks_are_rejected() {
        let fixture = setup();
        fixture
            .registry
            .mount_project(&fixture.active, &fixture.source)
            .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::{symlink, PermissionsExt};
            assert_eq!(
                fs::metadata(fixture.registry.path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o077,
                0
            );
            let target = fixture._base.path().join("outside.json");
            fs::write(&target, "{}\n").unwrap();
            let unsafe_path = fixture._base.path().join("unsafe-mounts.json");
            symlink(&target, &unsafe_path).unwrap();
            let unsafe_registry = ContextMountRegistry::at(&unsafe_path);
            assert!(matches!(
                unsafe_registry.list(&fixture.active),
                Err(LeyCoreError::UnsafeProjectLayout(_))
            ));
        }
    }

    #[test]
    fn mounted_reference_reads_serialize_with_unmount() {
        use std::sync::mpsc;
        use std::time::Duration;

        let fixture = setup();
        let mounted = fixture
            .registry
            .mount_project(&fixture.active, &fixture.source)
            .unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let reader_registry = fixture.registry.clone();
        let reader_active = fixture.active.clone();
        let reader = std::thread::spawn(move || {
            reader_registry
                .with_resolved_project_mounts_locked(reader_active, |resolved| {
                    assert_eq!(resolved.ready.len(), 1);
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap();
        });
        entered_rx.recv().unwrap();

        let (removed_tx, removed_rx) = mpsc::channel();
        let writer_registry = fixture.registry.clone();
        let writer_active = fixture.active.clone();
        let mount_id = mounted.mount.mount_id.clone();
        let writer = std::thread::spawn(move || {
            writer_registry.unmount(writer_active, &mount_id).unwrap();
            removed_tx.send(()).unwrap();
        });
        assert!(removed_rx.recv_timeout(Duration::from_millis(50)).is_err());
        release_tx.send(()).unwrap();
        reader.join().unwrap();
        removed_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        writer.join().unwrap();
        assert!(fixture
            .registry
            .list(&fixture.active)
            .unwrap()
            .mounts
            .is_empty());
    }

    #[test]
    fn mount_limit_and_corrupt_registry_fail_closed() {
        let fixture = setup();
        for index in 0..MAX_CONTEXT_MOUNTS_PER_PROJECT {
            let project = fixture._base.path().join(format!("bounded-source-{index}"));
            let vault = fixture._base.path().join(format!("bounded-vault-{index}"));
            fs::create_dir_all(&project).unwrap();
            fs::create_dir_all(&vault).unwrap();
            initialize_project(&project, Some("Bounded reference"), CaptureMode::Structured)
                .unwrap();
            fixture.binding_registry.bind(&project, &vault).unwrap();
            fixture
                .registry
                .mount_project(&fixture.active, &project)
                .unwrap();
        }
        let overflow_project = fixture._base.path().join("overflow-source");
        let overflow_vault = fixture._base.path().join("overflow-vault");
        fs::create_dir_all(&overflow_project).unwrap();
        fs::create_dir_all(&overflow_vault).unwrap();
        initialize_project(
            &overflow_project,
            Some("Overflow reference"),
            CaptureMode::Structured,
        )
        .unwrap();
        fixture
            .binding_registry
            .bind(&overflow_project, &overflow_vault)
            .unwrap();
        assert!(matches!(
            fixture
                .registry
                .mount_project(&fixture.active, &overflow_project),
            Err(LeyCoreError::InvalidContextMountRequest(_))
        ));

        fs::write(
            fixture.registry.path(),
            r#"{"schemaVersion":99,"mounts":{}}"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(fixture.registry.path(), fs::Permissions::from_mode(0o600))
                .unwrap();
        }
        assert!(matches!(
            fixture.registry.list(&fixture.active),
            Err(LeyCoreError::InvalidContextMountRegistry(_))
        ));
    }

    #[test]
    fn legacy_mounts_remain_agent_disabled_until_explicitly_readded() {
        let fixture = setup();
        let active_id = diagnose_project(&fixture.active)
            .unwrap()
            .identity
            .project_id;
        let source_id = diagnose_project(&fixture.source)
            .unwrap()
            .identity
            .project_id;
        let mount_id = generate_mount_id();
        let document = serde_json::json!({
            "schemaVersion": LEGACY_CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION,
            "mounts": {
                active_id.clone(): {
                    mount_id.clone(): {
                        "sourceProjectId": source_id,
                        "createdAtUnixMs": 1
                    }
                }
            }
        });
        fs::write(
            fixture.registry.path(),
            format!("{}\n", serde_json::to_string_pretty(&document).unwrap()),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(fixture.registry.path(), fs::Permissions::from_mode(0o600))
                .unwrap();
        }

        let before = fixture.registry.list(&fixture.active).unwrap();
        assert_eq!(before.mounts.len(), 1);
        assert!(!before.mounts[0].agent_context_enabled);
        assert_eq!(before.agent_context_enabled, 0);
        fixture
            .registry
            .with_resolved_project_mounts_locked(&fixture.active, |resolved| {
                assert!(resolved.ready.is_empty());
                assert!(resolved.unavailable.is_empty());
                Ok(())
            })
            .unwrap();

        let readded = fixture
            .registry
            .mount_project(&fixture.active, &fixture.source)
            .unwrap();
        assert!(!readded.created);
        assert_eq!(readded.mount.mount_id, mount_id);
        assert!(readded.mount.agent_context_enabled);
        let after = fixture.registry.list(&fixture.active).unwrap();
        assert_eq!(after.agent_context_enabled, 1);
        let persisted: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(fixture.registry.path()).unwrap()).unwrap();
        assert_eq!(
            persisted["schemaVersion"],
            CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION
        );
        assert_eq!(
            persisted["mounts"][&active_id][&mount_id]["agentContextEnabled"],
            true
        );
        assert_eq!(
            persisted["agentMountHistory"][&active_id][&mount_id],
            source_id
        );
    }

    #[test]
    fn v2_agent_enabled_mount_migrates_mount_history_before_unmount() {
        let fixture = setup();
        let active_id = diagnose_project(&fixture.active)
            .unwrap()
            .identity
            .project_id;
        let source_id = diagnose_project(&fixture.source)
            .unwrap()
            .identity
            .project_id;
        let mount_id = generate_mount_id();
        let document = serde_json::json!({
            "schemaVersion": AGENT_ENABLED_CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION,
            "mounts": {
                active_id.clone(): {
                    mount_id.clone(): {
                        "sourceProjectId": source_id.clone(),
                        "createdAtUnixMs": 1,
                        "agentContextEnabled": true
                    }
                }
            }
        });
        fs::write(
            fixture.registry.path(),
            format!("{}\n", serde_json::to_string_pretty(&document).unwrap()),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(fixture.registry.path(), fs::Permissions::from_mode(0o600))
                .unwrap();
        }

        fixture
            .registry
            .unmount(&fixture.active, &mount_id)
            .unwrap()
            .unwrap();
        let persisted: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(fixture.registry.path()).unwrap()).unwrap();
        assert_eq!(
            persisted["schemaVersion"],
            CONTEXT_MOUNT_REGISTRY_SCHEMA_VERSION
        );
        assert!(persisted["mounts"]
            .get(&active_id)
            .is_none_or(serde_json::Value::is_null));
        assert_eq!(
            persisted["agentMountHistory"][&active_id][&mount_id],
            source_id
        );
        fixture
            .registry
            .with_agent_context_sources_locked(&fixture.active, |sources| {
                assert!(sources.active.is_empty());
                assert_eq!(
                    sources.historical,
                    vec![ContextMountEgressSource {
                        mount_id: mount_id.clone(),
                        source_project_id: source_id.clone(),
                    }]
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn concurrent_mounts_do_not_overwrite_each_other() {
        let fixture = setup();
        let mut sources = vec![fixture.source.clone(), fixture.source_two.clone()];
        for index in 0..4 {
            let project = fixture._base.path().join(format!("source-extra-{index}"));
            let vault = fixture
                ._base
                .path()
                .join(format!("source-extra-vault-{index}"));
            fs::create_dir_all(&project).unwrap();
            fs::create_dir_all(&vault).unwrap();
            initialize_project(
                &project,
                Some(&format!("Reference extra {index}")),
                CaptureMode::Structured,
            )
            .unwrap();
            fixture.binding_registry.bind(&project, &vault).unwrap();
            sources.push(project);
        }
        let workers = sources
            .into_iter()
            .map(|source| {
                let registry = fixture.registry.clone();
                let active = fixture.active.clone();
                std::thread::spawn(move || registry.mount_project(active, source).unwrap())
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().unwrap();
        }
        let list = fixture.registry.list(&fixture.active).unwrap();
        assert_eq!(list.mounts.len(), 6);
        assert_eq!(list.ready, 6);
    }
}
