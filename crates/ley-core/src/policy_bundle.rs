use crate::knowledge_scope::validate_knowledge_scope_id;
use crate::project_memory_search::lexical_score;
use crate::specification::validate_specification_id;
use crate::specification::{specification_task_terms, SpecificationAuthoritySnapshot};
use crate::{
    default_binding_registry_path, diagnose_project, evaluate_agent_egress, validate_project_id,
    AgentEgressBlockReason, AgentEgressPolicy, AgentEgressScopeKind, AgentEgressTarget,
    ApprovedSpecificationSource, BindingRegistry, EgressPolicySnapshot, KnowledgeScopeKind,
    KnowledgeScopeRegistry, LeyCoreError, ProjectCatalog, SpecificationRegistry,
    BINDING_REGISTRY_FILE, METADATA_FILE_LIMIT_BYTES, PROJECT_CATALOG_FILE,
    SPECIFICATION_REGISTRY_FILE,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const POLICY_BUNDLE_REGISTRY_FILE: &str = "policy-bundles-v1.json";
const POLICY_BUNDLE_REGISTRY_LOCK_FILE: &str = "policy-bundles-v1.lock";
pub const POLICY_BUNDLE_REGISTRY_SCHEMA_VERSION: u32 = 1;
pub const MAX_POLICY_BUNDLES: usize = 32;
pub const MAX_POLICY_BUNDLE_SOURCES: usize = 16;
pub const MAX_ATTACHED_POLICY_BUNDLES_PER_PROJECT: usize = 8;
pub const MAX_POLICY_BUNDLE_HISTORY_PER_PROJECT: usize = 256;

const PRIVACY_NOTICE: &str = "Policy Bundle authority is OS-private. It stores stable bundle/scope/project/Specification identities, exact approved content hashes, explicit project attachments, and bounded attachment history only. Policy Markdown remains in the user's ordinary vault; project and vault paths remain owned by Ley's private project catalog and binding registry.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyBundleSourceInput {
    pub source_project: PathBuf,
    pub specification_id: String,
}

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
pub struct PolicyBundleMutation {
    pub bundle: PolicyBundle,
    pub created: bool,
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
pub struct PolicyBundleAttachmentMutation {
    pub attachment: PolicyBundleAttachment,
    pub created: bool,
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

#[derive(Debug, Clone)]
pub(crate) struct PolicyBundleTaskCandidate {
    pub bundle_id: String,
    pub scope_id: String,
    pub bundle_name: String,
    pub source_project_id: String,
    pub source_project_name: String,
    pub source: ApprovedSpecificationSource,
    pub lexical_score: u32,
    pub exact_match: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PolicyBundleTaskExclusionReason {
    SourceProjectUnavailable,
    SourceIdentityChanged,
    SourceVaultUnavailable,
    SpecificationNotApproved,
    SpecificationRevisionChanged,
    SpecificationSourceUnavailable,
    LowRelevance,
    EgressBlockedSourceProject,
    EgressBlockedSpecification,
}

#[derive(Debug, Clone)]
pub(crate) struct PolicyBundleTaskExclusion {
    pub bundle_id: String,
    pub scope_id: String,
    pub source_project_id: String,
    pub source_project_name: Option<String>,
    pub specification_id: String,
    pub reason: PolicyBundleTaskExclusionReason,
}

#[derive(Debug, Clone)]
pub(crate) struct PolicyBundleTaskBundle {
    pub bundle_id: String,
    pub scope_id: String,
    pub name: String,
    pub source_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct PolicyBundleTaskScan {
    pub active_project_id: String,
    pub bundles: Vec<PolicyBundleTaskBundle>,
    pub candidates: Vec<PolicyBundleTaskCandidate>,
    pub exclusions: Vec<PolicyBundleTaskExclusion>,
    pub authorized_sources: usize,
    pub current_sources: usize,
    pub unavailable_sources: usize,
    pub low_relevance_sources: usize,
    pub egress_blocked_sources: usize,
}

pub(crate) struct PolicyBundleTaskScanRequest<'a> {
    pub active_scope_ids: &'a BTreeSet<String>,
    pub task: &'a str,
    pub specification_authority: &'a SpecificationAuthoritySnapshot<'a>,
    pub policies: &'a EgressPolicySnapshot,
    pub target: AgentEgressTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PolicyBundleEgressPolicyOrigin {
    SourceProject,
    SourceSpecification,
}

#[derive(Debug, Clone)]
pub(crate) struct PolicyBundleAgentEgressExclusion {
    pub bundle_id: String,
    pub source_project_id: String,
    pub specification_id: String,
    pub scope_kind: AgentEgressScopeKind,
    pub scope_id: String,
    pub policy_origin: PolicyBundleEgressPolicyOrigin,
    pub policy: AgentEgressPolicy,
    pub block_reason: AgentEgressBlockReason,
}

#[derive(Debug, Clone)]
struct ResolvedPolicySourceLocation {
    source_project_id: String,
    source_project_name: String,
    specification_id: String,
    expected_content_hash: String,
    project_root: PathBuf,
    vault_path: PathBuf,
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
}

impl PolicyBundleRegistry {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        Ok(Self::at(
            default_binding_registry_path()?.with_file_name(POLICY_BUNDLE_REGISTRY_FILE),
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

    pub fn create(
        &self,
        scope_id: &str,
        name: &str,
        source_specs: &[PolicyBundleSourceInput],
        scope_registry: &KnowledgeScopeRegistry,
        specification_registry: &SpecificationRegistry,
    ) -> Result<PolicyBundleMutation, LeyCoreError> {
        validate_knowledge_scope_id(scope_id).map_err(LeyCoreError::InvalidPolicyBundleRequest)?;
        let name = validate_bundle_name(name).map_err(LeyCoreError::InvalidPolicyBundleRequest)?;
        if source_specs.is_empty() || source_specs.len() > MAX_POLICY_BUNDLE_SOURCES {
            return Err(LeyCoreError::InvalidPolicyBundleRequest(format!(
                "policy bundle must contain between 1 and {MAX_POLICY_BUNDLE_SOURCES} Specifications"
            )));
        }
        let scope = scope_registry
            .list()?
            .scopes
            .into_iter()
            .find(|scope| scope.scope_id == scope_id)
            .ok_or_else(|| LeyCoreError::KnowledgeScopeNotFound(scope_id.to_owned()))?;
        let allowed_projects = scope
            .sources
            .iter()
            .map(|source| source.source_project_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut sources = Vec::with_capacity(source_specs.len());
        let mut unique = BTreeSet::new();
        for input in source_specs {
            let diagnostic = diagnose_project(&input.source_project)?;
            if !allowed_projects.contains(diagnostic.identity.project_id.as_str()) {
                return Err(LeyCoreError::InvalidPolicyBundleRequest(format!(
                    "source project {} is not a member of knowledge scope {scope_id}",
                    diagnostic.identity.project_id
                )));
            }
            let binding = self.binding_registry.resolve(&diagnostic.root, None)?;
            let source = specification_registry.read_approved_source(
                &diagnostic.root,
                &binding.vault_path,
                &input.specification_id,
            )?;
            let key = (source.project_id.clone(), source.specification_id.clone());
            if !unique.insert(key) {
                return Err(LeyCoreError::InvalidPolicyBundleRequest(
                    "policy bundle Specification sources must be unique".to_owned(),
                ));
            }
            sources.push(PolicyBundleSourceRef {
                source_project_id: source.project_id,
                specification_id: source.specification_id,
                content_hash: source.content_hash,
            });
        }
        sort_source_refs(&mut sources);
        let created_at_unix_ms = unix_time_ms();
        let (bundle_id, created) = self.mutate(|document| {
            if let Some((bundle_id, _)) = document.bundles.iter().find(|(_, entry)| {
                entry.scope_id == scope_id && entry.name == name && entry.sources == sources
            }) {
                return Ok((bundle_id.clone(), false));
            }
            if document.bundles.len() >= MAX_POLICY_BUNDLES {
                return Err(LeyCoreError::InvalidPolicyBundleRequest(format!(
                    "at most {MAX_POLICY_BUNDLES} policy bundles may be retained"
                )));
            }
            let bundle_id = generate_bundle_id();
            document.bundles.insert(
                bundle_id.clone(),
                PolicyBundleEntry {
                    scope_id: scope_id.to_owned(),
                    name: name.clone(),
                    sources: sources.clone(),
                    created_at_unix_ms,
                },
            );
            Ok((bundle_id, true))
        })?;
        let bundle = self
            .list(scope_registry)?
            .bundles
            .into_iter()
            .find(|bundle| bundle.bundle_id == bundle_id)
            .expect("created policy bundle remains in registry");
        Ok(PolicyBundleMutation { bundle, created })
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

    pub fn attach(
        &self,
        active_project: impl AsRef<Path>,
        bundle_id: &str,
        scope_registry: &KnowledgeScopeRegistry,
    ) -> Result<PolicyBundleAttachmentMutation, LeyCoreError> {
        validate_policy_bundle_id(bundle_id).map_err(LeyCoreError::InvalidPolicyBundleRequest)?;
        let active = diagnose_project(active_project)?;
        self.binding_registry.resolve(&active.root, None)?;
        let active_project_id = active.identity.project_id;
        let bundle = self
            .read_locked()?
            .bundles
            .get(bundle_id)
            .cloned()
            .ok_or_else(|| LeyCoreError::PolicyBundleNotFound(bundle_id.to_owned()))?;
        if bundle
            .sources
            .iter()
            .any(|source| source.source_project_id == active_project_id)
        {
            return Err(LeyCoreError::InvalidPolicyBundleRequest(
                "a project cannot attach a policy bundle that sources itself".to_owned(),
            ));
        }
        scope_registry
            .attached(&active.root)?
            .attachments
            .into_iter()
            .find(|attachment| attachment.scope_id == bundle.scope_id)
            .ok_or_else(|| {
                LeyCoreError::InvalidPolicyBundleRequest(format!(
                    "knowledge scope {} must be attached before policy bundle {bundle_id}",
                    bundle.scope_id
                ))
            })?;
        let now = unix_time_ms();
        let (entry, created) = self.mutate(|document| {
            let current = document
                .attachments
                .get(&active_project_id)
                .map_or(0, BTreeMap::len);
            if let Some(existing) = document
                .attachments
                .get(&active_project_id)
                .and_then(|items| items.get(bundle_id))
                .cloned()
            {
                return Ok((existing, false));
            } else if current >= MAX_ATTACHED_POLICY_BUNDLES_PER_PROJECT {
                return Err(LeyCoreError::InvalidPolicyBundleRequest(format!(
                    "an active project may attach at most {MAX_ATTACHED_POLICY_BUNDLES_PER_PROJECT} policy bundles"
                )));
            }
            let history = document
                .attachment_history
                .entry(active_project_id.clone())
                .or_default();
            if !history.contains_key(bundle_id)
                && history.len() >= MAX_POLICY_BUNDLE_HISTORY_PER_PROJECT
            {
                return Err(LeyCoreError::InvalidPolicyBundleRequest(format!(
                    "project reached the {MAX_POLICY_BUNDLE_HISTORY_PER_PROJECT} policy-bundle history limit"
                )));
            }
            history.insert(bundle_id.to_owned(), bundle.sources.clone());
            let entry = PolicyBundleAttachmentEntry {
                attached_at_unix_ms: now,
            };
            document
                .attachments
                .entry(active_project_id.clone())
                .or_default()
                .insert(bundle_id.to_owned(), entry.clone());
            Ok((entry, true))
        })?;
        Ok(PolicyBundleAttachmentMutation {
            attachment: attachment(
                &active_project_id,
                bundle_id,
                &bundle,
                &entry,
                PolicyBundleAttachmentState::Active,
            ),
            created,
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
        self.with_locked_document(|document| {
            let mut active = Vec::new();
            for (bundle_id, _attachment) in document
                .attachments
                .get(&active_project_id)
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
                .get(&active_project_id)
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

    pub(crate) fn with_task_scan_for_agent_locked<T>(
        &self,
        active_project: impl AsRef<Path>,
        request: PolicyBundleTaskScanRequest<'_>,
        operation: impl FnOnce(
            PolicyBundleTaskScan,
            Vec<PolicyBundleAgentEgressExclusion>,
        ) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let PolicyBundleTaskScanRequest {
            active_scope_ids,
            task,
            specification_authority,
            policies,
            target,
        } = request;
        let active_project_id = diagnose_project(active_project)?.identity.project_id;
        let normalized_task = task.trim().to_lowercase();
        let terms = specification_task_terms(&normalized_task);
        self.with_locked_document(|document| {
            let mut bundles = Vec::new();
            let mut candidates = Vec::new();
            let mut exclusions = Vec::new();
            let mut egress_exclusions = Vec::new();
            let mut authorized_sources = 0usize;
            let mut current_sources = 0usize;
            let mut unavailable_sources = 0usize;
            let mut low_relevance_sources = 0usize;
            let mut egress_blocked_sources = 0usize;

            for (bundle_id, _attachment) in document
                .attachments
                .get(&active_project_id)
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
                bundles.push(PolicyBundleTaskBundle {
                    bundle_id: bundle_id.clone(),
                    scope_id: bundle.scope_id.clone(),
                    name: bundle.name.clone(),
                    source_count: bundle.sources.len(),
                });
                authorized_sources = authorized_sources.saturating_add(bundle.sources.len());

                for source in &bundle.sources {
                    let project_decision = evaluate_agent_egress(
                        policies.project_policy(&source.source_project_id),
                        target,
                    );
                    if !project_decision.allowed {
                        egress_blocked_sources = egress_blocked_sources.saturating_add(1);
                        exclusions.push(PolicyBundleTaskExclusion {
                            bundle_id: bundle_id.clone(),
                            scope_id: bundle.scope_id.clone(),
                            source_project_id: source.source_project_id.clone(),
                            source_project_name: None,
                            specification_id: source.specification_id.clone(),
                            reason: PolicyBundleTaskExclusionReason::EgressBlockedSourceProject,
                        });
                        egress_exclusions.push(PolicyBundleAgentEgressExclusion {
                            bundle_id: bundle_id.clone(),
                            source_project_id: source.source_project_id.clone(),
                            specification_id: source.specification_id.clone(),
                            scope_kind: AgentEgressScopeKind::Project,
                            scope_id: source.source_project_id.clone(),
                            policy_origin: PolicyBundleEgressPolicyOrigin::SourceProject,
                            policy: project_decision.policy,
                            block_reason: project_decision
                                .block_reason
                                .expect("blocked source-project egress has a reason"),
                        });
                        continue;
                    }

                    let specification_decision = evaluate_agent_egress(
                        policies.specification_policy(
                            &source.source_project_id,
                            &source.specification_id,
                        ),
                        target,
                    );
                    if !specification_decision.allowed {
                        egress_blocked_sources = egress_blocked_sources.saturating_add(1);
                        exclusions.push(PolicyBundleTaskExclusion {
                            bundle_id: bundle_id.clone(),
                            scope_id: bundle.scope_id.clone(),
                            source_project_id: source.source_project_id.clone(),
                            source_project_name: None,
                            specification_id: source.specification_id.clone(),
                            reason: PolicyBundleTaskExclusionReason::EgressBlockedSpecification,
                        });
                        egress_exclusions.push(PolicyBundleAgentEgressExclusion {
                            bundle_id: bundle_id.clone(),
                            source_project_id: source.source_project_id.clone(),
                            specification_id: source.specification_id.clone(),
                            scope_kind: AgentEgressScopeKind::Specification,
                            scope_id: source.specification_id.clone(),
                            policy_origin: PolicyBundleEgressPolicyOrigin::SourceSpecification,
                            policy: specification_decision.policy,
                            block_reason: specification_decision
                                .block_reason
                                .expect("blocked source-Specification egress has a reason"),
                        });
                        continue;
                    }

                    let location = match self.resolve_source_for_agent(source)? {
                        ResolvedBundleSource::Ready(location) => location,
                        ResolvedBundleSource::Unavailable(unavailable) => {
                            unavailable_sources = unavailable_sources.saturating_add(1);
                            exclusions.push(PolicyBundleTaskExclusion {
                                bundle_id: bundle_id.clone(),
                                scope_id: bundle.scope_id.clone(),
                                source_project_id: unavailable.source_project_id,
                                source_project_name: unavailable.source_project_name,
                                specification_id: unavailable.specification_id,
                                reason: task_exclusion_reason_from_source_status(
                                    unavailable.status,
                                ),
                            });
                            continue;
                        }
                    };

                    let approved = match specification_authority.read_approved_source(
                        &location.source_project_id,
                        &location.vault_path,
                        &location.specification_id,
                    ) {
                        Ok(approved) => approved,
                        Err(LeyCoreError::SpecificationNotApproved(_)) => {
                            unavailable_sources = unavailable_sources.saturating_add(1);
                            exclusions.push(PolicyBundleTaskExclusion {
                                bundle_id: bundle_id.clone(),
                                scope_id: bundle.scope_id.clone(),
                                source_project_id: location.source_project_id,
                                source_project_name: Some(location.source_project_name),
                                specification_id: location.specification_id,
                                reason: PolicyBundleTaskExclusionReason::SpecificationNotApproved,
                            });
                            continue;
                        }
                        Err(LeyCoreError::SpecificationApprovalStale { .. }) => {
                            unavailable_sources = unavailable_sources.saturating_add(1);
                            exclusions.push(PolicyBundleTaskExclusion {
                                bundle_id: bundle_id.clone(),
                                scope_id: bundle.scope_id.clone(),
                                source_project_id: location.source_project_id,
                                source_project_name: Some(location.source_project_name),
                                specification_id: location.specification_id,
                                reason:
                                    PolicyBundleTaskExclusionReason::SpecificationRevisionChanged,
                            });
                            continue;
                        }
                        Err(LeyCoreError::Io { source: error, .. })
                            if error.kind() == std::io::ErrorKind::NotFound =>
                        {
                            unavailable_sources = unavailable_sources.saturating_add(1);
                            exclusions.push(PolicyBundleTaskExclusion {
                                bundle_id: bundle_id.clone(),
                                scope_id: bundle.scope_id.clone(),
                                source_project_id: location.source_project_id,
                                source_project_name: Some(location.source_project_name),
                                specification_id: location.specification_id,
                                reason:
                                    PolicyBundleTaskExclusionReason::SpecificationSourceUnavailable,
                            });
                            continue;
                        }
                        Err(error) => return Err(error),
                    };
                    if approved.content_hash != location.expected_content_hash {
                        unavailable_sources = unavailable_sources.saturating_add(1);
                        exclusions.push(PolicyBundleTaskExclusion {
                            bundle_id: bundle_id.clone(),
                            scope_id: bundle.scope_id.clone(),
                            source_project_id: location.source_project_id,
                            source_project_name: Some(location.source_project_name),
                            specification_id: location.specification_id,
                            reason: PolicyBundleTaskExclusionReason::SpecificationRevisionChanged,
                        });
                        continue;
                    }
                    current_sources = current_sources.saturating_add(1);
                    let searchable = format!("{}\n{}", approved.relative_path, approved.source);
                    let (score, exact_match) = lexical_score(&searchable, &normalized_task, &terms);
                    if score == 0 {
                        low_relevance_sources = low_relevance_sources.saturating_add(1);
                        exclusions.push(PolicyBundleTaskExclusion {
                            bundle_id: bundle_id.clone(),
                            scope_id: bundle.scope_id.clone(),
                            source_project_id: location.source_project_id,
                            source_project_name: Some(location.source_project_name),
                            specification_id: location.specification_id,
                            reason: PolicyBundleTaskExclusionReason::LowRelevance,
                        });
                        continue;
                    }
                    candidates.push(PolicyBundleTaskCandidate {
                        bundle_id: bundle_id.clone(),
                        scope_id: bundle.scope_id.clone(),
                        bundle_name: bundle.name.clone(),
                        source_project_id: location.source_project_id,
                        source_project_name: location.source_project_name,
                        source: approved,
                        lexical_score: score,
                        exact_match,
                    });
                }
            }

            let mut historical_sources = document
                .attachment_history
                .get(&active_project_id)
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
            sort_egress_sources(&mut historical_sources);
            historical_sources.dedup();
            for source in &historical_sources {
                let project_decision = evaluate_agent_egress(
                    policies.project_policy(&source.source_project_id),
                    target,
                );
                if !project_decision.allowed {
                    egress_exclusions.push(PolicyBundleAgentEgressExclusion {
                        bundle_id: source.bundle_id.clone(),
                        source_project_id: source.source_project_id.clone(),
                        specification_id: source.specification_id.clone(),
                        scope_kind: AgentEgressScopeKind::Project,
                        scope_id: source.source_project_id.clone(),
                        policy_origin: PolicyBundleEgressPolicyOrigin::SourceProject,
                        policy: project_decision.policy,
                        block_reason: project_decision
                            .block_reason
                            .expect("blocked historical source-project egress has a reason"),
                    });
                }
                let specification_decision = evaluate_agent_egress(
                    policies
                        .specification_policy(&source.source_project_id, &source.specification_id),
                    target,
                );
                if !specification_decision.allowed {
                    egress_exclusions.push(PolicyBundleAgentEgressExclusion {
                        bundle_id: source.bundle_id.clone(),
                        source_project_id: source.source_project_id.clone(),
                        specification_id: source.specification_id.clone(),
                        scope_kind: AgentEgressScopeKind::Specification,
                        scope_id: source.specification_id.clone(),
                        policy_origin: PolicyBundleEgressPolicyOrigin::SourceSpecification,
                        policy: specification_decision.policy,
                        block_reason: specification_decision
                            .block_reason
                            .expect("blocked historical source-Specification egress has a reason"),
                    });
                }
            }

            bundles.sort_by(|left, right| left.bundle_id.cmp(&right.bundle_id));
            candidates.sort_by(|left, right| {
                right
                    .exact_match
                    .cmp(&left.exact_match)
                    .then_with(|| right.lexical_score.cmp(&left.lexical_score))
                    .then_with(|| left.bundle_id.cmp(&right.bundle_id))
                    .then_with(|| left.source_project_id.cmp(&right.source_project_id))
                    .then_with(|| {
                        left.source
                            .specification_id
                            .cmp(&right.source.specification_id)
                    })
            });
            exclusions.sort_by(|left, right| {
                left.bundle_id
                    .cmp(&right.bundle_id)
                    .then_with(|| left.source_project_id.cmp(&right.source_project_id))
                    .then_with(|| left.specification_id.cmp(&right.specification_id))
            });
            egress_exclusions.sort_by(|left, right| {
                left.scope_kind
                    .cmp(&right.scope_kind)
                    .then_with(|| left.scope_id.cmp(&right.scope_id))
                    .then_with(|| left.bundle_id.cmp(&right.bundle_id))
            });
            egress_exclusions.dedup_by(|left, right| {
                left.scope_kind == right.scope_kind
                    && left.scope_id == right.scope_id
                    && left.policy_origin == right.policy_origin
                    && left.policy == right.policy
            });
            operation(
                PolicyBundleTaskScan {
                    active_project_id,
                    bundles,
                    candidates,
                    exclusions,
                    authorized_sources,
                    current_sources,
                    unavailable_sources,
                    low_relevance_sources,
                    egress_blocked_sources,
                },
                egress_exclusions,
            )
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
        match self.resolve_source_for_agent(source)? {
            ResolvedBundleSource::Ready(ready) => {
                let approved = match self.specification_registry.read_approved_source(
                    &ready.project_root,
                    &ready.vault_path,
                    &ready.specification_id,
                ) {
                    Ok(approved) => approved,
                    Err(LeyCoreError::SpecificationNotApproved(_)) => {
                        return Ok(unavailable_source(
                            source,
                            Some(ready.source_project_name),
                            PolicyBundleSourceStatus::SpecificationNotApproved,
                        ))
                    }
                    Err(LeyCoreError::SpecificationApprovalStale { .. }) => {
                        return Ok(unavailable_source(
                            source,
                            Some(ready.source_project_name),
                            PolicyBundleSourceStatus::SpecificationRevisionChanged,
                        ))
                    }
                    Err(LeyCoreError::Io { source: error, .. })
                        if error.kind() == std::io::ErrorKind::NotFound =>
                    {
                        return Ok(unavailable_source(
                            source,
                            Some(ready.source_project_name),
                            PolicyBundleSourceStatus::SpecificationSourceUnavailable,
                        ))
                    }
                    Err(error) => return Err(error),
                };
                let status = if approved.content_hash == ready.expected_content_hash {
                    PolicyBundleSourceStatus::Ready
                } else {
                    PolicyBundleSourceStatus::SpecificationRevisionChanged
                };
                Ok(PolicyBundleSource {
                    source_project_id: ready.source_project_id,
                    source_project_name: Some(ready.source_project_name),
                    specification_id: ready.specification_id,
                    content_hash: ready.expected_content_hash,
                    status,
                })
            }
            ResolvedBundleSource::Unavailable(unavailable) => Ok(unavailable),
        }
    }

    fn resolve_source_for_agent(
        &self,
        source: &PolicyBundleSourceRef,
    ) -> Result<ResolvedBundleSource, LeyCoreError> {
        let Some(observed) = self.project_catalog.get(&source.source_project_id)? else {
            return Ok(ResolvedBundleSource::Unavailable(unavailable_source(
                source,
                None,
                PolicyBundleSourceStatus::SourceProjectUnavailable,
            )));
        };
        let diagnostic = match diagnose_project(&observed.root_path) {
            Ok(diagnostic) if diagnostic.identity.project_id == source.source_project_id => {
                diagnostic
            }
            Ok(diagnostic) => {
                return Ok(ResolvedBundleSource::Unavailable(unavailable_source(
                    source,
                    Some(diagnostic.identity.name),
                    PolicyBundleSourceStatus::SourceIdentityChanged,
                )))
            }
            Err(_) => {
                return Ok(ResolvedBundleSource::Unavailable(unavailable_source(
                    source,
                    None,
                    PolicyBundleSourceStatus::SourceProjectUnavailable,
                )))
            }
        };
        let binding = match self.binding_registry.resolve_observed(&diagnostic) {
            Ok(binding) => binding,
            Err(_) => {
                return Ok(ResolvedBundleSource::Unavailable(unavailable_source(
                    source,
                    Some(diagnostic.identity.name),
                    PolicyBundleSourceStatus::SourceVaultUnavailable,
                )))
            }
        };
        Ok(ResolvedBundleSource::Ready(ResolvedPolicySourceLocation {
            source_project_id: source.source_project_id.clone(),
            source_project_name: diagnostic.identity.name,
            specification_id: source.specification_id.clone(),
            expected_content_hash: source.content_hash.clone(),
            project_root: diagnostic.root,
            vault_path: binding.vault_path,
        }))
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

enum ResolvedBundleSource {
    Ready(ResolvedPolicySourceLocation),
    Unavailable(PolicyBundleSource),
}

fn task_exclusion_reason_from_source_status(
    status: PolicyBundleSourceStatus,
) -> PolicyBundleTaskExclusionReason {
    match status {
        PolicyBundleSourceStatus::Ready => {
            unreachable!("ready policy source is not an exclusion")
        }
        PolicyBundleSourceStatus::SourceProjectUnavailable => {
            PolicyBundleTaskExclusionReason::SourceProjectUnavailable
        }
        PolicyBundleSourceStatus::SourceIdentityChanged => {
            PolicyBundleTaskExclusionReason::SourceIdentityChanged
        }
        PolicyBundleSourceStatus::SourceVaultUnavailable => {
            PolicyBundleTaskExclusionReason::SourceVaultUnavailable
        }
        PolicyBundleSourceStatus::SpecificationNotApproved => {
            PolicyBundleTaskExclusionReason::SpecificationNotApproved
        }
        PolicyBundleSourceStatus::SpecificationRevisionChanged => {
            PolicyBundleTaskExclusionReason::SpecificationRevisionChanged
        }
        PolicyBundleSourceStatus::SpecificationSourceUnavailable => {
            PolicyBundleTaskExclusionReason::SpecificationSourceUnavailable
        }
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

fn generate_bundle_id() -> String {
    format!("pbd_{}", Uuid::new_v4().simple())
}

fn sort_source_refs(values: &mut [PolicyBundleSourceRef]) {
    values.sort_by(|left, right| {
        left.source_project_id
            .cmp(&right.source_project_id)
            .then_with(|| left.specification_id.cmp(&right.specification_id))
            .then_with(|| left.content_hash.cmp(&right.content_hash))
    });
}

fn sort_egress_sources(values: &mut [PolicyBundleEgressSource]) {
    values.sort_by(|left, right| {
        left.bundle_id
            .cmp(&right.bundle_id)
            .then_with(|| left.source_project_id.cmp(&right.source_project_id))
            .then_with(|| left.specification_id.cmp(&right.specification_id))
    });
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX epoch")
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
        generate_specification_id, initialize_project, CaptureMode, KnowledgeScopeMutation,
        SpecificationRegistry, KNOWLEDGE_SCOPE_REGISTRY_FILE,
    };
    use tempfile::tempdir;

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

    fn team_scope(fixture: &Fixture, include_second: bool) -> KnowledgeScopeMutation {
        let sources = if include_second {
            vec![fixture.source.clone(), fixture.source_two.clone()]
        } else {
            vec![fixture.source.clone()]
        };
        fixture
            .scope_registry
            .create(KnowledgeScopeKind::Team, "Platform team", &sources)
            .unwrap()
    }

    fn policy_sources(fixture: &Fixture, include_second: bool) -> Vec<PolicyBundleSourceInput> {
        let mut sources = vec![PolicyBundleSourceInput {
            source_project: fixture.source.clone(),
            specification_id: fixture.source_specification_id.clone(),
        }];
        if include_second {
            sources.push(PolicyBundleSourceInput {
                source_project: fixture.source_two.clone(),
                specification_id: fixture.source_two_specification_id.clone(),
            });
        }
        sources
    }

    fn active_scope_ids(fixture: &Fixture) -> BTreeSet<String> {
        fixture
            .scope_registry
            .attached(&fixture.active)
            .unwrap()
            .attachments
            .into_iter()
            .map(|attachment| attachment.scope_id)
            .collect()
    }

    #[test]
    fn bundle_authority_is_exact_revision_idempotent_and_path_body_safe() {
        let fixture = setup();
        let scope = team_scope(&fixture, true);
        let sources = policy_sources(&fixture, true);
        let created = fixture
            .bundle_registry
            .create(
                &scope.scope.scope_id,
                "Engineering policy",
                &sources,
                &fixture.scope_registry,
                &fixture.specification_registry,
            )
            .unwrap();
        assert!(created.created);
        assert!(validate_policy_bundle_id(&created.bundle.bundle_id).is_ok());
        assert_eq!(created.bundle.sources.len(), 2);
        assert!(created
            .bundle
            .sources
            .iter()
            .all(|source| source.status == PolicyBundleSourceStatus::Ready));

        let retry = fixture
            .bundle_registry
            .create(
                &scope.scope.scope_id,
                "Engineering policy",
                &sources.iter().cloned().rev().collect::<Vec<_>>(),
                &fixture.scope_registry,
                &fixture.specification_registry,
            )
            .unwrap();
        assert!(!retry.created);
        assert_eq!(retry.bundle.bundle_id, created.bundle.bundle_id);
        assert_eq!(
            fixture
                .bundle_registry
                .list(&fixture.scope_registry)
                .unwrap()
                .bundles
                .len(),
            1
        );

        let stored = fs::read_to_string(fixture.bundle_registry.path()).unwrap();
        let listed = serde_json::to_string(
            &fixture
                .bundle_registry
                .list(&fixture.scope_registry)
                .unwrap(),
        )
        .unwrap();
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
            assert!(!listed.contains(private));
        }

        fs::write(
            fixture.source_vault.join("TeamPolicy.md"),
            "# Team policy\n\nUse signed commits and two reviewers for releases.\n",
        )
        .unwrap();
        let changed = fixture
            .bundle_registry
            .list(&fixture.scope_registry)
            .unwrap();
        assert_eq!(
            changed.bundles[0]
                .sources
                .iter()
                .find(|source| source.specification_id == fixture.source_specification_id)
                .unwrap()
                .status,
            PolicyBundleSourceStatus::SpecificationRevisionChanged
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
        let still_old_revision = fixture
            .bundle_registry
            .list(&fixture.scope_registry)
            .unwrap();
        assert_eq!(
            still_old_revision.bundles[0]
                .sources
                .iter()
                .find(|source| source.specification_id == fixture.source_specification_id)
                .unwrap()
                .status,
            PolicyBundleSourceStatus::SpecificationRevisionChanged
        );
        let replacement = fixture
            .bundle_registry
            .create(
                &scope.scope.scope_id,
                "Engineering policy",
                &sources,
                &fixture.scope_registry,
                &fixture.specification_registry,
            )
            .unwrap();
        assert!(replacement.created);
        assert_ne!(replacement.bundle.bundle_id, created.bundle.bundle_id);

        let single_source_scope = team_scope(&fixture, false);
        assert!(matches!(
            fixture.bundle_registry.create(
                &single_source_scope.scope.scope_id,
                "Out of scope policy",
                &[PolicyBundleSourceInput {
                    source_project: fixture.source_two.clone(),
                    specification_id: fixture.source_two_specification_id.clone(),
                }],
                &fixture.scope_registry,
                &fixture.specification_registry,
            ),
            Err(LeyCoreError::InvalidPolicyBundleRequest(_))
        ));
    }

    #[test]
    fn bundle_activation_is_explicit_parent_scope_gated_and_history_is_retained() {
        let fixture = setup();
        let scope = team_scope(&fixture, false);
        let bundle = fixture
            .bundle_registry
            .create(
                &scope.scope.scope_id,
                "Team rules",
                &policy_sources(&fixture, false),
                &fixture.scope_registry,
                &fixture.specification_registry,
            )
            .unwrap();

        assert!(matches!(
            fixture.bundle_registry.attach(
                &fixture.active,
                &bundle.bundle.bundle_id,
                &fixture.scope_registry,
            ),
            Err(LeyCoreError::InvalidPolicyBundleRequest(_))
        ));
        let no_scope_ids = active_scope_ids(&fixture);
        fixture
            .bundle_registry
            .with_agent_context_sources_locked(&fixture.active, &no_scope_ids, |sources| {
                assert!(sources.active.is_empty());
                assert!(sources.historical.is_empty());
                Ok(())
            })
            .unwrap();

        fixture
            .scope_registry
            .attach(&fixture.active, &scope.scope.scope_id)
            .unwrap();
        let scope_only_ids = active_scope_ids(&fixture);
        fixture
            .bundle_registry
            .with_agent_context_sources_locked(&fixture.active, &scope_only_ids, |sources| {
                assert!(sources.active.is_empty());
                assert!(sources.historical.is_empty());
                Ok(())
            })
            .unwrap();

        let attached = fixture
            .bundle_registry
            .attach(
                &fixture.active,
                &bundle.bundle.bundle_id,
                &fixture.scope_registry,
            )
            .unwrap();
        assert!(attached.created);
        assert_eq!(
            attached.attachment.state,
            PolicyBundleAttachmentState::Active
        );
        let retry = fixture
            .bundle_registry
            .attach(
                &fixture.active,
                &bundle.bundle.bundle_id,
                &fixture.scope_registry,
            )
            .unwrap();
        assert!(!retry.created);
        fixture
            .bundle_registry
            .with_agent_context_sources_locked(&fixture.active, &scope_only_ids, |sources| {
                assert_eq!(sources.active.len(), 1);
                assert_eq!(sources.historical.len(), 1);
                assert_eq!(sources.active[0].bundle_id, bundle.bundle.bundle_id);
                assert_eq!(
                    sources.active[0].specification_id,
                    fixture.source_specification_id
                );
                Ok(())
            })
            .unwrap();

        fixture
            .scope_registry
            .detach(&fixture.active, &scope.scope.scope_id)
            .unwrap();
        let detached = fixture
            .bundle_registry
            .attached(&fixture.active, &fixture.scope_registry)
            .unwrap();
        assert_eq!(
            detached.attachments[0].state,
            PolicyBundleAttachmentState::ParentScopeDetachedOrReattached
        );
        let no_scope_ids = active_scope_ids(&fixture);
        fixture
            .bundle_registry
            .with_agent_context_sources_locked(&fixture.active, &no_scope_ids, |sources| {
                assert!(sources.active.is_empty());
                assert_eq!(sources.historical.len(), 1);
                Ok(())
            })
            .unwrap();

        fixture
            .scope_registry
            .attach(&fixture.active, &scope.scope.scope_id)
            .unwrap();
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
            .detach(&fixture.active, &bundle.bundle.bundle_id)
            .unwrap()
            .is_some());
        assert!(fixture
            .bundle_registry
            .attached(&fixture.active, &fixture.scope_registry)
            .unwrap()
            .attachments
            .is_empty());
        let active_scope_ids = active_scope_ids(&fixture);
        fixture
            .bundle_registry
            .with_agent_context_sources_locked(&fixture.active, &active_scope_ids, |sources| {
                assert!(sources.active.is_empty());
                assert_eq!(sources.historical.len(), 1);
                assert_eq!(sources.historical[0].bundle_id, bundle.bundle.bundle_id);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn registry_is_private_and_corruption_or_symlink_fails_closed() {
        let fixture = setup();
        let scope = team_scope(&fixture, false);
        fixture
            .bundle_registry
            .create(
                &scope.scope.scope_id,
                "Team rules",
                &policy_sources(&fixture, false),
                &fixture.scope_registry,
                &fixture.specification_registry,
            )
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

        fs::write(
            fixture.bundle_registry.path(),
            r#"{"schemaVersion":99,"bundles":{},"attachments":{},"attachmentHistory":{}}"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                fixture.bundle_registry.path(),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        assert!(matches!(
            fixture.bundle_registry.list(&fixture.scope_registry),
            Err(LeyCoreError::InvalidPolicyBundleRegistry(_))
        ));
    }
}
