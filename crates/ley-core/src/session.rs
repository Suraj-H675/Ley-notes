use crate::ingestion::{
    load_project_memory, lock_project_memory_lifecycle, redact_secrets, ProjectMemoryLifecycleLock,
};
use crate::learning::erase_learnings_citing_session_under_lifecycle;
use crate::{diagnose_project, project_memory_overview, LeyCoreError, RedactionFinding};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The schema version used by turn-evidence events and their derived projection.
///
/// The original four lifecycle events remain schema version 1 so existing
/// ledgers are never upgraded merely by being read or by a normal lifecycle
/// mutation.
pub const SESSION_SCHEMA_VERSION: u32 = 2;
const SESSION_V1_SCHEMA_VERSION: u32 = 1;
pub const SESSION_EVENT_LIMIT_BYTES: u64 = 1_048_576;
pub const SESSION_PROJECTION_LIMIT_BYTES: u64 = 67_108_864;
pub const SESSION_EVENT_LIMIT: usize = 10_000;
pub const SESSION_TURN_EVIDENCE_LIMIT_BYTES: usize = 1_048_576;
pub const SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS: usize = 4_000;
pub const SESSION_RESPONSE_EVIDENCE_LIMIT_CHARACTERS: usize = 8_000;

const STORE_ROOT: &str = ".ley";
const AGENT_MEMORY_DIRECTORY: &str = "agent-memory";
const PROJECTS_DIRECTORY: &str = "projects";
const SESSIONS_DIRECTORY: &str = "sessions";
const EVENTS_DIRECTORY: &str = "events";
const SESSION_LOCK_FILE: &str = "sessions-v1.lock";
const SESSION_FILE: &str = "session-v1.json";
const SESSION_V2_FILE: &str = "session-v2.json";
const SESSION_MARKDOWN_FILE: &str = "session.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionStatus {
    Active,
    Completed,
    Paused,
    Abandoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionSourceKind {
    ManualCli,
    HostHook,
    Mcp,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionSource {
    pub kind: SessionSourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

impl Default for SessionSource {
    fn default() -> Self {
        Self {
            kind: SessionSourceKind::ManualCli,
            host: None,
            agent: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttemptOutcome {
    Helped,
    NoEffect,
    Worsened,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationStatus {
    Passed,
    Failed,
    Skipped,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanItemInput {
    pub text: String,
    pub status: PlanStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecisionInput {
    pub title: String,
    pub decision: String,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskInput {
    pub title: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttemptInput {
    pub action: String,
    pub outcome: AttemptOutcome,
    #[serde(default)]
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolutionInput {
    pub root_cause: String,
    pub change: String,
    #[serde(default)]
    pub verification: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProblemInput {
    pub title: String,
    pub symptom: String,
    #[serde(default)]
    pub expected: String,
    #[serde(default)]
    pub attempts: Vec<AttemptInput>,
    #[serde(default)]
    pub resolution: Option<ResolutionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandInput {
    pub command: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerificationInput {
    pub kind: String,
    pub status: VerificationStatus,
    pub summary: String,
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartSessionInput {
    pub request_id: String,
    pub name: String,
    pub goal: String,
    #[serde(default)]
    pub source: SessionSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckpointInput {
    pub request_id: String,
    pub summary: String,
    #[serde(default)]
    pub plan: Vec<PlanItemInput>,
    #[serde(default)]
    pub decisions: Vec<DecisionInput>,
    #[serde(default)]
    pub tasks: Vec<TaskInput>,
    #[serde(default)]
    pub problems: Vec<ProblemInput>,
    #[serde(default)]
    pub touched_artifacts: Vec<String>,
    #[serde(default)]
    pub commands: Vec<CommandInput>,
    #[serde(default)]
    pub verification: Vec<VerificationInput>,
    #[serde(default)]
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinishSessionInput {
    pub request_id: String,
    pub status: SessionStatus,
    pub summary: String,
    #[serde(default)]
    pub final_response: String,
    #[serde(default)]
    pub handoff: String,
    #[serde(default)]
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameSessionInput {
    pub request_id: String,
    #[serde(default)]
    pub expected_event_count: Option<u64>,
    pub name: String,
    pub note: String,
}

/// The producer that observed a turn. Turn evidence deliberately has a
/// narrower origin vocabulary than session creation: these records are either
/// trusted host-hook observations or an explicit local CLI action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TurnEvidenceOrigin {
    HostHook,
    ManualCli,
}

/// Whether a turn body was retained. Minimal capture and an exhausted
/// per-session capacity both still append a disclosure event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TurnEvidenceRetention {
    Captured,
    OmittedMinimal,
    OmittedCapacity,
}

/// Input shared by prompt and response evidence recording.
///
/// `correlation_material` is accepted only to derive an opaque `trn_` value.
/// It is never persisted, hashed into the request fingerprint, or exposed by
/// a derived projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TurnEvidenceInput {
    pub request_id: String,
    pub origin: TurnEvidenceOrigin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_material: Option<String>,
    pub text: String,
}

/// An observed, append-only prompt or response record reconstructed from an
/// immutable event. `text` is absent for disclosure-only captures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionTurnEvidence {
    pub record_id: String,
    pub event_id: String,
    pub sequence: u64,
    pub recorded_at_unix_ms: u64,
    pub origin: TurnEvidenceOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_reference: Option<String>,
    pub capture_mode: crate::CaptureMode,
    pub retention: TurnEvidenceRetention,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EraseSessionMemoryInput {
    pub expected_event_count: u64,
    pub expected_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryRedaction {
    pub field: String,
    pub kind: String,
    pub lines: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionArtifactCitation {
    pub artifact_path: String,
    pub artifact_snapshot_id: String,
    pub content_hash: String,
    pub start_line: u64,
    pub end_line: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionProjectRevision {
    pub graph_snapshot_id: String,
    pub artifact_snapshot_id: String,
    pub captured_at_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub tracked_changes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanItem {
    pub id: String,
    pub text: String,
    pub status: PlanStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecisionRecord {
    pub id: String,
    pub title: String,
    pub decision: String,
    pub rationale: String,
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttemptRecord {
    pub id: String,
    pub action: String,
    pub outcome: AttemptOutcome,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolutionRecord {
    pub id: String,
    pub root_cause: String,
    pub change: String,
    pub verification: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProblemRecord {
    pub id: String,
    pub title: String,
    pub symptom: String,
    pub expected: String,
    pub attempts: Vec<AttemptRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ResolutionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandRecord {
    pub id: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerificationRecord {
    pub id: String,
    pub kind: String,
    pub status: VerificationStatus,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionCheckpoint {
    pub id: String,
    pub event_id: String,
    pub recorded_at_unix_ms: u64,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_revision: Option<SessionProjectRevision>,
    pub plan: Vec<PlanItem>,
    pub decisions: Vec<DecisionRecord>,
    pub tasks: Vec<TaskRecord>,
    pub problems: Vec<ProblemRecord>,
    pub touched_artifacts: Vec<SessionArtifactCitation>,
    pub commands: Vec<CommandRecord>,
    pub verification: Vec<VerificationRecord>,
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionFinish {
    pub event_id: String,
    pub recorded_at_unix_ms: u64,
    pub status: SessionStatus,
    pub summary: String,
    pub final_response: String,
    pub handoff: String,
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionRename {
    pub event_id: String,
    pub recorded_at_unix_ms: u64,
    pub name: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentSession {
    pub schema_version: u32,
    pub project_id: String,
    pub session_id: String,
    pub original_name: String,
    pub name: String,
    pub goal: String,
    pub status: SessionStatus,
    pub source: SessionSource,
    pub artifact_snapshot_id_at_start: String,
    pub started_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_unix_ms: Option<u64>,
    pub event_count: u64,
    pub checkpoints: Vec<SessionCheckpoint>,
    #[serde(default)]
    pub prompts: Vec<SessionTurnEvidence>,
    #[serde(default)]
    pub responses: Vec<SessionTurnEvidence>,
    pub renames: Vec<SessionRename>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish: Option<SessionFinish>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionSummary {
    pub project_id: String,
    pub session_id: String,
    pub name: String,
    pub goal: String,
    pub status: SessionStatus,
    pub started_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub event_count: u64,
    pub checkpoints: usize,
    pub prompts: usize,
    pub responses: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionMutation {
    pub session: AgentSession,
    pub event_id: String,
    pub replayed: bool,
    pub session_path: String,
    pub markdown_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionEvent {
    schema_version: u32,
    event_id: String,
    project_id: String,
    session_id: String,
    request_id: String,
    request_fingerprint: String,
    sequence: u64,
    recorded_at_unix_ms: u64,
    redactions: Vec<MemoryRedaction>,
    #[serde(flatten)]
    payload: SessionEventPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum SessionEventPayload {
    SessionStarted {
        name: String,
        goal: String,
        source: SessionSource,
        artifact_snapshot_id: String,
    },
    CheckpointRecorded(Box<SessionCheckpoint>),
    SessionFinished(SessionFinish),
    SessionRenamed(SessionRename),
    UserPromptObserved(SessionTurnEvidence),
    AssistantResponseObserved(SessionTurnEvidence),
}

pub fn generate_request_id() -> String {
    format!("req_{}", uuid::Uuid::new_v4().simple())
}

/// Derives a stable opaque turn reference from correlation material supplied by
/// a trusted adapter. The material itself must stay in the adapter process and
/// must never be put into a session input other than this transient field.
pub fn derive_turn_reference(correlation_material: &str) -> String {
    deterministic_id(
        "trn",
        &format!("ley-turn-reference-v1:{correlation_material}"),
        64,
    )
}

/// Records an observed user prompt as a first-class immutable event.
pub fn record_session_prompt(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: TurnEvidenceInput,
) -> Result<SessionMutation, LeyCoreError> {
    record_session_turn(project_start, vault, session_id, input, true)
}

/// Records an observed assistant response as a first-class immutable event.
pub fn record_session_response(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: TurnEvidenceInput,
) -> Result<SessionMutation, LeyCoreError> {
    record_session_turn(project_start, vault, session_id, input, false)
}

fn record_session_turn(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: TurnEvidenceInput,
    is_prompt: bool,
) -> Result<SessionMutation, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
    let kind = if is_prompt {
        "user-prompt-observed"
    } else {
        "assistant-response-observed"
    };
    let event_id = deterministic_id(
        "evt",
        &format!("{session_id}:{}:{kind}", input.request_id),
        64,
    );
    let request_id = input.request_id.clone();
    let (evidence, redactions) = normalize_turn_evidence(
        input,
        &event_id,
        diagnostic.capture.mode,
        if is_prompt {
            SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS
        } else {
            SESSION_RESPONSE_EVIDENCE_LIMIT_CHARACTERS
        },
    )?;
    let payload = if is_prompt {
        SessionEventPayload::UserPromptObserved(evidence)
    } else {
        SessionEventPayload::AssistantResponseObserved(evidence)
    };
    mutate_session(
        &diagnostic.identity.project_id,
        session_id,
        PendingEvent {
            event_id,
            request_id,
            redactions,
            payload,
            schema_version: SESSION_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: None,
        },
        vault,
    )
}

pub fn start_session(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    input: StartSessionInput,
) -> Result<SessionMutation, LeyCoreError> {
    validate_request_id(&input.request_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    let memory = project_memory_overview(&diagnostic.root, &vault)?;
    let mut redactions = Vec::new();
    let name = sanitize_text("name", &input.name, 1, 128, &mut redactions)?;
    let goal = sanitize_text("goal", &input.goal, 1, 16_000, &mut redactions)?;
    let source = sanitize_source(input.source, &mut redactions)?;
    let session_id = deterministic_id(
        "ses",
        &format!("{}:{}", diagnostic.identity.project_id, input.request_id),
        32,
    );
    let event_id = deterministic_id(
        "evt",
        &format!("{session_id}:{}:session-started", input.request_id),
        64,
    );
    let payload = SessionEventPayload::SessionStarted {
        name,
        goal,
        source,
        artifact_snapshot_id: memory.artifact_snapshot_id,
    };
    mutate_session(
        &diagnostic.identity.project_id,
        &session_id,
        PendingEvent {
            event_id,
            request_id: input.request_id,
            redactions,
            payload,
            schema_version: SESSION_V1_SCHEMA_VERSION,
            allow_create: true,
            expected_event_count: None,
        },
        vault,
    )
}

pub fn checkpoint_session(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: CheckpointInput,
) -> Result<SessionMutation, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    let request_id = input.request_id.clone();
    let diagnostic = diagnose_project(&project_start)?;
    let memory = load_project_memory(&diagnostic.root, &vault)?;
    let event_id = deterministic_id(
        "evt",
        &format!("{session_id}:{}:checkpoint-recorded", input.request_id),
        64,
    );
    let recorded_at = unix_time_ms();
    let (checkpoint, redactions) = normalize_checkpoint(input, &event_id, recorded_at, &memory)?;
    mutate_session(
        &diagnostic.identity.project_id,
        session_id,
        PendingEvent {
            event_id,
            request_id,
            redactions,
            payload: SessionEventPayload::CheckpointRecorded(Box::new(checkpoint)),
            schema_version: SESSION_V1_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: None,
        },
        vault,
    )
}

pub fn finish_session(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: FinishSessionInput,
) -> Result<SessionMutation, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    if input.status == SessionStatus::Active {
        return Err(LeyCoreError::InvalidSessionRequest(
            "finished session status cannot be active".to_owned(),
        ));
    }
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
    let event_id = deterministic_id(
        "evt",
        &format!("{session_id}:{}:session-finished", input.request_id),
        64,
    );
    let recorded_at = unix_time_ms();
    let mut redactions = Vec::new();
    let finish = SessionFinish {
        event_id: event_id.clone(),
        recorded_at_unix_ms: recorded_at,
        status: input.status,
        summary: sanitize_text("summary", &input.summary, 1, 16_000, &mut redactions)?,
        final_response: sanitize_text(
            "finalResponse",
            &input.final_response,
            0,
            32_000,
            &mut redactions,
        )?,
        handoff: sanitize_text("handoff", &input.handoff, 0, 16_000, &mut redactions)?,
        unresolved: sanitize_list("unresolved", input.unresolved, 100, 4_000, &mut redactions)?,
    };
    mutate_session(
        &diagnostic.identity.project_id,
        session_id,
        PendingEvent {
            event_id,
            request_id: input.request_id,
            redactions,
            payload: SessionEventPayload::SessionFinished(finish),
            schema_version: SESSION_V1_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: None,
        },
        vault,
    )
}

pub fn rename_session(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: RenameSessionInput,
) -> Result<SessionMutation, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
    let event_id = deterministic_id(
        "evt",
        &format!("{session_id}:{}:session-renamed", input.request_id),
        64,
    );
    let recorded_at = unix_time_ms();
    let mut redactions = Vec::new();
    let rename = SessionRename {
        event_id: event_id.clone(),
        recorded_at_unix_ms: recorded_at,
        name: sanitize_text("name", &input.name, 1, 128, &mut redactions)?,
        note: sanitize_text("note", &input.note, 1, 4_000, &mut redactions)?,
    };
    mutate_session(
        &diagnostic.identity.project_id,
        session_id,
        PendingEvent {
            event_id,
            request_id: input.request_id,
            redactions,
            payload: SessionEventPayload::SessionRenamed(rename),
            schema_version: SESSION_V1_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: input.expected_event_count,
        },
        vault,
    )
}

pub fn read_session(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
) -> Result<AgentSession, LeyCoreError> {
    validate_session_id(session_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
    let Some(store) = SessionStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Err(LeyCoreError::SessionNotFound(session_id.to_owned()));
    };
    let _lock = store.lock(true)?;
    store.rebuild_session(session_id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMemoryErasure {
    pub project_id: String,
    pub session_id: String,
    pub session_name: String,
    pub erased_learning_ids: Vec<String>,
    pub ordinary_notes_preserved: bool,
    pub canvas_documents_preserved: bool,
    pub project_evidence_preserved: bool,
}

pub fn erase_session_memory(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: EraseSessionMemoryInput,
) -> Result<SessionMemoryErasure, LeyCoreError> {
    validate_session_id(session_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
    let vault_path = vault
        .as_ref()
        .canonicalize()
        .map_err(|source| LeyCoreError::Io {
            path: vault.as_ref().to_path_buf(),
            source,
        })?;
    let _lifecycle =
        lock_project_memory_lifecycle(&vault_path, &diagnostic.identity.project_id, false, true)?;
    let vault_dir = Dir::open_ambient_dir(&vault_path, ambient_authority()).map_err(|source| {
        LeyCoreError::Io {
            path: vault_path.clone(),
            source,
        }
    })?;
    let ley_dir = open_existing_dir(&vault_dir, STORE_ROOT)?;
    let memory_dir = open_existing_dir(&ley_dir, AGENT_MEMORY_DIRECTORY)?;
    let projects_dir = open_existing_dir(&memory_dir, PROJECTS_DIRECTORY)?;
    let project_dir = open_existing_dir(&projects_dir, &diagnostic.identity.project_id)?;
    let sessions_dir = open_existing_dir(&project_dir, SESSIONS_DIRECTORY)?;
    let session_dir = sessions_dir
        .open_dir_nofollow(session_id)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                LeyCoreError::SessionNotFound(session_id.to_owned())
            } else {
                session_io(session_id, error)
            }
        })?;
    let events = read_session_events(&session_dir, &diagnostic.identity.project_id, session_id)?;
    let session = replay_events(&events, &diagnostic.identity.project_id, session_id)?;
    if session.event_count != input.expected_event_count {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "session changed from {} events to {}; reload before erasing",
            input.expected_event_count, session.event_count
        )));
    }
    if session.name != input.expected_name {
        return Err(LeyCoreError::InvalidSessionRequest(
            "session name changed; reload and type the current name before erasing".to_owned(),
        ));
    }

    let erased_learning_ids = erase_learnings_citing_session_under_lifecycle(
        &project_dir,
        &diagnostic.identity.project_id,
        session_id,
    )?;
    sessions_dir
        .remove_dir_all(session_id)
        .map_err(|source| session_io(session_id, source))?;
    Ok(SessionMemoryErasure {
        project_id: diagnostic.identity.project_id,
        session_id: session_id.to_owned(),
        session_name: session.name,
        erased_learning_ids,
        ordinary_notes_preserved: true,
        canvas_documents_preserved: true,
        project_evidence_preserved: true,
    })
}

pub fn list_sessions(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
) -> Result<Vec<SessionSummary>, LeyCoreError> {
    let mut sessions = Vec::new();
    visit_session_records(project_start, vault, |session| {
        sessions.push(SessionSummary::from(&session));
    })?;
    sessions.sort_by(|left, right| {
        right
            .updated_at_unix_ms
            .cmp(&left.updated_at_unix_ms)
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    Ok(sessions)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSessionStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub paused_sessions: usize,
    pub completed_sessions: usize,
    pub abandoned_sessions: usize,
}

pub fn project_session_stats(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
) -> Result<ProjectSessionStats, LeyCoreError> {
    let mut stats = ProjectSessionStats::default();
    visit_session_records(project_start, vault, |session| {
        stats.total_sessions += 1;
        match session.status {
            SessionStatus::Active => stats.active_sessions += 1,
            SessionStatus::Paused => stats.paused_sessions += 1,
            SessionStatus::Completed => stats.completed_sessions += 1,
            SessionStatus::Abandoned => stats.abandoned_sessions += 1,
        }
    })?;
    Ok(stats)
}

pub(crate) fn visit_session_records(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    mut visitor: impl FnMut(AgentSession),
) -> Result<usize, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
    let Some(store) = SessionStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Ok(0);
    };
    let _lock = store.lock(true)?;
    let session_ids = store.session_ids()?;
    let total_sessions = session_ids.len();
    for session_id in session_ids {
        visitor(store.rebuild_session(&session_id)?);
    }
    Ok(total_sessions)
}

impl From<&AgentSession> for SessionSummary {
    fn from(session: &AgentSession) -> Self {
        Self {
            project_id: session.project_id.clone(),
            session_id: session.session_id.clone(),
            name: session.name.clone(),
            goal: session.goal.clone(),
            status: session.status,
            started_at_unix_ms: session.started_at_unix_ms,
            updated_at_unix_ms: session.updated_at_unix_ms,
            event_count: session.event_count,
            checkpoints: session.checkpoints.len(),
            prompts: session.prompts.len(),
            responses: session.responses.len(),
        }
    }
}

fn normalize_checkpoint(
    input: CheckpointInput,
    event_id: &str,
    recorded_at: u64,
    memory: &crate::ingestion::LoadedProjectMemory,
) -> Result<(SessionCheckpoint, Vec<MemoryRedaction>), LeyCoreError> {
    if input.plan.len() > 100
        || input.decisions.len() > 100
        || input.tasks.len() > 100
        || input.problems.len() > 50
        || input.commands.len() > 200
        || input.verification.len() > 200
        || input.touched_artifacts.len() > 200
    {
        return Err(LeyCoreError::InvalidSessionRequest(
            "checkpoint collection limits were exceeded".to_owned(),
        ));
    }
    let mut redactions = Vec::new();
    let summary = sanitize_text("summary", &input.summary, 1, 16_000, &mut redactions)?;
    let mut plan = Vec::new();
    for (index, item) in input.plan.into_iter().enumerate() {
        plan.push(PlanItem {
            id: child_id("pln", event_id, index),
            text: sanitize_text(
                &format!("plan[{index}].text"),
                &item.text,
                1,
                4_000,
                &mut redactions,
            )?,
            status: item.status,
        });
    }
    let mut decisions = Vec::new();
    for (index, item) in input.decisions.into_iter().enumerate() {
        decisions.push(DecisionRecord {
            id: child_id("dec", event_id, index),
            title: sanitize_text(
                &format!("decisions[{index}].title"),
                &item.title,
                1,
                256,
                &mut redactions,
            )?,
            decision: sanitize_text(
                &format!("decisions[{index}].decision"),
                &item.decision,
                1,
                8_000,
                &mut redactions,
            )?,
            rationale: sanitize_text(
                &format!("decisions[{index}].rationale"),
                &item.rationale,
                0,
                8_000,
                &mut redactions,
            )?,
            alternatives: sanitize_list(
                &format!("decisions[{index}].alternatives"),
                item.alternatives,
                20,
                2_000,
                &mut redactions,
            )?,
        });
    }
    let mut tasks = Vec::new();
    for (index, item) in input.tasks.into_iter().enumerate() {
        tasks.push(TaskRecord {
            id: child_id("tsk", event_id, index),
            title: sanitize_text(
                &format!("tasks[{index}].title"),
                &item.title,
                1,
                256,
                &mut redactions,
            )?,
            status: item.status,
            details: sanitize_text(
                &format!("tasks[{index}].details"),
                &item.details,
                0,
                4_000,
                &mut redactions,
            )?,
        });
    }
    let mut problems = Vec::new();
    for (problem_index, item) in input.problems.into_iter().enumerate() {
        if item.attempts.len() > 50 {
            return Err(LeyCoreError::InvalidSessionRequest(
                "a problem cannot contain more than 50 attempts".to_owned(),
            ));
        }
        let problem_id = child_id("prb", event_id, problem_index);
        let mut attempts = Vec::new();
        for (attempt_index, attempt) in item.attempts.into_iter().enumerate() {
            attempts.push(AttemptRecord {
                id: child_id("att", &problem_id, attempt_index),
                action: sanitize_text(
                    &format!("problems[{problem_index}].attempts[{attempt_index}].action"),
                    &attempt.action,
                    1,
                    8_000,
                    &mut redactions,
                )?,
                outcome: attempt.outcome,
                evidence: sanitize_text(
                    &format!("problems[{problem_index}].attempts[{attempt_index}].evidence"),
                    &attempt.evidence,
                    0,
                    8_000,
                    &mut redactions,
                )?,
            });
        }
        let resolution = item
            .resolution
            .map(|resolution| {
                Ok(ResolutionRecord {
                    id: child_id("res", &problem_id, 0),
                    root_cause: sanitize_text(
                        &format!("problems[{problem_index}].resolution.rootCause"),
                        &resolution.root_cause,
                        1,
                        8_000,
                        &mut redactions,
                    )?,
                    change: sanitize_text(
                        &format!("problems[{problem_index}].resolution.change"),
                        &resolution.change,
                        1,
                        8_000,
                        &mut redactions,
                    )?,
                    verification: sanitize_text(
                        &format!("problems[{problem_index}].resolution.verification"),
                        &resolution.verification,
                        0,
                        8_000,
                        &mut redactions,
                    )?,
                })
            })
            .transpose()?;
        problems.push(ProblemRecord {
            id: problem_id,
            title: sanitize_text(
                &format!("problems[{problem_index}].title"),
                &item.title,
                1,
                256,
                &mut redactions,
            )?,
            symptom: sanitize_text(
                &format!("problems[{problem_index}].symptom"),
                &item.symptom,
                1,
                8_000,
                &mut redactions,
            )?,
            expected: sanitize_text(
                &format!("problems[{problem_index}].expected"),
                &item.expected,
                0,
                8_000,
                &mut redactions,
            )?,
            attempts,
            resolution,
        });
    }
    let touched_artifacts =
        citations_for_paths(memory, input.touched_artifacts.into_iter().collect())?;
    let project_revision = Some(SessionProjectRevision {
        graph_snapshot_id: memory.graph.graph_snapshot_id.clone(),
        artifact_snapshot_id: memory.graph.artifact_snapshot_id.clone(),
        captured_at_unix_ms: memory.graph.generated_at_unix_ms,
        head: memory.graph.git.as_ref().and_then(|git| git.head.clone()),
        branch: memory.graph.git.as_ref().and_then(|git| git.branch.clone()),
        tracked_changes: memory
            .graph
            .git
            .as_ref()
            .map_or(0, |git| git.changes.len() as u64),
    });
    let mut commands = Vec::new();
    for (index, item) in input.commands.into_iter().enumerate() {
        commands.push(CommandRecord {
            id: child_id("cmd", event_id, index),
            command: sanitize_text(
                &format!("commands[{index}].command"),
                &item.command,
                1,
                8_000,
                &mut redactions,
            )?,
            exit_code: item.exit_code,
            summary: sanitize_text(
                &format!("commands[{index}].summary"),
                &item.summary,
                0,
                4_000,
                &mut redactions,
            )?,
        });
    }
    let mut verification = Vec::new();
    for (index, item) in input.verification.into_iter().enumerate() {
        verification.push(VerificationRecord {
            id: child_id("ver", event_id, index),
            kind: sanitize_text(
                &format!("verification[{index}].kind"),
                &item.kind,
                1,
                64,
                &mut redactions,
            )?,
            status: item.status,
            summary: sanitize_text(
                &format!("verification[{index}].summary"),
                &item.summary,
                1,
                8_000,
                &mut redactions,
            )?,
            command: item
                .command
                .map(|command| {
                    sanitize_text(
                        &format!("verification[{index}].command"),
                        &command,
                        1,
                        8_000,
                        &mut redactions,
                    )
                })
                .transpose()?,
        });
    }
    Ok((
        SessionCheckpoint {
            id: child_id("ckp", event_id, 0),
            event_id: event_id.to_owned(),
            recorded_at_unix_ms: recorded_at,
            summary,
            project_revision,
            plan,
            decisions,
            tasks,
            problems,
            touched_artifacts,
            commands,
            verification,
            unresolved: sanitize_list("unresolved", input.unresolved, 100, 4_000, &mut redactions)?,
        },
        redactions,
    ))
}

fn citations_for_paths(
    memory: &crate::ingestion::LoadedProjectMemory,
    paths: Vec<String>,
) -> Result<Vec<SessionArtifactCitation>, LeyCoreError> {
    let mut unique = BTreeSet::new();
    let mut citations = Vec::new();
    for path in paths {
        if !unique.insert(path.clone()) {
            continue;
        }
        let artifact = memory
            .manifest
            .files
            .iter()
            .find(|artifact| artifact.path == path)
            .ok_or_else(|| {
                LeyCoreError::InvalidSessionRequest(format!(
                    "touched artifact is not in the current approved snapshot: {path}"
                ))
            })?;
        citations.push(SessionArtifactCitation {
            artifact_path: artifact.path.clone(),
            artifact_snapshot_id: memory.manifest.snapshot_id.clone(),
            content_hash: artifact.content_hash.clone(),
            start_line: 1,
            end_line: artifact.line_count.max(1),
        });
    }
    citations.sort_by(|left, right| left.artifact_path.cmp(&right.artifact_path));
    Ok(citations)
}

fn normalize_turn_evidence(
    input: TurnEvidenceInput,
    event_id: &str,
    capture_mode: crate::CaptureMode,
    maximum_characters: usize,
) -> Result<(SessionTurnEvidence, Vec<MemoryRedaction>), LeyCoreError> {
    let host = input.host.map(sanitize_turn_host).transpose()?;
    let turn_reference = input
        .correlation_material
        .as_deref()
        .map(derive_validated_turn_reference)
        .transpose()?;
    let recorded_at_unix_ms = unix_time_ms();
    let record_id = child_id("tev", event_id, 0);
    if capture_mode == crate::CaptureMode::Minimal {
        return Ok((
            SessionTurnEvidence {
                record_id,
                event_id: event_id.to_owned(),
                sequence: 0,
                recorded_at_unix_ms,
                origin: input.origin,
                host,
                turn_reference,
                capture_mode,
                retention: TurnEvidenceRetention::OmittedMinimal,
                text: None,
                truncated: false,
            },
            Vec::new(),
        ));
    }

    let mut redactions = Vec::new();
    let (text, truncated) = sanitize_bounded_turn_text(
        "turnEvidence.text",
        &input.text,
        maximum_characters,
        &mut redactions,
    )?;
    Ok((
        SessionTurnEvidence {
            record_id,
            event_id: event_id.to_owned(),
            sequence: 0,
            recorded_at_unix_ms,
            origin: input.origin,
            host,
            turn_reference,
            capture_mode,
            retention: TurnEvidenceRetention::Captured,
            text,
            truncated,
        },
        redactions,
    ))
}

fn sanitize_turn_host(value: String) -> Result<String, LeyCoreError> {
    let value = value.trim();
    if is_valid_turn_host(value) {
        Ok(value.to_owned())
    } else {
        Err(LeyCoreError::InvalidSessionRequest(
            "turn evidence host must be codex or claude-code".to_owned(),
        ))
    }
}

fn is_valid_turn_host(value: &str) -> bool {
    matches!(value, "codex" | "claude-code")
}

fn derive_validated_turn_reference(correlation_material: &str) -> Result<String, LeyCoreError> {
    if correlation_material.is_empty()
        || correlation_material.chars().count() > 4_096
        || correlation_material.chars().any(|character| {
            character == '\0'
                || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        })
    {
        return Err(LeyCoreError::InvalidSessionRequest(
            "turn correlation material must contain at most 4096 safe characters".to_owned(),
        ));
    }
    Ok(derive_turn_reference(correlation_material))
}

fn sanitize_bounded_turn_text(
    field: &str,
    value: &str,
    maximum_characters: usize,
    redactions: &mut Vec<MemoryRedaction>,
) -> Result<(Option<String>, bool), LeyCoreError> {
    let value = value.trim();
    if value.chars().any(|character| {
        character == '\0' || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    }) {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "{field} must contain safe characters"
        )));
    }
    // Redact the supplied text before selecting the persisted visible window.
    // This keeps the post-redaction body within the stated character cap even
    // when a replacement marker is longer than the matched secret.
    let (sanitized, mut findings) = redact_secrets(value);
    let mut characters = sanitized.chars();
    let bounded: String = characters.by_ref().take(maximum_characters).collect();
    let truncated = characters.next().is_some();
    if bounded.is_empty() {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "{field} cannot be empty"
        )));
    }
    let retained_line_count = bounded.bytes().filter(|byte| *byte == b'\n').count() as u64 + 1;
    for finding in &mut findings {
        finding.lines.retain(|line| *line <= retained_line_count);
    }
    redactions.extend(
        findings
            .into_iter()
            .filter(|finding| !finding.lines.is_empty())
            .map(|RedactionFinding { kind, lines }| MemoryRedaction {
                field: field.to_owned(),
                kind,
                lines,
            }),
    );
    Ok((Some(bounded), truncated))
}

fn retained_turn_evidence_bytes(events: &[SessionEvent]) -> usize {
    events
        .iter()
        .filter_map(|event| turn_evidence(&event.payload))
        .filter_map(|evidence| evidence.text.as_deref())
        .map(str::len)
        .sum()
}

fn turn_evidence(payload: &SessionEventPayload) -> Option<&SessionTurnEvidence> {
    match payload {
        SessionEventPayload::UserPromptObserved(evidence)
        | SessionEventPayload::AssistantResponseObserved(evidence) => Some(evidence),
        _ => None,
    }
}

fn turn_evidence_mut(payload: &mut SessionEventPayload) -> Option<&mut SessionTurnEvidence> {
    match payload {
        SessionEventPayload::UserPromptObserved(evidence)
        | SessionEventPayload::AssistantResponseObserved(evidence) => Some(evidence),
        _ => None,
    }
}

struct PendingEvent {
    event_id: String,
    request_id: String,
    redactions: Vec<MemoryRedaction>,
    payload: SessionEventPayload,
    schema_version: u32,
    allow_create: bool,
    expected_event_count: Option<u64>,
}

fn mutate_session(
    project_id: &str,
    session_id: &str,
    mut pending: PendingEvent,
    vault: impl AsRef<Path>,
) -> Result<SessionMutation, LeyCoreError> {
    let store = SessionStore::open(&vault, project_id, pending.allow_create)?
        .ok_or_else(|| LeyCoreError::SessionNotFound(session_id.to_owned()))?;
    let _lock = store.lock(false)?;
    let session_dir = if pending.allow_create {
        store.open_or_create_session(session_id)?
    } else {
        store.open_session(session_id)?
    };
    let events_dir = if pending.allow_create {
        open_or_create_private_dir(&session_dir, EVENTS_DIRECTORY)?
    } else {
        open_existing_dir(&session_dir, EVENTS_DIRECTORY)?
    };
    let event_name = format!("{}.json", pending.event_id);
    let existing = store.read_events(session_id, &session_dir)?;
    if let Some(event) = existing
        .iter()
        .find(|event| event.event_id == pending.event_id)
    {
        align_turn_evidence_retry(&mut pending, &event.payload);
        let request_fingerprint = request_fingerprint(
            project_id,
            session_id,
            &pending.request_id,
            &pending.payload,
        )?;
        if event.request_fingerprint != request_fingerprint {
            return Err(LeyCoreError::SessionIdempotencyConflict(pending.request_id));
        }
        if !retry_payload_matches(&event.payload, &pending.payload) {
            return Err(LeyCoreError::SessionIdempotencyConflict(pending.request_id));
        }
        let session = store.rebuild_session_from_dir(session_id, &session_dir)?;
        store.persist_projection(&session_dir, &session)?;
        return Ok(mutation(session, &pending.event_id, true));
    }
    apply_turn_evidence_capacity(&mut pending, retained_turn_evidence_bytes(&existing));
    let request_fingerprint = request_fingerprint(
        project_id,
        session_id,
        &pending.request_id,
        &pending.payload,
    )?;
    if existing
        .iter()
        .any(|event| event.request_id == pending.request_id)
    {
        return Err(LeyCoreError::SessionIdempotencyConflict(pending.request_id));
    }
    if let Some(expected) = pending.expected_event_count {
        let actual = existing.len() as u64;
        if actual != expected {
            return Err(LeyCoreError::InvalidSessionRequest(format!(
                "session changed from {expected} events to {actual}; reload before saving"
            )));
        }
    }
    let sequence = existing.len() as u64 + 1;
    if sequence as usize > SESSION_EVENT_LIMIT {
        return Err(LeyCoreError::InvalidSessionStore(format!(
            "session exceeds {SESSION_EVENT_LIMIT} events"
        )));
    }
    if pending.allow_create && !existing.is_empty() {
        return Err(LeyCoreError::SessionIdempotencyConflict(pending.request_id));
    }
    if !pending.allow_create && existing.is_empty() {
        return Err(LeyCoreError::SessionNotFound(session_id.to_owned()));
    }
    if !existing.is_empty() {
        let current = replay_events(&existing, project_id, session_id)?;
        if current.status != SessionStatus::Active
            && !matches!(&pending.payload, SessionEventPayload::SessionRenamed(_))
        {
            return Err(LeyCoreError::InvalidSessionRequest(format!(
                "session {session_id} is already {}",
                enum_label(current.status)
            )));
        }
        if let SessionEventPayload::SessionRenamed(rename) = &pending.payload {
            if current.name == rename.name {
                return Err(LeyCoreError::InvalidSessionRequest(format!(
                    "session {session_id} is already named {}",
                    rename.name
                )));
            }
        }
    }
    if let Some(evidence) = turn_evidence_mut(&mut pending.payload) {
        evidence.sequence = sequence;
    }
    let minimum_recorded_at = existing
        .last()
        .map(|event| event.recorded_at_unix_ms)
        .unwrap_or(1);
    let recorded_at_unix_ms =
        normalize_payload_recorded_at(&mut pending.payload, minimum_recorded_at);
    let event = SessionEvent {
        schema_version: pending.schema_version,
        event_id: pending.event_id.clone(),
        project_id: project_id.to_owned(),
        session_id: session_id.to_owned(),
        request_id: pending.request_id,
        request_fingerprint,
        sequence,
        recorded_at_unix_ms,
        redactions: pending.redactions,
        payload: pending.payload,
    };
    let body = json_body(&event, SESSION_EVENT_LIMIT_BYTES, &event_name)?;
    write_immutable_private(&events_dir, &event_name, &body)?;
    let session = store.rebuild_session_from_dir(session_id, &session_dir)?;
    store.persist_projection(&session_dir, &session)?;
    Ok(mutation(session, &pending.event_id, false))
}

fn align_turn_evidence_retry(pending: &mut PendingEvent, stored: &SessionEventPayload) {
    let (Some(pending), Some(stored)) = (
        turn_evidence_mut(&mut pending.payload),
        turn_evidence(stored),
    ) else {
        return;
    };
    pending.sequence = stored.sequence;
    if stored.retention != TurnEvidenceRetention::Captured {
        // Capacity/minimal disclosures deliberately retain no body-derived
        // metadata. Preserve their original disclosure outcome on an exact
        // retry instead of reconsidering the now-current capacity.
        pending.capture_mode = stored.capture_mode;
        pending.retention = stored.retention;
        pending.text = None;
        pending.truncated = false;
    }
}

fn apply_turn_evidence_capacity(pending: &mut PendingEvent, retained_bytes: usize) {
    let Some(evidence) = turn_evidence_mut(&mut pending.payload) else {
        return;
    };
    let Some(text) = evidence.text.as_deref() else {
        return;
    };
    if retained_bytes.saturating_add(text.len()) > SESSION_TURN_EVIDENCE_LIMIT_BYTES {
        evidence.retention = TurnEvidenceRetention::OmittedCapacity;
        evidence.text = None;
        evidence.truncated = false;
        // Redaction metadata for an omitted body would disclose facts about
        // text that this event intentionally did not retain.
        pending.redactions.clear();
    }
}

fn retry_payload_matches(stored: &SessionEventPayload, pending: &SessionEventPayload) -> bool {
    match (stored, pending) {
        (
            SessionEventPayload::UserPromptObserved(stored),
            SessionEventPayload::UserPromptObserved(pending),
        )
        | (
            SessionEventPayload::AssistantResponseObserved(stored),
            SessionEventPayload::AssistantResponseObserved(pending),
        ) => {
            stored.record_id == pending.record_id
                && stored.event_id == pending.event_id
                && stored.sequence == pending.sequence
                && stored.origin == pending.origin
                && stored.host == pending.host
                && stored.turn_reference == pending.turn_reference
                && stored.capture_mode == pending.capture_mode
                && stored.retention == pending.retention
                && stored.text == pending.text
                && stored.truncated == pending.truncated
        }
        _ => true,
    }
}

fn mutation(session: AgentSession, event_id: &str, replayed: bool) -> SessionMutation {
    let base = format!(
        "{STORE_ROOT}/{AGENT_MEMORY_DIRECTORY}/{PROJECTS_DIRECTORY}/{}/{SESSIONS_DIRECTORY}/{}",
        session.project_id, session.session_id
    );
    let session_path = format!("{base}/{}", projection_file_name(&session));
    SessionMutation {
        session,
        event_id: event_id.to_owned(),
        replayed,
        session_path,
        markdown_path: format!("{base}/{SESSION_MARKDOWN_FILE}"),
    }
}

fn normalize_payload_recorded_at(payload: &mut SessionEventPayload, minimum: u64) -> u64 {
    match payload {
        SessionEventPayload::SessionStarted { .. } => unix_time_ms().max(minimum),
        SessionEventPayload::CheckpointRecorded(checkpoint) => {
            checkpoint.recorded_at_unix_ms = checkpoint.recorded_at_unix_ms.max(minimum);
            checkpoint.recorded_at_unix_ms
        }
        SessionEventPayload::SessionFinished(finish) => {
            finish.recorded_at_unix_ms = finish.recorded_at_unix_ms.max(minimum);
            finish.recorded_at_unix_ms
        }
        SessionEventPayload::SessionRenamed(rename) => {
            rename.recorded_at_unix_ms = rename.recorded_at_unix_ms.max(minimum);
            rename.recorded_at_unix_ms
        }
        SessionEventPayload::UserPromptObserved(evidence)
        | SessionEventPayload::AssistantResponseObserved(evidence) => {
            evidence.recorded_at_unix_ms = evidence.recorded_at_unix_ms.max(minimum);
            evidence.recorded_at_unix_ms
        }
    }
}

fn projection_file_name(session: &AgentSession) -> &'static str {
    if session.schema_version >= SESSION_SCHEMA_VERSION {
        SESSION_V2_FILE
    } else {
        SESSION_FILE
    }
}

struct SessionStore {
    _lifecycle: ProjectMemoryLifecycleLock,
    project_id: String,
    project_dir: Dir,
    sessions_dir: Dir,
}

struct SessionLock {
    file: File,
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        let _ = File::unlock(&self.file);
    }
}

impl SessionStore {
    fn open(
        vault: impl AsRef<Path>,
        project_id: &str,
        create: bool,
    ) -> Result<Option<Self>, LeyCoreError> {
        let vault_path = vault
            .as_ref()
            .canonicalize()
            .map_err(|source| LeyCoreError::Io {
                path: vault.as_ref().to_path_buf(),
                source,
            })?;
        let lifecycle = lock_project_memory_lifecycle(&vault_path, project_id, false, false)?;
        let vault_dir =
            Dir::open_ambient_dir(&vault_path, ambient_authority()).map_err(|source| {
                LeyCoreError::Io {
                    path: vault_path.clone(),
                    source,
                }
            })?;
        let ley_dir = open_existing_dir(&vault_dir, STORE_ROOT)?;
        let memory_dir = open_existing_dir(&ley_dir, AGENT_MEMORY_DIRECTORY)?;
        let projects_dir = open_existing_dir(&memory_dir, PROJECTS_DIRECTORY)?;
        let project_dir = open_existing_dir(&projects_dir, project_id)?;
        if create {
            Self::ensure_lock_file(&project_dir)?;
        }
        let sessions_dir = if create {
            open_or_create_private_dir(&project_dir, SESSIONS_DIRECTORY)?
        } else {
            match project_dir.open_dir_nofollow(SESSIONS_DIRECTORY) {
                Ok(directory) => directory,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(source) => return Err(session_io(SESSIONS_DIRECTORY, source)),
            }
        };
        Ok(Some(Self {
            _lifecycle: lifecycle,
            project_id: project_id.to_owned(),
            project_dir,
            sessions_dir,
        }))
    }

    fn ensure_lock_file(project_dir: &Dir) -> Result<(), LeyCoreError> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create(true)
            .follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = project_dir
            .open_with(SESSION_LOCK_FILE, &options)
            .map_err(|source| session_io(SESSION_LOCK_FILE, source))?;
        ensure_private_file(&lock, SESSION_LOCK_FILE)
    }

    fn lock(&self, shared: bool) -> Result<SessionLock, LeyCoreError> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        if !shared {
            options.write(true).create(true);
            #[cfg(unix)]
            {
                use cap_std::fs::OpenOptionsExt;
                options.mode(0o600);
            }
        }
        let lock = self
            .project_dir
            .open_with(SESSION_LOCK_FILE, &options)
            .map_err(|source| session_io(SESSION_LOCK_FILE, source))?;
        ensure_private_file(&lock, SESSION_LOCK_FILE)?;
        let file = lock.into_std();
        if shared {
            File::lock_shared(&file).map_err(|source| session_io(SESSION_LOCK_FILE, source))?;
        } else {
            file.lock()
                .map_err(|source| session_io(SESSION_LOCK_FILE, source))?;
        }
        Ok(SessionLock { file })
    }

    fn open_or_create_session(&self, session_id: &str) -> Result<Dir, LeyCoreError> {
        validate_session_id(session_id)?;
        open_or_create_private_dir(&self.sessions_dir, session_id)
    }

    fn open_session(&self, session_id: &str) -> Result<Dir, LeyCoreError> {
        validate_session_id(session_id)?;
        self.sessions_dir
            .open_dir_nofollow(session_id)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    LeyCoreError::SessionNotFound(session_id.to_owned())
                } else {
                    session_io(session_id, error)
                }
            })
    }

    fn session_ids(&self) -> Result<Vec<String>, LeyCoreError> {
        let entries = self
            .sessions_dir
            .entries()
            .map_err(|source| session_io(SESSIONS_DIRECTORY, source))?;
        let mut ids = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| session_io(SESSIONS_DIRECTORY, source))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if validate_session_id(name).is_ok()
                && entry
                    .file_type()
                    .map_err(|source| session_io(name, source))?
                    .is_dir()
            {
                let session_dir = self.open_session(name)?;
                let events_dir = match session_dir.open_dir_nofollow(EVENTS_DIRECTORY) {
                    Ok(directory) => directory,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(source) => return Err(session_io(EVENTS_DIRECTORY, source)),
                };
                let mut entries = events_dir
                    .entries()
                    .map_err(|source| session_io(EVENTS_DIRECTORY, source))?;
                if let Some(entry) = entries.next() {
                    entry.map_err(|source| session_io(EVENTS_DIRECTORY, source))?;
                    ids.push(name.to_owned());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    fn rebuild_session(&self, session_id: &str) -> Result<AgentSession, LeyCoreError> {
        let session_dir = self.open_session(session_id)?;
        self.rebuild_session_from_dir(session_id, &session_dir)
    }

    fn rebuild_session_from_dir(
        &self,
        session_id: &str,
        session_dir: &Dir,
    ) -> Result<AgentSession, LeyCoreError> {
        let events = self.read_events(session_id, session_dir)?;
        replay_events(&events, &self.project_id, session_id)
    }

    fn read_events(
        &self,
        session_id: &str,
        session_dir: &Dir,
    ) -> Result<Vec<SessionEvent>, LeyCoreError> {
        read_session_events(session_dir, &self.project_id, session_id)
    }

    fn persist_projection(
        &self,
        session_dir: &Dir,
        session: &AgentSession,
    ) -> Result<(), LeyCoreError> {
        let projection_file = projection_file_name(session);
        let body = json_body(session, SESSION_PROJECTION_LIMIT_BYTES, projection_file)?;
        write_atomic_private(session_dir, projection_file, &body)?;
        let markdown = render_session_markdown(session);
        if markdown.len() as u64 > SESSION_PROJECTION_LIMIT_BYTES {
            return Err(LeyCoreError::MetadataTooLarge {
                path: PathBuf::from(SESSION_MARKDOWN_FILE),
                limit_bytes: SESSION_PROJECTION_LIMIT_BYTES,
            });
        }
        write_atomic_private(session_dir, SESSION_MARKDOWN_FILE, markdown.as_bytes())
    }
}

fn read_session_events(
    session_dir: &Dir,
    project_id: &str,
    session_id: &str,
) -> Result<Vec<SessionEvent>, LeyCoreError> {
    let events_dir = session_dir
        .open_dir_nofollow(EVENTS_DIRECTORY)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                LeyCoreError::SessionNotFound(session_id.to_owned())
            } else {
                session_io(EVENTS_DIRECTORY, error)
            }
        })?;
    let entries = events_dir
        .entries()
        .map_err(|source| session_io(EVENTS_DIRECTORY, source))?;
    let mut events = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| session_io(EVENTS_DIRECTORY, source))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(LeyCoreError::InvalidSessionStore(
                "event filename is not UTF-8".to_owned(),
            ));
        };
        if !name.ends_with(".json")
            || !entry
                .file_type()
                .map_err(|error| session_io(name, error))?
                .is_file()
        {
            return Err(LeyCoreError::InvalidSessionStore(format!(
                "unexpected session event entry: {name}"
            )));
        }
        if events.len() >= SESSION_EVENT_LIMIT {
            return Err(LeyCoreError::InvalidSessionStore(format!(
                "session exceeds {SESSION_EVENT_LIMIT} events"
            )));
        }
        let bytes =
            read_private_file(&events_dir, name, SESSION_EVENT_LIMIT_BYTES)?.ok_or_else(|| {
                LeyCoreError::InvalidSessionStore(format!(
                    "event disappeared while reading: {name}"
                ))
            })?;
        let event: SessionEvent = parse_json(name, &bytes)?;
        validate_event(&event, project_id, session_id)?;
        if name != format!("{}.json", event.event_id) {
            return Err(LeyCoreError::InvalidSessionStore(format!(
                "event filename does not match its ID: {name}"
            )));
        }
        events.push(event);
    }
    events.sort_by_key(|event| event.sequence);
    Ok(events)
}

fn replay_events(
    events: &[SessionEvent],
    project_id: &str,
    session_id: &str,
) -> Result<AgentSession, LeyCoreError> {
    if events.is_empty() {
        return Err(LeyCoreError::SessionNotFound(session_id.to_owned()));
    }
    for (index, event) in events.iter().enumerate() {
        if event.sequence != index as u64 + 1 {
            return Err(LeyCoreError::InvalidSessionStore(
                "session event sequence is not contiguous".to_owned(),
            ));
        }
        if index > 0 && event.recorded_at_unix_ms < events[index - 1].recorded_at_unix_ms {
            return Err(LeyCoreError::InvalidSessionStore(
                "session event timestamps are not monotonic".to_owned(),
            ));
        }
    }
    let first = &events[0];
    let SessionEventPayload::SessionStarted {
        name,
        goal,
        source,
        artifact_snapshot_id,
    } = &first.payload
    else {
        return Err(LeyCoreError::InvalidSessionStore(
            "the first event must start the session".to_owned(),
        ));
    };
    let mut session = AgentSession {
        schema_version: if events
            .iter()
            .any(|event| event.schema_version == SESSION_SCHEMA_VERSION)
        {
            SESSION_SCHEMA_VERSION
        } else {
            SESSION_V1_SCHEMA_VERSION
        },
        project_id: project_id.to_owned(),
        session_id: session_id.to_owned(),
        name: name.clone(),
        original_name: name.clone(),
        goal: goal.clone(),
        status: SessionStatus::Active,
        source: source.clone(),
        artifact_snapshot_id_at_start: artifact_snapshot_id.clone(),
        started_at_unix_ms: first.recorded_at_unix_ms,
        updated_at_unix_ms: first.recorded_at_unix_ms,
        finished_at_unix_ms: None,
        event_count: events.len() as u64,
        checkpoints: Vec::new(),
        prompts: Vec::new(),
        responses: Vec::new(),
        renames: Vec::new(),
        finish: None,
    };
    for event in &events[1..] {
        if session.status != SessionStatus::Active
            && !matches!(&event.payload, SessionEventPayload::SessionRenamed(_))
        {
            return Err(LeyCoreError::InvalidSessionStore(
                "events cannot follow a finished session".to_owned(),
            ));
        }
        match &event.payload {
            SessionEventPayload::SessionStarted { .. } => {
                return Err(LeyCoreError::InvalidSessionStore(
                    "a session can only be started once".to_owned(),
                ))
            }
            SessionEventPayload::CheckpointRecorded(checkpoint) => {
                session.checkpoints.push(checkpoint.as_ref().clone());
            }
            SessionEventPayload::SessionFinished(finish) => {
                session.status = finish.status;
                session.finished_at_unix_ms = Some(finish.recorded_at_unix_ms);
                session.finish = Some(finish.clone());
            }
            SessionEventPayload::SessionRenamed(rename) => {
                session.name = rename.name.clone();
                session.renames.push(rename.clone());
            }
            SessionEventPayload::UserPromptObserved(evidence) => {
                session.prompts.push(evidence.clone());
            }
            SessionEventPayload::AssistantResponseObserved(evidence) => {
                session.responses.push(evidence.clone());
            }
        }
        session.updated_at_unix_ms = event.recorded_at_unix_ms;
    }
    if retained_turn_evidence_bytes(events) > SESSION_TURN_EVIDENCE_LIMIT_BYTES {
        return invalid_session_store("session turn-evidence capacity was exceeded");
    }
    Ok(session)
}

fn validate_event(
    event: &SessionEvent,
    project_id: &str,
    session_id: &str,
) -> Result<(), LeyCoreError> {
    if !matches!(
        event.schema_version,
        SESSION_V1_SCHEMA_VERSION | SESSION_SCHEMA_VERSION
    ) || event.project_id != project_id
        || event.session_id != session_id
        || event.sequence == 0
    {
        return Err(LeyCoreError::InvalidSessionStore(
            "session event identity is invalid".to_owned(),
        ));
    }
    validate_event_id(&event.event_id)?;
    validate_request_id(&event.request_id)?;
    if !is_sha256(&event.request_fingerprint) {
        return Err(LeyCoreError::InvalidSessionStore(
            "session request fingerprint is invalid".to_owned(),
        ));
    }
    let expected = request_fingerprint(project_id, session_id, &event.request_id, &event.payload)?;
    if expected != event.request_fingerprint {
        return Err(LeyCoreError::InvalidSessionStore(
            "session request fingerprint does not match its event".to_owned(),
        ));
    }
    let kind = match event.payload {
        SessionEventPayload::SessionStarted { .. } => "session-started",
        SessionEventPayload::CheckpointRecorded(_) => "checkpoint-recorded",
        SessionEventPayload::SessionFinished(_) => "session-finished",
        SessionEventPayload::SessionRenamed(_) => "session-renamed",
        SessionEventPayload::UserPromptObserved(_) => "user-prompt-observed",
        SessionEventPayload::AssistantResponseObserved(_) => "assistant-response-observed",
    };
    let expected_event = deterministic_id(
        "evt",
        &format!("{session_id}:{}:{kind}", event.request_id),
        64,
    );
    if event.event_id != expected_event {
        return Err(LeyCoreError::InvalidSessionStore(
            "session event ID does not match its request".to_owned(),
        ));
    }
    if event.recorded_at_unix_ms == 0 {
        return Err(LeyCoreError::InvalidSessionStore(
            "session event timestamp is invalid".to_owned(),
        ));
    }
    validate_redactions(&event.redactions)?;
    validate_event_payload(event)?;
    Ok(())
}

fn validate_event_payload(event: &SessionEvent) -> Result<(), LeyCoreError> {
    match &event.payload {
        SessionEventPayload::SessionStarted {
            name,
            goal,
            source,
            artifact_snapshot_id,
        } => {
            validate_stored_text("name", name, 1, 128)?;
            validate_stored_text("goal", goal, 1, 16_000)?;
            if let Some(host) = &source.host {
                validate_stored_text("source.host", host, 1, 128)?;
            }
            if let Some(agent) = &source.agent {
                validate_stored_text("source.agent", agent, 1, 128)?;
            }
            if !valid_prefixed_hex(artifact_snapshot_id, "snp_", 64) {
                return invalid_session_store("session artifact snapshot ID is invalid");
            }
        }
        SessionEventPayload::CheckpointRecorded(checkpoint) => {
            if checkpoint.event_id != event.event_id
                || checkpoint.recorded_at_unix_ms != event.recorded_at_unix_ms
                || checkpoint.id != child_id("ckp", &event.event_id, 0)
            {
                return invalid_session_store("checkpoint identity is invalid");
            }
            validate_stored_text("checkpoint.summary", &checkpoint.summary, 1, 16_000)?;
            validate_checkpoint_records(checkpoint, &event.event_id)?;
        }
        SessionEventPayload::SessionFinished(finish) => {
            if finish.event_id != event.event_id
                || finish.recorded_at_unix_ms != event.recorded_at_unix_ms
                || finish.status == SessionStatus::Active
            {
                return invalid_session_store("session finish identity or status is invalid");
            }
            validate_stored_text("finish.summary", &finish.summary, 1, 16_000)?;
            validate_stored_text("finish.finalResponse", &finish.final_response, 0, 32_000)?;
            validate_stored_text("finish.handoff", &finish.handoff, 0, 16_000)?;
            validate_stored_list("finish.unresolved", &finish.unresolved, 100, 4_000)?;
        }
        SessionEventPayload::SessionRenamed(rename) => {
            if rename.event_id != event.event_id
                || rename.recorded_at_unix_ms != event.recorded_at_unix_ms
            {
                return invalid_session_store("session rename identity is invalid");
            }
            validate_stored_text("rename.name", &rename.name, 1, 128)?;
            validate_stored_text("rename.note", &rename.note, 1, 4_000)?;
        }
        SessionEventPayload::UserPromptObserved(evidence) => {
            validate_turn_evidence(event, evidence, SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS)?;
        }
        SessionEventPayload::AssistantResponseObserved(evidence) => {
            validate_turn_evidence(event, evidence, SESSION_RESPONSE_EVIDENCE_LIMIT_CHARACTERS)?;
        }
    }
    let is_turn_event = matches!(
        event.payload,
        SessionEventPayload::UserPromptObserved(_)
            | SessionEventPayload::AssistantResponseObserved(_)
    );
    if event.schema_version == SESSION_V1_SCHEMA_VERSION && is_turn_event {
        return invalid_session_store(
            "turn evidence events require session event schema version 2",
        );
    }
    if event.schema_version == SESSION_SCHEMA_VERSION && !is_turn_event {
        return invalid_session_store("lifecycle events require session event schema version 1");
    }
    Ok(())
}

fn validate_turn_evidence(
    event: &SessionEvent,
    evidence: &SessionTurnEvidence,
    maximum_characters: usize,
) -> Result<(), LeyCoreError> {
    if evidence.record_id != child_id("tev", &event.event_id, 0)
        || evidence.event_id != event.event_id
        || evidence.sequence != event.sequence
        || evidence.recorded_at_unix_ms != event.recorded_at_unix_ms
        || evidence.recorded_at_unix_ms == 0
    {
        return invalid_session_store("turn evidence identity is invalid");
    }
    if let Some(host) = &evidence.host {
        if !is_valid_turn_host(host) {
            return invalid_session_store("turn evidence host is invalid");
        }
    }
    if let Some(turn_reference) = &evidence.turn_reference {
        if !valid_prefixed_hex(turn_reference, "trn_", 64) {
            return invalid_session_store("turn evidence reference is invalid");
        }
    }
    match evidence.retention {
        TurnEvidenceRetention::Captured => {
            if evidence.capture_mode == crate::CaptureMode::Minimal {
                return invalid_session_store("minimal capture cannot retain turn evidence text");
            }
            let text = evidence.text.as_ref().ok_or_else(|| {
                LeyCoreError::InvalidSessionStore(
                    "captured turn evidence must retain text".to_owned(),
                )
            })?;
            validate_stored_text("turnEvidence.text", text, 1, maximum_characters)?;
        }
        TurnEvidenceRetention::OmittedMinimal => {
            if evidence.capture_mode != crate::CaptureMode::Minimal
                || evidence.text.is_some()
                || evidence.truncated
            {
                return invalid_session_store("minimal turn evidence disclosure is invalid");
            }
        }
        TurnEvidenceRetention::OmittedCapacity => {
            if evidence.capture_mode == crate::CaptureMode::Minimal
                || evidence.text.is_some()
                || evidence.truncated
            {
                return invalid_session_store("capacity turn evidence disclosure is invalid");
            }
        }
    }
    Ok(())
}

fn validate_checkpoint_records(
    checkpoint: &SessionCheckpoint,
    event_id: &str,
) -> Result<(), LeyCoreError> {
    if checkpoint.plan.len() > 100
        || checkpoint.decisions.len() > 100
        || checkpoint.tasks.len() > 100
        || checkpoint.problems.len() > 50
        || checkpoint.touched_artifacts.len() > 200
        || checkpoint.commands.len() > 200
        || checkpoint.verification.len() > 200
    {
        return invalid_session_store("checkpoint collection limits were exceeded");
    }
    if let Some(revision) = &checkpoint.project_revision {
        validate_project_revision(revision)?;
    }
    for (index, item) in checkpoint.plan.iter().enumerate() {
        validate_child_id(&item.id, "pln", event_id, index)?;
        validate_stored_text("plan.text", &item.text, 1, 4_000)?;
    }
    for (index, item) in checkpoint.decisions.iter().enumerate() {
        validate_child_id(&item.id, "dec", event_id, index)?;
        validate_stored_text("decision.title", &item.title, 1, 256)?;
        validate_stored_text("decision.decision", &item.decision, 1, 8_000)?;
        validate_stored_text("decision.rationale", &item.rationale, 0, 8_000)?;
        validate_stored_list("decision.alternatives", &item.alternatives, 20, 2_000)?;
    }
    for (index, item) in checkpoint.tasks.iter().enumerate() {
        validate_child_id(&item.id, "tsk", event_id, index)?;
        validate_stored_text("task.title", &item.title, 1, 256)?;
        validate_stored_text("task.details", &item.details, 0, 4_000)?;
    }
    for (problem_index, item) in checkpoint.problems.iter().enumerate() {
        validate_child_id(&item.id, "prb", event_id, problem_index)?;
        validate_stored_text("problem.title", &item.title, 1, 256)?;
        validate_stored_text("problem.symptom", &item.symptom, 1, 8_000)?;
        validate_stored_text("problem.expected", &item.expected, 0, 8_000)?;
        if item.attempts.len() > 50 {
            return invalid_session_store("a problem contains too many attempts");
        }
        for (attempt_index, attempt) in item.attempts.iter().enumerate() {
            validate_child_id(&attempt.id, "att", &item.id, attempt_index)?;
            validate_stored_text("attempt.action", &attempt.action, 1, 8_000)?;
            validate_stored_text("attempt.evidence", &attempt.evidence, 0, 8_000)?;
        }
        if let Some(resolution) = &item.resolution {
            validate_child_id(&resolution.id, "res", &item.id, 0)?;
            validate_stored_text("resolution.rootCause", &resolution.root_cause, 1, 8_000)?;
            validate_stored_text("resolution.change", &resolution.change, 1, 8_000)?;
            validate_stored_text(
                "resolution.verification",
                &resolution.verification,
                0,
                8_000,
            )?;
        }
    }
    let mut artifact_paths = BTreeSet::new();
    for citation in &checkpoint.touched_artifacts {
        validate_artifact_citation(citation)?;
        if !artifact_paths.insert(citation.artifact_path.as_str()) {
            return invalid_session_store("checkpoint has duplicate artifact citations");
        }
    }
    for (index, item) in checkpoint.commands.iter().enumerate() {
        validate_child_id(&item.id, "cmd", event_id, index)?;
        validate_stored_text("command.command", &item.command, 1, 8_000)?;
        validate_stored_text("command.summary", &item.summary, 0, 4_000)?;
    }
    for (index, item) in checkpoint.verification.iter().enumerate() {
        validate_child_id(&item.id, "ver", event_id, index)?;
        validate_stored_text("verification.kind", &item.kind, 1, 64)?;
        validate_stored_text("verification.summary", &item.summary, 1, 8_000)?;
        if let Some(command) = &item.command {
            validate_stored_text("verification.command", command, 1, 8_000)?;
        }
    }
    validate_stored_list("checkpoint.unresolved", &checkpoint.unresolved, 100, 4_000)
}

fn validate_project_revision(revision: &SessionProjectRevision) -> Result<(), LeyCoreError> {
    let valid_head = revision.head.as_ref().is_none_or(|head| {
        matches!(head.len(), 40 | 64)
            && head
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    });
    let valid_branch = revision
        .branch
        .as_ref()
        .is_none_or(|branch| !branch.is_empty() && branch.chars().count() <= 1024);
    if !valid_prefixed_hex(&revision.graph_snapshot_id, "grf_", 64)
        || !valid_prefixed_hex(&revision.artifact_snapshot_id, "snp_", 64)
        || revision.captured_at_unix_ms == 0
        || !valid_head
        || !valid_branch
        || revision.tracked_changes > 1_000_000
    {
        return invalid_session_store("session project revision is invalid");
    }
    Ok(())
}

fn validate_child_id(
    actual: &str,
    prefix: &str,
    parent: &str,
    index: usize,
) -> Result<(), LeyCoreError> {
    if actual != child_id(prefix, parent, index) {
        return invalid_session_store("session child record ID is invalid");
    }
    Ok(())
}

fn validate_artifact_citation(citation: &SessionArtifactCitation) -> Result<(), LeyCoreError> {
    let path = Path::new(&citation.artifact_path);
    if citation.artifact_path.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || !valid_prefixed_hex(&citation.artifact_snapshot_id, "snp_", 64)
        || !is_sha256(&citation.content_hash)
        || citation.start_line == 0
        || citation.end_line < citation.start_line
    {
        return invalid_session_store("session artifact citation is invalid");
    }
    Ok(())
}

fn validate_redactions(redactions: &[MemoryRedaction]) -> Result<(), LeyCoreError> {
    if redactions.len() > 2_000 {
        return invalid_session_store("session event has too many redaction records");
    }
    for redaction in redactions {
        validate_stored_text("redaction.field", &redaction.field, 1, 256)?;
        validate_stored_text("redaction.kind", &redaction.kind, 1, 128)?;
        if redaction.lines.is_empty()
            || redaction.lines.len() > 10_000
            || redaction.lines.contains(&0)
        {
            return invalid_session_store("session redaction line metadata is invalid");
        }
    }
    Ok(())
}

fn validate_stored_list(
    field: &str,
    values: &[String],
    maximum_items: usize,
    maximum_characters: usize,
) -> Result<(), LeyCoreError> {
    if values.len() > maximum_items {
        return invalid_session_store(&format!("{field} has too many items"));
    }
    for value in values {
        validate_stored_text(field, value, 1, maximum_characters)?;
    }
    Ok(())
}

fn validate_stored_text(
    field: &str,
    value: &str,
    minimum: usize,
    maximum: usize,
) -> Result<(), LeyCoreError> {
    let length = value.chars().count();
    if value != value.trim()
        || length < minimum
        || length > maximum
        || value.chars().any(|character| {
            character == '\0'
                || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        })
    {
        return invalid_session_store(&format!("{field} contains invalid stored text"));
    }
    let (redacted, _) = redact_secrets(value);
    if redacted != value {
        return invalid_session_store(&format!("{field} contains unredacted secret material"));
    }
    Ok(())
}

fn invalid_session_store<T>(message: &str) -> Result<T, LeyCoreError> {
    Err(LeyCoreError::InvalidSessionStore(message.to_owned()))
}

fn sanitize_source(
    source: SessionSource,
    redactions: &mut Vec<MemoryRedaction>,
) -> Result<SessionSource, LeyCoreError> {
    Ok(SessionSource {
        kind: source.kind,
        host: source
            .host
            .map(|value| sanitize_text("source.host", &value, 1, 128, redactions))
            .transpose()?,
        agent: source
            .agent
            .map(|value| sanitize_text("source.agent", &value, 1, 128, redactions))
            .transpose()?,
    })
}

fn sanitize_text(
    field: &str,
    value: &str,
    minimum: usize,
    maximum: usize,
    redactions: &mut Vec<MemoryRedaction>,
) -> Result<String, LeyCoreError> {
    let value = value.trim();
    let length = value.chars().count();
    if length < minimum
        || length > maximum
        || value.chars().any(|character| {
            character == '\0'
                || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        })
    {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "{field} must contain {minimum} to {maximum} safe characters"
        )));
    }
    let (sanitized, findings) = redact_secrets(value);
    redactions.extend(
        findings
            .into_iter()
            .map(|RedactionFinding { kind, lines }| MemoryRedaction {
                field: field.to_owned(),
                kind,
                lines,
            }),
    );
    Ok(sanitized)
}

fn sanitize_list(
    field: &str,
    values: Vec<String>,
    maximum_items: usize,
    maximum_characters: usize,
    redactions: &mut Vec<MemoryRedaction>,
) -> Result<Vec<String>, LeyCoreError> {
    if values.len() > maximum_items {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "{field} cannot contain more than {maximum_items} items"
        )));
    }
    let mut sanitized = Vec::new();
    for (index, value) in values.into_iter().enumerate() {
        sanitized.push(sanitize_text(
            &format!("{field}[{index}]"),
            &value,
            1,
            maximum_characters,
            redactions,
        )?);
    }
    Ok(sanitized)
}

fn validate_request_id(value: &str) -> Result<(), LeyCoreError> {
    if !valid_prefixed_hex(value, "req_", 32) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "requestId must match req_ followed by 32 lowercase hexadecimal characters".to_owned(),
        ));
    }
    Ok(())
}

fn validate_session_id(value: &str) -> Result<(), LeyCoreError> {
    if !valid_prefixed_hex(value, "ses_", 32) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "sessionId must match ses_ followed by 32 lowercase hexadecimal characters".to_owned(),
        ));
    }
    Ok(())
}

fn validate_event_id(value: &str) -> Result<(), LeyCoreError> {
    if !valid_prefixed_hex(value, "evt_", 64) {
        return Err(LeyCoreError::InvalidSessionStore(
            "event ID is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn valid_prefixed_hex(value: &str, prefix: &str, length: usize) -> bool {
    value.strip_prefix(prefix).is_some_and(|hex| {
        hex.len() == length
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn is_sha256(value: &str) -> bool {
    valid_prefixed_hex(value, "sha256:", 64)
}

fn deterministic_id(prefix: &str, value: &str, hex_length: usize) -> String {
    let hash = format!("{:x}", Sha256::digest(value.as_bytes()));
    format!("{prefix}_{}", &hash[..hex_length])
}

fn child_id(prefix: &str, parent: &str, index: usize) -> String {
    deterministic_id(prefix, &format!("{parent}:{index}"), 32)
}

fn request_fingerprint(
    project_id: &str,
    session_id: &str,
    request_id: &str,
    payload: &SessionEventPayload,
) -> Result<String, LeyCoreError> {
    let stable_payload = match payload {
        SessionEventPayload::SessionStarted {
            name, goal, source, ..
        } => SessionEventPayload::SessionStarted {
            name: name.clone(),
            goal: goal.clone(),
            source: source.clone(),
            artifact_snapshot_id: String::new(),
        },
        SessionEventPayload::CheckpointRecorded(checkpoint) => {
            let mut checkpoint = checkpoint.clone();
            checkpoint.recorded_at_unix_ms = 0;
            checkpoint.project_revision = None;
            for citation in &mut checkpoint.touched_artifacts {
                citation.artifact_snapshot_id.clear();
                citation.content_hash.clear();
                citation.start_line = 0;
                citation.end_line = 0;
            }
            SessionEventPayload::CheckpointRecorded(checkpoint)
        }
        SessionEventPayload::SessionFinished(finish) => {
            let mut finish = finish.clone();
            finish.recorded_at_unix_ms = 0;
            SessionEventPayload::SessionFinished(finish)
        }
        SessionEventPayload::SessionRenamed(rename) => {
            let mut rename = rename.clone();
            rename.recorded_at_unix_ms = 0;
            SessionEventPayload::SessionRenamed(rename)
        }
        SessionEventPayload::UserPromptObserved(evidence) => {
            let mut evidence = evidence.clone();
            evidence.recorded_at_unix_ms = 0;
            evidence.sequence = 0;
            // A request fingerprint must never become a persisted raw-body
            // hash. Retained bodies are compared directly only for an exact
            // retry while the process has reconstructed the immutable event.
            evidence.text = None;
            SessionEventPayload::UserPromptObserved(evidence)
        }
        SessionEventPayload::AssistantResponseObserved(evidence) => {
            let mut evidence = evidence.clone();
            evidence.recorded_at_unix_ms = 0;
            evidence.sequence = 0;
            evidence.text = None;
            SessionEventPayload::AssistantResponseObserved(evidence)
        }
    };
    let bytes = serde_json::to_vec(&(project_id, session_id, request_id, stable_payload))
        .map_err(|error| LeyCoreError::InvalidSessionStore(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn json_body<T: Serialize>(value: &T, limit: u64, name: &str) -> Result<Vec<u8>, LeyCoreError> {
    let mut body = serde_json::to_vec_pretty(value)
        .map_err(|error| LeyCoreError::InvalidSessionStore(error.to_string()))?;
    body.push(b'\n');
    if body.len() as u64 > limit {
        return Err(LeyCoreError::MetadataTooLarge {
            path: PathBuf::from(name),
            limit_bytes: limit,
        });
    }
    Ok(body)
}

fn parse_json<T: for<'de> Deserialize<'de>>(name: &str, bytes: &[u8]) -> Result<T, LeyCoreError> {
    serde_json::from_slice(bytes)
        .map_err(|error| LeyCoreError::InvalidSessionStore(format!("{name}: {error}")))
}

fn render_session_markdown(session: &AgentSession) -> String {
    let mut output = String::new();
    output.push_str("---\n");
    output.push_str("leyType: agent-session\n");
    output.push_str(&format!("projectId: {}\n", session.project_id));
    output.push_str(&format!("sessionId: {}\n", session.session_id));
    output.push_str(&format!("status: {}\n", enum_label(session.status)));
    output.push_str(&format!(
        "startedAtUnixMs: {}\nupdatedAtUnixMs: {}\n",
        session.started_at_unix_ms, session.updated_at_unix_ms
    ));
    output.push_str("---\n\n# ");
    output.push_str(&markdown_inline(&session.name));
    output.push_str("\n\n## Goal\n\n");
    push_quote(&mut output, &session.goal);
    if !session.renames.is_empty() {
        output.push_str("\n## Naming history\n\n");
        output.push_str(&format!(
            "- Original: {}\n",
            markdown_inline(&session.original_name)
        ));
        for rename in &session.renames {
            output.push_str(&format!(
                "- {}: {} — {}\n",
                rename.recorded_at_unix_ms,
                markdown_inline(&rename.name),
                markdown_inline(&rename.note)
            ));
        }
    }
    output.push_str("\n## Capture source\n\n");
    output.push_str(&format!("- Kind: `{}`\n", enum_label(session.source.kind)));
    if let Some(host) = &session.source.host {
        output.push_str(&format!("- Host: {}\n", markdown_inline(host)));
    }
    if let Some(agent) = &session.source.agent {
        output.push_str(&format!("- Agent: {}\n", markdown_inline(agent)));
    }
    if !session.prompts.is_empty() || !session.responses.is_empty() {
        output.push_str("\n## Observed turn evidence\n");
        for evidence in &session.prompts {
            render_turn_evidence_markdown(&mut output, "User prompt", evidence);
        }
        for evidence in &session.responses {
            render_turn_evidence_markdown(&mut output, "Assistant response", evidence);
        }
    }
    for (index, checkpoint) in session.checkpoints.iter().enumerate() {
        output.push_str(&format!(
            "\n## Checkpoint {} · {}\n\n",
            index + 1,
            checkpoint.recorded_at_unix_ms
        ));
        push_quote(&mut output, &checkpoint.summary);
        if let Some(revision) = &checkpoint.project_revision {
            output.push_str("\n### Captured project revision\n\n");
            if let Some(head) = &revision.head {
                output.push_str(&format!("- Git HEAD: `{head}`\n"));
            } else {
                output.push_str("- Git HEAD: not present in this capture\n");
            }
            if let Some(branch) = &revision.branch {
                output.push_str(&format!("- Branch: `{}`\n", markdown_inline(branch)));
            }
            output.push_str(&format!(
                "- Graph snapshot: `{}`\n- Artifact snapshot: `{}`\n- Captured at: `{}`\n- Tracked changes: `{}`\n",
                revision.graph_snapshot_id,
                revision.artifact_snapshot_id,
                revision.captured_at_unix_ms,
                revision.tracked_changes
            ));
        }
        if !checkpoint.plan.is_empty() {
            output.push_str("\n### Plan\n\n");
            for item in &checkpoint.plan {
                output.push_str(&format!(
                    "- [{}] {} `{}`\n",
                    if item.status == PlanStatus::Completed {
                        "x"
                    } else {
                        " "
                    },
                    markdown_inline(&item.text),
                    enum_label(item.status)
                ));
            }
        }
        if !checkpoint.decisions.is_empty() {
            output.push_str("\n### Decisions\n\n");
            for decision in &checkpoint.decisions {
                output.push_str(&format!(
                    "- **{}**: {}\n",
                    markdown_inline(&decision.title),
                    markdown_inline(&decision.decision)
                ));
            }
        }
        if !checkpoint.tasks.is_empty() {
            output.push_str("\n### Tasks\n\n");
            for task in &checkpoint.tasks {
                output.push_str(&format!(
                    "- [{}] {} `{}`\n",
                    if task.status == TaskStatus::Completed {
                        "x"
                    } else {
                        " "
                    },
                    markdown_inline(&task.title),
                    enum_label(task.status)
                ));
            }
        }
        if !checkpoint.problems.is_empty() {
            output.push_str("\n### Problems and outcomes\n\n");
            for problem in &checkpoint.problems {
                output.push_str(&format!(
                    "#### {}\n\n**Symptom**\n\n",
                    markdown_inline(&problem.title)
                ));
                push_quote(&mut output, &problem.symptom);
                for attempt in &problem.attempts {
                    output.push_str(&format!(
                        "\n- **Attempt · `{}`**: {}\n",
                        enum_label(attempt.outcome),
                        markdown_inline(&attempt.action)
                    ));
                }
                if let Some(resolution) = &problem.resolution {
                    output.push_str("\n**Resolution**\n\n");
                    push_quote(&mut output, &resolution.change);
                }
            }
        }
        if !checkpoint.touched_artifacts.is_empty() {
            output.push_str("\n### Touched artifacts\n\n");
            for artifact in &checkpoint.touched_artifacts {
                output.push_str(&format!(
                    "- `{}` · `{}`\n",
                    artifact.artifact_path, artifact.content_hash
                ));
            }
        }
        if !checkpoint.commands.is_empty() {
            output.push_str("\n### Commands\n\n");
            for command in &checkpoint.commands {
                output.push_str(&format!(
                    "- `{}`{}{}\n",
                    markdown_inline(&command.command),
                    command
                        .exit_code
                        .map(|code| format!(" · exit `{code}`"))
                        .unwrap_or_default(),
                    if command.summary.is_empty() {
                        String::new()
                    } else {
                        format!(" · {}", markdown_inline(&command.summary))
                    }
                ));
            }
        }
        if !checkpoint.verification.is_empty() {
            output.push_str("\n### Verification\n\n");
            for verification in &checkpoint.verification {
                output.push_str(&format!(
                    "- **{} · `{}`**: {}\n",
                    markdown_inline(&verification.kind),
                    enum_label(verification.status),
                    markdown_inline(&verification.summary)
                ));
            }
        }
        if !checkpoint.unresolved.is_empty() {
            output.push_str("\n### Unresolved\n\n");
            for item in &checkpoint.unresolved {
                output.push_str(&format!("- {}\n", markdown_inline(item)));
            }
        }
    }
    if let Some(finish) = &session.finish {
        output.push_str("\n## Session result\n\n");
        push_quote(&mut output, &finish.summary);
        if !finish.final_response.is_empty() {
            output.push_str("\n### Final response\n\n");
            push_quote(&mut output, &finish.final_response);
        }
        if !finish.handoff.is_empty() {
            output.push_str("\n### Handoff\n\n");
            push_quote(&mut output, &finish.handoff);
        }
        if !finish.unresolved.is_empty() {
            output.push_str("\n### Remaining work\n\n");
            for item in &finish.unresolved {
                output.push_str(&format!("- {}\n", markdown_inline(item)));
            }
        }
    }
    output
}

fn render_turn_evidence_markdown(output: &mut String, label: &str, evidence: &SessionTurnEvidence) {
    output.push_str(&format!(
        "\n### {label} · {}\n\n",
        evidence.recorded_at_unix_ms
    ));
    output.push_str(&format!(
        "- Origin: `{}`\n- Capture mode: `{}`\n- Retention: `{}`\n",
        enum_label(evidence.origin),
        evidence.capture_mode,
        enum_label(evidence.retention),
    ));
    if let Some(host) = &evidence.host {
        output.push_str(&format!("- Host: `{}`\n", markdown_inline(host)));
    }
    if let Some(turn_reference) = &evidence.turn_reference {
        output.push_str(&format!("- Turn reference: `{turn_reference}`\n"));
    }
    match evidence.retention {
        TurnEvidenceRetention::OmittedMinimal => {
            output.push_str("- Body: omitted by Minimal capture mode\n");
        }
        TurnEvidenceRetention::OmittedCapacity => {
            output.push_str(
                "- Body: omitted because the session turn-evidence capacity was reached\n",
            );
        }
        TurnEvidenceRetention::Captured => {
            if let Some(text) = &evidence.text {
                if text.contains("[REDACTED:") {
                    output.push_str("- Body: captured with redactions\n");
                } else {
                    output.push_str("- Body: captured\n");
                }
                if evidence.truncated {
                    output.push_str("- Body: truncated to the capture limit\n");
                }
                output.push('\n');
                push_quote(output, text);
            } else {
                output.push_str("- Body: no visible text was supplied\n");
            }
        }
    }
}

fn push_quote(output: &mut String, value: &str) {
    for line in value.lines() {
        output.push_str("> ");
        output.push_str(line);
        output.push('\n');
    }
}

fn markdown_inline(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', " ")
}

trait EnumLabel {
    fn label(self) -> &'static str;
}

fn enum_label<T: EnumLabel + Copy>(value: T) -> &'static str {
    value.label()
}

macro_rules! enum_labels {
    ($type:ty, {$($variant:ident => $label:literal),+ $(,)?}) => {
        impl EnumLabel for $type {
            fn label(self) -> &'static str {
                match self {
                    $(Self::$variant => $label),+
                }
            }
        }
    };
}

enum_labels!(SessionStatus, {
    Active => "active",
    Completed => "completed",
    Paused => "paused",
    Abandoned => "abandoned",
});
enum_labels!(SessionSourceKind, {
    ManualCli => "manual-cli",
    HostHook => "host-hook",
    Mcp => "mcp",
    Import => "import",
});
enum_labels!(TurnEvidenceOrigin, {
    HostHook => "host-hook",
    ManualCli => "manual-cli",
});
enum_labels!(TurnEvidenceRetention, {
    Captured => "captured",
    OmittedMinimal => "omitted-minimal",
    OmittedCapacity => "omitted-capacity",
});
enum_labels!(PlanStatus, {
    Pending => "pending",
    InProgress => "in-progress",
    Completed => "completed",
    Blocked => "blocked",
});
enum_labels!(TaskStatus, {
    Pending => "pending",
    InProgress => "in-progress",
    Completed => "completed",
    Blocked => "blocked",
    Cancelled => "cancelled",
});
enum_labels!(AttemptOutcome, {
    Helped => "helped",
    NoEffect => "no-effect",
    Worsened => "worsened",
    Unknown => "unknown",
});
enum_labels!(VerificationStatus, {
    Passed => "passed",
    Failed => "failed",
    Skipped => "skipped",
    Unknown => "unknown",
});

fn open_existing_dir(parent: &Dir, name: &str) -> Result<Dir, LeyCoreError> {
    parent
        .open_dir_nofollow(name)
        .map_err(|source| session_io(name, source))
}

fn open_or_create_private_dir(parent: &Dir, name: &str) -> Result<Dir, LeyCoreError> {
    match parent.open_dir_nofollow(name) {
        Ok(directory) => return Ok(directory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(session_io(name, source)),
    }
    let mut builder = cap_std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use cap_std::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match parent.create_dir_with(name, &builder) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(source) => return Err(session_io(name, source)),
    }
    parent
        .open_dir_nofollow(name)
        .map_err(|source| session_io(name, source))
}

fn read_private_file(
    directory: &Dir,
    name: &str,
    limit: u64,
) -> Result<Option<Vec<u8>>, LeyCoreError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = match directory.open_with(name, &options) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(session_io(name, source)),
    };
    ensure_private_file(&file, name)?;
    let metadata = file.metadata().map_err(|source| session_io(name, source))?;
    if metadata.len() > limit {
        return Err(LeyCoreError::MetadataTooLarge {
            path: PathBuf::from(name),
            limit_bytes: limit,
        });
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| session_io(name, source))?;
    if bytes.len() as u64 > limit {
        return Err(LeyCoreError::MetadataTooLarge {
            path: PathBuf::from(name),
            limit_bytes: limit,
        });
    }
    Ok(Some(bytes))
}

fn ensure_private_file(file: &cap_std::fs::File, name: &str) -> Result<(), LeyCoreError> {
    let metadata = file.metadata().map_err(|source| session_io(name, source))?;
    if !metadata.is_file() {
        return Err(LeyCoreError::InvalidSessionStore(format!(
            "{name} is not a regular file"
        )));
    }
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LeyCoreError::InvalidSessionStore(format!(
                "{name} must use private mode 600"
            )));
        }
    }
    Ok(())
}

fn write_immutable_private(directory: &Dir, name: &str, body: &[u8]) -> Result<(), LeyCoreError> {
    if let Some(existing) = read_private_file(directory, name, body.len() as u64)? {
        if existing == body {
            return Ok(());
        }
        return Err(LeyCoreError::InvalidSessionStore(format!(
            "immutable session event collision at {name}"
        )));
    }
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = directory
        .open_with(name, &options)
        .map_err(|source| session_io(name, source))?;
    file.write_all(body)
        .map_err(|source| session_io(name, source))?;
    file.sync_all().map_err(|source| session_io(name, source))
}

fn write_atomic_private(directory: &Dir, name: &str, body: &[u8]) -> Result<(), LeyCoreError> {
    let mut temporary =
        cap_tempfile::TempFile::new(directory).map_err(|source| session_io(name, source))?;
    let mut permissions = temporary
        .as_file()
        .metadata()
        .map_err(|source| session_io(name, source))?
        .permissions();
    permissions.set_readonly(false);
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        permissions.set_mode(0o600);
    }
    temporary
        .as_file()
        .set_permissions(permissions)
        .map_err(|source| session_io(name, source))?;
    temporary
        .write_all(body)
        .map_err(|source| session_io(name, source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| session_io(name, source))?;
    temporary
        .replace(name)
        .map_err(|source| session_io(name, source))
}

fn session_io(name: &str, source: std::io::Error) -> LeyCoreError {
    LeyCoreError::Io {
        path: PathBuf::from(name),
        source,
    }
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
    use crate::{ingest_project, initialize_project, CaptureMode};
    use std::process::Command;
    use std::sync::{Arc, Barrier};
    use tempfile::tempdir;

    fn setup_memory() -> (tempfile::TempDir, PathBuf, PathBuf) {
        setup_memory_with_mode(CaptureMode::Structured)
    }

    fn setup_memory_with_mode(mode: CaptureMode) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Session test"), mode).unwrap();
        std::fs::write(
            project.join("README.md"),
            "# Session memory\n\nA durable checkpoint target.\n",
        )
        .unwrap();
        std::fs::write(project.join("src.rs"), "pub fn remember() {}\n").unwrap();
        ingest_project(&project, &vault).unwrap();
        (base, project, vault)
    }

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
    }

    fn numbered_request_id(number: usize) -> String {
        format!("req_{number:032x}")
    }

    fn turn_input(request_id: String, text: impl Into<String>) -> TurnEvidenceInput {
        TurnEvidenceInput {
            request_id,
            origin: TurnEvidenceOrigin::HostHook,
            host: Some("codex".to_owned()),
            correlation_material: Some("trusted-adapter-correlation".to_owned()),
            text: text.into(),
        }
    }

    fn git(project: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    fn start_input(request_id: String) -> StartSessionInput {
        StartSessionInput {
            request_id,
            name: "Implementation session".to_owned(),
            goal: "Ship durable structured memory".to_owned(),
            source: SessionSource {
                kind: SessionSourceKind::HostHook,
                host: Some("codex".to_owned()),
                agent: Some("gpt-5".to_owned()),
            },
        }
    }

    fn checkpoint_input(request_id: String, summary: &str) -> CheckpointInput {
        CheckpointInput {
            request_id,
            summary: summary.to_owned(),
            plan: vec![PlanItemInput {
                text: "Persist an immutable event".to_owned(),
                status: PlanStatus::Completed,
            }],
            decisions: vec![DecisionInput {
                title: "Event storage".to_owned(),
                decision: "Use one JSON file per immutable event".to_owned(),
                rationale: "Atomic creation and recovery are straightforward".to_owned(),
                alternatives: vec!["Append-only JSONL".to_owned()],
            }],
            tasks: vec![TaskInput {
                title: "Verify replay".to_owned(),
                status: TaskStatus::Completed,
                details: "Rebuilt the session from source events".to_owned(),
            }],
            problems: vec![ProblemInput {
                title: "Interrupted projection".to_owned(),
                symptom: "The derived Markdown file is missing".to_owned(),
                expected: "Source events remain readable".to_owned(),
                attempts: vec![AttemptInput {
                    action: "Replay immutable events".to_owned(),
                    outcome: AttemptOutcome::Helped,
                    evidence: "Session state was reconstructed".to_owned(),
                }],
                resolution: Some(ResolutionInput {
                    root_cause: "Projection write was interrupted".to_owned(),
                    change: "Treat events as authoritative".to_owned(),
                    verification: "Deleted the projection and replayed successfully".to_owned(),
                }),
            }],
            touched_artifacts: vec!["README.md".to_owned()],
            commands: vec![CommandInput {
                command: "cargo test -p ley-core".to_owned(),
                exit_code: Some(0),
                summary: "Core tests passed".to_owned(),
            }],
            verification: vec![VerificationInput {
                kind: "test".to_owned(),
                status: VerificationStatus::Passed,
                summary: "Session lifecycle passed".to_owned(),
                command: Some("cargo test -p ley-core".to_owned()),
            }],
            unresolved: vec!["Expose lifecycle tools through MCP".to_owned()],
        }
    }

    fn finish_input(request_id: String) -> FinishSessionInput {
        FinishSessionInput {
            request_id,
            status: SessionStatus::Completed,
            summary: "Durable session capture is working".to_owned(),
            final_response: "Implemented and verified the lifecycle".to_owned(),
            handoff: "Add reviewed learnings next".to_owned(),
            unresolved: vec!["Learning promotion remains".to_owned()],
        }
    }

    fn session_directory(project: &Path, vault: &Path, session_id: &str) -> PathBuf {
        let project_id = diagnose_project(project).unwrap().identity.project_id;
        vault
            .join(STORE_ROOT)
            .join(AGENT_MEMORY_DIRECTORY)
            .join(PROJECTS_DIRECTORY)
            .join(project_id)
            .join(SESSIONS_DIRECTORY)
            .join(session_id)
    }

    #[test]
    fn v1_ledger_remains_readable_without_a_read_rewrite() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('0'))).unwrap();
        let directory = session_directory(&project, &vault, &started.session.session_id);
        let v1 = directory.join(SESSION_FILE);
        let before = std::fs::read(&v1).unwrap();

        let replayed = read_session(&project, &vault, &started.session.session_id).unwrap();

        assert_eq!(replayed.schema_version, SESSION_V1_SCHEMA_VERSION);
        assert_eq!(std::fs::read(v1).unwrap(), before);
        assert!(!directory.join(SESSION_V2_FILE).exists());
    }

    #[test]
    fn structured_turn_evidence_is_redacted_truncated_and_upgrades_projection() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('1'))).unwrap();
        let directory = session_directory(&project, &vault, &started.session.session_id);
        let v1_before = std::fs::read(directory.join(SESSION_FILE)).unwrap();
        let secret = "sk-abcdefghijklmnopqrstuvwxyz123456";
        let prompt = format!("Use {secret} {}", "x".repeat(4_100));

        let mutation = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('2'), prompt.clone()),
        )
        .unwrap();
        let evidence = &mutation.session.prompts[0];
        let stored_text = evidence.text.as_deref().unwrap();
        assert_eq!(mutation.session.schema_version, SESSION_SCHEMA_VERSION);
        assert_eq!(evidence.retention, TurnEvidenceRetention::Captured);
        assert!(evidence.truncated);
        assert!(stored_text.chars().count() <= SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS);
        assert!(stored_text.contains("[REDACTED:provider-token]"));
        assert!(!stored_text.contains(secret));
        assert!(evidence
            .turn_reference
            .as_deref()
            .unwrap()
            .starts_with("trn_"));
        assert_eq!(
            evidence.turn_reference,
            Some(derive_turn_reference("trusted-adapter-correlation"))
        );
        assert!(directory.join(SESSION_V2_FILE).exists());
        assert_eq!(
            std::fs::read(directory.join(SESSION_FILE)).unwrap(),
            v1_before
        );
        assert!(mutation.session_path.ends_with(SESSION_V2_FILE));
        let markdown = std::fs::read_to_string(directory.join(SESSION_MARKDOWN_FILE)).unwrap();
        assert!(markdown.contains("captured with redactions"));
        assert!(markdown.contains("truncated to the capture limit"));
    }

    #[test]
    fn minimal_turn_evidence_appends_a_body_free_disclosure() {
        let (_base, project, vault) = setup_memory_with_mode(CaptureMode::Minimal);
        let started = start_session(&project, &vault, start_input(request_id('3'))).unwrap();
        let text = "This must never appear in Minimal evidence.";
        let mutation = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('4'), text),
        )
        .unwrap();
        let evidence = &mutation.session.responses[0];
        assert_eq!(evidence.retention, TurnEvidenceRetention::OmittedMinimal);
        assert!(evidence.text.is_none());
        assert!(!evidence.truncated);
        let mut stored = String::new();
        collect_file_text(
            &session_directory(&project, &vault, &started.session.session_id),
            &mut stored,
        );
        assert!(!stored.contains(text));
    }

    #[test]
    fn turn_evidence_capacity_omission_is_idempotent_without_body_metadata() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(numbered_request_id(1))).unwrap();
        let response = "r".repeat(SESSION_RESPONSE_EVIDENCE_LIMIT_CHARACTERS);
        for index in 2..=132 {
            let mutation = record_session_response(
                &project,
                &vault,
                &started.session.session_id,
                turn_input(numbered_request_id(index), response.clone()),
            )
            .unwrap();
            assert_eq!(mutation.session.responses.len(), index - 1);
        }
        let capacity_request = numbered_request_id(133);
        let omitted = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(capacity_request.clone(), response.clone()),
        )
        .unwrap();
        let evidence = omitted.session.responses.last().unwrap();
        assert_eq!(evidence.retention, TurnEvidenceRetention::OmittedCapacity);
        assert!(evidence.text.is_none());
        assert!(!evidence.truncated);
        let replayed = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(capacity_request, response),
        )
        .unwrap();
        assert!(replayed.replayed);
        assert_eq!(replayed.session.event_count, omitted.session.event_count);
        let replayed_early = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(
                numbered_request_id(2),
                "r".repeat(SESSION_RESPONSE_EVIDENCE_LIMIT_CHARACTERS),
            ),
        )
        .unwrap();
        assert!(replayed_early.replayed);
        assert_eq!(
            replayed_early.session.event_count,
            omitted.session.event_count
        );
        let event_path = session_directory(&project, &vault, &started.session.session_id)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", omitted.event_id));
        let event: serde_json::Value =
            serde_json::from_slice(&std::fs::read(event_path).unwrap()).unwrap();
        assert!(event["data"].get("text").is_none());
        assert!(event["data"].get("originalLength").is_none());
        assert!(event["data"].get("bodyHash").is_none());
    }

    #[test]
    fn turn_evidence_request_conflicts_and_terminal_sessions_reject_appends() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('5'))).unwrap();
        let request = request_id('6');
        let captured = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request.clone(), "original prompt"),
        )
        .unwrap();
        assert!(
            record_session_prompt(
                &project,
                &vault,
                &started.session.session_id,
                turn_input(request.clone(), "original prompt"),
            )
            .unwrap()
            .replayed
        );
        assert!(matches!(
            record_session_prompt(
                &project,
                &vault,
                &started.session.session_id,
                turn_input(request, "changed prompt"),
            ),
            Err(LeyCoreError::SessionIdempotencyConflict(_))
        ));
        finish_session(
            &project,
            &vault,
            &started.session.session_id,
            finish_input(request_id('7')),
        )
        .unwrap();
        assert!(matches!(
            record_session_response(
                &project,
                &vault,
                &started.session.session_id,
                turn_input(request_id('8'), "too late"),
            ),
            Err(LeyCoreError::InvalidSessionRequest(_))
        ));
        assert_eq!(
            read_session(&project, &vault, &started.session.session_id)
                .unwrap()
                .event_count,
            captured.session.event_count + 1
        );
    }

    #[test]
    fn lifecycle_is_cited_replayable_and_idempotent() {
        let (_base, project, vault) = setup_memory();
        let start_request = request_id('1');
        let started = start_session(&project, &vault, start_input(start_request.clone())).unwrap();
        let start_snapshot = started.session.artifact_snapshot_id_at_start.clone();
        std::fs::write(
            project.join("README.md"),
            "# Session memory\n\nThe project changed after session start.\n",
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let replayed = start_session(&project, &vault, start_input(start_request.clone())).unwrap();
        assert!(replayed.replayed);
        assert_eq!(replayed.event_id, started.event_id);
        assert_eq!(replayed.session.event_count, 1);
        assert_eq!(
            replayed.session.artifact_snapshot_id_at_start,
            start_snapshot
        );
        let start_event = session_directory(&project, &vault, &started.session.session_id)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", started.event_id));
        let start_json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(start_event).unwrap()).unwrap();
        assert!(start_json["data"]["artifactSnapshotId"].is_string());
        assert!(start_json["data"].get("artifact_snapshot_id").is_none());

        let checkpoint_request = request_id('2');
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            checkpoint_input(
                checkpoint_request.clone(),
                "Captured the implementation result",
            ),
        )
        .unwrap();
        assert_eq!(checkpoint.session.event_count, 2);
        let revision = checkpoint.session.checkpoints[0]
            .project_revision
            .as_ref()
            .unwrap();
        assert_eq!(
            revision.artifact_snapshot_id,
            checkpoint.session.checkpoints[0].touched_artifacts[0].artifact_snapshot_id
        );
        assert!(valid_prefixed_hex(&revision.graph_snapshot_id, "grf_", 64));
        assert!(revision.head.is_none());
        assert!(revision.branch.is_none());
        let cited_graph_snapshot = revision.graph_snapshot_id.clone();
        let citation = &checkpoint.session.checkpoints[0].touched_artifacts[0];
        assert_eq!(citation.artifact_path, "README.md");
        assert!(valid_prefixed_hex(
            &citation.artifact_snapshot_id,
            "snp_",
            64
        ));
        assert!(is_sha256(&citation.content_hash));
        assert_eq!(citation.start_line, 1);
        assert!(citation.end_line >= citation.start_line);
        let cited_snapshot = citation.artifact_snapshot_id.clone();
        let cited_hash = citation.content_hash.clone();
        std::fs::write(
            project.join("README.md"),
            "# Session memory\n\nThe project changed after checkpoint delivery.\n",
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let replayed_checkpoint = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            checkpoint_input(checkpoint_request, "Captured the implementation result"),
        )
        .unwrap();
        assert!(replayed_checkpoint.replayed);
        assert_eq!(
            replayed_checkpoint.session.checkpoints[0].touched_artifacts[0].artifact_snapshot_id,
            cited_snapshot
        );
        assert_eq!(
            replayed_checkpoint.session.checkpoints[0].touched_artifacts[0].content_hash,
            cited_hash
        );
        assert_eq!(
            replayed_checkpoint.session.checkpoints[0]
                .project_revision
                .as_ref()
                .unwrap()
                .graph_snapshot_id,
            cited_graph_snapshot
        );

        let finish_request = request_id('3');
        let finished = finish_session(
            &project,
            &vault,
            &started.session.session_id,
            finish_input(finish_request.clone()),
        )
        .unwrap();
        assert_eq!(finished.session.status, SessionStatus::Completed);
        assert_eq!(finished.session.event_count, 3);
        assert!(
            finish_session(
                &project,
                &vault,
                &started.session.session_id,
                finish_input(finish_request),
            )
            .unwrap()
            .replayed
        );
        let rename_request = request_id('4');
        let renamed = rename_session(
            &project,
            &vault,
            &started.session.session_id,
            RenameSessionInput {
                request_id: rename_request.clone(),
                expected_event_count: Some(finished.session.event_count),
                name: "Ship durable session memory".to_owned(),
                note: "The original agent suggestion was too generic.".to_owned(),
            },
        )
        .unwrap();
        assert_eq!(renamed.session.original_name, "Implementation session");
        assert_eq!(renamed.session.name, "Ship durable session memory");
        assert_eq!(renamed.session.renames.len(), 1);
        assert_eq!(renamed.session.event_count, 4);
        assert!(
            rename_session(
                &project,
                &vault,
                &started.session.session_id,
                RenameSessionInput {
                    request_id: rename_request,
                    expected_event_count: Some(finished.session.event_count),
                    name: "Ship durable session memory".to_owned(),
                    note: "The original agent suggestion was too generic.".to_owned(),
                },
            )
            .unwrap()
            .replayed
        );
        assert!(matches!(
            rename_session(
                &project,
                &vault,
                &started.session.session_id,
                RenameSessionInput {
                    request_id: request_id('5'),
                    expected_event_count: Some(finished.session.event_count),
                    name: "Stale rename".to_owned(),
                    note: "This must not overwrite the inspected version.".to_owned(),
                },
            ),
            Err(LeyCoreError::InvalidSessionRequest(message))
                if message.contains("reload before saving")
        ));
        assert!(matches!(
            rename_session(
                &project,
                &vault,
                &started.session.session_id,
                RenameSessionInput {
                    request_id: request_id('6'),
                    expected_event_count: Some(renamed.session.event_count),
                    name: "Ship durable session memory".to_owned(),
                    note: "This would not change the current name.".to_owned(),
                },
            ),
            Err(LeyCoreError::InvalidSessionRequest(message))
                if message.contains("already named")
        ));
        assert_eq!(
            read_session(&project, &vault, &started.session.session_id)
                .unwrap()
                .event_count,
            4
        );

        let directory = session_directory(&project, &vault, &started.session.session_id);
        let markdown = std::fs::read_to_string(directory.join(SESSION_MARKDOWN_FILE)).unwrap();
        assert!(markdown.contains("# Ship durable session memory"));
        assert!(markdown.contains("## Problems and outcomes"));
        assert!(markdown.contains("### Captured project revision"));
        assert!(markdown.contains("Git HEAD: not present in this capture"));
        assert!(markdown.contains("README.md"));
        assert!(markdown.contains("### Final response"));
        assert!(markdown.contains("Implemented and verified the lifecycle"));
        std::fs::remove_file(directory.join(SESSION_FILE)).unwrap();
        std::fs::remove_file(directory.join(SESSION_MARKDOWN_FILE)).unwrap();
        assert_eq!(
            read_session(&project, &vault, &started.session.session_id)
                .unwrap()
                .event_count,
            4
        );
        let listed = list_sessions(&project, &vault).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, SessionStatus::Completed);
        assert_eq!(
            project_session_stats(&project, &vault).unwrap(),
            ProjectSessionStats {
                total_sessions: 1,
                completed_sessions: 1,
                ..ProjectSessionStats::default()
            }
        );
    }

    #[test]
    fn checkpoints_pin_ingested_git_revisions_without_reading_live_head() {
        let (_base, project, vault) = setup_memory();
        git(&project, &["init", "-b", "main"]);
        git(&project, &["config", "user.name", "Ley Test"]);
        git(
            &project,
            &["config", "user.email", "ley-test@example.invalid"],
        );
        git(&project, &["add", "."]);
        git(&project, &["commit", "-m", "first capture"]);
        let first_head = git(&project, &["rev-parse", "HEAD"]);
        ingest_project(&project, &vault).unwrap();

        let started = start_session(&project, &vault, start_input(request_id('b'))).unwrap();
        let first = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            checkpoint_input(request_id('c'), "Pinned first revision"),
        )
        .unwrap();
        let first_revision = first.session.checkpoints[0]
            .project_revision
            .as_ref()
            .unwrap();
        assert_eq!(first_revision.head.as_deref(), Some(first_head.as_str()));
        assert_eq!(first_revision.branch.as_deref(), Some("main"));
        assert_eq!(first_revision.tracked_changes, 0);

        std::fs::write(
            project.join("README.md"),
            "# Session memory\n\nCommitted after Ley's capture.\n",
        )
        .unwrap();
        git(&project, &["add", "README.md"]);
        git(&project, &["commit", "-m", "second capture"]);
        let second_head = git(&project, &["rev-parse", "HEAD"]);
        let before_refresh = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            checkpoint_input(request_id('d'), "Still pinned to approved memory"),
        )
        .unwrap();
        assert_eq!(
            before_refresh.session.checkpoints[1]
                .project_revision
                .as_ref()
                .unwrap()
                .head
                .as_deref(),
            Some(first_head.as_str())
        );

        ingest_project(&project, &vault).unwrap();
        let after_refresh = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            checkpoint_input(request_id('e'), "Pinned refreshed memory"),
        )
        .unwrap();
        assert_eq!(
            after_refresh.session.checkpoints[2]
                .project_revision
                .as_ref()
                .unwrap()
                .head
                .as_deref(),
            Some(second_head.as_str())
        );
    }

    #[test]
    fn request_conflicts_and_post_finish_appends_do_not_mutate_history() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('4'))).unwrap();
        let session_id = &started.session.session_id;
        let checkpoint_request = request_id('5');
        checkpoint_session(
            &project,
            &vault,
            session_id,
            checkpoint_input(checkpoint_request.clone(), "Original checkpoint"),
        )
        .unwrap();
        assert!(matches!(
            checkpoint_session(
                &project,
                &vault,
                session_id,
                checkpoint_input(checkpoint_request, "Different checkpoint"),
            ),
            Err(LeyCoreError::SessionIdempotencyConflict(_))
        ));
        assert!(matches!(
            finish_session(&project, &vault, session_id, finish_input(request_id('5')),),
            Err(LeyCoreError::SessionIdempotencyConflict(_))
        ));
        finish_session(&project, &vault, session_id, finish_input(request_id('6'))).unwrap();
        assert!(matches!(
            checkpoint_session(
                &project,
                &vault,
                session_id,
                checkpoint_input(request_id('7'), "Too late"),
            ),
            Err(LeyCoreError::InvalidSessionRequest(_))
        ));
        assert_eq!(
            read_session(&project, &vault, session_id)
                .unwrap()
                .event_count,
            3
        );
    }

    #[test]
    fn secrets_are_redacted_from_events_and_projections() {
        let (_base, project, vault) = setup_memory();
        let secret = "sk-abcdefghijklmnopqrstuvwxyz123456";
        let mut input = start_input(request_id('8'));
        input.goal = format!("Use token {secret} without persisting it");
        let started = start_session(&project, &vault, input).unwrap();
        let mut checkpoint = checkpoint_input(request_id('9'), "Checked credential handling");
        checkpoint.commands[0].command = format!("tool --token {secret}");
        checkpoint_session(&project, &vault, &started.session.session_id, checkpoint).unwrap();

        let directory = session_directory(&project, &vault, &started.session.session_id);
        let mut stored = String::new();
        collect_file_text(&directory, &mut stored);
        assert!(!stored.contains(secret));
        assert!(stored.contains("[REDACTED:provider-token]"));
        assert!(stored.contains("\"redactions\""));
    }

    #[test]
    fn invalid_artifacts_and_interrupted_empty_sessions_are_handled_safely() {
        let (_base, project, vault) = setup_memory();
        assert!(list_sessions(&project, &vault).unwrap().is_empty());
        let project_id = diagnose_project(&project).unwrap().identity.project_id;
        let project_store = vault
            .join(STORE_ROOT)
            .join(AGENT_MEMORY_DIRECTORY)
            .join(PROJECTS_DIRECTORY)
            .join(&project_id);
        assert!(!project_store.join(SESSIONS_DIRECTORY).exists());

        let started = start_session(&project, &vault, start_input(request_id('a'))).unwrap();
        let orphan = project_store
            .join(SESSIONS_DIRECTORY)
            .join(format!("ses_{}", "f".repeat(32)));
        std::fs::create_dir_all(orphan.join(EVENTS_DIRECTORY)).unwrap();
        assert_eq!(list_sessions(&project, &vault).unwrap().len(), 1);
        assert!(matches!(
            checkpoint_session(
                &project,
                &vault,
                &started.session.session_id,
                CheckpointInput {
                    touched_artifacts: vec!["../outside.txt".to_owned()],
                    ..checkpoint_input(request_id('b'), "Invalid citation")
                },
            ),
            Err(LeyCoreError::InvalidSessionRequest(_))
        ));
        assert_eq!(
            read_session(&project, &vault, &started.session.session_id)
                .unwrap()
                .event_count,
            1
        );
    }

    #[test]
    fn concurrent_checkpoints_are_serialized_without_loss() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('c'))).unwrap();
        let barrier = Arc::new(Barrier::new(5));
        let mut workers = Vec::new();
        for (index, digit) in ['1', '2', '3', '4'].into_iter().enumerate() {
            let project = project.clone();
            let vault = vault.clone();
            let session_id = started.session.session_id.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                checkpoint_session(
                    project,
                    vault,
                    &session_id,
                    checkpoint_input(request_id(digit), &format!("Concurrent checkpoint {index}")),
                )
                .unwrap();
            }));
        }
        barrier.wait();
        for worker in workers {
            worker.join().unwrap();
        }
        let session = read_session(&project, &vault, &started.session.session_id).unwrap();
        assert_eq!(session.event_count, 5);
        assert_eq!(session.checkpoints.len(), 4);
        let directory = session_directory(&project, &vault, &started.session.session_id);
        assert_eq!(
            std::fs::read_dir(directory.join(EVENTS_DIRECTORY))
                .unwrap()
                .count(),
            5
        );
    }

    #[test]
    fn corrupted_event_identity_is_rejected() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('d'))).unwrap();
        let event_path = session_directory(&project, &vault, &started.session.session_id)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", started.event_id));
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        value["sequence"] = serde_json::json!(7);
        std::fs::write(&event_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(_))
        ));
    }

    #[test]
    fn forged_checkpoint_revision_is_rejected_during_replay() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('f'))).unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            checkpoint_input(request_id('0'), "Revision integrity"),
        )
        .unwrap();
        let event_path = session_directory(&project, &vault, &started.session.session_id)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", checkpoint.event_id));
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        value["data"]["projectRevision"]["graphSnapshotId"] =
            serde_json::json!(format!("grf_{}", "A".repeat(64)));
        std::fs::write(&event_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_event_entries_are_rejected_without_following_them() {
        use std::os::unix::fs::symlink;

        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('e'))).unwrap();
        let events =
            session_directory(&project, &vault, &started.session.session_id).join(EVENTS_DIRECTORY);
        symlink(
            "/etc/passwd",
            events.join(format!("evt_{}.json", "f".repeat(64))),
        )
        .unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(_))
        ));
    }

    fn collect_file_text(directory: &Path, output: &mut String) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                collect_file_text(&entry.path(), output);
            } else {
                output.push_str(&std::fs::read_to_string(entry.path()).unwrap());
            }
        }
    }
}
