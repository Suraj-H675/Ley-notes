use crate::ingestion::{
    load_project_memory, lock_project_memory_lifecycle, redact_secrets, ArtifactMediaType,
    ProjectMemoryLifecycleLock,
};
use crate::learning::erase_learnings_citing_session_under_lifecycle;
use crate::retrieval::{project_artifact_snapshot_id, validate_project_memory};
use crate::{
    diagnose_project, AgentEgressTarget, CompiledContextPack, LeyCoreError,
    ProjectMemoryResultKind, RedactionFinding,
};
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
pub const SESSION_RECOVERY_SCHEMA_VERSION: u32 = 3;
pub const SESSION_VERIFICATION_EVIDENCE_SCHEMA_VERSION: u32 = 4;
pub const SESSION_CONTEXT_UTILITY_SCHEMA_VERSION: u32 = 5;
pub const SESSION_MULTIMODAL_EVIDENCE_SCHEMA_VERSION: u32 = 6;
pub const SESSION_IMPORTED_TURN_SCHEMA_VERSION: u32 = 7;
pub const SESSION_TYPED_RECOVERY_SCHEMA_VERSION: u32 = 8;
pub const SESSION_TASK_RECOVERY_SCHEMA_VERSION: u32 = 9;
pub const SESSION_PLAN_RECOVERY_SCHEMA_VERSION: u32 = 10;
pub const SESSION_EVENT_LIMIT_BYTES: u64 = 1_048_576;
pub const SESSION_PROJECTION_LIMIT_BYTES: u64 = 67_108_864;
pub const SESSION_EVENT_LIMIT: usize = 10_000;
pub const SESSION_TURN_EVIDENCE_LIMIT_BYTES: usize = 1_048_576;
pub const SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS: usize = 4_000;
pub const SESSION_RESPONSE_EVIDENCE_LIMIT_CHARACTERS: usize = 8_000;
const SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT: usize = 1_000;
pub const SESSION_CONTEXT_UTILITY_OUTCOME_LIMIT: usize = 20;
pub const SESSION_CONTEXT_UTILITY_INCLUDED_RECORD_LIMIT: usize = 64;

const STORE_ROOT: &str = ".ley";
const AGENT_MEMORY_DIRECTORY: &str = "agent-memory";
const PROJECTS_DIRECTORY: &str = "projects";
const SESSIONS_DIRECTORY: &str = "sessions";
const EVENTS_DIRECTORY: &str = "events";
const SESSION_LOCK_FILE: &str = "sessions-v1.lock";
const SESSION_FILE: &str = "session-v1.json";
const SESSION_V2_FILE: &str = "session-v2.json";
const SESSION_V3_FILE: &str = "session-v3.json";
const SESSION_V4_FILE: &str = "session-v4.json";
const SESSION_V5_FILE: &str = "session-v5.json";
const SESSION_V6_FILE: &str = "session-v6.json";
const SESSION_V7_FILE: &str = "session-v7.json";
const SESSION_V8_FILE: &str = "session-v8.json";
const SESSION_V9_FILE: &str = "session-v9.json";
const SESSION_V10_FILE: &str = "session-v10.json";
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<String>,
}

impl Default for SessionSource {
    fn default() -> Self {
        Self {
            kind: SessionSourceKind::ManualCli,
            host: None,
            agent: None,
            source_reference: None,
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
    #[serde(default)]
    pub evidence_artifact_paths: Vec<String>,
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
    Import,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_recorded_at_unix_ms: Option<u64>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<ArtifactMediaType>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_artifacts: Vec<SessionArtifactCitation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextUtilityRecordSource {
    Specification,
    ActiveProjectMemory,
    MountedReference,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextUtilityIncludedRecord {
    pub source: ContextUtilityRecordSource,
    pub entity_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specification_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_project_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextUtilityOutcomeKind {
    Checkpoint,
    SessionFinish,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextUtilityOutcomeEvidence {
    pub event_id: String,
    pub recorded_at_unix_ms: u64,
    pub kind: ContextUtilityOutcomeKind,
    pub completed_tasks: usize,
    pub blocked_tasks: usize,
    pub cancelled_tasks: usize,
    pub resolved_problems: usize,
    pub helped_attempts: usize,
    pub no_effect_attempts: usize,
    pub worsened_attempts: usize,
    pub unknown_attempts: usize,
    pub passed_verifications: usize,
    pub failed_verifications: usize,
    pub skipped_verifications: usize,
    pub unknown_verifications: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_status: Option<SessionStatus>,
    pub unresolved_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextUtilityBindingInput {
    pub request_id: String,
    pub expected_event_count: u64,
    pub expected_context_pack_id: String,
    pub task: String,
    pub max_results: usize,
    pub max_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextUtilityBinding {
    pub id: String,
    pub event_id: String,
    pub recorded_at_unix_ms: u64,
    pub expected_event_count: u64,
    pub context_pack_id: String,
    pub task_excerpt: String,
    pub artifact_snapshot_id: String,
    pub graph_snapshot_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_target: Option<AgentEgressTarget>,
    pub max_results: usize,
    pub max_tokens: usize,
    pub estimated_tokens: usize,
    pub included_records: Vec<ContextUtilityIncludedRecord>,
    pub omitted_included_records: usize,
    pub context_pack_revalidated: bool,
    pub context_usage_proven: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextUtilityObservationInput {
    pub request_id: String,
    pub expected_event_count: u64,
    pub binding_id: String,
    pub downstream_event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextUtilityObservation {
    pub id: String,
    pub event_id: String,
    pub recorded_at_unix_ms: u64,
    pub expected_event_count: u64,
    pub binding_id: String,
    pub context_pack_id: String,
    pub downstream_event_ids: Vec<String>,
    pub downstream_outcomes: Vec<ContextUtilityOutcomeEvidence>,
    pub context_usage_proven: bool,
    pub causal_utility_proven: bool,
    pub trust_changes_applied: bool,
    pub ranking_changes_applied: bool,
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
    #[serde(default)]
    pub context_utility_bindings: Vec<ContextUtilityBinding>,
    #[serde(default)]
    pub context_utility_observations: Vec<ContextUtilityObservation>,
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
    pub source_kind: SessionSourceKind,
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecoveryCheckpointProvenance {
    expected_event_count: u64,
    candidate_fingerprint: String,
    binding_fingerprint: String,
    evidence_record_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecoveryCheckpointEvent {
    checkpoint: Box<SessionCheckpoint>,
    provenance: RecoveryCheckpointProvenance,
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
    RecoveryCheckpointRecorded(RecoveryCheckpointEvent),
    SessionFinished(SessionFinish),
    ContextUtilityBound(ContextUtilityBinding),
    ContextUtilityObserved(ContextUtilityObservation),
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
    record_session_turn(project_start, vault, session_id, input, None, true)
}

/// Records an observed assistant response as a first-class immutable event.
pub fn record_session_response(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: TurnEvidenceInput,
) -> Result<SessionMutation, LeyCoreError> {
    record_session_turn(project_start, vault, session_id, input, None, false)
}

pub(crate) fn record_imported_session_prompt(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: TurnEvidenceInput,
    source_recorded_at_unix_ms: u64,
) -> Result<SessionMutation, LeyCoreError> {
    if input.origin != TurnEvidenceOrigin::Import {
        return Err(LeyCoreError::InvalidSessionRequest(
            "imported turn evidence must use import origin".to_owned(),
        ));
    }
    record_session_turn(
        project_start,
        vault,
        session_id,
        input,
        Some(source_recorded_at_unix_ms),
        true,
    )
}

fn record_session_turn(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: TurnEvidenceInput,
    source_recorded_at_unix_ms: Option<u64>,
    is_prompt: bool,
) -> Result<SessionMutation, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
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
        source_recorded_at_unix_ms,
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
            schema_version: if source_recorded_at_unix_ms.is_some() {
                SESSION_IMPORTED_TURN_SCHEMA_VERSION
            } else {
                SESSION_SCHEMA_VERSION
            },
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
    let artifact_snapshot_id = project_artifact_snapshot_id(&diagnostic.root, &vault)?;
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
        artifact_snapshot_id,
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
    checkpoint_session_with_expected_count(project_start, vault, session_id, input, None)
}

/// Appends a checkpoint only if the caller compiled/reviewed the current
/// session event count. Exact retries of an already-written request remain
/// idempotent even after the session advances.
pub fn checkpoint_session_if_current(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    expected_event_count: u64,
    input: CheckpointInput,
) -> Result<SessionMutation, LeyCoreError> {
    checkpoint_session_with_expected_count(
        project_start,
        vault,
        session_id,
        input,
        Some(expected_event_count),
    )
}

fn checkpoint_session_with_expected_count(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: CheckpointInput,
    expected_event_count: Option<u64>,
) -> Result<SessionMutation, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    let request_id = input.request_id.clone();
    let diagnostic = diagnose_project(&project_start)?;
    let memory = load_project_memory(&diagnostic.root, &vault)?;
    let has_verification_evidence = input
        .verification
        .iter()
        .any(|verification| !verification.evidence_artifact_paths.is_empty());
    let has_multimodal_evidence = input
        .touched_artifacts
        .iter()
        .chain(
            input
                .verification
                .iter()
                .flat_map(|verification| verification.evidence_artifact_paths.iter()),
        )
        .any(|path| {
            memory
                .manifest
                .files
                .iter()
                .any(|artifact| artifact.path == *path && artifact.media_type.is_some())
        });
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
            schema_version: if has_multimodal_evidence {
                SESSION_MULTIMODAL_EVIDENCE_SCHEMA_VERSION
            } else if has_verification_evidence {
                SESSION_VERIFICATION_EVIDENCE_SCHEMA_VERSION
            } else {
                SESSION_V1_SCHEMA_VERSION
            },
            allow_create: false,
            expected_event_count,
        },
        vault,
    )
}

#[derive(Debug, Clone)]
pub(crate) struct RecoveredUnresolvedCheckpointInput {
    pub request_id: String,
    pub expected_event_count: u64,
    pub candidate_fingerprint: String,
    pub evidence_record_ids: Vec<String>,
    pub summary: String,
    pub unresolved: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveredStructuredKind {
    Decision,
    Problem,
}

#[derive(Debug, Clone)]
pub(crate) struct RecoveredStructuredCheckpointInput {
    pub request_id: String,
    pub expected_event_count: u64,
    pub candidate_fingerprint: String,
    pub evidence_record_ids: Vec<String>,
    pub kind: RecoveredStructuredKind,
    pub subject: String,
    pub statement: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RecoveredTaskCheckpointInput {
    pub request_id: String,
    pub expected_event_count: u64,
    pub candidate_fingerprint: String,
    pub evidence_record_ids: Vec<String>,
    pub title: String,
    pub status: TaskStatus,
    pub details: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RecoveredPlanCheckpointInput {
    pub request_id: String,
    pub expected_event_count: u64,
    pub candidate_fingerprint: String,
    pub evidence_record_ids: Vec<String>,
    pub text: String,
    pub status: PlanStatus,
}

pub(crate) fn replay_recovered_unresolved_session_if_present(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: RecoveredUnresolvedCheckpointInput,
) -> Result<Option<SessionMutation>, LeyCoreError> {
    let (project_id, pending) =
        recovered_unresolved_pending(project_start.as_ref(), vault.as_ref(), session_id, input)?;
    replay_pending_event_if_present(&project_id, session_id, pending, vault)
}

pub(crate) fn checkpoint_recovered_unresolved_session(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: RecoveredUnresolvedCheckpointInput,
) -> Result<SessionMutation, LeyCoreError> {
    let (project_id, pending) =
        recovered_unresolved_pending(project_start.as_ref(), vault.as_ref(), session_id, input)?;
    mutate_session(&project_id, session_id, pending, vault)
}

pub(crate) fn replay_recovered_structured_session_if_present(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: RecoveredStructuredCheckpointInput,
) -> Result<Option<SessionMutation>, LeyCoreError> {
    let (project_id, pending) =
        recovered_structured_pending(project_start.as_ref(), vault.as_ref(), session_id, input)?;
    replay_pending_event_if_present(&project_id, session_id, pending, vault)
}

pub(crate) fn checkpoint_recovered_structured_session(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: RecoveredStructuredCheckpointInput,
) -> Result<SessionMutation, LeyCoreError> {
    let (project_id, pending) =
        recovered_structured_pending(project_start.as_ref(), vault.as_ref(), session_id, input)?;
    mutate_session(&project_id, session_id, pending, vault)
}

pub(crate) fn replay_recovered_task_session_if_present(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: RecoveredTaskCheckpointInput,
) -> Result<Option<SessionMutation>, LeyCoreError> {
    let (project_id, pending) =
        recovered_task_pending(project_start.as_ref(), vault.as_ref(), session_id, input)?;
    replay_pending_event_if_present(&project_id, session_id, pending, vault)
}

pub(crate) fn checkpoint_recovered_task_session(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: RecoveredTaskCheckpointInput,
) -> Result<SessionMutation, LeyCoreError> {
    let (project_id, pending) =
        recovered_task_pending(project_start.as_ref(), vault.as_ref(), session_id, input)?;
    mutate_session(&project_id, session_id, pending, vault)
}

pub(crate) fn replay_recovered_plan_session_if_present(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: RecoveredPlanCheckpointInput,
) -> Result<Option<SessionMutation>, LeyCoreError> {
    let (project_id, pending) =
        recovered_plan_pending(project_start.as_ref(), vault.as_ref(), session_id, input)?;
    replay_pending_event_if_present(&project_id, session_id, pending, vault)
}

pub(crate) fn checkpoint_recovered_plan_session(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: RecoveredPlanCheckpointInput,
) -> Result<SessionMutation, LeyCoreError> {
    let (project_id, pending) =
        recovered_plan_pending(project_start.as_ref(), vault.as_ref(), session_id, input)?;
    mutate_session(&project_id, session_id, pending, vault)
}

fn recovered_unresolved_pending(
    project_start: &Path,
    vault: &Path,
    session_id: &str,
    input: RecoveredUnresolvedCheckpointInput,
) -> Result<(String, PendingEvent), LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    if !is_sha256(&input.candidate_fingerprint) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery candidate fingerprint must be sha256".to_owned(),
        ));
    }
    if input.evidence_record_ids.is_empty()
        || input.evidence_record_ids.len() > SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT
    {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "recovery binding must cite between 1 and {SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT} evidence records"
        )));
    }
    let mut evidence_record_ids = input.evidence_record_ids;
    evidence_record_ids.sort();
    if evidence_record_ids
        .windows(2)
        .any(|window| window[0] == window[1])
        || evidence_record_ids
            .iter()
            .any(|record_id| !valid_prefixed_hex(record_id, "tev_", 32))
    {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery binding evidence IDs must be unique tev_ identifiers".to_owned(),
        ));
    }

    let diagnostic = diagnose_project(project_start)?;
    let memory = load_project_memory(&diagnostic.root, vault)?;
    let event_id = deterministic_id(
        "evt",
        &format!(
            "{session_id}:{}:recovery-checkpoint-recorded",
            input.request_id
        ),
        64,
    );
    let recorded_at = unix_time_ms();
    let checkpoint_input = CheckpointInput {
        request_id: input.request_id.clone(),
        summary: input.summary,
        plan: Vec::new(),
        decisions: Vec::new(),
        tasks: Vec::new(),
        problems: Vec::new(),
        touched_artifacts: Vec::new(),
        commands: Vec::new(),
        verification: Vec::new(),
        unresolved: vec![input.unresolved],
    };
    let (checkpoint, redactions) =
        normalize_checkpoint(checkpoint_input, &event_id, recorded_at, &memory)?;
    let binding_fingerprint = recovery_binding_fingerprint(
        session_id,
        input.expected_event_count,
        &input.candidate_fingerprint,
        &evidence_record_ids,
        &checkpoint.summary,
        &checkpoint.unresolved[0],
    );
    Ok((
        diagnostic.identity.project_id,
        PendingEvent {
            event_id,
            request_id: input.request_id,
            redactions,
            payload: SessionEventPayload::RecoveryCheckpointRecorded(RecoveryCheckpointEvent {
                checkpoint: Box::new(checkpoint),
                provenance: RecoveryCheckpointProvenance {
                    expected_event_count: input.expected_event_count,
                    candidate_fingerprint: input.candidate_fingerprint,
                    binding_fingerprint,
                    evidence_record_ids,
                },
            }),
            schema_version: SESSION_RECOVERY_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: Some(input.expected_event_count),
        },
    ))
}

fn recovered_structured_pending(
    project_start: &Path,
    vault: &Path,
    session_id: &str,
    input: RecoveredStructuredCheckpointInput,
) -> Result<(String, PendingEvent), LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    if !is_sha256(&input.candidate_fingerprint) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery candidate fingerprint must be sha256".to_owned(),
        ));
    }
    if input.evidence_record_ids.is_empty()
        || input.evidence_record_ids.len() > SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT
    {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "recovery binding must cite between 1 and {SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT} evidence records"
        )));
    }
    let mut evidence_record_ids = input.evidence_record_ids;
    evidence_record_ids.sort();
    if evidence_record_ids
        .windows(2)
        .any(|window| window[0] == window[1])
        || evidence_record_ids
            .iter()
            .any(|record_id| !valid_prefixed_hex(record_id, "tev_", 32))
    {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery binding evidence IDs must be unique tev_ identifiers".to_owned(),
        ));
    }

    let diagnostic = diagnose_project(project_start)?;
    let memory = load_project_memory(&diagnostic.root, vault)?;
    let event_id = deterministic_id(
        "evt",
        &format!(
            "{session_id}:{}:recovery-checkpoint-recorded",
            input.request_id
        ),
        64,
    );
    let recorded_at = unix_time_ms();
    let (decisions, problems) = match input.kind {
        RecoveredStructuredKind::Decision => (
            vec![DecisionInput {
                title: input.subject.clone(),
                decision: input.statement.clone(),
                rationale: String::new(),
                alternatives: Vec::new(),
            }],
            Vec::new(),
        ),
        RecoveredStructuredKind::Problem => (
            Vec::new(),
            vec![ProblemInput {
                title: input.subject.clone(),
                symptom: input.statement.clone(),
                expected: String::new(),
                attempts: Vec::new(),
                resolution: None,
            }],
        ),
    };
    let checkpoint_input = CheckpointInput {
        request_id: input.request_id.clone(),
        summary: input.subject,
        plan: Vec::new(),
        decisions,
        tasks: Vec::new(),
        problems,
        touched_artifacts: Vec::new(),
        commands: Vec::new(),
        verification: Vec::new(),
        unresolved: Vec::new(),
    };
    let (checkpoint, redactions) =
        normalize_checkpoint(checkpoint_input, &event_id, recorded_at, &memory)?;
    let (kind, subject, statement) =
        typed_recovery_checkpoint_claim(&checkpoint).ok_or_else(|| {
            LeyCoreError::InvalidSessionRequest(
                "typed recovery checkpoint normalization produced an unsupported shape".to_owned(),
            )
        })?;
    let binding_fingerprint = recovery_binding_fingerprint_v2(
        session_id,
        input.expected_event_count,
        &input.candidate_fingerprint,
        &evidence_record_ids,
        kind,
        subject,
        statement,
    );
    Ok((
        diagnostic.identity.project_id,
        PendingEvent {
            event_id,
            request_id: input.request_id,
            redactions,
            payload: SessionEventPayload::RecoveryCheckpointRecorded(RecoveryCheckpointEvent {
                checkpoint: Box::new(checkpoint),
                provenance: RecoveryCheckpointProvenance {
                    expected_event_count: input.expected_event_count,
                    candidate_fingerprint: input.candidate_fingerprint,
                    binding_fingerprint,
                    evidence_record_ids,
                },
            }),
            schema_version: SESSION_TYPED_RECOVERY_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: Some(input.expected_event_count),
        },
    ))
}

fn recovered_task_pending(
    project_start: &Path,
    vault: &Path,
    session_id: &str,
    input: RecoveredTaskCheckpointInput,
) -> Result<(String, PendingEvent), LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    if !is_sha256(&input.candidate_fingerprint) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery candidate fingerprint must be sha256".to_owned(),
        ));
    }
    if input.evidence_record_ids.is_empty()
        || input.evidence_record_ids.len() > SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT
    {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "recovery binding must cite between 1 and {SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT} evidence records"
        )));
    }
    let mut evidence_record_ids = input.evidence_record_ids;
    evidence_record_ids.sort();
    if evidence_record_ids
        .windows(2)
        .any(|window| window[0] == window[1])
        || evidence_record_ids
            .iter()
            .any(|record_id| !valid_prefixed_hex(record_id, "tev_", 32))
    {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery binding evidence IDs must be unique tev_ identifiers".to_owned(),
        ));
    }

    let diagnostic = diagnose_project(project_start)?;
    let memory = load_project_memory(&diagnostic.root, vault)?;
    let event_id = deterministic_id(
        "evt",
        &format!(
            "{session_id}:{}:recovery-checkpoint-recorded",
            input.request_id
        ),
        64,
    );
    let recorded_at = unix_time_ms();
    let checkpoint_input = CheckpointInput {
        request_id: input.request_id.clone(),
        summary: input.title.clone(),
        plan: Vec::new(),
        decisions: Vec::new(),
        tasks: vec![TaskInput {
            title: input.title,
            status: input.status,
            details: input.details,
        }],
        problems: Vec::new(),
        touched_artifacts: Vec::new(),
        commands: Vec::new(),
        verification: Vec::new(),
        unresolved: Vec::new(),
    };
    let (checkpoint, redactions) =
        normalize_checkpoint(checkpoint_input, &event_id, recorded_at, &memory)?;
    let (title, status, details) =
        task_recovery_checkpoint_claim(&checkpoint).ok_or_else(|| {
            LeyCoreError::InvalidSessionRequest(
                "task recovery checkpoint normalization produced an unsupported shape".to_owned(),
            )
        })?;
    let binding_fingerprint = recovery_binding_fingerprint_v3(
        session_id,
        input.expected_event_count,
        &input.candidate_fingerprint,
        &evidence_record_ids,
        title,
        status,
        details,
    );
    Ok((
        diagnostic.identity.project_id,
        PendingEvent {
            event_id,
            request_id: input.request_id,
            redactions,
            payload: SessionEventPayload::RecoveryCheckpointRecorded(RecoveryCheckpointEvent {
                checkpoint: Box::new(checkpoint),
                provenance: RecoveryCheckpointProvenance {
                    expected_event_count: input.expected_event_count,
                    candidate_fingerprint: input.candidate_fingerprint,
                    binding_fingerprint,
                    evidence_record_ids,
                },
            }),
            schema_version: SESSION_TASK_RECOVERY_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: Some(input.expected_event_count),
        },
    ))
}

fn recovered_plan_pending(
    project_start: &Path,
    vault: &Path,
    session_id: &str,
    input: RecoveredPlanCheckpointInput,
) -> Result<(String, PendingEvent), LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    if !is_sha256(&input.candidate_fingerprint) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery candidate fingerprint must be sha256".to_owned(),
        ));
    }
    if input.evidence_record_ids.is_empty()
        || input.evidence_record_ids.len() > SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT
    {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "recovery binding must cite between 1 and {SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT} evidence records"
        )));
    }
    let mut evidence_record_ids = input.evidence_record_ids;
    evidence_record_ids.sort();
    if evidence_record_ids
        .windows(2)
        .any(|window| window[0] == window[1])
        || evidence_record_ids
            .iter()
            .any(|record_id| !valid_prefixed_hex(record_id, "tev_", 32))
    {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery binding evidence IDs must be unique tev_ identifiers".to_owned(),
        ));
    }

    let diagnostic = diagnose_project(project_start)?;
    let memory = load_project_memory(&diagnostic.root, vault)?;
    let event_id = deterministic_id(
        "evt",
        &format!(
            "{session_id}:{}:recovery-checkpoint-recorded",
            input.request_id
        ),
        64,
    );
    let recorded_at = unix_time_ms();
    let checkpoint_input = CheckpointInput {
        request_id: input.request_id.clone(),
        summary: input.text.clone(),
        plan: vec![PlanItemInput {
            text: input.text,
            status: input.status,
        }],
        decisions: Vec::new(),
        tasks: Vec::new(),
        problems: Vec::new(),
        touched_artifacts: Vec::new(),
        commands: Vec::new(),
        verification: Vec::new(),
        unresolved: Vec::new(),
    };
    let (checkpoint, redactions) =
        normalize_checkpoint(checkpoint_input, &event_id, recorded_at, &memory)?;
    let (text, status) = plan_recovery_checkpoint_claim(&checkpoint).ok_or_else(|| {
        LeyCoreError::InvalidSessionRequest(
            "plan recovery checkpoint normalization produced an unsupported shape".to_owned(),
        )
    })?;
    let binding_fingerprint = recovery_binding_fingerprint_v4(
        session_id,
        input.expected_event_count,
        &input.candidate_fingerprint,
        &evidence_record_ids,
        text,
        status,
    );
    Ok((
        diagnostic.identity.project_id,
        PendingEvent {
            event_id,
            request_id: input.request_id,
            redactions,
            payload: SessionEventPayload::RecoveryCheckpointRecorded(RecoveryCheckpointEvent {
                checkpoint: Box::new(checkpoint),
                provenance: RecoveryCheckpointProvenance {
                    expected_event_count: input.expected_event_count,
                    candidate_fingerprint: input.candidate_fingerprint,
                    binding_fingerprint,
                    evidence_record_ids,
                },
            }),
            schema_version: SESSION_PLAN_RECOVERY_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: Some(input.expected_event_count),
        },
    ))
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
    validate_project_memory(&diagnostic.root, &vault)?;
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

pub fn bind_context_utility_pack(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: ContextUtilityBindingInput,
    pack: &CompiledContextPack,
) -> Result<SessionMutation, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    if input.expected_event_count == 0 {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility expectedEventCount must be positive".to_owned(),
        ));
    }
    if !valid_prefixed_hex(&input.expected_context_pack_id, "cpk_", 64)
        || !valid_prefixed_hex(&pack.context_pack_id, "cpk_", 64)
    {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility contextPackId must be a cpk_ sha256 identifier".to_owned(),
        ));
    }
    if input.expected_context_pack_id != pack.context_pack_id {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context pack changed; recompile before recording utility evidence".to_owned(),
        ));
    }
    if input.task != pack.task {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility task does not match the supplied pack".to_owned(),
        ));
    }
    if !(1..=20).contains(&input.max_results) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility maxResults must be between 1 and 20".to_owned(),
        ));
    }
    if !(500..=8_000).contains(&input.max_tokens) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility maxTokens must be between 500 and 8000".to_owned(),
        ));
    }
    if input.max_tokens != pack.max_tokens {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility maxTokens does not match the supplied pack".to_owned(),
        ));
    }
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
    if pack.project_id != diagnostic.identity.project_id {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility pack belongs to a different project".to_owned(),
        ));
    }
    if !valid_prefixed_hex(&pack.artifact_snapshot_id, "snp_", 64)
        || !valid_prefixed_hex(&pack.graph_snapshot_id, "grf_", 64)
    {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility pack snapshot identity is invalid".to_owned(),
        ));
    }

    let event_id = deterministic_id(
        "evt",
        &format!("{session_id}:{}:context-utility-bound", input.request_id),
        64,
    );
    let recorded_at_unix_ms = unix_time_ms();
    let mut redactions = Vec::new();
    let task_excerpt = sanitize_text(
        "contextUtility.taskExcerpt",
        &pack.task,
        1,
        256,
        &mut redactions,
    )?;
    let (included_records, omitted_included_records) = context_utility_included_records(pack);
    let binding = ContextUtilityBinding {
        id: child_id("cub", &event_id, 0),
        event_id: event_id.clone(),
        recorded_at_unix_ms,
        expected_event_count: input.expected_event_count,
        context_pack_id: pack.context_pack_id.clone(),
        task_excerpt,
        artifact_snapshot_id: pack.artifact_snapshot_id.clone(),
        graph_snapshot_id: pack.graph_snapshot_id.clone(),
        egress_target: pack.egress_target,
        max_results: input.max_results,
        max_tokens: input.max_tokens,
        estimated_tokens: pack.estimated_tokens,
        included_records,
        omitted_included_records,
        context_pack_revalidated: true,
        context_usage_proven: false,
    };
    mutate_session(
        &diagnostic.identity.project_id,
        session_id,
        PendingEvent {
            event_id,
            request_id: input.request_id,
            redactions,
            payload: SessionEventPayload::ContextUtilityBound(binding),
            schema_version: SESSION_CONTEXT_UTILITY_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: Some(input.expected_event_count),
        },
        vault,
    )
}

pub fn replay_context_utility_binding_if_present(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    input: &ContextUtilityBindingInput,
) -> Result<Option<SessionMutation>, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    if input.expected_event_count == 0 {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility expectedEventCount must be positive".to_owned(),
        ));
    }
    if !valid_prefixed_hex(&input.expected_context_pack_id, "cpk_", 64) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility contextPackId must be a cpk_ sha256 identifier".to_owned(),
        ));
    }
    if !(1..=20).contains(&input.max_results) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility maxResults must be between 1 and 20".to_owned(),
        ));
    }
    if !(500..=8_000).contains(&input.max_tokens) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility maxTokens must be between 500 and 8000".to_owned(),
        ));
    }
    let mut task_redactions = Vec::new();
    let task_excerpt = sanitize_text(
        "contextUtility.taskExcerpt",
        &input.task,
        1,
        256,
        &mut task_redactions,
    )?;

    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
    let event_id = deterministic_id(
        "evt",
        &format!("{session_id}:{}:context-utility-bound", input.request_id),
        64,
    );
    let Some(store) = SessionStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Err(LeyCoreError::SessionNotFound(session_id.to_owned()));
    };
    let _lock = store.lock(false)?;
    let session_dir = store.open_session(session_id)?;
    let existing = store.read_events(session_id, &session_dir)?;
    let Some(event) = existing.iter().find(|event| event.event_id == event_id) else {
        if existing
            .iter()
            .any(|event| event.request_id == input.request_id)
        {
            return Err(LeyCoreError::SessionIdempotencyConflict(
                input.request_id.clone(),
            ));
        }
        return Ok(None);
    };
    let SessionEventPayload::ContextUtilityBound(binding) = &event.payload else {
        return Err(LeyCoreError::InvalidSessionStore(
            "context utility binding event has the wrong payload kind".to_owned(),
        ));
    };
    if event.request_id != input.request_id
        || binding.expected_event_count != input.expected_event_count
        || binding.context_pack_id != input.expected_context_pack_id
        || binding.task_excerpt != task_excerpt
        || binding.max_results != input.max_results
        || binding.max_tokens != input.max_tokens
    {
        return Err(LeyCoreError::SessionIdempotencyConflict(
            input.request_id.clone(),
        ));
    }
    let session = store.rebuild_session_from_dir(session_id, &session_dir)?;
    store.persist_projection(&session_dir, &session)?;
    Ok(Some(mutation(session, &event_id, true)))
}

pub fn record_context_utility_observation(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    mut input: ContextUtilityObservationInput,
) -> Result<SessionMutation, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_request_id(&input.request_id)?;
    if input.expected_event_count == 0 {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility expectedEventCount must be positive".to_owned(),
        ));
    }
    if !valid_prefixed_hex(&input.binding_id, "cub_", 32) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility bindingId must be a cub_ identifier".to_owned(),
        ));
    }
    if input.downstream_event_ids.is_empty()
        || input.downstream_event_ids.len() > SESSION_CONTEXT_UTILITY_OUTCOME_LIMIT
    {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "context utility must cite between 1 and {SESSION_CONTEXT_UTILITY_OUTCOME_LIMIT} downstream events"
        )));
    }
    for event_id in &input.downstream_event_ids {
        validate_event_id(event_id)?;
    }
    input.downstream_event_ids.sort();
    if input
        .downstream_event_ids
        .windows(2)
        .any(|window| window[0] == window[1])
    {
        return Err(LeyCoreError::InvalidSessionRequest(
            "context utility downstream event IDs must be unique".to_owned(),
        ));
    }
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
    let event_id = deterministic_id(
        "evt",
        &format!("{session_id}:{}:context-utility-observed", input.request_id),
        64,
    );
    let observation = ContextUtilityObservation {
        id: child_id("cut", &event_id, 0),
        event_id: event_id.clone(),
        recorded_at_unix_ms: unix_time_ms(),
        expected_event_count: input.expected_event_count,
        binding_id: input.binding_id,
        context_pack_id: String::new(),
        downstream_event_ids: input.downstream_event_ids,
        downstream_outcomes: Vec::new(),
        context_usage_proven: false,
        causal_utility_proven: false,
        trust_changes_applied: false,
        ranking_changes_applied: false,
    };
    mutate_session(
        &diagnostic.identity.project_id,
        session_id,
        PendingEvent {
            event_id,
            request_id: input.request_id,
            redactions: Vec::new(),
            payload: SessionEventPayload::ContextUtilityObserved(observation),
            schema_version: SESSION_CONTEXT_UTILITY_SCHEMA_VERSION,
            allow_create: false,
            expected_event_count: Some(input.expected_event_count),
        },
        vault,
    )
}

fn context_utility_included_records(
    pack: &CompiledContextPack,
) -> (Vec<ContextUtilityIncludedRecord>, usize) {
    let mut records = Vec::new();
    for specification in &pack.specifications {
        records.push(ContextUtilityIncludedRecord {
            source: ContextUtilityRecordSource::Specification,
            entity_id: specification.specification_id.clone(),
            kind: Some("specification".to_owned()),
            session_id: None,
            learning_id: None,
            specification_id: Some(specification.specification_id.clone()),
            mount_id: None,
            source_project_id: None,
        });
    }
    for item in &pack.items {
        records.push(ContextUtilityIncludedRecord {
            source: ContextUtilityRecordSource::ActiveProjectMemory,
            entity_id: item.entity_id.clone(),
            kind: Some(project_memory_result_kind_label(item.kind).to_owned()),
            session_id: item.session_id.clone(),
            learning_id: item.learning_id.clone(),
            specification_id: None,
            mount_id: None,
            source_project_id: None,
        });
    }
    for item in &pack.mounted_references {
        records.push(ContextUtilityIncludedRecord {
            source: ContextUtilityRecordSource::MountedReference,
            entity_id: item.entity_id.clone(),
            kind: Some(project_memory_result_kind_label(item.kind).to_owned()),
            session_id: item.session_id.clone(),
            learning_id: item.learning_id.clone(),
            specification_id: None,
            mount_id: Some(item.mount_id.clone()),
            source_project_id: Some(item.source_project_id.clone()),
        });
    }
    records.sort();
    records.dedup();
    let omitted = records
        .len()
        .saturating_sub(SESSION_CONTEXT_UTILITY_INCLUDED_RECORD_LIMIT);
    records.truncate(SESSION_CONTEXT_UTILITY_INCLUDED_RECORD_LIMIT);
    (records, omitted)
}

fn project_memory_result_kind_label(kind: ProjectMemoryResultKind) -> &'static str {
    match kind {
        ProjectMemoryResultKind::Session => "session",
        ProjectMemoryResultKind::Revision => "revision",
        ProjectMemoryResultKind::Decision => "decision",
        ProjectMemoryResultKind::Problem => "problem",
        ProjectMemoryResultKind::Learning => "learning",
        ProjectMemoryResultKind::Artifact => "artifact",
        ProjectMemoryResultKind::Symbol => "symbol",
        ProjectMemoryResultKind::Dependency => "dependency",
    }
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
    validate_project_memory(&diagnostic.root, &vault)?;
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
    validate_project_memory(&diagnostic.root, &vault)?;
    let Some(store) = SessionStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Err(LeyCoreError::SessionNotFound(session_id.to_owned()));
    };
    let _lock = store.lock(true)?;
    store.rebuild_session(session_id)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionRecoveryDerivationOrigin {
    pub candidate_fingerprint: String,
    pub evidence_record_ids: Vec<String>,
}

pub(crate) fn read_recovery_derivation_origin(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
    checkpoint_event_id: &str,
) -> Result<Option<SessionRecoveryDerivationOrigin>, LeyCoreError> {
    validate_session_id(session_id)?;
    validate_event_id(checkpoint_event_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
    let Some(store) = SessionStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Err(LeyCoreError::SessionNotFound(session_id.to_owned()));
    };
    let _lock = store.lock(true)?;
    let session_dir = store.open_session(session_id)?;
    let events = store.read_events(session_id, &session_dir)?;
    let event = events
        .iter()
        .find(|event| event.event_id == checkpoint_event_id)
        .ok_or_else(|| {
            LeyCoreError::InvalidSessionStore(
                "learning evidence checkpoint event is missing from its session ledger".to_owned(),
            )
        })?;
    match &event.payload {
        SessionEventPayload::CheckpointRecorded(_) => Ok(None),
        SessionEventPayload::RecoveryCheckpointRecorded(recovery) => {
            Ok(Some(SessionRecoveryDerivationOrigin {
                candidate_fingerprint: recovery.provenance.candidate_fingerprint.clone(),
                evidence_record_ids: recovery.provenance.evidence_record_ids.clone(),
            }))
        }
        _ => Err(LeyCoreError::InvalidSessionStore(
            "learning evidence checkpoint ID resolved to a non-checkpoint event".to_owned(),
        )),
    }
}

pub(crate) fn read_session_for_memory_compiler(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    session_id: &str,
) -> Result<(AgentSession, Option<u64>), LeyCoreError> {
    validate_session_id(session_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
    let Some(store) = SessionStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Err(LeyCoreError::SessionNotFound(session_id.to_owned()));
    };
    let _lock = store.lock(true)?;
    let session_dir = store.open_session(session_id)?;
    let events = store.read_events(session_id, &session_dir)?;
    let latest_checkpoint_sequence = events.iter().rev().find_map(|event| {
        matches!(
            &event.payload,
            SessionEventPayload::CheckpointRecorded(_)
                | SessionEventPayload::RecoveryCheckpointRecorded(_)
        )
        .then_some(event.sequence)
    });
    let session = replay_events(&events, &diagnostic.identity.project_id, session_id)?;
    if session.checkpoints.is_empty() != latest_checkpoint_sequence.is_none() {
        return Err(LeyCoreError::InvalidSessionStore(
            "checkpoint event boundary did not match the replayed session".to_owned(),
        ));
    }
    Ok((session, latest_checkpoint_sequence))
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
    validate_project_memory(&diagnostic.root, &vault)?;
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
    validate_project_memory(&diagnostic.root, &vault)?;
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
            source_kind: session.source.kind,
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
    let verification_evidence_artifacts = input
        .verification
        .iter()
        .map(|verification| verification.evidence_artifact_paths.len())
        .sum::<usize>();
    if input.plan.len() > 100
        || input.decisions.len() > 100
        || input.tasks.len() > 100
        || input.problems.len() > 50
        || input.commands.len() > 200
        || input.verification.len() > 200
        || input.touched_artifacts.len() > 200
        || verification_evidence_artifacts > 200
        || input
            .verification
            .iter()
            .any(|verification| verification.evidence_artifact_paths.len() > 20)
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
    let touched_artifacts = artifact_citations_for_paths(
        memory,
        input.touched_artifacts.into_iter().collect(),
        "touched artifact",
    )?;
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
        let evidence_artifacts = artifact_citations_for_paths(
            memory,
            item.evidence_artifact_paths,
            "verification evidence artifact",
        )?;
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
            evidence_artifacts,
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

fn artifact_citations_for_paths(
    memory: &crate::ingestion::LoadedProjectMemory,
    paths: Vec<String>,
    description: &str,
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
                    "{description} is not in the current approved snapshot: {path}"
                ))
            })?;
        citations.push(SessionArtifactCitation {
            artifact_path: artifact.path.clone(),
            artifact_snapshot_id: memory.manifest.snapshot_id.clone(),
            content_hash: artifact.content_hash.clone(),
            media_type: artifact.media_type,
            start_line: if artifact.media_type.is_some() { 0 } else { 1 },
            end_line: if artifact.media_type.is_some() {
                0
            } else {
                artifact.line_count.max(1)
            },
        });
    }
    citations.sort_by(|left, right| left.artifact_path.cmp(&right.artifact_path));
    Ok(citations)
}

fn normalize_turn_evidence(
    input: TurnEvidenceInput,
    event_id: &str,
    capture_mode: crate::CaptureMode,
    source_recorded_at_unix_ms: Option<u64>,
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
    match input.origin {
        TurnEvidenceOrigin::Import => {
            if host.is_none() {
                return Err(LeyCoreError::InvalidSessionRequest(
                    "imported turn evidence must identify its supported host".to_owned(),
                ));
            }
            if source_recorded_at_unix_ms.is_none_or(|value| value == 0) {
                return Err(LeyCoreError::InvalidSessionRequest(
                    "imported turn evidence must include a positive source timestamp".to_owned(),
                ));
            }
        }
        TurnEvidenceOrigin::HostHook | TurnEvidenceOrigin::ManualCli => {
            if source_recorded_at_unix_ms.is_some() {
                return Err(LeyCoreError::InvalidSessionRequest(
                    "only imported turn evidence may carry a source timestamp".to_owned(),
                ));
            }
        }
    }
    if capture_mode == crate::CaptureMode::Minimal {
        return Ok((
            SessionTurnEvidence {
                record_id,
                event_id: event_id.to_owned(),
                sequence: 0,
                recorded_at_unix_ms,
                source_recorded_at_unix_ms,
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
            source_recorded_at_unix_ms,
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

fn replay_pending_event_if_present(
    project_id: &str,
    session_id: &str,
    mut pending: PendingEvent,
    vault: impl AsRef<Path>,
) -> Result<Option<SessionMutation>, LeyCoreError> {
    let Some(store) = SessionStore::open(&vault, project_id, false)? else {
        return Err(LeyCoreError::SessionNotFound(session_id.to_owned()));
    };
    let _lock = store.lock(false)?;
    let session_dir = store.open_session(session_id)?;
    let existing = store.read_events(session_id, &session_dir)?;
    let Some(event) = existing
        .iter()
        .find(|event| event.event_id == pending.event_id)
    else {
        if existing
            .iter()
            .any(|event| event.request_id == pending.request_id)
        {
            return Err(LeyCoreError::SessionIdempotencyConflict(pending.request_id));
        }
        return Ok(None);
    };
    align_turn_evidence_retry(&mut pending, &event.payload);
    let request_fingerprint = request_fingerprint(
        project_id,
        session_id,
        &pending.request_id,
        &pending.payload,
    )?;
    if event.request_fingerprint != request_fingerprint
        || !retry_payload_matches(&event.payload, &pending.payload)
    {
        return Err(LeyCoreError::SessionIdempotencyConflict(pending.request_id));
    }
    let session = store.rebuild_session_from_dir(session_id, &session_dir)?;
    store.persist_projection(&session_dir, &session)?;
    Ok(Some(mutation(session, &pending.event_id, true)))
}

fn validate_pending_recovery_window(
    session_id: &str,
    schema_version: u32,
    payload: &SessionEventPayload,
    existing: &[SessionEvent],
) -> Result<(), LeyCoreError> {
    let SessionEventPayload::RecoveryCheckpointRecorded(recovery) = payload else {
        return Ok(());
    };
    if recovery.provenance.expected_event_count != existing.len() as u64 {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery checkpoint event count changed; reverify before saving".to_owned(),
        ));
    }
    let mut recovery_window = Vec::new();
    for event in existing {
        match &event.payload {
            SessionEventPayload::CheckpointRecorded(_)
            | SessionEventPayload::RecoveryCheckpointRecorded(_) => recovery_window.clear(),
            SessionEventPayload::UserPromptObserved(evidence)
            | SessionEventPayload::AssistantResponseObserved(evidence) => {
                recovery_window.push(evidence.record_id.clone());
            }
            _ => {}
        }
    }
    recovery_window.sort();
    if recovery_window != recovery.provenance.evidence_record_ids {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery evidence window changed; recompile and reverify before saving".to_owned(),
        ));
    }
    let normalized_candidate_fingerprint = match schema_version {
        SESSION_RECOVERY_SCHEMA_VERSION => {
            crate::memory_transition::unresolved_candidate_fingerprint(
                session_id,
                recovery.provenance.expected_event_count,
                &recovery.checkpoint.summary,
                recovery.checkpoint.unresolved.first().ok_or_else(|| {
                    LeyCoreError::InvalidSessionRequest(
                        "bound unresolved recovery checkpoint lost its unresolved claim".to_owned(),
                    )
                })?,
                &recovery.provenance.evidence_record_ids,
            )
        }
        SESSION_TYPED_RECOVERY_SCHEMA_VERSION => {
            let (kind, subject, statement) = typed_recovery_checkpoint_claim(&recovery.checkpoint)
                .ok_or_else(|| {
                    LeyCoreError::InvalidSessionRequest(
                        "typed recovery checkpoint changed under normalization".to_owned(),
                    )
                })?;
            crate::memory_transition::recovery_candidate_fingerprint(
                session_id,
                recovery.provenance.expected_event_count,
                recovery_memory_candidate_kind(kind),
                subject,
                statement,
                &recovery.provenance.evidence_record_ids,
            )
        }
        SESSION_TASK_RECOVERY_SCHEMA_VERSION => {
            let (title, status, details) = task_recovery_checkpoint_claim(&recovery.checkpoint)
                .ok_or_else(|| {
                    LeyCoreError::InvalidSessionRequest(
                        "task recovery checkpoint changed under normalization".to_owned(),
                    )
                })?;
            crate::memory_transition::task_candidate_fingerprint(
                session_id,
                recovery.provenance.expected_event_count,
                title,
                status,
                details,
                &recovery.provenance.evidence_record_ids,
            )
        }
        SESSION_PLAN_RECOVERY_SCHEMA_VERSION => {
            let (text, status) =
                plan_recovery_checkpoint_claim(&recovery.checkpoint).ok_or_else(|| {
                    LeyCoreError::InvalidSessionRequest(
                        "plan recovery checkpoint changed under normalization".to_owned(),
                    )
                })?;
            crate::memory_transition::plan_candidate_fingerprint(
                session_id,
                recovery.provenance.expected_event_count,
                text,
                status,
                &recovery.provenance.evidence_record_ids,
            )
        }
        _ => {
            return Err(LeyCoreError::InvalidSessionRequest(
                "recovery checkpoint uses an unsupported schema version".to_owned(),
            ))
        }
    };
    if normalized_candidate_fingerprint != recovery.provenance.candidate_fingerprint {
        return Err(LeyCoreError::InvalidSessionRequest(
            "recovery candidate changed under checkpoint normalization; recompile and reverify the sanitized content"
                .to_owned(),
        ));
    }
    Ok(())
}

fn resolve_pending_context_utility(
    payload: &mut SessionEventPayload,
    existing: &[SessionEvent],
) -> Result<(), LeyCoreError> {
    let SessionEventPayload::ContextUtilityObserved(observation) = payload else {
        return Ok(());
    };
    let (binding_event, binding) = existing
        .iter()
        .find_map(|event| match &event.payload {
            SessionEventPayload::ContextUtilityBound(binding)
                if binding.id == observation.binding_id =>
            {
                Some((event, binding))
            }
            _ => None,
        })
        .ok_or_else(|| {
            LeyCoreError::InvalidSessionRequest(
                "context utility binding is missing from this session".to_owned(),
            )
        })?;
    let mut outcomes = Vec::with_capacity(observation.downstream_event_ids.len());
    for event_id in &observation.downstream_event_ids {
        let event = existing
            .iter()
            .find(|event| &event.event_id == event_id)
            .ok_or_else(|| {
                LeyCoreError::InvalidSessionRequest(format!(
                    "context utility downstream event is missing from this session: {event_id}"
                ))
            })?;
        if event.sequence <= binding_event.sequence {
            return Err(LeyCoreError::InvalidSessionRequest(format!(
                "context utility event {event_id} does not occur after its bound context pack"
            )));
        }
        let outcome = context_utility_outcome_from_event(event).ok_or_else(|| {
            LeyCoreError::InvalidSessionRequest(format!(
                "context utility event {event_id} is not a checkpoint or session finish outcome"
            ))
        })?;
        if !context_utility_has_outcome_signal(&outcome) {
            return Err(LeyCoreError::InvalidSessionRequest(format!(
                "context utility event {event_id} contains no typed downstream outcome signal"
            )));
        }
        outcomes.push(outcome);
    }
    observation.context_pack_id = binding.context_pack_id.clone();
    observation.downstream_outcomes = outcomes;
    Ok(())
}

fn context_utility_outcome_from_event(
    event: &SessionEvent,
) -> Option<ContextUtilityOutcomeEvidence> {
    match &event.payload {
        SessionEventPayload::CheckpointRecorded(checkpoint) => {
            Some(context_utility_checkpoint_outcome(event, checkpoint))
        }
        SessionEventPayload::RecoveryCheckpointRecorded(recovery) => Some(
            context_utility_checkpoint_outcome(event, recovery.checkpoint.as_ref()),
        ),
        SessionEventPayload::SessionFinished(finish) => Some(ContextUtilityOutcomeEvidence {
            event_id: event.event_id.clone(),
            recorded_at_unix_ms: event.recorded_at_unix_ms,
            kind: ContextUtilityOutcomeKind::SessionFinish,
            completed_tasks: 0,
            blocked_tasks: 0,
            cancelled_tasks: 0,
            resolved_problems: 0,
            helped_attempts: 0,
            no_effect_attempts: 0,
            worsened_attempts: 0,
            unknown_attempts: 0,
            passed_verifications: 0,
            failed_verifications: 0,
            skipped_verifications: 0,
            unknown_verifications: 0,
            session_status: Some(finish.status),
            unresolved_count: finish.unresolved.len(),
        }),
        _ => None,
    }
}

fn context_utility_checkpoint_outcome(
    event: &SessionEvent,
    checkpoint: &SessionCheckpoint,
) -> ContextUtilityOutcomeEvidence {
    let mut outcome = ContextUtilityOutcomeEvidence {
        event_id: event.event_id.clone(),
        recorded_at_unix_ms: event.recorded_at_unix_ms,
        kind: ContextUtilityOutcomeKind::Checkpoint,
        completed_tasks: 0,
        blocked_tasks: 0,
        cancelled_tasks: 0,
        resolved_problems: 0,
        helped_attempts: 0,
        no_effect_attempts: 0,
        worsened_attempts: 0,
        unknown_attempts: 0,
        passed_verifications: 0,
        failed_verifications: 0,
        skipped_verifications: 0,
        unknown_verifications: 0,
        session_status: None,
        unresolved_count: checkpoint.unresolved.len(),
    };
    for task in &checkpoint.tasks {
        match task.status {
            TaskStatus::Completed => outcome.completed_tasks += 1,
            TaskStatus::Blocked => outcome.blocked_tasks += 1,
            TaskStatus::Cancelled => outcome.cancelled_tasks += 1,
            TaskStatus::Pending | TaskStatus::InProgress => {}
        }
    }
    for problem in &checkpoint.problems {
        if problem.resolution.is_some() {
            outcome.resolved_problems += 1;
        }
        for attempt in &problem.attempts {
            match attempt.outcome {
                AttemptOutcome::Helped => outcome.helped_attempts += 1,
                AttemptOutcome::NoEffect => outcome.no_effect_attempts += 1,
                AttemptOutcome::Worsened => outcome.worsened_attempts += 1,
                AttemptOutcome::Unknown => outcome.unknown_attempts += 1,
            }
        }
    }
    for verification in &checkpoint.verification {
        match verification.status {
            VerificationStatus::Passed => outcome.passed_verifications += 1,
            VerificationStatus::Failed => outcome.failed_verifications += 1,
            VerificationStatus::Skipped => outcome.skipped_verifications += 1,
            VerificationStatus::Unknown => outcome.unknown_verifications += 1,
        }
    }
    outcome
}

fn context_utility_has_outcome_signal(outcome: &ContextUtilityOutcomeEvidence) -> bool {
    outcome.session_status.is_some()
        || outcome.completed_tasks > 0
        || outcome.blocked_tasks > 0
        || outcome.cancelled_tasks > 0
        || outcome.resolved_problems > 0
        || outcome.helped_attempts > 0
        || outcome.no_effect_attempts > 0
        || outcome.worsened_attempts > 0
        || outcome.unknown_attempts > 0
        || outcome.passed_verifications > 0
        || outcome.failed_verifications > 0
        || outcome.skipped_verifications > 0
        || outcome.unknown_verifications > 0
        || outcome.unresolved_count > 0
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
        align_context_utility_retry(&mut pending, &event.payload);
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
    resolve_pending_context_utility(&mut pending.payload, &existing)?;
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
    validate_pending_recovery_window(
        session_id,
        pending.schema_version,
        &pending.payload,
        &existing,
    )?;
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
            && !matches!(
                &pending.payload,
                SessionEventPayload::SessionRenamed(_)
                    | SessionEventPayload::ContextUtilityObserved(_)
            )
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

fn align_context_utility_retry(pending: &mut PendingEvent, stored: &SessionEventPayload) {
    match (&mut pending.payload, stored) {
        (
            SessionEventPayload::ContextUtilityBound(pending),
            SessionEventPayload::ContextUtilityBound(stored),
        ) => {
            pending.recorded_at_unix_ms = stored.recorded_at_unix_ms;
        }
        (
            SessionEventPayload::ContextUtilityObserved(pending),
            SessionEventPayload::ContextUtilityObserved(stored),
        ) => {
            pending.recorded_at_unix_ms = stored.recorded_at_unix_ms;
            pending.context_pack_id = stored.context_pack_id.clone();
            pending.downstream_outcomes = stored.downstream_outcomes.clone();
        }
        _ => {}
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
                && stored.source_recorded_at_unix_ms == pending.source_recorded_at_unix_ms
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
        SessionEventPayload::RecoveryCheckpointRecorded(recovery) => {
            recovery.checkpoint.recorded_at_unix_ms =
                recovery.checkpoint.recorded_at_unix_ms.max(minimum);
            recovery.checkpoint.recorded_at_unix_ms
        }
        SessionEventPayload::SessionFinished(finish) => {
            finish.recorded_at_unix_ms = finish.recorded_at_unix_ms.max(minimum);
            finish.recorded_at_unix_ms
        }
        SessionEventPayload::ContextUtilityBound(binding) => {
            binding.recorded_at_unix_ms = binding.recorded_at_unix_ms.max(minimum);
            binding.recorded_at_unix_ms
        }
        SessionEventPayload::ContextUtilityObserved(observation) => {
            observation.recorded_at_unix_ms = observation.recorded_at_unix_ms.max(minimum);
            observation.recorded_at_unix_ms
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
    if session.schema_version >= SESSION_PLAN_RECOVERY_SCHEMA_VERSION {
        SESSION_V10_FILE
    } else if session.schema_version >= SESSION_TASK_RECOVERY_SCHEMA_VERSION {
        SESSION_V9_FILE
    } else if session.schema_version >= SESSION_TYPED_RECOVERY_SCHEMA_VERSION {
        SESSION_V8_FILE
    } else if session.schema_version >= SESSION_IMPORTED_TURN_SCHEMA_VERSION {
        SESSION_V7_FILE
    } else if session.schema_version >= SESSION_MULTIMODAL_EVIDENCE_SCHEMA_VERSION {
        SESSION_V6_FILE
    } else if session.schema_version >= SESSION_CONTEXT_UTILITY_SCHEMA_VERSION {
        SESSION_V5_FILE
    } else if session.schema_version >= SESSION_VERIFICATION_EVIDENCE_SCHEMA_VERSION {
        SESSION_V4_FILE
    } else if session.schema_version >= SESSION_RECOVERY_SCHEMA_VERSION {
        SESSION_V3_FILE
    } else if session.schema_version >= SESSION_SCHEMA_VERSION {
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
        schema_version: events
            .iter()
            .map(|event| event.schema_version)
            .max()
            .unwrap_or(SESSION_V1_SCHEMA_VERSION),
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
        context_utility_bindings: Vec::new(),
        context_utility_observations: Vec::new(),
        renames: Vec::new(),
        finish: None,
    };
    let mut recovery_window = Vec::new();
    for (offset, event) in events[1..].iter().enumerate() {
        let event_index = offset + 1;
        if session.status != SessionStatus::Active
            && !matches!(
                &event.payload,
                SessionEventPayload::SessionRenamed(_)
                    | SessionEventPayload::ContextUtilityObserved(_)
            )
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
                recovery_window.clear();
            }
            SessionEventPayload::RecoveryCheckpointRecorded(recovery) => {
                validate_recovery_checkpoint_history(event, recovery, &recovery_window)?;
                session
                    .checkpoints
                    .push(recovery.checkpoint.as_ref().clone());
                recovery_window.clear();
            }
            SessionEventPayload::SessionFinished(finish) => {
                session.status = finish.status;
                session.finished_at_unix_ms = Some(finish.recorded_at_unix_ms);
                session.finish = Some(finish.clone());
            }
            SessionEventPayload::ContextUtilityBound(binding) => {
                validate_context_utility_binding_history(event, binding)?;
                session.context_utility_bindings.push(binding.clone());
            }
            SessionEventPayload::ContextUtilityObserved(observation) => {
                validate_context_utility_history(event, observation, &events[..event_index])?;
                session
                    .context_utility_observations
                    .push(observation.clone());
            }
            SessionEventPayload::SessionRenamed(rename) => {
                session.name = rename.name.clone();
                session.renames.push(rename.clone());
            }
            SessionEventPayload::UserPromptObserved(evidence) => {
                recovery_window.push(evidence.record_id.clone());
                session.prompts.push(evidence.clone());
            }
            SessionEventPayload::AssistantResponseObserved(evidence) => {
                recovery_window.push(evidence.record_id.clone());
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

fn validate_recovery_checkpoint_history(
    event: &SessionEvent,
    recovery: &RecoveryCheckpointEvent,
    recovery_window: &[String],
) -> Result<(), LeyCoreError> {
    if recovery.provenance.expected_event_count != event.sequence.saturating_sub(1) {
        return invalid_session_store(
            "recovery checkpoint expected event count does not match its sequence",
        );
    }
    let mut expected_evidence = recovery_window.to_vec();
    expected_evidence.sort();
    if expected_evidence != recovery.provenance.evidence_record_ids {
        return invalid_session_store(
            "recovery checkpoint evidence does not match the complete post-checkpoint window",
        );
    }
    let expected_binding = recovery_binding_fingerprint_for_event(event, recovery)?;
    if expected_binding != recovery.provenance.binding_fingerprint {
        return invalid_session_store("recovery checkpoint binding fingerprint is invalid");
    }
    Ok(())
}

fn validate_context_utility_history(
    event: &SessionEvent,
    observation: &ContextUtilityObservation,
    previous_events: &[SessionEvent],
) -> Result<(), LeyCoreError> {
    if observation.expected_event_count != event.sequence.saturating_sub(1) {
        return invalid_session_store(
            "context utility expected event count does not match its sequence",
        );
    }
    let (binding_event, binding) = previous_events
        .iter()
        .find_map(|candidate| match &candidate.payload {
            SessionEventPayload::ContextUtilityBound(binding)
                if binding.id == observation.binding_id =>
            {
                Some((candidate, binding))
            }
            _ => None,
        })
        .ok_or_else(|| {
            LeyCoreError::InvalidSessionStore(
                "context utility binding is missing from prior session history".to_owned(),
            )
        })?;
    if observation.context_pack_id != binding.context_pack_id {
        return invalid_session_store(
            "context utility observation context pack does not match its binding",
        );
    }
    let mut expected = Vec::with_capacity(observation.downstream_event_ids.len());
    for event_id in &observation.downstream_event_ids {
        let downstream = previous_events
            .iter()
            .find(|candidate| &candidate.event_id == event_id)
            .ok_or_else(|| {
                LeyCoreError::InvalidSessionStore(
                    "context utility downstream event is missing from prior session history"
                        .to_owned(),
                )
            })?;
        if downstream.sequence <= binding_event.sequence {
            return invalid_session_store(
                "context utility downstream event does not follow its bound context pack",
            );
        }
        let outcome = context_utility_outcome_from_event(downstream).ok_or_else(|| {
            LeyCoreError::InvalidSessionStore(
                "context utility cites a non-outcome session event".to_owned(),
            )
        })?;
        if !context_utility_has_outcome_signal(&outcome) {
            return invalid_session_store(
                "context utility cites a checkpoint without typed downstream outcome evidence",
            );
        }
        expected.push(outcome);
    }
    if expected != observation.downstream_outcomes {
        return invalid_session_store(
            "context utility outcome evidence does not match cited session events",
        );
    }
    Ok(())
}

fn validate_context_utility_binding_history(
    event: &SessionEvent,
    binding: &ContextUtilityBinding,
) -> Result<(), LeyCoreError> {
    if binding.expected_event_count != event.sequence.saturating_sub(1) {
        return invalid_session_store(
            "context utility binding expected event count does not match its sequence",
        );
    }
    Ok(())
}

fn validate_event(
    event: &SessionEvent,
    project_id: &str,
    session_id: &str,
) -> Result<(), LeyCoreError> {
    if !matches!(
        event.schema_version,
        SESSION_V1_SCHEMA_VERSION
            | SESSION_SCHEMA_VERSION
            | SESSION_RECOVERY_SCHEMA_VERSION
            | SESSION_VERIFICATION_EVIDENCE_SCHEMA_VERSION
            | SESSION_CONTEXT_UTILITY_SCHEMA_VERSION
            | SESSION_MULTIMODAL_EVIDENCE_SCHEMA_VERSION
            | SESSION_IMPORTED_TURN_SCHEMA_VERSION
            | SESSION_TYPED_RECOVERY_SCHEMA_VERSION
            | SESSION_TASK_RECOVERY_SCHEMA_VERSION
            | SESSION_PLAN_RECOVERY_SCHEMA_VERSION
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
        SessionEventPayload::RecoveryCheckpointRecorded(_) => "recovery-checkpoint-recorded",
        SessionEventPayload::SessionFinished(_) => "session-finished",
        SessionEventPayload::ContextUtilityBound(_) => "context-utility-bound",
        SessionEventPayload::ContextUtilityObserved(_) => "context-utility-observed",
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
            if let Some(source_reference) = &source.source_reference {
                validate_stored_text("source.sourceReference", source_reference, 1, 80)?;
            }
            match source.kind {
                SessionSourceKind::Import => {
                    if let Some(source_reference) = source.source_reference.as_deref() {
                        if source.host.as_deref() != Some("codex")
                            || source.agent.is_some()
                            || !valid_prefixed_hex(source_reference, "hsi_", 64)
                        {
                            return invalid_session_store(
                                "imported session source provenance is invalid",
                            );
                        }
                    }
                }
                SessionSourceKind::ManualCli
                | SessionSourceKind::HostHook
                | SessionSourceKind::Mcp => {
                    if source.source_reference.is_some() {
                        return invalid_session_store(
                            "non-imported session cannot carry historical source provenance",
                        );
                    }
                }
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
        SessionEventPayload::RecoveryCheckpointRecorded(recovery) => {
            validate_recovery_checkpoint_event(event, recovery)?;
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
        SessionEventPayload::ContextUtilityBound(binding) => {
            validate_context_utility_binding(event, binding)?;
        }
        SessionEventPayload::ContextUtilityObserved(observation) => {
            validate_context_utility_observation(event, observation)?;
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
    let is_imported_turn_event = matches!(
        &event.payload,
        SessionEventPayload::UserPromptObserved(evidence)
            | SessionEventPayload::AssistantResponseObserved(evidence)
            if evidence.origin == TurnEvidenceOrigin::Import
    );
    let is_recovery_checkpoint = matches!(
        event.payload,
        SessionEventPayload::RecoveryCheckpointRecorded(_)
    );
    let is_verification_evidence_checkpoint = matches!(
        &event.payload,
        SessionEventPayload::CheckpointRecorded(checkpoint)
            if checkpoint
                .verification
                .iter()
                .any(|verification| !verification.evidence_artifacts.is_empty())
    );
    let is_multimodal_evidence_checkpoint = matches!(
        &event.payload,
        SessionEventPayload::CheckpointRecorded(checkpoint)
            if checkpoint
                .touched_artifacts
                .iter()
                .chain(
                    checkpoint
                        .verification
                        .iter()
                        .flat_map(|verification| verification.evidence_artifacts.iter())
                )
                .any(|citation| citation.media_type.is_some())
    );
    let is_context_utility = matches!(
        event.payload,
        SessionEventPayload::ContextUtilityBound(_)
            | SessionEventPayload::ContextUtilityObserved(_)
    );
    if event.schema_version == SESSION_V1_SCHEMA_VERSION
        && (is_turn_event
            || is_recovery_checkpoint
            || is_verification_evidence_checkpoint
            || is_context_utility
            || is_multimodal_evidence_checkpoint)
    {
        return invalid_session_store(
            "schema version 1 cannot store turn evidence, bound recovery checkpoints, verification evidence links, or context utility observations",
        );
    }
    if event.schema_version == SESSION_SCHEMA_VERSION && (!is_turn_event || is_imported_turn_event)
    {
        return invalid_session_store(
            "schema version 2 is reserved for non-imported turn evidence",
        );
    }
    if event.schema_version == SESSION_RECOVERY_SCHEMA_VERSION && !is_recovery_checkpoint {
        return invalid_session_store(
            "schema version 3 is reserved for bound recovery checkpoints",
        );
    }
    if event.schema_version == SESSION_TYPED_RECOVERY_SCHEMA_VERSION && !is_recovery_checkpoint {
        return invalid_session_store(
            "schema version 8 is reserved for typed bound recovery checkpoints",
        );
    }
    if event.schema_version == SESSION_TASK_RECOVERY_SCHEMA_VERSION && !is_recovery_checkpoint {
        return invalid_session_store(
            "schema version 9 is reserved for bound task recovery checkpoints",
        );
    }
    if event.schema_version == SESSION_PLAN_RECOVERY_SCHEMA_VERSION && !is_recovery_checkpoint {
        return invalid_session_store(
            "schema version 10 is reserved for bound plan recovery checkpoints",
        );
    }
    if event.schema_version == SESSION_VERIFICATION_EVIDENCE_SCHEMA_VERSION
        && !is_verification_evidence_checkpoint
    {
        return invalid_session_store(
            "schema version 4 is reserved for checkpoints with verification evidence links",
        );
    }
    if event.schema_version == SESSION_CONTEXT_UTILITY_SCHEMA_VERSION && !is_context_utility {
        return invalid_session_store(
            "schema version 5 is reserved for context utility bindings and observations",
        );
    }
    if is_multimodal_evidence_checkpoint
        && event.schema_version != SESSION_MULTIMODAL_EVIDENCE_SCHEMA_VERSION
    {
        return invalid_session_store(
            "multimodal artifact citations require session event schema version 6",
        );
    }
    if event.schema_version == SESSION_MULTIMODAL_EVIDENCE_SCHEMA_VERSION
        && !is_multimodal_evidence_checkpoint
    {
        return invalid_session_store(
            "schema version 6 is reserved for checkpoints with multimodal artifact citations",
        );
    }
    if is_imported_turn_event && event.schema_version != SESSION_IMPORTED_TURN_SCHEMA_VERSION {
        return invalid_session_store(
            "imported turn evidence requires session event schema version 7",
        );
    }
    if event.schema_version == SESSION_IMPORTED_TURN_SCHEMA_VERSION && !is_imported_turn_event {
        return invalid_session_store("schema version 7 is reserved for imported turn evidence");
    }
    Ok(())
}

fn validate_context_utility_binding(
    event: &SessionEvent,
    binding: &ContextUtilityBinding,
) -> Result<(), LeyCoreError> {
    if binding.event_id != event.event_id
        || binding.id != child_id("cub", &event.event_id, 0)
        || binding.recorded_at_unix_ms != event.recorded_at_unix_ms
        || binding.expected_event_count == 0
    {
        return invalid_session_store("context utility binding identity is invalid");
    }
    if !valid_prefixed_hex(&binding.context_pack_id, "cpk_", 64)
        || !valid_prefixed_hex(&binding.artifact_snapshot_id, "snp_", 64)
        || !valid_prefixed_hex(&binding.graph_snapshot_id, "grf_", 64)
    {
        return invalid_session_store("context utility pack identity is invalid");
    }
    validate_stored_text("contextUtility.taskExcerpt", &binding.task_excerpt, 1, 256)?;
    if !(1..=20).contains(&binding.max_results)
        || !(500..=8_000).contains(&binding.max_tokens)
        || binding.estimated_tokens > binding.max_tokens
    {
        return invalid_session_store("context utility compiler limits are invalid");
    }
    if binding.included_records.len() > SESSION_CONTEXT_UTILITY_INCLUDED_RECORD_LIMIT
        || binding
            .included_records
            .windows(2)
            .any(|window| window[0] >= window[1])
    {
        return invalid_session_store(
            "context utility included records must be bounded, sorted, and unique",
        );
    }
    for record in &binding.included_records {
        validate_stored_text("contextUtility.record.entityId", &record.entity_id, 1, 256)?;
        if let Some(kind) = &record.kind {
            validate_stored_text("contextUtility.record.kind", kind, 1, 64)?;
        }
        if let Some(session_id) = &record.session_id {
            validate_session_id(session_id)?;
        }
        if let Some(learning_id) = &record.learning_id {
            if !valid_prefixed_hex(learning_id, "lrn_", 32) {
                return invalid_session_store("context utility learning ID is invalid");
            }
        }
        if let Some(specification_id) = &record.specification_id {
            if !valid_prefixed_hex(specification_id, "spec_", 32) {
                return invalid_session_store("context utility specification ID is invalid");
            }
        }
        if let Some(mount_id) = &record.mount_id {
            if !valid_prefixed_hex(mount_id, "mnt_", 32) {
                return invalid_session_store("context utility mount ID is invalid");
            }
        }
        if let Some(source_project_id) = &record.source_project_id {
            if !valid_prefixed_hex(source_project_id, "prj_", 32) {
                return invalid_session_store("context utility source project ID is invalid");
            }
        }
        match record.source {
            ContextUtilityRecordSource::Specification => {
                if record.specification_id.as_deref() != Some(record.entity_id.as_str())
                    || record.session_id.is_some()
                    || record.learning_id.is_some()
                    || record.mount_id.is_some()
                    || record.source_project_id.is_some()
                {
                    return invalid_session_store(
                        "context utility specification record shape is invalid",
                    );
                }
            }
            ContextUtilityRecordSource::ActiveProjectMemory => {
                if record.specification_id.is_some()
                    || record.mount_id.is_some()
                    || record.source_project_id.is_some()
                {
                    return invalid_session_store(
                        "context utility active-project record shape is invalid",
                    );
                }
            }
            ContextUtilityRecordSource::MountedReference => {
                if record.specification_id.is_some()
                    || record.mount_id.is_none()
                    || record.source_project_id.is_none()
                {
                    return invalid_session_store(
                        "context utility mounted-reference record shape is invalid",
                    );
                }
            }
        }
    }
    if !binding.context_pack_revalidated || binding.context_usage_proven {
        return invalid_session_store(
            "context utility binding must remain revalidated but usage-unproven",
        );
    }
    Ok(())
}

fn validate_context_utility_observation(
    event: &SessionEvent,
    observation: &ContextUtilityObservation,
) -> Result<(), LeyCoreError> {
    if observation.event_id != event.event_id
        || observation.id != child_id("cut", &event.event_id, 0)
        || observation.recorded_at_unix_ms != event.recorded_at_unix_ms
        || observation.expected_event_count == 0
        || !valid_prefixed_hex(&observation.binding_id, "cub_", 32)
        || !valid_prefixed_hex(&observation.context_pack_id, "cpk_", 64)
    {
        return invalid_session_store("context utility observation identity is invalid");
    }
    if observation.downstream_event_ids.is_empty()
        || observation.downstream_event_ids.len() > SESSION_CONTEXT_UTILITY_OUTCOME_LIMIT
        || observation
            .downstream_event_ids
            .windows(2)
            .any(|window| window[0] >= window[1])
        || observation.downstream_outcomes.len() != observation.downstream_event_ids.len()
    {
        return invalid_session_store("context utility downstream outcome set is invalid");
    }
    for (event_id, outcome) in observation
        .downstream_event_ids
        .iter()
        .zip(&observation.downstream_outcomes)
    {
        validate_event_id(event_id)?;
        if &outcome.event_id != event_id || !context_utility_has_outcome_signal(outcome) {
            return invalid_session_store("context utility outcome evidence is invalid");
        }
        match outcome.kind {
            ContextUtilityOutcomeKind::Checkpoint if outcome.session_status.is_some() => {
                return invalid_session_store(
                    "checkpoint utility outcome cannot have session status",
                )
            }
            ContextUtilityOutcomeKind::SessionFinish => {
                if outcome.session_status.is_none() {
                    return invalid_session_store(
                        "session-finish utility outcome must include terminal status",
                    );
                }
                if matches!(outcome.session_status, Some(SessionStatus::Active)) {
                    return invalid_session_store(
                        "session-finish utility outcome cannot be active",
                    );
                }
            }
            _ => {}
        }
    }
    if observation.context_usage_proven
        || observation.causal_utility_proven
        || observation.trust_changes_applied
        || observation.ranking_changes_applied
    {
        return invalid_session_store(
            "context utility observation must remain correlation-only and non-authoritative",
        );
    }
    Ok(())
}

fn validate_recovery_checkpoint_event(
    event: &SessionEvent,
    recovery: &RecoveryCheckpointEvent,
) -> Result<(), LeyCoreError> {
    let checkpoint = recovery.checkpoint.as_ref();
    if checkpoint.event_id != event.event_id
        || checkpoint.recorded_at_unix_ms != event.recorded_at_unix_ms
        || checkpoint.id != child_id("ckp", &event.event_id, 0)
    {
        return invalid_session_store("recovery checkpoint identity is invalid");
    }
    validate_stored_text("checkpoint.summary", &checkpoint.summary, 1, 16_000)?;
    validate_checkpoint_records(checkpoint, &event.event_id)?;
    match event.schema_version {
        SESSION_RECOVERY_SCHEMA_VERSION => {
            if !checkpoint.plan.is_empty()
                || !checkpoint.decisions.is_empty()
                || !checkpoint.tasks.is_empty()
                || !checkpoint.problems.is_empty()
                || !checkpoint.touched_artifacts.is_empty()
                || !checkpoint.commands.is_empty()
                || !checkpoint.verification.is_empty()
                || checkpoint.unresolved.len() != 1
            {
                return invalid_session_store(
                    "schema-v3 bound recovery checkpoint must contain exactly one unresolved claim and no other typed records",
                );
            }
        }
        SESSION_TYPED_RECOVERY_SCHEMA_VERSION => {
            if typed_recovery_checkpoint_claim(checkpoint).is_none() {
                return invalid_session_store(
                    "schema-v8 bound recovery checkpoint must contain exactly one minimal decision or problem claim",
                );
            }
        }
        SESSION_TASK_RECOVERY_SCHEMA_VERSION => {
            if task_recovery_checkpoint_claim(checkpoint).is_none() {
                return invalid_session_store(
                    "schema-v9 bound recovery checkpoint must contain exactly one task claim",
                );
            }
        }
        SESSION_PLAN_RECOVERY_SCHEMA_VERSION => {
            if plan_recovery_checkpoint_claim(checkpoint).is_none() {
                return invalid_session_store(
                    "schema-v10 bound recovery checkpoint must contain exactly one plan claim",
                );
            }
        }
        _ => return invalid_session_store("bound recovery checkpoint schema version is invalid"),
    }
    if recovery.provenance.expected_event_count != event.sequence.saturating_sub(1) {
        return invalid_session_store(
            "recovery checkpoint expected event count does not match its sequence",
        );
    }
    if !is_sha256(&recovery.provenance.candidate_fingerprint)
        || !is_sha256(&recovery.provenance.binding_fingerprint)
    {
        return invalid_session_store("recovery checkpoint fingerprint is invalid");
    }
    let evidence = &recovery.provenance.evidence_record_ids;
    if evidence.is_empty() || evidence.len() > SESSION_RECOVERY_BINDING_EVIDENCE_LIMIT {
        return invalid_session_store("recovery checkpoint evidence set is invalid");
    }
    if evidence
        .iter()
        .any(|record_id| !valid_prefixed_hex(record_id, "tev_", 32))
        || evidence.windows(2).any(|window| window[0] >= window[1])
    {
        return invalid_session_store(
            "recovery checkpoint evidence IDs must be sorted unique tev_ identifiers",
        );
    }
    let expected_candidate = match event.schema_version {
        SESSION_RECOVERY_SCHEMA_VERSION => {
            crate::memory_transition::unresolved_candidate_fingerprint(
                &event.session_id,
                recovery.provenance.expected_event_count,
                &checkpoint.summary,
                &checkpoint.unresolved[0],
                evidence,
            )
        }
        SESSION_TYPED_RECOVERY_SCHEMA_VERSION => {
            let (kind, subject, statement) = typed_recovery_checkpoint_claim(checkpoint)
                .ok_or_else(|| {
                    LeyCoreError::InvalidSessionStore(
                        "typed recovery checkpoint claim shape is invalid".to_owned(),
                    )
                })?;
            crate::memory_transition::recovery_candidate_fingerprint(
                &event.session_id,
                recovery.provenance.expected_event_count,
                recovery_memory_candidate_kind(kind),
                subject,
                statement,
                evidence,
            )
        }
        SESSION_TASK_RECOVERY_SCHEMA_VERSION => {
            let (title, status, details) =
                task_recovery_checkpoint_claim(checkpoint).ok_or_else(|| {
                    LeyCoreError::InvalidSessionStore(
                        "task recovery checkpoint claim shape is invalid".to_owned(),
                    )
                })?;
            crate::memory_transition::task_candidate_fingerprint(
                &event.session_id,
                recovery.provenance.expected_event_count,
                title,
                status,
                details,
                evidence,
            )
        }
        SESSION_PLAN_RECOVERY_SCHEMA_VERSION => {
            let (text, status) = plan_recovery_checkpoint_claim(checkpoint).ok_or_else(|| {
                LeyCoreError::InvalidSessionStore(
                    "plan recovery checkpoint claim shape is invalid".to_owned(),
                )
            })?;
            crate::memory_transition::plan_candidate_fingerprint(
                &event.session_id,
                recovery.provenance.expected_event_count,
                text,
                status,
                evidence,
            )
        }
        _ => unreachable!("recovery schema checked above"),
    };
    if recovery.provenance.candidate_fingerprint != expected_candidate {
        return invalid_session_store(
            "recovery checkpoint candidate fingerprint does not match its bound claim",
        );
    }
    let expected_binding = recovery_binding_fingerprint_for_event(event, recovery)?;
    if recovery.provenance.binding_fingerprint != expected_binding {
        return invalid_session_store("recovery checkpoint binding fingerprint is invalid");
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
    match evidence.origin {
        TurnEvidenceOrigin::Import => {
            if evidence.host.as_deref() != Some("codex")
                || evidence
                    .source_recorded_at_unix_ms
                    .is_none_or(|value| value == 0)
            {
                return invalid_session_store("imported turn provenance is invalid");
            }
        }
        TurnEvidenceOrigin::HostHook | TurnEvidenceOrigin::ManualCli => {
            if evidence.source_recorded_at_unix_ms.is_some() {
                return invalid_session_store(
                    "non-imported turn evidence cannot carry a source timestamp",
                );
            }
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
    let mut verification_evidence_artifacts = 0usize;
    for (index, item) in checkpoint.verification.iter().enumerate() {
        validate_child_id(&item.id, "ver", event_id, index)?;
        validate_stored_text("verification.kind", &item.kind, 1, 64)?;
        validate_stored_text("verification.summary", &item.summary, 1, 8_000)?;
        if let Some(command) = &item.command {
            validate_stored_text("verification.command", command, 1, 8_000)?;
        }
        if item.evidence_artifacts.len() > 20 {
            return invalid_session_store(
                "a verification record contains too many evidence artifact citations",
            );
        }
        verification_evidence_artifacts =
            verification_evidence_artifacts.saturating_add(item.evidence_artifacts.len());
        if verification_evidence_artifacts > 200 {
            return invalid_session_store(
                "checkpoint contains too many verification evidence artifact citations",
            );
        }
        let mut evidence_paths = BTreeSet::new();
        for citation in &item.evidence_artifacts {
            validate_artifact_citation(citation)?;
            if !evidence_paths.insert(citation.artifact_path.as_str()) {
                return invalid_session_store(
                    "verification record has duplicate evidence artifact citations",
                );
            }
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
    let valid_range = if citation.media_type.is_some() {
        citation.start_line == 0 && citation.end_line == 0
    } else {
        citation.start_line > 0 && citation.end_line >= citation.start_line
    };
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
        || !valid_range
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
    let host = source
        .host
        .map(|value| sanitize_text("source.host", &value, 1, 128, redactions))
        .transpose()?;
    let agent = source
        .agent
        .map(|value| sanitize_text("source.agent", &value, 1, 128, redactions))
        .transpose()?;
    let source_reference = source
        .source_reference
        .map(|value| sanitize_text("source.sourceReference", &value, 1, 80, redactions))
        .transpose()?;
    match source.kind {
        SessionSourceKind::Import => {
            if host.as_deref() != Some("codex") {
                return Err(LeyCoreError::InvalidSessionRequest(
                    "historical session import must identify the supported codex host".to_owned(),
                ));
            }
            if agent.is_some() {
                return Err(LeyCoreError::InvalidSessionRequest(
                    "codex message-history import cannot claim an agent/model identity".to_owned(),
                ));
            }
            if source_reference
                .as_deref()
                .is_none_or(|value| !valid_prefixed_hex(value, "hsi_", 64))
            {
                return Err(LeyCoreError::InvalidSessionRequest(
                    "historical session import requires an opaque hsi_ source reference".to_owned(),
                ));
            }
        }
        SessionSourceKind::ManualCli | SessionSourceKind::HostHook | SessionSourceKind::Mcp => {
            if source_reference.is_some() {
                return Err(LeyCoreError::InvalidSessionRequest(
                    "only imported sessions may carry a historical source reference".to_owned(),
                ));
            }
        }
    }
    Ok(SessionSource {
        kind: source.kind,
        host,
        agent,
        source_reference,
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

fn stabilize_checkpoint_for_fingerprint(checkpoint: &mut SessionCheckpoint) {
    checkpoint.recorded_at_unix_ms = 0;
    checkpoint.project_revision = None;
    for citation in &mut checkpoint.touched_artifacts {
        citation.artifact_snapshot_id.clear();
        citation.content_hash.clear();
        citation.start_line = 0;
        citation.end_line = 0;
    }
}

fn recovery_binding_fingerprint(
    session_id: &str,
    expected_event_count: u64,
    candidate_fingerprint: &str,
    evidence_record_ids: &[String],
    summary: &str,
    unresolved: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-recovery-checkpoint-binding-v1");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(expected_event_count.to_le_bytes());
    hasher.update([0]);
    hasher.update(candidate_fingerprint.as_bytes());
    hasher.update([0]);
    hasher.update(summary.as_bytes());
    hasher.update([0]);
    hasher.update(unresolved.as_bytes());
    for record_id in evidence_record_ids {
        hasher.update([0]);
        hasher.update(record_id.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn recovery_binding_fingerprint_v2(
    session_id: &str,
    expected_event_count: u64,
    candidate_fingerprint: &str,
    evidence_record_ids: &[String],
    kind: RecoveredStructuredKind,
    subject: &str,
    statement: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-recovery-checkpoint-binding-v2");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(expected_event_count.to_le_bytes());
    hasher.update([0]);
    hasher.update(candidate_fingerprint.as_bytes());
    hasher.update([0]);
    hasher.update(match kind {
        RecoveredStructuredKind::Decision => b"decision".as_slice(),
        RecoveredStructuredKind::Problem => b"problem".as_slice(),
    });
    hasher.update([0]);
    hasher.update(subject.as_bytes());
    hasher.update([0]);
    hasher.update(statement.as_bytes());
    for record_id in evidence_record_ids {
        hasher.update([0]);
        hasher.update(record_id.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn recovery_binding_fingerprint_v3(
    session_id: &str,
    expected_event_count: u64,
    candidate_fingerprint: &str,
    evidence_record_ids: &[String],
    title: &str,
    status: TaskStatus,
    details: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-recovery-checkpoint-binding-v3");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(expected_event_count.to_le_bytes());
    hasher.update([0]);
    hasher.update(candidate_fingerprint.as_bytes());
    hasher.update([0]);
    hasher.update(b"task");
    hasher.update([0]);
    hasher.update(title.as_bytes());
    hasher.update([0]);
    hasher.update(enum_label(status).as_bytes());
    hasher.update([0]);
    hasher.update(details.as_bytes());
    for record_id in evidence_record_ids {
        hasher.update([0]);
        hasher.update(record_id.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn recovery_binding_fingerprint_v4(
    session_id: &str,
    expected_event_count: u64,
    candidate_fingerprint: &str,
    evidence_record_ids: &[String],
    text: &str,
    status: PlanStatus,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-recovery-checkpoint-binding-v4");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(expected_event_count.to_le_bytes());
    hasher.update([0]);
    hasher.update(candidate_fingerprint.as_bytes());
    hasher.update([0]);
    hasher.update(b"plan");
    hasher.update([0]);
    hasher.update(text.as_bytes());
    hasher.update([0]);
    hasher.update(enum_label(status).as_bytes());
    for record_id in evidence_record_ids {
        hasher.update([0]);
        hasher.update(record_id.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn typed_recovery_checkpoint_claim(
    checkpoint: &SessionCheckpoint,
) -> Option<(RecoveredStructuredKind, &str, &str)> {
    let no_other_records = checkpoint.plan.is_empty()
        && checkpoint.tasks.is_empty()
        && checkpoint.touched_artifacts.is_empty()
        && checkpoint.commands.is_empty()
        && checkpoint.verification.is_empty()
        && checkpoint.unresolved.is_empty();
    if !no_other_records {
        return None;
    }
    if checkpoint.decisions.len() == 1 && checkpoint.problems.is_empty() {
        let decision = &checkpoint.decisions[0];
        if checkpoint.summary == decision.title
            && decision.rationale.is_empty()
            && decision.alternatives.is_empty()
        {
            return Some((
                RecoveredStructuredKind::Decision,
                decision.title.as_str(),
                decision.decision.as_str(),
            ));
        }
    }
    if checkpoint.problems.len() == 1 && checkpoint.decisions.is_empty() {
        let problem = &checkpoint.problems[0];
        if checkpoint.summary == problem.title
            && problem.expected.is_empty()
            && problem.attempts.is_empty()
            && problem.resolution.is_none()
        {
            return Some((
                RecoveredStructuredKind::Problem,
                problem.title.as_str(),
                problem.symptom.as_str(),
            ));
        }
    }
    None
}

fn task_recovery_checkpoint_claim(
    checkpoint: &SessionCheckpoint,
) -> Option<(&str, TaskStatus, &str)> {
    if !checkpoint.plan.is_empty()
        || !checkpoint.decisions.is_empty()
        || checkpoint.tasks.len() != 1
        || !checkpoint.problems.is_empty()
        || !checkpoint.touched_artifacts.is_empty()
        || !checkpoint.commands.is_empty()
        || !checkpoint.verification.is_empty()
        || !checkpoint.unresolved.is_empty()
    {
        return None;
    }
    let task = &checkpoint.tasks[0];
    if checkpoint.summary != task.title {
        return None;
    }
    Some((task.title.as_str(), task.status, task.details.as_str()))
}

fn plan_recovery_checkpoint_claim(checkpoint: &SessionCheckpoint) -> Option<(&str, PlanStatus)> {
    if checkpoint.plan.len() != 1
        || !checkpoint.decisions.is_empty()
        || !checkpoint.tasks.is_empty()
        || !checkpoint.problems.is_empty()
        || !checkpoint.touched_artifacts.is_empty()
        || !checkpoint.commands.is_empty()
        || !checkpoint.verification.is_empty()
        || !checkpoint.unresolved.is_empty()
    {
        return None;
    }
    let plan = &checkpoint.plan[0];
    if checkpoint.summary != plan.text {
        return None;
    }
    Some((plan.text.as_str(), plan.status))
}

fn recovery_memory_candidate_kind(
    kind: RecoveredStructuredKind,
) -> crate::memory_transition::MemoryCandidateKind {
    match kind {
        RecoveredStructuredKind::Decision => {
            crate::memory_transition::MemoryCandidateKind::Decision
        }
        RecoveredStructuredKind::Problem => crate::memory_transition::MemoryCandidateKind::Problem,
    }
}

fn recovery_binding_fingerprint_for_event(
    event: &SessionEvent,
    recovery: &RecoveryCheckpointEvent,
) -> Result<String, LeyCoreError> {
    match event.schema_version {
        SESSION_RECOVERY_SCHEMA_VERSION => Ok(recovery_binding_fingerprint(
            &event.session_id,
            recovery.provenance.expected_event_count,
            &recovery.provenance.candidate_fingerprint,
            &recovery.provenance.evidence_record_ids,
            &recovery.checkpoint.summary,
            recovery.checkpoint.unresolved.first().ok_or_else(|| {
                LeyCoreError::InvalidSessionStore(
                    "schema-v3 recovery checkpoint has no unresolved claim".to_owned(),
                )
            })?,
        )),
        SESSION_TYPED_RECOVERY_SCHEMA_VERSION => {
            let (kind, subject, statement) = typed_recovery_checkpoint_claim(&recovery.checkpoint)
                .ok_or_else(|| {
                    LeyCoreError::InvalidSessionStore(
                        "schema-v8 recovery checkpoint claim shape is invalid".to_owned(),
                    )
                })?;
            Ok(recovery_binding_fingerprint_v2(
                &event.session_id,
                recovery.provenance.expected_event_count,
                &recovery.provenance.candidate_fingerprint,
                &recovery.provenance.evidence_record_ids,
                kind,
                subject,
                statement,
            ))
        }
        SESSION_TASK_RECOVERY_SCHEMA_VERSION => {
            let (title, status, details) = task_recovery_checkpoint_claim(&recovery.checkpoint)
                .ok_or_else(|| {
                    LeyCoreError::InvalidSessionStore(
                        "schema-v9 recovery checkpoint task shape is invalid".to_owned(),
                    )
                })?;
            Ok(recovery_binding_fingerprint_v3(
                &event.session_id,
                recovery.provenance.expected_event_count,
                &recovery.provenance.candidate_fingerprint,
                &recovery.provenance.evidence_record_ids,
                title,
                status,
                details,
            ))
        }
        SESSION_PLAN_RECOVERY_SCHEMA_VERSION => {
            let (text, status) =
                plan_recovery_checkpoint_claim(&recovery.checkpoint).ok_or_else(|| {
                    LeyCoreError::InvalidSessionStore(
                        "schema-v10 recovery checkpoint plan shape is invalid".to_owned(),
                    )
                })?;
            Ok(recovery_binding_fingerprint_v4(
                &event.session_id,
                recovery.provenance.expected_event_count,
                &recovery.provenance.candidate_fingerprint,
                &recovery.provenance.evidence_record_ids,
                text,
                status,
            ))
        }
        _ => invalid_session_store("recovery checkpoint schema version is invalid"),
    }
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
            stabilize_checkpoint_for_fingerprint(&mut checkpoint);
            SessionEventPayload::CheckpointRecorded(checkpoint)
        }
        SessionEventPayload::RecoveryCheckpointRecorded(recovery) => {
            let mut recovery = recovery.clone();
            stabilize_checkpoint_for_fingerprint(&mut recovery.checkpoint);
            SessionEventPayload::RecoveryCheckpointRecorded(recovery)
        }
        SessionEventPayload::SessionFinished(finish) => {
            let mut finish = finish.clone();
            finish.recorded_at_unix_ms = 0;
            SessionEventPayload::SessionFinished(finish)
        }
        SessionEventPayload::ContextUtilityBound(binding) => {
            let mut binding = binding.clone();
            binding.recorded_at_unix_ms = 0;
            SessionEventPayload::ContextUtilityBound(binding)
        }
        SessionEventPayload::ContextUtilityObserved(observation) => {
            let mut observation = observation.clone();
            observation.recorded_at_unix_ms = 0;
            SessionEventPayload::ContextUtilityObserved(observation)
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
    if let Some(source_reference) = &session.source.source_reference {
        output.push_str(&format!(
            "- Historical source reference: `{source_reference}`\n"
        ));
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
                if let Some(command) = &verification.command {
                    output.push_str(&format!("  - command: `{}`\n", markdown_inline(command)));
                }
                for citation in &verification.evidence_artifacts {
                    if let Some(media_type) = citation.media_type {
                        output.push_str(&format!(
                            "  - evidence: `{}` · snapshot `{}` · `{}` · original media `{}`\n",
                            markdown_inline(&citation.artifact_path),
                            markdown_inline(&citation.artifact_snapshot_id),
                            markdown_inline(&citation.content_hash),
                            media_type.mime_type()
                        ));
                    } else {
                        output.push_str(&format!(
                            "  - evidence: `{}` · snapshot `{}` · `{}` · lines {}–{}\n",
                            markdown_inline(&citation.artifact_path),
                            markdown_inline(&citation.artifact_snapshot_id),
                            markdown_inline(&citation.content_hash),
                            citation.start_line,
                            citation.end_line
                        ));
                    }
                }
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
    if let Some(source_recorded_at_unix_ms) = evidence.source_recorded_at_unix_ms {
        output.push_str(&format!(
            "- Source recorded at: `{source_recorded_at_unix_ms}`\n"
        ));
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
    Import => "import",
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
    use crate::{
        compile_project_context_with_registries, ingest_project, initialize_project, CaptureMode,
        ContextCompileLimits, ContextMountRegistry, SpecificationRegistry,
    };
    use std::process::Command;
    use std::sync::{Arc, Barrier};
    use std::time::Duration;
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

    fn png_fixture(marker: u8) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes.extend_from_slice(&[0, 0, 0, 13]);
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, marker]);
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(b"IEND");
        bytes.extend_from_slice(&[0xae, 0x42, 0x60, 0x82]);
        bytes
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

    fn recovered_unresolved_input(
        request_id: String,
        expected_event_count: u64,
        candidate_fingerprint: String,
        evidence_record_ids: Vec<String>,
        summary: &str,
        unresolved: &str,
    ) -> RecoveredUnresolvedCheckpointInput {
        RecoveredUnresolvedCheckpointInput {
            request_id,
            expected_event_count,
            candidate_fingerprint,
            evidence_record_ids,
            summary: summary.to_owned(),
            unresolved: unresolved.to_owned(),
        }
    }

    fn recovered_structured_input(
        request_id: String,
        expected_event_count: u64,
        candidate_fingerprint: String,
        evidence_record_ids: Vec<String>,
        kind: RecoveredStructuredKind,
        subject: &str,
        statement: &str,
    ) -> RecoveredStructuredCheckpointInput {
        RecoveredStructuredCheckpointInput {
            request_id,
            expected_event_count,
            candidate_fingerprint,
            evidence_record_ids,
            kind,
            subject: subject.to_owned(),
            statement: statement.to_owned(),
        }
    }

    fn recovered_task_input(
        request_id: String,
        expected_event_count: u64,
        candidate_fingerprint: String,
        evidence_record_ids: Vec<String>,
        title: &str,
        status: TaskStatus,
        details: &str,
    ) -> RecoveredTaskCheckpointInput {
        RecoveredTaskCheckpointInput {
            request_id,
            expected_event_count,
            candidate_fingerprint,
            evidence_record_ids,
            title: title.to_owned(),
            status,
            details: details.to_owned(),
        }
    }

    fn recovered_plan_input(
        request_id: String,
        expected_event_count: u64,
        candidate_fingerprint: String,
        evidence_record_ids: Vec<String>,
        text: &str,
        status: PlanStatus,
    ) -> RecoveredPlanCheckpointInput {
        RecoveredPlanCheckpointInput {
            request_id,
            expected_event_count,
            candidate_fingerprint,
            evidence_record_ids,
            text: text.to_owned(),
            status,
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
                source_reference: None,
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
                evidence_artifact_paths: Vec::new(),
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

    fn compile_test_pack(
        base: &tempfile::TempDir,
        project: &Path,
        vault: &Path,
        task: &str,
    ) -> CompiledContextPack {
        let specifications =
            SpecificationRegistry::at(base.path().join("utility-specifications-v1.json"));
        let mounts = ContextMountRegistry::at(base.path().join("utility-context-mounts-v1.json"));
        compile_project_context_with_registries(
            project,
            vault,
            task,
            ContextCompileLimits {
                max_results: 8,
                max_tokens: 1_500,
            },
            &specifications,
            &mounts,
        )
        .unwrap()
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
    fn verification_evidence_is_snapshot_bound_and_upgrades_projection() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('e'))).unwrap();
        let directory = session_directory(&project, &vault, &started.session.session_id);
        let mut input = checkpoint_input(request_id('f'), "Linked verification evidence");
        input.verification[0].evidence_artifact_paths = vec!["README.md".to_owned()];

        let mutation =
            checkpoint_session(&project, &vault, &started.session.session_id, input).unwrap();
        let checkpoint = mutation.session.checkpoints.last().unwrap();
        let revision = checkpoint.project_revision.as_ref().unwrap();
        let evidence = &checkpoint.verification[0].evidence_artifacts;

        assert_eq!(
            mutation.session.schema_version,
            SESSION_VERIFICATION_EVIDENCE_SCHEMA_VERSION
        );
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].artifact_path, "README.md");
        assert_eq!(
            evidence[0].artifact_snapshot_id,
            revision.artifact_snapshot_id
        );
        assert!(evidence[0].content_hash.starts_with("sha256:"));
        assert_eq!(evidence[0].start_line, 1);
        assert!(evidence[0].end_line >= evidence[0].start_line);
        assert!(directory.join(SESSION_V4_FILE).exists());
        assert!(mutation.session_path.ends_with(SESSION_V4_FILE));

        let replayed = read_session(&project, &vault, &started.session.session_id).unwrap();
        assert_eq!(
            replayed.checkpoints[0].verification[0].evidence_artifacts,
            evidence.as_slice()
        );
    }

    #[test]
    fn multimodal_verification_evidence_uses_v6_and_non_text_citations() {
        let (_base, project, vault) = setup_memory_with_mode(CaptureMode::FullEvidence);
        let image = png_fixture(0);
        std::fs::write(project.join("verification.png"), &image).unwrap();
        ingest_project(&project, &vault).unwrap();

        let started = start_session(&project, &vault, start_input(request_id('6'))).unwrap();
        let directory = session_directory(&project, &vault, &started.session.session_id);
        let mut input = checkpoint_input(request_id('7'), "Visual verification evidence");
        input.verification[0].evidence_artifact_paths = vec!["verification.png".to_owned()];

        let mutation =
            checkpoint_session(&project, &vault, &started.session.session_id, input).unwrap();
        assert_eq!(
            mutation.session.schema_version,
            SESSION_MULTIMODAL_EVIDENCE_SCHEMA_VERSION
        );
        assert!(directory.join(SESSION_V6_FILE).exists());
        assert!(mutation.session_path.ends_with(SESSION_V6_FILE));

        let verification = &mutation.session.checkpoints[0].verification[0];
        let citation = &verification.evidence_artifacts[0];
        assert_eq!(citation.artifact_path, "verification.png");
        assert_eq!(citation.media_type, Some(ArtifactMediaType::Png));
        assert_eq!(citation.start_line, 0);
        assert_eq!(citation.end_line, 0);

        let media = crate::read_verification_media_evidence(
            &project,
            &vault,
            &started.session.session_id,
            &verification.id,
            "verification.png",
            image.len(),
        )
        .unwrap();
        assert_eq!(media.data, image);
        assert_eq!(media.evidence_role, "original-media");
        assert!(!media.derived_description_included);
        assert!(!media.live_source_checked);

        let replayed = read_session(&project, &vault, &started.session.session_id).unwrap();
        assert_eq!(
            replayed.checkpoints[0].verification[0].evidence_artifacts,
            verification.evidence_artifacts
        );
        let markdown = std::fs::read_to_string(directory.join(SESSION_MARKDOWN_FILE)).unwrap();
        assert!(markdown.contains("original media `image/png`"));
        assert!(!markdown.contains("lines 0–0"));
    }

    #[test]
    fn verification_evidence_rejects_paths_outside_the_captured_snapshot() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('a'))).unwrap();
        let mut input = checkpoint_input(request_id('b'), "Reject missing evidence");
        input.verification[0].evidence_artifact_paths = vec!["runtime/test.log".to_owned()];

        let error =
            checkpoint_session(&project, &vault, &started.session.session_id, input).unwrap_err();
        assert!(matches!(
            error,
            LeyCoreError::InvalidSessionRequest(message)
                if message.contains("verification evidence artifact is not in the current approved snapshot")
        ));
        let replayed = read_session(&project, &vault, &started.session.session_id).unwrap();
        assert_eq!(replayed.event_count, 1);
        assert!(replayed.checkpoints.is_empty());
    }

    #[test]
    fn context_utility_binds_exact_pack_to_checkpoint_and_terminal_outcomes() {
        let (base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('1'))).unwrap();
        let session_id = started.session.session_id.clone();
        let pack = compile_test_pack(
            &base,
            &project,
            &vault,
            "durable checkpoint target structured memory",
        );
        let binding_input = ContextUtilityBindingInput {
            request_id: request_id('2'),
            expected_event_count: 1,
            expected_context_pack_id: pack.context_pack_id.clone(),
            task: pack.task.clone(),
            max_results: 8,
            max_tokens: 1_500,
        };
        let bound =
            bind_context_utility_pack(&project, &vault, &session_id, binding_input.clone(), &pack)
                .unwrap();
        assert_eq!(bound.session.event_count, 2);
        assert_eq!(bound.session.context_utility_bindings.len(), 1);
        assert_eq!(
            bound.session.schema_version,
            SESSION_CONTEXT_UTILITY_SCHEMA_VERSION
        );
        let binding = &bound.session.context_utility_bindings[0];
        assert_eq!(binding.context_pack_id, pack.context_pack_id);
        assert_eq!(binding.expected_event_count, 1);
        assert!(binding.context_pack_revalidated);
        assert!(!binding.context_usage_proven);
        assert!(!binding.included_records.is_empty());
        let binding_id = binding.id.clone();

        std::thread::sleep(Duration::from_millis(2));
        let bound_retry =
            bind_context_utility_pack(&project, &vault, &session_id, binding_input.clone(), &pack)
                .unwrap();
        assert!(bound_retry.replayed);
        assert_eq!(bound_retry.event_id, bound.event_id);
        assert_eq!(bound_retry.session.event_count, 2);

        let mut changed_task_retry = binding_input.clone();
        changed_task_retry.task = "different utility task".to_owned();
        let conflict = replay_context_utility_binding_if_present(
            &project,
            &vault,
            &session_id,
            &changed_task_retry,
        )
        .unwrap_err();
        assert!(matches!(
            conflict,
            LeyCoreError::SessionIdempotencyConflict(request_id)
                if request_id == binding_input.request_id
        ));

        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &session_id,
            checkpoint_input(request_id('3'), "Utility downstream checkpoint"),
        )
        .unwrap();
        let finished =
            finish_session(&project, &vault, &session_id, finish_input(request_id('4'))).unwrap();
        assert_eq!(finished.session.status, SessionStatus::Completed);
        assert_eq!(finished.session.event_count, 4);

        let input = ContextUtilityObservationInput {
            request_id: request_id('5'),
            expected_event_count: finished.session.event_count,
            binding_id: binding_id.clone(),
            downstream_event_ids: vec![checkpoint.event_id.clone(), finished.event_id.clone()],
        };
        let observed =
            record_context_utility_observation(&project, &vault, &session_id, input.clone())
                .unwrap();

        assert_eq!(
            observed.session.schema_version,
            SESSION_CONTEXT_UTILITY_SCHEMA_VERSION
        );
        assert_eq!(observed.session.status, SessionStatus::Completed);
        assert_eq!(observed.session.event_count, 5);
        assert_eq!(observed.session.context_utility_bindings.len(), 1);
        assert_eq!(observed.session.context_utility_observations.len(), 1);
        assert!(observed.session_path.ends_with(SESSION_V5_FILE));
        let utility = &observed.session.context_utility_observations[0];
        assert_eq!(utility.context_pack_id, pack.context_pack_id);
        assert_eq!(utility.binding_id, binding_id);
        assert_eq!(utility.expected_event_count, 4);
        assert!(!utility.context_usage_proven);
        assert!(!utility.causal_utility_proven);
        assert!(!utility.trust_changes_applied);
        assert!(!utility.ranking_changes_applied);
        assert_eq!(utility.downstream_outcomes.len(), 2);
        let checkpoint_outcome = utility
            .downstream_outcomes
            .iter()
            .find(|outcome| outcome.kind == ContextUtilityOutcomeKind::Checkpoint)
            .unwrap();
        assert_eq!(checkpoint_outcome.completed_tasks, 1);
        assert_eq!(checkpoint_outcome.resolved_problems, 1);
        assert_eq!(checkpoint_outcome.helped_attempts, 1);
        assert_eq!(checkpoint_outcome.passed_verifications, 1);
        assert_eq!(checkpoint_outcome.unresolved_count, 1);
        let finish_outcome = utility
            .downstream_outcomes
            .iter()
            .find(|outcome| outcome.kind == ContextUtilityOutcomeKind::SessionFinish)
            .unwrap();
        assert_eq!(
            finish_outcome.session_status,
            Some(SessionStatus::Completed)
        );
        assert_eq!(finish_outcome.unresolved_count, 1);

        let replayed =
            record_context_utility_observation(&project, &vault, &session_id, input).unwrap();
        assert!(replayed.replayed);
        assert_eq!(replayed.event_id, observed.event_id);
        assert_eq!(replayed.session.event_count, 5);

        let rebuilt = read_session(&project, &vault, &session_id).unwrap();
        assert_eq!(
            rebuilt.context_utility_bindings,
            observed.session.context_utility_bindings
        );
        assert_eq!(
            rebuilt.context_utility_observations,
            observed.session.context_utility_observations
        );
    }

    #[test]
    fn context_utility_rejects_unbound_non_downstream_or_non_outcome_evidence() {
        let (base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('5'))).unwrap();
        let session_id = started.session.session_id.clone();
        let pack = compile_test_pack(&base, &project, &vault, "session memory");

        let forged = bind_context_utility_pack(
            &project,
            &vault,
            &session_id,
            ContextUtilityBindingInput {
                request_id: request_id('6'),
                expected_event_count: 1,
                expected_context_pack_id: format!("cpk_{}", "0".repeat(64)),
                task: pack.task.clone(),
                max_results: 8,
                max_tokens: 1_500,
            },
            &pack,
        )
        .unwrap_err();
        assert!(matches!(
            forged,
            LeyCoreError::InvalidSessionRequest(message)
                if message.contains("context pack changed")
        ));

        let missing_binding = record_context_utility_observation(
            &project,
            &vault,
            &session_id,
            ContextUtilityObservationInput {
                request_id: request_id('7'),
                expected_event_count: 1,
                binding_id: format!("cub_{}", "0".repeat(32)),
                downstream_event_ids: vec![started.event_id.clone()],
            },
        )
        .unwrap_err();
        assert!(matches!(
            missing_binding,
            LeyCoreError::InvalidSessionRequest(message)
                if message.contains("binding is missing")
        ));

        let bound = bind_context_utility_pack(
            &project,
            &vault,
            &session_id,
            ContextUtilityBindingInput {
                request_id: request_id('8'),
                expected_event_count: 1,
                expected_context_pack_id: pack.context_pack_id.clone(),
                task: pack.task.clone(),
                max_results: 8,
                max_tokens: 1_500,
            },
            &pack,
        )
        .unwrap();
        let binding_id = bound.session.context_utility_bindings[0].id.clone();

        let predates = record_context_utility_observation(
            &project,
            &vault,
            &session_id,
            ContextUtilityObservationInput {
                request_id: request_id('9'),
                expected_event_count: 2,
                binding_id: binding_id.clone(),
                downstream_event_ids: vec![started.event_id],
            },
        )
        .unwrap_err();
        assert!(matches!(
            predates,
            LeyCoreError::InvalidSessionRequest(message)
                if message.contains("does not occur after its bound context pack")
        ));

        let renamed = rename_session(
            &project,
            &vault,
            &session_id,
            RenameSessionInput {
                request_id: request_id('a'),
                expected_event_count: Some(2),
                name: "Renamed utility session".to_owned(),
                note: "Create a post-binding non-outcome event".to_owned(),
            },
        )
        .unwrap();
        let non_outcome = record_context_utility_observation(
            &project,
            &vault,
            &session_id,
            ContextUtilityObservationInput {
                request_id: request_id('b'),
                expected_event_count: 3,
                binding_id,
                downstream_event_ids: vec![renamed.event_id],
            },
        )
        .unwrap_err();
        assert!(matches!(
            non_outcome,
            LeyCoreError::InvalidSessionRequest(message)
                if message.contains("not a checkpoint or session finish outcome")
        ));
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
    fn bound_recovery_checkpoint_is_provenance_preserving_and_idempotent() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('a'))).unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('b'), "Investigate the retry loop"),
        )
        .unwrap();
        let prompt_id = prompt.session.prompts[0].record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('c'), "No durable conclusion yet"),
        )
        .unwrap();
        let response_id = response.session.responses[0].record_id.clone();
        let candidate_fingerprint = crate::memory_transition::unresolved_candidate_fingerprint(
            &started.session.session_id,
            3,
            "Retry investigation",
            "The retry investigation remains open",
            &[prompt_id.clone(), response_id.clone()],
        );
        let recovery_request = request_id('d');

        let committed = checkpoint_recovered_unresolved_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_unresolved_input(
                recovery_request.clone(),
                3,
                candidate_fingerprint.clone(),
                vec![response_id.clone(), prompt_id.clone()],
                "Retry investigation",
                "The retry investigation remains open",
            ),
        )
        .unwrap();

        assert_eq!(
            committed.session.schema_version,
            SESSION_RECOVERY_SCHEMA_VERSION
        );
        assert_eq!(committed.session.event_count, 4);
        assert_eq!(committed.session.checkpoints.len(), 1);
        assert_eq!(
            committed.session.checkpoints[0].unresolved,
            vec!["The retry investigation remains open"]
        );
        let directory = session_directory(&project, &vault, &started.session.session_id);
        assert!(directory.join(SESSION_V3_FILE).exists());
        assert!(committed.session_path.ends_with(SESSION_V3_FILE));

        let replayed = checkpoint_recovered_unresolved_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_unresolved_input(
                recovery_request.clone(),
                3,
                candidate_fingerprint.clone(),
                vec![prompt_id.clone(), response_id.clone()],
                "Retry investigation",
                "The retry investigation remains open",
            ),
        )
        .unwrap();
        assert!(replayed.replayed);
        assert_eq!(replayed.session.event_count, 4);

        assert!(matches!(
            checkpoint_recovered_unresolved_session(
                &project,
                &vault,
                &started.session.session_id,
                recovered_unresolved_input(
                    recovery_request,
                    3,
                    format!("sha256:{}", "b".repeat(64)),
                    vec![prompt_id, response_id],
                    "Retry investigation",
                    "The retry investigation remains open",
                ),
            ),
            Err(LeyCoreError::SessionIdempotencyConflict(_))
        ));
    }

    #[test]
    fn bound_recovery_checkpoint_rejects_stale_or_incomplete_window_before_append() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('1'))).unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('2'), "Investigate the retry loop"),
        )
        .unwrap();
        let prompt_id = prompt.session.prompts[0].record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('3'), "No durable conclusion yet"),
        )
        .unwrap();
        let response_id = response.session.responses[0].record_id.clone();

        assert!(matches!(
            checkpoint_recovered_unresolved_session(
                &project,
                &vault,
                &started.session.session_id,
                recovered_unresolved_input(
                    request_id('4'),
                    2,
                    format!("sha256:{}", "c".repeat(64)),
                    vec![prompt_id.clone()],
                    "Retry investigation",
                    "The retry investigation remains open",
                ),
            ),
            Err(LeyCoreError::InvalidSessionRequest(message))
                if message.contains("reload before saving")
        ));
        assert!(matches!(
            checkpoint_recovered_unresolved_session(
                &project,
                &vault,
                &started.session.session_id,
                recovered_unresolved_input(
                    request_id('5'),
                    3,
                    format!("sha256:{}", "d".repeat(64)),
                    vec![response_id.clone()],
                    "Retry investigation",
                    "The retry investigation remains open",
                ),
            ),
            Err(LeyCoreError::InvalidSessionRequest(message))
                if message.contains("recovery evidence window changed")
        ));
        let secret_statement = "The retry investigation remains open with token=secret-value";
        let secret_fingerprint = crate::memory_transition::unresolved_candidate_fingerprint(
            &started.session.session_id,
            3,
            "Retry investigation",
            secret_statement,
            &[prompt_id.clone(), response_id.clone()],
        );
        assert!(matches!(
            checkpoint_recovered_unresolved_session(
                &project,
                &vault,
                &started.session.session_id,
                recovered_unresolved_input(
                    request_id('6'),
                    3,
                    secret_fingerprint,
                    vec![prompt_id, response_id],
                    "Retry investigation",
                    secret_statement,
                ),
            ),
            Err(LeyCoreError::InvalidSessionRequest(message))
                if message.contains("changed under checkpoint normalization")
        ));
        assert_eq!(
            read_session(&project, &vault, &started.session.session_id)
                .unwrap()
                .event_count,
            3
        );
        assert_eq!(
            std::fs::read_dir(
                session_directory(&project, &vault, &started.session.session_id)
                    .join(EVENTS_DIRECTORY)
            )
            .unwrap()
            .count(),
            3
        );
    }

    #[test]
    fn bound_recovery_checkpoint_replay_rejects_incomplete_or_rebound_provenance() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('5'))).unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('6'), "Investigate the retry loop"),
        )
        .unwrap();
        let prompt_id = prompt.session.prompts[0].record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('7'), "No durable conclusion yet"),
        )
        .unwrap();
        let response_id = response.session.responses[0].record_id.clone();
        let candidate_fingerprint = crate::memory_transition::unresolved_candidate_fingerprint(
            &started.session.session_id,
            3,
            "Retry investigation",
            "The retry investigation remains open",
            &[prompt_id.clone(), response_id.clone()],
        );
        let committed = checkpoint_recovered_unresolved_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_unresolved_input(
                request_id('8'),
                3,
                candidate_fingerprint,
                vec![prompt_id, response_id],
                "Retry investigation",
                "The retry investigation remains open",
            ),
        )
        .unwrap();
        let event_path = session_directory(&project, &vault, &started.session.session_id)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", committed.event_id));
        let mut event: SessionEvent =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        let SessionEventPayload::RecoveryCheckpointRecorded(recovery) = &mut event.payload else {
            panic!("expected recovery checkpoint event");
        };
        recovery.provenance.evidence_record_ids.pop();
        recovery.provenance.candidate_fingerprint =
            crate::memory_transition::unresolved_candidate_fingerprint(
                &event.session_id,
                recovery.provenance.expected_event_count,
                &recovery.checkpoint.summary,
                &recovery.checkpoint.unresolved[0],
                &recovery.provenance.evidence_record_ids,
            );
        recovery.provenance.binding_fingerprint = recovery_binding_fingerprint(
            &event.session_id,
            recovery.provenance.expected_event_count,
            &recovery.provenance.candidate_fingerprint,
            &recovery.provenance.evidence_record_ids,
            &recovery.checkpoint.summary,
            &recovery.checkpoint.unresolved[0],
        );
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.session_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(message))
                if message.contains("complete post-checkpoint window")
        ));

        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('9'))).unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('a'), "Investigate the retry loop"),
        )
        .unwrap();
        let prompt_id = prompt.session.prompts[0].record_id.clone();
        let candidate_fingerprint = crate::memory_transition::unresolved_candidate_fingerprint(
            &started.session.session_id,
            2,
            "Retry investigation",
            "The retry investigation remains open",
            std::slice::from_ref(&prompt_id),
        );
        let committed = checkpoint_recovered_unresolved_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_unresolved_input(
                request_id('b'),
                2,
                candidate_fingerprint,
                vec![prompt_id],
                "Retry investigation",
                "The retry investigation remains open",
            ),
        )
        .unwrap();
        let event_path = session_directory(&project, &vault, &started.session.session_id)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", committed.event_id));
        let mut event: SessionEvent =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        let SessionEventPayload::RecoveryCheckpointRecorded(recovery) = &mut event.payload else {
            panic!("expected recovery checkpoint event");
        };
        recovery.checkpoint.unresolved[0] = "A different unresolved claim".to_owned();
        recovery.provenance.candidate_fingerprint =
            crate::memory_transition::unresolved_candidate_fingerprint(
                &event.session_id,
                recovery.provenance.expected_event_count,
                &recovery.checkpoint.summary,
                &recovery.checkpoint.unresolved[0],
                &recovery.provenance.evidence_record_ids,
            );
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.session_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(message))
                if message.contains("binding fingerprint")
        ));
    }

    #[test]
    fn typed_bound_recovery_is_schema_v8_and_rejects_rebinding_or_kind_tamper() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('1'))).unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('2'), "Choose the persistence engine"),
        )
        .unwrap();
        let prompt_id = prompt.session.prompts[0].record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('3'), "Use SQLite for local-first persistence"),
        )
        .unwrap();
        let response_id = response.session.responses[0].record_id.clone();
        let candidate_fingerprint = crate::memory_transition::recovery_candidate_fingerprint(
            &started.session.session_id,
            3,
            crate::memory_transition::MemoryCandidateKind::Decision,
            "Persistence engine",
            "Use SQLite for local-first persistence",
            &[prompt_id.clone(), response_id.clone()],
        );
        let committed = checkpoint_recovered_structured_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_structured_input(
                request_id('4'),
                3,
                candidate_fingerprint.clone(),
                vec![response_id.clone(), prompt_id.clone()],
                RecoveredStructuredKind::Decision,
                "Persistence engine",
                "Use SQLite for local-first persistence",
            ),
        )
        .unwrap();
        assert_eq!(
            committed.session.schema_version,
            SESSION_TYPED_RECOVERY_SCHEMA_VERSION
        );
        assert!(committed.session_path.ends_with(SESSION_V8_FILE));
        assert_eq!(committed.session.checkpoints.len(), 1);
        assert_eq!(committed.session.checkpoints[0].decisions.len(), 1);
        assert!(committed.session.checkpoints[0].problems.is_empty());
        let replayed = checkpoint_recovered_structured_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_structured_input(
                request_id('4'),
                3,
                candidate_fingerprint,
                vec![prompt_id.clone(), response_id.clone()],
                RecoveredStructuredKind::Decision,
                "Persistence engine",
                "Use SQLite for local-first persistence",
            ),
        )
        .unwrap();
        assert!(replayed.replayed);

        let event_path = session_directory(&project, &vault, &started.session.session_id)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", committed.event_id));
        let original = std::fs::read(&event_path).unwrap();
        let mut event: SessionEvent = serde_json::from_slice(&original).unwrap();
        let SessionEventPayload::RecoveryCheckpointRecorded(recovery) = &mut event.payload else {
            panic!("expected typed recovery checkpoint event");
        };
        recovery.provenance.evidence_record_ids.pop();
        let (kind, subject, statement) = typed_recovery_checkpoint_claim(&recovery.checkpoint)
            .expect("typed recovery checkpoint must retain one claim");
        recovery.provenance.candidate_fingerprint =
            crate::memory_transition::recovery_candidate_fingerprint(
                &event.session_id,
                recovery.provenance.expected_event_count,
                recovery_memory_candidate_kind(kind),
                subject,
                statement,
                &recovery.provenance.evidence_record_ids,
            );
        recovery.provenance.binding_fingerprint = recovery_binding_fingerprint_v2(
            &event.session_id,
            recovery.provenance.expected_event_count,
            &recovery.provenance.candidate_fingerprint,
            &recovery.provenance.evidence_record_ids,
            kind,
            subject,
            statement,
        );
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.session_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(message))
                if message.contains("complete post-checkpoint window")
        ));

        std::fs::write(&event_path, original).unwrap();
        let mut event: SessionEvent =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        let SessionEventPayload::RecoveryCheckpointRecorded(recovery) = &mut event.payload else {
            panic!("expected typed recovery checkpoint event");
        };
        let decision = recovery.checkpoint.decisions.remove(0);
        recovery.checkpoint.problems.push(ProblemRecord {
            id: child_id("prb", &event.event_id, 0),
            title: decision.title,
            symptom: decision.decision,
            expected: String::new(),
            attempts: Vec::new(),
            resolution: None,
        });
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.session_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(message))
                if message.contains("candidate fingerprint")
        ));
    }

    #[test]
    fn task_bound_recovery_is_schema_v9_and_rejects_rebinding_or_status_tamper() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('1'))).unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('2'), "Track the release build task"),
        )
        .unwrap();
        let prompt_id = prompt.session.prompts[0].record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(
                request_id('3'),
                "Release build completed after smoke testing",
            ),
        )
        .unwrap();
        let response_id = response.session.responses[0].record_id.clone();
        let candidate_fingerprint = crate::memory_transition::task_candidate_fingerprint(
            &started.session.session_id,
            3,
            "Release build",
            TaskStatus::Completed,
            "Smoke test passed",
            &[prompt_id.clone(), response_id.clone()],
        );
        let committed = checkpoint_recovered_task_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_task_input(
                request_id('4'),
                3,
                candidate_fingerprint.clone(),
                vec![response_id.clone(), prompt_id.clone()],
                "Release build",
                TaskStatus::Completed,
                "Smoke test passed",
            ),
        )
        .unwrap();
        assert_eq!(
            committed.session.schema_version,
            SESSION_TASK_RECOVERY_SCHEMA_VERSION
        );
        assert!(committed.session_path.ends_with(SESSION_V9_FILE));
        assert_eq!(committed.session.checkpoints.len(), 1);
        assert_eq!(committed.session.checkpoints[0].tasks.len(), 1);
        assert_eq!(
            committed.session.checkpoints[0].tasks[0].status,
            TaskStatus::Completed
        );
        let replayed = checkpoint_recovered_task_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_task_input(
                request_id('4'),
                3,
                candidate_fingerprint,
                vec![prompt_id.clone(), response_id.clone()],
                "Release build",
                TaskStatus::Completed,
                "Smoke test passed",
            ),
        )
        .unwrap();
        assert!(replayed.replayed);

        let event_path = session_directory(&project, &vault, &started.session.session_id)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", committed.event_id));
        let original = std::fs::read(&event_path).unwrap();
        let mut event: SessionEvent = serde_json::from_slice(&original).unwrap();
        let SessionEventPayload::RecoveryCheckpointRecorded(recovery) = &mut event.payload else {
            panic!("expected task recovery checkpoint event");
        };
        recovery.provenance.evidence_record_ids.pop();
        let (title, status, details) = task_recovery_checkpoint_claim(&recovery.checkpoint)
            .expect("task recovery checkpoint must retain one task");
        recovery.provenance.candidate_fingerprint =
            crate::memory_transition::task_candidate_fingerprint(
                &event.session_id,
                recovery.provenance.expected_event_count,
                title,
                status,
                details,
                &recovery.provenance.evidence_record_ids,
            );
        recovery.provenance.binding_fingerprint = recovery_binding_fingerprint_v3(
            &event.session_id,
            recovery.provenance.expected_event_count,
            &recovery.provenance.candidate_fingerprint,
            &recovery.provenance.evidence_record_ids,
            title,
            status,
            details,
        );
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.session_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(message))
                if message.contains("complete post-checkpoint window")
        ));

        std::fs::write(&event_path, original).unwrap();
        let mut event: SessionEvent =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        let SessionEventPayload::RecoveryCheckpointRecorded(recovery) = &mut event.payload else {
            panic!("expected task recovery checkpoint event");
        };
        recovery.checkpoint.tasks[0].status = TaskStatus::Pending;
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.session_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(message))
                if message.contains("candidate fingerprint")
        ));
    }

    #[test]
    fn plan_bound_recovery_is_schema_v10_and_rejects_rebinding_or_status_tamper() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('1'))).unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('2'), "Track the release plan"),
        )
        .unwrap();
        let prompt_id = prompt.session.prompts[0].record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            turn_input(request_id('3'), "The release plan is completed"),
        )
        .unwrap();
        let response_id = response.session.responses[0].record_id.clone();
        let candidate_fingerprint = crate::memory_transition::plan_candidate_fingerprint(
            &started.session.session_id,
            3,
            "Ship the release",
            PlanStatus::Completed,
            &[prompt_id.clone(), response_id.clone()],
        );
        let committed = checkpoint_recovered_plan_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_plan_input(
                request_id('4'),
                3,
                candidate_fingerprint.clone(),
                vec![response_id.clone(), prompt_id.clone()],
                "Ship the release",
                PlanStatus::Completed,
            ),
        )
        .unwrap();
        assert_eq!(
            committed.session.schema_version,
            SESSION_PLAN_RECOVERY_SCHEMA_VERSION
        );
        assert!(committed.session_path.ends_with(SESSION_V10_FILE));
        assert_eq!(committed.session.checkpoints.len(), 1);
        assert_eq!(committed.session.checkpoints[0].plan.len(), 1);
        assert_eq!(
            committed.session.checkpoints[0].plan[0].status,
            PlanStatus::Completed
        );
        let replayed = checkpoint_recovered_plan_session(
            &project,
            &vault,
            &started.session.session_id,
            recovered_plan_input(
                request_id('4'),
                3,
                candidate_fingerprint,
                vec![prompt_id.clone(), response_id.clone()],
                "Ship the release",
                PlanStatus::Completed,
            ),
        )
        .unwrap();
        assert!(replayed.replayed);

        let event_path = session_directory(&project, &vault, &started.session.session_id)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", committed.event_id));
        let original = std::fs::read(&event_path).unwrap();
        let mut event: SessionEvent = serde_json::from_slice(&original).unwrap();
        let SessionEventPayload::RecoveryCheckpointRecorded(recovery) = &mut event.payload else {
            panic!("expected plan recovery checkpoint event");
        };
        recovery.provenance.evidence_record_ids.pop();
        let (text, status) = plan_recovery_checkpoint_claim(&recovery.checkpoint)
            .expect("plan recovery checkpoint must retain one plan");
        recovery.provenance.candidate_fingerprint =
            crate::memory_transition::plan_candidate_fingerprint(
                &event.session_id,
                recovery.provenance.expected_event_count,
                text,
                status,
                &recovery.provenance.evidence_record_ids,
            );
        recovery.provenance.binding_fingerprint = recovery_binding_fingerprint_v4(
            &event.session_id,
            recovery.provenance.expected_event_count,
            &recovery.provenance.candidate_fingerprint,
            &recovery.provenance.evidence_record_ids,
            text,
            status,
        );
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.session_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(message))
                if message.contains("complete post-checkpoint window")
        ));

        std::fs::write(&event_path, original).unwrap();
        let mut event: SessionEvent =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        let SessionEventPayload::RecoveryCheckpointRecorded(recovery) = &mut event.payload else {
            panic!("expected plan recovery checkpoint event");
        };
        recovery.checkpoint.plan[0].status = PlanStatus::Pending;
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.session_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
        assert!(matches!(
            read_session(&project, &vault, &started.session.session_id),
            Err(LeyCoreError::InvalidSessionStore(message))
                if message.contains("candidate fingerprint")
        ));
    }

    #[test]
    fn recovery_ledger_replays_schema_v3_v8_v9_and_v10_events_together() {
        let (_base, project, vault) = setup_memory();
        let started = start_session(&project, &vault, start_input(request_id('1'))).unwrap();
        let session_id = started.session.session_id.clone();

        let unresolved_prompt = record_session_prompt(
            &project,
            &vault,
            &session_id,
            turn_input(request_id('2'), "Investigate the retry loop"),
        )
        .unwrap();
        let unresolved_prompt_id = unresolved_prompt.session.prompts[0].record_id.clone();
        let unresolved_response = record_session_response(
            &project,
            &vault,
            &session_id,
            turn_input(request_id('3'), "No durable conclusion yet"),
        )
        .unwrap();
        let unresolved_response_id = unresolved_response.session.responses[0].record_id.clone();
        let unresolved_fingerprint = crate::memory_transition::unresolved_candidate_fingerprint(
            &session_id,
            3,
            "Retry investigation",
            "The retry investigation remains open",
            &[unresolved_prompt_id.clone(), unresolved_response_id.clone()],
        );
        let unresolved = checkpoint_recovered_unresolved_session(
            &project,
            &vault,
            &session_id,
            recovered_unresolved_input(
                request_id('4'),
                3,
                unresolved_fingerprint,
                vec![unresolved_response_id, unresolved_prompt_id],
                "Retry investigation",
                "The retry investigation remains open",
            ),
        )
        .unwrap();
        assert_eq!(
            unresolved.session.schema_version,
            SESSION_RECOVERY_SCHEMA_VERSION
        );
        assert!(unresolved.session_path.ends_with(SESSION_V3_FILE));

        let decision_prompt = record_session_prompt(
            &project,
            &vault,
            &session_id,
            turn_input(request_id('5'), "Choose the persistence engine"),
        )
        .unwrap();
        let decision_prompt_id = decision_prompt
            .session
            .prompts
            .last()
            .unwrap()
            .record_id
            .clone();
        let decision_response = record_session_response(
            &project,
            &vault,
            &session_id,
            turn_input(request_id('6'), "Use SQLite for local-first persistence"),
        )
        .unwrap();
        let decision_response_id = decision_response
            .session
            .responses
            .last()
            .unwrap()
            .record_id
            .clone();
        let decision_fingerprint = crate::memory_transition::recovery_candidate_fingerprint(
            &session_id,
            6,
            crate::memory_transition::MemoryCandidateKind::Decision,
            "Persistence engine",
            "Use SQLite for local-first persistence",
            &[decision_prompt_id.clone(), decision_response_id.clone()],
        );
        let decision = checkpoint_recovered_structured_session(
            &project,
            &vault,
            &session_id,
            recovered_structured_input(
                request_id('7'),
                6,
                decision_fingerprint,
                vec![decision_response_id, decision_prompt_id],
                RecoveredStructuredKind::Decision,
                "Persistence engine",
                "Use SQLite for local-first persistence",
            ),
        )
        .unwrap();
        assert_eq!(
            decision.session.schema_version,
            SESSION_TYPED_RECOVERY_SCHEMA_VERSION
        );
        assert!(decision.session_path.ends_with(SESSION_V8_FILE));

        let task_prompt = record_session_prompt(
            &project,
            &vault,
            &session_id,
            turn_input(request_id('8'), "Track the release build task"),
        )
        .unwrap();
        let task_prompt_id = task_prompt
            .session
            .prompts
            .last()
            .unwrap()
            .record_id
            .clone();
        let task_response = record_session_response(
            &project,
            &vault,
            &session_id,
            turn_input(
                request_id('9'),
                "Release build completed after smoke testing",
            ),
        )
        .unwrap();
        let task_response_id = task_response
            .session
            .responses
            .last()
            .unwrap()
            .record_id
            .clone();
        let task_fingerprint = crate::memory_transition::task_candidate_fingerprint(
            &session_id,
            9,
            "Release build",
            TaskStatus::Completed,
            "Smoke test passed",
            &[task_prompt_id.clone(), task_response_id.clone()],
        );
        let task = checkpoint_recovered_task_session(
            &project,
            &vault,
            &session_id,
            recovered_task_input(
                request_id('a'),
                9,
                task_fingerprint,
                vec![task_response_id, task_prompt_id],
                "Release build",
                TaskStatus::Completed,
                "Smoke test passed",
            ),
        )
        .unwrap();
        assert_eq!(
            task.session.schema_version,
            SESSION_TASK_RECOVERY_SCHEMA_VERSION
        );
        assert!(task.session_path.ends_with(SESSION_V9_FILE));

        let plan_prompt = record_session_prompt(
            &project,
            &vault,
            &session_id,
            turn_input(request_id('b'), "Track the release plan"),
        )
        .unwrap();
        let plan_prompt_id = plan_prompt
            .session
            .prompts
            .last()
            .unwrap()
            .record_id
            .clone();
        let plan_response = record_session_response(
            &project,
            &vault,
            &session_id,
            turn_input(request_id('c'), "The release plan is completed"),
        )
        .unwrap();
        let plan_response_id = plan_response
            .session
            .responses
            .last()
            .unwrap()
            .record_id
            .clone();
        let plan_fingerprint = crate::memory_transition::plan_candidate_fingerprint(
            &session_id,
            12,
            "Ship the release",
            PlanStatus::Completed,
            &[plan_prompt_id.clone(), plan_response_id.clone()],
        );
        let plan = checkpoint_recovered_plan_session(
            &project,
            &vault,
            &session_id,
            recovered_plan_input(
                request_id('d'),
                12,
                plan_fingerprint,
                vec![plan_response_id, plan_prompt_id],
                "Ship the release",
                PlanStatus::Completed,
            ),
        )
        .unwrap();
        assert_eq!(
            plan.session.schema_version,
            SESSION_PLAN_RECOVERY_SCHEMA_VERSION
        );
        assert!(plan.session_path.ends_with(SESSION_V10_FILE));

        let replayed = read_session(&project, &vault, &session_id).unwrap();
        assert_eq!(
            replayed.schema_version,
            SESSION_PLAN_RECOVERY_SCHEMA_VERSION
        );
        assert_eq!(replayed.event_count, 13);
        assert_eq!(replayed.checkpoints.len(), 4);
        assert_eq!(
            replayed.checkpoints[0].unresolved,
            vec!["The retry investigation remains open"]
        );
        assert_eq!(replayed.checkpoints[1].decisions.len(), 1);
        assert_eq!(
            replayed.checkpoints[1].decisions[0].decision,
            "Use SQLite for local-first persistence"
        );
        assert_eq!(replayed.checkpoints[2].tasks.len(), 1);
        assert_eq!(replayed.checkpoints[2].tasks[0].title, "Release build");
        assert_eq!(
            replayed.checkpoints[2].tasks[0].status,
            TaskStatus::Completed
        );
        assert_eq!(
            replayed.checkpoints[2].tasks[0].details,
            "Smoke test passed"
        );
        assert_eq!(replayed.checkpoints[3].plan.len(), 1);
        assert_eq!(replayed.checkpoints[3].plan[0].text, "Ship the release");
        assert_eq!(
            replayed.checkpoints[3].plan[0].status,
            PlanStatus::Completed
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
