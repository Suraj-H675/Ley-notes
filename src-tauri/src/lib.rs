use ley_core::{
    correct_learning, diagnose_project, generate_learning_request_id, ingest_project,
    initialize_project, list_learning_contexts, list_sessions, project_activity_view,
    project_artifact_inventory, project_graph_view, project_memory_overview,
    project_resume_context, project_session_stats, read_learning, read_learning_context,
    read_session_context, review_learning, search_observed_projects, update_capture_mode,
    BindingRegistry, BindingSource, CaptureMode, CorrectLearningInput, CrossProjectSearch,
    IngestionResult, LearningActor, LearningContextPack, LearningEvidenceInput,
    LearningFeedbackAction, LearningList, LearningListScope, LeyCoreError, MemoryOverview,
    ProjectActivityView, ProjectArtifactInventory, ProjectCatalog, ProjectDiagnostic,
    ProjectGraphView, ProjectProblemScope, ProjectResumePack, ProjectVaultBinding,
    ReviewLearningInput, SessionContextPack, SessionSummary, DEFAULT_ARTIFACT_RESULTS,
    DEFAULT_CROSS_PROJECT_SEARCH_RESULTS, DEFAULT_GRAPH_VIEW_EDGES, DEFAULT_GRAPH_VIEW_NODES,
    DEFAULT_LEARNING_CONTEXT_ARTIFACTS, DEFAULT_LEARNING_CONTEXT_CHARACTERS,
    DEFAULT_LEARNING_CONTEXT_EVIDENCE, DEFAULT_LEARNING_CONTEXT_HISTORY,
    DEFAULT_PROJECT_ACTIVITY_RESULTS, DEFAULT_PROJECT_CATALOG_RESULTS, DEFAULT_RESUME_CHARACTERS,
    DEFAULT_RESUME_LEARNINGS, DEFAULT_RESUME_SESSIONS, DEFAULT_SESSION_CONTEXT_CHARACTERS,
    DEFAULT_SESSION_CONTEXT_CHECKPOINTS, MAX_LEARNING_LIST_RESULTS,
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VaultFile {
    path: String,
    content: String,
    created_at: u64,
    updated_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CanvasFile {
    path: String,
    content: String,
    updated_at: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VaultChange {
    paths: Vec<String>,
}

struct ActiveVaultWatcher {
    root: PathBuf,
    _watcher: RecommendedWatcher,
}

#[derive(Default)]
struct VaultWatcherState(Mutex<Option<ActiveVaultWatcher>>);

static SUPPRESSED_CHANGES: OnceLock<Mutex<HashMap<PathBuf, Instant>>> = OnceLock::new();

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
#[serde(tag = "status", rename_all = "kebab-case")]
enum AgentProjectInspection {
    Uninitialized {
        suggested_name: String,
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
            return Ok(AgentProjectInspection::Uninitialized { suggested_name });
        }
        Err(error) => return Err(error.to_string()),
    };
    let binding = match resolved_agent_binding(&diagnostic.root) {
        Ok(binding) => binding,
        Err(LeyCoreError::VaultNotBound(_)) => {
            return Ok(AgentProjectInspection::Unbound {
                project_id: diagnostic.identity.project_id,
                project_name: diagnostic.identity.name,
                capture_mode: diagnostic.capture.mode,
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

#[tauri::command]
fn initialize_agent_project(
    project_path: String,
    vault_path: String,
) -> Result<AgentMemoryDashboard, String> {
    initialize_project(&project_path, None, CaptureMode::Structured)
        .and_then(|initialization| {
            let binding =
                BindingRegistry::system_default()?.bind(&initialization.root, &vault_path)?;
            ingest_project(&initialization.root, &binding.vault_path)?;
            load_agent_memory_dashboard(&initialization.root, binding)
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn connect_agent_project(
    project_path: String,
    vault_path: String,
) -> Result<AgentMemoryDashboard, String> {
    BindingRegistry::system_default()
        .and_then(|registry| registry.bind(&project_path, &vault_path))
        .and_then(|binding| {
            ingest_project(&project_path, &binding.vault_path)?;
            load_agent_memory_dashboard(Path::new(&project_path), binding)
        })
        .map_err(|error| error.to_string())
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
fn review_agent_learning(
    project_path: String,
    learning_id: String,
    expected_event_count: u64,
    action: LearningFeedbackAction,
    note: String,
) -> Result<AgentMemoryDashboard, String> {
    resolved_agent_binding(Path::new(&project_path))
        .and_then(|binding| {
            review_learning(
                &project_path,
                &binding.vault_path,
                &learning_id,
                ReviewLearningInput {
                    request_id: generate_learning_request_id(),
                    expected_event_count: Some(expected_event_count),
                    actor: LearningActor::User,
                    action,
                    note,
                    replacement_learning_id: None,
                },
            )?;
            load_agent_memory_dashboard(Path::new(&project_path), binding)
        })
        .map_err(|error| error.to_string())
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
    query: Option<String>,
    max_nodes: Option<usize>,
    max_edges: Option<usize>,
) -> Result<ProjectGraphView, String> {
    let override_path = vault_override.as_deref().map(Path::new);
    let binding = BindingRegistry::system_default()
        .and_then(|registry| registry.resolve(&project_path, override_path))
        .map_err(|error| error.to_string())?;
    project_graph_view(
        project_path,
        binding.vault_path,
        query.as_deref().unwrap_or_default(),
        max_nodes.unwrap_or(DEFAULT_GRAPH_VIEW_NODES),
        max_edges.unwrap_or(DEFAULT_GRAPH_VIEW_EDGES),
    )
    .map_err(|error| error.to_string())
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

fn suppress_change(path: &Path) {
    let changes = SUPPRESSED_CHANGES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut changes) = changes.lock() {
        changes.insert(path.to_path_buf(), Instant::now());
    }
}

fn is_suppressed(path: &Path) -> bool {
    let changes = SUPPRESSED_CHANGES.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut changes) = changes.lock() else {
        return false;
    };
    changes.retain(|_, created| created.elapsed() < Duration::from_millis(750));
    changes.contains_key(path)
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

fn build_vault_watcher<F>(root: PathBuf, mut on_change: F) -> Result<RecommendedWatcher, String>
where
    F: FnMut(Vec<String>) + Send + 'static,
{
    let event_root = root.clone();
    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<Event>| {
            let Ok(event) = result else { return };
            let mut paths: Vec<String> = event
                .paths
                .iter()
                .filter_map(|path| relevant_change_path(&event_root, path))
                .collect();
            paths.sort();
            paths.dedup();
            if !paths.is_empty() {
                on_change(paths);
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
    let watcher = build_vault_watcher(root.clone(), move |paths| {
        let _ = app.emit("ley-vault-changed", VaultChange { paths });
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
    suppress_change(&target);
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
    fs::rename(temp, target).map_err(|error| format!("Cannot replace canvas: {error}"))
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
    suppress_change(&source);
    suppress_change(&candidate);
    fs::rename(source, candidate).map_err(|error| format!("Cannot move canvas to .trash: {error}"))
}

#[tauri::command]
fn write_vault_file(
    vault_path: String,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    let root = canonical_vault(&vault_path)?;
    let target = markdown_path(&root, &relative_path)?;
    suppress_change(&target);
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
    Ok(())
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
    suppress_change(&source);
    suppress_change(&target);
    fs::rename(source, target).map_err(|error| format!("Cannot rename note: {error}"))
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
    let original = source
        .file_name()
        .ok_or("The note has no filename")?
        .to_string_lossy();
    let mut candidate = trash.join(original.as_ref());
    let mut suffix = 2;
    while candidate.exists() {
        let stem = Path::new(original.as_ref())
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        candidate = trash.join(format!("{stem} {suffix}.md"));
        suffix += 1;
    }
    suppress_change(&source);
    suppress_change(&candidate);
    fs::rename(source, &candidate)
        .map_err(|error| format!("Cannot move note to .trash: {error}"))?;
    Ok(candidate
        .strip_prefix(&root)
        .unwrap_or(&candidate)
        .to_string_lossy()
        .replace('\\', "/"))
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
            search_agent_projects,
            inspect_agent_project,
            initialize_agent_project,
            connect_agent_project,
            refresh_agent_project,
            read_agent_learning,
            read_agent_session,
            review_agent_learning,
            correct_agent_learning,
            bind_agent_project,
            resolve_agent_project_vault,
            unbind_agent_project,
            ingest_agent_project,
            read_agent_artifacts,
            read_agent_project_graph_view,
            read_agent_project_activity,
            scan_vault,
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
        assert_eq!(trashed, ".trash/Renamed.md");
        assert!(root.join(".trash/Renamed.md").is_file());
        assert!(scan_vault(vault.clone()).unwrap().is_empty());

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
        let watcher = build_vault_watcher(root.clone(), move |paths| {
            let _ = sender.send(paths);
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
            LearningProvenance, ProposeLearningInput, ReviewLearningInput, SessionSource,
            StartSessionInput,
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

        let pending = load_agent_memory_dashboard(&project, binding.clone()).unwrap();
        assert_eq!(pending.resume.total_sessions, 1);
        assert_eq!(pending.sessions.len(), 1);
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
}
