use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
#[cfg(test)]
use ley_core::initialize_project;
use ley_core::{
    correct_learning, diagnose_project, erase_project_memory, erase_session_memory,
    generate_learning_request_id, generate_request_id, ingest_project,
    ingest_project_with_expected_capture_plan, initialize_project_retiring_bootstrap,
    list_learning_contexts, list_sessions, preview_initial_capture, project_activity_view,
    project_artifact_inventory, project_graph_history, project_graph_view_filtered,
    project_memory_overview, project_resume_context, project_session_stats, read_learning,
    read_learning_context, read_project_cited_evidence, read_project_cited_media,
    read_project_graph_evidence, read_session_context, read_session_turns_context, rename_session,
    review_learning, search_observed_projects, search_project_memory, update_capture_mode,
    ArtifactMediaType, BindingRegistry, BindingSource, CaptureFile, CaptureMode, CapturePolicy,
    CorrectLearningInput, CrossProjectSearch, EraseSessionMemoryInput, EvidenceExcerpt,
    GraphCitation, IngestionResult, LearningActor, LearningContextPack, LearningEvidenceInput,
    LearningFeedbackAction, LearningList, LearningListScope, LeyCoreError, MemoryOverview,
    ProjectActivityView, ProjectArtifactInventory, ProjectCatalog, ProjectDiagnostic,
    ProjectGraphFilters, ProjectGraphHistory, ProjectGraphView, ProjectMemorySearch,
    ProjectMemorySearchLimits, ProjectProblemScope, ProjectResumePack, ProjectVaultBinding,
    RenameSessionInput, ReviewLearningInput, RevisionCompatibility, SessionContextPack,
    SessionMemoryErasure, SessionSummary, SessionTurnsContextPack, SpecificationAuthorityList,
    SpecificationRegistry, DEFAULT_ARTIFACT_RESULTS, DEFAULT_CROSS_PROJECT_SEARCH_RESULTS,
    DEFAULT_GRAPH_HISTORY_RESULTS, DEFAULT_GRAPH_VIEW_EDGES, DEFAULT_GRAPH_VIEW_NODES,
    DEFAULT_LEARNING_CONTEXT_ARTIFACTS, DEFAULT_LEARNING_CONTEXT_CHARACTERS,
    DEFAULT_LEARNING_CONTEXT_EVIDENCE, DEFAULT_LEARNING_CONTEXT_HISTORY,
    DEFAULT_PROJECT_ACTIVITY_RESULTS, DEFAULT_PROJECT_CATALOG_RESULTS,
    DEFAULT_PROJECT_MEMORY_SEARCH_RESULTS, DEFAULT_PROJECT_MEMORY_SEARCH_TOKENS,
    DEFAULT_RESUME_CHARACTERS, DEFAULT_RESUME_LEARNINGS, DEFAULT_RESUME_SESSIONS,
    DEFAULT_SESSION_CONTEXT_CHARACTERS, DEFAULT_SESSION_CONTEXT_CHECKPOINTS,
    DEFAULT_SESSION_TURN_CHARACTERS, DEFAULT_SESSION_TURN_RESULTS, MAX_LEARNING_LIST_RESULTS,
    MAX_MEDIA_EVIDENCE_BYTES,
};
use ley_core::{
    semantic_model_status as local_semantic_model_status, supported_semantic_model,
    SemanticModelDescriptor, SemanticModelInstallation, SemanticModelStatus,
};
use ley_semantic_installer::install_supported_semantic_model;
use notify::{
    event::{EventKind, ModifyKind, RenameMode},
    Config, Event, RecommendedWatcher, RecursiveMode, Watcher,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, State};
use walkdir::{DirEntry, WalkDir};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VaultFile {
    path: String,
    content: String,
    created_at: u64,
    updated_at: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CanvasFile {
    path: String,
    content: String,
    updated_at: u64,
}

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VaultPathChange {
    kind: &'static str,
    /// Vault-relative destination/current path.
    path: String,
    /// Vault-relative previous path, only for a paired notify rename event.
    from: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VaultChange {
    /// Kept for existing consumers which only need to know affected paths.
    paths: Vec<String>,
    changes: Vec<VaultPathChange>,
    /// An oversized native event was intentionally bounded; callers rescan.
    full_rescan: bool,
}

struct ActiveVaultWatcher {
    root: PathBuf,
    _watcher: RecommendedWatcher,
}

#[derive(Default)]
struct VaultWatcherState(Mutex<Option<ActiveVaultWatcher>>);

#[derive(Clone, PartialEq)]
enum FileState {
    Missing,
    Present {
        len: u64,
        modified: Option<SystemTime>,
    },
}

struct SuppressedChange {
    created: Instant,
    state: FileState,
}

static SUPPRESSED_CHANGES: OnceLock<Mutex<HashMap<PathBuf, SuppressedChange>>> = OnceLock::new();

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentMemoryBinding {
    project_id: String,
    vault_name: String,
    source: BindingSource,
}

impl From<ProjectVaultBinding> for AgentMemoryBinding {
    fn from(binding: ProjectVaultBinding) -> Self {
        let vault_name = binding
            .vault_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Ley vault")
            .to_owned();
        Self {
            project_id: binding.project_id,
            vault_name,
            source: binding.source,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentMemoryDashboard {
    binding: AgentMemoryBinding,
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
        preview: AgentInitialCapturePreview,
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
        binding: AgentMemoryBinding,
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
    store_raw_transcripts: bool,
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
        binding: binding.into(),
        overview,
        resume,
        sessions,
        review_inbox,
        all_learnings,
    })
}

fn resolved_agent_binding(project_path: &Path) -> Result<ProjectVaultBinding, LeyCoreError> {
    BindingRegistry::system_default()?.resolve(project_path, None)
}

fn verify_open_vault_binding(
    binding: &ProjectVaultBinding,
    open_vault_path: &str,
) -> Result<(), String> {
    let open_vault = canonical_vault(open_vault_path)?;
    if binding.vault_path == open_vault {
        return Ok(());
    }
    let bound_name = binding
        .vault_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("the bound vault");
    let open_name = open_vault
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("the open vault");
    Err(format!(
        "This project’s Agent Memory belongs to “{bound_name}”, but notes are open in “{open_name}”. Open the bound vault before creating a linked note."
    ))
}

fn agent_project_catalog_item(
    observed: ley_core::ObservedProject,
    registry: &BindingRegistry,
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
    agent_project_catalog_item_for_diagnostic(observed, diagnostic, registry)
}

fn agent_project_catalog_item_for_diagnostic(
    observed: ley_core::ObservedProject,
    diagnostic: ProjectDiagnostic,
    registry: &BindingRegistry,
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
    let binding = match registry.resolve_observed(&diagnostic) {
        Ok(binding) => binding,
        Err(LeyCoreError::VaultNotBound(_)) => {
            item.state = AgentProjectCatalogState::Unbound;
            item.status_detail = "Choose a filesystem vault before capturing memory.".to_owned();
            return item;
        }
        Err(LeyCoreError::BoundVaultUnavailable { path, .. }) => {
            item.state = AgentProjectCatalogState::VaultUnavailable;
            item.vault_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned);
            item.status_detail =
                "The bound vault moved or is currently unavailable. Reconnect it explicitly."
                    .to_owned();
            return item;
        }
        Err(error) => {
            item.state = AgentProjectCatalogState::MemoryError;
            item.status_detail = error.to_string();
            return item;
        }
    };
    item.vault_name = Some(
        binding
            .vault_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Ley vault")
            .to_owned(),
    );
    let overview = match project_memory_overview(&diagnostic.root, &binding.vault_path) {
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
    let sessions = match project_session_stats(&diagnostic.root, &binding.vault_path) {
        Ok(sessions) => sessions,
        Err(error) => {
            item.state = AgentProjectCatalogState::MemoryError;
            item.status_detail = error.to_string();
            return item;
        }
    };
    let review = match list_learning_contexts(
        &diagnostic.root,
        &binding.vault_path,
        LearningListScope::NeedsReview,
        MAX_LEARNING_LIST_RESULTS,
    ) {
        Ok(review) => review,
        Err(error) => {
            item.state = AgentProjectCatalogState::MemoryError;
            item.status_detail = error.to_string();
            return item;
        }
    };
    item.files = Some(overview.files);
    item.graph_nodes = Some(overview.graph_nodes);
    item.sessions = Some(sessions.total_sessions);
    item.active_sessions = Some(sessions.active_sessions + sessions.paused_sessions);
    item.review_items = Some(review.total_matching);
    item.freshness = Some(overview.freshness.to_owned());
    item
}

fn load_agent_project_catalog_from(
    catalog: &ProjectCatalog,
    registry: &BindingRegistry,
) -> Result<AgentProjectCatalogView, LeyCoreError> {
    let observed = catalog.list(DEFAULT_PROJECT_CATALOG_RESULTS)?;
    let projects = observed
        .projects
        .into_iter()
        .map(|project| agent_project_catalog_item(project, registry))
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
    load_agent_project_catalog_from(
        &ProjectCatalog::system_default()?,
        &BindingRegistry::system_default()?,
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
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|_| {
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
                store_raw_transcripts: diagnostic.capture.store_raw_transcripts,
                ignore_file_present: diagnostic.ignore_file_present,
                capture_fingerprint: preview.capture_fingerprint,
                eligible_files: preview.files.len(),
                eligible_bytes: preview.included_bytes,
                skipped_oversized: preview.skipped_oversized.len(),
                skipped_total_limit: preview.skipped_total_limit.len(),
                skipped_symlinks: preview.skipped_symlinks.len(),
                privacy_notice: "Preview reads file metadata only. Applying a mode refreshes the redacted local snapshot; Ley never uploads it.",
            })
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_agent_capture_mode(
    project_path: String,
    expected_mode: CaptureMode,
    mode: CaptureMode,
    full_evidence_consent: bool,
) -> Result<AgentMemoryDashboard, String> {
    let binding =
        resolved_agent_binding(Path::new(&project_path)).map_err(|error| error.to_string())?;
    update_capture_mode(
        &project_path,
        &binding.project_id,
        expected_mode,
        mode,
        full_evidence_consent,
    )
    .map_err(|error| error.to_string())?;
    ingest_project(&project_path, &binding.vault_path).map_err(|error| {
        format!(
            "The capture mode was saved, but the local snapshot refresh failed: {error}. Retry Refresh snapshot."
        )
    })?;
    load_agent_memory_dashboard(Path::new(&project_path), binding)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn erase_agent_project_memory(project_path: String) -> Result<AgentProjectInspection, String> {
    let registry = BindingRegistry::system_default().map_err(|error| error.to_string())?;
    erase_agent_project_memory_with_registry(Path::new(&project_path), &registry)
}

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SemanticModelSetup {
    status: SemanticModelStatus,
    model: SemanticModelDescriptor,
    total_bytes: u64,
}

/// Reports whether the pinned local semantic-retrieval model is ready without using the network.
#[tauri::command]
fn semantic_model_status() -> SemanticModelSetup {
    let model = supported_semantic_model();
    let total_bytes = model.files.iter().map(|file| file.bytes).sum();
    SemanticModelSetup {
        status: local_semantic_model_status(),
        model,
        total_bytes,
    }
}

/// Explicitly downloads and installs Ley's pinned semantic-retrieval model off the UI thread.
#[tauri::command]
async fn install_semantic_model() -> Result<SemanticModelInstallation, String> {
    tauri::async_runtime::spawn_blocking(install_supported_semantic_model)
        .await
        .map_err(|_| "Semantic model installation was interrupted.".to_owned())?
        .map_err(|error| error.to_string())
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
        let binding = resolved_agent_binding(Path::new(&project_path))?;
        search_project_memory(
            &project_path,
            &binding.vault_path,
            &query,
            ProjectMemorySearchLimits {
                max_results: DEFAULT_PROJECT_MEMORY_SEARCH_RESULTS,
                max_tokens: DEFAULT_PROJECT_MEMORY_SEARCH_TOKENS,
            },
            revision_filter,
        )
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
    inspect_initialized_agent_project_with_registry(diagnostic, &registry)
}

fn inspect_initialized_agent_project_with_registry(
    diagnostic: ProjectDiagnostic,
    registry: &BindingRegistry,
) -> Result<AgentProjectInspection, String> {
    let binding = match registry.resolve(&diagnostic.root, None) {
        Ok(binding) => binding,
        Err(LeyCoreError::VaultNotBound(_)) => {
            let preview =
                agent_existing_capture_preview(&diagnostic).map_err(|error| error.to_string())?;
            return Ok(AgentProjectInspection::Unbound {
                project_id: diagnostic.identity.project_id,
                project_name: diagnostic.identity.name,
                capture_mode: diagnostic.capture.mode,
                preview,
            });
        }
        Err(LeyCoreError::BoundVaultUnavailable { path, .. }) => {
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
    match load_agent_memory_dashboard(&diagnostic.root, binding.clone()) {
        Ok(dashboard) => Ok(AgentProjectInspection::Ready {
            dashboard: Box::new(dashboard),
        }),
        Err(LeyCoreError::ProjectMemoryUnavailable(_)) => {
            Ok(AgentProjectInspection::NeedsCapture {
                project_id: diagnostic.identity.project_id,
                project_name: diagnostic.identity.name,
                capture_mode: diagnostic.capture.mode,
                binding: binding.into(),
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
        "This preview inspects capture paths and file metadata only. It creates no .ley metadata, vault binding, or Agent Memory until you approve initialization.",
    )
}

fn agent_existing_capture_preview(
    diagnostic: &ProjectDiagnostic,
) -> Result<AgentInitialCapturePreview, LeyCoreError> {
    let preview = ley_core::preview_capture(&diagnostic.root)?;
    agent_capture_preview_summary(
        &diagnostic.root,
        &diagnostic.capture,
        preview.mode,
        preview.capture_fingerprint,
        preview.plan_fingerprint,
        preview.files,
        preview.included_bytes,
        preview.skipped_oversized,
        preview.skipped_total_limit,
        preview.skipped_symlinks,
        "This preview reads capture paths and file metadata only. The project is initialized, but Ley will not bind or create Agent Memory in the selected vault until you approve this capture plan.",
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

#[tauri::command]
fn verify_agent_project_note_vault(
    project_path: String,
    open_vault_path: String,
) -> Result<(), String> {
    let binding =
        resolved_agent_binding(Path::new(&project_path)).map_err(|error| error.to_string())?;
    verify_open_vault_binding(&binding, &open_vault_path)
}

fn read_project_specifications_with_registry(
    project_path: &Path,
    binding: &ProjectVaultBinding,
    registry: &SpecificationRegistry,
) -> Result<SpecificationAuthorityList, String> {
    registry
        .list(project_path, &binding.vault_path)
        .map_err(|error| error.to_string())
}

fn approve_project_specification_with_registry(
    project_path: &Path,
    binding: &ProjectVaultBinding,
    open_vault_path: &str,
    registry: &SpecificationRegistry,
    specification_id: &str,
    relative_path: &str,
) -> Result<SpecificationAuthorityList, String> {
    verify_open_vault_binding(binding, open_vault_path)?;
    registry
        .approve(
            project_path,
            &binding.vault_path,
            specification_id,
            relative_path,
        )
        .map_err(|error| error.to_string())?;
    read_project_specifications_with_registry(project_path, binding, registry)
}

fn revoke_project_specification_with_registry(
    project_path: &Path,
    binding: &ProjectVaultBinding,
    open_vault_path: &str,
    registry: &SpecificationRegistry,
    specification_id: &str,
) -> Result<SpecificationAuthorityList, String> {
    verify_open_vault_binding(binding, open_vault_path)?;
    registry
        .revoke(project_path, specification_id)
        .map_err(|error| error.to_string())?;
    read_project_specifications_with_registry(project_path, binding, registry)
}

#[tauri::command]
fn read_agent_project_specifications(
    project_path: String,
) -> Result<SpecificationAuthorityList, String> {
    let binding =
        resolved_agent_binding(Path::new(&project_path)).map_err(|error| error.to_string())?;
    let registry = SpecificationRegistry::system_default().map_err(|error| error.to_string())?;
    read_project_specifications_with_registry(Path::new(&project_path), &binding, &registry)
}

#[tauri::command]
fn approve_agent_project_specification(
    project_path: String,
    open_vault_path: String,
    specification_id: String,
    relative_path: String,
) -> Result<SpecificationAuthorityList, String> {
    let binding =
        resolved_agent_binding(Path::new(&project_path)).map_err(|error| error.to_string())?;
    let registry = SpecificationRegistry::system_default().map_err(|error| error.to_string())?;
    approve_project_specification_with_registry(
        Path::new(&project_path),
        &binding,
        &open_vault_path,
        &registry,
        &specification_id,
        &relative_path,
    )
}

#[tauri::command]
fn revoke_agent_project_specification(
    project_path: String,
    open_vault_path: String,
    specification_id: String,
) -> Result<SpecificationAuthorityList, String> {
    let binding =
        resolved_agent_binding(Path::new(&project_path)).map_err(|error| error.to_string())?;
    let registry = SpecificationRegistry::system_default().map_err(|error| error.to_string())?;
    revoke_project_specification_with_registry(
        Path::new(&project_path),
        &binding,
        &open_vault_path,
        &registry,
        &specification_id,
    )
}

#[tauri::command]
fn initialize_agent_project(
    project_path: String,
    vault_path: String,
    expected_approval_fingerprint: String,
) -> Result<AgentMemoryDashboard, String> {
    let reviewed = agent_initial_capture_preview(Path::new(&project_path))
        .map_err(|error| error.to_string())?;
    if reviewed.approval_fingerprint != expected_approval_fingerprint {
        return Err(LeyCoreError::CapturePreviewChanged.to_string());
    }
    let initialization =
        initialize_project_retiring_bootstrap(&project_path, None, CaptureMode::Structured)
            .map_err(|error| error.to_string())?;
    if !initialization.created {
        return Err(LeyCoreError::CapturePreviewChanged.to_string());
    }
    ingest_project_with_expected_capture_plan(
        &initialization.root,
        &vault_path,
        &reviewed.plan_fingerprint,
    )
    .map_err(|error| error.to_string())?;
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.bind(&initialization.root, &vault_path))
        .map_err(|error| error.to_string())?;
    load_agent_memory_dashboard(&initialization.root, binding).map_err(|error| error.to_string())
}

#[tauri::command]
fn connect_agent_project(
    project_path: String,
    vault_path: String,
    expected_approval_fingerprint: Option<String>,
) -> Result<AgentMemoryDashboard, String> {
    let registry = BindingRegistry::system_default().map_err(|error| error.to_string())?;
    connect_agent_project_with_registry(
        Path::new(&project_path),
        Path::new(&vault_path),
        expected_approval_fingerprint.as_deref(),
        &registry,
    )
    .map_err(|error| error.to_string())
}

fn connect_agent_project_with_registry(
    project_path: &Path,
    vault_path: &Path,
    expected_approval_fingerprint: Option<&str>,
    registry: &BindingRegistry,
) -> Result<AgentMemoryDashboard, LeyCoreError> {
    let diagnostic = diagnose_project(project_path)?;
    match registry.resolve_observed(&diagnostic) {
        Err(LeyCoreError::VaultNotBound(_)) => {
            let reviewed = agent_existing_capture_preview(&diagnostic)?;
            if expected_approval_fingerprint != Some(reviewed.approval_fingerprint.as_str()) {
                return Err(LeyCoreError::CapturePreviewChanged);
            }
            ingest_project_with_expected_capture_plan(
                &diagnostic.root,
                vault_path,
                &reviewed.plan_fingerprint,
            )?;
        }
        Err(LeyCoreError::BoundVaultUnavailable { .. }) | Ok(_) => {
            ingest_project(&diagnostic.root, vault_path)?;
        }
        Err(error) => return Err(error),
    }
    let binding = registry.bind(&diagnostic.root, vault_path)?;
    load_agent_memory_dashboard(&diagnostic.root, binding)
}

#[tauri::command]
fn refresh_agent_project(project_path: String) -> Result<AgentMemoryDashboard, String> {
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|binding| {
            ingest_project(&project_path, &binding.vault_path)?;
            load_agent_memory_dashboard(Path::new(&project_path), binding)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_learning(
    project_path: String,
    learning_id: String,
) -> Result<LearningContextPack, String> {
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|binding| {
            read_learning_context(
                &project_path,
                &binding.vault_path,
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
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|binding| {
            read_session_context(
                &project_path,
                &binding.vault_path,
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
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|binding| {
            read_session_turns_context(
                &project_path,
                &binding.vault_path,
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
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|binding| {
            rename_session(
                &project_path,
                &binding.vault_path,
                &session_id,
                RenameSessionInput {
                    request_id: generate_request_id(),
                    expected_event_count: Some(expected_event_count),
                    name,
                    note,
                },
            )?;
            load_agent_memory_dashboard(Path::new(&project_path), binding)
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
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|binding| {
            let erasure = erase_session_memory(
                &project_path,
                &binding.vault_path,
                &session_id,
                EraseSessionMemoryInput {
                    expected_event_count,
                    expected_name,
                },
            )?;
            let dashboard = load_agent_memory_dashboard(Path::new(&project_path), binding)?;
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
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|binding| {
            review_agent_learning_with_binding(
                Path::new(&project_path),
                binding,
                &learning_id,
                expected_event_count,
                action,
                note,
            )
        })
        .map_err(|error| error.to_string())
}

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
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|binding| {
            correct_agent_learning_with_binding(
                Path::new(&project_path),
                binding,
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

#[tauri::command]
fn bind_agent_project(
    project_path: String,
    vault_path: String,
) -> Result<ProjectVaultBinding, String> {
    BindingRegistry::system_default()
        .and_then(|registry| registry.bind(project_path, vault_path))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn resolve_agent_project_vault(
    project_path: String,
    vault_override: Option<String>,
) -> Result<ProjectVaultBinding, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(project_path, override_path))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn unbind_agent_project(project_path: String) -> Result<Option<ProjectVaultBinding>, String> {
    BindingRegistry::system_default()
        .and_then(|registry| registry.unbind(project_path))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn ingest_agent_project(
    project_path: String,
    vault_override: Option<String>,
) -> Result<IngestionResult, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(&project_path, override_path))
        .map_err(|error| error.to_string())?;
    ingest_project(project_path, binding.vault_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_artifacts(
    project_path: String,
    vault_override: Option<String>,
    query: Option<String>,
    max_results: Option<usize>,
) -> Result<ProjectArtifactInventory, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(&project_path, override_path))
        .map_err(|error| error.to_string())?;
    project_artifact_inventory(
        project_path,
        binding.vault_path,
        query.as_deref().unwrap_or_default(),
        max_results.unwrap_or(DEFAULT_ARTIFACT_RESULTS),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_project_graph_view(
    project_path: String,
    vault_override: Option<String>,
    graph_snapshot_id: Option<String>,
    query: Option<String>,
    max_nodes: Option<usize>,
    max_edges: Option<usize>,
    filters: Option<ProjectGraphFilters>,
) -> Result<ProjectGraphView, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(&project_path, override_path))
        .map_err(|error| error.to_string())?;
    project_graph_view_filtered(
        project_path,
        binding.vault_path,
        graph_snapshot_id.as_deref(),
        query.as_deref().unwrap_or_default(),
        max_nodes.unwrap_or(DEFAULT_GRAPH_VIEW_NODES),
        max_edges.unwrap_or(DEFAULT_GRAPH_VIEW_EDGES),
        &filters.unwrap_or_default(),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_project_graph_history(
    project_path: String,
    vault_override: Option<String>,
    max_results: Option<usize>,
) -> Result<ProjectGraphHistory, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(&project_path, override_path))
        .map_err(|error| error.to_string())?;
    project_graph_history(
        project_path,
        binding.vault_path,
        max_results.unwrap_or(DEFAULT_GRAPH_HISTORY_RESULTS),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_agent_project_graph_evidence(
    project_path: String,
    vault_override: Option<String>,
    graph_snapshot_id: String,
    citation: GraphCitation,
    context_lines: Option<u64>,
    max_characters: Option<usize>,
) -> Result<EvidenceExcerpt, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(&project_path, override_path))
        .map_err(|error| error.to_string())?;
    read_project_graph_evidence(
        project_path,
        binding.vault_path,
        &graph_snapshot_id,
        &citation,
        context_lines.unwrap_or(3),
        max_characters.unwrap_or(8_000),
    )
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
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(&project_path, override_path))
        .map_err(|error| error.to_string())?;
    read_project_cited_evidence(
        project_path,
        binding.vault_path,
        &citation,
        context_lines.unwrap_or(3),
        max_characters.unwrap_or(8_000),
    )
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
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(&project_path, override_path))
        .map_err(|error| error.to_string())?;
    let media = read_project_cited_media(
        project_path,
        binding.vault_path,
        &artifact_path,
        &artifact_snapshot_id,
        &content_hash,
        max_bytes.unwrap_or(MAX_MEDIA_EVIDENCE_BYTES),
    )
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
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(&project_path, override_path))
        .map_err(|error| error.to_string())?;
    project_activity_view(
        project_path,
        binding.vault_path,
        query.as_deref().unwrap_or_default(),
        problem_scope.unwrap_or(ProjectProblemScope::All),
        max_results.unwrap_or(DEFAULT_PROJECT_ACTIVITY_RESULTS),
    )
    .map_err(|error| error.to_string())
}

fn current_file_state(path: &Path) -> FileState {
    match fs::metadata(path) {
        Ok(metadata) => FileState::Present {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        },
        Err(_) => FileState::Missing,
    }
}

fn suppress_current_change(path: &Path) {
    let changes = SUPPRESSED_CHANGES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut changes) = changes.lock() {
        changes.insert(
            path.to_path_buf(),
            SuppressedChange {
                created: Instant::now(),
                state: current_file_state(path),
            },
        );
    }
}

fn is_suppressed(path: &Path) -> bool {
    let changes = SUPPRESSED_CHANGES.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut changes) = changes.lock() else {
        return false;
    };
    changes.retain(|_, change| change.created.elapsed() < Duration::from_millis(750));
    let Some(expected) = changes.get(path) else {
        return false;
    };
    if expected.state == current_file_state(path) {
        true
    } else {
        changes.remove(path);
        false
    }
}

fn unix_millis(value: Result<SystemTime, std::io::Error>) -> u64 {
    value
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn canonical_vault(vault_path: &str) -> Result<PathBuf, String> {
    let root =
        fs::canonicalize(vault_path).map_err(|error| format!("Cannot open vault: {error}"))?;
    if !root.is_dir() {
        return Err("The selected vault is not a folder".into());
    }
    Ok(root)
}

fn safe_relative(relative_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative_path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err("Vault paths must be relative".into());
    }
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Vault path contains an unsafe segment".into());
    }
    Ok(path.to_path_buf())
}

fn markdown_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = safe_relative(relative_path)?;
    if relative.extension().and_then(|part| part.to_str()) != Some("md") {
        return Err("Ley can only mutate Markdown notes".into());
    }
    Ok(root.join(relative))
}

fn attachment_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = safe_relative(relative_path)?;
    if relative
        .components()
        .next()
        .and_then(|part| part.as_os_str().to_str())
        != Some("attachments")
    {
        return Err("Attachments must be stored inside the attachments folder".into());
    }
    let allowed = [
        "png", "jpg", "jpeg", "gif", "webp", "pdf", "mp3", "wav", "mp4", "webm",
    ];
    let extension = relative
        .extension()
        .and_then(|part| part.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if !allowed.contains(&extension.as_str()) {
        return Err(format!("Unsupported attachment type: {extension}"));
    }
    Ok(root.join(relative))
}

fn canvas_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = safe_relative(relative_path)?;
    if relative
        .components()
        .next()
        .and_then(|part| part.as_os_str().to_str())
        != Some("canvases")
        || relative.extension().and_then(|part| part.to_str()) != Some("canvas")
    {
        return Err("Canvas files must use canvases/*.canvas".into());
    }
    Ok(root.join(relative))
}

fn visible_entry(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if entry.depth() == 0 {
        return true;
    }
    !name.starts_with('.') && name != "node_modules"
}

fn relevant_change_path(root: &Path, path: &Path) -> Option<String> {
    if is_suppressed(path) {
        return None;
    }
    let relative = path.strip_prefix(root).ok()?;
    if relative.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|part| part.starts_with('.') || part == "node_modules")
    }) {
        return None;
    }
    let extension = relative.extension()?.to_str()?.to_lowercase();
    if extension != "md" && extension != "canvas" {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn visible_vault_path(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    !relative.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|part| part.starts_with('.') || part == "node_modules")
    })
}

const MAX_WATCH_CHANGES: usize = 64;

fn vault_change_from_event(root: &Path, event: Event) -> Option<VaultChange> {
    let relevant_paths: Vec<String> = event
        .paths
        .iter()
        .filter_map(|path| relevant_change_path(root, path))
        .collect();
    let mut paths = relevant_paths.clone();
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        // Recursive watchers commonly report only the directory path when a
        // folder containing notes is renamed or removed. Its extension cannot
        // identify the affected Markdown children, so request one bounded full
        // scan rather than silently leaving stale paths in the projection.
        let structural_directory_change = matches!(
            event.kind,
            EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
        ) && event
            .paths
            .iter()
            .any(|path| path.extension().is_none() && visible_vault_path(root, path));
        if structural_directory_change {
            return Some(VaultChange {
                paths: Vec::new(),
                changes: Vec::new(),
                full_rescan: true,
            });
        }
        return None;
    }
    if paths.len() > MAX_WATCH_CHANGES {
        return Some(VaultChange {
            paths: Vec::new(),
            changes: Vec::new(),
            full_rescan: true,
        });
    }

    // Notify guarantees an old/new pair only for a single Name event that
    // carries exactly both in-root Markdown/Canvas paths. Some backends emit
    // separate From/To notifications instead; those deliberately degrade to
    // ordinary changes so the frontend's unique-hash scan can decide safely.
    if matches!(
        event.kind,
        EventKind::Modify(ModifyKind::Name(RenameMode::Both))
    ) && event.paths.len() == 2
        && relevant_paths.len() == 2
        && paths.len() == 2
    {
        return Some(VaultChange {
            paths: paths.clone(),
            changes: vec![VaultPathChange {
                kind: "rename",
                from: Some(relevant_paths[0].clone()),
                path: relevant_paths[1].clone(),
            }],
            full_rescan: false,
        });
    }

    let kind = match event.kind {
        EventKind::Create(_) => "create",
        EventKind::Remove(_) => "remove",
        _ => "modify",
    };
    Some(VaultChange {
        changes: paths
            .iter()
            .map(|path| VaultPathChange {
                kind,
                path: path.clone(),
                from: None,
            })
            .collect(),
        paths,
        full_rescan: false,
    })
}

fn build_vault_event_watcher<F>(
    root: PathBuf,
    mut on_change: F,
) -> Result<RecommendedWatcher, String>
where
    F: FnMut(VaultChange) + Send + 'static,
{
    let event_root = root.clone();
    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<Event>| {
            let Ok(event) = result else { return };
            if let Some(change) = vault_change_from_event(&event_root, event) {
                on_change(change);
            }
        },
        Config::default(),
    )
    .map_err(|error| format!("Cannot create vault watcher: {error}"))?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|error| format!("Cannot watch vault: {error}"))?;
    Ok(watcher)
}

#[tauri::command]
fn watch_vault(
    app: AppHandle,
    state: State<'_, VaultWatcherState>,
    vault_path: String,
) -> Result<(), String> {
    let root = canonical_vault(&vault_path)?;
    let watcher = build_vault_event_watcher(root.clone(), move |change| {
        let _ = app.emit("ley-vault-changed", change);
    })?;
    let mut active = state
        .0
        .lock()
        .map_err(|_| "Vault watcher lock is unavailable".to_string())?;
    *active = Some(ActiveVaultWatcher {
        root,
        _watcher: watcher,
    });
    Ok(())
}

#[tauri::command]
fn stop_watching_vault(
    state: State<'_, VaultWatcherState>,
    vault_path: String,
) -> Result<(), String> {
    let mut active = state
        .0
        .lock()
        .map_err(|_| "Vault watcher lock is unavailable".to_string())?;
    let requested = fs::canonicalize(&vault_path).unwrap_or_else(|_| PathBuf::from(vault_path));
    if active
        .as_ref()
        .is_some_and(|watcher| watcher.root == requested)
    {
        *active = None;
    }
    Ok(())
}

#[tauri::command]
fn scan_vault(vault_path: String) -> Result<Vec<VaultFile>, String> {
    let root = canonical_vault(&vault_path)?;
    let mut files = Vec::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(visible_entry)
    {
        let entry = entry.map_err(|error| format!("Failed to scan vault: {error}"))?;
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(|part| part.to_str()) != Some("md")
        {
            continue;
        }
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| "A scanned file escaped the vault root")?
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Cannot inspect {relative}: {error}"))?;
        let content =
            fs::read_to_string(path).map_err(|error| format!("Cannot read {relative}: {error}"))?;
        files.push(VaultFile {
            path: relative,
            content,
            created_at: unix_millis(metadata.created()),
            updated_at: unix_millis(metadata.modified()),
        });
    }

    files.sort_by_cached_key(|file| file.path.to_lowercase());
    Ok(files)
}

#[tauri::command]
fn scan_trashed_vault_files(vault_path: String) -> Result<Vec<VaultFile>, String> {
    let root = canonical_vault(&vault_path)?;
    let trash = root.join(".trash");
    if !trash.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(&trash).follow_links(false) {
        let entry = entry.map_err(|error| format!("Failed to scan trash: {error}"))?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|part| part.to_str()) != Some("md")
        {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&root)
            .map_err(|_| "A trashed file escaped the vault root")?
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Cannot inspect {relative}: {error}"))?;
        let content = fs::read_to_string(entry.path())
            .map_err(|error| format!("Cannot read {relative}: {error}"))?;
        files.push(VaultFile {
            path: relative,
            content,
            created_at: unix_millis(metadata.created()),
            updated_at: unix_millis(metadata.modified()),
        });
    }

    files.sort_by_cached_key(|file| file.path.to_lowercase());
    Ok(files)
}

#[tauri::command]
fn scan_canvases(vault_path: String) -> Result<Vec<CanvasFile>, String> {
    let root = canonical_vault(&vault_path)?;
    let canvas_root = root.join("canvases");
    if !canvas_root.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(&canvas_root).follow_links(false) {
        let entry = entry.map_err(|error| format!("Failed to scan canvases: {error}"))?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|part| part.to_str()) != Some("canvas")
        {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&root)
            .map_err(|_| "A canvas escaped the vault root")?
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Cannot inspect {relative}: {error}"))?;
        let content = fs::read_to_string(entry.path())
            .map_err(|error| format!("Cannot read {relative}: {error}"))?;
        files.push(CanvasFile {
            path: relative,
            content,
            updated_at: unix_millis(metadata.modified()),
        });
    }
    files.sort_by_cached_key(|file| file.path.to_lowercase());
    Ok(files)
}

#[tauri::command]
fn write_canvas_file(
    vault_path: String,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    serde_json::from_str::<serde_json::Value>(&content)
        .map_err(|error| format!("Canvas JSON is invalid: {error}"))?;
    let root = canonical_vault(&vault_path)?;
    let target = canvas_path(&root, &relative_path)?;
    let parent = target.parent().ok_or("The canvas has no parent folder")?;
    fs::create_dir_all(parent).map_err(|error| format!("Cannot create canvas folder: {error}"))?;
    let temp = parent.join(format!(
        ".{}.ley-write",
        target.file_name().unwrap_or_default().to_string_lossy()
    ));
    let mut file =
        fs::File::create(&temp).map_err(|error| format!("Cannot stage canvas: {error}"))?;
    file.write_all(content.as_bytes())
        .map_err(|error| format!("Cannot write canvas: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Cannot flush canvas: {error}"))?;
    fs::rename(temp, &target).map_err(|error| format!("Cannot replace canvas: {error}"))?;
    suppress_current_change(&target);
    Ok(())
}

#[tauri::command]
fn trash_canvas_file(vault_path: String, relative_path: String) -> Result<(), String> {
    let root = canonical_vault(&vault_path)?;
    let source = canvas_path(&root, &relative_path)?;
    if !source.exists() {
        return Ok(());
    }
    let trash = root.join(".trash");
    fs::create_dir_all(&trash).map_err(|error| format!("Cannot create .trash: {error}"))?;
    let original = source
        .file_name()
        .ok_or("The canvas has no filename")?
        .to_string_lossy();
    let mut candidate = trash.join(original.as_ref());
    let mut suffix = 2;
    while candidate.exists() {
        let stem = Path::new(original.as_ref())
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        candidate = trash.join(format!("{stem} {suffix}.canvas"));
        suffix += 1;
    }
    fs::rename(&source, &candidate)
        .map_err(|error| format!("Cannot move canvas to .trash: {error}"))?;
    suppress_current_change(&source);
    suppress_current_change(&candidate);
    Ok(())
}

#[tauri::command]
fn write_vault_file(
    vault_path: String,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    let root = canonical_vault(&vault_path)?;
    let target = markdown_path(&root, &relative_path)?;
    let parent = target.parent().ok_or("The note has no parent folder")?;
    fs::create_dir_all(parent).map_err(|error| format!("Cannot create note folder: {error}"))?;

    let temp_name = format!(
        ".{}.ley-write",
        target.file_name().unwrap_or_default().to_string_lossy()
    );
    let temp = parent.join(temp_name);
    let mut file =
        fs::File::create(&temp).map_err(|error| format!("Cannot stage note: {error}"))?;
    file.write_all(content.as_bytes())
        .map_err(|error| format!("Cannot write note: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Cannot flush note: {error}"))?;
    fs::rename(&temp, &target).map_err(|error| format!("Cannot replace note: {error}"))?;
    suppress_current_change(&target);
    Ok(())
}

#[tauri::command]
fn read_vault_file(vault_path: String, relative_path: String) -> Result<String, String> {
    let root = canonical_vault(&vault_path)?;
    let target = markdown_path(&root, &relative_path)?;
    fs::read_to_string(target).map_err(|error| format!("Cannot read note: {error}"))
}

#[tauri::command]
fn write_vault_attachment(
    vault_path: String,
    relative_path: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    if bytes.len() > 50 * 1024 * 1024 {
        return Err("Attachments larger than 50 MB are not supported yet".into());
    }
    let root = canonical_vault(&vault_path)?;
    let target = attachment_path(&root, &relative_path)?;
    let parent = target
        .parent()
        .ok_or("The attachment has no parent folder")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot create attachment folder: {error}"))?;

    let temp_name = format!(
        ".{}.ley-write",
        target.file_name().unwrap_or_default().to_string_lossy()
    );
    let temp = parent.join(temp_name);
    let mut file =
        fs::File::create(&temp).map_err(|error| format!("Cannot stage attachment: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("Cannot write attachment: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Cannot flush attachment: {error}"))?;
    fs::rename(&temp, &target).map_err(|error| format!("Cannot replace attachment: {error}"))
}

#[tauri::command]
fn read_vault_attachment(vault_path: String, relative_path: String) -> Result<Vec<u8>, String> {
    let root = canonical_vault(&vault_path)?;
    let target = attachment_path(&root, &relative_path)?;
    fs::read(target).map_err(|error| format!("Cannot read attachment: {error}"))
}

#[tauri::command]
fn rename_vault_file(vault_path: String, from: String, to: String) -> Result<(), String> {
    let root = canonical_vault(&vault_path)?;
    let source = markdown_path(&root, &from)?;
    let target = markdown_path(&root, &to)?;
    if target.exists() {
        return Err(format!("A note already exists at {to}"));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create destination folder: {error}"))?;
    }
    fs::rename(&source, &target).map_err(|error| format!("Cannot rename note: {error}"))?;
    suppress_current_change(&source);
    suppress_current_change(&target);
    Ok(())
}

#[tauri::command]
fn trash_vault_file(vault_path: String, relative_path: String) -> Result<String, String> {
    let root = canonical_vault(&vault_path)?;
    let source = markdown_path(&root, &relative_path)?;
    if !source.exists() {
        return Err("The note no longer exists".into());
    }
    let trash = root.join(".trash");
    fs::create_dir_all(&trash).map_err(|error| format!("Cannot create .trash: {error}"))?;
    let relative = safe_relative(&relative_path)?;
    let mut candidate = trash.join(&relative);
    if let Some(parent) = candidate.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create trash folder: {error}"))?;
    }
    let mut suffix = 2;
    while candidate.exists() {
        let stem = source.file_stem().unwrap_or_default().to_string_lossy();
        candidate = candidate
            .parent()
            .unwrap_or(&trash)
            .join(format!("{stem} {suffix}.md"));
        suffix += 1;
    }
    fs::rename(&source, &candidate)
        .map_err(|error| format!("Cannot move note to .trash: {error}"))?;
    suppress_current_change(&source);
    suppress_current_change(&candidate);
    Ok(candidate
        .strip_prefix(&root)
        .unwrap_or(&candidate)
        .to_string_lossy()
        .replace('\\', "/"))
}

#[tauri::command]
fn restore_trashed_vault_file(vault_path: String, trashed_path: String) -> Result<String, String> {
    let root = canonical_vault(&vault_path)?;
    let relative = safe_relative(&trashed_path)?;
    if relative
        .components()
        .next()
        .and_then(|part| part.as_os_str().to_str())
        != Some(".trash")
    {
        return Err("Only files inside .trash can be restored".into());
    }
    let source = markdown_path(&root, &trashed_path)?;
    if !source.is_file() {
        return Err("That trashed note no longer exists".into());
    }

    let original_name = source
        .file_name()
        .ok_or("The trashed note has no filename")?
        .to_string_lossy()
        .to_string();
    let relative_segments: Vec<_> = relative.components().skip(1).collect();
    let mut destination = if relative_segments.len() > 1 {
        let folder = relative_segments[..relative_segments.len() - 1]
            .iter()
            .fold(root.clone(), |current, segment| current.join(segment));
        fs::create_dir_all(&folder)
            .map_err(|error| format!("Cannot restore note folder: {error}"))?;
        folder.join(&original_name)
    } else {
        root.join(&original_name)
    };
    let mut suffix = 2;
    while destination.exists() {
        let stem = Path::new(&original_name)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        destination = destination
            .parent()
            .unwrap_or(&root)
            .join(format!("{stem} {suffix}.md"));
        suffix += 1;
    }

    let restored_relative = destination
        .strip_prefix(&root)
        .map_err(|_| "A restored file escaped the vault root")?
        .to_string_lossy()
        .replace('\\', "/");
    fs::rename(&source, &destination).map_err(|error| format!("Cannot restore note: {error}"))?;
    suppress_current_change(&source);
    suppress_current_change(&destination);
    Ok(restored_relative)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(VaultWatcherState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_agent_projects,
            forget_agent_project,
            read_agent_capture_settings,
            update_agent_capture_mode,
            erase_agent_project_memory,
            semantic_model_status,
            install_semantic_model,
            search_agent_projects,
            search_agent_project_memory,
            inspect_agent_project,
            verify_agent_project_note_vault,
            read_agent_project_specifications,
            approve_agent_project_specification,
            revoke_agent_project_specification,
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
            bind_agent_project,
            resolve_agent_project_vault,
            unbind_agent_project,
            ingest_agent_project,
            read_agent_artifacts,
            read_agent_project_graph_history,
            read_agent_project_graph_view,
            read_agent_project_graph_evidence,
            read_agent_cited_evidence,
            read_agent_media_evidence,
            read_agent_project_activity,
            scan_vault,
            scan_trashed_vault_files,
            read_vault_file,
            restore_trashed_vault_file,
            scan_canvases,
            write_canvas_file,
            trash_canvas_file,
            write_vault_file,
            write_vault_attachment,
            read_vault_attachment,
            rename_vault_file,
            trash_vault_file,
            watch_vault,
            stop_watching_vault
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ley");
}

#[cfg(test)]
mod tests {
    use super::*;

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
        write_vault_file(
            vault.to_string_lossy().into_owned(),
            "Portable/User note.md".to_owned(),
            note_body.clone(),
        )
        .unwrap();
        write_canvas_file(
            vault.to_string_lossy().into_owned(),
            "canvases/User board.canvas".to_owned(),
            canvas_body.clone(),
        )
        .unwrap();

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
            binding: preserved_binding,
            ..
        } = inspection
        else {
            panic!("expected NeedsCapture after whole-project Agent Memory erasure");
        };
        assert_eq!(project_id, initialized.identity.project_id);
        assert_eq!(project_name, "Portable erasure project");
        assert_eq!(preserved_binding.project_id, binding.project_id);
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
    fn linked_agent_notes_require_the_canonically_bound_vault() {
        let root = std::env::temp_dir().join(format!(
            "ley-native-agent-note-vault-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let bound = root.join("Bound vault");
        let other = root.join("Other vault");
        fs::create_dir_all(&bound).unwrap();
        fs::create_dir_all(&other).unwrap();
        let binding = ProjectVaultBinding {
            project_id: "prj_1234567890abcdef1234567890abcdef".to_owned(),
            vault_path: bound.canonicalize().unwrap(),
            source: BindingSource::Persisted,
        };

        verify_open_vault_binding(&binding, bound.to_str().unwrap()).unwrap();
        let error = verify_open_vault_binding(&binding, other.to_str().unwrap()).unwrap_err();
        assert!(error.contains("Bound vault"));
        assert!(error.contains("Other vault"));
        assert!(!error.contains(root.to_str().unwrap()));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn specification_authority_requires_bound_vault_and_tracks_exact_revision() {
        let root = std::env::temp_dir().join(format!(
            "ley-native-specification-authority-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let project = root.join("project");
        let vault = root.join("vault");
        let other_vault = root.join("other-vault");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(&other_vault).unwrap();
        let initialized = initialize_project(
            &project,
            Some("Specification project"),
            CaptureMode::Structured,
        )
        .unwrap();
        fs::create_dir_all(vault.join("Specs")).unwrap();
        fs::write(
            vault.join("Specs/Product.md"),
            "---\nley-type: specification\n---\n# Product\n\n## Acceptance criteria\n\n- Works offline.\n",
        )
        .unwrap();
        let binding = ProjectVaultBinding {
            project_id: initialized.identity.project_id.clone(),
            vault_path: vault.canonicalize().unwrap(),
            source: BindingSource::Persisted,
        };
        let registry = SpecificationRegistry::at(root.join("config/specifications.json"));
        let specification_id = ley_core::generate_specification_id();

        let wrong_vault = approve_project_specification_with_registry(
            &project,
            &binding,
            other_vault.to_str().unwrap(),
            &registry,
            &specification_id,
            "Specs/Product.md",
        )
        .unwrap_err();
        assert!(wrong_vault.contains("other-vault"));
        assert!(registry
            .list(&project, &vault)
            .unwrap()
            .specifications
            .is_empty());

        let approved = approve_project_specification_with_registry(
            &project,
            &binding,
            vault.to_str().unwrap(),
            &registry,
            &specification_id,
            "Specs/Product.md",
        )
        .unwrap();
        assert_eq!(approved.current, 1);
        assert_eq!(
            approved.specifications[0].approval.specification_id,
            specification_id
        );

        fs::write(
            vault.join("Specs/Product.md"),
            "---\nley-type: specification\n---\n# Product\n\n## Acceptance criteria\n\n- Works offline.\n- Syncs later.\n",
        )
        .unwrap();
        let changed =
            read_project_specifications_with_registry(&project, &binding, &registry).unwrap();
        assert_eq!(changed.current, 0);
        assert_eq!(changed.changed, 1);

        let revoked = revoke_project_specification_with_registry(
            &project,
            &binding,
            vault.to_str().unwrap(),
            &registry,
            &specification_id,
        )
        .unwrap();
        assert!(revoked.specifications.is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn filesystem_vault_lifecycle_is_real_and_confined() {
        let root = std::env::temp_dir().join(format!("ley-native-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let vault = root.to_string_lossy().to_string();

        write_vault_file(
            vault.clone(),
            "projects/First note.md".into(),
            "---\ntags: [test]\n---\n# First\n\nLinked to [[Second]].".into(),
        )
        .unwrap();
        assert!(root.join("projects/First note.md").is_file());

        let scanned = scan_vault(vault.clone()).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].path, "projects/First note.md");
        assert!(scanned[0].content.contains("[[Second]]"));

        rename_vault_file(
            vault.clone(),
            "projects/First note.md".into(),
            "projects/Renamed.md".into(),
        )
        .unwrap();
        assert!(!root.join("projects/First note.md").exists());
        assert!(root.join("projects/Renamed.md").is_file());

        let trashed = trash_vault_file(vault.clone(), "projects/Renamed.md".into()).unwrap();
        assert_eq!(trashed, ".trash/projects/Renamed.md");
        assert!(root.join(".trash/projects/Renamed.md").is_file());
        assert!(scan_vault(vault.clone()).unwrap().is_empty());

        let trashed_files = scan_trashed_vault_files(vault.clone()).unwrap();
        assert_eq!(trashed_files.len(), 1);
        assert_eq!(trashed_files[0].path, ".trash/projects/Renamed.md");

        let restored =
            restore_trashed_vault_file(vault.clone(), trashed_files[0].path.clone()).unwrap();
        assert_eq!(restored, "projects/Renamed.md");
        assert!(!root.join(".trash/projects/Renamed.md").exists());
        assert_eq!(scan_trashed_vault_files(vault.clone()).unwrap().len(), 0);

        write_vault_file(vault.clone(), "Renamed.md".into(), "current".into()).unwrap();
        write_vault_file(vault.clone(), "Renamed 2.md".into(), "older".into()).unwrap();
        let second_trash = trash_vault_file(vault.clone(), "Renamed 2.md".into()).unwrap();
        assert_eq!(second_trash, ".trash/Renamed 2.md");
        write_vault_file(vault.clone(), "projects/Nested.md".into(), "nested".into()).unwrap();
        let nested_trash = trash_vault_file(vault.clone(), "projects/Nested.md".into()).unwrap();
        assert_eq!(nested_trash, ".trash/projects/Nested.md");
        assert_eq!(
            restore_trashed_vault_file(vault.clone(), nested_trash).unwrap(),
            "projects/Nested.md"
        );
        let collision_restore = restore_trashed_vault_file(vault.clone(), second_trash).unwrap();
        assert_eq!(collision_restore, "Renamed 2.md");

        assert!(write_vault_file(vault, "../escape.md".into(), "nope".into()).is_err());
        assert!(!root.parent().unwrap().join("escape.md").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn attachment_io_is_real_and_scoped() {
        let root = std::env::temp_dir().join(format!("ley-attachment-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let vault = root.to_string_lossy().to_string();
        let bytes = vec![0x89, b'P', b'N', b'G'];

        write_vault_attachment(
            vault.clone(),
            "attachments/diagram.png".into(),
            bytes.clone(),
        )
        .unwrap();
        assert_eq!(
            read_vault_attachment(vault.clone(), "attachments/diagram.png".into()).unwrap(),
            bytes
        );
        assert!(write_vault_attachment(vault.clone(), "../diagram.png".into(), vec![]).is_err());
        assert!(write_vault_attachment(vault, "notes/script.js".into(), vec![]).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canvas_files_round_trip_as_interoperable_json() {
        let root = std::env::temp_dir().join(format!("ley-canvas-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let vault = root.to_string_lossy().to_string();
        let content = r#"{"nodes":[{"id":"a","type":"text","text":"Idea","x":0,"y":0,"width":260,"height":140}],"edges":[]}"#;
        write_canvas_file(
            vault.clone(),
            "canvases/Ideas.canvas".into(),
            content.into(),
        )
        .unwrap();
        let canvases = scan_canvases(vault.clone()).unwrap();
        assert_eq!(canvases.len(), 1);
        assert_eq!(canvases[0].path, "canvases/Ideas.canvas");
        assert!(canvases[0].content.contains("\"nodes\""));
        trash_canvas_file(vault.clone(), "canvases/Ideas.canvas".into()).unwrap();
        assert!(root.join(".trash/Ideas.canvas").is_file());
        assert!(scan_canvases(vault.clone()).unwrap().is_empty());
        assert!(write_canvas_file(vault, "../escape.canvas".into(), "{}".into()).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_watcher_reports_external_markdown_changes_and_ignores_hidden_files() {
        let root = std::env::temp_dir().join(format!("ley-watcher-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".trash")).unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let watcher = build_vault_event_watcher(root.clone(), move |change| {
            let _ = sender.send(change.paths);
        })
        .unwrap();

        fs::write(root.join("External.md"), "# Changed outside Ley").unwrap();
        let paths = receiver.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(paths.contains(&"External.md".to_string()));
        assert!(relevant_change_path(&root, &root.join(".trash/Hidden.md")).is_none());
        assert!(relevant_change_path(&root, &root.join("image.png")).is_none());

        drop(watcher);
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
        let vault = root.join("vault");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&vault).unwrap();
        fs::write(project.join("README.md"), "# Reviewed\n").unwrap();
        let reviewed = agent_initial_capture_preview(&project).unwrap();
        fs::write(
            project.join("new-after-review.ts"),
            "export const changed = true;\n",
        )
        .unwrap();

        let error = match initialize_agent_project(
            project.to_string_lossy().into_owned(),
            vault.to_string_lossy().into_owned(),
            reviewed.approval_fingerprint,
        ) {
            Ok(_) => panic!("stale approval unexpectedly initialized the project"),
            Err(error) => error,
        };
        assert!(error.contains("capture plan changed"));
        assert!(!project.join(ley_core::LEY_DIRECTORY).exists());
        assert!(fs::read_dir(&vault).unwrap().next().is_none());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unbound_connect_requires_fresh_project_bound_capture_approval() {
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
        let reviewed = agent_existing_capture_preview(&diagnostic).unwrap();
        let registry = BindingRegistry::at(config.join("bindings.json"));
        fs::write(
            project.join("new-after-review.ts"),
            "export const changed = true;\n",
        )
        .unwrap();

        assert!(matches!(
            connect_agent_project_with_registry(
                &project,
                &vault,
                Some(&reviewed.approval_fingerprint),
                &registry,
            ),
            Err(LeyCoreError::CapturePreviewChanged)
        ));
        assert!(matches!(
            registry.resolve_observed(&diagnostic),
            Err(LeyCoreError::VaultNotBound(_))
        ));
        assert!(fs::read_dir(&vault).unwrap().next().is_none());

        let refreshed = agent_existing_capture_preview(&diagnostic).unwrap();
        let dashboard = connect_agent_project_with_registry(
            &project,
            &vault,
            Some(&refreshed.approval_fingerprint),
            &registry,
        )
        .unwrap();
        assert_eq!(dashboard.binding.project_id, diagnostic.identity.project_id);
        assert!(vault.join(".ley").exists());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_catalog_summarizes_ready_unbound_and_unavailable_projects() {
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
        fs::remove_dir_all(&unavailable).unwrap();
        fs::rename(&moved_vault, root.join("vault-after-move")).unwrap();

        let view = load_agent_project_catalog_from(&catalog, &registry).unwrap();
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
                && project.project_path == unavailable.canonicalize().unwrap_or(unavailable.clone())
        }));

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
