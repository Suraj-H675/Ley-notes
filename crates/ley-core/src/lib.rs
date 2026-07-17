use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

mod binding;
mod graph;
mod ingestion;
mod learning;
mod learning_context;
mod resume_context;
mod retrieval;
mod session;
mod session_context;

pub use binding::{
    default_binding_registry_path, BindingRegistry, BindingSource, ProjectVaultBinding,
    APP_IDENTIFIER, BINDING_REGISTRY_FILE, BINDING_REGISTRY_SCHEMA_VERSION,
};
pub use graph::{
    FactProvenance, GitChange, GitState, GraphCitation, GraphDiagnostic, GraphEdge, GraphEdgeKind,
    GraphNode, GraphNodeKind, ProjectGraph, PROJECT_GRAPH_LIMIT_BYTES,
    PROJECT_GRAPH_SCHEMA_VERSION,
};
pub use ingestion::{
    ingest_project, read_project_graph, ArtifactKind, ArtifactRecord, ArtifactSkipReason,
    IngestionResult, RedactionFinding, RenamedArtifact, SkippedArtifact, AGENT_MEMORY_DIRECTORY,
    ARTIFACT_MANIFEST_LIMIT_BYTES, ARTIFACT_MANIFEST_SCHEMA_VERSION,
};
pub use learning::{
    correct_learning, generate_learning_request_id, learning_review_inbox, list_learnings,
    propose_learning, read_learning, review_learning, CorrectLearningInput, LearningActor,
    LearningEvidence, LearningEvidenceInput, LearningFeedbackAction, LearningFreshness,
    LearningIndex, LearningKind, LearningMutation, LearningProvenance, LearningRecord,
    LearningRedaction, LearningReviewEntry, LearningState, LearningSummary, LearningTrustState,
    ProposeLearningInput, ReviewLearningInput, LEARNING_EVENT_LIMIT, LEARNING_EVENT_LIMIT_BYTES,
    LEARNING_INDEX_LIMIT_BYTES, LEARNING_SCHEMA_VERSION,
};
pub use learning_context::{
    list_learning_contexts, read_learning_context, LearningContextPack, LearningList,
    LearningListScope, DEFAULT_LEARNING_CONTEXT_ARTIFACTS, DEFAULT_LEARNING_CONTEXT_CHARACTERS,
    DEFAULT_LEARNING_CONTEXT_EVIDENCE, DEFAULT_LEARNING_CONTEXT_HISTORY,
    DEFAULT_LEARNING_LIST_RESULTS, MAX_LEARNING_CONTEXT_ARTIFACTS, MAX_LEARNING_CONTEXT_CHARACTERS,
    MAX_LEARNING_CONTEXT_EVIDENCE, MAX_LEARNING_CONTEXT_HISTORY, MAX_LEARNING_LIST_RESULTS,
    MIN_LEARNING_CONTEXT_CHARACTERS,
};
pub use resume_context::{
    project_resume_context, ProjectResumePack, ResumeCheckpoint, ResumeDecision, ResumeLearning,
    ResumeProblem, ResumeResult, ResumeSession, ResumeTask, DEFAULT_RESUME_CHARACTERS,
    DEFAULT_RESUME_LEARNINGS, DEFAULT_RESUME_SESSIONS, MAX_RESUME_CHARACTERS, MAX_RESUME_LEARNINGS,
    MAX_RESUME_SESSIONS, MIN_RESUME_CHARACTERS,
};
pub use retrieval::{
    find_project_context, find_project_graph_path, project_memory_overview, read_project_evidence,
    traverse_project_graph, ContextItem, ContextItemKind, ContextPack, EvidenceExcerpt,
    GraphDirection, GraphPath, GraphTraversal, MemoryOverview, RetrievalLimits,
    DEFAULT_CONTEXT_RESULTS, DEFAULT_CONTEXT_TOKENS, MAX_CONTEXT_RESULTS, MAX_CONTEXT_TOKENS,
};
pub use session::{
    checkpoint_session, finish_session, generate_request_id, list_sessions, read_session,
    start_session, AgentSession, AttemptInput, AttemptOutcome, AttemptRecord, CheckpointInput,
    CommandInput, CommandRecord, DecisionInput, DecisionRecord, FinishSessionInput,
    MemoryRedaction, PlanItem, PlanItemInput, PlanStatus, ProblemInput, ProblemRecord,
    ResolutionInput, ResolutionRecord, SessionArtifactCitation, SessionCheckpoint, SessionFinish,
    SessionMutation, SessionSource, SessionSourceKind, SessionStatus, SessionSummary,
    StartSessionInput, TaskInput, TaskRecord, TaskStatus, VerificationInput, VerificationRecord,
    VerificationStatus, SESSION_EVENT_LIMIT, SESSION_EVENT_LIMIT_BYTES,
    SESSION_PROJECTION_LIMIT_BYTES, SESSION_SCHEMA_VERSION,
};
pub use session_context::{
    list_session_contexts, read_session_context, SessionContextAttempt, SessionContextCheckpoint,
    SessionContextCitation, SessionContextCommand, SessionContextDecision, SessionContextFinish,
    SessionContextPack, SessionContextProblem, SessionContextResolution, SessionContextTask,
    SessionContextVerification, SessionList, SessionListItem, DEFAULT_SESSION_CONTEXT_CHARACTERS,
    DEFAULT_SESSION_CONTEXT_CHECKPOINTS, DEFAULT_SESSION_LIST_RESULTS,
    MAX_SESSION_CONTEXT_CHARACTERS, MAX_SESSION_CONTEXT_CHECKPOINTS, MAX_SESSION_LIST_RESULTS,
    MIN_SESSION_CONTEXT_CHARACTERS,
};

pub const LEY_DIRECTORY: &str = ".ley";
pub const PROJECT_FILE: &str = "project.json";
pub const CAPTURE_FILE: &str = "capture.json";
pub const IGNORE_FILE: &str = ".leyignore";
pub const PROJECT_SCHEMA_VERSION: u32 = 1;
pub const METADATA_FILE_LIMIT_BYTES: u64 = 1_048_576;

pub const DEFAULT_IGNORE_RULES: &str = r#"# Ley project capture exclusions
# Git ignore rules are applied separately. These rules are always local to this project.
.ley/
.git/
node_modules/
target/
dist/
build/
coverage/
.env
.env.*
*.pem
*.key
*.p12
*.pfx
*.keystore
.npmrc
.pypirc
credentials.json
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureMode {
    Minimal,
    Structured,
    FullEvidence,
}

impl CaptureMode {
    pub fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "minimal" => Ok(Self::Minimal),
            "structured" => Ok(Self::Structured),
            "full" | "full-evidence" => Ok(Self::FullEvidence),
            _ => Err(LeyCoreError::InvalidCaptureMode(value.to_owned())),
        }
    }
}

impl std::fmt::Display for CaptureMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Minimal => "minimal",
            Self::Structured => "structured",
            Self::FullEvidence => "full-evidence",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectIdentity {
    pub schema_version: u32,
    pub project_id: String,
    pub name: String,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturePolicy {
    pub schema_version: u32,
    pub mode: CaptureMode,
    pub approved_roots: Vec<String>,
    pub respect_gitignore: bool,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub store_raw_transcripts: bool,
}

impl CapturePolicy {
    pub fn for_mode(mode: CaptureMode) -> Self {
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            mode,
            approved_roots: vec![".".to_owned()],
            respect_gitignore: true,
            max_file_bytes: 1_048_576,
            max_total_bytes: 536_870_912,
            store_raw_transcripts: mode == CaptureMode::FullEvidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInitialization {
    pub root: PathBuf,
    pub identity: ProjectIdentity,
    pub capture: CapturePolicy,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDiagnostic {
    pub root: PathBuf,
    pub identity: ProjectIdentity,
    pub capture: CapturePolicy,
    pub ignore_file_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFile {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturePreview {
    pub root: PathBuf,
    pub project_id: String,
    pub mode: CaptureMode,
    pub capture_fingerprint: String,
    pub files: Vec<CaptureFile>,
    pub included_bytes: u64,
    pub skipped_oversized: Vec<CaptureFile>,
    pub skipped_total_limit: Vec<CaptureFile>,
    pub skipped_symlinks: Vec<String>,
}

#[derive(Debug, Error)]
pub enum LeyCoreError {
    #[error("project path is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("no initialized Ley project found from: {0}")]
    ProjectNotFound(PathBuf),
    #[error("project name must be between 1 and 128 visible characters")]
    InvalidProjectName,
    #[error("unsupported capture mode '{0}'; use minimal, structured, or full")]
    InvalidCaptureMode(String),
    #[error("invalid Ley project identity: {0}")]
    InvalidProjectIdentity(String),
    #[error("invalid Ley capture policy: {0}")]
    InvalidCapturePolicy(String),
    #[error("invalid Ley vault binding registry: {0}")]
    InvalidBindingRegistry(String),
    #[error("no private configuration directory is available on this operating system")]
    ConfigDirectoryUnavailable,
    #[error("project {0} is not bound to a Ley vault; run 'ley bind --vault <path>'")]
    VaultNotBound(String),
    #[error("the bound Ley vault is unavailable; rebind project {project_id}: {path}")]
    BoundVaultUnavailable { project_id: String, path: PathBuf },
    #[error("a Ley vault cannot be the project directory or live inside it: {0}")]
    OverlappingProjectVault(PathBuf),
    #[error("project file changed after capture preview; rerun ingestion: {0}")]
    ProjectChangedDuringIngestion(String),
    #[error("invalid Ley artifact store: {0}")]
    InvalidArtifactStore(String),
    #[error("invalid Ley project graph: {0}")]
    InvalidProjectGraph(String),
    #[error("Ley project memory is unavailable: {0}")]
    ProjectMemoryUnavailable(String),
    #[error("invalid Ley retrieval request: {0}")]
    InvalidRetrievalRequest(String),
    #[error("invalid Ley session store: {0}")]
    InvalidSessionStore(String),
    #[error("invalid Ley session request: {0}")]
    InvalidSessionRequest(String),
    #[error("Ley session not found: {0}")]
    SessionNotFound(String),
    #[error("session request ID was reused with different content: {0}")]
    SessionIdempotencyConflict(String),
    #[error("invalid Ley learning store: {0}")]
    InvalidLearningStore(String),
    #[error("invalid Ley learning request: {0}")]
    InvalidLearningRequest(String),
    #[error("Ley learning not found: {0}")]
    LearningNotFound(String),
    #[error("learning request ID was reused with different content: {0}")]
    LearningIdempotencyConflict(String),
    #[error("unsafe Ley project layout at {0}")]
    UnsafeProjectLayout(PathBuf),
    #[error("Ley metadata exceeds the {limit_bytes}-byte limit: {path}")]
    MetadataTooLarge { path: PathBuf, limit_bytes: u64 },
    #[error("invalid ignore rule in {path}: {message}")]
    InvalidIgnoreRule { path: PathBuf, message: String },
    #[error("could not scan project: {0}")]
    CaptureWalk(String),
    #[error("project path is not valid UTF-8: {0}")]
    NonUtf8Path(PathBuf),
    #[error("could not access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not parse {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

pub fn initialize_project(
    root: impl AsRef<Path>,
    requested_name: Option<&str>,
    mode: CaptureMode,
) -> Result<ProjectInitialization, LeyCoreError> {
    let root = canonical_directory(root.as_ref())?;
    let ley_directory = root.join(LEY_DIRECTORY);
    match fs::symlink_metadata(&ley_directory) {
        Ok(_) => {
            let diagnostic = diagnose_project(&root)?;
            return Ok(ProjectInitialization {
                root,
                identity: diagnostic.identity,
                capture: diagnostic.capture,
                created: false,
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LeyCoreError::Io {
                path: ley_directory,
                source,
            })
        }
    }

    let name = validated_project_name(requested_name, &root)?;
    let identity = ProjectIdentity {
        schema_version: PROJECT_SCHEMA_VERSION,
        project_id: format!("prj_{}", Uuid::new_v4().simple()),
        name,
        created_at_unix_ms: unix_time_ms(),
    };
    let capture = CapturePolicy::for_mode(mode);

    let staging = root.join(format!(".ley.tmp-{}", Uuid::new_v4().simple()));
    fs::create_dir(&staging).map_err(|source| LeyCoreError::Io {
        path: staging.clone(),
        source,
    })?;
    let staged = (|| {
        write_json_atomic(&staging.join(PROJECT_FILE), &identity)?;
        write_json_atomic(&staging.join(CAPTURE_FILE), &capture)?;
        write_new_file(&staging.join(IGNORE_FILE), DEFAULT_IGNORE_RULES.as_bytes())?;
        fs::rename(&staging, &ley_directory).map_err(|source| LeyCoreError::Io {
            path: ley_directory.clone(),
            source,
        })
    })();
    if staged.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    staged?;

    Ok(ProjectInitialization {
        root,
        identity,
        capture,
        created: true,
    })
}

pub fn diagnose_project(start: impl AsRef<Path>) -> Result<ProjectDiagnostic, LeyCoreError> {
    let root = find_project_root(start.as_ref())?;
    let ley_directory = root.join(LEY_DIRECTORY);
    let identity: ProjectIdentity = read_json(&ley_directory.join(PROJECT_FILE))?;
    validate_identity(&identity)?;
    let capture: CapturePolicy = read_json(&ley_directory.join(CAPTURE_FILE))?;
    validate_capture(&capture)?;
    let ignore_file_present = optional_regular_metadata_file(&ley_directory.join(IGNORE_FILE))?;

    Ok(ProjectDiagnostic {
        root,
        identity,
        capture,
        ignore_file_present,
    })
}

pub fn preview_capture(start: impl AsRef<Path>) -> Result<CapturePreview, LeyCoreError> {
    use ignore::gitignore::GitignoreBuilder;
    use ignore::WalkBuilder;

    let diagnostic = diagnose_project(start)?;
    let root = &diagnostic.root;
    let ignore_path = root.join(LEY_DIRECTORY).join(IGNORE_FILE);
    let ignore_metadata = ensure_regular_metadata_file(&ignore_path)?;
    if ignore_metadata.len() > METADATA_FILE_LIMIT_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: ignore_path.clone(),
            limit_bytes: METADATA_FILE_LIMIT_BYTES,
        });
    }
    let ignore_body = fs::read_to_string(&ignore_path).map_err(|source| LeyCoreError::Io {
        path: ignore_path.clone(),
        source,
    })?;
    let mut fingerprint = Sha256::new();
    fingerprint.update(
        serde_json::to_vec(&diagnostic.capture).expect("validated capture policy is serializable"),
    );
    fingerprint.update([0]);
    fingerprint.update(ignore_body.as_bytes());
    let capture_fingerprint = format!("sha256:{:x}", fingerprint.finalize());
    let mut ignore_builder = GitignoreBuilder::new(root);
    for line in ignore_body.lines() {
        ignore_builder
            .add_line(Some(ignore_path.clone()), line)
            .map_err(|error| LeyCoreError::InvalidIgnoreRule {
                path: ignore_path.clone(),
                message: error.to_string(),
            })?;
    }
    let ley_ignore = ignore_builder
        .build()
        .map_err(|error| LeyCoreError::InvalidIgnoreRule {
            path: ignore_path,
            message: error.to_string(),
        })?;

    let mut candidates = BTreeMap::<String, u64>::new();
    let mut skipped_symlinks = Vec::new();
    let approved_paths = diagnostic
        .capture
        .approved_roots
        .iter()
        .map(|approved_root| {
            checked_capture_root(root, approved_root)?;
            Ok(normalized_capture_relative(approved_root))
        })
        .collect::<Result<Vec<_>, LeyCoreError>>()?;
    let filter_root = root.clone();
    let filter = ley_ignore.clone();
    let mut builder = WalkBuilder::new(root);
    builder
        .parents(false)
        .hidden(false)
        .ignore(false)
        .git_ignore(diagnostic.capture.respect_gitignore)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .sort_by_file_path(|left, right| left.cmp(right))
        .filter_entry(move |entry| {
            let Ok(relative) = entry.path().strip_prefix(&filter_root) else {
                return false;
            };
            let is_in_scope = approved_paths
                .iter()
                .any(|approved| relative.starts_with(approved) || approved.starts_with(relative));
            let is_directory = entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false);
            is_in_scope && !filter.matched(entry.path(), is_directory).is_ignore()
        });

    for result in builder.build() {
        let entry = result.map_err(|error| LeyCoreError::CaptureWalk(error.to_string()))?;
        if entry.depth() == 0 {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let relative = capture_relative_path(root, path)?;
        if metadata.file_type().is_symlink() {
            skipped_symlinks.push(relative);
        } else if metadata.is_file() {
            candidates.insert(relative, metadata.len());
        }
    }

    skipped_symlinks.sort();
    skipped_symlinks.dedup();
    let mut files = Vec::new();
    let mut skipped_oversized = Vec::new();
    let mut skipped_total_limit = Vec::new();
    let mut included_bytes = 0_u64;
    for (path, bytes) in candidates {
        let file = CaptureFile { path, bytes };
        if bytes > diagnostic.capture.max_file_bytes {
            skipped_oversized.push(file);
        } else if included_bytes.saturating_add(bytes) > diagnostic.capture.max_total_bytes {
            skipped_total_limit.push(file);
        } else {
            included_bytes += bytes;
            files.push(file);
        }
    }

    Ok(CapturePreview {
        root: diagnostic.root,
        project_id: diagnostic.identity.project_id,
        mode: diagnostic.capture.mode,
        capture_fingerprint,
        files,
        included_bytes,
        skipped_oversized,
        skipped_total_limit,
        skipped_symlinks,
    })
}

fn checked_capture_root(project_root: &Path, relative: &str) -> Result<PathBuf, LeyCoreError> {
    let mut current = project_root.to_path_buf();
    for component in Path::new(relative).components() {
        if component == Component::CurDir {
            continue;
        }
        let Component::Normal(segment) = component else {
            return Err(LeyCoreError::InvalidCapturePolicy(
                "approvedRoots must contain safe project-relative paths".to_owned(),
            ));
        };
        current.push(segment);
        let metadata = fs::symlink_metadata(&current).map_err(|source| LeyCoreError::Io {
            path: current.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LeyCoreError::UnsafeProjectLayout(current));
        }
    }
    Ok(current)
}

fn normalized_capture_relative(relative: &str) -> PathBuf {
    Path::new(relative)
        .components()
        .filter_map(|component| match component {
            Component::Normal(segment) => Some(segment),
            Component::CurDir => None,
            _ => unreachable!("capture policy was validated before normalization"),
        })
        .collect()
}

fn capture_relative_path(root: &Path, path: &Path) -> Result<String, LeyCoreError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| LeyCoreError::UnsafeProjectLayout(path.to_path_buf()))?;
    let value = relative
        .to_str()
        .ok_or_else(|| LeyCoreError::NonUtf8Path(relative.to_path_buf()))?;
    Ok(value.replace(std::path::MAIN_SEPARATOR, "/"))
}

pub fn find_project_root(start: &Path) -> Result<PathBuf, LeyCoreError> {
    let canonical = if start.is_dir() {
        canonical_directory(start)?
    } else if let Some(parent) = start.parent() {
        canonical_directory(parent)?
    } else {
        return Err(LeyCoreError::ProjectNotFound(start.to_path_buf()));
    };
    for candidate in canonical.ancestors() {
        let ley_directory = candidate.join(LEY_DIRECTORY);
        match fs::symlink_metadata(&ley_directory) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(LeyCoreError::Io {
                    path: ley_directory,
                    source,
                })
            }
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(LeyCoreError::UnsafeProjectLayout(ley_directory))
            }
            Ok(_) => {}
        }
        ensure_regular_metadata_file(&ley_directory.join(PROJECT_FILE))?;
        return Ok(candidate.to_path_buf());
    }
    Err(LeyCoreError::ProjectNotFound(canonical))
}

fn canonical_directory(path: &Path) -> Result<PathBuf, LeyCoreError> {
    if !path.is_dir() {
        return Err(LeyCoreError::NotDirectory(path.to_path_buf()));
    }
    path.canonicalize().map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn validated_project_name(requested: Option<&str>, root: &Path) -> Result<String, LeyCoreError> {
    let fallback = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Project");
    let name = requested.unwrap_or(fallback).trim();
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(LeyCoreError::InvalidProjectName);
    }
    Ok(name.to_owned())
}

fn validate_identity(identity: &ProjectIdentity) -> Result<(), LeyCoreError> {
    if identity.schema_version != PROJECT_SCHEMA_VERSION {
        return Err(LeyCoreError::InvalidProjectIdentity(format!(
            "unsupported schema version {}",
            identity.schema_version
        )));
    }
    validate_project_id(&identity.project_id)?;
    validated_project_name(Some(&identity.name), Path::new("."))?;
    Ok(())
}

pub(crate) fn validate_project_id(project_id: &str) -> Result<(), LeyCoreError> {
    let Some(uuid) = project_id.strip_prefix("prj_") else {
        return Err(LeyCoreError::InvalidProjectIdentity(
            "projectId must start with prj_".to_owned(),
        ));
    };
    if uuid.len() != 32
        || !uuid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || Uuid::parse_str(uuid).is_err()
    {
        return Err(LeyCoreError::InvalidProjectIdentity(
            "projectId must contain a 32-character lowercase hexadecimal UUID".to_owned(),
        ));
    }
    Ok(())
}

fn validate_capture(capture: &CapturePolicy) -> Result<(), LeyCoreError> {
    if capture.schema_version != PROJECT_SCHEMA_VERSION {
        return Err(LeyCoreError::InvalidCapturePolicy(format!(
            "unsupported schema version {}",
            capture.schema_version
        )));
    }
    let unique_roots = capture.approved_roots.iter().collect::<HashSet<_>>();
    if capture.approved_roots.is_empty()
        || unique_roots.len() != capture.approved_roots.len()
        || capture.approved_roots.iter().any(|root| {
            root.is_empty()
                || Path::new(root)
                    .components()
                    .any(|part| !matches!(part, Component::CurDir | Component::Normal(_)))
        })
    {
        return Err(LeyCoreError::InvalidCapturePolicy(
            "approvedRoots must contain safe project-relative paths".to_owned(),
        ));
    }
    if capture.max_file_bytes == 0 || capture.max_total_bytes < capture.max_file_bytes {
        return Err(LeyCoreError::InvalidCapturePolicy(
            "capture byte limits are inconsistent".to_owned(),
        ));
    }
    if capture.store_raw_transcripts && capture.mode != CaptureMode::FullEvidence {
        return Err(LeyCoreError::InvalidCapturePolicy(
            "raw transcripts require full-evidence mode".to_owned(),
        ));
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, LeyCoreError> {
    let metadata = ensure_regular_metadata_file(path)?;
    if metadata.len() > METADATA_FILE_LIMIT_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: path.to_path_buf(),
            limit_bytes: METADATA_FILE_LIMIT_BYTES,
        });
    }
    let bytes = fs::read(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| LeyCoreError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn ensure_regular_metadata_file(path: &Path) -> Result<fs::Metadata, LeyCoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LeyCoreError::UnsafeProjectLayout(path.to_path_buf()));
    }
    Ok(metadata)
}

fn optional_regular_metadata_file(path: &Path) -> Result<bool, LeyCoreError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(LeyCoreError::UnsafeProjectLayout(path.to_path_buf()))
        }
        Ok(_) => Ok(true),
    }
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), LeyCoreError> {
    let mut body =
        serde_json::to_vec_pretty(value).expect("serializing project metadata cannot fail");
    body.push(b'\n');
    let temporary = path.with_extension(format!("tmp-{}", Uuid::new_v4().simple()));
    write_new_file(&temporary, &body)?;
    fs::rename(&temporary, path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_new_file(path: &Path, body: &[u8]) -> Result<(), LeyCoreError> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|source| LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    file.write_all(body).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    file.sync_all().map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })
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
    use tempfile::tempdir;

    #[test]
    fn initializes_only_the_approved_project_metadata() {
        let directory = tempdir().unwrap();
        let initialized =
            initialize_project(directory.path(), Some("Example"), CaptureMode::Structured).unwrap();

        assert!(initialized.created);
        assert_eq!(initialized.identity.name, "Example");
        assert!(initialized.identity.project_id.starts_with("prj_"));
        assert_eq!(initialized.capture.mode, CaptureMode::Structured);
        assert!(!initialized.capture.store_raw_transcripts);

        let mut entries = fs::read_dir(directory.path().join(LEY_DIRECTORY))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        entries.sort();
        assert_eq!(entries, vec![IGNORE_FILE, CAPTURE_FILE, PROJECT_FILE]);
        assert!(
            fs::read_to_string(directory.path().join(LEY_DIRECTORY).join(PROJECT_FILE))
                .unwrap()
                .contains("\"projectId\"")
        );
    }

    #[test]
    fn initialization_is_idempotent_and_does_not_change_capture_consent() {
        let directory = tempdir().unwrap();
        let first = initialize_project(directory.path(), None, CaptureMode::Minimal).unwrap();
        let second =
            initialize_project(directory.path(), Some("Renamed"), CaptureMode::FullEvidence)
                .unwrap();

        assert!(!second.created);
        assert_eq!(second.identity, first.identity);
        assert_eq!(second.capture.mode, CaptureMode::Minimal);
        assert!(!second.capture.store_raw_transcripts);
    }

    #[test]
    fn doctor_discovers_parent_project_and_rejects_unsafe_policy() {
        let directory = tempdir().unwrap();
        initialize_project(directory.path(), None, CaptureMode::FullEvidence).unwrap();
        let nested = directory.path().join("src/feature");
        fs::create_dir_all(&nested).unwrap();
        let diagnostic = diagnose_project(&nested).unwrap();
        assert_eq!(diagnostic.capture.mode, CaptureMode::FullEvidence);
        assert!(diagnostic.capture.store_raw_transcripts);

        let capture_path = directory.path().join(LEY_DIRECTORY).join(CAPTURE_FILE);
        let mut capture: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&capture_path).unwrap()).unwrap();
        capture["approvedRoots"] = serde_json::json!(["../outside"]);
        fs::write(&capture_path, serde_json::to_vec_pretty(&capture).unwrap()).unwrap();
        assert!(matches!(
            diagnose_project(&nested),
            Err(LeyCoreError::InvalidCapturePolicy(_))
        ));
    }

    #[test]
    fn doctor_rejects_duplicate_roots_and_oversized_metadata() {
        let directory = tempdir().unwrap();
        initialize_project(directory.path(), None, CaptureMode::Structured).unwrap();
        let capture_path = directory.path().join(LEY_DIRECTORY).join(CAPTURE_FILE);
        let mut capture: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&capture_path).unwrap()).unwrap();
        capture["approvedRoots"] = serde_json::json!([".", "."]);
        fs::write(&capture_path, serde_json::to_vec_pretty(&capture).unwrap()).unwrap();
        assert!(matches!(
            diagnose_project(directory.path()),
            Err(LeyCoreError::InvalidCapturePolicy(_))
        ));

        fs::write(
            &capture_path,
            vec![b' '; (METADATA_FILE_LIMIT_BYTES + 1) as usize],
        )
        .unwrap();
        assert!(matches!(
            diagnose_project(directory.path()),
            Err(LeyCoreError::MetadataTooLarge { .. })
        ));
    }

    #[test]
    fn capture_preview_is_sorted_hermetic_and_layers_ignore_rules() {
        let directory = tempdir().unwrap();
        initialize_project(directory.path(), None, CaptureMode::Structured).unwrap();
        fs::create_dir_all(directory.path().join(".github/workflows")).unwrap();
        fs::create_dir_all(directory.path().join("target")).unwrap();
        fs::write(directory.path().join("b.rs"), b"b").unwrap();
        fs::write(directory.path().join("a.rs"), b"a").unwrap();
        fs::write(directory.path().join("ignored.txt"), b"ignored").unwrap();
        fs::write(directory.path().join(".env"), b"TOKEN=value").unwrap();
        fs::write(directory.path().join("target/generated.rs"), b"generated").unwrap();
        fs::write(
            directory.path().join(".github/workflows/check.yml"),
            b"name: check",
        )
        .unwrap();
        fs::write(directory.path().join(".gitignore"), b"ignored.txt\n").unwrap();

        let preview = preview_capture(directory.path()).unwrap();
        let paths = preview
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![".github/workflows/check.yml", ".gitignore", "a.rs", "b.rs"]
        );
        assert!(preview.skipped_oversized.is_empty());
        assert!(preview.skipped_total_limit.is_empty());
    }

    #[test]
    fn capture_preview_reports_limits_without_reading_file_contents() {
        let directory = tempdir().unwrap();
        initialize_project(directory.path(), None, CaptureMode::Structured).unwrap();
        fs::create_dir(directory.path().join("src")).unwrap();
        fs::write(directory.path().join("src/a.txt"), b"aaa").unwrap();
        fs::write(directory.path().join("src/b.txt"), b"bbb").unwrap();
        fs::write(directory.path().join("src/c.txt"), b"ccccc").unwrap();
        fs::write(directory.path().join("src/ignored.txt"), b"ignored").unwrap();
        fs::write(directory.path().join(".gitignore"), b"src/ignored.txt\n").unwrap();

        let capture_path = directory.path().join(LEY_DIRECTORY).join(CAPTURE_FILE);
        let mut capture: CapturePolicy = read_json(&capture_path).unwrap();
        capture.approved_roots = vec!["src".to_owned()];
        capture.max_file_bytes = 4;
        capture.max_total_bytes = 4;
        fs::write(&capture_path, serde_json::to_vec_pretty(&capture).unwrap()).unwrap();

        let preview = preview_capture(directory.path()).unwrap();
        assert_eq!(
            preview.files,
            vec![CaptureFile {
                path: "src/a.txt".to_owned(),
                bytes: 3
            }]
        );
        assert_eq!(
            preview.skipped_total_limit,
            vec![CaptureFile {
                path: "src/b.txt".to_owned(),
                bytes: 3
            }]
        );
        assert_eq!(
            preview.skipped_oversized,
            vec![CaptureFile {
                path: "src/c.txt".to_owned(),
                bytes: 5
            }]
        );
        assert!(!preview
            .files
            .iter()
            .chain(&preview.skipped_oversized)
            .chain(&preview.skipped_total_limit)
            .any(|file| file.path == "src/ignored.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn project_metadata_cannot_escape_through_symlinks() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().unwrap();
        let outside = tempdir().unwrap();
        initialize_project(outside.path(), Some("Outside"), CaptureMode::Structured).unwrap();
        symlink(
            outside.path().join(LEY_DIRECTORY),
            directory.path().join(LEY_DIRECTORY),
        )
        .unwrap();

        assert!(matches!(
            diagnose_project(directory.path()),
            Err(LeyCoreError::UnsafeProjectLayout(_))
        ));
        assert!(matches!(
            initialize_project(directory.path(), None, CaptureMode::Structured),
            Err(LeyCoreError::UnsafeProjectLayout(_))
        ));

        let file_link_project = tempdir().unwrap();
        initialize_project(
            file_link_project.path(),
            Some("File link"),
            CaptureMode::Structured,
        )
        .unwrap();
        let capture_path = file_link_project
            .path()
            .join(LEY_DIRECTORY)
            .join(CAPTURE_FILE);
        fs::remove_file(&capture_path).unwrap();
        symlink(
            outside.path().join(LEY_DIRECTORY).join(CAPTURE_FILE),
            &capture_path,
        )
        .unwrap();
        assert!(matches!(
            diagnose_project(file_link_project.path()),
            Err(LeyCoreError::UnsafeProjectLayout(_))
        ));
    }
}
