use crate::knowledge_scope::validate_knowledge_scope_id;
use crate::specification::validate_specification_id;
use crate::{
    default_binding_registry_path, diagnose_project, validate_project_id, ApprovedSourceRegistry,
    BindingRegistry, ContinuityStore, KnowledgeScopeKind, KnowledgeScopeRegistry, LeyCoreError,
    ProjectCatalog, SpecificationRegistry, BINDING_REGISTRY_FILE, CONTINUITY_DATABASE_FILE,
    METADATA_FILE_LIMIT_BYTES, PROJECT_CATALOG_FILE, SPECIFICATION_REGISTRY_FILE,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const POLICY_BUNDLE_REGISTRY_FILE: &str = "policy-bundles-v1.json";
const POLICY_BUNDLE_REGISTRY_LOCK_FILE: &str = "policy-bundles-v1.lock";
pub const POLICY_BUNDLE_REGISTRY_SCHEMA_VERSION: u32 = 1;
pub const MAX_POLICY_BUNDLES: usize = 32;
pub const MAX_POLICY_BUNDLE_SOURCES: usize = 16;
pub const MAX_ATTACHED_POLICY_BUNDLES_PER_PROJECT: usize = 8;
pub const MAX_POLICY_BUNDLE_HISTORY_PER_PROJECT: usize = 256;

const PRIVACY_NOTICE: &str = "Policy Bundle compatibility state is OS-private. It stores stable bundle/scope/project/approved-source identities, exact approved content hashes, explicit project attachments, and bounded attachment history only. Source content is resolved through native approved-source authority; the legacy vault/binding is consulted only once if that source project has not completed approved-source migration.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyBundleSourceRef {
    pub source_project_id: String,
    pub specification_id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyBundleSourceStatus {
    Ready,
    SourceProjectUnavailable,
    SourceIdentityChanged,
    SourceVaultUnavailable,
    SpecificationNotApproved,
    SpecificationRevisionChanged,
    SpecificationSourceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyBundleSource {
    pub source_project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_project_name: Option<String>,
    pub specification_id: String,
    pub content_hash: String,
    pub status: PolicyBundleSourceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyBundle {
    pub bundle_id: String,
    pub scope_id: String,
    pub scope_kind: KnowledgeScopeKind,
    pub scope_name: String,
    pub name: String,
    pub sources: Vec<PolicyBundleSource>,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyBundleList {
    pub bundles: Vec<PolicyBundle>,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyBundleAttachmentState {
    Active,
    ParentScopeDetachedOrReattached,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyBundleAttachment {
    pub active_project_id: String,
    pub bundle_id: String,
    pub scope_id: String,
    pub bundle_name: String,
    pub source_count: usize,
    pub attached_at_unix_ms: u64,
    pub state: PolicyBundleAttachmentState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyBundleAttachmentList {
    pub active_project_id: String,
    pub attachments: Vec<PolicyBundleAttachment>,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyBundleEgressSource {
    pub bundle_id: String,
    pub source_project_id: String,
    pub specification_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyBundleEgressSources {
    pub active: Vec<PolicyBundleEgressSource>,
    pub historical: Vec<PolicyBundleEgressSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyBundleEntry {
    scope_id: String,
    name: String,
    sources: Vec<PolicyBundleSourceRef>,
    created_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyBundleAttachmentEntry {
    attached_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyBundleRegistryDocument {
    schema_version: u32,
    bundles: BTreeMap<String, PolicyBundleEntry>,
    #[serde(default)]
    attachments: BTreeMap<String, BTreeMap<String, PolicyBundleAttachmentEntry>>,
    #[serde(default)]
    attachment_history: BTreeMap<String, BTreeMap<String, Vec<PolicyBundleSourceRef>>>,
}

impl PolicyBundleRegistryDocument {
    fn empty() -> Self {
        Self {
            schema_version: POLICY_BUNDLE_REGISTRY_SCHEMA_VERSION,
            bundles: BTreeMap::new(),
            attachments: BTreeMap::new(),
            attachment_history: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), LeyCoreError> {
        if self.schema_version != POLICY_BUNDLE_REGISTRY_SCHEMA_VERSION {
            return Err(LeyCoreError::InvalidPolicyBundleRegistry(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        if self.bundles.len() > MAX_POLICY_BUNDLES {
            return Err(LeyCoreError::InvalidPolicyBundleRegistry(format!(
                "registry has more than {MAX_POLICY_BUNDLES} policy bundles"
            )));
        }
        for (bundle_id, entry) in &self.bundles {
            validate_policy_bundle_id(bundle_id)
                .map_err(LeyCoreError::InvalidPolicyBundleRegistry)?;
            validate_knowledge_scope_id(&entry.scope_id)
                .map_err(LeyCoreError::InvalidPolicyBundleRegistry)?;
            validate_bundle_name(&entry.name).map_err(LeyCoreError::InvalidPolicyBundleRegistry)?;
            validate_source_refs(&entry.sources)
                .map_err(LeyCoreError::InvalidPolicyBundleRegistry)?;
            if entry.created_at_unix_ms == 0 {
                return Err(LeyCoreError::InvalidPolicyBundleRegistry(
                    "policy bundle creation time must be non-zero".to_owned(),
                ));
            }
        }
        for (active_project_id, attachments) in &self.attachments {
            validate_project_id(active_project_id).map_err(|error| {
                LeyCoreError::InvalidPolicyBundleRegistry(format!(
                    "invalid active project ID '{active_project_id}': {error}"
                ))
            })?;
            if attachments.len() > MAX_ATTACHED_POLICY_BUNDLES_PER_PROJECT {
                return Err(LeyCoreError::InvalidPolicyBundleRegistry(format!(
                    "project {active_project_id} has more than {MAX_ATTACHED_POLICY_BUNDLES_PER_PROJECT} attached policy bundles"
                )));
            }
            for (bundle_id, attachment) in attachments {
                let bundle = self.bundles.get(bundle_id).ok_or_else(|| {
                    LeyCoreError::InvalidPolicyBundleRegistry(format!(
                        "attachment references missing policy bundle {bundle_id}"
                    ))
                })?;
                if attachment.attached_at_unix_ms == 0 {
                    return Err(LeyCoreError::InvalidPolicyBundleRegistry(format!(
                        "policy bundle {bundle_id} attachment time must be non-zero"
                    )));
                }
                if bundle
                    .sources
                    .iter()
                    .any(|source| source.source_project_id == *active_project_id)
                {
                    return Err(LeyCoreError::InvalidPolicyBundleRegistry(format!(
                        "policy bundle {bundle_id} cannot source its active project"
                    )));
                }
            }
        }
        for (active_project_id, history) in &self.attachment_history {
            validate_project_id(active_project_id).map_err(|error| {
                LeyCoreError::InvalidPolicyBundleRegistry(format!(
                    "invalid historical active project ID '{active_project_id}': {error}"
                ))
            })?;
            if history.len() > MAX_POLICY_BUNDLE_HISTORY_PER_PROJECT {
                return Err(LeyCoreError::InvalidPolicyBundleRegistry(format!(
                    "project {active_project_id} has more than {MAX_POLICY_BUNDLE_HISTORY_PER_PROJECT} historical policy bundle attachments"
                )));
            }
            for (bundle_id, sources) in history {
                validate_policy_bundle_id(bundle_id)
                    .map_err(LeyCoreError::InvalidPolicyBundleRegistry)?;
                validate_source_refs(sources).map_err(LeyCoreError::InvalidPolicyBundleRegistry)?;
                if sources
                    .iter()
                    .any(|source| source.source_project_id == *active_project_id)
                {
                    return Err(LeyCoreError::InvalidPolicyBundleRegistry(format!(
                        "historical policy bundle {bundle_id} has invalid source ancestry"
                    )));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PolicyBundleRegistry {
    path: PathBuf,
    project_catalog: ProjectCatalog,
    binding_registry: BindingRegistry,
    specification_registry: SpecificationRegistry,
    approved_source_registry: ApprovedSourceRegistry,
}

impl PolicyBundleRegistry {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        let path = default_binding_registry_path()?.with_file_name(POLICY_BUNDLE_REGISTRY_FILE);
        Ok(Self {
            project_catalog: ProjectCatalog::system_default()?,
            binding_registry: BindingRegistry::system_default()?,
            specification_registry: SpecificationRegistry::system_default()?,
            approved_source_registry: ApprovedSourceRegistry::system_default()?,
            path,
        })
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            project_catalog: ProjectCatalog::at(path.with_file_name(PROJECT_CATALOG_FILE)),
            binding_registry: BindingRegistry::at(path.with_file_name(BINDING_REGISTRY_FILE)),
            specification_registry: SpecificationRegistry::at(
                path.with_file_name(SPECIFICATION_REGISTRY_FILE),
            ),
            approved_source_registry: ApprovedSourceRegistry::at(ContinuityStore::at(
                path.with_file_name(CONTINUITY_DATABASE_FILE),
            )),
            path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn list(
        &self,
        scope_registry: &KnowledgeScopeRegistry,
    ) -> Result<PolicyBundleList, LeyCoreError> {
        let document = self.read_locked()?;
        let scopes = scope_registry.list()?;
        let scope_map = scopes
            .scopes
            .into_iter()
            .map(|scope| (scope.scope_id.clone(), scope))
            .collect::<BTreeMap<_, _>>();
        let mut bundles = document
            .bundles
            .into_iter()
            .map(|(bundle_id, entry)| {
                let scope = scope_map.get(&entry.scope_id).ok_or_else(|| {
                    LeyCoreError::InvalidPolicyBundleRegistry(format!(
                        "policy bundle {bundle_id} references missing knowledge scope {}",
                        entry.scope_id
                    ))
                })?;
                self.resolve_bundle(bundle_id, entry, scope.kind, &scope.name)
            })
            .collect::<Result<Vec<_>, _>>()?;
        bundles.sort_by(|left, right| {
            right
                .created_at_unix_ms
                .cmp(&left.created_at_unix_ms)
                .then_with(|| left.bundle_id.cmp(&right.bundle_id))
        });
        Ok(PolicyBundleList {
            bundles,
            privacy_notice: PRIVACY_NOTICE,
        })
    }

    pub fn attached(
        &self,
        active_project: impl AsRef<Path>,
        scope_registry: &KnowledgeScopeRegistry,
    ) -> Result<PolicyBundleAttachmentList, LeyCoreError> {
        let active = diagnose_project(active_project)?;
        let active_project_id = active.identity.project_id;
        let active_scope_ids = scope_registry
            .attached(&active.root)?
            .attachments
            .into_iter()
            .map(|attachment| attachment.scope_id)
            .collect::<BTreeSet<_>>();
        let document = self.read_locked()?;
        let mut attachments = document
            .attachments
            .get(&active_project_id)
            .into_iter()
            .flat_map(|items| items.iter())
            .map(|(bundle_id, entry)| {
                let bundle = document.bundles.get(bundle_id).ok_or_else(|| {
                    LeyCoreError::InvalidPolicyBundleRegistry(format!(
                        "attachment references missing policy bundle {bundle_id}"
                    ))
                })?;
                let state = if active_scope_ids.contains(&bundle.scope_id) {
                    PolicyBundleAttachmentState::Active
                } else {
                    PolicyBundleAttachmentState::ParentScopeDetachedOrReattached
                };
                Ok(attachment(
                    &active_project_id,
                    bundle_id,
                    bundle,
                    entry,
                    state,
                ))
            })
            .collect::<Result<Vec<_>, LeyCoreError>>()?;
        attachments.sort_by(|left, right| {
            right
                .attached_at_unix_ms
                .cmp(&left.attached_at_unix_ms)
                .then_with(|| left.bundle_id.cmp(&right.bundle_id))
        });
        Ok(PolicyBundleAttachmentList {
            active_project_id,
            attachments,
            privacy_notice: PRIVACY_NOTICE,
        })
    }

    pub fn detach(
        &self,
        active_project: impl AsRef<Path>,
        bundle_id: &str,
    ) -> Result<Option<PolicyBundleAttachment>, LeyCoreError> {
        validate_policy_bundle_id(bundle_id).map_err(LeyCoreError::InvalidPolicyBundleRequest)?;
        let active_project_id = diagnose_project(active_project)?.identity.project_id;
        self.mutate(|document| {
            let Some(attachments) = document.attachments.get_mut(&active_project_id) else {
                return Ok(None);
            };
            let Some(entry) = attachments.remove(bundle_id) else {
                return Ok(None);
            };
            if attachments.is_empty() {
                document.attachments.remove(&active_project_id);
            }
            let bundle = document.bundles.get(bundle_id).ok_or_else(|| {
                LeyCoreError::InvalidPolicyBundleRegistry(format!(
                    "attachment references missing policy bundle {bundle_id}"
                ))
            })?;
            Ok(Some(attachment(
                &active_project_id,
                bundle_id,
                bundle,
                &entry,
                PolicyBundleAttachmentState::ParentScopeDetachedOrReattached,
            )))
        })
    }

    pub fn with_agent_context_sources_locked<T>(
        &self,
        active_project: impl AsRef<Path>,
        active_scope_ids: &BTreeSet<String>,
        operation: impl FnOnce(PolicyBundleEgressSources) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let active_project_id = diagnose_project(active_project)?.identity.project_id;
        self.with_agent_context_sources_for_project_id_locked(
            &active_project_id,
            active_scope_ids,
            operation,
        )
    }

    pub fn with_agent_context_sources_for_project_id_locked<T>(
        &self,
        active_project_id: &str,
        active_scope_ids: &BTreeSet<String>,
        operation: impl FnOnce(PolicyBundleEgressSources) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        validate_project_id(active_project_id)?;
        self.with_locked_document(|document| {
            let mut active = Vec::new();
            for (bundle_id, _attachment) in document
                .attachments
                .get(active_project_id)
                .into_iter()
                .flat_map(|items| items.iter())
            {
                let bundle = document.bundles.get(bundle_id).ok_or_else(|| {
                    LeyCoreError::InvalidPolicyBundleRegistry(format!(
                        "attachment references missing policy bundle {bundle_id}"
                    ))
                })?;
                if !active_scope_ids.contains(&bundle.scope_id) {
                    continue;
                }
                active.extend(
                    bundle
                        .sources
                        .iter()
                        .map(|source| PolicyBundleEgressSource {
                            bundle_id: bundle_id.clone(),
                            source_project_id: source.source_project_id.clone(),
                            specification_id: source.specification_id.clone(),
                        }),
                );
            }
            let mut historical = document
                .attachment_history
                .get(active_project_id)
                .into_iter()
                .flat_map(|items| items.iter())
                .flat_map(|(bundle_id, sources)| {
                    sources.iter().map(|source| PolicyBundleEgressSource {
                        bundle_id: bundle_id.clone(),
                        source_project_id: source.source_project_id.clone(),
                        specification_id: source.specification_id.clone(),
                    })
                })
                .collect::<Vec<_>>();
            sort_egress_sources(&mut active);
            sort_egress_sources(&mut historical);
            historical.dedup();
            operation(PolicyBundleEgressSources { active, historical })
        })
    }

    fn resolve_bundle(
        &self,
        bundle_id: String,
        entry: PolicyBundleEntry,
        scope_kind: KnowledgeScopeKind,
        scope_name: &str,
    ) -> Result<PolicyBundle, LeyCoreError> {
        let mut sources = entry
            .sources
            .iter()
            .map(|source| self.resolve_source(source))
            .collect::<Result<Vec<_>, _>>()?;
        sources.sort_by(|left, right| {
            left.source_project_id
                .cmp(&right.source_project_id)
                .then_with(|| left.specification_id.cmp(&right.specification_id))
        });
        Ok(PolicyBundle {
            bundle_id,
            scope_id: entry.scope_id,
            scope_kind,
            scope_name: scope_name.to_owned(),
            name: entry.name,
            sources,
            created_at_unix_ms: entry.created_at_unix_ms,
        })
    }

    fn resolve_source(
        &self,
        source: &PolicyBundleSourceRef,
    ) -> Result<PolicyBundleSource, LeyCoreError> {
        let Some(observed) = self.project_catalog.get(&source.source_project_id)? else {
            return Ok(unavailable_source(
                source,
                None,
                PolicyBundleSourceStatus::SourceProjectUnavailable,
            ));
        };
        let diagnostic = match diagnose_project(&observed.root_path) {
            Ok(diagnostic) if diagnostic.identity.project_id == source.source_project_id => {
                diagnostic
            }
            Ok(diagnostic) => {
                return Ok(unavailable_source(
                    source,
                    Some(diagnostic.identity.name),
                    PolicyBundleSourceStatus::SourceIdentityChanged,
                ))
            }
            Err(_) => {
                return Ok(unavailable_source(
                    source,
                    None,
                    PolicyBundleSourceStatus::SourceProjectUnavailable,
                ))
            }
        };

        if !self
            .approved_source_registry
            .authority_ready_for_expected_project(&diagnostic.root, &source.source_project_id)?
        {
            let binding = match self.binding_registry.resolve_observed(&diagnostic) {
                Ok(binding) => binding,
                Err(_) => {
                    return Ok(unavailable_source(
                        source,
                        Some(diagnostic.identity.name),
                        PolicyBundleSourceStatus::SourceVaultUnavailable,
                    ))
                }
            };
            self.approved_source_registry
                .migrate_legacy_specifications_for_expected_project(
                    &diagnostic.root,
                    &binding.vault_path,
                    &self.specification_registry,
                    &source.source_project_id,
                )?;
        }

        let approved = match self
            .approved_source_registry
            .read_specification_compat_for_expected_project(
                &diagnostic.root,
                &source.source_project_id,
                &source.specification_id,
            ) {
            Ok(approved) => approved,
            Err(LeyCoreError::ApprovedSourceNotFound(_)) => {
                return Ok(unavailable_source(
                    source,
                    Some(diagnostic.identity.name),
                    PolicyBundleSourceStatus::SpecificationNotApproved,
                ))
            }
            Err(LeyCoreError::ApprovedSourceStale { .. }) => {
                return Ok(unavailable_source(
                    source,
                    Some(diagnostic.identity.name),
                    PolicyBundleSourceStatus::SpecificationRevisionChanged,
                ))
            }
            Err(LeyCoreError::Io { source: error, .. })
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                return Ok(unavailable_source(
                    source,
                    Some(diagnostic.identity.name),
                    PolicyBundleSourceStatus::SpecificationSourceUnavailable,
                ))
            }
            Err(error) => return Err(error),
        };
        let status = if approved.content_hash == source.content_hash {
            PolicyBundleSourceStatus::Ready
        } else {
            PolicyBundleSourceStatus::SpecificationRevisionChanged
        };
        Ok(PolicyBundleSource {
            source_project_id: source.source_project_id.clone(),
            source_project_name: Some(diagnostic.identity.name),
            specification_id: source.specification_id.clone(),
            content_hash: source.content_hash.clone(),
            status,
        })
    }

    fn with_locked_document<T>(
        &self,
        operation: impl FnOnce(&PolicyBundleRegistryDocument) -> Result<T, LeyCoreError>,
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

    fn read_locked(&self) -> Result<PolicyBundleRegistryDocument, LeyCoreError> {
        self.with_locked_document(|document| Ok(document.clone()))
    }

    fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut PolicyBundleRegistryDocument) -> Result<T, LeyCoreError>,
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
            LeyCoreError::InvalidPolicyBundleRegistry(
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

    fn read_document(&self) -> Result<PolicyBundleRegistryDocument, LeyCoreError> {
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(PolicyBundleRegistryDocument::empty())
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
        let document: PolicyBundleRegistryDocument =
            serde_json::from_slice(&bytes).map_err(|source| LeyCoreError::Json {
                path: self.path.clone(),
                source,
            })?;
        document.validate()?;
        Ok(document)
    }

    fn write_document(&self, document: &PolicyBundleRegistryDocument) -> Result<(), LeyCoreError> {
        reject_non_regular_if_present(&self.path)?;
        let mut body = serde_json::to_vec_pretty(document)
            .expect("validated policy bundle registry is serializable");
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
        self.path.with_file_name(POLICY_BUNDLE_REGISTRY_LOCK_FILE)
    }
}

fn unavailable_source(
    source: &PolicyBundleSourceRef,
    source_project_name: Option<String>,
    status: PolicyBundleSourceStatus,
) -> PolicyBundleSource {
    PolicyBundleSource {
        source_project_id: source.source_project_id.clone(),
        source_project_name,
        specification_id: source.specification_id.clone(),
        content_hash: source.content_hash.clone(),
        status,
    }
}

fn attachment(
    active_project_id: &str,
    bundle_id: &str,
    bundle: &PolicyBundleEntry,
    entry: &PolicyBundleAttachmentEntry,
    state: PolicyBundleAttachmentState,
) -> PolicyBundleAttachment {
    PolicyBundleAttachment {
        active_project_id: active_project_id.to_owned(),
        bundle_id: bundle_id.to_owned(),
        scope_id: bundle.scope_id.clone(),
        bundle_name: bundle.name.clone(),
        source_count: bundle.sources.len(),
        attached_at_unix_ms: entry.attached_at_unix_ms,
        state,
    }
}

fn validate_source_refs(sources: &[PolicyBundleSourceRef]) -> Result<(), String> {
    if sources.is_empty() || sources.len() > MAX_POLICY_BUNDLE_SOURCES {
        return Err(format!(
            "policy bundle must contain between 1 and {MAX_POLICY_BUNDLE_SOURCES} Specification sources"
        ));
    }
    let mut unique = BTreeSet::new();
    for source in sources {
        validate_project_id(&source.source_project_id).map_err(|error| error.to_string())?;
        validate_specification_id(&source.specification_id)?;
        validate_content_hash(&source.content_hash)?;
        if !unique.insert((
            source.source_project_id.as_str(),
            source.specification_id.as_str(),
        )) {
            return Err("policy bundle Specification sources must be unique".to_owned());
        }
    }
    Ok(())
}

pub fn validate_policy_bundle_id(value: &str) -> Result<(), String> {
    let Some(uuid) = value.strip_prefix("pbd_") else {
        return Err("policyBundleId must start with pbd_".to_owned());
    };
    if uuid.len() != 32
        || !uuid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || Uuid::parse_str(uuid).is_err()
    {
        return Err(
            "policyBundleId must contain a 32-character lowercase hexadecimal UUID".to_owned(),
        );
    }
    Ok(())
}

fn validate_content_hash(value: &str) -> Result<(), String> {
    let Some(digest) = value.strip_prefix("sha256:") else {
        return Err("policy bundle content hash must use sha256:<64 lowercase hex>".to_owned());
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("policy bundle content hash must use sha256:<64 lowercase hex>".to_owned());
    }
    Ok(())
}

fn validate_bundle_name(value: &str) -> Result<String, String> {
    let value = value.trim();
    let characters = value.chars().count();
    if characters == 0 || characters > 128 || value.chars().any(char::is_control) {
        return Err("policy bundle name must contain 1 to 128 visible characters".to_owned());
    }
    Ok(value.to_owned())
}

fn sort_egress_sources(values: &mut [PolicyBundleEgressSource]) {
    values.sort_by(|left, right| {
        left.bundle_id
            .cmp(&right.bundle_id)
            .then_with(|| left.source_project_id.cmp(&right.source_project_id))
            .then_with(|| left.specification_id.cmp(&right.specification_id))
    });
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
                return Err(LeyCoreError::InvalidPolicyBundleRegistry(format!(
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
    use crate::{
        generate_specification_id, initialize_project, CaptureMode, SpecificationRegistry,
        KNOWLEDGE_SCOPE_REGISTRY_FILE, KNOWLEDGE_SCOPE_REGISTRY_SCHEMA_VERSION,
    };
    use tempfile::tempdir;

    const SCOPE_ID: &str = "ksc_11111111111111111111111111111111";
    const BUNDLE_ID: &str = "pbd_11111111111111111111111111111111";
    const NATIVE_BUNDLE_ID: &str = "pbd_22222222222222222222222222222222";

    struct Fixture {
        _base: tempfile::TempDir,
        active: PathBuf,
        source: PathBuf,
        source_two: PathBuf,
        active_vault: PathBuf,
        source_vault: PathBuf,
        source_two_vault: PathBuf,
        bundle_registry: PolicyBundleRegistry,
        scope_registry: KnowledgeScopeRegistry,
        specification_registry: SpecificationRegistry,
        source_specification_id: String,
        source_two_specification_id: String,
    }

    fn setup() -> Fixture {
        let base = tempdir().unwrap();
        let config = base.path().join("config");
        let active = base.path().join("active");
        let source = base.path().join("source");
        let source_two = base.path().join("source-two");
        let active_vault = base.path().join("active-vault");
        let source_vault = base.path().join("source-vault");
        let source_two_vault = base.path().join("source-two-vault");
        for path in [
            &active,
            &source,
            &source_two,
            &active_vault,
            &source_vault,
            &source_two_vault,
        ] {
            fs::create_dir_all(path).unwrap();
        }
        initialize_project(&active, Some("Active"), CaptureMode::Structured).unwrap();
        initialize_project(&source, Some("Policy source"), CaptureMode::Structured).unwrap();
        initialize_project(
            &source_two,
            Some("Second policy source"),
            CaptureMode::Structured,
        )
        .unwrap();

        let binding_registry = BindingRegistry::at(config.join(BINDING_REGISTRY_FILE));
        binding_registry.bind(&active, &active_vault).unwrap();
        binding_registry.bind(&source, &source_vault).unwrap();
        binding_registry
            .bind(&source_two, &source_two_vault)
            .unwrap();

        fs::write(
            source_vault.join("TeamPolicy.md"),
            "# Team policy\n\nUse signed commits for releases.\n",
        )
        .unwrap();
        fs::write(
            source_two_vault.join("SecurityPolicy.md"),
            "# Security policy\n\nNever commit credentials.\n",
        )
        .unwrap();
        let specification_registry =
            SpecificationRegistry::at(config.join(SPECIFICATION_REGISTRY_FILE));
        let source_specification_id = generate_specification_id();
        let source_two_specification_id = generate_specification_id();
        specification_registry
            .approve(
                &source,
                &source_vault,
                &source_specification_id,
                "TeamPolicy.md",
            )
            .unwrap();
        specification_registry
            .approve(
                &source_two,
                &source_two_vault,
                &source_two_specification_id,
                "SecurityPolicy.md",
            )
            .unwrap();

        Fixture {
            _base: base,
            active,
            source,
            source_two,
            active_vault,
            source_vault,
            source_two_vault,
            bundle_registry: PolicyBundleRegistry::at(config.join(POLICY_BUNDLE_REGISTRY_FILE)),
            scope_registry: KnowledgeScopeRegistry::at(config.join(KNOWLEDGE_SCOPE_REGISTRY_FILE)),
            specification_registry,
            source_specification_id,
            source_two_specification_id,
        }
    }

    fn write_private_json(path: &Path, document: &serde_json::Value) {
        fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(document).unwrap()),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    fn seed_scope(
        fixture: &Fixture,
        scope_id: &str,
        source_project_ids: &[String],
        attached: bool,
        retain_history: bool,
    ) {
        let active_id = diagnose_project(&fixture.active)
            .unwrap()
            .identity
            .project_id;
        let attachments = if attached {
            serde_json::json!({ active_id.clone(): { scope_id: 1_700_000_000_100_u64 } })
        } else {
            serde_json::json!({})
        };
        let history = if retain_history {
            serde_json::json!({ active_id: { scope_id: source_project_ids } })
        } else {
            serde_json::json!({})
        };
        write_private_json(
            fixture.scope_registry.path(),
            &serde_json::json!({
                "schemaVersion": KNOWLEDGE_SCOPE_REGISTRY_SCHEMA_VERSION,
                "scopes": {
                    scope_id: {
                        "kind": "team",
                        "name": "Platform team",
                        "sourceProjectIds": source_project_ids,
                        "createdAtUnixMs": 1_700_000_000_000_u64
                    }
                },
                "attachments": attachments,
                "attachmentHistory": history
            }),
        );
    }

    fn seed_bundle(
        fixture: &Fixture,
        bundle_id: &str,
        scope_id: &str,
        name: &str,
        sources: &[PolicyBundleSourceRef],
        attached: bool,
        retain_history: bool,
    ) {
        let active_id = diagnose_project(&fixture.active)
            .unwrap()
            .identity
            .project_id;
        let source_values = sources
            .iter()
            .map(|source| {
                serde_json::json!({
                    "sourceProjectId": source.source_project_id,
                    "specificationId": source.specification_id,
                    "contentHash": source.content_hash
                })
            })
            .collect::<Vec<_>>();
        let attachments = if attached {
            serde_json::json!({ active_id.clone(): {
                bundle_id: { "attachedAtUnixMs": 1_700_000_000_100_u64 }
            }})
        } else {
            serde_json::json!({})
        };
        let history = if retain_history {
            serde_json::json!({ active_id: { bundle_id: source_values } })
        } else {
            serde_json::json!({})
        };
        write_private_json(
            fixture.bundle_registry.path(),
            &serde_json::json!({
                "schemaVersion": POLICY_BUNDLE_REGISTRY_SCHEMA_VERSION,
                "bundles": {
                    bundle_id: {
                        "scopeId": scope_id,
                        "name": name,
                        "sources": source_values,
                        "createdAtUnixMs": 1_700_000_000_000_u64
                    }
                },
                "attachments": attachments,
                "attachmentHistory": history
            }),
        );
    }

    fn legacy_source_ref(
        fixture: &Fixture,
        project: &Path,
        vault: &Path,
        specification_id: &str,
    ) -> PolicyBundleSourceRef {
        let source = fixture
            .specification_registry
            .read_approved_source(project, vault, specification_id)
            .unwrap();
        PolicyBundleSourceRef {
            source_project_id: source.project_id,
            specification_id: source.specification_id,
            content_hash: source.content_hash,
        }
    }

    #[test]
    fn bundle_inspection_migrates_approved_sources_and_preserves_snapshot_safety() {
        let fixture = setup();
        let source_id = diagnose_project(&fixture.source)
            .unwrap()
            .identity
            .project_id;
        let source_two_id = diagnose_project(&fixture.source_two)
            .unwrap()
            .identity
            .project_id;
        let source_refs = vec![
            legacy_source_ref(
                &fixture,
                &fixture.source,
                &fixture.source_vault,
                &fixture.source_specification_id,
            ),
            legacy_source_ref(
                &fixture,
                &fixture.source_two,
                &fixture.source_two_vault,
                &fixture.source_two_specification_id,
            ),
        ];
        seed_scope(
            &fixture,
            SCOPE_ID,
            &[source_id.clone(), source_two_id],
            false,
            false,
        );
        seed_bundle(
            &fixture,
            BUNDLE_ID,
            SCOPE_ID,
            "Engineering policy",
            &source_refs,
            false,
            false,
        );

        let listed = fixture
            .bundle_registry
            .list(&fixture.scope_registry)
            .unwrap();
        assert_eq!(listed.bundles.len(), 1);
        assert_eq!(listed.bundles[0].bundle_id, BUNDLE_ID);
        assert_eq!(listed.bundles[0].sources.len(), 2);
        assert!(listed.bundles[0]
            .sources
            .iter()
            .all(|source| source.status == PolicyBundleSourceStatus::Ready));
        assert!(fixture
            .bundle_registry
            .approved_source_registry
            .authority_ready_for_expected_project(&fixture.source, &source_id)
            .unwrap());
        assert!(fixture
            .bundle_registry
            .approved_source_registry
            .authority_ready_for_expected_project(
                &fixture.source_two,
                &diagnose_project(&fixture.source_two)
                    .unwrap()
                    .identity
                    .project_id,
            )
            .unwrap());

        let stored = fs::read_to_string(fixture.bundle_registry.path()).unwrap();
        let listed_json = serde_json::to_string(&listed).unwrap();
        for private in [
            fixture.active.to_str().unwrap(),
            fixture.source.to_str().unwrap(),
            fixture.source_two.to_str().unwrap(),
            fixture.active_vault.to_str().unwrap(),
            fixture.source_vault.to_str().unwrap(),
            fixture.source_two_vault.to_str().unwrap(),
            "Use signed commits for releases.",
            "Never commit credentials.",
        ] {
            assert!(!stored.contains(private));
            assert!(!listed_json.contains(private));
        }

        fs::write(
            fixture.source_vault.join("TeamPolicy.md"),
            "# Team policy\n\nUse signed commits and two reviewers for releases.\n",
        )
        .unwrap();
        let pinned = fixture
            .bundle_registry
            .list(&fixture.scope_registry)
            .unwrap();
        assert_eq!(
            pinned.bundles[0]
                .sources
                .iter()
                .find(|source| source.specification_id == fixture.source_specification_id)
                .unwrap()
                .status,
            PolicyBundleSourceStatus::Ready
        );

        fixture
            .specification_registry
            .approve(
                &fixture.source,
                &fixture.source_vault,
                &fixture.source_specification_id,
                "TeamPolicy.md",
            )
            .unwrap();
        assert_eq!(
            fixture
                .bundle_registry
                .list(&fixture.scope_registry)
                .unwrap()
                .bundles[0]
                .sources
                .iter()
                .find(|source| source.specification_id == fixture.source_specification_id)
                .unwrap()
                .status,
            PolicyBundleSourceStatus::Ready
        );
        fs::remove_file(fixture.specification_registry.path()).unwrap();
        fs::remove_dir_all(&fixture.source_vault).unwrap();
        fs::remove_dir_all(&fixture.source_two_vault).unwrap();
        let migrated = fixture
            .bundle_registry
            .list(&fixture.scope_registry)
            .unwrap();
        assert!(migrated.bundles[0]
            .sources
            .iter()
            .all(|source| source.status == PolicyBundleSourceStatus::Ready));
        let migrated_json = serde_json::to_string(&migrated).unwrap();
        assert!(!migrated_json.contains(fixture.source_vault.to_str().unwrap()));
        assert!(!migrated_json.contains("Use signed commits"));

        fs::create_dir_all(fixture.source.join("docs")).unwrap();
        fs::write(
            fixture.source.join("docs/NativePolicy.md"),
            "# Native policy\n\nUse deterministic release manifests.\n",
        )
        .unwrap();
        let native = fixture
            .bundle_registry
            .approved_source_registry
            .approve_project_file(&fixture.source, "docs/NativePolicy.md")
            .unwrap();
        let mut document: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(fixture.bundle_registry.path()).unwrap())
                .unwrap();
        document["bundles"][NATIVE_BUNDLE_ID] = serde_json::json!({
            "scopeId": SCOPE_ID,
            "name": "Native project policy",
            "sources": [{
                "sourceProjectId": source_id,
                "specificationId": native.source_id,
                "contentHash": native.content_hash
            }],
            "createdAtUnixMs": 1_700_000_000_001_u64
        });
        write_private_json(fixture.bundle_registry.path(), &document);
        let native_ready = fixture
            .bundle_registry
            .list(&fixture.scope_registry)
            .unwrap();
        assert_eq!(
            native_ready
                .bundles
                .iter()
                .find(|bundle| bundle.bundle_id == NATIVE_BUNDLE_ID)
                .unwrap()
                .sources[0]
                .status,
            PolicyBundleSourceStatus::Ready
        );
        fs::write(
            fixture.source.join("docs/NativePolicy.md"),
            "# Native policy\n\nChanged without approval.\n",
        )
        .unwrap();
        let native_changed = fixture
            .bundle_registry
            .list(&fixture.scope_registry)
            .unwrap();
        assert_eq!(
            native_changed
                .bundles
                .iter()
                .find(|bundle| bundle.bundle_id == NATIVE_BUNDLE_ID)
                .unwrap()
                .sources[0]
                .status,
            PolicyBundleSourceStatus::SpecificationRevisionChanged
        );
    }

    #[test]
    fn attachment_inspection_and_cleanup_preserve_parent_scope_and_source_history() {
        let fixture = setup();
        let source_id = diagnose_project(&fixture.source)
            .unwrap()
            .identity
            .project_id;
        let source_ref = legacy_source_ref(
            &fixture,
            &fixture.source,
            &fixture.source_vault,
            &fixture.source_specification_id,
        );
        seed_scope(
            &fixture,
            SCOPE_ID,
            std::slice::from_ref(&source_id),
            false,
            false,
        );
        seed_bundle(
            &fixture,
            BUNDLE_ID,
            SCOPE_ID,
            "Team rules",
            std::slice::from_ref(&source_ref),
            false,
            false,
        );
        assert!(fixture
            .bundle_registry
            .attached(&fixture.active, &fixture.scope_registry)
            .unwrap()
            .attachments
            .is_empty());
        let no_scope_ids = BTreeSet::new();
        fixture
            .bundle_registry
            .with_agent_context_sources_locked(&fixture.active, &no_scope_ids, |sources| {
                assert!(sources.active.is_empty());
                assert!(sources.historical.is_empty());
                Ok(())
            })
            .unwrap();

        seed_scope(
            &fixture,
            SCOPE_ID,
            std::slice::from_ref(&source_id),
            true,
            true,
        );
        seed_bundle(
            &fixture,
            BUNDLE_ID,
            SCOPE_ID,
            "Team rules",
            std::slice::from_ref(&source_ref),
            true,
            true,
        );
        let active_scope_ids = BTreeSet::from([SCOPE_ID.to_owned()]);
        let active_attachment = fixture
            .bundle_registry
            .attached(&fixture.active, &fixture.scope_registry)
            .unwrap();
        assert_eq!(active_attachment.attachments.len(), 1);
        assert_eq!(
            active_attachment.attachments[0].state,
            PolicyBundleAttachmentState::Active
        );
        fixture
            .bundle_registry
            .with_agent_context_sources_locked(&fixture.active, &active_scope_ids, |sources| {
                assert_eq!(sources.active.len(), 1);
                assert_eq!(sources.historical.len(), 1);
                assert_eq!(sources.active[0].bundle_id, BUNDLE_ID);
                assert_eq!(
                    sources.active[0].specification_id,
                    fixture.source_specification_id
                );
                Ok(())
            })
            .unwrap();

        seed_scope(
            &fixture,
            SCOPE_ID,
            std::slice::from_ref(&source_id),
            false,
            true,
        );
        let detached_scope = fixture
            .bundle_registry
            .attached(&fixture.active, &fixture.scope_registry)
            .unwrap();
        assert_eq!(
            detached_scope.attachments[0].state,
            PolicyBundleAttachmentState::ParentScopeDetachedOrReattached
        );
        fixture
            .bundle_registry
            .with_agent_context_sources_locked(&fixture.active, &BTreeSet::new(), |sources| {
                assert!(sources.active.is_empty());
                assert_eq!(sources.historical.len(), 1);
                Ok(())
            })
            .unwrap();

        seed_scope(
            &fixture,
            SCOPE_ID,
            std::slice::from_ref(&source_id),
            true,
            true,
        );
        assert_eq!(
            fixture
                .bundle_registry
                .attached(&fixture.active, &fixture.scope_registry)
                .unwrap()
                .attachments[0]
                .state,
            PolicyBundleAttachmentState::Active
        );

        assert!(fixture
            .bundle_registry
            .detach(&fixture.active, BUNDLE_ID)
            .unwrap()
            .is_some());
        assert!(fixture
            .bundle_registry
            .attached(&fixture.active, &fixture.scope_registry)
            .unwrap()
            .attachments
            .is_empty());
        fixture
            .bundle_registry
            .with_agent_context_sources_locked(&fixture.active, &active_scope_ids, |sources| {
                assert!(sources.active.is_empty());
                assert_eq!(sources.historical.len(), 1);
                assert_eq!(sources.historical[0].bundle_id, BUNDLE_ID);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn registry_is_private_and_corruption_or_symlink_fails_closed() {
        let fixture = setup();
        let source_id = diagnose_project(&fixture.source)
            .unwrap()
            .identity
            .project_id;
        let source_ref = legacy_source_ref(
            &fixture,
            &fixture.source,
            &fixture.source_vault,
            &fixture.source_specification_id,
        );
        seed_scope(
            &fixture,
            SCOPE_ID,
            std::slice::from_ref(&source_id),
            false,
            false,
        );
        seed_bundle(
            &fixture,
            BUNDLE_ID,
            SCOPE_ID,
            "Team rules",
            std::slice::from_ref(&source_ref),
            false,
            false,
        );
        fixture
            .bundle_registry
            .list(&fixture.scope_registry)
            .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::{symlink, PermissionsExt};
            assert_eq!(
                fs::metadata(fixture.bundle_registry.path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(fixture.bundle_registry.lock_path())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            fs::set_permissions(
                fixture.bundle_registry.path(),
                fs::Permissions::from_mode(0o644),
            )
            .unwrap();
            assert!(matches!(
                fixture.bundle_registry.list(&fixture.scope_registry),
                Err(LeyCoreError::InvalidPolicyBundleRegistry(_))
            ));
            fs::set_permissions(
                fixture.bundle_registry.path(),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();

            let target = fixture._base.path().join("outside.json");
            fs::write(&target, "{}\n").unwrap();
            let unsafe_path = fixture._base.path().join("unsafe-policy-bundles.json");
            symlink(&target, &unsafe_path).unwrap();
            assert!(matches!(
                PolicyBundleRegistry::at(&unsafe_path).list(&fixture.scope_registry),
                Err(LeyCoreError::UnsafeProjectLayout(_))
            ));
        }

        write_private_json(
            fixture.bundle_registry.path(),
            &serde_json::json!({
                "schemaVersion": 99,
                "bundles": {},
                "attachments": {},
                "attachmentHistory": {}
            }),
        );
        assert!(matches!(
            fixture.bundle_registry.list(&fixture.scope_registry),
            Err(LeyCoreError::InvalidPolicyBundleRegistry(_))
        ));
    }
}
