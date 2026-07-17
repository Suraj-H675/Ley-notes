use crate::ingestion::{load_project_memory, redact_secrets};
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

pub const SESSION_SCHEMA_VERSION: u32 = 1;
pub const SESSION_EVENT_LIMIT_BYTES: u64 = 1_048_576;
pub const SESSION_PROJECTION_LIMIT_BYTES: u64 = 67_108_864;
pub const SESSION_EVENT_LIMIT: usize = 10_000;

const STORE_ROOT: &str = ".ley";
const AGENT_MEMORY_DIRECTORY: &str = "agent-memory";
const PROJECTS_DIRECTORY: &str = "projects";
const SESSIONS_DIRECTORY: &str = "sessions";
const EVENTS_DIRECTORY: &str = "events";
const SESSION_LOCK_FILE: &str = "sessions-v1.lock";
const SESSION_FILE: &str = "session-v1.json";
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
pub struct AgentSession {
    pub schema_version: u32,
    pub project_id: String,
    pub session_id: String,
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
    CheckpointRecorded(SessionCheckpoint),
    SessionFinished(SessionFinish),
}

pub fn generate_request_id() -> String {
    format!("req_{}", uuid::Uuid::new_v4().simple())
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
            allow_create: true,
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
            payload: SessionEventPayload::CheckpointRecorded(checkpoint),
            allow_create: false,
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
            allow_create: false,
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

pub fn list_sessions(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
) -> Result<Vec<SessionSummary>, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
    let Some(store) = SessionStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Ok(Vec::new());
    };
    let _lock = store.lock(true)?;
    let mut sessions = Vec::new();
    for session_id in store.session_ids()? {
        let session = store.rebuild_session(&session_id)?;
        sessions.push(SessionSummary::from(&session));
    }
    sessions.sort_by(|left, right| {
        right
            .updated_at_unix_ms
            .cmp(&left.updated_at_unix_ms)
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    Ok(sessions)
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

struct PendingEvent {
    event_id: String,
    request_id: String,
    redactions: Vec<MemoryRedaction>,
    payload: SessionEventPayload,
    allow_create: bool,
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
    let request_fingerprint = request_fingerprint(
        project_id,
        session_id,
        &pending.request_id,
        &pending.payload,
    )?;
    let event_name = format!("{}.json", pending.event_id);
    if let Some(existing) = read_private_file(&events_dir, &event_name, SESSION_EVENT_LIMIT_BYTES)?
    {
        let event: SessionEvent = parse_json(&event_name, &existing)?;
        validate_event(&event, project_id, session_id)?;
        if event.request_fingerprint != request_fingerprint {
            return Err(LeyCoreError::SessionIdempotencyConflict(pending.request_id));
        }
        let session = store.rebuild_session_from_dir(session_id, &session_dir)?;
        store.persist_projection(&session_dir, &session)?;
        return Ok(mutation(session, &pending.event_id, true));
    }
    let existing = store.read_events(session_id, &session_dir)?;
    if existing
        .iter()
        .any(|event| event.request_id == pending.request_id)
    {
        return Err(LeyCoreError::SessionIdempotencyConflict(pending.request_id));
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
        if current.status != SessionStatus::Active {
            return Err(LeyCoreError::InvalidSessionRequest(format!(
                "session {session_id} is already {}",
                enum_label(current.status)
            )));
        }
    }
    let minimum_recorded_at = existing
        .last()
        .map(|event| event.recorded_at_unix_ms)
        .unwrap_or(1);
    let recorded_at_unix_ms =
        normalize_payload_recorded_at(&mut pending.payload, minimum_recorded_at);
    let event = SessionEvent {
        schema_version: SESSION_SCHEMA_VERSION,
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

fn mutation(session: AgentSession, event_id: &str, replayed: bool) -> SessionMutation {
    let base = format!(
        "{STORE_ROOT}/{AGENT_MEMORY_DIRECTORY}/{PROJECTS_DIRECTORY}/{}/{SESSIONS_DIRECTORY}/{}",
        session.project_id, session.session_id
    );
    SessionMutation {
        session,
        event_id: event_id.to_owned(),
        replayed,
        session_path: format!("{base}/{SESSION_FILE}"),
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
    }
}

struct SessionStore {
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
                    .map_err(|e| session_io(name, e))?
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
            let bytes = read_private_file(&events_dir, name, SESSION_EVENT_LIMIT_BYTES)?
                .ok_or_else(|| {
                    LeyCoreError::InvalidSessionStore(format!(
                        "event disappeared while reading: {name}"
                    ))
                })?;
            let event: SessionEvent = parse_json(name, &bytes)?;
            validate_event(&event, &self.project_id, session_id)?;
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

    fn persist_projection(
        &self,
        session_dir: &Dir,
        session: &AgentSession,
    ) -> Result<(), LeyCoreError> {
        let body = json_body(session, SESSION_PROJECTION_LIMIT_BYTES, SESSION_FILE)?;
        write_atomic_private(session_dir, SESSION_FILE, &body)?;
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
        schema_version: SESSION_SCHEMA_VERSION,
        project_id: project_id.to_owned(),
        session_id: session_id.to_owned(),
        name: name.clone(),
        goal: goal.clone(),
        status: SessionStatus::Active,
        source: source.clone(),
        artifact_snapshot_id_at_start: artifact_snapshot_id.clone(),
        started_at_unix_ms: first.recorded_at_unix_ms,
        updated_at_unix_ms: first.recorded_at_unix_ms,
        finished_at_unix_ms: None,
        event_count: events.len() as u64,
        checkpoints: Vec::new(),
        finish: None,
    };
    for event in &events[1..] {
        if session.status != SessionStatus::Active {
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
                session.checkpoints.push(checkpoint.clone());
            }
            SessionEventPayload::SessionFinished(finish) => {
                session.status = finish.status;
                session.finished_at_unix_ms = Some(finish.recorded_at_unix_ms);
                session.finish = Some(finish.clone());
            }
        }
        session.updated_at_unix_ms = event.recorded_at_unix_ms;
    }
    Ok(session)
}

fn validate_event(
    event: &SessionEvent,
    project_id: &str,
    session_id: &str,
) -> Result<(), LeyCoreError> {
    if event.schema_version != SESSION_SCHEMA_VERSION
        || event.project_id != project_id
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
    output.push_str("\n## Capture source\n\n");
    output.push_str(&format!("- Kind: `{}`\n", enum_label(session.source.kind)));
    if let Some(host) = &session.source.host {
        output.push_str(&format!("- Host: {}\n", markdown_inline(host)));
    }
    if let Some(agent) = &session.source.agent {
        output.push_str(&format!("- Agent: {}\n", markdown_inline(agent)));
    }
    for (index, checkpoint) in session.checkpoints.iter().enumerate() {
        output.push_str(&format!(
            "\n## Checkpoint {} · {}\n\n",
            index + 1,
            checkpoint.recorded_at_unix_ms
        ));
        push_quote(&mut output, &checkpoint.summary);
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
    use std::sync::{Arc, Barrier};
    use tempfile::tempdir;

    fn setup_memory() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Session test"), CaptureMode::Structured).unwrap();
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

        let directory = session_directory(&project, &vault, &started.session.session_id);
        let markdown = std::fs::read_to_string(directory.join(SESSION_MARKDOWN_FILE)).unwrap();
        assert!(markdown.contains("# Implementation session"));
        assert!(markdown.contains("## Problems and outcomes"));
        assert!(markdown.contains("README.md"));
        assert!(markdown.contains("### Final response"));
        assert!(markdown.contains("Implemented and verified the lifecycle"));
        std::fs::remove_file(directory.join(SESSION_FILE)).unwrap();
        std::fs::remove_file(directory.join(SESSION_MARKDOWN_FILE)).unwrap();
        assert_eq!(
            read_session(&project, &vault, &started.session.session_id)
                .unwrap()
                .event_count,
            3
        );
        let listed = list_sessions(&project, &vault).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, SessionStatus::Completed);
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
