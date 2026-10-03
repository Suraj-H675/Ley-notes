use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
#[cfg(test)]
use ley_core::{
    correct_learning, erase_project_memory, ingest_project, initialize_project, read_learning,
    rename_session, review_learning,
};
use ley_core::{
    correct_learning_with_continuity_transition, diagnose_project,
    erase_project_memory_with_continuity_transition, erase_project_memory_with_native_authority,
    erase_session_memory_with_continuity_transition, establish_native_born_project_authorities,
    export_portable_continuity, generate_learning_request_id, generate_request_id,
    ingest_project_with_continuity_transition,
    ingest_project_with_expected_capture_plan_and_native_authority,
    ingest_project_with_native_authority, initialize_project_retiring_bootstrap,
    list_learning_contexts, list_learning_contexts_with_continuity_transition, list_sessions,
    list_sessions_with_continuity_transition, native_born_project_registration_exists,
    prepare_legacy_project_binding, preview_initial_capture,
    project_activity_view_with_continuity_transition,
    project_artifact_inventory_with_continuity_transition, project_memory_overview,
    project_memory_overview_with_continuity_transition, project_resume_context,
    project_resume_context_with_continuity_transition, project_session_stats,
    project_session_stats_with_continuity_transition,
    read_learning_context_with_continuity_transition, read_learning_with_continuity_transition,
    read_project_cited_evidence_with_continuity_transition,
    read_project_cited_media_with_continuity_transition,
    read_session_context_with_continuity_transition,
    read_session_turns_context_with_continuity_transition, register_native_born_project,
    rename_session_with_continuity_transition, review_learning_with_continuity_transition,
    search_observed_projects, search_project_memory_with_continuity_transition,
    update_capture_mode, validate_project_memory, AgentEgressPolicy, ApprovedSourceAuthorityList,
    ApprovedSourceKind, ApprovedSourceRegistry, ArtifactMediaType, BindingRegistry, BindingSource,
    CaptureFile, CaptureMode, CapturePolicy, ContinuityStore, CorrectLearningInput,
    CrossProjectSearch, EgressPolicyRegistry, EraseSessionMemoryInput, EvidenceExcerpt,
    GraphCitation, IngestionResult, LearningActor, LearningContextPack, LearningEvidenceInput,
    LearningFeedbackAction, LearningList, LearningListScope, LeyCoreError, MemoryOverview,
    ProjectActivityView, ProjectAgentEgressPolicy, ProjectArtifactInventory, ProjectCatalog,
    ProjectDiagnostic, ProjectMemorySearch, ProjectMemorySearchLimits, ProjectProblemScope,
    ProjectResumePack, ProjectVaultBinding, RenameSessionInput, ReviewLearningInput,
    RevisionCompatibility, SessionContextPack, SessionMemoryErasure, SessionSummary,
    SessionTurnsContextPack, SpecificationRegistry, DEFAULT_ARTIFACT_RESULTS,
    DEFAULT_CROSS_PROJECT_SEARCH_RESULTS, DEFAULT_LEARNING_CONTEXT_ARTIFACTS,
    DEFAULT_LEARNING_CONTEXT_CHARACTERS, DEFAULT_LEARNING_CONTEXT_EVIDENCE,
    DEFAULT_LEARNING_CONTEXT_HISTORY, DEFAULT_PROJECT_ACTIVITY_RESULTS,
    DEFAULT_PROJECT_CATALOG_RESULTS, DEFAULT_PROJECT_MEMORY_SEARCH_RESULTS,
    DEFAULT_PROJECT_MEMORY_SEARCH_TOKENS, DEFAULT_RESUME_CHARACTERS, DEFAULT_RESUME_LEARNINGS,
    DEFAULT_RESUME_SESSIONS, DEFAULT_SESSION_CONTEXT_CHARACTERS,
    DEFAULT_SESSION_CONTEXT_CHECKPOINTS, DEFAULT_SESSION_TURN_CHARACTERS,
    DEFAULT_SESSION_TURN_RESULTS, MAX_LEARNING_LIST_RESULTS, MAX_MEDIA_EVIDENCE_BYTES,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentMediaEvidence {
    artifact_path: String,
    artifact_snapshot_id: String,
    content_hash: String,
    media_type: ArtifactMediaType,
    mime_type: String,
    source_bytes: u64,
    data_url: String,
    evidence_role: &'static str,
    source_boundary: &'static str,
    live_source_checked: bool,
    derived_description_included: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum AgentMemoryStorage {
    Native {
        project_id: String,
    },
    LegacyVault {
        project_id: String,
        vault_name: String,
        source: BindingSource,
    },
}

impl From<ProjectVaultBinding> for AgentMemoryStorage {
    fn from(binding: ProjectVaultBinding) -> Self {
        let vault_name = binding
            .vault_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Ley vault")
            .to_owned();
        Self::LegacyVault {
            project_id: binding.project_id,
            vault_name,
            source: binding.source,
        }
    }
}

#[derive(Debug, Clone)]
struct AgentContinuityAccess {
    project_id: String,
    legacy_vault_path: PathBuf,
    storage: AgentMemoryStorage,
}

impl AgentContinuityAccess {
    fn from_legacy_binding(binding: ProjectVaultBinding) -> Self {
        Self {
            project_id: binding.project_id.clone(),
            legacy_vault_path: binding.vault_path.clone(),
            storage: binding.into(),
        }
    }

    fn native(project_id: String, store: &ContinuityStore) -> Result<Self, LeyCoreError> {
        let parent = store.path().parent().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "continuity database has no parent directory for native transition access"
                    .to_owned(),
            )
        })?;
        let legacy_vault_path = parent.join("native-no-legacy-vault").join(&project_id);
        match fs::symlink_metadata(&legacy_vault_path) {
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(source) => {
                return Err(LeyCoreError::Io {
                    path: legacy_vault_path,
                    source,
                })
            }
            Ok(_) => return Err(LeyCoreError::UnsafeProjectLayout(legacy_vault_path)),
        }
        Ok(Self {
            project_id: project_id.clone(),
            legacy_vault_path,
            storage: AgentMemoryStorage::Native { project_id },
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentMemoryDashboard {
    storage: AgentMemoryStorage,
    overview: MemoryOverview,
    resume: ProjectResumePack,
    sessions: Vec<SessionSummary>,
    review_inbox: LearningList,
    all_learnings: LearningList,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentSessionErasure {
    dashboard: AgentMemoryDashboard,
    erasure: SessionMemoryErasure,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentContinuityExport {
    project_id: String,
    destination: PathBuf,
    event_count: usize,
    artifact_snapshots: usize,
    evidence_blobs: usize,
}

const INITIAL_CAPTURE_PREVIEW_PATHS: usize = 8;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentInitialCaptureSkippedPath {
    path: String,
    reason: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentInitialCapturePreview {
    mode: CaptureMode,
    approved_roots: Vec<String>,
    respect_gitignore: bool,
    max_file_bytes: u64,
    max_total_bytes: u64,
    capture_fingerprint: String,
    plan_fingerprint: String,
    approval_fingerprint: String,
    eligible_files: usize,
    eligible_bytes: u64,
    included_paths: Vec<String>,
    omitted_included_paths: usize,
    skipped_oversized: usize,
    skipped_total_limit: usize,
    skipped_symlinks: usize,
    skipped_paths: Vec<AgentInitialCaptureSkippedPath>,
    omitted_skipped_paths: usize,
    exclusion_notice: &'static str,
    privacy_notice: &'static str,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum AgentProjectInspection {
    Uninitialized {
        suggested_name: String,
        preview: AgentInitialCapturePreview,
    },
    Unbound {
        project_id: String,
        project_name: String,
        capture_mode: CaptureMode,
    },
    VaultUnavailable {
        project_id: String,
        project_name: String,
        capture_mode: CaptureMode,
        previous_vault_name: String,
    },
    NeedsCapture {
        project_id: String,
        project_name: String,
        capture_mode: CaptureMode,
        storage: AgentMemoryStorage,
    },
    Ready {
        dashboard: Box<AgentMemoryDashboard>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum AgentProjectCatalogState {
    Ready,
    Unbound,
    NeedsCapture,
    ProjectUnavailable,
    VaultUnavailable,
    IdentityChanged,
    MemoryError,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentProjectCatalogItem {
    project_id: String,
    project_path: PathBuf,
    project_name: String,
    capture_mode: Option<CaptureMode>,
    state: AgentProjectCatalogState,
    last_opened_at_unix_ms: u64,
    vault_name: Option<String>,
    files: Option<usize>,
    graph_nodes: Option<usize>,
    sessions: Option<usize>,
    active_sessions: Option<usize>,
    review_items: Option<usize>,
    freshness: Option<String>,
    status_detail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentProjectCatalogView {
    projects: Vec<AgentProjectCatalogItem>,
    total_projects: usize,
    omitted_projects: usize,
    ready_projects: usize,
    attention_projects: usize,
    privacy_notice: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentCaptureSettings {
    project_id: String,
    project_name: String,
    mode: CaptureMode,
    approved_roots: Vec<String>,
    respect_gitignore: bool,
    max_file_bytes: u64,
    max_total_bytes: u64,
    ignore_file_present: bool,
    capture_fingerprint: String,
    eligible_files: usize,
    eligible_bytes: u64,
    skipped_oversized: usize,
    skipped_total_limit: usize,
    skipped_symlinks: usize,
    privacy_notice: &'static str,
}

fn load_agent_memory_dashboard(
    project_path: &Path,
    binding: ProjectVaultBinding,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    let storage = binding.clone().into();
    let overview = project_memory_overview(project_path, &binding.vault_path)?;
    let resume = project_resume_context(
        project_path,
        &binding.vault_path,
        DEFAULT_RESUME_SESSIONS,
        DEFAULT_RESUME_LEARNINGS,
        DEFAULT_RESUME_CHARACTERS,
    )?;
    let sessions = list_sessions(project_path, &binding.vault_path)?;
    let review_inbox = list_learning_contexts(
        project_path,
        &binding.vault_path,
        LearningListScope::NeedsReview,
        MAX_LEARNING_LIST_RESULTS,
    )?;
    let all_learnings = list_learning_contexts(
        project_path,
        &binding.vault_path,
        LearningListScope::All,
        MAX_LEARNING_LIST_RESULTS,
    )?;
    Ok(AgentMemoryDashboard {
        storage,
        overview,
        resume,
        sessions,
        review_inbox,
        all_learnings,
    })
}

fn load_agent_memory_dashboard_with_continuity_store(
    project_path: &Path,
    binding: ProjectVaultBinding,
    store: &ContinuityStore,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    load_agent_memory_dashboard_with_access(
        project_path,
        &AgentContinuityAccess::from_legacy_binding(binding),
        store,
    )
}

fn load_agent_memory_dashboard_with_access(
    project_path: &Path,
    access: &AgentContinuityAccess,
    store: &ContinuityStore,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    let overview = project_memory_overview_with_continuity_transition(
        project_path,
        &access.legacy_vault_path,
        store,
    )?;
    let resume = project_resume_context_with_continuity_transition(
        project_path,
        &access.legacy_vault_path,
        store,
        DEFAULT_RESUME_SESSIONS,
        DEFAULT_RESUME_LEARNINGS,
        DEFAULT_RESUME_CHARACTERS,
    )?;
    let sessions =
        list_sessions_with_continuity_transition(project_path, &access.legacy_vault_path, store)?;
    let review_inbox = list_learning_contexts_with_continuity_transition(
        project_path,
        &access.legacy_vault_path,
        store,
        LearningListScope::NeedsReview,
        MAX_LEARNING_LIST_RESULTS,
    )?;
    let all_learnings = list_learning_contexts_with_continuity_transition(
        project_path,
        &access.legacy_vault_path,
        store,
        LearningListScope::All,
        MAX_LEARNING_LIST_RESULTS,
    )?;
    Ok(AgentMemoryDashboard {
        storage: access.storage.clone(),
        overview,
        resume,
        sessions,
        review_inbox,
        all_learnings,
    })
}

fn load_agent_memory_dashboard_transition(
    project_path: &Path,
    binding: ProjectVaultBinding,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    let store = ContinuityStore::system_default()?;
    load_agent_memory_dashboard_with_continuity_store(project_path, binding, &store)
}

fn load_native_agent_memory_dashboard(
    project_path: &Path,
    store: &ContinuityStore,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    let project_id = diagnose_project(project_path)?.identity.project_id;
    let access = AgentContinuityAccess::native(project_id, store)?;
    load_agent_memory_dashboard_with_access(project_path, &access, store)
}

fn resolved_agent_binding(project_path: &Path) -> Result<ProjectVaultBinding, LeyCoreError> {
    BindingRegistry::system_default()?.resolve(project_path, None)
}

fn with_transition_agent_access<T>(
    project_path: &Path,
    vault_override: Option<&Path>,
    operation: impl FnOnce(&AgentContinuityAccess, &ContinuityStore) -> Result<T, LeyCoreError>,
) -> Result<T, LeyCoreError> {
    let registry = BindingRegistry::system_default()?;
    let store = ContinuityStore::system_default()?;
    with_transition_agent_access_from(project_path, vault_override, &registry, &store, operation)
}

fn with_transition_agent_access_from<T>(
    project_path: &Path,
    vault_override: Option<&Path>,
    registry: &BindingRegistry,
    store: &ContinuityStore,
    operation: impl FnOnce(&AgentContinuityAccess, &ContinuityStore) -> Result<T, LeyCoreError>,
) -> Result<T, LeyCoreError> {
    match registry.resolve(project_path, vault_override) {
        Ok(binding) => operation(&AgentContinuityAccess::from_legacy_binding(binding), store),
        Err(LeyCoreError::VaultNotBound(project_id)) if vault_override.is_none() => {
            let native_ready =
                ley_core::native_canonical_read_authority_available(project_path, store)?;
            let native_registered = native_born_project_registration_exists(project_path, store)?;
            if !native_ready && !native_registered {
                return Err(LeyCoreError::VaultNotBound(project_id));
            }
            operation(&AgentContinuityAccess::native(project_id, store)?, store)
        }
        Err(LeyCoreError::BoundVaultUnavailable { project_id, path })
            if vault_override.is_none() =>
        {
            let unavailable_path = path.clone();
            let unavailable = LeyCoreError::BoundVaultUnavailable {
                project_id: project_id.clone(),
                path: path.clone(),
            };
            let access = AgentContinuityAccess::from_legacy_binding(ProjectVaultBinding {
                project_id,
                vault_path: path,
                source: BindingSource::Persisted,
            });
            match operation(&access, store) {
                Ok(value) => Ok(value),
                Err(LeyCoreError::Io { path, source })
                    if source.kind() == ErrorKind::NotFound
                        && (path == unavailable_path || path.starts_with(&unavailable_path)) =>
                {
                    Err(unavailable)
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

fn ingest_agent_project_with_access(
    project_path: &Path,
    access: &AgentContinuityAccess,
    store: &ContinuityStore,
) -> Result<IngestionResult, LeyCoreError> {
    match &access.storage {
        AgentMemoryStorage::Native { .. } => {
            let result = ingest_project_with_native_authority(project_path, store)?;
            establish_native_born_project_authorities(project_path, store)?;
            Ok(result)
        }
        AgentMemoryStorage::LegacyVault { .. } => ingest_project_with_continuity_transition(
            project_path,
            &access.legacy_vault_path,
            store,
        ),
    }
}

#[cfg(test)]
fn with_transition_agent_binding_from<T>(
    project_path: &Path,
    vault_override: Option<&Path>,
    registry: &BindingRegistry,
    store: &ContinuityStore,
    operation: impl FnOnce(&ProjectVaultBinding, &ContinuityStore) -> Result<T, LeyCoreError>,
) -> Result<T, LeyCoreError> {
    match registry.resolve(project_path, vault_override) {
        Ok(binding) => operation(&binding, store),
        Err(LeyCoreError::BoundVaultUnavailable { project_id, path })
            if vault_override.is_none() =>
        {
            let unavailable_path = path.clone();
            let unavailable = LeyCoreError::BoundVaultUnavailable {
                project_id: project_id.clone(),
                path: path.clone(),
            };
            let binding = ProjectVaultBinding {
                project_id,
                vault_path: path,
                source: BindingSource::Persisted,
            };
            match operation(&binding, store) {
                Ok(value) => Ok(value),
                Err(LeyCoreError::Io { path, source })
                    if source.kind() == ErrorKind::NotFound
                        && (path == unavailable_path || path.starts_with(&unavailable_path)) =>
                {
                    Err(unavailable)
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

fn with_transition_agent_session_read<T>(
    project_path: &Path,
    operation: impl FnOnce(&Path, &Path, &ContinuityStore) -> Result<T, LeyCoreError>,
) -> Result<T, LeyCoreError> {
    let registry = BindingRegistry::system_default()?;
    let store = ContinuityStore::system_default()?;
    with_transition_agent_session_read_from(project_path, &registry, &store, operation)
}

fn with_transition_agent_session_read_from<T>(
    project_path: &Path,
    registry: &BindingRegistry,
    store: &ContinuityStore,
    operation: impl FnOnce(&Path, &Path, &ContinuityStore) -> Result<T, LeyCoreError>,
) -> Result<T, LeyCoreError> {
    with_transition_agent_access_from(project_path, None, registry, store, |access, store| {
        operation(project_path, &access.legacy_vault_path, store)
    })
}

fn agent_project_catalog_item(
    observed: ley_core::ObservedProject,
    registry: &BindingRegistry,
    transition_store: Option<&ContinuityStore>,
) -> AgentProjectCatalogItem {
    let fallback_name = observed
        .root_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unavailable project")
        .to_owned();
    let base = |state, status_detail| AgentProjectCatalogItem {
        project_id: observed.project_id.clone(),
        project_path: observed.root_path.clone(),
        project_name: fallback_name.clone(),
        capture_mode: None,
        state,
        last_opened_at_unix_ms: observed.last_opened_at_unix_ms,
        vault_name: None,
        files: None,
        graph_nodes: None,
        sessions: None,
        active_sessions: None,
        review_items: None,
        freshness: None,
        status_detail,
    };

    let diagnostic = match diagnose_project(&observed.root_path) {
        Ok(diagnostic) => diagnostic,
        Err(error) => {
            return base(
                AgentProjectCatalogState::ProjectUnavailable,
                error.to_string(),
            )
        }
    };
    if diagnostic.identity.project_id != observed.project_id {
        return base(
            AgentProjectCatalogState::IdentityChanged,
            "This folder now contains a different Ley project identity.".to_owned(),
        );
    }
    agent_project_catalog_item_for_diagnostic(observed, diagnostic, registry, transition_store)
}

fn agent_project_catalog_item_for_diagnostic(
    observed: ley_core::ObservedProject,
    diagnostic: ProjectDiagnostic,
    registry: &BindingRegistry,
    transition_store: Option<&ContinuityStore>,
) -> AgentProjectCatalogItem {
    let mut item = AgentProjectCatalogItem {
        project_id: observed.project_id,
        project_path: observed.root_path,
        project_name: diagnostic.identity.name.clone(),
        capture_mode: Some(diagnostic.capture.mode),
        state: AgentProjectCatalogState::Ready,
        last_opened_at_unix_ms: observed.last_opened_at_unix_ms,
        vault_name: None,
        files: None,
        graph_nodes: None,
        sessions: None,
        active_sessions: None,
        review_items: None,
        freshness: None,
        status_detail: "Ready to resume locally.".to_owned(),
    };
    let access = match registry.resolve_observed(&diagnostic) {
        Ok(binding) => {
            item.vault_name = Some(
                binding
                    .vault_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Ley vault")
                    .to_owned(),
            );
            AgentContinuityAccess::from_legacy_binding(binding)
        }
        Err(LeyCoreError::VaultNotBound(_)) => {
            let Some(store) = transition_store else {
                item.state = AgentProjectCatalogState::Unbound;
                item.status_detail =
                    "Legacy project requires migration before local continuity is ready."
                        .to_owned();
                return item;
            };
            match ley_core::native_canonical_read_authority_available(&diagnostic.root, store) {
                Ok(true) => match AgentContinuityAccess::native(
                    diagnostic.identity.project_id.clone(),
                    store,
                ) {
                    Ok(access) => access,
                    Err(error) => {
                        item.state = AgentProjectCatalogState::MemoryError;
                        item.status_detail = error.to_string();
                        return item;
                    }
                },
                Ok(false) => {
                    match native_born_project_registration_exists(&diagnostic.root, store) {
                        Ok(true) => {
                            item.state = AgentProjectCatalogState::NeedsCapture;
                            item.status_detail =
                                "Native continuity memory was erased; review capture to enable it again."
                                    .to_owned();
                        }
                        Ok(false) => {
                            item.state = AgentProjectCatalogState::Unbound;
                            item.status_detail =
                                "Legacy project requires migration before local continuity is ready."
                                    .to_owned();
                        }
                        Err(error) => {
                            item.state = AgentProjectCatalogState::MemoryError;
                            item.status_detail = error.to_string();
                        }
                    }
                    return item;
                }
                Err(error) => {
                    item.state = AgentProjectCatalogState::MemoryError;
                    item.status_detail = error.to_string();
                    return item;
                }
            }
        }
        Err(LeyCoreError::BoundVaultUnavailable { project_id, path }) => {
            item.vault_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned);
            let native_ready = transition_store
                .map(|store| {
                    ley_core::native_canonical_read_authority_available(&diagnostic.root, store)
                })
                .transpose();
            match native_ready {
                Ok(Some(true)) => AgentContinuityAccess::from_legacy_binding(ProjectVaultBinding {
                    project_id,
                    vault_path: path,
                    source: BindingSource::Persisted,
                }),
                Ok(_) => {
                    item.state = AgentProjectCatalogState::VaultUnavailable;
                    item.status_detail = "The legacy vault is unavailable and this project has not completed native continuity migration."
                        .to_owned();
                    return item;
                }
                Err(error) => {
                    item.state = AgentProjectCatalogState::MemoryError;
                    item.status_detail = error.to_string();
                    return item;
                }
            }
        }
        Err(error) => {
            item.state = AgentProjectCatalogState::MemoryError;
            item.status_detail = error.to_string();
            return item;
        }
    };
    let overview_result = match transition_store {
        Some(store) => project_memory_overview_with_continuity_transition(
            &diagnostic.root,
            &access.legacy_vault_path,
            store,
        ),
        None => project_memory_overview(&diagnostic.root, &access.legacy_vault_path),
    };
    let overview = match overview_result {
        Ok(overview) => overview,
        Err(LeyCoreError::ProjectMemoryUnavailable(_)) => {
            item.state = AgentProjectCatalogState::NeedsCapture;
            item.status_detail = "Connected locally; create the first project snapshot.".to_owned();
            return item;
        }
        Err(error) => {
            item.state = AgentProjectCatalogState::MemoryError;
            item.status_detail = error.to_string();
            return item;
        }
    };
    let sessions = match transition_store {
        Some(store) => project_session_stats_with_continuity_transition(
            &diagnostic.root,
            &access.legacy_vault_path,
            store,
        ),
        None => project_session_stats(&diagnostic.root, &access.legacy_vault_path),
    };
    let sessions = match sessions {
        Ok(sessions) => sessions,
        Err(error) => {
            item.state = AgentProjectCatalogState::MemoryError;
            item.status_detail = error.to_string();
            return item;
        }
    };
    let review = match transition_store {
        Some(store) => list_learning_contexts_with_continuity_transition(
            &diagnostic.root,
            &access.legacy_vault_path,
            store,
            LearningListScope::NeedsReview,
            MAX_LEARNING_LIST_RESULTS,
        ),
        None => list_learning_contexts(
            &diagnostic.root,
            &access.legacy_vault_path,
            LearningListScope::NeedsReview,
            MAX_LEARNING_LIST_RESULTS,
        ),
    };
    let review = match review {
        Ok(review) => review,
        Err(error) => {
            item.state = AgentProjectCatalogState::MemoryError;
            item.status_detail = error.to_string();
            return item;
        }
    };
    item.files = Some(overview.files);
    item.graph_nodes = overview.graph_nodes;
    item.sessions = Some(sessions.total_sessions);
    item.active_sessions = Some(sessions.active_sessions + sessions.paused_sessions);
    item.review_items = Some(review.total_matching);
    item.freshness = Some(overview.freshness.to_owned());
    item
}

fn load_agent_project_catalog_from(
    catalog: &ProjectCatalog,
    registry: &BindingRegistry,
    transition_store: Option<&ContinuityStore>,
) -> Result<AgentProjectCatalogView, LeyCoreError> {
    let observed = catalog.list(DEFAULT_PROJECT_CATALOG_RESULTS)?;
    let projects = observed
        .projects
        .into_iter()
        .map(|project| agent_project_catalog_item(project, registry, transition_store))
        .collect::<Vec<_>>();
    let ready_projects = projects
        .iter()
        .filter(|project| matches!(project.state, AgentProjectCatalogState::Ready))
        .count();
    Ok(AgentProjectCatalogView {
        attention_projects: projects.len().saturating_sub(ready_projects),
        projects,
        total_projects: observed.total_projects,
        omitted_projects: observed.omitted_projects,
        ready_projects,
        privacy_notice: "Ley lists only initialized projects you explicitly opened on this device. It never scans neighboring folders.",
    })
}

fn load_agent_project_catalog() -> Result<AgentProjectCatalogView, LeyCoreError> {
    let store = ContinuityStore::system_default()?;
    load_agent_project_catalog_from(
        &ProjectCatalog::system_default()?,
        &BindingRegistry::system_default()?,
        Some(&store),
    )
}

#[tauri::command]
fn list_agent_projects(
    legacy_project_path: Option<String>,
) -> Result<AgentProjectCatalogView, String> {
    if let Some(path) = legacy_project_path {
        match ProjectCatalog::system_default().and_then(|catalog| catalog.observe(path)) {
            Ok(_) | Err(LeyCoreError::ProjectNotFound(_) | LeyCoreError::NotDirectory(_)) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    load_agent_project_catalog().map_err(|error| error.to_string())
}

#[tauri::command]
fn forget_agent_project(project_id: String) -> Result<AgentProjectCatalogView, String> {
    ProjectCatalog::system_default()
        .and_then(|catalog| catalog.forget(&project_id))
        .and_then(|_| load_agent_project_catalog())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_capture_settings(project_path: String) -> Result<AgentCaptureSettings, String> {
    (|| -> Result<AgentCaptureSettings, LeyCoreError> {
        let diagnostic = diagnose_project(&project_path)?;
        let preview = ley_core::preview_capture(&project_path)?;
        Ok(AgentCaptureSettings {
            project_id: diagnostic.identity.project_id,
            project_name: diagnostic.identity.name,
            mode: diagnostic.capture.mode,
            approved_roots: diagnostic.capture.approved_roots,
            respect_gitignore: diagnostic.capture.respect_gitignore,
            max_file_bytes: diagnostic.capture.max_file_bytes,
            max_total_bytes: diagnostic.capture.max_total_bytes,
            ignore_file_present: diagnostic.ignore_file_present,
            capture_fingerprint: preview.capture_fingerprint,
            eligible_files: preview.files.len(),
            eligible_bytes: preview.included_bytes,
            skipped_oversized: preview.skipped_oversized.len(),
            skipped_total_limit: preview.skipped_total_limit.len(),
            skipped_symlinks: preview.skipped_symlinks.len(),
            privacy_notice: "Preview reads file metadata only. Applying a mode refreshes the redacted local snapshot; Ley never uploads it.",
        })
    })()
        .map_err(|error| error.to_string())
}

fn read_agent_egress_policy_with(
    project_path: &Path,
    registry: &EgressPolicyRegistry,
    store: &ContinuityStore,
) -> Result<ProjectAgentEgressPolicy, LeyCoreError> {
    registry.list_transition(project_path, store)
}

#[tauri::command]
fn read_agent_egress_policy(project_path: String) -> Result<ProjectAgentEgressPolicy, String> {
    let registry = EgressPolicyRegistry::system_default().map_err(|error| error.to_string())?;
    let store = ContinuityStore::system_default().map_err(|error| error.to_string())?;
    read_agent_egress_policy_with(Path::new(&project_path), &registry, &store)
        .map_err(|error| error.to_string())
}

fn update_agent_egress_policy_with(
    project_path: &Path,
    registry: &EgressPolicyRegistry,
    store: &ContinuityStore,
    expected_project_id: &str,
    expected_policy: AgentEgressPolicy,
    policy: AgentEgressPolicy,
) -> Result<ProjectAgentEgressPolicy, LeyCoreError> {
    registry.set_project_policy_transition_if_current(
        project_path,
        store,
        expected_project_id,
        expected_policy,
        policy,
    )?;
    registry.list_transition(project_path, store)
}

#[tauri::command]
fn update_agent_egress_policy(
    project_path: String,
    expected_project_id: String,
    expected_policy: AgentEgressPolicy,
    policy: AgentEgressPolicy,
) -> Result<ProjectAgentEgressPolicy, String> {
    let registry = EgressPolicyRegistry::system_default().map_err(|error| error.to_string())?;
    let store = ContinuityStore::system_default().map_err(|error| error.to_string())?;
    update_agent_egress_policy_with(
        Path::new(&project_path),
        &registry,
        &store,
        &expected_project_id,
        expected_policy,
        policy,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_agent_capture_mode(
    project_path: String,
    expected_mode: CaptureMode,
    mode: CaptureMode,
    full_evidence_consent: bool,
) -> Result<AgentMemoryDashboard, String> {
    with_transition_agent_access(Path::new(&project_path), None, |access, store| {
        update_capture_mode(
            &project_path,
            &access.project_id,
            expected_mode,
            mode,
            full_evidence_consent,
        )?;
        ingest_agent_project_with_access(Path::new(&project_path), access, store)?;
        load_agent_memory_dashboard_with_access(Path::new(&project_path), access, store)
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn erase_agent_project_memory(project_path: String) -> Result<AgentProjectInspection, String> {
    let registry = BindingRegistry::system_default().map_err(|error| error.to_string())?;
    let store = ContinuityStore::system_default().map_err(|error| error.to_string())?;
    erase_agent_project_memory_with_registry_and_store(Path::new(&project_path), &registry, &store)
}

#[cfg(test)]
fn erase_agent_project_memory_with_registry(
    project_path: &Path,
    registry: &BindingRegistry,
) -> Result<AgentProjectInspection, String> {
    let binding = registry
        .resolve(project_path, None)
        .map_err(|error| error.to_string())?;
    erase_project_memory(project_path, &binding.vault_path).map_err(|error| error.to_string())?;
    let diagnostic = diagnose_project(project_path).map_err(|error| error.to_string())?;
    inspect_initialized_agent_project_with_registry(diagnostic, registry)
}

fn erase_agent_project_memory_with_registry_and_store(
    project_path: &Path,
    registry: &BindingRegistry,
    store: &ContinuityStore,
) -> Result<AgentProjectInspection, String> {
    with_transition_agent_access_from(project_path, None, registry, store, |access, store| {
        match &access.storage {
            AgentMemoryStorage::Native { .. } => {
                erase_project_memory_with_native_authority(project_path, store).map(|_| ())
            }
            AgentMemoryStorage::LegacyVault { .. } => {
                erase_project_memory_with_continuity_transition(
                    project_path,
                    &access.legacy_vault_path,
                    store,
                )
                .map(|_| ())
            }
        }
    })
    .map_err(|error| error.to_string())?;
    let diagnostic = diagnose_project(project_path).map_err(|error| error.to_string())?;
    inspect_initialized_agent_project_with_registry_and_store(diagnostic, registry, Some(store))
}

#[tauri::command]
async fn search_agent_projects(query: String) -> Result<CrossProjectSearch, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let catalog = ProjectCatalog::system_default()?;
        let registry = BindingRegistry::system_default()?;
        search_observed_projects(
            &catalog,
            &registry,
            &query,
            DEFAULT_CROSS_PROJECT_SEARCH_RESULTS,
        )
    })
    .await
    .map_err(|_| "Cross-project search was interrupted.".to_owned())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn search_agent_project_memory(
    project_path: String,
    query: String,
    revision_filter: Option<RevisionCompatibility>,
) -> Result<ProjectMemorySearch, String> {
    tauri::async_runtime::spawn_blocking(move || {
        with_transition_agent_access(Path::new(&project_path), None, |access, store| {
            search_project_memory_with_continuity_transition(
                &project_path,
                &access.legacy_vault_path,
                store,
                &query,
                ProjectMemorySearchLimits {
                    max_results: DEFAULT_PROJECT_MEMORY_SEARCH_RESULTS,
                    max_tokens: DEFAULT_PROJECT_MEMORY_SEARCH_TOKENS,
                },
                revision_filter,
            )
        })
    })
    .await
    .map_err(|_| "Project memory search was interrupted.".to_owned())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn inspect_agent_project(project_path: String) -> Result<AgentProjectInspection, String> {
    let path = Path::new(&project_path);
    let diagnostic = match diagnose_project(path) {
        Ok(diagnostic) => diagnostic,
        Err(LeyCoreError::ProjectNotFound(_)) => {
            let suggested_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("New project")
                .to_owned();
            let preview = agent_initial_capture_preview(path).map_err(|error| error.to_string())?;
            return Ok(AgentProjectInspection::Uninitialized {
                suggested_name,
                preview,
            });
        }
        Err(error) => return Err(error.to_string()),
    };
    let registry = BindingRegistry::system_default().map_err(|error| error.to_string())?;
    let store = ContinuityStore::system_default().map_err(|error| error.to_string())?;
    inspect_initialized_agent_project_with_registry_and_store(diagnostic, &registry, Some(&store))
}

#[cfg(test)]
fn inspect_initialized_agent_project_with_registry(
    diagnostic: ProjectDiagnostic,
    registry: &BindingRegistry,
) -> Result<AgentProjectInspection, String> {
    inspect_initialized_agent_project_with_registry_and_store(diagnostic, registry, None)
}

fn inspect_initialized_agent_project_with_registry_and_store(
    diagnostic: ProjectDiagnostic,
    registry: &BindingRegistry,
    transition_store: Option<&ContinuityStore>,
) -> Result<AgentProjectInspection, String> {
    let binding = match registry.resolve(&diagnostic.root, None) {
        Ok(binding) => binding,
        Err(LeyCoreError::VaultNotBound(_)) => {
            if let Some(store) = transition_store {
                if ley_core::native_canonical_read_authority_available(&diagnostic.root, store)
                    .map_err(|error| error.to_string())?
                {
                    let dashboard = load_native_agent_memory_dashboard(&diagnostic.root, store)
                        .map_err(|error| error.to_string())?;
                    return Ok(AgentProjectInspection::Ready {
                        dashboard: Box::new(dashboard),
                    });
                }
                if native_born_project_registration_exists(&diagnostic.root, store)
                    .map_err(|error| error.to_string())?
                {
                    return Ok(AgentProjectInspection::NeedsCapture {
                        project_id: diagnostic.identity.project_id.clone(),
                        project_name: diagnostic.identity.name.clone(),
                        capture_mode: diagnostic.capture.mode,
                        storage: AgentMemoryStorage::Native {
                            project_id: diagnostic.identity.project_id,
                        },
                    });
                }
            }
            return Ok(AgentProjectInspection::Unbound {
                project_id: diagnostic.identity.project_id,
                project_name: diagnostic.identity.name,
                capture_mode: diagnostic.capture.mode,
            });
        }
        Err(LeyCoreError::BoundVaultUnavailable { project_id, path }) => {
            if let Some(store) = transition_store {
                if ley_core::native_canonical_read_authority_available(&diagnostic.root, store)
                    .map_err(|error| error.to_string())?
                {
                    let access = AgentContinuityAccess::from_legacy_binding(ProjectVaultBinding {
                        project_id,
                        vault_path: path.clone(),
                        source: BindingSource::Persisted,
                    });
                    let dashboard =
                        load_agent_memory_dashboard_with_access(&diagnostic.root, &access, store)
                            .map_err(|error| error.to_string())?;
                    return Ok(AgentProjectInspection::Ready {
                        dashboard: Box::new(dashboard),
                    });
                }
            }
            return Ok(AgentProjectInspection::VaultUnavailable {
                project_id: diagnostic.identity.project_id,
                project_name: diagnostic.identity.name,
                capture_mode: diagnostic.capture.mode,
                previous_vault_name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("previous vault")
                    .to_owned(),
            });
        }
        Err(error) => return Err(error.to_string()),
    };
    let dashboard = match transition_store {
        Some(store) => load_agent_memory_dashboard_with_continuity_store(
            &diagnostic.root,
            binding.clone(),
            store,
        ),
        None => load_agent_memory_dashboard(&diagnostic.root, binding.clone()),
    };
    match dashboard {
        Ok(dashboard) => Ok(AgentProjectInspection::Ready {
            dashboard: Box::new(dashboard),
        }),
        Err(LeyCoreError::ProjectMemoryUnavailable(_)) => {
            Ok(AgentProjectInspection::NeedsCapture {
                project_id: diagnostic.identity.project_id,
                project_name: diagnostic.identity.name,
                capture_mode: diagnostic.capture.mode,
                storage: binding.into(),
            })
        }
        Err(error) => Err(error.to_string()),
    }
}

fn agent_initial_capture_preview(
    project_path: &Path,
) -> Result<AgentInitialCapturePreview, LeyCoreError> {
    let preview = preview_initial_capture(project_path, CaptureMode::Structured)?;
    let policy = CapturePolicy::for_mode(CaptureMode::Structured);
    agent_capture_preview_summary(
        &preview.root,
        &policy,
        preview.mode,
        preview.capture_fingerprint,
        preview.plan_fingerprint,
        preview.files,
        preview.included_bytes,
        preview.skipped_oversized,
        preview.skipped_total_limit,
        preview.skipped_symlinks,
        "This preview inspects capture paths and file metadata only. It creates no .ley metadata or Agent Memory until you approve initialization.",
    )
}

#[allow(clippy::too_many_arguments)]
fn agent_capture_preview_summary(
    root: &Path,
    policy: &CapturePolicy,
    mode: CaptureMode,
    capture_fingerprint: String,
    plan_fingerprint: String,
    files: Vec<CaptureFile>,
    included_bytes: u64,
    skipped_oversized: Vec<CaptureFile>,
    skipped_total_limit: Vec<CaptureFile>,
    skipped_symlinks: Vec<String>,
    privacy_notice: &'static str,
) -> Result<AgentInitialCapturePreview, LeyCoreError> {
    let approval_fingerprint = agent_capture_approval_fingerprint(root, &plan_fingerprint)?;
    let included_paths = files
        .iter()
        .take(INITIAL_CAPTURE_PREVIEW_PATHS)
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    let mut skipped_paths = skipped_oversized
        .iter()
        .map(|file| AgentInitialCaptureSkippedPath {
            path: file.path.clone(),
            reason: "oversized",
        })
        .chain(
            skipped_total_limit
                .iter()
                .map(|file| AgentInitialCaptureSkippedPath {
                    path: file.path.clone(),
                    reason: "total-limit",
                }),
        )
        .chain(
            skipped_symlinks
                .iter()
                .map(|path| AgentInitialCaptureSkippedPath {
                    path: path.clone(),
                    reason: "symlink",
                }),
        )
        .collect::<Vec<_>>();
    let total_skipped_paths = skipped_paths.len();
    skipped_paths.truncate(INITIAL_CAPTURE_PREVIEW_PATHS);
    Ok(AgentInitialCapturePreview {
        mode,
        approved_roots: policy.approved_roots.clone(),
        respect_gitignore: policy.respect_gitignore,
        max_file_bytes: policy.max_file_bytes,
        max_total_bytes: policy.max_total_bytes,
        capture_fingerprint,
        plan_fingerprint,
        approval_fingerprint,
        eligible_files: files.len(),
        eligible_bytes: included_bytes,
        omitted_included_paths: files.len().saturating_sub(included_paths.len()),
        included_paths,
        skipped_oversized: skipped_oversized.len(),
        skipped_total_limit: skipped_total_limit.len(),
        skipped_symlinks: skipped_symlinks.len(),
        omitted_skipped_paths: total_skipped_paths.saturating_sub(skipped_paths.len()),
        skipped_paths,
        exclusion_notice: "Ley respects .gitignore and its local defaults exclude .ley, .git, dependency/build output folders, dotenv files, private keys, package-manager credentials, and credentials.json.",
        privacy_notice,
    })
}

fn agent_capture_approval_fingerprint(
    root: &Path,
    plan_fingerprint: &str,
) -> Result<String, LeyCoreError> {
    let canonical_root = root
        .to_str()
        .ok_or_else(|| LeyCoreError::NonUtf8Path(root.to_path_buf()))?;
    let mut digest = Sha256::new();
    digest.update(b"ley:desktop-capture-approval:v1\0");
    digest.update(canonical_root.as_bytes());
    digest.update([0]);
    digest.update(plan_fingerprint.as_bytes());
    Ok(format!("sha256:{:x}", digest.finalize()))
}

fn native_approved_source_registry(project_path: &Path) -> Result<ApprovedSourceRegistry, String> {
    let registry = ApprovedSourceRegistry::system_default().map_err(|error| error.to_string())?;
    if !registry
        .authority_ready(project_path)
        .map_err(|error| error.to_string())?
    {
        let binding = resolved_agent_binding(project_path).map_err(|error| error.to_string())?;
        let legacy = SpecificationRegistry::system_default().map_err(|error| error.to_string())?;
        registry
            .migrate_legacy_specifications(project_path, &binding.vault_path, &legacy)
            .map_err(|error| error.to_string())?;
    }
    Ok(registry)
}

fn read_project_approved_sources(
    project_path: &Path,
) -> Result<ApprovedSourceAuthorityList, String> {
    native_approved_source_registry(project_path)?
        .authority(project_path)
        .map_err(|error| error.to_string())
}

fn approved_markdown_path_for_external_editor(
    project_path: &Path,
    source_id: &str,
    registry: &ApprovedSourceRegistry,
) -> Result<PathBuf, LeyCoreError> {
    let content = registry.read(project_path, source_id)?;
    if content.approval.source_kind != ApprovedSourceKind::ProjectFile {
        return Err(LeyCoreError::InvalidApprovedSourceRequest(
            "only current project-file approved sources can be opened externally".to_owned(),
        ));
    }
    let relative_path = content
        .approval
        .project_relative_path
        .as_deref()
        .ok_or_else(|| {
            LeyCoreError::InvalidApprovedSourceRequest(
                "approved project-file source has no project-relative path".to_owned(),
            )
        })?;
    let markdown = Path::new(relative_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("mdx")
        });
    if !markdown {
        return Err(LeyCoreError::InvalidApprovedSourceRequest(
            "open-in-editor is limited to approved Markdown project files".to_owned(),
        ));
    }
    let diagnostic = diagnose_project(project_path)?;
    if content.approval.project_id != diagnostic.identity.project_id {
        return Err(LeyCoreError::InvalidProjectIdentity(
            "approved source project identity changed before external open".to_owned(),
        ));
    }
    revalidate_regular_project_path_no_symlinks(&diagnostic.root, relative_path)
}

fn revalidate_regular_project_path_no_symlinks(
    project_root: &Path,
    relative_path: &str,
) -> Result<PathBuf, LeyCoreError> {
    let mut current = project_root.to_path_buf();
    let mut components = Path::new(relative_path).components().peekable();
    while let Some(component) = components.next() {
        let Component::Normal(segment) = component else {
            return Err(LeyCoreError::InvalidApprovedSourceRequest(
                "approved source path is not a safe project-relative path".to_owned(),
            ));
        };
        current.push(segment);
        let metadata = fs::symlink_metadata(&current).map_err(|source| LeyCoreError::Io {
            path: current.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(LeyCoreError::UnsafeProjectLayout(current));
        }
        if components.peek().is_some() {
            if !metadata.is_dir() {
                return Err(LeyCoreError::UnsafeProjectLayout(current));
            }
        } else if !metadata.is_file() {
            return Err(LeyCoreError::UnsafeProjectLayout(current));
        }
    }
    if current == project_root {
        return Err(LeyCoreError::InvalidApprovedSourceRequest(
            "approved source path must name a project file".to_owned(),
        ));
    }
    Ok(current)
}

fn canonical_export_parent(path: &Path) -> Result<PathBuf, LeyCoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LeyCoreError::UnsafeProjectLayout(path.to_path_buf()));
    }
    path.canonicalize().map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn export_agent_project_continuity_with_access(
    project_path: &Path,
    destination_parent: &Path,
    access: &AgentContinuityAccess,
    store: &ContinuityStore,
) -> Result<AgentContinuityExport, LeyCoreError> {
    let diagnostic = diagnose_project(project_path)?;
    if diagnostic.identity.project_id != access.project_id {
        return Err(LeyCoreError::InvalidProjectIdentity(
            "project identity changed before continuity export".to_owned(),
        ));
    }
    let destination_parent = canonical_export_parent(destination_parent)?;
    if destination_parent.starts_with(&diagnostic.root) {
        return Err(LeyCoreError::InvalidPortableContinuityBundle(
            "export destination must be outside the project so private continuity is not recaptured or committed with source".to_owned(),
        ));
    }
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let destination = destination_parent.join(format!(
        "ley-continuity-{}-{}-{unique}",
        access.project_id,
        std::process::id()
    ));
    let manifest = export_portable_continuity(
        store,
        &access.legacy_vault_path,
        &access.project_id,
        &destination,
    )?;
    Ok(AgentContinuityExport {
        project_id: access.project_id.clone(),
        destination,
        event_count: manifest.event_count,
        artifact_snapshots: manifest.artifact_snapshots.len(),
        evidence_blobs: manifest.evidence_blobs.len(),
    })
}

#[tauri::command]
fn read_agent_project_approved_sources(
    project_path: String,
) -> Result<ApprovedSourceAuthorityList, String> {
    read_project_approved_sources(Path::new(&project_path))
}

#[tauri::command]
fn open_agent_project_markdown_source(
    project_path: String,
    source_id: String,
) -> Result<(), String> {
    let project_path = Path::new(&project_path);
    let registry = native_approved_source_registry(project_path)?;
    let path = approved_markdown_path_for_external_editor(project_path, &source_id, &registry)
        .map_err(|error| error.to_string())?;
    open::that_detached(&path)
        .map_err(|error| format!("could not open approved Markdown source externally: {error}"))
}

#[tauri::command]
fn export_agent_project_continuity(
    project_path: String,
    destination_parent: String,
) -> Result<AgentContinuityExport, String> {
    with_transition_agent_access(Path::new(&project_path), None, |access, store| {
        export_agent_project_continuity_with_access(
            Path::new(&project_path),
            Path::new(&destination_parent),
            access,
            store,
        )
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn approve_agent_project_file_source(
    project_path: String,
    relative_path: String,
) -> Result<ApprovedSourceAuthorityList, String> {
    let project_path = Path::new(&project_path);
    let registry = native_approved_source_registry(project_path)?;
    registry
        .approve_project_file(project_path, &relative_path)
        .map_err(|error| error.to_string())?;
    registry
        .authority(project_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn reapprove_agent_project_file_source(
    project_path: String,
    source_id: String,
) -> Result<ApprovedSourceAuthorityList, String> {
    let project_path = Path::new(&project_path);
    let registry = native_approved_source_registry(project_path)?;
    registry
        .reapprove_project_file(project_path, &source_id)
        .map_err(|error| error.to_string())?;
    registry
        .authority(project_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn revoke_agent_project_approved_source(
    project_path: String,
    source_id: String,
) -> Result<ApprovedSourceAuthorityList, String> {
    let project_path = Path::new(&project_path);
    let registry = native_approved_source_registry(project_path)?;
    registry
        .revoke(project_path, &source_id)
        .map_err(|error| error.to_string())?;
    registry
        .authority(project_path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn initialize_agent_project(
    project_path: String,
    expected_approval_fingerprint: String,
) -> Result<AgentMemoryDashboard, String> {
    let store = ContinuityStore::system_default().map_err(|error| error.to_string())?;
    let catalog = ProjectCatalog::system_default().map_err(|error| error.to_string())?;
    initialize_agent_project_with_store_and_catalog(
        Path::new(&project_path),
        &expected_approval_fingerprint,
        &store,
        &catalog,
    )
    .map_err(|error| error.to_string())
}

fn initialize_agent_project_with_store_and_catalog(
    project_path: &Path,
    expected_approval_fingerprint: &str,
    store: &ContinuityStore,
    catalog: &ProjectCatalog,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    let reviewed = agent_initial_capture_preview(project_path)?;
    if reviewed.approval_fingerprint != expected_approval_fingerprint {
        return Err(LeyCoreError::CapturePreviewChanged);
    }
    let initialization =
        initialize_project_retiring_bootstrap(project_path, None, CaptureMode::Structured)?;
    if !initialization.created {
        return Err(LeyCoreError::CapturePreviewChanged);
    }
    catalog.observe(&initialization.root)?;
    register_native_born_project(&initialization.root, store)?;
    ingest_project_with_expected_capture_plan_and_native_authority(
        &initialization.root,
        &reviewed.plan_fingerprint,
        store,
    )?;
    establish_native_born_project_authorities(&initialization.root, store)?;
    load_native_agent_memory_dashboard(&initialization.root, store)
}

#[tauri::command]
fn connect_agent_project(
    project_path: String,
    vault_path: String,
) -> Result<AgentMemoryDashboard, String> {
    let registry = BindingRegistry::system_default().map_err(|error| error.to_string())?;
    let store = ContinuityStore::system_default().map_err(|error| error.to_string())?;
    let binding = connect_agent_project_binding_with_registry(
        Path::new(&project_path),
        Path::new(&vault_path),
        &registry,
        &store,
    )
    .map_err(|error| error.to_string())?;
    load_agent_memory_dashboard_transition(Path::new(&project_path), binding)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
fn connect_agent_project_with_registry(
    project_path: &Path,
    vault_path: &Path,
    registry: &BindingRegistry,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    let continuity_dir = registry
        .path()
        .parent()
        .ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "test binding registry has no private parent directory".to_owned(),
            )
        })?
        .join("continuity-private");
    fs::create_dir_all(&continuity_dir).map_err(|source| LeyCoreError::Io {
        path: continuity_dir.clone(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&continuity_dir, fs::Permissions::from_mode(0o700)).map_err(
            |source| LeyCoreError::Io {
                path: continuity_dir.clone(),
                source,
            },
        )?;
    }
    let store = ContinuityStore::at(continuity_dir.join("continuity.sqlite3"));
    let binding =
        connect_agent_project_binding_with_registry(project_path, vault_path, registry, &store)?;
    load_agent_memory_dashboard(project_path, binding)
}

fn connect_agent_project_binding_with_registry(
    project_path: &Path,
    vault_path: &Path,
    registry: &BindingRegistry,
    store: &ContinuityStore,
) -> Result<ProjectVaultBinding, LeyCoreError> {
    let diagnostic = diagnose_project(project_path)?;
    match registry.resolve_observed(&diagnostic) {
        Err(LeyCoreError::VaultNotBound(_)) => {
            prepare_legacy_project_binding(&diagnostic.root, store)?;
            validate_project_memory(&diagnostic.root, vault_path).map_err(|_| {
                LeyCoreError::InvalidContinuityStore(
                    "selected legacy vault cannot be verified for this project; choose the original or moved Ley vault containing this project's captured memory"
                        .to_owned(),
                )
            })?;
            ingest_project_with_continuity_transition(&diagnostic.root, vault_path, store)?;
        }
        Err(LeyCoreError::BoundVaultUnavailable { .. }) => {
            validate_project_memory(&diagnostic.root, vault_path).map_err(|_| {
                LeyCoreError::InvalidContinuityStore(
                    "selected legacy vault cannot be verified for this project; choose the original or moved Ley vault containing this project's captured memory"
                        .to_owned(),
                )
            })?;
            ingest_project_with_continuity_transition(&diagnostic.root, vault_path, store)?;
        }
        Ok(_) => {
            return Err(LeyCoreError::InvalidBindingRequest(
                "project is already bound to an available legacy vault; refresh it instead of reconnecting"
                    .to_owned(),
            ));
        }
        Err(error) => return Err(error),
    }
    registry.bind(&diagnostic.root, vault_path)
}

#[tauri::command]
fn refresh_agent_project(project_path: String) -> Result<AgentMemoryDashboard, String> {
    with_transition_agent_access(Path::new(&project_path), None, |access, store| {
        ingest_agent_project_with_access(Path::new(&project_path), access, store)?;
        load_agent_memory_dashboard_with_access(Path::new(&project_path), access, store)
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_learning(
    project_path: String,
    learning_id: String,
) -> Result<LearningContextPack, String> {
    with_transition_agent_access(Path::new(&project_path), None, |access, store| {
        read_learning_context_with_continuity_transition(
            &project_path,
            &access.legacy_vault_path,
            store,
            &learning_id,
            DEFAULT_LEARNING_CONTEXT_EVIDENCE,
            DEFAULT_LEARNING_CONTEXT_HISTORY,
            DEFAULT_LEARNING_CONTEXT_ARTIFACTS,
            DEFAULT_LEARNING_CONTEXT_CHARACTERS,
        )
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_session(
    project_path: String,
    session_id: String,
) -> Result<SessionContextPack, String> {
    with_transition_agent_session_read(Path::new(&project_path), |project, vault, store| {
        read_session_context_with_continuity_transition(
            project,
            vault,
            store,
            &session_id,
            DEFAULT_SESSION_CONTEXT_CHECKPOINTS,
            DEFAULT_SESSION_CONTEXT_CHARACTERS,
        )
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_session_turns(
    project_path: String,
    session_id: String,
) -> Result<SessionTurnsContextPack, String> {
    with_transition_agent_session_read(Path::new(&project_path), |project, vault, store| {
        read_session_turns_context_with_continuity_transition(
            project,
            vault,
            store,
            &session_id,
            DEFAULT_SESSION_TURN_RESULTS,
            DEFAULT_SESSION_TURN_CHARACTERS,
        )
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn rename_agent_session(
    project_path: String,
    session_id: String,
    expected_event_count: u64,
    name: String,
    note: String,
) -> Result<AgentMemoryDashboard, String> {
    if note.trim().is_empty() {
        return Err("A rename reason is required.".to_owned());
    }
    with_transition_agent_access(Path::new(&project_path), None, |access, store| {
        rename_session_with_continuity_transition(
            &project_path,
            &access.legacy_vault_path,
            store,
            &session_id,
            RenameSessionInput {
                request_id: generate_request_id(),
                expected_event_count: Some(expected_event_count),
                name,
                note,
            },
        )?;
        load_agent_memory_dashboard_with_access(Path::new(&project_path), access, store)
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn erase_agent_session(
    project_path: String,
    session_id: String,
    expected_event_count: u64,
    expected_name: String,
) -> Result<AgentSessionErasure, String> {
    with_transition_agent_access(Path::new(&project_path), None, |access, store| {
        let erasure = erase_session_memory_with_continuity_transition(
            &project_path,
            &access.legacy_vault_path,
            store,
            &session_id,
            EraseSessionMemoryInput {
                expected_event_count,
                expected_name,
            },
        )?;
        let dashboard =
            load_agent_memory_dashboard_with_access(Path::new(&project_path), access, store)?;
        Ok(AgentSessionErasure { dashboard, erasure })
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn review_agent_learning(
    project_path: String,
    learning_id: String,
    expected_event_count: u64,
    action: LearningFeedbackAction,
    note: String,
) -> Result<AgentMemoryDashboard, String> {
    with_transition_agent_access(Path::new(&project_path), None, |access, store| {
        review_agent_learning_with_access_and_store(
            Path::new(&project_path),
            access,
            store,
            &learning_id,
            expected_event_count,
            action,
            note,
        )
    })
    .map_err(|error| error.to_string())
}

fn review_agent_learning_with_access_and_store(
    project_path: &Path,
    access: &AgentContinuityAccess,
    store: &ContinuityStore,
    learning_id: &str,
    expected_event_count: u64,
    action: LearningFeedbackAction,
    note: String,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    review_learning_with_continuity_transition(
        project_path,
        &access.legacy_vault_path,
        store,
        learning_id,
        ReviewLearningInput {
            request_id: generate_learning_request_id(),
            expected_event_count: Some(expected_event_count),
            actor: LearningActor::User,
            action,
            note,
            replacement_learning_id: None,
        },
    )?;
    load_agent_memory_dashboard_with_access(project_path, access, store)
}

#[cfg(test)]
fn review_agent_learning_with_binding(
    project_path: &Path,
    binding: ProjectVaultBinding,
    learning_id: &str,
    expected_event_count: u64,
    action: LearningFeedbackAction,
    note: String,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    review_learning(
        project_path,
        &binding.vault_path,
        learning_id,
        ReviewLearningInput {
            request_id: generate_learning_request_id(),
            expected_event_count: Some(expected_event_count),
            actor: LearningActor::User,
            action,
            note,
            replacement_learning_id: None,
        },
    )?;
    load_agent_memory_dashboard(project_path, binding)
}

#[tauri::command]
fn correct_agent_learning(
    project_path: String,
    learning_id: String,
    expected_event_count: u64,
    title: String,
    guidance: String,
    confidence_percent: u8,
    note: String,
) -> Result<AgentMemoryDashboard, String> {
    if note.trim().is_empty() {
        return Err("A correction reason is required.".to_owned());
    }
    with_transition_agent_access(Path::new(&project_path), None, |access, store| {
        correct_agent_learning_with_access_and_store(
            Path::new(&project_path),
            access,
            store,
            &learning_id,
            AgentLearningCorrection {
                expected_event_count,
                title,
                guidance,
                confidence_percent,
                note,
            },
        )
    })
    .map_err(|error| error.to_string())
}

struct AgentLearningCorrection {
    expected_event_count: u64,
    title: String,
    guidance: String,
    confidence_percent: u8,
    note: String,
}

#[cfg(test)]
fn correct_agent_learning_with_binding(
    project_path: &Path,
    binding: ProjectVaultBinding,
    learning_id: &str,
    correction: AgentLearningCorrection,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    let current = read_learning(project_path, &binding.vault_path, learning_id)?;
    let evidence = current
        .evidence
        .into_iter()
        .map(|item| LearningEvidenceInput {
            session_id: item.session_id,
            record_id: item.record_id,
            note: item.note,
        })
        .collect();
    correct_learning(
        project_path,
        &binding.vault_path,
        learning_id,
        CorrectLearningInput {
            request_id: generate_learning_request_id(),
            expected_event_count: Some(correction.expected_event_count),
            actor: LearningActor::User,
            title: correction.title,
            guidance: correction.guidance,
            confidence_percent: correction.confidence_percent,
            evidence,
            note: correction.note,
        },
    )?;
    load_agent_memory_dashboard(project_path, binding)
}

fn correct_agent_learning_with_access_and_store(
    project_path: &Path,
    access: &AgentContinuityAccess,
    store: &ContinuityStore,
    learning_id: &str,
    correction: AgentLearningCorrection,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    let current = read_learning_with_continuity_transition(
        project_path,
        &access.legacy_vault_path,
        store,
        learning_id,
    )?;
    let evidence = current
        .evidence
        .into_iter()
        .map(|item| LearningEvidenceInput {
            session_id: item.session_id,
            record_id: item.record_id,
            note: item.note,
        })
        .collect();
    correct_learning_with_continuity_transition(
        project_path,
        &access.legacy_vault_path,
        store,
        learning_id,
        CorrectLearningInput {
            request_id: generate_learning_request_id(),
            expected_event_count: Some(correction.expected_event_count),
            actor: LearningActor::User,
            title: correction.title,
            guidance: correction.guidance,
            confidence_percent: correction.confidence_percent,
            evidence,
            note: correction.note,
        },
    )?;
    load_agent_memory_dashboard_with_access(project_path, access, store)
}

#[tauri::command]
fn read_agent_artifacts(
    project_path: String,
    vault_override: Option<String>,
    query: Option<String>,
    max_results: Option<usize>,
) -> Result<ProjectArtifactInventory, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    with_transition_agent_access(Path::new(&project_path), override_path, |access, store| {
        project_artifact_inventory_with_continuity_transition(
            &project_path,
            &access.legacy_vault_path,
            store,
            query.as_deref().unwrap_or_default(),
            max_results.unwrap_or(DEFAULT_ARTIFACT_RESULTS),
        )
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_cited_evidence(
    project_path: String,
    vault_override: Option<String>,
    citation: GraphCitation,
    context_lines: Option<u64>,
    max_characters: Option<usize>,
) -> Result<EvidenceExcerpt, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    with_transition_agent_access(Path::new(&project_path), override_path, |access, store| {
        read_project_cited_evidence_with_continuity_transition(
            &project_path,
            &access.legacy_vault_path,
            store,
            &citation,
            context_lines.unwrap_or(3),
            max_characters.unwrap_or(8_000),
        )
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_media_evidence(
    project_path: String,
    vault_override: Option<String>,
    artifact_path: String,
    artifact_snapshot_id: String,
    content_hash: String,
    max_bytes: Option<usize>,
) -> Result<AgentMediaEvidence, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    let media =
        with_transition_agent_access(Path::new(&project_path), override_path, |access, store| {
            read_project_cited_media_with_continuity_transition(
                &project_path,
                &access.legacy_vault_path,
                store,
                &artifact_path,
                &artifact_snapshot_id,
                &content_hash,
                max_bytes.unwrap_or(MAX_MEDIA_EVIDENCE_BYTES),
            )
        })
        .map_err(|error| error.to_string())?;
    let mime_type = media.media_type.mime_type().to_owned();
    Ok(AgentMediaEvidence {
        artifact_path: media.artifact_path,
        artifact_snapshot_id: media.artifact_snapshot_id,
        content_hash: media.content_hash,
        media_type: media.media_type,
        mime_type: mime_type.clone(),
        source_bytes: media.source_bytes,
        data_url: format!(
            "data:{mime_type};base64,{}",
            BASE64_STANDARD.encode(media.data)
        ),
        evidence_role: media.evidence_role,
        source_boundary: media.source_boundary,
        live_source_checked: media.live_source_checked,
        derived_description_included: media.derived_description_included,
    })
}

#[tauri::command]
fn read_agent_project_activity(
    project_path: String,
    vault_override: Option<String>,
    query: Option<String>,
    problem_scope: Option<ProjectProblemScope>,
    max_results: Option<usize>,
) -> Result<ProjectActivityView, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    with_transition_agent_access(Path::new(&project_path), override_path, |access, store| {
        project_activity_view_with_continuity_transition(
            &project_path,
            &access.legacy_vault_path,
            store,
            query.as_deref().unwrap_or_default(),
            problem_scope.unwrap_or(ProjectProblemScope::All),
            max_results.unwrap_or(DEFAULT_PROJECT_ACTIVITY_RESULTS),
        )
    })
    .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_agent_projects,
            forget_agent_project,
            read_agent_capture_settings,
            read_agent_egress_policy,
            update_agent_egress_policy,
            update_agent_capture_mode,
            erase_agent_project_memory,
            search_agent_projects,
            search_agent_project_memory,
            inspect_agent_project,
            read_agent_project_approved_sources,
            open_agent_project_markdown_source,
            export_agent_project_continuity,
            approve_agent_project_file_source,
            reapprove_agent_project_file_source,
            revoke_agent_project_approved_source,
            initialize_agent_project,
            connect_agent_project,
            refresh_agent_project,
            read_agent_learning,
            read_agent_session,
            read_agent_session_turns,
            rename_agent_session,
            erase_agent_session,
            review_agent_learning,
            correct_agent_learning,
            read_agent_artifacts,
            read_agent_cited_evidence,
            read_agent_media_evidence,
            read_agent_project_activity
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ley");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_project_egress_reads_updates_and_rejects_stale_policy() {
        let root = std::env::temp_dir().join(format!(
            "ley-desktop-project-egress-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        initialize_project(&project, Some("Desktop egress"), CaptureMode::Structured).unwrap();
        let registry = EgressPolicyRegistry::at(root.join("private/egress.json"));
        let store = ContinuityStore::at(root.join("private/continuity.sqlite3"));
        let specification_id = "spec_11111111111111111111111111111111";
        registry
            .set_specification_policy(&project, specification_id, AgentEgressPolicy::NeverSend)
            .unwrap();

        let initial = read_agent_egress_policy_with(&project, &registry, &store).unwrap();
        assert_eq!(initial.project_policy, AgentEgressPolicy::AgentOk);
        assert_eq!(initial.specification_overrides.len(), 1);

        let updated = update_agent_egress_policy_with(
            &project,
            &registry,
            &store,
            &initial.project_id,
            AgentEgressPolicy::AgentOk,
            AgentEgressPolicy::ConfirmPerUse,
        )
        .unwrap();
        assert_eq!(updated.project_policy, AgentEgressPolicy::ConfirmPerUse);
        assert_eq!(updated.specification_overrides.len(), 1);
        assert_eq!(
            updated.specification_overrides[0].policy,
            AgentEgressPolicy::NeverSend
        );

        registry
            .set_project_policy_transition(&project, &store, AgentEgressPolicy::LocalModelOnly)
            .unwrap();
        let stale = update_agent_egress_policy_with(
            &project,
            &registry,
            &store,
            &updated.project_id,
            AgentEgressPolicy::ConfirmPerUse,
            AgentEgressPolicy::AgentOk,
        )
        .unwrap_err();
        assert!(matches!(
            stale,
            LeyCoreError::AgentEgressPolicyChanged {
                ref expected,
                ref current,
                ..
            } if expected == "confirm-per-use" && current == "local-model-only"
        ));
        assert_eq!(
            read_agent_egress_policy_with(&project, &registry, &store)
                .unwrap()
                .project_policy,
            AgentEgressPolicy::LocalModelOnly
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn desktop_session_reads_use_proven_native_snapshot_after_vault_disappears() {
        let root = std::env::temp_dir().join(format!(
            "ley-desktop-session-transition-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let project = root.join("project");
        let vault = root.join("vault");
        let config = root.join("config");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(&config).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(project.join("README.md"), "# Desktop transition\n").unwrap();
        initialize_project(
            &project,
            Some("Desktop session transition"),
            CaptureMode::Structured,
        )
        .unwrap();
        let registry = BindingRegistry::at(config.join("bindings.json"));
        registry.bind(&project, &vault).unwrap();
        ingest_project(&project, &vault).unwrap();
        let started = ley_core::start_session(
            &project,
            &vault,
            ley_core::StartSessionInput {
                request_id: format!("req_{}", "d".repeat(32)),
                name: "Desktop native read".to_owned(),
                goal: "Keep session detail readable after the legacy vault disappears".to_owned(),
                source: ley_core::SessionSource::default(),
            },
        )
        .unwrap();
        ley_core::record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            ley_core::TurnEvidenceInput {
                request_id: format!("req_{}", "e".repeat(32)),
                origin: ley_core::TurnEvidenceOrigin::HostHook,
                host: Some("codex".to_owned()),
                correlation_material: Some("desktop-transition-turn".to_owned()),
                text: "Preserve this bounded prompt.".to_owned(),
            },
        )
        .unwrap();
        let store = ContinuityStore::at(config.join("continuity.sqlite3"));

        let imported = with_transition_agent_session_read_from(
            &project,
            &registry,
            &store,
            |project, vault, store| {
                read_session_context_with_continuity_transition(
                    project,
                    vault,
                    store,
                    &started.session.session_id,
                    DEFAULT_SESSION_CONTEXT_CHECKPOINTS,
                    DEFAULT_SESSION_CONTEXT_CHARACTERS,
                )
            },
        )
        .unwrap();
        assert_eq!(imported.prompt_count, 1);

        fs::remove_dir_all(&vault).unwrap();
        let context = with_transition_agent_session_read_from(
            &project,
            &registry,
            &store,
            |project, vault, store| {
                read_session_context_with_continuity_transition(
                    project,
                    vault,
                    store,
                    &started.session.session_id,
                    DEFAULT_SESSION_CONTEXT_CHECKPOINTS,
                    DEFAULT_SESSION_CONTEXT_CHARACTERS,
                )
            },
        )
        .unwrap();
        assert_eq!(context.session_id, started.session.session_id);
        assert_eq!(context.prompt_count, 1);
        assert!(context.revision_freshness.captured_head.is_none());

        let turns = with_transition_agent_session_read_from(
            &project,
            &registry,
            &store,
            |project, vault, store| {
                read_session_turns_context_with_continuity_transition(
                    project,
                    vault,
                    store,
                    &started.session.session_id,
                    DEFAULT_SESSION_TURN_RESULTS,
                    DEFAULT_SESSION_TURN_CHARACTERS,
                )
            },
        )
        .unwrap();
        assert_eq!(turns.prompt_count, 1);
        assert_eq!(
            turns.turns[0].text.as_deref(),
            Some("Preserve this bounded prompt.")
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn desktop_artifact_refresh_uses_native_authority_after_vault_disappears() {
        let root = std::env::temp_dir().join(format!(
            "ley-desktop-artifact-transition-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let project = root.join("project");
        let vault = root.join("vault");
        let pending_project = root.join("pending-project");
        let pending_vault = root.join("pending-vault");
        let config = root.join("config");
        for directory in [&project, &vault, &pending_project, &pending_vault, &config] {
            fs::create_dir_all(directory).unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(
            project.join("README.md"),
            "# Native desktop artifact\nfirst\n",
        )
        .unwrap();
        fs::write(
            pending_project.join("README.md"),
            "# Pending desktop artifact\n",
        )
        .unwrap();
        initialize_project(
            &project,
            Some("Desktop artifact transition"),
            CaptureMode::Structured,
        )
        .unwrap();
        initialize_project(
            &pending_project,
            Some("Desktop pending artifact transition"),
            CaptureMode::Structured,
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        ingest_project(&pending_project, &pending_vault).unwrap();

        let registry = BindingRegistry::at(config.join("bindings.json"));
        registry.bind(&project, &vault).unwrap();
        registry.bind(&pending_project, &pending_vault).unwrap();
        let store = ContinuityStore::at(config.join("continuity.sqlite3"));

        let first = with_transition_agent_binding_from(
            &project,
            None,
            &registry,
            &store,
            |binding, store| {
                ingest_project_with_continuity_transition(&project, &binding.vault_path, store)
            },
        )
        .unwrap();
        assert!(first.manifest_path.is_none());

        fs::remove_dir_all(&vault).unwrap();
        fs::write(
            project.join("README.md"),
            "# Native desktop artifact\nsecond after vault loss\n",
        )
        .unwrap();
        let refreshed = with_transition_agent_binding_from(
            &project,
            None,
            &registry,
            &store,
            |binding, store| {
                ingest_project_with_continuity_transition(&project, &binding.vault_path, store)
            },
        )
        .unwrap();
        assert_ne!(refreshed.snapshot_id, first.snapshot_id);
        assert!(refreshed.changed);
        assert!(refreshed.manifest_path.is_none());

        fs::remove_dir_all(&pending_vault).unwrap();
        assert!(matches!(
            with_transition_agent_binding_from(
                &pending_project,
                None,
                &registry,
                &store,
                |binding, store| {
                    ingest_project_with_continuity_transition(
                        &pending_project,
                        &binding.vault_path,
                        store,
                    )
                },
            ),
            Err(LeyCoreError::BoundVaultUnavailable { .. })
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn desktop_project_erasure_preserves_user_owned_markdown_canvas_and_binding() {
        let root = std::env::temp_dir().join(format!(
            "ley-native-project-erasure-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let project = root.join("project");
        let vault = root.join("vault");
        let config = root.join("config");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(&config).unwrap();

        let source = b"# Portable project\n\nproject_source_canary_7d21\n";
        fs::write(project.join("README.md"), source).unwrap();
        let initialized = initialize_project(
            &project,
            Some("Portable erasure project"),
            CaptureMode::Structured,
        )
        .unwrap();
        let project_metadata =
            fs::read(project.join(ley_core::LEY_DIRECTORY).join("project.json")).unwrap();
        let binding_registry_path = config.join(ley_core::BINDING_REGISTRY_FILE);
        let registry = BindingRegistry::at(&binding_registry_path);
        let binding = registry.bind(&project, &vault).unwrap();
        let binding_registry_bytes = fs::read(&binding_registry_path).unwrap();
        ingest_project(&project, &vault).unwrap();

        let private_memory_canary = "private_agent_memory_canary_8be4";
        let note_body = format!(
            "# Portable user note\n\nThis independent Markdown copy keeps {private_memory_canary}.\n"
        );
        let canvas_body = format!(
            "{{\"nodes\":[{{\"id\":\"a\",\"type\":\"text\",\"text\":\"{private_memory_canary}\",\"x\":0,\"y\":0,\"width\":260,\"height\":140}}],\"edges\":[]}}"
        );
        fs::create_dir_all(vault.join("Portable")).unwrap();
        fs::create_dir_all(vault.join("canvases")).unwrap();
        fs::write(vault.join("Portable/User note.md"), &note_body).unwrap();
        fs::write(vault.join("canvases/User board.canvas"), &canvas_body).unwrap();

        let started = ley_core::start_session(
            &project,
            &vault,
            ley_core::StartSessionInput {
                request_id: ley_core::generate_request_id(),
                name: "Private erasure session".to_owned(),
                goal: format!("Retain {private_memory_canary} only in Agent Memory"),
                source: ley_core::SessionSource::default(),
            },
        )
        .unwrap();
        let checkpoint = ley_core::checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            ley_core::CheckpointInput {
                request_id: ley_core::generate_request_id(),
                summary: format!("Captured {private_memory_canary} before project erasure"),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: Vec::new(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let checkpoint_id = checkpoint.session.checkpoints[0].id.clone();
        let proposed = ley_core::propose_learning(
            &project,
            &vault,
            ley_core::ProposeLearningInput {
                request_id: ley_core::generate_learning_request_id(),
                actor: ley_core::LearningActor::Agent,
                kind: ley_core::LearningKind::Fact,
                title: "Private erasure learning".to_owned(),
                guidance: format!("Remember {private_memory_canary} only inside Agent Memory."),
                confidence_percent: 90,
                provenance: ley_core::LearningProvenance::Inferred,
                evidence: vec![ley_core::LearningEvidenceInput {
                    session_id: started.session.session_id.clone(),
                    record_id: checkpoint_id,
                    note: "Whole-project erasure acceptance evidence.".to_owned(),
                }],
            },
        )
        .unwrap();
        assert!(ley_core::read_session_context(
            &project,
            &vault,
            &started.session.session_id,
            5,
            8_000
        )
        .unwrap()
        .checkpoints
        .iter()
        .any(|checkpoint| checkpoint.summary.contains(private_memory_canary)));
        assert!(ley_core::read_learning_context(
            &project,
            &vault,
            &proposed.learning.learning_id,
            10,
            10,
            10,
            8_000
        )
        .unwrap()
        .guidance
        .contains(private_memory_canary));

        let project_store = vault
            .join(".ley")
            .join("agent-memory")
            .join("projects")
            .join(&initialized.identity.project_id);
        assert!(project_store.is_dir());

        let inspection = erase_agent_project_memory_with_registry(&project, &registry).unwrap();
        let AgentProjectInspection::NeedsCapture {
            project_id,
            project_name,
            storage: preserved_storage,
            ..
        } = inspection
        else {
            panic!("expected NeedsCapture after whole-project Agent Memory erasure");
        };
        assert_eq!(project_id, initialized.identity.project_id);
        assert_eq!(project_name, "Portable erasure project");
        let AgentMemoryStorage::LegacyVault {
            project_id: preserved_project_id,
            ..
        } = preserved_storage
        else {
            panic!("legacy erasure compatibility should preserve the legacy vault storage marker");
        };
        assert_eq!(preserved_project_id, binding.project_id);
        assert!(!project_store.exists());
        assert!(matches!(
            ley_core::project_memory_overview(&project, &vault),
            Err(LeyCoreError::ProjectMemoryUnavailable(_))
        ));

        assert_eq!(fs::read(project.join("README.md")).unwrap(), source);
        assert_eq!(
            fs::read(project.join(ley_core::LEY_DIRECTORY).join("project.json")).unwrap(),
            project_metadata
        );
        assert_eq!(
            registry.resolve(&project, None).unwrap().vault_path,
            binding.vault_path
        );
        assert_eq!(
            fs::read(&binding_registry_path).unwrap(),
            binding_registry_bytes
        );
        assert_eq!(
            fs::read_to_string(vault.join("Portable/User note.md")).unwrap(),
            note_body
        );
        assert_eq!(
            fs::read_to_string(vault.join("canvases/User board.canvas")).unwrap(),
            canvas_body
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn approved_source_authority_migrates_legacy_then_tracks_project_file_revision() {
        let root = std::env::temp_dir().join(format!(
            "ley-native-specification-authority-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let project = root.join("project");
        let vault = root.join("vault");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        initialize_project(
            &project,
            Some("Approved source project"),
            CaptureMode::Structured,
        )
        .unwrap();
        fs::create_dir_all(vault.join("Specs")).unwrap();
        fs::write(
            vault.join("Specs/Product.md"),
            "# Legacy product intent\n\n- Works offline.\n",
        )
        .unwrap();
        let legacy = SpecificationRegistry::at(root.join("config/specifications.json"));
        let legacy_id = ley_core::generate_specification_id();
        legacy
            .approve(&project, &vault, &legacy_id, "Specs/Product.md")
            .unwrap();
        let store = ley_core::ContinuityStore::at(root.join("private/continuity.sqlite3"));
        let registry = ApprovedSourceRegistry::at(store.clone());
        let migrated = registry
            .migrate_legacy_specifications(&project, &vault, &legacy)
            .unwrap();
        assert!(migrated.authority_migrated);
        let export_parent = root.join("legacy-exports");
        fs::create_dir_all(&export_parent).unwrap();
        let legacy_access = AgentContinuityAccess::from_legacy_binding(ProjectVaultBinding {
            project_id: diagnose_project(&project).unwrap().identity.project_id,
            vault_path: vault.clone(),
            source: BindingSource::Persisted,
        });
        let legacy_export = export_agent_project_continuity_with_access(
            &project,
            &export_parent,
            &legacy_access,
            &store,
        )
        .unwrap();
        assert!(legacy_export.destination.is_dir());
        assert_eq!(legacy_export.project_id, legacy_access.project_id);
        let authority = registry.authority(&project).unwrap();
        assert_eq!(authority.current, 1);
        assert_eq!(authority.sources[0].approval.source_id, legacy_id);
        assert_eq!(
            authority.sources[0].approval.source_kind,
            ley_core::ApprovedSourceKind::ImportedSnapshot
        );

        fs::write(
            vault.join("Specs/Product.md"),
            "# Legacy product intent\n\n- Changed after migration.\n",
        )
        .unwrap();
        assert_eq!(registry.authority(&project).unwrap().current, 1);

        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs/requirements.md"),
            "# Current project intent\n\n- Stay offline.\n",
        )
        .unwrap();
        let project_file = registry
            .approve_project_file(&project, "docs/requirements.md")
            .unwrap();
        let authority = registry.authority(&project).unwrap();
        assert_eq!(authority.current, 2);
        assert_eq!(authority.changed, 0);
        assert_eq!(
            approved_markdown_path_for_external_editor(
                &project,
                &project_file.source_id,
                &registry,
            )
            .unwrap(),
            project.join("docs/requirements.md")
        );
        assert!(matches!(
            approved_markdown_path_for_external_editor(&project, &legacy_id, &registry),
            Err(LeyCoreError::InvalidApprovedSourceRequest(message))
                if message.contains("project-file")
        ));
        fs::write(project.join("docs/plain.txt"), "plain intent\n").unwrap();
        let non_markdown = registry
            .approve_project_file(&project, "docs/plain.txt")
            .unwrap();
        assert!(matches!(
            approved_markdown_path_for_external_editor(
                &project,
                &non_markdown.source_id,
                &registry,
            ),
            Err(LeyCoreError::InvalidApprovedSourceRequest(message))
                if message.contains("Markdown")
        ));
        assert!(registry.revoke(&project, &non_markdown.source_id).unwrap());

        fs::write(
            project.join("docs/requirements.md"),
            "# Current project intent\n\n- Changed without approval.\n",
        )
        .unwrap();
        let changed = registry.authority(&project).unwrap();
        assert_eq!(changed.current, 1);
        assert_eq!(changed.changed, 1);
        assert!(matches!(
            approved_markdown_path_for_external_editor(
                &project,
                &project_file.source_id,
                &registry,
            ),
            Err(LeyCoreError::ApprovedSourceStale { .. })
        ));

        let reapproved = registry
            .reapprove_project_file(&project, &project_file.source_id)
            .unwrap();
        assert_eq!(reapproved.source_id, project_file.source_id);
        assert_eq!(registry.authority(&project).unwrap().current, 2);
        assert!(registry.revoke(&project, &project_file.source_id).unwrap());
        assert!(matches!(
            approved_markdown_path_for_external_editor(
                &project,
                &project_file.source_id,
                &registry,
            ),
            Err(LeyCoreError::ApprovedSourceNotFound(_))
        ));
        assert!(registry.revoke(&project, &legacy_id).unwrap());
        assert!(registry.authority(&project).unwrap().sources.is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn uninitialized_project_inspection_previews_capture_without_writing_metadata() {
        let root = std::env::temp_dir().join(format!(
            "ley-initial-preview-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::write(root.join("README.md"), "# Preview\n").unwrap();
        fs::write(root.join("src/main.ts"), "export const ready = true;\n").unwrap();
        fs::write(root.join(".env"), "TOKEN=secret\n").unwrap();
        fs::write(root.join("target/generated.js"), "generated\n").unwrap();

        let inspection = inspect_agent_project(root.to_string_lossy().into_owned()).unwrap();
        assert!(!root.join(ley_core::LEY_DIRECTORY).exists());
        let AgentProjectInspection::Uninitialized {
            suggested_name,
            preview,
        } = inspection
        else {
            panic!("expected uninitialized inspection");
        };
        assert!(suggested_name.contains("ley-initial-preview-test"));
        assert_eq!(preview.mode, CaptureMode::Structured);
        assert_eq!(preview.approved_roots, vec!["."]);
        assert!(preview.respect_gitignore);
        assert_eq!(preview.eligible_files, 2);
        assert!(preview
            .included_paths
            .iter()
            .any(|path| path == "README.md"));
        assert!(preview
            .included_paths
            .iter()
            .any(|path| path == "src/main.ts"));
        assert!(preview.privacy_notice.contains("creates no .ley metadata"));

        let other = root.with_file_name(format!(
            "{}-other",
            root.file_name().unwrap().to_string_lossy()
        ));
        fs::create_dir_all(other.join("src")).unwrap();
        fs::create_dir_all(other.join("target")).unwrap();
        fs::write(other.join("README.md"), "# Preview\n").unwrap();
        fs::write(other.join("src/main.ts"), "export const ready = true;\n").unwrap();
        fs::write(other.join(".env"), "TOKEN=secret\n").unwrap();
        fs::write(other.join("target/generated.js"), "generated\n").unwrap();
        let other_preview = agent_initial_capture_preview(&other).unwrap();
        assert_eq!(preview.plan_fingerprint, other_preview.plan_fingerprint);
        assert_ne!(
            preview.approval_fingerprint,
            other_preview.approval_fingerprint
        );

        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(other).unwrap();
    }

    #[test]
    fn fresh_agent_project_initializes_native_continuity_without_vault_binding() {
        let root = std::env::temp_dir().join(format!(
            "ley-native-initialize-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = root.join("project");
        let private = root.join("private");
        let config = root.join("config");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&private).unwrap();
        fs::create_dir_all(&config).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
            fs::set_permissions(&config, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(project.join("README.md"), "# Native project\n").unwrap();

        let reviewed = agent_initial_capture_preview(&project).unwrap();
        let store = ContinuityStore::at(private.join("continuity.sqlite3"));
        let catalog = ProjectCatalog::at(config.join(ley_core::PROJECT_CATALOG_FILE));
        let dashboard = initialize_agent_project_with_store_and_catalog(
            &project,
            &reviewed.approval_fingerprint,
            &store,
            &catalog,
        )
        .unwrap();

        let AgentMemoryStorage::Native { project_id } = dashboard.storage else {
            panic!("fresh projects must use native continuity storage");
        };
        let diagnostic = diagnose_project(&project).unwrap();
        assert_eq!(project_id, diagnostic.identity.project_id);
        assert!(ley_core::native_canonical_read_authority_available(&project, &store).unwrap());
        let export_parent = root.join("exports");
        fs::create_dir_all(&export_parent).unwrap();
        let native_access = AgentContinuityAccess::native(project_id.clone(), &store).unwrap();
        let exported = export_agent_project_continuity_with_access(
            &project,
            &export_parent,
            &native_access,
            &store,
        )
        .unwrap();
        assert_eq!(exported.project_id, project_id);
        assert!(exported.destination.starts_with(&export_parent));
        assert!(exported.destination.is_dir());
        assert!(!exported.destination.starts_with(&project));
        assert!(matches!(
            export_agent_project_continuity_with_access(
                &project,
                &project,
                &native_access,
                &store,
            ),
            Err(LeyCoreError::InvalidPortableContinuityBundle(message))
                if message.contains("outside the project")
        ));
        let registry = BindingRegistry::at(config.join(ley_core::BINDING_REGISTRY_FILE));
        assert!(matches!(
            registry.resolve_observed(&diagnostic),
            Err(LeyCoreError::VaultNotBound(_))
        ));
        let late_vault = root.join("late-legacy-vault");
        fs::create_dir_all(&late_vault).unwrap();
        let late_connect =
            connect_agent_project_binding_with_registry(&project, &late_vault, &registry, &store);
        assert!(matches!(
            late_connect,
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("cannot bind a legacy vault to a project already registered for native continuity")
        ));
        assert!(matches!(
            registry.resolve_observed(&diagnostic),
            Err(LeyCoreError::VaultNotBound(_))
        ));
        assert_eq!(
            catalog
                .list(ley_core::DEFAULT_PROJECT_CATALOG_RESULTS)
                .unwrap()
                .projects
                .len(),
            1
        );

        let erased =
            erase_agent_project_memory_with_registry_and_store(&project, &registry, &store)
                .unwrap();
        let AgentProjectInspection::NeedsCapture { storage, .. } = erased else {
            panic!("erased native projects must remain native and require recapture");
        };
        assert!(matches!(storage, AgentMemoryStorage::Native { .. }));
        assert!(!ley_core::native_canonical_read_authority_available(&project, &store).unwrap());
        assert!(native_born_project_registration_exists(&project, &store).unwrap());
        assert!(matches!(
            registry.resolve_observed(&diagnostic),
            Err(LeyCoreError::VaultNotBound(_))
        ));

        let catalog_view =
            load_agent_project_catalog_from(&catalog, &registry, Some(&store)).unwrap();
        let catalog_item = catalog_view
            .projects
            .iter()
            .find(|item| item.project_id == diagnostic.identity.project_id)
            .unwrap();
        assert_eq!(catalog_item.state, AgentProjectCatalogState::NeedsCapture);

        let (recaptured, dashboard) = with_transition_agent_access_from(
            &project,
            None,
            &registry,
            &store,
            |access, store| {
                let recaptured = ingest_agent_project_with_access(&project, access, store)?;
                let dashboard = load_agent_memory_dashboard_with_access(&project, access, store)?;
                Ok((recaptured, dashboard))
            },
        )
        .unwrap();
        assert!(recaptured.changed);
        assert!(matches!(
            dashboard.storage,
            AgentMemoryStorage::Native { .. }
        ));
        assert!(ley_core::native_canonical_read_authority_available(&project, &store).unwrap());
        assert!(matches!(
            registry.resolve_observed(&diagnostic),
            Err(LeyCoreError::VaultNotBound(_))
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_initial_capture_approval_rejects_before_project_or_vault_write() {
        let root = std::env::temp_dir().join(format!(
            "ley-stale-initial-preview-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = root.join("project");
        let vault = root.join("vault-must-not-be-created");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("README.md"), "# Reviewed\n").unwrap();
        let reviewed = agent_initial_capture_preview(&project).unwrap();
        fs::write(
            project.join("new-after-review.ts"),
            "export const changed = true;\n",
        )
        .unwrap();

        let error = match initialize_agent_project(
            project.to_string_lossy().into_owned(),
            reviewed.approval_fingerprint,
        ) {
            Ok(_) => panic!("stale approval unexpectedly initialized the project"),
            Err(error) => error,
        };
        assert!(error.contains("capture plan changed"));
        assert!(!project.join(ley_core::LEY_DIRECTORY).exists());
        assert!(!vault.exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unbound_connect_requires_existing_matching_legacy_memory() {
        let root = std::env::temp_dir().join(format!(
            "ley-unbound-review-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = root.join("project");
        let vault = root.join("vault");
        let config = root.join("config");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(project.join("README.md"), "# Reviewed\n").unwrap();
        initialize_project(&project, Some("Unbound review"), CaptureMode::Structured).unwrap();
        let diagnostic = diagnose_project(&project).unwrap();
        let registry = BindingRegistry::at(config.join("bindings.json"));

        let error = match connect_agent_project_with_registry(&project, &vault, &registry) {
            Ok(_) => panic!("empty legacy vault unexpectedly reconnected the project"),
            Err(error) => error,
        };
        assert!(matches!(error, LeyCoreError::InvalidContinuityStore(_)));
        assert!(error
            .to_string()
            .contains("selected legacy vault cannot be verified for this project"));
        assert!(matches!(
            registry.resolve_observed(&diagnostic),
            Err(LeyCoreError::VaultNotBound(_))
        ));
        assert!(fs::read_dir(&vault).unwrap().next().is_none());

        ingest_project(&project, &vault).unwrap();
        let dashboard = connect_agent_project_with_registry(&project, &vault, &registry).unwrap();
        let AgentMemoryStorage::LegacyVault { project_id, .. } = dashboard.storage else {
            panic!("legacy unbound migration should still report legacy-vault storage");
        };
        assert_eq!(project_id, diagnostic.identity.project_id);
        assert!(vault.join(".ley").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unavailable_binding_reconnect_requires_matching_legacy_memory_before_rebind() {
        let root = std::env::temp_dir().join(format!(
            "ley-unavailable-reconnect-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        // macOS exposes its temporary directory through `/var`, which is a
        // symlink to `/private/var`. SQLite's NOFOLLOW open intentionally
        // rejects that alias, so exercise the real canonical directory.
        let root = root.canonicalize().unwrap();
        let project = root.join("project");
        let original_vault = root.join("original-vault");
        let moved_vault = root.join("moved-vault");
        let wrong_vault = root.join("wrong-vault");
        let config = root.join("config");
        for directory in [&project, &original_vault, &wrong_vault, &config] {
            fs::create_dir_all(directory).unwrap();
        }
        fs::write(project.join("README.md"), "# Legacy continuity\n").unwrap();
        initialize_project(
            &project,
            Some("Unavailable reconnect"),
            CaptureMode::Structured,
        )
        .unwrap();
        let diagnostic = diagnose_project(&project).unwrap();
        ingest_project(&project, &original_vault).unwrap();
        let registry = BindingRegistry::at(config.join("bindings.json"));
        let original_binding = registry.bind(&project, &original_vault).unwrap();
        fs::rename(&original_vault, &moved_vault).unwrap();

        assert!(matches!(
            registry.resolve_observed(&diagnostic),
            Err(LeyCoreError::BoundVaultUnavailable { ref path, .. })
                if path == &original_binding.vault_path
        ));

        let error = match connect_agent_project_with_registry(&project, &wrong_vault, &registry) {
            Ok(_) => panic!("wrong legacy vault unexpectedly reconnected the project"),
            Err(error) => error,
        };
        assert!(matches!(error, LeyCoreError::InvalidContinuityStore(_)));
        assert!(error
            .to_string()
            .contains("selected legacy vault cannot be verified for this project"));
        assert!(fs::read_dir(&wrong_vault).unwrap().next().is_none());
        assert!(matches!(
            registry.resolve_observed(&diagnostic),
            Err(LeyCoreError::BoundVaultUnavailable { ref path, .. })
                if path == &original_binding.vault_path
        ));

        let dashboard =
            connect_agent_project_with_registry(&project, &moved_vault, &registry).unwrap();
        assert!(matches!(
            dashboard.storage,
            AgentMemoryStorage::LegacyVault { ref project_id, .. }
                if project_id == &diagnostic.identity.project_id
        ));
        assert_eq!(
            registry.resolve_observed(&diagnostic).unwrap().vault_path,
            moved_vault.canonicalize().unwrap()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn portability_project_catalog_summarizes_ready_unbound_and_unavailable_projects() {
        let root =
            std::env::temp_dir().join(format!("ley-project-catalog-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let config = root.join("config");
        let vault = root.join("vault");
        let moved_vault = root.join("moved-vault");
        let ready = root.join("ready");
        let unbound = root.join("unbound");
        let unavailable = root.join("unavailable");
        let disconnected = root.join("disconnected");
        for directory in [
            &vault,
            &moved_vault,
            &ready,
            &unbound,
            &unavailable,
            &disconnected,
        ] {
            fs::create_dir_all(directory).unwrap();
        }
        fs::write(ready.join("main.rs"), "fn ready_marker() {}\n").unwrap();
        initialize_project(&ready, Some("Ready project"), CaptureMode::Structured).unwrap();
        initialize_project(&unbound, Some("Unbound project"), CaptureMode::Minimal).unwrap();
        initialize_project(
            &unavailable,
            Some("Unavailable project"),
            CaptureMode::Structured,
        )
        .unwrap();
        initialize_project(
            &disconnected,
            Some("Disconnected project"),
            CaptureMode::Structured,
        )
        .unwrap();

        let catalog = ProjectCatalog::at(config.join(ley_core::PROJECT_CATALOG_FILE));
        let registry = BindingRegistry::at(config.join(ley_core::BINDING_REGISTRY_FILE));
        registry.bind(&ready, &vault).unwrap();
        ingest_project(&ready, &vault).unwrap();
        registry.bind(&disconnected, &moved_vault).unwrap();
        catalog.observe(&unbound).unwrap();
        catalog.observe(&unavailable).unwrap();
        let unavailable_canonical = unavailable.canonicalize().unwrap();
        fs::remove_dir_all(&unavailable).unwrap();
        fs::rename(&moved_vault, root.join("vault-after-move")).unwrap();

        let view = load_agent_project_catalog_from(&catalog, &registry, None).unwrap();
        assert_eq!(view.total_projects, 4);
        assert_eq!(view.ready_projects, 1);
        assert_eq!(view.attention_projects, 3);
        let ready_item = view
            .projects
            .iter()
            .find(|project| project.project_name == "Ready project")
            .unwrap();
        assert_eq!(ready_item.state, AgentProjectCatalogState::Ready);
        assert_eq!(ready_item.files, Some(1));
        assert_eq!(ready_item.sessions, Some(0));
        let unbound_item = view
            .projects
            .iter()
            .find(|project| project.project_name == "Unbound project")
            .unwrap();
        assert_eq!(unbound_item.state, AgentProjectCatalogState::Unbound);
        assert!(view.projects.iter().any(|project| {
            project.project_name == "Disconnected project"
                && project.state == AgentProjectCatalogState::VaultUnavailable
        }));
        assert!(view.projects.iter().any(|project| {
            project.state == AgentProjectCatalogState::ProjectUnavailable
                && project.project_path == unavailable_canonical
        }));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_catalog_counts_native_sessions_after_session_authority_cutover() {
        let root = std::env::temp_dir().join(format!(
            "ley-project-catalog-native-session-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let config = root.join("config");
        let continuity = root.join("continuity");
        let vault = root.join("vault");
        let project = root.join("project");
        for directory in [&config, &continuity, &vault, &project] {
            fs::create_dir_all(directory).unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&continuity, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(project.join("main.rs"), "fn native_session_marker() {}\n").unwrap();
        initialize_project(
            &project,
            Some("Native session catalog"),
            CaptureMode::Structured,
        )
        .unwrap();
        let catalog = ProjectCatalog::at(config.join(ley_core::PROJECT_CATALOG_FILE));
        let registry = BindingRegistry::at(config.join(ley_core::BINDING_REGISTRY_FILE));
        registry.bind(&project, &vault).unwrap();
        ingest_project(&project, &vault).unwrap();
        catalog.observe(&project).unwrap();
        let store = ContinuityStore::at(continuity.join("continuity.sqlite3"));

        let started = ley_core::start_session_with_continuity_transition(
            &project,
            &vault,
            &store,
            ley_core::StartSessionInput {
                request_id: generate_request_id(),
                name: "Native catalog session".to_owned(),
                goal: "Keep project-card session counts on native authority".to_owned(),
                source: ley_core::SessionSource::default(),
            },
        )
        .unwrap();
        assert_eq!(started.session.event_count, 1);
        assert_eq!(
            project_session_stats(&project, &vault)
                .unwrap()
                .total_sessions,
            0
        );

        let view = load_agent_project_catalog_from(&catalog, &registry, Some(&store)).unwrap();
        let item = view
            .projects
            .iter()
            .find(|item| item.project_name == "Native session catalog")
            .unwrap();
        assert_eq!(item.sessions, Some(1));
        assert_eq!(item.active_sessions, Some(1));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn agent_memory_dashboard_reads_sessions_and_reviewable_lessons() {
        use ley_core::{
            checkpoint_session, propose_learning, review_learning, start_session, CheckpointInput,
            LearningActor, LearningEvidenceInput, LearningFeedbackAction, LearningKind,
            LearningProvenance, ProposeLearningInput, RenameSessionInput, ReviewLearningInput,
            SessionSource, StartSessionInput,
        };

        let root =
            std::env::temp_dir().join(format!("ley-agent-dashboard-test-{}", std::process::id()));
        let project = root.join("project");
        let vault = root.join("vault");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::write(
            project.join("README.md"),
            "# Dashboard\n\nUse cited memory.",
        )
        .unwrap();
        let initialized =
            initialize_project(&project, Some("Dashboard project"), CaptureMode::Structured)
                .unwrap();
        ingest_project(&project, &vault).unwrap();
        let session = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: format!("req_{}", "1".repeat(32)),
                name: "Build memory dashboard".into(),
                goal: "Expose real continuity data in the desktop app.".into(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &session.session.session_id,
            CheckpointInput {
                request_id: format!("req_{}", "2".repeat(32)),
                summary: "The desktop bridge reads the shared engine.".into(),
                plan: vec![],
                decisions: vec![],
                tasks: vec![],
                problems: vec![],
                touched_artifacts: vec!["README.md".into()],
                commands: vec![],
                verification: vec![],
                unresolved: vec![],
            },
        )
        .unwrap();
        let learning = propose_learning(
            &project,
            &vault,
            ProposeLearningInput {
                request_id: format!("req_{}", "3".repeat(32)),
                actor: LearningActor::Agent,
                kind: LearningKind::Procedure,
                title: "Use the shared local engine".into(),
                guidance: "Read sessions and lessons through ley-core.".into(),
                confidence_percent: 90,
                provenance: LearningProvenance::AgentAuthored,
                evidence: vec![LearningEvidenceInput {
                    session_id: session.session.session_id.clone(),
                    record_id: checkpoint.session.checkpoints.last().unwrap().id.clone(),
                    note: "Captured in the dashboard implementation session.".into(),
                }],
            },
        )
        .unwrap();
        let binding = ProjectVaultBinding {
            project_id: initialized.identity.project_id,
            vault_path: vault.clone(),
            source: BindingSource::Override,
        };

        rename_session(
            &project,
            &vault,
            &session.session.session_id,
            RenameSessionInput {
                request_id: format!("req_{}", "5".repeat(32)),
                expected_event_count: Some(checkpoint.session.event_count),
                name: "Ship memory dashboard".into(),
                note: "The completed session now has a more specific name.".into(),
            },
        )
        .unwrap();
        let pending = load_agent_memory_dashboard(&project, binding.clone()).unwrap();
        assert_eq!(pending.resume.total_sessions, 1);
        assert_eq!(pending.sessions.len(), 1);
        assert_eq!(pending.sessions[0].name, "Ship memory dashboard");
        assert_eq!(pending.resume.sessions[0].name, "Ship memory dashboard");
        assert_eq!(pending.review_inbox.total_matching, 1);
        assert_eq!(pending.overview.files, 1);

        review_learning(
            &project,
            &vault,
            &learning.learning.learning_id,
            ReviewLearningInput {
                request_id: format!("req_{}", "4".repeat(32)),
                expected_event_count: None,
                actor: LearningActor::User,
                action: LearningFeedbackAction::Confirm,
                note: "Verified in the native dashboard.".into(),
                replacement_learning_id: None,
            },
        )
        .unwrap();
        let reviewed = load_agent_memory_dashboard(&project, binding.clone()).unwrap();
        assert_eq!(reviewed.review_inbox.total_matching, 0);
        assert_eq!(reviewed.resume.total_current_trusted_learnings, 1);

        let confirmed = read_learning(&project, &vault, &learning.learning.learning_id).unwrap();
        let corrected = correct_agent_learning_with_binding(
            &project,
            binding,
            &learning.learning.learning_id,
            AgentLearningCorrection {
                expected_event_count: confirmed.event_count,
                title: "Use the shared local projections".into(),
                guidance: "Read complete evidence through ley-core before changing memory.".into(),
                confidence_percent: 94,
                note: "The first claim did not mention bounded UI projections.".into(),
            },
        )
        .unwrap();
        assert_eq!(corrected.review_inbox.total_matching, 1);
        assert_eq!(corrected.resume.total_current_trusted_learnings, 0);
        assert_eq!(
            corrected.all_learnings.learnings[0].title,
            "Use the shared local projections"
        );
        let correction = read_learning(&project, &vault, &learning.learning.learning_id).unwrap();
        assert_eq!(correction.event_count, 3);
        assert_eq!(correction.evidence.len(), 1);
        assert_eq!(
            correction.evidence[0].note,
            "Captured in the dashboard implementation session."
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn desktop_learning_transition_helpers_write_and_refresh_native_authority() {
        use ley_core::{
            checkpoint_session, start_session, CheckpointInput, LearningEvidenceInput,
            LearningKind, LearningProvenance, ProposeLearningInput, SessionSource,
            StartSessionInput,
        };

        let root = std::env::temp_dir().join(format!(
            "ley-desktop-learning-transition-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = root.join("project");
        let vault = root.join("vault");
        let continuity = root.join("continuity");
        let _ = fs::remove_dir_all(&root);
        for directory in [&project, &vault, &continuity] {
            fs::create_dir_all(directory).unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&continuity, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(
            project.join("README.md"),
            "# Desktop transition learning\n\nKeep learning writes on continuity.",
        )
        .unwrap();
        let initialized = initialize_project(
            &project,
            Some("Desktop transition learning"),
            CaptureMode::Structured,
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let session = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: format!("req_{}", "1".repeat(32)),
                name: "Capture native learning evidence".into(),
                goal: "Exercise the production desktop learning transition helpers.".into(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &session.session.session_id,
            CheckpointInput {
                request_id: format!("req_{}", "2".repeat(32)),
                summary: "Captured one desktop learning source.".into(),
                plan: vec![],
                decisions: vec![],
                tasks: vec![],
                problems: vec![],
                touched_artifacts: vec!["README.md".into()],
                commands: vec![],
                verification: vec![],
                unresolved: vec![],
            },
        )
        .unwrap();
        let store = ContinuityStore::at(continuity.join("continuity.sqlite3"));
        let proposed = ley_core::propose_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            ProposeLearningInput {
                request_id: format!("req_{}", "3".repeat(32)),
                actor: LearningActor::Agent,
                kind: LearningKind::Procedure,
                title: "Use transition learning authority".into(),
                guidance: "Refresh the desktop from native learning continuity.".into(),
                confidence_percent: 82,
                provenance: LearningProvenance::AgentAuthored,
                evidence: vec![LearningEvidenceInput {
                    session_id: session.session.session_id,
                    record_id: checkpoint.session.checkpoints.last().unwrap().id.clone(),
                    note: "Desktop production-helper evidence.".into(),
                }],
            },
        )
        .unwrap();
        let binding = ProjectVaultBinding {
            project_id: initialized.identity.project_id,
            vault_path: vault.canonicalize().unwrap(),
            source: BindingSource::Override,
        };
        let access = AgentContinuityAccess::from_legacy_binding(binding);

        let corrected = correct_agent_learning_with_access_and_store(
            &project,
            &access,
            &store,
            &proposed.learning.learning_id,
            AgentLearningCorrection {
                expected_event_count: 1,
                title: "Use native transition learning authority".into(),
                guidance: "Read, correct, and review learning state from continuity.".into(),
                confidence_percent: 91,
                note: "Clarify the authority boundary.".into(),
            },
        )
        .unwrap();
        assert_eq!(corrected.review_inbox.total_matching, 1);
        assert_eq!(
            corrected.all_learnings.learnings[0].title,
            "Use native transition learning authority"
        );
        let current = ley_core::read_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            &proposed.learning.learning_id,
        )
        .unwrap();
        assert_eq!(current.event_count, 2);

        let reviewed = review_agent_learning_with_access_and_store(
            &project,
            &access,
            &store,
            &proposed.learning.learning_id,
            current.event_count,
            LearningFeedbackAction::Confirm,
            "Confirmed after reading the native correction.".into(),
        )
        .unwrap();
        assert_eq!(reviewed.review_inbox.total_matching, 0);
        assert_eq!(reviewed.resume.total_current_trusted_learnings, 1);
        let final_learning = ley_core::read_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            &proposed.learning.learning_id,
        )
        .unwrap();
        assert_eq!(final_learning.event_count, 3);
        assert_eq!(
            final_learning.trust_state,
            ley_core::LearningTrustState::Trusted
        );
        assert!(read_learning(&project, &vault, &proposed.learning.learning_id).is_err());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn desktop_learning_review_rejects_stale_visible_event_count() {
        use ley_core::{
            checkpoint_session, propose_learning, start_session, CheckpointInput,
            LearningEvidenceInput, LearningKind, LearningProvenance, ProposeLearningInput,
            SessionSource, StartSessionInput,
        };

        let root = std::env::temp_dir().join(format!(
            "ley-desktop-stale-learning-review-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = root.join("project");
        let vault = root.join("vault");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::write(
            project.join("README.md"),
            "# Stale review\n\nKeep desktop reviews version-bound.",
        )
        .unwrap();
        let initialized = initialize_project(
            &project,
            Some("Desktop stale review"),
            CaptureMode::Structured,
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();

        let session = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: format!("req_{}", "a".repeat(32)),
                name: "Capture review evidence".into(),
                goal: "Create one reviewable learning.".into(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &session.session.session_id,
            CheckpointInput {
                request_id: format!("req_{}", "b".repeat(32)),
                summary: "Captured one stable review source.".into(),
                plan: vec![],
                decisions: vec![],
                tasks: vec![],
                problems: vec![],
                touched_artifacts: vec!["README.md".into()],
                commands: vec![],
                verification: vec![],
                unresolved: vec![],
            },
        )
        .unwrap();
        let evidence = LearningEvidenceInput {
            session_id: session.session.session_id.clone(),
            record_id: checkpoint.session.checkpoints.last().unwrap().id.clone(),
            note: "Desktop stale-review evidence.".into(),
        };
        let proposed = propose_learning(
            &project,
            &vault,
            ProposeLearningInput {
                request_id: format!("req_{}", "c".repeat(32)),
                actor: LearningActor::Agent,
                kind: LearningKind::Procedure,
                title: "Inspect before review".into(),
                guidance: "Read the current claim before trusting it.".into(),
                confidence_percent: 80,
                provenance: LearningProvenance::AgentAuthored,
                evidence: vec![evidence.clone()],
            },
        )
        .unwrap();
        let binding = ProjectVaultBinding {
            project_id: initialized.identity.project_id,
            vault_path: vault.canonicalize().unwrap(),
            source: BindingSource::Override,
        };
        let visible_event_count = proposed.learning.event_count;
        assert_eq!(visible_event_count, 1);

        let concurrent = correct_learning(
            &project,
            &vault,
            &proposed.learning.learning_id,
            CorrectLearningInput {
                request_id: format!("req_{}", "d".repeat(32)),
                expected_event_count: Some(visible_event_count),
                actor: LearningActor::User,
                title: "Inspect the latest claim before review".into(),
                guidance: "Reload the latest learning text before trusting it.".into(),
                confidence_percent: 85,
                evidence: vec![evidence],
                note: "Another desktop window corrected the learning.".into(),
            },
        )
        .unwrap();
        assert_eq!(concurrent.learning.event_count, 2);

        let stale_error = match review_agent_learning_with_binding(
            &project,
            binding.clone(),
            &proposed.learning.learning_id,
            visible_event_count,
            LearningFeedbackAction::Confirm,
            "This stale inspector must not trust unseen text.".into(),
        ) {
            Ok(_) => panic!("stale desktop review unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(stale_error.to_string().contains("reload before saving"));

        let after_stale = read_learning(&project, &vault, &proposed.learning.learning_id).unwrap();
        assert_eq!(after_stale.event_count, concurrent.learning.event_count);
        assert_eq!(after_stale.title, concurrent.learning.title);
        assert_eq!(
            after_stale.trust_state,
            ley_core::LearningTrustState::ReviewRequired
        );

        let reviewed = review_agent_learning_with_binding(
            &project,
            binding,
            &proposed.learning.learning_id,
            after_stale.event_count,
            LearningFeedbackAction::Confirm,
            "Reloaded the corrected claim before confirming it.".into(),
        )
        .unwrap();
        assert_eq!(reviewed.review_inbox.total_matching, 0);
        assert_eq!(reviewed.resume.total_current_trusted_learnings, 1);

        let final_learning =
            read_learning(&project, &vault, &proposed.learning.learning_id).unwrap();
        assert_eq!(final_learning.event_count, 3);
        assert_eq!(final_learning.state, ley_core::LearningState::Verified);
        assert_eq!(
            final_learning.trust_state,
            ley_core::LearningTrustState::Trusted
        );

        fs::remove_dir_all(root).unwrap();
    }
}
