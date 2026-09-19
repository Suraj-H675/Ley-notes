use crate::{
    default_binding_registry_path, diagnose_project, validate_project_id, BindingRegistry,
    LeyCoreError, ProjectCatalog, BINDING_REGISTRY_FILE, METADATA_FILE_LIMIT_BYTES,
    PROJECT_CATALOG_FILE,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const KNOWLEDGE_SCOPE_REGISTRY_FILE: &str = "knowledge-scopes-v1.json";
const KNOWLEDGE_SCOPE_REGISTRY_LOCK_FILE: &str = "knowledge-scopes-v1.lock";
pub const KNOWLEDGE_SCOPE_REGISTRY_SCHEMA_VERSION: u32 = 1;
pub const MAX_KNOWLEDGE_SCOPES: usize = 32;
pub const MAX_KNOWLEDGE_SCOPE_SOURCES: usize = 16;
pub const MAX_ATTACHED_KNOWLEDGE_SCOPES_PER_PROJECT: usize = 8;
pub const MAX_KNOWLEDGE_SCOPE_HISTORY_PER_PROJECT: usize = 256;

const PRIVACY_NOTICE: &str = "Knowledge Scope authority is OS-private. It stores stable scope/project identities, kind/name, read-only membership, explicit project attachments, and bounded attachment history only. Project and vault paths remain owned by Ley's private project catalog and binding registry. Scope membership is immutable in this first slice; changing membership requires creating a new scope.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeScopeKind {
    Team,
    Organization,
}

impl KnowledgeScopeKind {
    pub fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "team" => Ok(Self::Team),
            "organization" | "org" => Ok(Self::Organization),
            _ => Err(LeyCoreError::InvalidKnowledgeScopeRequest(
                "knowledge scope kind must be team or organization".to_owned(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeScopePermission {
    ReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeScopeSourceStatus {
    Ready,
    SourceProjectUnavailable,
    SourceIdentityChanged,
    SourceVaultUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeScopeSource {
    pub source_project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_project_name: Option<String>,
    pub status: KnowledgeScopeSourceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeScope {
    pub scope_id: String,
    pub kind: KnowledgeScopeKind,
    pub name: String,
    pub permission: KnowledgeScopePermission,
    pub sources: Vec<KnowledgeScopeSource>,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeScopeMutation {
    pub scope: KnowledgeScope,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeScopeList {
    pub scopes: Vec<KnowledgeScope>,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeScopeAttachment {
    pub active_project_id: String,
    pub scope_id: String,
    pub kind: KnowledgeScopeKind,
    pub name: String,
    pub permission: KnowledgeScopePermission,
    pub source_count: usize,
    pub attached_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeScopeAttachmentMutation {
    pub attachment: KnowledgeScopeAttachment,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeScopeAttachmentList {
    pub active_project_id: String,
    pub attachments: Vec<KnowledgeScopeAttachment>,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeScopeEgressSource {
    pub scope_id: String,
    pub source_project_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeScopeEgressSources {
    pub active: Vec<KnowledgeScopeEgressSource>,
    pub historical: Vec<KnowledgeScopeEgressSource>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedKnowledgeScopeSource {
    pub source_project_id: String,
    pub source_project_name: String,
    pub project_root: PathBuf,
    pub vault_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedKnowledgeScope {
    pub scope_id: String,
    pub kind: KnowledgeScopeKind,
    pub name: String,
    pub ready: Vec<ResolvedKnowledgeScopeSource>,
    pub unavailable: Vec<KnowledgeScopeSource>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedKnowledgeScopes {
    pub active_project_id: String,
    pub scopes: Vec<ResolvedKnowledgeScope>,
    pub historical_sources: Vec<KnowledgeScopeEgressSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct KnowledgeScopeEntry {
    kind: KnowledgeScopeKind,
    name: String,
    source_project_ids: Vec<String>,
    created_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct KnowledgeScopeRegistryDocument {
    schema_version: u32,
    scopes: BTreeMap<String, KnowledgeScopeEntry>,
    #[serde(default)]
    attachments: BTreeMap<String, BTreeMap<String, u64>>,
    #[serde(default)]
    attachment_history: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

impl KnowledgeScopeRegistryDocument {
    fn empty() -> Self {
        Self {
            schema_version: KNOWLEDGE_SCOPE_REGISTRY_SCHEMA_VERSION,
            scopes: BTreeMap::new(),
            attachments: BTreeMap::new(),
            attachment_history: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), LeyCoreError> {
        if self.schema_version != KNOWLEDGE_SCOPE_REGISTRY_SCHEMA_VERSION {
            return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        if self.scopes.len() > MAX_KNOWLEDGE_SCOPES {
            return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                "registry has more than {MAX_KNOWLEDGE_SCOPES} knowledge scopes"
            )));
        }
        for (scope_id, entry) in &self.scopes {
            validate_knowledge_scope_id(scope_id)
                .map_err(LeyCoreError::InvalidKnowledgeScopeRegistry)?;
            validate_scope_name(&entry.name)
                .map_err(LeyCoreError::InvalidKnowledgeScopeRegistry)?;
            if entry.source_project_ids.is_empty()
                || entry.source_project_ids.len() > MAX_KNOWLEDGE_SCOPE_SOURCES
            {
                return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                    "scope {scope_id} must contain between 1 and {MAX_KNOWLEDGE_SCOPE_SOURCES} source projects"
                )));
            }
            if entry.created_at_unix_ms == 0 {
                return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(
                    "knowledge scope creation time must be non-zero".to_owned(),
                ));
            }
            let mut unique = BTreeSet::new();
            for project_id in &entry.source_project_ids {
                validate_project_id(project_id).map_err(|error| {
                    LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                        "invalid source project ID in scope {scope_id}: {error}"
                    ))
                })?;
                if !unique.insert(project_id) {
                    return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                        "scope {scope_id} contains duplicate source project IDs"
                    )));
                }
            }
        }
        for (active_project_id, attachments) in &self.attachments {
            validate_project_id(active_project_id).map_err(|error| {
                LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                    "invalid active project ID '{active_project_id}': {error}"
                ))
            })?;
            if attachments.len() > MAX_ATTACHED_KNOWLEDGE_SCOPES_PER_PROJECT {
                return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                    "project {active_project_id} has more than {MAX_ATTACHED_KNOWLEDGE_SCOPES_PER_PROJECT} attached knowledge scopes"
                )));
            }
            for (scope_id, attached_at) in attachments {
                let scope = self.scopes.get(scope_id).ok_or_else(|| {
                    LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                        "attachment references missing scope {scope_id}"
                    ))
                })?;
                if scope
                    .source_project_ids
                    .iter()
                    .any(|id| id == active_project_id)
                {
                    return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                        "scope {scope_id} cannot be attached to one of its source projects"
                    )));
                }
                if *attached_at == 0 {
                    return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(
                        "knowledge scope attachment time must be non-zero".to_owned(),
                    ));
                }
            }
        }
        for (active_project_id, history) in &self.attachment_history {
            validate_project_id(active_project_id).map_err(|error| {
                LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                    "invalid historical active project ID '{active_project_id}': {error}"
                ))
            })?;
            if history.len() > MAX_KNOWLEDGE_SCOPE_HISTORY_PER_PROJECT {
                return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                    "project {active_project_id} has more than {MAX_KNOWLEDGE_SCOPE_HISTORY_PER_PROJECT} historical knowledge scope attachments"
                )));
            }
            for (scope_id, project_ids) in history {
                validate_knowledge_scope_id(scope_id)
                    .map_err(LeyCoreError::InvalidKnowledgeScopeRegistry)?;
                if project_ids.is_empty() || project_ids.len() > MAX_KNOWLEDGE_SCOPE_SOURCES {
                    return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                        "historical scope {scope_id} has an invalid source project count"
                    )));
                }
                let mut unique = BTreeSet::new();
                for project_id in project_ids {
                    validate_project_id(project_id).map_err(|error| {
                        LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                            "invalid historical source project ID for {scope_id}: {error}"
                        ))
                    })?;
                    if project_id == active_project_id || !unique.insert(project_id) {
                        return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                            "historical scope {scope_id} has invalid source ancestry"
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct KnowledgeScopeRegistry {
    path: PathBuf,
    project_catalog: ProjectCatalog,
    binding_registry: BindingRegistry,
}

impl KnowledgeScopeRegistry {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        Ok(Self::at(
            default_binding_registry_path()?.with_file_name(KNOWLEDGE_SCOPE_REGISTRY_FILE),
        ))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            project_catalog: ProjectCatalog::at(path.with_file_name(PROJECT_CATALOG_FILE)),
            binding_registry: BindingRegistry::at(path.with_file_name(BINDING_REGISTRY_FILE)),
            path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn create(
        &self,
        kind: KnowledgeScopeKind,
        name: &str,
        source_projects: &[PathBuf],
    ) -> Result<KnowledgeScopeMutation, LeyCoreError> {
        let name = validate_scope_name(name).map_err(LeyCoreError::InvalidKnowledgeScopeRequest)?;
        if source_projects.is_empty() || source_projects.len() > MAX_KNOWLEDGE_SCOPE_SOURCES {
            return Err(LeyCoreError::InvalidKnowledgeScopeRequest(format!(
                "knowledge scope must contain between 1 and {MAX_KNOWLEDGE_SCOPE_SOURCES} source projects"
            )));
        }
        let mut source_project_ids = Vec::with_capacity(source_projects.len());
        let mut unique = BTreeSet::new();
        for source_project in source_projects {
            let diagnostic = diagnose_project(source_project)?;
            self.binding_registry.resolve(&diagnostic.root, None)?;
            if !unique.insert(diagnostic.identity.project_id.clone()) {
                return Err(LeyCoreError::InvalidKnowledgeScopeRequest(
                    "knowledge scope source projects must be unique".to_owned(),
                ));
            }
            source_project_ids.push(diagnostic.identity.project_id);
        }
        source_project_ids.sort();
        let created_at_unix_ms = unix_time_ms();
        let (scope_id, created) = self.mutate(|document| {
            if let Some((scope_id, _)) = document.scopes.iter().find(|(_, entry)| {
                entry.kind == kind
                    && entry.name == name
                    && entry.source_project_ids == source_project_ids
            }) {
                return Ok((scope_id.clone(), false));
            }
            if document.scopes.len() >= MAX_KNOWLEDGE_SCOPES {
                return Err(LeyCoreError::InvalidKnowledgeScopeRequest(format!(
                    "at most {MAX_KNOWLEDGE_SCOPES} knowledge scopes may be retained"
                )));
            }
            let scope_id = generate_scope_id();
            document.scopes.insert(
                scope_id.clone(),
                KnowledgeScopeEntry {
                    kind,
                    name: name.clone(),
                    source_project_ids: source_project_ids.clone(),
                    created_at_unix_ms,
                },
            );
            Ok((scope_id, true))
        })?;
        let scope = self
            .list()?
            .scopes
            .into_iter()
            .find(|scope| scope.scope_id == scope_id)
            .expect("created scope remains in registry");
        Ok(KnowledgeScopeMutation { scope, created })
    }

    pub fn list(&self) -> Result<KnowledgeScopeList, LeyCoreError> {
        let document = self.read_locked()?;
        let mut scopes = document
            .scopes
            .into_iter()
            .map(|(scope_id, entry)| self.resolve_scope(scope_id, entry))
            .collect::<Result<Vec<_>, _>>()?;
        scopes.sort_by(|left, right| {
            right
                .created_at_unix_ms
                .cmp(&left.created_at_unix_ms)
                .then_with(|| left.scope_id.cmp(&right.scope_id))
        });
        Ok(KnowledgeScopeList {
            scopes,
            privacy_notice: PRIVACY_NOTICE,
        })
    }

    pub fn attach(
        &self,
        active_project: impl AsRef<Path>,
        scope_id: &str,
    ) -> Result<KnowledgeScopeAttachmentMutation, LeyCoreError> {
        validate_knowledge_scope_id(scope_id)
            .map_err(LeyCoreError::InvalidKnowledgeScopeRequest)?;
        let active = diagnose_project(active_project)?;
        self.binding_registry.resolve(&active.root, None)?;
        let active_project_id = active.identity.project_id;
        let now = unix_time_ms();
        let (scope, attached_at, created) = self.mutate(|document| {
            let scope = document
                .scopes
                .get(scope_id)
                .cloned()
                .ok_or_else(|| LeyCoreError::KnowledgeScopeNotFound(scope_id.to_owned()))?;
            if scope
                .source_project_ids
                .iter()
                .any(|project_id| project_id == &active_project_id)
            {
                return Err(LeyCoreError::InvalidKnowledgeScopeRequest(
                    "a project cannot attach a knowledge scope that includes itself as a source"
                        .to_owned(),
                ));
            }
            if let Some(existing) = document
                .attachments
                .get(&active_project_id)
                .and_then(|items| items.get(scope_id))
                .copied()
            {
                return Ok((scope, existing, false));
            }
            let current = document
                .attachments
                .get(&active_project_id)
                .map_or(0, BTreeMap::len);
            if current >= MAX_ATTACHED_KNOWLEDGE_SCOPES_PER_PROJECT {
                return Err(LeyCoreError::InvalidKnowledgeScopeRequest(format!(
                    "an active project may attach at most {MAX_ATTACHED_KNOWLEDGE_SCOPES_PER_PROJECT} knowledge scopes"
                )));
            }
            let history = document
                .attachment_history
                .entry(active_project_id.clone())
                .or_default();
            if !history.contains_key(scope_id)
                && history.len() >= MAX_KNOWLEDGE_SCOPE_HISTORY_PER_PROJECT
            {
                return Err(LeyCoreError::InvalidKnowledgeScopeRequest(format!(
                    "project reached the {MAX_KNOWLEDGE_SCOPE_HISTORY_PER_PROJECT} knowledge-scope history limit"
                )));
            }
            history.insert(scope_id.to_owned(), scope.source_project_ids.clone());
            document
                .attachments
                .entry(active_project_id.clone())
                .or_default()
                .insert(scope_id.to_owned(), now);
            Ok((scope, now, true))
        })?;
        Ok(KnowledgeScopeAttachmentMutation {
            attachment: attachment(&active_project_id, scope_id, &scope, attached_at),
            created,
        })
    }

    pub fn attached(
        &self,
        active_project: impl AsRef<Path>,
    ) -> Result<KnowledgeScopeAttachmentList, LeyCoreError> {
        let active_project_id = diagnose_project(active_project)?.identity.project_id;
        let document = self.read_locked()?;
        let mut attachments = document
            .attachments
            .get(&active_project_id)
            .into_iter()
            .flat_map(|items| items.iter())
            .map(|(scope_id, attached_at)| {
                let scope = document.scopes.get(scope_id).ok_or_else(|| {
                    LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                        "attachment references missing scope {scope_id}"
                    ))
                })?;
                Ok(attachment(
                    &active_project_id,
                    scope_id,
                    scope,
                    *attached_at,
                ))
            })
            .collect::<Result<Vec<_>, LeyCoreError>>()?;
        attachments.sort_by(|left, right| {
            right
                .attached_at_unix_ms
                .cmp(&left.attached_at_unix_ms)
                .then_with(|| left.scope_id.cmp(&right.scope_id))
        });
        Ok(KnowledgeScopeAttachmentList {
            active_project_id,
            attachments,
            privacy_notice: PRIVACY_NOTICE,
        })
    }

    pub fn detach(
        &self,
        active_project: impl AsRef<Path>,
        scope_id: &str,
    ) -> Result<Option<KnowledgeScopeAttachment>, LeyCoreError> {
        validate_knowledge_scope_id(scope_id)
            .map_err(LeyCoreError::InvalidKnowledgeScopeRequest)?;
        let active_project_id = diagnose_project(active_project)?.identity.project_id;
        self.mutate(|document| {
            let Some(attachments) = document.attachments.get_mut(&active_project_id) else {
                return Ok(None);
            };
            let Some(attached_at) = attachments.remove(scope_id) else {
                return Ok(None);
            };
            if attachments.is_empty() {
                document.attachments.remove(&active_project_id);
            }
            let scope = document.scopes.get(scope_id).ok_or_else(|| {
                LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                    "attachment references missing scope {scope_id}"
                ))
            })?;
            Ok(Some(attachment(
                &active_project_id,
                scope_id,
                scope,
                attached_at,
            )))
        })
    }

    pub fn with_agent_context_sources_locked<T>(
        &self,
        active_project: impl AsRef<Path>,
        operation: impl FnOnce(KnowledgeScopeEgressSources) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let active_project_id = diagnose_project(active_project)?.identity.project_id;
        self.with_locked_document(|document| {
            let mut active = Vec::new();
            for scope_id in document
                .attachments
                .get(&active_project_id)
                .into_iter()
                .flat_map(|items| items.keys())
            {
                let scope = document.scopes.get(scope_id).ok_or_else(|| {
                    LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                        "attachment references missing scope {scope_id}"
                    ))
                })?;
                active.extend(scope.source_project_ids.iter().map(|source_project_id| {
                    KnowledgeScopeEgressSource {
                        scope_id: scope_id.clone(),
                        source_project_id: source_project_id.clone(),
                    }
                }));
            }
            let mut historical = document
                .attachment_history
                .get(&active_project_id)
                .into_iter()
                .flat_map(|items| items.iter())
                .flat_map(|(scope_id, source_ids)| {
                    source_ids
                        .iter()
                        .map(|source_project_id| KnowledgeScopeEgressSource {
                            scope_id: scope_id.clone(),
                            source_project_id: source_project_id.clone(),
                        })
                })
                .collect::<Vec<_>>();
            active.sort_by(|left, right| {
                left.scope_id
                    .cmp(&right.scope_id)
                    .then_with(|| left.source_project_id.cmp(&right.source_project_id))
            });
            historical.sort_by(|left, right| {
                left.scope_id
                    .cmp(&right.scope_id)
                    .then_with(|| left.source_project_id.cmp(&right.source_project_id))
            });
            historical.dedup();
            operation(KnowledgeScopeEgressSources { active, historical })
        })
    }

    pub(crate) fn with_resolved_scopes_locked<T>(
        &self,
        active_project: impl AsRef<Path>,
        operation: impl FnOnce(ResolvedKnowledgeScopes) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let active_project_id = diagnose_project(active_project)?.identity.project_id;
        self.with_locked_document(|document| {
            let mut scopes = Vec::new();
            for scope_id in document
                .attachments
                .get(&active_project_id)
                .into_iter()
                .flat_map(|items| items.keys())
            {
                let entry = document.scopes.get(scope_id).ok_or_else(|| {
                    LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
                        "attachment references missing scope {scope_id}"
                    ))
                })?;
                let mut ready = Vec::new();
                let mut unavailable = Vec::new();
                for source_project_id in &entry.source_project_ids {
                    match self.resolve_source_for_agent(source_project_id)? {
                        ResolvedSource::Ready(source) => ready.push(source),
                        ResolvedSource::Unavailable(source) => unavailable.push(source),
                    }
                }
                ready.sort_by(|left, right| left.source_project_id.cmp(&right.source_project_id));
                unavailable
                    .sort_by(|left, right| left.source_project_id.cmp(&right.source_project_id));
                scopes.push(ResolvedKnowledgeScope {
                    scope_id: scope_id.clone(),
                    kind: entry.kind,
                    name: entry.name.clone(),
                    ready,
                    unavailable,
                });
            }
            scopes.sort_by(|left, right| left.scope_id.cmp(&right.scope_id));
            let mut historical_sources = document
                .attachment_history
                .get(&active_project_id)
                .into_iter()
                .flat_map(|items| items.iter())
                .flat_map(|(scope_id, source_ids)| {
                    source_ids
                        .iter()
                        .map(|source_project_id| KnowledgeScopeEgressSource {
                            scope_id: scope_id.clone(),
                            source_project_id: source_project_id.clone(),
                        })
                })
                .collect::<Vec<_>>();
            historical_sources.sort_by(|left, right| {
                left.scope_id
                    .cmp(&right.scope_id)
                    .then_with(|| left.source_project_id.cmp(&right.source_project_id))
            });
            historical_sources.dedup();
            operation(ResolvedKnowledgeScopes {
                active_project_id,
                scopes,
                historical_sources,
            })
        })
    }

    fn resolve_scope(
        &self,
        scope_id: String,
        entry: KnowledgeScopeEntry,
    ) -> Result<KnowledgeScope, LeyCoreError> {
        let mut sources = entry
            .source_project_ids
            .iter()
            .map(|source_project_id| self.resolve_source(source_project_id))
            .collect::<Result<Vec<_>, _>>()?;
        sources.sort_by(|left, right| left.source_project_id.cmp(&right.source_project_id));
        Ok(KnowledgeScope {
            scope_id,
            kind: entry.kind,
            name: entry.name,
            permission: KnowledgeScopePermission::ReadOnly,
            sources,
            created_at_unix_ms: entry.created_at_unix_ms,
        })
    }

    fn resolve_source(
        &self,
        source_project_id: &str,
    ) -> Result<KnowledgeScopeSource, LeyCoreError> {
        match self.resolve_source_for_agent(source_project_id)? {
            ResolvedSource::Ready(source) => Ok(KnowledgeScopeSource {
                source_project_id: source.source_project_id,
                source_project_name: Some(source.source_project_name),
                status: KnowledgeScopeSourceStatus::Ready,
            }),
            ResolvedSource::Unavailable(source) => Ok(source),
        }
    }

    fn resolve_source_for_agent(
        &self,
        source_project_id: &str,
    ) -> Result<ResolvedSource, LeyCoreError> {
        let Some(observed) = self.project_catalog.get(source_project_id)? else {
            return Ok(ResolvedSource::Unavailable(KnowledgeScopeSource {
                source_project_id: source_project_id.to_owned(),
                source_project_name: None,
                status: KnowledgeScopeSourceStatus::SourceProjectUnavailable,
            }));
        };
        let diagnostic = match diagnose_project(&observed.root_path) {
            Ok(diagnostic) if diagnostic.identity.project_id == source_project_id => diagnostic,
            Ok(diagnostic) => {
                return Ok(ResolvedSource::Unavailable(KnowledgeScopeSource {
                    source_project_id: source_project_id.to_owned(),
                    source_project_name: Some(diagnostic.identity.name),
                    status: KnowledgeScopeSourceStatus::SourceIdentityChanged,
                }))
            }
            Err(_) => {
                return Ok(ResolvedSource::Unavailable(KnowledgeScopeSource {
                    source_project_id: source_project_id.to_owned(),
                    source_project_name: None,
                    status: KnowledgeScopeSourceStatus::SourceProjectUnavailable,
                }))
            }
        };
        let binding = match self.binding_registry.resolve_observed(&diagnostic) {
            Ok(binding) => binding,
            Err(_) => {
                return Ok(ResolvedSource::Unavailable(KnowledgeScopeSource {
                    source_project_id: source_project_id.to_owned(),
                    source_project_name: Some(diagnostic.identity.name),
                    status: KnowledgeScopeSourceStatus::SourceVaultUnavailable,
                }))
            }
        };
        Ok(ResolvedSource::Ready(ResolvedKnowledgeScopeSource {
            source_project_id: source_project_id.to_owned(),
            source_project_name: diagnostic.identity.name,
            project_root: diagnostic.root,
            vault_path: binding.vault_path,
        }))
    }

    fn with_locked_document<T>(
        &self,
        operation: impl FnOnce(&KnowledgeScopeRegistryDocument) -> Result<T, LeyCoreError>,
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

    fn read_locked(&self) -> Result<KnowledgeScopeRegistryDocument, LeyCoreError> {
        self.with_locked_document(|document| Ok(document.clone()))
    }

    fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut KnowledgeScopeRegistryDocument) -> Result<T, LeyCoreError>,
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
            LeyCoreError::InvalidKnowledgeScopeRegistry(
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

    fn read_document(&self) -> Result<KnowledgeScopeRegistryDocument, LeyCoreError> {
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(KnowledgeScopeRegistryDocument::empty())
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
        let document: KnowledgeScopeRegistryDocument =
            serde_json::from_slice(&bytes).map_err(|source| LeyCoreError::Json {
                path: self.path.clone(),
                source,
            })?;
        document.validate()?;
        Ok(document)
    }

    fn write_document(
        &self,
        document: &KnowledgeScopeRegistryDocument,
    ) -> Result<(), LeyCoreError> {
        reject_non_regular_if_present(&self.path)?;
        let mut body = serde_json::to_vec_pretty(document)
            .expect("validated knowledge scope registry is serializable");
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
        self.path.with_file_name(KNOWLEDGE_SCOPE_REGISTRY_LOCK_FILE)
    }
}

enum ResolvedSource {
    Ready(ResolvedKnowledgeScopeSource),
    Unavailable(KnowledgeScopeSource),
}

fn attachment(
    active_project_id: &str,
    scope_id: &str,
    scope: &KnowledgeScopeEntry,
    attached_at_unix_ms: u64,
) -> KnowledgeScopeAttachment {
    KnowledgeScopeAttachment {
        active_project_id: active_project_id.to_owned(),
        scope_id: scope_id.to_owned(),
        kind: scope.kind,
        name: scope.name.clone(),
        permission: KnowledgeScopePermission::ReadOnly,
        source_count: scope.source_project_ids.len(),
        attached_at_unix_ms,
    }
}

fn validate_scope_name(value: &str) -> Result<String, String> {
    let value = value.trim();
    let characters = value.chars().count();
    if characters == 0 || characters > 128 || value.chars().any(char::is_control) {
        return Err("knowledge scope name must contain 1 to 128 visible characters".to_owned());
    }
    Ok(value.to_owned())
}

fn generate_scope_id() -> String {
    format!("ksc_{}", Uuid::new_v4().simple())
}

pub(crate) fn validate_knowledge_scope_id(value: &str) -> Result<(), String> {
    let Some(uuid) = value.strip_prefix("ksc_") else {
        return Err("scopeId must start with ksc_".to_owned());
    };
    if uuid.len() != 32
        || !uuid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || Uuid::parse_str(uuid).is_err()
    {
        return Err("scopeId must contain a 32-character lowercase hexadecimal UUID".to_owned());
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
                return Err(LeyCoreError::InvalidKnowledgeScopeRegistry(format!(
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
        registry: KnowledgeScopeRegistry,
    }

    fn setup() -> Fixture {
        let base = tempdir().unwrap();
        let config = base.path().join("config");
        let registry = KnowledgeScopeRegistry::at(config.join(KNOWLEDGE_SCOPE_REGISTRY_FILE));
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
        }
    }

    #[test]
    fn scope_authority_is_immutable_idempotent_project_scoped_and_path_safe() {
        let fixture = setup();
        let created = fixture
            .registry
            .create(
                KnowledgeScopeKind::Team,
                "Platform team",
                &[fixture.source.clone(), fixture.source_two.clone()],
            )
            .unwrap();
        assert!(created.created);
        assert_eq!(created.scope.permission, KnowledgeScopePermission::ReadOnly);
        assert_eq!(created.scope.sources.len(), 2);
        assert!(created
            .scope
            .sources
            .iter()
            .all(|source| source.status == KnowledgeScopeSourceStatus::Ready));

        let retry = fixture
            .registry
            .create(
                KnowledgeScopeKind::Team,
                "Platform team",
                &[fixture.source_two.clone(), fixture.source.clone()],
            )
            .unwrap();
        assert!(!retry.created);
        assert_eq!(retry.scope.scope_id, created.scope.scope_id);
        assert_eq!(fixture.registry.list().unwrap().scopes.len(), 1);

        let attached = fixture
            .registry
            .attach(&fixture.active, &created.scope.scope_id)
            .unwrap();
        assert!(attached.created);
        assert_eq!(
            attached.attachment.permission,
            KnowledgeScopePermission::ReadOnly
        );
        let retry_attach = fixture
            .registry
            .attach(&fixture.active, &created.scope.scope_id)
            .unwrap();
        assert!(!retry_attach.created);
        assert!(fixture
            .registry
            .attached(&fixture.active_two)
            .unwrap()
            .attachments
            .is_empty());

        let serialized = serde_json::to_string(&fixture.registry.list().unwrap()).unwrap();
        assert!(!serialized.contains(fixture.active.to_str().unwrap()));
        assert!(!serialized.contains(fixture.source.to_str().unwrap()));
        assert!(!serialized.contains(fixture.source_two.to_str().unwrap()));
        assert!(!serialized.contains(fixture.source_vault.to_str().unwrap()));

        assert!(matches!(
            fixture
                .registry
                .create(
                    KnowledgeScopeKind::Team,
                    "Self source",
                    std::slice::from_ref(&fixture.active)
                )
                .and_then(|scope| fixture
                    .registry
                    .attach(&fixture.active, &scope.scope.scope_id)),
            Err(LeyCoreError::InvalidKnowledgeScopeRequest(_))
        ));
    }

    #[test]
    fn source_availability_is_disclosed_without_removing_scope_authority() {
        let fixture = setup();
        let created = fixture
            .registry
            .create(
                KnowledgeScopeKind::Organization,
                "Architecture group",
                std::slice::from_ref(&fixture.source),
            )
            .unwrap();
        fs::remove_dir_all(&fixture.source_vault).unwrap();
        let no_vault = fixture.registry.list().unwrap();
        assert_eq!(
            no_vault.scopes[0].sources[0].status,
            KnowledgeScopeSourceStatus::SourceVaultUnavailable
        );
        fs::remove_dir_all(&fixture.source).unwrap();
        let no_project = fixture.registry.list().unwrap();
        assert_eq!(no_project.scopes[0].scope_id, created.scope.scope_id);
        assert_eq!(
            no_project.scopes[0].sources[0].status,
            KnowledgeScopeSourceStatus::SourceProjectUnavailable
        );
    }

    #[test]
    fn registry_is_private_bounded_and_corruption_or_symlink_fails_closed() {
        let fixture = setup();
        for index in 0..MAX_KNOWLEDGE_SCOPES {
            fixture
                .registry
                .create(
                    KnowledgeScopeKind::Team,
                    &format!("Team {index}"),
                    std::slice::from_ref(&fixture.source),
                )
                .unwrap();
        }
        assert!(matches!(
            fixture.registry.create(
                KnowledgeScopeKind::Team,
                "Overflow team",
                std::slice::from_ref(&fixture.source)
            ),
            Err(LeyCoreError::InvalidKnowledgeScopeRequest(_))
        ));

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
            let unsafe_path = fixture._base.path().join("unsafe-scopes.json");
            symlink(&target, &unsafe_path).unwrap();
            assert!(matches!(
                KnowledgeScopeRegistry::at(&unsafe_path).list(),
                Err(LeyCoreError::UnsafeProjectLayout(_))
            ));
        }

        fs::write(
            fixture.registry.path(),
            r#"{"schemaVersion":99,"scopes":{}}"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(fixture.registry.path(), fs::Permissions::from_mode(0o600))
                .unwrap();
        }
        assert!(matches!(
            fixture.registry.list(),
            Err(LeyCoreError::InvalidKnowledgeScopeRegistry(_))
        ));
    }

    #[test]
    fn locked_scope_reads_serialize_with_detach_and_keep_historical_source_ancestry() {
        use std::sync::mpsc;
        use std::time::Duration;

        let fixture = setup();
        let scope = fixture
            .registry
            .create(
                KnowledgeScopeKind::Team,
                "Platform team",
                std::slice::from_ref(&fixture.source),
            )
            .unwrap();
        fixture
            .registry
            .attach(&fixture.active, &scope.scope.scope_id)
            .unwrap();
        let source_project_id = scope.scope.sources[0].source_project_id.clone();
        let scope_id = scope.scope.scope_id.clone();

        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let reader_registry = fixture.registry.clone();
        let reader_active = fixture.active.clone();
        let reader = std::thread::spawn(move || {
            reader_registry
                .with_resolved_scopes_locked(reader_active, |resolved| {
                    assert_eq!(resolved.scopes.len(), 1);
                    assert_eq!(resolved.scopes[0].ready.len(), 1);
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap();
        });
        entered_rx.recv().unwrap();

        let (detached_tx, detached_rx) = mpsc::channel();
        let writer_registry = fixture.registry.clone();
        let writer_active = fixture.active.clone();
        let writer_scope_id = scope_id.clone();
        let writer = std::thread::spawn(move || {
            writer_registry
                .detach(writer_active, &writer_scope_id)
                .unwrap();
            detached_tx.send(()).unwrap();
        });
        assert!(detached_rx.recv_timeout(Duration::from_millis(50)).is_err());
        release_tx.send(()).unwrap();
        reader.join().unwrap();
        detached_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        writer.join().unwrap();
        assert!(fixture
            .registry
            .attached(&fixture.active)
            .unwrap()
            .attachments
            .is_empty());
        fixture
            .registry
            .with_agent_context_sources_locked(&fixture.active, |sources| {
                assert!(sources.active.is_empty());
                assert!(sources.historical.iter().any(|source| {
                    source.scope_id == scope_id && source.source_project_id == source_project_id
                }));
                Ok(())
            })
            .unwrap();
    }
}
