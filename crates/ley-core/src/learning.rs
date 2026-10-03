use crate::continuity_store::ContinuityEventLinkInput;
use crate::ingestion::{
    load_project_memory, lock_project_memory_lifecycle, redact_secrets, ProjectMemoryLifecycleLock,
};
use crate::retrieval::validate_project_memory;
use crate::session::{
    read_recovery_derivation_origin, read_recovery_derivation_origin_with_continuity_transition,
};
use crate::{
    diagnose_project, read_session, read_session_with_continuity_transition, ContinuityStore,
    LeyCoreError, RedactionFinding, SessionArtifactCitation, SessionStatus,
};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const LEARNING_SCHEMA_VERSION: u32 = 3;
const PREVIOUS_LEARNING_SCHEMA_VERSION: u32 = 2;
const LEGACY_LEARNING_SCHEMA_VERSION: u32 = 1;
pub const MAX_LEARNING_ORIGIN_SOURCES: usize = 256;
pub const LEARNING_EVENT_LIMIT_BYTES: u64 = 1_048_576;
pub const LEARNING_INDEX_LIMIT_BYTES: u64 = 67_108_864;
pub const LEARNING_EVENT_LIMIT: usize = 10_000;

const STORE_ROOT: &str = ".ley";
const AGENT_MEMORY_DIRECTORY: &str = "agent-memory";
const PROJECTS_DIRECTORY: &str = "projects";
const LEARNINGS_DIRECTORY: &str = "learnings";
const EVENTS_DIRECTORY: &str = "events";
const LEARNING_LOCK_FILE: &str = "learnings-v1.lock";
const LEARNING_INDEX_FILE: &str = "learnings-v1.json";
const LEARNING_REVIEW_FILE: &str = "review.md";
const LEARNING_AUTHORITY_CUTOVER_EVENT_KIND: &str = "learning-authority-cutover";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LearningKind {
    Procedure,
    Constraint,
    Pitfall,
    Convention,
    Fact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LearningState {
    Tentative,
    Verified,
    Contested,
    Superseded,
    Rejected,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LearningTrustState {
    ReviewRequired,
    Trusted,
    Contested,
    Superseded,
    Rejected,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LearningProvenance {
    UserAuthored,
    AgentAuthored,
    Inferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LearningActor {
    User,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LearningFeedbackAction {
    Confirm,
    Contest,
    Reject,
    MarkStale,
    Supersede,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LearningFreshness {
    Current,
    SourceChanged,
    Uncited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningEvidenceInput {
    pub session_id: String,
    pub record_id: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProposeLearningInput {
    pub request_id: String,
    pub actor: LearningActor,
    pub kind: LearningKind,
    pub title: String,
    pub guidance: String,
    pub confidence_percent: u8,
    pub provenance: LearningProvenance,
    pub evidence: Vec<LearningEvidenceInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CorrectLearningInput {
    pub request_id: String,
    #[serde(default)]
    pub expected_event_count: Option<u64>,
    pub actor: LearningActor,
    pub title: String,
    pub guidance: String,
    pub confidence_percent: u8,
    pub evidence: Vec<LearningEvidenceInput>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewLearningInput {
    pub request_id: String,
    #[serde(default)]
    pub expected_event_count: Option<u64>,
    pub actor: LearningActor,
    pub action: LearningFeedbackAction,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub replacement_learning_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningRedaction {
    pub field: String,
    pub kind: String,
    pub lines: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningEvidence {
    pub session_id: String,
    pub record_id: String,
    pub record_type: String,
    pub session_status: SessionStatus,
    pub session_updated_at_unix_ms: u64,
    pub note: String,
    pub artifacts: Vec<SessionArtifactCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum LearningOriginSource {
    SessionRecord {
        session_id: String,
        record_id: String,
        record_type: String,
    },
    CapturedArtifact {
        artifact_snapshot_id: String,
        artifact_path: String,
        content_hash: String,
    },
    TurnEvidence {
        session_id: String,
        record_id: String,
    },
    ToolEvidence {
        session_id: String,
        record_id: String,
    },
    RecoveryCandidate {
        session_id: String,
        candidate_fingerprint: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningOriginLineage {
    pub mechanically_resolved: bool,
    pub causal_completeness_proven: bool,
    pub omitted_sources: usize,
    pub automatic_authority_ceiling: LearningTrustState,
    pub sources: Vec<LearningOriginSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningOriginSummary {
    pub mechanically_resolved: bool,
    pub causal_completeness_proven: bool,
    pub omitted_sources: usize,
    pub automatic_authority_ceiling: LearningTrustState,
    pub recorded_sources: usize,
    pub session_records: usize,
    pub captured_artifacts: usize,
    pub turn_evidence: usize,
    pub tool_evidence: usize,
    pub recovery_candidates: usize,
}

impl From<&LearningOriginLineage> for LearningOriginSummary {
    fn from(lineage: &LearningOriginLineage) -> Self {
        let mut summary = Self {
            mechanically_resolved: lineage.mechanically_resolved,
            causal_completeness_proven: lineage.causal_completeness_proven,
            omitted_sources: lineage.omitted_sources,
            automatic_authority_ceiling: lineage.automatic_authority_ceiling,
            recorded_sources: lineage.sources.len(),
            session_records: 0,
            captured_artifacts: 0,
            turn_evidence: 0,
            tool_evidence: 0,
            recovery_candidates: 0,
        };
        for source in &lineage.sources {
            match source {
                LearningOriginSource::SessionRecord { .. } => summary.session_records += 1,
                LearningOriginSource::CapturedArtifact { .. } => summary.captured_artifacts += 1,
                LearningOriginSource::TurnEvidence { .. } => summary.turn_evidence += 1,
                LearningOriginSource::ToolEvidence { .. } => summary.tool_evidence += 1,
                LearningOriginSource::RecoveryCandidate { .. } => summary.recovery_candidates += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningReviewEntry {
    pub event_id: String,
    pub recorded_at_unix_ms: u64,
    pub actor: LearningActor,
    pub action: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningRecord {
    pub schema_version: u32,
    pub project_id: String,
    pub learning_id: String,
    pub kind: LearningKind,
    pub title: String,
    pub guidance: String,
    pub state: LearningState,
    pub trust_state: LearningTrustState,
    pub provenance: LearningProvenance,
    pub origin_lineage: LearningOriginLineage,
    pub confidence_percent: u8,
    pub freshness: LearningFreshness,
    pub corroborating_sessions: usize,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub valid_from_unix_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until_unix_ms: Option<u64>,
    pub evidence: Vec<LearningEvidence>,
    pub history: Vec<LearningReviewEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub event_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningIndex {
    pub schema_version: u32,
    pub project_id: String,
    pub generated_at_unix_ms: u64,
    pub learnings: Vec<LearningRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningSummary {
    pub project_id: String,
    pub learning_id: String,
    pub kind: LearningKind,
    pub title: String,
    pub guidance_excerpt: String,
    pub state: LearningState,
    pub trust_state: LearningTrustState,
    pub provenance: LearningProvenance,
    pub origin_lineage_summary: LearningOriginSummary,
    pub confidence_percent: u8,
    pub freshness: LearningFreshness,
    pub corroborating_sessions: usize,
    pub updated_at_unix_ms: u64,
    pub event_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningMutation {
    pub learning: LearningRecord,
    pub event_id: String,
    pub replayed: bool,
    pub index_path: String,
    pub review_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LearningWriteResult {
    pub learning: LearningRecord,
    pub event_id: String,
    pub replayed: bool,
}

impl From<LearningMutation> for LearningWriteResult {
    fn from(mutation: LearningMutation) -> Self {
        Self {
            learning: mutation.learning,
            event_id: mutation.event_id,
            replayed: mutation.replayed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LearningEvent {
    schema_version: u32,
    event_id: String,
    project_id: String,
    learning_id: String,
    request_id: String,
    request_fingerprint: String,
    sequence: u64,
    recorded_at_unix_ms: u64,
    redactions: Vec<LearningRedaction>,
    #[serde(flatten)]
    payload: LearningEventPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum LearningEventPayload {
    Proposed {
        actor: LearningActor,
        kind: LearningKind,
        title: String,
        guidance: String,
        confidence_percent: u8,
        provenance: LearningProvenance,
        evidence: Vec<LearningEvidence>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_lineage: Option<LearningOriginLineage>,
    },
    Corrected {
        actor: LearningActor,
        title: String,
        guidance: String,
        confidence_percent: u8,
        evidence: Vec<LearningEvidence>,
        note: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_lineage: Option<LearningOriginLineage>,
    },
    Reviewed {
        actor: LearningActor,
        action: LearningFeedbackAction,
        note: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        replacement_learning_id: Option<String>,
    },
}

pub fn generate_learning_request_id() -> String {
    format!("req_{}", uuid::Uuid::new_v4().simple())
}

pub fn propose_learning(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    input: ProposeLearningInput,
) -> Result<LearningMutation, LeyCoreError> {
    propose_learning_with_session_transition(project_start.as_ref(), vault.as_ref(), None, input)
}

pub fn propose_learning_with_continuity_transition(
    project_start: impl AsRef<Path>,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
    input: ProposeLearningInput,
) -> Result<LearningWriteResult, LeyCoreError> {
    let project_start = project_start.as_ref();
    let legacy_vault = legacy_vault.as_ref();
    crate::session::ensure_native_session_authority(project_start, legacy_vault, store)?;
    ensure_native_learning_authority(project_start, legacy_vault, store)?;
    let (diagnostic, learning_id, pending) =
        prepare_learning_proposal(project_start, legacy_vault, Some(store), input)?;
    mutate_learning_in_continuity_store(
        &diagnostic.root,
        legacy_vault,
        store,
        &diagnostic.identity,
        &learning_id,
        pending,
    )
}

fn propose_learning_with_session_transition(
    project_start: &Path,
    vault: &Path,
    transition_store: Option<&ContinuityStore>,
    input: ProposeLearningInput,
) -> Result<LearningMutation, LeyCoreError> {
    let (diagnostic, learning_id, pending) =
        prepare_learning_proposal(project_start, vault, transition_store, input)?;
    mutate_learning(
        &diagnostic.identity.project_id,
        &diagnostic.root,
        &learning_id,
        pending,
        vault,
    )
}

fn prepare_learning_proposal(
    project_start: &Path,
    vault: &Path,
    transition_store: Option<&ContinuityStore>,
    input: ProposeLearningInput,
) -> Result<(crate::ProjectDiagnostic, String, PendingLearningEvent), LeyCoreError> {
    validate_request_id(&input.request_id)?;
    validate_confidence(input.confidence_percent)?;
    validate_provenance_authority(input.actor, input.provenance)?;
    let diagnostic = diagnose_project(project_start)?;
    validate_learning_project_memory(&diagnostic.root, vault, transition_store)?;
    let learning_id = deterministic_id(
        "lrn",
        &format!("{}:{}", diagnostic.identity.project_id, input.request_id),
        32,
    );
    let event_id = deterministic_id(
        "lev",
        &format!("{learning_id}:{}:proposed", input.request_id),
        64,
    );
    let mut redactions = Vec::new();
    let (evidence, origin_lineage) = resolve_evidence(
        &diagnostic.root,
        vault,
        transition_store,
        input.evidence,
        &mut redactions,
    )?;
    let payload = LearningEventPayload::Proposed {
        actor: input.actor,
        kind: input.kind,
        title: sanitize_text("title", &input.title, 1, 256, &mut redactions)?,
        guidance: sanitize_text("guidance", &input.guidance, 1, 16_000, &mut redactions)?,
        confidence_percent: input.confidence_percent,
        provenance: input.provenance,
        evidence,
        origin_lineage: Some(origin_lineage),
    };
    Ok((
        diagnostic,
        learning_id,
        PendingLearningEvent {
            event_id,
            request_id: input.request_id,
            expected_event_count: None,
            expected_replacement_event_count: None,
            redactions,
            payload,
            allow_create: true,
        },
    ))
}

pub fn correct_learning(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    learning_id: &str,
    input: CorrectLearningInput,
) -> Result<LearningMutation, LeyCoreError> {
    correct_learning_with_session_transition(
        project_start.as_ref(),
        vault.as_ref(),
        None,
        learning_id,
        input,
    )
}

pub fn correct_learning_with_continuity_transition(
    project_start: impl AsRef<Path>,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
    learning_id: &str,
    input: CorrectLearningInput,
) -> Result<LearningWriteResult, LeyCoreError> {
    let project_start = project_start.as_ref();
    let legacy_vault = legacy_vault.as_ref();
    crate::session::ensure_native_session_authority(project_start, legacy_vault, store)?;
    ensure_native_learning_authority(project_start, legacy_vault, store)?;
    let (diagnostic, pending) =
        prepare_learning_correction(project_start, legacy_vault, Some(store), learning_id, input)?;
    mutate_learning_in_continuity_store(
        &diagnostic.root,
        legacy_vault,
        store,
        &diagnostic.identity,
        learning_id,
        pending,
    )
}

fn correct_learning_with_session_transition(
    project_start: &Path,
    vault: &Path,
    transition_store: Option<&ContinuityStore>,
    learning_id: &str,
    input: CorrectLearningInput,
) -> Result<LearningMutation, LeyCoreError> {
    let (diagnostic, pending) =
        prepare_learning_correction(project_start, vault, transition_store, learning_id, input)?;
    mutate_learning(
        &diagnostic.identity.project_id,
        &diagnostic.root,
        learning_id,
        pending,
        vault,
    )
}

fn prepare_learning_correction(
    project_start: &Path,
    vault: &Path,
    transition_store: Option<&ContinuityStore>,
    learning_id: &str,
    input: CorrectLearningInput,
) -> Result<(crate::ProjectDiagnostic, PendingLearningEvent), LeyCoreError> {
    validate_learning_id(learning_id)?;
    validate_request_id(&input.request_id)?;
    validate_confidence(input.confidence_percent)?;
    let diagnostic = diagnose_project(project_start)?;
    validate_learning_project_memory(&diagnostic.root, vault, transition_store)?;
    let event_id = deterministic_id(
        "lev",
        &format!("{learning_id}:{}:corrected", input.request_id),
        64,
    );
    let mut redactions = Vec::new();
    let (evidence, origin_lineage) = resolve_evidence(
        &diagnostic.root,
        vault,
        transition_store,
        input.evidence,
        &mut redactions,
    )?;
    let payload = LearningEventPayload::Corrected {
        actor: input.actor,
        title: sanitize_text("title", &input.title, 1, 256, &mut redactions)?,
        guidance: sanitize_text("guidance", &input.guidance, 1, 16_000, &mut redactions)?,
        confidence_percent: input.confidence_percent,
        evidence,
        note: sanitize_text("note", &input.note, 0, 4_000, &mut redactions)?,
        origin_lineage: Some(origin_lineage),
    };
    Ok((
        diagnostic,
        PendingLearningEvent {
            event_id,
            request_id: input.request_id,
            expected_event_count: input.expected_event_count,
            expected_replacement_event_count: None,
            redactions,
            payload,
            allow_create: false,
        },
    ))
}

pub fn review_learning(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    learning_id: &str,
    input: ReviewLearningInput,
) -> Result<LearningMutation, LeyCoreError> {
    validate_learning_id(learning_id)?;
    validate_request_id(&input.request_id)?;
    validate_review_authority(input.actor, input.action)?;
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
    match input.action {
        LearningFeedbackAction::Supersede => {
            let replacement = input.replacement_learning_id.as_deref().ok_or_else(|| {
                LeyCoreError::InvalidLearningRequest(
                    "supersede requires replacementLearningId".to_owned(),
                )
            })?;
            validate_learning_id(replacement)?;
            if replacement == learning_id {
                return Err(LeyCoreError::InvalidLearningRequest(
                    "a learning cannot supersede itself".to_owned(),
                ));
            }
            read_learning(&diagnostic.root, &vault, replacement)?;
        }
        _ if input.replacement_learning_id.is_some() => {
            return Err(LeyCoreError::InvalidLearningRequest(
                "replacementLearningId is only valid for supersede".to_owned(),
            ));
        }
        _ => {}
    }
    let event_id = deterministic_id(
        "lev",
        &format!(
            "{learning_id}:{}:reviewed:{}",
            input.request_id,
            feedback_label(input.action)
        ),
        64,
    );
    let mut redactions = Vec::new();
    let payload = LearningEventPayload::Reviewed {
        actor: input.actor,
        action: input.action,
        note: sanitize_text("note", &input.note, 0, 4_000, &mut redactions)?,
        replacement_learning_id: input.replacement_learning_id,
    };
    mutate_learning(
        &diagnostic.identity.project_id,
        &diagnostic.root,
        learning_id,
        PendingLearningEvent {
            event_id,
            request_id: input.request_id,
            expected_event_count: input.expected_event_count,
            expected_replacement_event_count: None,
            redactions,
            payload,
            allow_create: false,
        },
        vault,
    )
}

pub fn review_learning_with_continuity_transition(
    project_start: impl AsRef<Path>,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
    learning_id: &str,
    input: ReviewLearningInput,
) -> Result<LearningWriteResult, LeyCoreError> {
    review_learning_with_continuity_transition_guarded(
        project_start,
        legacy_vault,
        store,
        learning_id,
        None,
        input,
    )
}

pub fn review_learning_with_continuity_transition_guarded(
    project_start: impl AsRef<Path>,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
    learning_id: &str,
    expected_replacement_event_count: Option<u64>,
    input: ReviewLearningInput,
) -> Result<LearningWriteResult, LeyCoreError> {
    let project_start = project_start.as_ref();
    let legacy_vault = legacy_vault.as_ref();
    crate::session::ensure_native_session_authority(project_start, legacy_vault, store)?;
    ensure_native_learning_authority(project_start, legacy_vault, store)?;
    validate_learning_id(learning_id)?;
    validate_request_id(&input.request_id)?;
    validate_review_authority(input.actor, input.action)?;
    let diagnostic = diagnose_project(project_start)?;
    validate_learning_project_memory(&diagnostic.root, legacy_vault, Some(store))?;
    match input.action {
        LearningFeedbackAction::Supersede => {
            let replacement = input.replacement_learning_id.as_deref().ok_or_else(|| {
                LeyCoreError::InvalidLearningRequest(
                    "supersede requires replacementLearningId".to_owned(),
                )
            })?;
            validate_learning_id(replacement)?;
            if replacement == learning_id {
                return Err(LeyCoreError::InvalidLearningRequest(
                    "a learning cannot supersede itself".to_owned(),
                ));
            }
            read_learning_from_continuity_snapshot(
                &diagnostic.root,
                legacy_vault,
                store,
                replacement,
            )?;
        }
        _ if input.replacement_learning_id.is_some()
            || expected_replacement_event_count.is_some() =>
        {
            return Err(LeyCoreError::InvalidLearningRequest(
                "replacement learning guards are only valid for supersede".to_owned(),
            ));
        }
        _ => {}
    }
    let event_id = deterministic_id(
        "lev",
        &format!(
            "{learning_id}:{}:reviewed:{}",
            input.request_id,
            feedback_label(input.action)
        ),
        64,
    );
    let mut redactions = Vec::new();
    let payload = LearningEventPayload::Reviewed {
        actor: input.actor,
        action: input.action,
        note: sanitize_text("note", &input.note, 0, 4_000, &mut redactions)?,
        replacement_learning_id: input.replacement_learning_id,
    };
    mutate_learning_in_continuity_store(
        &diagnostic.root,
        legacy_vault,
        store,
        &diagnostic.identity,
        learning_id,
        PendingLearningEvent {
            event_id,
            request_id: input.request_id,
            expected_event_count: input.expected_event_count,
            expected_replacement_event_count,
            redactions,
            payload,
            allow_create: false,
        },
    )
}

pub fn read_learning(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    learning_id: &str,
) -> Result<LearningRecord, LeyCoreError> {
    validate_learning_id(learning_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
    let Some(store) = LearningStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Err(LeyCoreError::LearningNotFound(learning_id.to_owned()));
    };
    let _lock = store.lock(true)?;
    let events = store.read_events()?;
    let mut learning = replay_one(&events, &diagnostic.identity.project_id, learning_id)?;
    refresh_freshness(&mut learning, &diagnostic.root, &vault)?;
    Ok(learning)
}

pub fn list_learnings(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
) -> Result<Vec<LearningSummary>, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
    let Some(store) = LearningStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Ok(Vec::new());
    };
    let _lock = store.lock(true)?;
    let events = store.read_events()?;
    let mut records = replay_all(&events, &diagnostic.identity.project_id)?;
    for record in &mut records {
        refresh_freshness(record, &diagnostic.root, &vault)?;
    }
    records.sort_by(|left, right| {
        right
            .updated_at_unix_ms
            .cmp(&left.updated_at_unix_ms)
            .then_with(|| left.learning_id.cmp(&right.learning_id))
    });
    Ok(records.iter().map(LearningSummary::from).collect())
}

pub(crate) fn continuity_events_for_migration(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
) -> Result<Vec<crate::ContinuityEventInput>, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &vault)?;
    let Some(store) = LearningStore::open(&vault, &diagnostic.identity.project_id, false)? else {
        return Ok(Vec::new());
    };
    let _lock = store.lock(true)?;
    let events = store.read_events()?;
    replay_all(&events, &diagnostic.identity.project_id)?;
    events
        .into_iter()
        .map(|event| {
            let kind = match &event.payload {
                LearningEventPayload::Proposed { .. } => "legacy-learning-proposed",
                LearningEventPayload::Corrected { .. } => "legacy-learning-corrected",
                LearningEventPayload::Reviewed { .. } => "legacy-learning-reviewed",
            };
            let payload = serde_json::to_value(&event).map_err(|error| {
                LeyCoreError::InvalidLearningStore(format!(
                    "validated learning event could not be serialized for migration: {error}"
                ))
            })?;
            Ok(crate::ContinuityEventInput {
                event_id: event.event_id,
                project_id: event.project_id,
                subject_id: Some(event.learning_id),
                session_id: None,
                session_sequence: None,
                request_id: None,
                request_fingerprint: None,
                kind: kind.to_owned(),
                payload_version: 1,
                recorded_at_unix_ms: event.recorded_at_unix_ms,
                revision_head: None,
                revision_branch: None,
                payload,
            })
        })
        .collect()
}

pub(crate) fn read_learning_from_continuity_snapshot(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    store: &ContinuityStore,
    learning_id: &str,
) -> Result<LearningRecord, LeyCoreError> {
    validate_learning_id(learning_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    let events = store
        .events_for_subject(&diagnostic.identity.project_id, learning_id)?
        .into_iter()
        .map(learning_event_from_continuity)
        .collect::<Result<Vec<_>, _>>()?;
    let mut learning = replay_one(&events, &diagnostic.identity.project_id, learning_id)?;
    refresh_freshness_with_continuity_transition(&mut learning, &diagnostic.root, vault, store)?;
    Ok(learning)
}

pub fn read_learning_with_continuity_transition(
    project_start: impl AsRef<Path>,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
    learning_id: &str,
) -> Result<LearningRecord, LeyCoreError> {
    let project_start = project_start.as_ref();
    let legacy_vault = legacy_vault.as_ref();
    sync_legacy_continuity_for_learning_read(project_start, legacy_vault, store)?;
    read_learning_from_continuity_snapshot(project_start, legacy_vault, store, learning_id)
}

pub(crate) fn list_learnings_from_continuity_snapshot(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<Vec<LearningSummary>, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    let events = store
        .learning_events(&diagnostic.identity.project_id)?
        .into_iter()
        .map(learning_event_from_continuity)
        .collect::<Result<Vec<_>, _>>()?;
    let mut records = replay_all(&events, &diagnostic.identity.project_id)?;
    refresh_all_freshness_with_continuity_transition(&mut records, &diagnostic.root, vault, store)?;
    records.sort_by(|left, right| {
        right
            .updated_at_unix_ms
            .cmp(&left.updated_at_unix_ms)
            .then_with(|| left.learning_id.cmp(&right.learning_id))
    });
    Ok(records.iter().map(LearningSummary::from).collect())
}

pub fn list_learnings_with_continuity_transition(
    project_start: impl AsRef<Path>,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<Vec<LearningSummary>, LeyCoreError> {
    let project_start = project_start.as_ref();
    let legacy_vault = legacy_vault.as_ref();
    sync_legacy_continuity_for_learning_read(project_start, legacy_vault, store)?;
    list_learnings_from_continuity_snapshot(project_start, legacy_vault, store)
}

pub(crate) fn list_learnings_for_project_id(
    store: &ContinuityStore,
    project_id: &str,
) -> Result<Vec<LearningSummary>, LeyCoreError> {
    crate::validate_project_id(project_id)?;
    let events = store
        .learning_events(project_id)?
        .into_iter()
        .map(learning_event_from_continuity)
        .collect::<Result<Vec<_>, _>>()?;
    let mut records = replay_all(&events, project_id)?;
    let hashes = store.current_artifact_hashes(project_id)?.ok_or_else(|| {
        LeyCoreError::ProjectMemoryUnavailable(
            "canonical native artifact continuity is unavailable".to_owned(),
        )
    })?;
    for learning in &mut records {
        refresh_freshness_from_hashes(learning, &hashes);
    }
    records.sort_by(|left, right| {
        right
            .updated_at_unix_ms
            .cmp(&left.updated_at_unix_ms)
            .then_with(|| left.learning_id.cmp(&right.learning_id))
    });
    Ok(records.iter().map(LearningSummary::from).collect())
}

fn sync_legacy_continuity_for_learning_read(
    project_start: &Path,
    legacy_vault: &Path,
    store: &ContinuityStore,
) -> Result<String, LeyCoreError> {
    let diagnostic = diagnose_project(project_start)?;
    let project_id = diagnostic.identity.project_id.clone();
    if learning_authority_cutover_is_complete(store, &project_id)? {
        return Ok(project_id);
    }
    if crate::session::session_authority_cutover_is_complete(store, &project_id)? {
        crate::continuity_import::import_legacy_learning_continuity(
            project_start,
            legacy_vault,
            store,
        )?;
    } else {
        crate::import_legacy_continuity(project_start, legacy_vault, store)?;
    }
    Ok(project_id)
}

fn learning_authority_cutover_event_id(project_id: &str) -> String {
    deterministic_id(
        "cut",
        &format!("ley-native-learning-authority-v1:{project_id}"),
        64,
    )
}

fn fence_legacy_learning_writes(
    project_start: impl AsRef<Path>,
    legacy_vault: impl AsRef<Path>,
) -> Result<bool, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    validate_project_memory(&diagnostic.root, &legacy_vault)?;
    let vault_path = legacy_vault
        .as_ref()
        .canonicalize()
        .map_err(|source| LeyCoreError::Io {
            path: legacy_vault.as_ref().to_path_buf(),
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

    let mut read_options = OpenOptions::new();
    read_options.read(true).follow(FollowSymlinks::No);
    let read_lock = match project_dir.open_with(LEARNING_LOCK_FILE, &read_options) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ensure_lock_file(&project_dir)?;
            project_dir
                .open_with(LEARNING_LOCK_FILE, &read_options)
                .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?
        }
        Err(source) => return Err(learning_io(LEARNING_LOCK_FILE, source)),
    };
    ensure_private_file(&read_lock, LEARNING_LOCK_FILE)?;
    if read_lock
        .metadata()
        .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?
        .permissions()
        .readonly()
    {
        return Ok(false);
    }
    drop(read_lock);

    let mut write_options = OpenOptions::new();
    write_options
        .read(true)
        .write(true)
        .follow(FollowSymlinks::No);
    let lock = project_dir
        .open_with(LEARNING_LOCK_FILE, &write_options)
        .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?;
    ensure_private_file(&lock, LEARNING_LOCK_FILE)?;
    let file = lock.into_std();
    file.lock()
        .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?;
    let mut permissions = file
        .metadata()
        .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?
        .permissions();
    permissions.set_readonly(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o400);
    }
    file.set_permissions(permissions)
        .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?;
    file.sync_all()
        .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?;
    let metadata = file
        .metadata()
        .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?;
    if !metadata.permissions().readonly() {
        return Err(LeyCoreError::InvalidLearningStore(
            "legacy learning writer fence did not become read-only".to_owned(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o222 != 0 {
            return Err(LeyCoreError::InvalidLearningStore(
                "legacy learning writer fence retained write permission".to_owned(),
            ));
        }
    }
    Ok(true)
}

fn learning_authority_cutover_input(
    summary: &crate::continuity_import::LegacyLearningImportSummary,
) -> crate::ContinuityEventInput {
    crate::ContinuityEventInput {
        event_id: learning_authority_cutover_event_id(&summary.project_id),
        project_id: summary.project_id.clone(),
        subject_id: None,
        session_id: None,
        session_sequence: None,
        request_id: None,
        request_fingerprint: None,
        kind: LEARNING_AUTHORITY_CUTOVER_EVENT_KIND.to_owned(),
        payload_version: 1,
        recorded_at_unix_ms: summary.manifest_recorded_at_unix_ms.saturating_add(1),
        revision_head: None,
        revision_branch: None,
        payload: serde_json::json!({
            "source": "native-learning-authority",
            "formatVersion": 1,
            "legacyLearningSnapshotEventId": summary.manifest_event_id,
            "legacyLearningEventDigest": summary.source_digest,
            "legacyLearningEventCount": summary.learning_events
        }),
    }
}

fn native_born_learning_authority_input(
    identity: &crate::ProjectIdentity,
) -> crate::ContinuityEventInput {
    crate::ContinuityEventInput {
        event_id: learning_authority_cutover_event_id(&identity.project_id),
        project_id: identity.project_id.clone(),
        subject_id: None,
        session_id: None,
        session_sequence: None,
        request_id: None,
        request_fingerprint: None,
        kind: LEARNING_AUTHORITY_CUTOVER_EVENT_KIND.to_owned(),
        payload_version: 2,
        recorded_at_unix_ms: identity.created_at_unix_ms.saturating_add(2),
        revision_head: None,
        revision_branch: None,
        payload: serde_json::json!({
            "source": "native-learning-authority",
            "formatVersion": 2,
            "origin": "native-born"
        }),
    }
}

pub(crate) fn learning_authority_cutover_is_complete(
    store: &ContinuityStore,
    project_id: &str,
) -> Result<bool, LeyCoreError> {
    let event_id = learning_authority_cutover_event_id(project_id);
    let Some(event) = store.event(project_id, &event_id)? else {
        return Ok(false);
    };
    if event.kind != LEARNING_AUTHORITY_CUTOVER_EVENT_KIND
        || event.subject_id.is_some()
        || event.session_id.is_some()
        || event.session_sequence.is_some()
        || event.request_id.is_some()
        || event.request_fingerprint.is_some()
        || event.revision_head.is_some()
        || event.revision_branch.is_some()
        || event.payload["source"] != "native-learning-authority"
    {
        return Err(LeyCoreError::InvalidContinuityStore(
            "native learning-authority marker is invalid".to_owned(),
        ));
    }
    match event.payload["formatVersion"].as_u64() {
        Some(2) => {
            if event.payload_version != 2
                || event.payload["origin"] != "native-born"
                || event.payload.as_object().map_or(0, |payload| payload.len()) != 3
            {
                return Err(LeyCoreError::InvalidContinuityStore(
                    "native-born learning-authority cutover marker is invalid".to_owned(),
                ));
            }
            return Ok(true);
        }
        Some(1) => {}
        _ => {
            return Err(LeyCoreError::InvalidContinuityStore(
                "native learning-authority marker has an unsupported format".to_owned(),
            ))
        }
    }
    let manifest_event_id = event.payload["legacyLearningSnapshotEventId"]
        .as_str()
        .ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "native learning-authority cutover marker is invalid".to_owned(),
            )
        })?;
    let manifest_digest = event.payload["legacyLearningEventDigest"]
        .as_str()
        .ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "native learning-authority cutover marker is invalid".to_owned(),
            )
        })?;
    let manifest_count = event.payload["legacyLearningEventCount"]
        .as_u64()
        .ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "native learning-authority cutover marker is invalid".to_owned(),
            )
        })?;
    let Some(manifest) = store.event(project_id, manifest_event_id)? else {
        return Err(LeyCoreError::InvalidContinuityStore(
            "native learning-authority cutover marker references a missing learning snapshot"
                .to_owned(),
        ));
    };
    if event.payload_version != 1
        || event.recorded_at_unix_ms != manifest.recorded_at_unix_ms.saturating_add(1)
        || event.payload.as_object().map_or(0, |payload| payload.len()) != 5
        || manifest.kind != "legacy-learning-snapshot-imported"
        || manifest.subject_id.is_some()
        || manifest.session_id.is_some()
        || manifest.session_sequence.is_some()
        || manifest.request_id.is_some()
        || manifest.request_fingerprint.is_some()
        || manifest.payload["source"] != "legacy-learning-memory"
        || manifest.payload["formatVersion"] != 1
        || manifest.payload["eventDigest"] != manifest_digest
        || manifest.payload["learningEventCount"] != manifest_count
    {
        return Err(LeyCoreError::InvalidContinuityStore(
            "native learning-authority cutover marker is invalid".to_owned(),
        ));
    }
    Ok(true)
}

pub(crate) fn establish_native_born_learning_authority(
    project_start: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<String, LeyCoreError> {
    let diagnostic = diagnose_project(project_start)?;
    let project_id = diagnostic.identity.project_id.clone();
    store.register_project(&diagnostic.identity)?;
    if learning_authority_cutover_is_complete(store, &project_id)? {
        let marker = store
            .event(
                &project_id,
                &learning_authority_cutover_event_id(&project_id),
            )?
            .ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(
                    "native learning-authority marker disappeared during initialization".to_owned(),
                )
            })?;
        if marker.payload["origin"] == "native-born" {
            return Ok(project_id);
        }
        return Err(LeyCoreError::InvalidContinuityStore(
            "native-born learning authority cannot replace a legacy-cutover authority".to_owned(),
        ));
    }
    if !crate::session::session_authority_cutover_is_complete(store, &project_id)? {
        return Err(LeyCoreError::InvalidLearningStore(
            "native-born learning authority requires native session authority first".to_owned(),
        ));
    }
    let input = native_born_learning_authority_input(&diagnostic.identity);
    match store.append_project_event_if_count(&input, 1) {
        Ok(_) => {}
        Err(error) => {
            if learning_authority_cutover_is_complete(store, &project_id)? {
                if store
                    .event(
                        &project_id,
                        &learning_authority_cutover_event_id(&project_id),
                    )?
                    .is_some_and(|marker| marker.payload["origin"] == "native-born")
                {
                    return Ok(project_id);
                }
            }
            return Err(error);
        }
    }
    if !learning_authority_cutover_is_complete(store, &project_id)? {
        return Err(LeyCoreError::InvalidContinuityStore(
            "native-born learning authority did not commit a valid marker".to_owned(),
        ));
    }
    Ok(project_id)
}

fn ensure_native_learning_authority(
    project_start: impl AsRef<Path>,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<String, LeyCoreError> {
    let project_start = project_start.as_ref();
    let legacy_vault = legacy_vault.as_ref();
    let project_id = diagnose_project(project_start)?.identity.project_id;
    if learning_authority_cutover_is_complete(store, &project_id)? {
        return Ok(project_id);
    }
    if !crate::session::session_authority_cutover_is_complete(store, &project_id)? {
        return Err(LeyCoreError::InvalidLearningStore(
            "native learning authority requires native session authority first".to_owned(),
        ));
    }
    fence_legacy_learning_writes(project_start, legacy_vault)?;
    let imported = crate::continuity_import::import_legacy_learning_continuity(
        project_start,
        legacy_vault,
        store,
    )?;
    store.append_event(&learning_authority_cutover_input(&imported))?;
    if !learning_authority_cutover_is_complete(store, &project_id)? {
        return Err(LeyCoreError::InvalidContinuityStore(
            "native learning-authority cutover did not commit a valid marker".to_owned(),
        ));
    }
    Ok(project_id)
}

fn learning_event_from_continuity(
    event: crate::ContinuityEvent,
) -> Result<LearningEvent, LeyCoreError> {
    let decoded: LearningEvent =
        serde_json::from_value(event.payload.clone()).map_err(|error| {
            LeyCoreError::InvalidContinuityStore(format!(
                "continuity learning event {} has invalid embedded payload: {error}",
                event.event_id
            ))
        })?;
    validate_event(&decoded, &event.project_id)?;
    let expected_suffix = match &decoded.payload {
        LearningEventPayload::Proposed { .. } => "learning-proposed",
        LearningEventPayload::Corrected { .. } => "learning-corrected",
        LearningEventPayload::Reviewed { .. } => "learning-reviewed",
    };
    let legacy_kind = format!("legacy-{expected_suffix}");
    if event.event_id != decoded.event_id
        || event.subject_id.as_deref() != Some(decoded.learning_id.as_str())
        || event.session_id.is_some()
        || event.session_sequence.is_some()
        || event.payload_version != 1
        || event.recorded_at_unix_ms != decoded.recorded_at_unix_ms
        || event.revision_head.is_some()
        || event.revision_branch.is_some()
        || (event.kind != legacy_kind && event.kind != expected_suffix)
    {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "continuity learning event {} does not match its embedded learning event",
            event.event_id
        )));
    }
    match (&event.request_id, &event.request_fingerprint) {
        (None, None) => {}
        (Some(request_id), Some(request_fingerprint))
            if request_id == &decoded.request_id
                && request_fingerprint == &decoded.request_fingerprint => {}
        _ => {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "continuity learning event {} has inconsistent request identity",
                event.event_id
            )))
        }
    }
    Ok(decoded)
}

pub fn learning_review_inbox(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
) -> Result<Vec<LearningSummary>, LeyCoreError> {
    Ok(list_learnings(project_start, vault)?
        .into_iter()
        .filter(summary_needs_review)
        .collect())
}

pub fn learning_review_inbox_with_continuity_transition(
    project_start: impl AsRef<Path>,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<Vec<LearningSummary>, LeyCoreError> {
    Ok(
        list_learnings_with_continuity_transition(project_start, legacy_vault, store)?
            .into_iter()
            .filter(summary_needs_review)
            .collect(),
    )
}

fn summary_needs_review(learning: &LearningSummary) -> bool {
    matches!(
        learning.trust_state,
        LearningTrustState::ReviewRequired
            | LearningTrustState::Contested
            | LearningTrustState::Stale
    ) || (learning.trust_state == LearningTrustState::Trusted
        && learning.freshness == LearningFreshness::SourceChanged)
}

fn record_needs_review(learning: &LearningRecord) -> bool {
    matches!(
        learning.trust_state,
        LearningTrustState::ReviewRequired
            | LearningTrustState::Contested
            | LearningTrustState::Stale
    ) || (learning.trust_state == LearningTrustState::Trusted
        && learning.freshness == LearningFreshness::SourceChanged)
}

impl From<&LearningRecord> for LearningSummary {
    fn from(learning: &LearningRecord) -> Self {
        Self {
            project_id: learning.project_id.clone(),
            learning_id: learning.learning_id.clone(),
            kind: learning.kind,
            title: learning.title.clone(),
            guidance_excerpt: text_excerpt(&learning.guidance, 512),
            state: learning.state,
            trust_state: learning.trust_state,
            provenance: learning.provenance,
            origin_lineage_summary: LearningOriginSummary::from(&learning.origin_lineage),
            confidence_percent: learning.confidence_percent,
            freshness: learning.freshness,
            corroborating_sessions: learning.corroborating_sessions,
            updated_at_unix_ms: learning.updated_at_unix_ms,
            event_count: learning.event_count,
            superseded_by: learning.superseded_by.clone(),
        }
    }
}

fn text_excerpt(value: &str, maximum: usize) -> String {
    let mut characters = value.chars();
    let mut output = characters.by_ref().take(maximum).collect::<String>();
    if characters.next().is_some() {
        output.pop();
        output.push('…');
    }
    output
}

fn validate_review_authority(
    actor: LearningActor,
    action: LearningFeedbackAction,
) -> Result<(), LeyCoreError> {
    if actor == LearningActor::Agent
        && matches!(
            action,
            LearningFeedbackAction::Confirm
                | LearningFeedbackAction::Reject
                | LearningFeedbackAction::Supersede
        )
    {
        return Err(LeyCoreError::InvalidLearningRequest(
            "only an explicit user action can confirm, reject, or supersede a learning".to_owned(),
        ));
    }
    Ok(())
}

fn validate_provenance_authority(
    actor: LearningActor,
    provenance: LearningProvenance,
) -> Result<(), LeyCoreError> {
    let valid = matches!(
        (actor, provenance),
        (LearningActor::User, LearningProvenance::UserAuthored)
            | (
                LearningActor::Agent,
                LearningProvenance::AgentAuthored | LearningProvenance::Inferred
            )
    );
    if !valid {
        return Err(LeyCoreError::InvalidLearningRequest(
            "actor and provenance do not describe the same authoring authority".to_owned(),
        ));
    }
    Ok(())
}

struct LearningOriginBuilder {
    seen: BTreeSet<LearningOriginSource>,
    sources: Vec<LearningOriginSource>,
    omitted_sources: usize,
}

impl LearningOriginBuilder {
    fn new() -> Self {
        Self {
            seen: BTreeSet::new(),
            sources: Vec::new(),
            omitted_sources: 0,
        }
    }

    fn push(&mut self, source: LearningOriginSource) {
        if !self.seen.insert(source.clone()) {
            return;
        }
        if self.sources.len() < MAX_LEARNING_ORIGIN_SOURCES {
            self.sources.push(source);
        } else {
            self.omitted_sources = self.omitted_sources.saturating_add(1);
        }
    }

    fn finish(mut self, source_mechanically_resolved: bool) -> LearningOriginLineage {
        self.sources.sort();
        LearningOriginLineage {
            mechanically_resolved: source_mechanically_resolved && self.omitted_sources == 0,
            causal_completeness_proven: false,
            omitted_sources: self.omitted_sources,
            automatic_authority_ceiling: LearningTrustState::ReviewRequired,
            sources: self.sources,
        }
    }
}

fn resolve_evidence(
    project: &Path,
    vault: &Path,
    transition_store: Option<&ContinuityStore>,
    inputs: Vec<LearningEvidenceInput>,
    redactions: &mut Vec<LearningRedaction>,
) -> Result<(Vec<LearningEvidence>, LearningOriginLineage), LeyCoreError> {
    if inputs.is_empty() || inputs.len() > 20 {
        return Err(LeyCoreError::InvalidLearningRequest(
            "learning evidence must contain 1 to 20 session references".to_owned(),
        ));
    }
    let mut unique = BTreeSet::new();
    let mut evidence = Vec::new();
    let mut lineage = LearningOriginBuilder::new();
    for (index, input) in inputs.into_iter().enumerate() {
        if !unique.insert((input.session_id.clone(), input.record_id.clone())) {
            return Err(LeyCoreError::InvalidLearningRequest(
                "learning evidence contains a duplicate session record".to_owned(),
            ));
        }
        let session = match transition_store {
            Some(store) => {
                read_session_with_continuity_transition(project, vault, store, &input.session_id)?
            }
            None => read_session(project, vault, &input.session_id)?,
        };
        let located = locate_session_record(&session, &input.record_id)?;
        if located.direct_turn_evidence {
            lineage.push(LearningOriginSource::TurnEvidence {
                session_id: session.session_id.clone(),
                record_id: input.record_id.clone(),
            });
        } else {
            lineage.push(LearningOriginSource::SessionRecord {
                session_id: session.session_id.clone(),
                record_id: input.record_id.clone(),
                record_type: located.record_type.clone(),
            });
        }
        for artifact in &located.artifacts {
            lineage.push(LearningOriginSource::CapturedArtifact {
                artifact_snapshot_id: artifact.artifact_snapshot_id.clone(),
                artifact_path: artifact.artifact_path.clone(),
                content_hash: artifact.content_hash.clone(),
            });
        }
        if let Some(checkpoint_event_id) = located.checkpoint_event_id {
            let recovery = match transition_store {
                Some(store) => read_recovery_derivation_origin_with_continuity_transition(
                    project,
                    vault,
                    store,
                    &session.session_id,
                    &checkpoint_event_id,
                    &input.record_id,
                )?,
                None => read_recovery_derivation_origin(
                    project,
                    vault,
                    &session.session_id,
                    &checkpoint_event_id,
                    &input.record_id,
                )?,
            };
            if let Some(recovery) = recovery {
                lineage.push(LearningOriginSource::RecoveryCandidate {
                    session_id: session.session_id.clone(),
                    candidate_fingerprint: recovery.candidate_fingerprint,
                });
                for record_id in recovery.evidence_record_ids {
                    if record_id.starts_with("tev_") {
                        lineage.push(LearningOriginSource::TurnEvidence {
                            session_id: session.session_id.clone(),
                            record_id,
                        });
                    } else if record_id.starts_with("toe_") {
                        lineage.push(LearningOriginSource::ToolEvidence {
                            session_id: session.session_id.clone(),
                            record_id,
                        });
                    } else {
                        return Err(LeyCoreError::InvalidLearningRequest(
                            "recovery origin contains an unsupported evidence namespace".to_owned(),
                        ));
                    }
                }
            }
        }
        evidence.push(LearningEvidence {
            session_id: session.session_id,
            record_id: input.record_id,
            record_type: located.record_type,
            session_status: session.status,
            session_updated_at_unix_ms: session.updated_at_unix_ms,
            note: sanitize_text(
                &format!("evidence[{index}].note"),
                &input.note,
                0,
                2_000,
                redactions,
            )?,
            artifacts: located.artifacts,
        });
    }
    evidence.sort_by(|left, right| {
        left.session_id
            .cmp(&right.session_id)
            .then_with(|| left.record_id.cmp(&right.record_id))
    });
    Ok((evidence, lineage.finish(true)))
}

fn validate_learning_project_memory(
    project: &Path,
    legacy_vault: &Path,
    transition_store: Option<&ContinuityStore>,
) -> Result<(), LeyCoreError> {
    if let Some(store) = transition_store {
        let project_id = diagnose_project(project)?.identity.project_id;
        if store.current_artifact_snapshot(&project_id)?.is_some() {
            return Ok(());
        }
    }
    validate_project_memory(project, legacy_vault)
}

struct LocatedLearningEvidence {
    record_type: String,
    artifacts: Vec<SessionArtifactCitation>,
    checkpoint_event_id: Option<String>,
    direct_turn_evidence: bool,
}

fn locate_session_record(
    session: &crate::AgentSession,
    record_id: &str,
) -> Result<LocatedLearningEvidence, LeyCoreError> {
    if let Some(turn) = session
        .prompts
        .iter()
        .find(|turn| turn.record_id == record_id)
    {
        if turn.retention != crate::TurnEvidenceRetention::Captured || turn.text.is_none() {
            return Err(LeyCoreError::InvalidLearningRequest(
                "learning evidence cannot cite body-free user turn evidence".to_owned(),
            ));
        }
        return Ok(LocatedLearningEvidence {
            record_type: "turn-user-prompt".to_owned(),
            artifacts: Vec::new(),
            checkpoint_event_id: None,
            direct_turn_evidence: true,
        });
    }
    if let Some(turn) = session
        .responses
        .iter()
        .find(|turn| turn.record_id == record_id)
    {
        if turn.retention != crate::TurnEvidenceRetention::Captured || turn.text.is_none() {
            return Err(LeyCoreError::InvalidLearningRequest(
                "learning evidence cannot cite body-free assistant turn evidence".to_owned(),
            ));
        }
        return Ok(LocatedLearningEvidence {
            record_type: "turn-assistant-response".to_owned(),
            artifacts: Vec::new(),
            checkpoint_event_id: None,
            direct_turn_evidence: true,
        });
    }
    if record_id == session.session_id {
        return Ok(LocatedLearningEvidence {
            record_type: "session".to_owned(),
            artifacts: Vec::new(),
            checkpoint_event_id: None,
            direct_turn_evidence: false,
        });
    }
    if session
        .finish
        .as_ref()
        .is_some_and(|finish| finish.event_id == record_id)
    {
        return Ok(LocatedLearningEvidence {
            record_type: "session-finish".to_owned(),
            artifacts: Vec::new(),
            checkpoint_event_id: None,
            direct_turn_evidence: false,
        });
    }
    for checkpoint in &session.checkpoints {
        let record_type = if checkpoint.id == record_id {
            Some("checkpoint")
        } else if checkpoint.plan.iter().any(|record| record.id == record_id) {
            Some("plan-item")
        } else if checkpoint
            .decisions
            .iter()
            .any(|record| record.id == record_id)
        {
            Some("decision")
        } else if checkpoint.tasks.iter().any(|record| record.id == record_id) {
            Some("task")
        } else if checkpoint
            .commands
            .iter()
            .any(|record| record.id == record_id)
        {
            Some("command")
        } else if checkpoint
            .verification
            .iter()
            .any(|record| record.id == record_id)
        {
            Some("verification")
        } else if checkpoint.unresolved.iter().enumerate().any(|(index, _)| {
            crate::session::unresolved_record_id(&checkpoint.event_id, index) == record_id
        }) {
            Some("unresolved")
        } else {
            checkpoint.problems.iter().find_map(|problem| {
                if problem.id == record_id {
                    Some("problem")
                } else if problem
                    .attempts
                    .iter()
                    .any(|attempt| attempt.id == record_id)
                {
                    Some("attempt")
                } else if problem
                    .resolution
                    .as_ref()
                    .is_some_and(|resolution| resolution.id == record_id)
                {
                    Some("resolution")
                } else {
                    None
                }
            })
        };
        if let Some(record_type) = record_type {
            return Ok(LocatedLearningEvidence {
                record_type: record_type.to_owned(),
                artifacts: checkpoint.touched_artifacts.clone(),
                checkpoint_event_id: Some(checkpoint.event_id.clone()),
                direct_turn_evidence: false,
            });
        }
    }
    Err(LeyCoreError::InvalidLearningRequest(format!(
        "session {} does not contain evidence record {record_id}",
        session.session_id
    )))
}

struct PendingLearningEvent {
    event_id: String,
    request_id: String,
    expected_event_count: Option<u64>,
    expected_replacement_event_count: Option<u64>,
    redactions: Vec<LearningRedaction>,
    payload: LearningEventPayload,
    allow_create: bool,
}

enum PreparedLearningMutation {
    Replay { event_id: String },
    Append(LearningEvent),
}

fn prepare_learning_mutation(
    project_id: &str,
    learning_id: &str,
    pending: PendingLearningEvent,
    events: &[LearningEvent],
) -> Result<PreparedLearningMutation, LeyCoreError> {
    let fingerprint = request_fingerprint(
        project_id,
        learning_id,
        &pending.request_id,
        &pending.payload,
    )?;
    if let Some(event) = events
        .iter()
        .find(|event| event.event_id == pending.event_id)
    {
        validate_event(event, project_id)?;
        if event.learning_id != learning_id || event.request_fingerprint != fingerprint {
            return Err(LeyCoreError::LearningIdempotencyConflict(
                pending.request_id,
            ));
        }
        return Ok(PreparedLearningMutation::Replay {
            event_id: event.event_id.clone(),
        });
    }
    if events
        .iter()
        .any(|event| event.learning_id == learning_id && event.request_id == pending.request_id)
    {
        return Err(LeyCoreError::LearningIdempotencyConflict(
            pending.request_id,
        ));
    }
    let learning_events = events
        .iter()
        .filter(|event| event.learning_id == learning_id)
        .collect::<Vec<_>>();
    if let Some(expected) = pending.expected_event_count {
        let actual = learning_events.len() as u64;
        if actual != expected {
            return Err(LeyCoreError::InvalidLearningRequest(format!(
                "learning changed from {expected} events to {actual}; reload before saving"
            )));
        }
    }
    if pending.allow_create && !learning_events.is_empty() {
        return Err(LeyCoreError::LearningIdempotencyConflict(
            pending.request_id,
        ));
    }
    if !pending.allow_create && learning_events.is_empty() {
        return Err(LeyCoreError::LearningNotFound(learning_id.to_owned()));
    }
    if !learning_events.is_empty() {
        let current = replay_one(events, project_id, learning_id)?;
        validate_transition(&current, &pending.payload)?;
        validate_pending_against_ledger(
            events,
            project_id,
            learning_id,
            pending.expected_replacement_event_count,
            &pending.payload,
        )?;
    }
    if events.len() >= LEARNING_EVENT_LIMIT {
        return Err(LeyCoreError::InvalidLearningStore(format!(
            "project exceeds {LEARNING_EVENT_LIMIT} learning events"
        )));
    }
    let sequence = learning_events.len() as u64 + 1;
    let minimum_time = learning_events
        .iter()
        .map(|event| event.recorded_at_unix_ms)
        .max()
        .unwrap_or(1);
    let recorded_at_unix_ms = unix_time_ms().max(minimum_time);
    Ok(PreparedLearningMutation::Append(LearningEvent {
        schema_version: LEARNING_SCHEMA_VERSION,
        event_id: pending.event_id,
        project_id: project_id.to_owned(),
        learning_id: learning_id.to_owned(),
        request_id: pending.request_id,
        request_fingerprint: fingerprint,
        sequence,
        recorded_at_unix_ms,
        redactions: pending.redactions,
        payload: pending.payload,
    }))
}

fn mutate_learning(
    project_id: &str,
    project_root: &Path,
    learning_id: &str,
    pending: PendingLearningEvent,
    vault: impl AsRef<Path>,
) -> Result<LearningMutation, LeyCoreError> {
    let store = LearningStore::open(&vault, project_id, true)?
        .expect("creating a learning store always returns a store");
    let _lock = store.lock(false)?;
    let events = store.read_events()?;
    let (event_id, replayed) =
        match prepare_learning_mutation(project_id, learning_id, pending, &events)? {
            PreparedLearningMutation::Replay { event_id } => (event_id, true),
            PreparedLearningMutation::Append(event) => {
                let event_id = event.event_id.clone();
                let event_name = format!("{event_id}.json");
                let body = json_body(&event, LEARNING_EVENT_LIMIT_BYTES, &event_name)?;
                write_immutable_private(&store.events_dir, &event_name, &body)?;
                (event_id, false)
            }
        };
    let events = store.read_events()?;
    let mut records = replay_all(&events, project_id)?;
    refresh_all_freshness(&mut records, project_root, &vault)?;
    store.persist_index(&records)?;
    let learning = records
        .into_iter()
        .find(|record| record.learning_id == learning_id)
        .ok_or_else(|| LeyCoreError::LearningNotFound(learning_id.to_owned()))?;
    Ok(learning_mutation(learning, &event_id, replayed))
}

fn mutate_learning_in_continuity_store(
    project_root: &Path,
    legacy_vault: &Path,
    store: &ContinuityStore,
    identity: &crate::ProjectIdentity,
    learning_id: &str,
    pending: PendingLearningEvent,
) -> Result<LearningWriteResult, LeyCoreError> {
    crate::validate_project_id(&identity.project_id)?;
    validate_learning_id(learning_id)?;
    validate_learning_project_memory(project_root, legacy_vault, Some(store))?;
    store.register_project(identity)?;

    let mut session_anchors = BTreeMap::new();
    for session_id in learning_payload_session_ids(&pending.payload) {
        let anchor = store
            .events_for_session(&identity.project_id, &session_id)?
            .into_iter()
            .find(|event| {
                matches!(
                    event.kind.as_str(),
                    "legacy-session-started" | "session-started"
                )
            })
            .ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "native learning evidence cites missing session {session_id}"
                ))
            })?;
        session_anchors.insert(session_id, anchor.event_id);
    }

    let (write, snapshot) =
        store.append_learning_event_transactional(&identity.project_id, |existing| {
            let decoded = existing
                .iter()
                .cloned()
                .map(learning_event_from_continuity)
                .collect::<Result<Vec<_>, _>>()?;
            let prepared =
                prepare_learning_mutation(&identity.project_id, learning_id, pending, &decoded)?;
            let (input, event) = match prepared {
                PreparedLearningMutation::Replay { event_id } => {
                    let stored = existing
                        .iter()
                        .find(|event| event.event_id == event_id)
                        .ok_or_else(|| {
                            LeyCoreError::InvalidContinuityStore(format!(
                                "replayed learning event {event_id} is missing from continuity storage"
                            ))
                        })?;
                    let decoded = decoded
                        .iter()
                        .find(|event| event.event_id == event_id)
                        .cloned()
                        .ok_or_else(|| {
                            LeyCoreError::InvalidContinuityStore(format!(
                                "replayed learning event {event_id} could not be decoded"
                            ))
                        })?;
                    (continuity_learning_event_input(stored), decoded)
                }
                PreparedLearningMutation::Append(event) => {
                    let input = native_learning_continuity_event_input(event.clone())?;
                    (input, event)
                }
            };
            let links = native_learning_event_links(&event, &decoded, &session_anchors)?;
            Ok((input, links))
        })?;
    let events = snapshot
        .into_iter()
        .map(learning_event_from_continuity)
        .collect::<Result<Vec<_>, _>>()?;
    let mut records = replay_all(&events, &identity.project_id)?;
    refresh_all_freshness_with_continuity_transition(
        &mut records,
        project_root,
        legacy_vault,
        store,
    )?;
    let learning = records
        .into_iter()
        .find(|record| record.learning_id == learning_id)
        .ok_or_else(|| LeyCoreError::LearningNotFound(learning_id.to_owned()))?;
    Ok(LearningWriteResult {
        learning,
        event_id: write.record.event_id,
        replayed: !write.created,
    })
}

fn learning_payload_session_ids(payload: &LearningEventPayload) -> BTreeSet<String> {
    match payload {
        LearningEventPayload::Proposed { evidence, .. }
        | LearningEventPayload::Corrected { evidence, .. } => evidence
            .iter()
            .map(|item| item.session_id.clone())
            .collect(),
        LearningEventPayload::Reviewed { .. } => BTreeSet::new(),
    }
}

fn continuity_learning_event_input(event: &crate::ContinuityEvent) -> crate::ContinuityEventInput {
    crate::ContinuityEventInput {
        event_id: event.event_id.clone(),
        project_id: event.project_id.clone(),
        subject_id: event.subject_id.clone(),
        session_id: event.session_id.clone(),
        session_sequence: event.session_sequence,
        request_id: event.request_id.clone(),
        request_fingerprint: event.request_fingerprint.clone(),
        kind: event.kind.clone(),
        payload_version: event.payload_version,
        recorded_at_unix_ms: event.recorded_at_unix_ms,
        revision_head: event.revision_head.clone(),
        revision_branch: event.revision_branch.clone(),
        payload: event.payload.clone(),
    }
}

fn native_learning_continuity_event_input(
    event: LearningEvent,
) -> Result<crate::ContinuityEventInput, LeyCoreError> {
    let kind = match &event.payload {
        LearningEventPayload::Proposed { .. } => "learning-proposed",
        LearningEventPayload::Corrected { .. } => "learning-corrected",
        LearningEventPayload::Reviewed { .. } => "learning-reviewed",
    }
    .to_owned();
    let payload = serde_json::to_value(&event).map_err(|error| {
        LeyCoreError::InvalidContinuityStore(format!(
            "native learning event could not be serialized: {error}"
        ))
    })?;
    Ok(crate::ContinuityEventInput {
        event_id: event.event_id,
        project_id: event.project_id,
        subject_id: Some(event.learning_id),
        session_id: None,
        session_sequence: None,
        request_id: None,
        request_fingerprint: None,
        kind,
        payload_version: 1,
        recorded_at_unix_ms: event.recorded_at_unix_ms,
        revision_head: None,
        revision_branch: None,
        payload,
    })
}

fn native_learning_event_links(
    event: &LearningEvent,
    existing: &[LearningEvent],
    session_anchors: &BTreeMap<String, String>,
) -> Result<Vec<ContinuityEventLinkInput>, LeyCoreError> {
    let mut links = BTreeSet::<(String, String, String)>::new();
    match &event.payload {
        LearningEventPayload::Proposed { evidence, .. }
        | LearningEventPayload::Corrected { evidence, .. } => {
            for evidence in evidence {
                let target = session_anchors.get(&evidence.session_id).ok_or_else(|| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "native learning event {} cites missing session {}",
                        event.event_id, evidence.session_id
                    ))
                })?;
                links.insert((
                    event.event_id.clone(),
                    target.clone(),
                    "depends-on-session".to_owned(),
                ));
            }
        }
        LearningEventPayload::Reviewed {
            action: LearningFeedbackAction::Supersede,
            replacement_learning_id: Some(replacement),
            ..
        } => {
            let target = existing
                .iter()
                .filter(|candidate| candidate.learning_id == *replacement)
                .min_by_key(|candidate| candidate.sequence)
                .map(|candidate| candidate.event_id.clone())
                .ok_or_else(|| {
                    LeyCoreError::InvalidLearningRequest(
                        "replacement learning does not exist in this project".to_owned(),
                    )
                })?;
            links.insert((event.event_id.clone(), target, "supersedes".to_owned()));
        }
        LearningEventPayload::Reviewed { .. } => {}
    }
    Ok(links
        .into_iter()
        .map(
            |(from_event_id, to_event_id, relation)| ContinuityEventLinkInput {
                from_event_id,
                to_event_id,
                relation,
            },
        )
        .collect())
}

fn validate_pending_against_ledger(
    events: &[LearningEvent],
    project_id: &str,
    learning_id: &str,
    expected_replacement_event_count: Option<u64>,
    payload: &LearningEventPayload,
) -> Result<(), LeyCoreError> {
    let LearningEventPayload::Reviewed {
        action: LearningFeedbackAction::Supersede,
        replacement_learning_id: Some(replacement),
        ..
    } = payload
    else {
        return Ok(());
    };
    let records = replay_all(events, project_id)?;
    let replacement_record = records
        .iter()
        .find(|record| record.learning_id == *replacement)
        .ok_or_else(|| {
            LeyCoreError::InvalidLearningRequest(
                "replacement learning does not exist in this project".to_owned(),
            )
        })?;
    if matches!(
        replacement_record.state,
        LearningState::Rejected | LearningState::Superseded
    ) {
        return Err(LeyCoreError::InvalidLearningRequest(format!(
            "replacement learning {} is already {}",
            replacement_record.learning_id,
            state_label(replacement_record.state)
        )));
    }
    if let Some(expected) = expected_replacement_event_count {
        let actual = replacement_record.event_count;
        if actual != expected {
            return Err(LeyCoreError::InvalidLearningRequest(format!(
                "replacement learning changed from {expected} events to {actual}; reload before saving"
            )));
        }
    }
    let replacements = records
        .iter()
        .map(|record| (record.learning_id.as_str(), record.superseded_by.as_deref()))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::from([learning_id]);
    let mut next = Some(replacement.as_str());
    while let Some(candidate) = next {
        if !seen.insert(candidate) {
            return Err(LeyCoreError::InvalidLearningRequest(
                "supersession would create a learning cycle".to_owned(),
            ));
        }
        next = *replacements.get(candidate).ok_or_else(|| {
            LeyCoreError::InvalidLearningRequest(
                "replacement learning does not exist in this project".to_owned(),
            )
        })?;
    }
    Ok(())
}

fn learning_mutation(learning: LearningRecord, event_id: &str, replayed: bool) -> LearningMutation {
    let base = format!(
        "{STORE_ROOT}/{AGENT_MEMORY_DIRECTORY}/{PROJECTS_DIRECTORY}/{}/{LEARNINGS_DIRECTORY}",
        learning.project_id
    );
    LearningMutation {
        learning,
        event_id: event_id.to_owned(),
        replayed,
        index_path: format!("{base}/{LEARNING_INDEX_FILE}"),
        review_path: format!("{base}/{LEARNING_REVIEW_FILE}"),
    }
}

fn validate_transition(
    current: &LearningRecord,
    payload: &LearningEventPayload,
) -> Result<(), LeyCoreError> {
    if matches!(
        current.state,
        LearningState::Rejected | LearningState::Superseded
    ) {
        return Err(LeyCoreError::InvalidLearningRequest(format!(
            "learning {} is already {}",
            current.learning_id,
            state_label(current.state)
        )));
    }
    if matches!(payload, LearningEventPayload::Proposed { .. }) {
        return Err(LeyCoreError::InvalidLearningRequest(
            "a learning can only be proposed once".to_owned(),
        ));
    }
    Ok(())
}

fn replay_all(
    events: &[LearningEvent],
    project_id: &str,
) -> Result<Vec<LearningRecord>, LeyCoreError> {
    let mut grouped = BTreeMap::<String, Vec<LearningEvent>>::new();
    for event in events {
        grouped
            .entry(event.learning_id.clone())
            .or_default()
            .push(event.clone());
    }
    let records = grouped
        .into_iter()
        .map(|(learning_id, events)| replay_learning_events(&events, project_id, &learning_id))
        .collect::<Result<Vec<_>, _>>()?;
    let replacements = records
        .iter()
        .map(|record| (record.learning_id.as_str(), record.superseded_by.as_deref()))
        .collect::<BTreeMap<_, _>>();
    for record in &records {
        let mut seen = BTreeSet::from([record.learning_id.as_str()]);
        let mut next = record.superseded_by.as_deref();
        while let Some(replacement) = next {
            if !seen.insert(replacement) {
                return Err(LeyCoreError::InvalidLearningStore(
                    "learning supersession graph contains a cycle".to_owned(),
                ));
            }
            next = *replacements.get(replacement).ok_or_else(|| {
                LeyCoreError::InvalidLearningStore(format!(
                    "learning {} references a missing replacement",
                    record.learning_id
                ))
            })?;
        }
    }
    Ok(records)
}

fn replay_one(
    events: &[LearningEvent],
    project_id: &str,
    learning_id: &str,
) -> Result<LearningRecord, LeyCoreError> {
    let events = events
        .iter()
        .filter(|event| event.learning_id == learning_id)
        .cloned()
        .collect::<Vec<_>>();
    if events.is_empty() {
        return Err(LeyCoreError::LearningNotFound(learning_id.to_owned()));
    }
    replay_learning_events(&events, project_id, learning_id)
}

fn legacy_origin_lineage(evidence: &[LearningEvidence]) -> LearningOriginLineage {
    let mut builder = LearningOriginBuilder::new();
    for item in evidence {
        builder.push(LearningOriginSource::SessionRecord {
            session_id: item.session_id.clone(),
            record_id: item.record_id.clone(),
            record_type: item.record_type.clone(),
        });
        for artifact in &item.artifacts {
            builder.push(LearningOriginSource::CapturedArtifact {
                artifact_snapshot_id: artifact.artifact_snapshot_id.clone(),
                artifact_path: artifact.artifact_path.clone(),
                content_hash: artifact.content_hash.clone(),
            });
        }
    }
    builder.finish(false)
}

fn event_origin_lineage(
    origin_lineage: &Option<LearningOriginLineage>,
    evidence: &[LearningEvidence],
) -> LearningOriginLineage {
    origin_lineage
        .clone()
        .unwrap_or_else(|| legacy_origin_lineage(evidence))
}

fn merge_origin_lineage(
    current: &LearningOriginLineage,
    next: LearningOriginLineage,
) -> LearningOriginLineage {
    let mut builder = LearningOriginBuilder::new();
    for source in current.sources.iter().chain(next.sources.iter()) {
        builder.push(source.clone());
    }
    let inherited_omissions = current.omitted_sources.saturating_add(next.omitted_sources);
    let mut merged = builder.finish(current.mechanically_resolved && next.mechanically_resolved);
    merged.omitted_sources = merged.omitted_sources.saturating_add(inherited_omissions);
    if merged.omitted_sources > 0 {
        merged.mechanically_resolved = false;
    }
    merged
}

fn replay_learning_events(
    events: &[LearningEvent],
    project_id: &str,
    learning_id: &str,
) -> Result<LearningRecord, LeyCoreError> {
    let mut events = events.to_vec();
    events.sort_by_key(|event| event.sequence);
    for (index, event) in events.iter().enumerate() {
        if event.sequence != index as u64 + 1 {
            return Err(LeyCoreError::InvalidLearningStore(
                "learning event sequence is not contiguous".to_owned(),
            ));
        }
        if index > 0 && event.recorded_at_unix_ms < events[index - 1].recorded_at_unix_ms {
            return Err(LeyCoreError::InvalidLearningStore(
                "learning event timestamps are not monotonic".to_owned(),
            ));
        }
    }
    let first = &events[0];
    let LearningEventPayload::Proposed {
        actor,
        kind,
        title,
        guidance,
        confidence_percent,
        provenance,
        evidence,
        origin_lineage,
    } = &first.payload
    else {
        return Err(LeyCoreError::InvalidLearningStore(
            "the first learning event must be a proposal".to_owned(),
        ));
    };
    let mut learning = LearningRecord {
        schema_version: LEARNING_SCHEMA_VERSION,
        project_id: project_id.to_owned(),
        learning_id: learning_id.to_owned(),
        kind: *kind,
        title: title.clone(),
        guidance: guidance.clone(),
        state: LearningState::Tentative,
        trust_state: LearningTrustState::ReviewRequired,
        provenance: *provenance,
        origin_lineage: event_origin_lineage(origin_lineage, evidence),
        confidence_percent: *confidence_percent,
        freshness: LearningFreshness::Uncited,
        corroborating_sessions: corroboration_count(evidence),
        created_at_unix_ms: first.recorded_at_unix_ms,
        updated_at_unix_ms: first.recorded_at_unix_ms,
        valid_from_unix_ms: first.recorded_at_unix_ms,
        valid_until_unix_ms: None,
        evidence: evidence.clone(),
        history: vec![LearningReviewEntry {
            event_id: first.event_id.clone(),
            recorded_at_unix_ms: first.recorded_at_unix_ms,
            actor: *actor,
            action: "proposed".to_owned(),
            note: String::new(),
        }],
        superseded_by: None,
        event_count: events.len() as u64,
    };
    for event in &events[1..] {
        match &event.payload {
            LearningEventPayload::Proposed { .. } => {
                return Err(LeyCoreError::InvalidLearningStore(
                    "a learning can only be proposed once".to_owned(),
                ))
            }
            LearningEventPayload::Corrected {
                actor,
                title,
                guidance,
                confidence_percent,
                evidence,
                note,
                origin_lineage,
            } => {
                learning.title = title.clone();
                learning.guidance = guidance.clone();
                learning.confidence_percent = *confidence_percent;
                learning.evidence = evidence.clone();
                learning.corroborating_sessions = corroboration_count(evidence);
                learning.origin_lineage = merge_origin_lineage(
                    &learning.origin_lineage,
                    event_origin_lineage(origin_lineage, evidence),
                );
                learning.state = LearningState::Tentative;
                learning.trust_state = LearningTrustState::ReviewRequired;
                learning.valid_from_unix_ms = event.recorded_at_unix_ms;
                learning.valid_until_unix_ms = None;
                learning.superseded_by = None;
                learning.history.push(LearningReviewEntry {
                    event_id: event.event_id.clone(),
                    recorded_at_unix_ms: event.recorded_at_unix_ms,
                    actor: *actor,
                    action: "corrected".to_owned(),
                    note: note.clone(),
                });
            }
            LearningEventPayload::Reviewed {
                actor,
                action,
                note,
                replacement_learning_id,
            } => {
                apply_review(
                    &mut learning,
                    *action,
                    replacement_learning_id.clone(),
                    event.recorded_at_unix_ms,
                );
                learning.history.push(LearningReviewEntry {
                    event_id: event.event_id.clone(),
                    recorded_at_unix_ms: event.recorded_at_unix_ms,
                    actor: *actor,
                    action: feedback_label(*action).to_owned(),
                    note: note.clone(),
                });
            }
        }
        learning.updated_at_unix_ms = event.recorded_at_unix_ms;
    }
    Ok(learning)
}

fn apply_review(
    learning: &mut LearningRecord,
    action: LearningFeedbackAction,
    replacement: Option<String>,
    at: u64,
) {
    match action {
        LearningFeedbackAction::Confirm => {
            learning.state = LearningState::Verified;
            learning.trust_state = LearningTrustState::Trusted;
            learning.valid_until_unix_ms = None;
        }
        LearningFeedbackAction::Contest => {
            learning.state = LearningState::Contested;
            learning.trust_state = LearningTrustState::Contested;
            learning.valid_until_unix_ms = None;
        }
        LearningFeedbackAction::Reject => {
            learning.state = LearningState::Rejected;
            learning.trust_state = LearningTrustState::Rejected;
            learning.valid_until_unix_ms = Some(at);
        }
        LearningFeedbackAction::MarkStale => {
            learning.state = LearningState::Stale;
            learning.trust_state = LearningTrustState::Stale;
            learning.valid_until_unix_ms = Some(at);
        }
        LearningFeedbackAction::Supersede => {
            learning.state = LearningState::Superseded;
            learning.trust_state = LearningTrustState::Superseded;
            learning.valid_until_unix_ms = Some(at);
            learning.superseded_by = replacement;
        }
    }
}

fn corroboration_count(evidence: &[LearningEvidence]) -> usize {
    evidence
        .iter()
        .map(|item| item.session_id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn refresh_freshness(
    learning: &mut LearningRecord,
    project: &Path,
    vault: impl AsRef<Path>,
) -> Result<(), LeyCoreError> {
    let memory = load_project_memory(project, vault)?;
    refresh_freshness_from_memory(learning, &memory);
    Ok(())
}

fn refresh_all_freshness(
    learnings: &mut [LearningRecord],
    project: &Path,
    vault: impl AsRef<Path>,
) -> Result<(), LeyCoreError> {
    let memory = load_project_memory(project, vault)?;
    for learning in learnings {
        refresh_freshness_from_memory(learning, &memory);
    }
    Ok(())
}

fn refresh_freshness_with_continuity_transition(
    learning: &mut LearningRecord,
    project: &Path,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<(), LeyCoreError> {
    let project_id = diagnose_project(project)?.identity.project_id;
    if let Some(hashes) = store.current_artifact_hashes(&project_id)? {
        refresh_freshness_from_hashes(learning, &hashes);
        return Ok(());
    }
    refresh_freshness(learning, project, legacy_vault)
}

fn refresh_all_freshness_with_continuity_transition(
    learnings: &mut [LearningRecord],
    project: &Path,
    legacy_vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<(), LeyCoreError> {
    let project_id = diagnose_project(project)?.identity.project_id;
    if let Some(hashes) = store.current_artifact_hashes(&project_id)? {
        for learning in learnings {
            refresh_freshness_from_hashes(learning, &hashes);
        }
        return Ok(());
    }
    refresh_all_freshness(learnings, project, legacy_vault)
}

fn refresh_freshness_from_hashes(learning: &mut LearningRecord, hashes: &BTreeMap<String, String>) {
    let citations = learning
        .evidence
        .iter()
        .flat_map(|evidence| &evidence.artifacts)
        .collect::<Vec<_>>();
    learning.freshness = if citations.is_empty() {
        LearningFreshness::Uncited
    } else if citations.iter().any(|citation| {
        hashes
            .get(&citation.artifact_path)
            .is_none_or(|content_hash| content_hash != &citation.content_hash)
    }) {
        LearningFreshness::SourceChanged
    } else {
        LearningFreshness::Current
    };
}

fn refresh_freshness_from_memory(
    learning: &mut LearningRecord,
    memory: &crate::ingestion::LoadedProjectMemory,
) {
    let citations = learning
        .evidence
        .iter()
        .flat_map(|evidence| &evidence.artifacts)
        .collect::<Vec<_>>();
    learning.freshness = if citations.is_empty() {
        LearningFreshness::Uncited
    } else if citations.iter().any(|citation| {
        memory
            .manifest
            .files
            .iter()
            .find(|artifact| artifact.path == citation.artifact_path)
            .is_none_or(|artifact| artifact.content_hash != citation.content_hash)
    }) {
        LearningFreshness::SourceChanged
    } else {
        LearningFreshness::Current
    };
}

struct LearningStore {
    _lifecycle: ProjectMemoryLifecycleLock,
    project_id: String,
    project_dir: Dir,
    learning_dir: Dir,
    events_dir: Dir,
}

struct LearningLock {
    file: File,
}

impl Drop for LearningLock {
    fn drop(&mut self) {
        let _ = File::unlock(&self.file);
    }
}

impl LearningStore {
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
                    path: vault_path,
                    source,
                }
            })?;
        let ley_dir = open_existing_dir(&vault_dir, STORE_ROOT)?;
        let memory_dir = open_existing_dir(&ley_dir, AGENT_MEMORY_DIRECTORY)?;
        let projects_dir = open_existing_dir(&memory_dir, PROJECTS_DIRECTORY)?;
        let project_dir = open_existing_dir(&projects_dir, project_id)?;
        if create {
            ensure_lock_file(&project_dir)?;
        }
        let learning_dir = if create {
            open_or_create_private_dir(&project_dir, LEARNINGS_DIRECTORY)?
        } else {
            match project_dir.open_dir_nofollow(LEARNINGS_DIRECTORY) {
                Ok(directory) => directory,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(source) => return Err(learning_io(LEARNINGS_DIRECTORY, source)),
            }
        };
        let events_dir = if create {
            open_or_create_private_dir(&learning_dir, EVENTS_DIRECTORY)?
        } else {
            learning_dir
                .open_dir_nofollow(EVENTS_DIRECTORY)
                .map_err(|source| learning_io(EVENTS_DIRECTORY, source))?
        };
        Ok(Some(Self {
            _lifecycle: lifecycle,
            project_id: project_id.to_owned(),
            project_dir,
            learning_dir,
            events_dir,
        }))
    }

    fn lock(&self, shared: bool) -> Result<LearningLock, LeyCoreError> {
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
            .open_with(LEARNING_LOCK_FILE, &options)
            .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?;
        ensure_private_file(&lock, LEARNING_LOCK_FILE)?;
        let file = lock.into_std();
        if shared {
            File::lock_shared(&file).map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?;
        } else {
            file.lock()
                .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?;
        }
        Ok(LearningLock { file })
    }

    fn read_events(&self) -> Result<Vec<LearningEvent>, LeyCoreError> {
        read_learning_events(&self.events_dir, &self.project_id)
    }

    fn persist_index(&self, records: &[LearningRecord]) -> Result<(), LeyCoreError> {
        persist_learning_index(&self.learning_dir, &self.project_id, records)
    }
}

pub(crate) fn erase_learnings_citing_session_under_lifecycle(
    project_dir: &Dir,
    project_id: &str,
    session_id: &str,
) -> Result<Vec<String>, LeyCoreError> {
    let learning_dir = match project_dir.open_dir_nofollow(LEARNINGS_DIRECTORY) {
        Ok(directory) => directory,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(learning_io(LEARNINGS_DIRECTORY, source)),
    };
    let events_dir = learning_dir
        .open_dir_nofollow(EVENTS_DIRECTORY)
        .map_err(|source| learning_io(EVENTS_DIRECTORY, source))?;
    let events = read_learning_events(&events_dir, project_id)?;
    replay_all(&events, project_id)?;
    let mut erased = events
        .iter()
        .filter(|event| learning_event_cites_session(event, session_id))
        .map(|event| event.learning_id.clone())
        .collect::<BTreeSet<_>>();

    loop {
        let before = erased.len();
        for event in &events {
            if let LearningEventPayload::Reviewed {
                action: LearningFeedbackAction::Supersede,
                replacement_learning_id: Some(replacement),
                ..
            } = &event.payload
            {
                if erased.contains(replacement) {
                    erased.insert(event.learning_id.clone());
                }
            }
        }
        if erased.len() == before {
            break;
        }
    }
    if erased.is_empty() {
        return Ok(Vec::new());
    }

    let remaining_events = events
        .iter()
        .filter(|event| !erased.contains(&event.learning_id))
        .cloned()
        .collect::<Vec<_>>();
    let mut remaining_records = replay_all(&remaining_events, project_id)?;
    preserve_projected_freshness(&learning_dir, project_id, &mut remaining_records)?;
    learning_projection_bodies(project_id, &remaining_records)?;

    for event in events
        .iter()
        .filter(|event| erased.contains(&event.learning_id))
    {
        let name = format!("{}.json", event.event_id);
        events_dir
            .remove_file(&name)
            .map_err(|source| learning_io(&name, source))?;
    }
    persist_learning_index(&learning_dir, project_id, &remaining_records)?;
    Ok(erased.into_iter().collect())
}

fn learning_event_cites_session(event: &LearningEvent, session_id: &str) -> bool {
    match &event.payload {
        LearningEventPayload::Proposed { evidence, .. }
        | LearningEventPayload::Corrected { evidence, .. } => {
            evidence.iter().any(|item| item.session_id == session_id)
        }
        LearningEventPayload::Reviewed { .. } => false,
    }
}

fn read_learning_events(
    events_dir: &Dir,
    project_id: &str,
) -> Result<Vec<LearningEvent>, LeyCoreError> {
    let entries = events_dir
        .entries()
        .map_err(|source| learning_io(EVENTS_DIRECTORY, source))?;
    let mut events = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| learning_io(EVENTS_DIRECTORY, source))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(LeyCoreError::InvalidLearningStore(
                "learning event filename is not UTF-8".to_owned(),
            ));
        };
        if !name.ends_with(".json")
            || !entry
                .file_type()
                .map_err(|source| learning_io(name, source))?
                .is_file()
        {
            return Err(LeyCoreError::InvalidLearningStore(format!(
                "unexpected learning event entry: {name}"
            )));
        }
        if events.len() >= LEARNING_EVENT_LIMIT {
            return Err(LeyCoreError::InvalidLearningStore(format!(
                "project exceeds {LEARNING_EVENT_LIMIT} learning events"
            )));
        }
        let bytes =
            read_private_file(events_dir, name, LEARNING_EVENT_LIMIT_BYTES)?.ok_or_else(|| {
                LeyCoreError::InvalidLearningStore(format!(
                    "learning event disappeared while reading: {name}"
                ))
            })?;
        let event: LearningEvent = parse_json(name, &bytes)?;
        validate_event(&event, project_id)?;
        if name != format!("{}.json", event.event_id) {
            return Err(LeyCoreError::InvalidLearningStore(format!(
                "learning event filename does not match its ID: {name}"
            )));
        }
        events.push(event);
    }
    events.sort_by(|left, right| {
        left.learning_id
            .cmp(&right.learning_id)
            .then_with(|| left.sequence.cmp(&right.sequence))
    });
    Ok(events)
}

fn preserve_projected_freshness(
    learning_dir: &Dir,
    project_id: &str,
    records: &mut [LearningRecord],
) -> Result<(), LeyCoreError> {
    let Some(bytes) = read_private_file(
        learning_dir,
        LEARNING_INDEX_FILE,
        LEARNING_INDEX_LIMIT_BYTES,
    )?
    else {
        return Ok(());
    };
    let Ok(index) = serde_json::from_slice::<LearningIndex>(&bytes) else {
        return Ok(());
    };
    if !matches!(
        index.schema_version,
        PREVIOUS_LEARNING_SCHEMA_VERSION | LEARNING_SCHEMA_VERSION
    ) || index.project_id != project_id
    {
        return Ok(());
    }
    let freshness = index
        .learnings
        .into_iter()
        .map(|learning| (learning.learning_id, learning.freshness))
        .collect::<BTreeMap<_, _>>();
    for record in records {
        if let Some(value) = freshness.get(&record.learning_id) {
            record.freshness = *value;
        }
    }
    Ok(())
}

fn learning_projection_bodies(
    project_id: &str,
    records: &[LearningRecord],
) -> Result<(Vec<u8>, Vec<u8>), LeyCoreError> {
    let mut learnings = records.to_vec();
    learnings.sort_by(|left, right| left.learning_id.cmp(&right.learning_id));
    let index = LearningIndex {
        schema_version: LEARNING_SCHEMA_VERSION,
        project_id: project_id.to_owned(),
        generated_at_unix_ms: unix_time_ms(),
        learnings,
    };
    let body = json_body(&index, LEARNING_INDEX_LIMIT_BYTES, LEARNING_INDEX_FILE)?;
    let markdown = render_review_markdown(&index);
    if markdown.len() as u64 > LEARNING_INDEX_LIMIT_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: PathBuf::from(LEARNING_REVIEW_FILE),
            limit_bytes: LEARNING_INDEX_LIMIT_BYTES,
        });
    }
    Ok((body, markdown.into_bytes()))
}

fn persist_learning_index(
    learning_dir: &Dir,
    project_id: &str,
    records: &[LearningRecord],
) -> Result<(), LeyCoreError> {
    let (body, markdown) = learning_projection_bodies(project_id, records)?;
    write_atomic_private(learning_dir, LEARNING_INDEX_FILE, &body)?;
    write_atomic_private(learning_dir, LEARNING_REVIEW_FILE, &markdown)
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
        .open_with(LEARNING_LOCK_FILE, &options)
        .map_err(|source| learning_io(LEARNING_LOCK_FILE, source))?;
    ensure_private_file(&lock, LEARNING_LOCK_FILE)
}

fn render_review_markdown(index: &LearningIndex) -> String {
    let mut output = String::from(
        "---\nleyType: agent-learning-review\nschemaVersion: 1\n---\n\n\
         # Learning review\n\n\
         Agent-authored and inferred learnings remain untrusted until explicitly confirmed.\n",
    );
    let review = index
        .learnings
        .iter()
        .filter(|learning| record_needs_review(learning))
        .collect::<Vec<_>>();
    output.push_str("\n## Review inbox\n\n");
    if review.is_empty() {
        output.push_str("No learnings need review.\n");
    }
    for learning in review {
        output.push_str(&format!(
            "### {}\n\n- ID: `{}`\n- State: `{}`\n- Trust: `{}`\n- Freshness: `{}`\n- Confidence: `{}%`\n- Corroborating sessions: `{}`\n\n> {}\n\n",
            markdown_inline(&learning.title),
            learning.learning_id,
            state_label(learning.state),
            trust_label(learning.trust_state),
            freshness_label(learning.freshness),
            learning.confidence_percent,
            learning.corroborating_sessions,
            markdown_inline(&learning.guidance)
        ));
    }
    output.push_str("## Verified learnings\n\n");
    let verified = index
        .learnings
        .iter()
        .filter(|learning| {
            learning.trust_state == LearningTrustState::Trusted
                && learning.freshness != LearningFreshness::SourceChanged
        })
        .collect::<Vec<_>>();
    if verified.is_empty() {
        output.push_str("No verified learnings.\n");
    }
    for learning in verified {
        output.push_str(&format!(
            "- **{}**: {} `{}`\n",
            markdown_inline(&learning.title),
            markdown_inline(&learning.guidance),
            learning.learning_id
        ));
    }
    output
}

fn validate_event(event: &LearningEvent, project_id: &str) -> Result<(), LeyCoreError> {
    if !matches!(
        event.schema_version,
        LEGACY_LEARNING_SCHEMA_VERSION | PREVIOUS_LEARNING_SCHEMA_VERSION | LEARNING_SCHEMA_VERSION
    ) || event.project_id != project_id
        || event.sequence == 0
        || event.recorded_at_unix_ms == 0
    {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning event identity is invalid".to_owned(),
        ));
    }
    validate_learning_id_store(&event.learning_id)?;
    validate_event_id(&event.event_id)?;
    validate_request_id_store(&event.request_id)?;
    if !is_sha256(&event.request_fingerprint) {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning request fingerprint is invalid".to_owned(),
        ));
    }
    let expected = request_fingerprint(
        project_id,
        &event.learning_id,
        &event.request_id,
        &event.payload,
    )?;
    if expected != event.request_fingerprint {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning request fingerprint does not match its event".to_owned(),
        ));
    }
    let kind = match &event.payload {
        LearningEventPayload::Proposed { .. } => "proposed".to_owned(),
        LearningEventPayload::Corrected { .. } => "corrected".to_owned(),
        LearningEventPayload::Reviewed { action, .. } => {
            format!("reviewed:{}", feedback_label(*action))
        }
    };
    let expected_event = deterministic_id(
        "lev",
        &format!("{}:{}:{kind}", event.learning_id, event.request_id),
        64,
    );
    if event.event_id != expected_event {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning event ID does not match its request".to_owned(),
        ));
    }
    if event.redactions.len() > 2_000
        || event
            .redactions
            .iter()
            .any(|item| item.field.is_empty() || item.kind.is_empty() || item.lines.contains(&0))
    {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning redaction metadata is invalid".to_owned(),
        ));
    }
    validate_event_payload(event.schema_version, &event.payload)?;
    Ok(())
}

fn request_fingerprint(
    project_id: &str,
    learning_id: &str,
    request_id: &str,
    payload: &LearningEventPayload,
) -> Result<String, LeyCoreError> {
    let mut stable = payload.clone();
    match &mut stable {
        LearningEventPayload::Proposed { evidence, .. }
        | LearningEventPayload::Corrected { evidence, .. } => {
            normalize_evidence_for_fingerprint(evidence)
        }
        LearningEventPayload::Reviewed { .. } => {}
    }
    let bytes = serde_json::to_vec(&(project_id, learning_id, request_id, stable))
        .map_err(|error| LeyCoreError::InvalidLearningStore(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn normalize_evidence_for_fingerprint(evidence: &mut [LearningEvidence]) {
    for item in evidence {
        item.session_status = SessionStatus::Active;
        item.session_updated_at_unix_ms = 0;
        for artifact in &mut item.artifacts {
            artifact.artifact_snapshot_id.clear();
            artifact.content_hash.clear();
            artifact.start_line = 0;
            artifact.end_line = 0;
        }
    }
}

fn validate_event_payload(
    schema_version: u32,
    payload: &LearningEventPayload,
) -> Result<(), LeyCoreError> {
    match payload {
        LearningEventPayload::Proposed {
            actor,
            title,
            guidance,
            confidence_percent,
            provenance,
            evidence,
            origin_lineage,
            ..
        } => {
            let valid_authority = matches!(
                (actor, provenance),
                (LearningActor::User, LearningProvenance::UserAuthored)
                    | (
                        LearningActor::Agent,
                        LearningProvenance::AgentAuthored | LearningProvenance::Inferred
                    )
            );
            if !valid_authority {
                return Err(LeyCoreError::InvalidLearningStore(
                    "learning proposal authority is invalid".to_owned(),
                ));
            }
            validate_stored_text("title", title, 1, 256)?;
            validate_stored_text("guidance", guidance, 1, 16_000)?;
            validate_stored_confidence(*confidence_percent)?;
            validate_stored_evidence(evidence)?;
            validate_stored_origin_lineage(schema_version, origin_lineage)?;
        }
        LearningEventPayload::Corrected {
            title,
            guidance,
            confidence_percent,
            evidence,
            note,
            origin_lineage,
            ..
        } => {
            validate_stored_text("title", title, 1, 256)?;
            validate_stored_text("guidance", guidance, 1, 16_000)?;
            validate_stored_text("note", note, 0, 4_000)?;
            validate_stored_confidence(*confidence_percent)?;
            validate_stored_evidence(evidence)?;
            validate_stored_origin_lineage(schema_version, origin_lineage)?;
        }
        LearningEventPayload::Reviewed {
            actor,
            action,
            note,
            replacement_learning_id,
        } => {
            if *actor == LearningActor::Agent
                && matches!(
                    action,
                    LearningFeedbackAction::Confirm
                        | LearningFeedbackAction::Reject
                        | LearningFeedbackAction::Supersede
                )
            {
                return Err(LeyCoreError::InvalidLearningStore(
                    "agent event exceeds learning review authority".to_owned(),
                ));
            }
            validate_stored_text("note", note, 0, 4_000)?;
            if *action == LearningFeedbackAction::Supersede {
                let replacement = replacement_learning_id.as_deref().ok_or_else(|| {
                    LeyCoreError::InvalidLearningStore(
                        "supersede event has no replacement learning".to_owned(),
                    )
                })?;
                validate_learning_id_store(replacement)?;
            } else if replacement_learning_id.is_some() {
                return Err(LeyCoreError::InvalidLearningStore(
                    "non-supersede event contains a replacement learning".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_stored_origin_lineage(
    schema_version: u32,
    lineage: &Option<LearningOriginLineage>,
) -> Result<(), LeyCoreError> {
    if schema_version == LEGACY_LEARNING_SCHEMA_VERSION {
        if lineage.is_some() {
            return Err(LeyCoreError::InvalidLearningStore(
                "legacy learning events cannot contain origin lineage".to_owned(),
            ));
        }
        return Ok(());
    }
    let lineage = lineage.as_ref().ok_or_else(|| {
        LeyCoreError::InvalidLearningStore(
            "current learning proposal/correction is missing origin lineage".to_owned(),
        )
    })?;
    if lineage.sources.is_empty()
        || lineage.sources.len() > MAX_LEARNING_ORIGIN_SOURCES
        || lineage.automatic_authority_ceiling != LearningTrustState::ReviewRequired
        || lineage.causal_completeness_proven
        || (lineage.mechanically_resolved && lineage.omitted_sources != 0)
    {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning origin lineage metadata is invalid".to_owned(),
        ));
    }
    let mut previous: Option<&LearningOriginSource> = None;
    for source in &lineage.sources {
        if previous.is_some_and(|previous| previous >= source) {
            return Err(LeyCoreError::InvalidLearningStore(
                "learning origin lineage must be sorted and unique".to_owned(),
            ));
        }
        validate_origin_source(schema_version, source)?;
        previous = Some(source);
    }
    Ok(())
}

fn validate_origin_source(
    schema_version: u32,
    source: &LearningOriginSource,
) -> Result<(), LeyCoreError> {
    match source {
        LearningOriginSource::SessionRecord {
            session_id,
            record_id,
            record_type,
        } => {
            if !valid_prefixed_hex(session_id, "ses_", 32) || !valid_record_id(record_id) {
                return Err(LeyCoreError::InvalidLearningStore(
                    "session-record origin identity is invalid".to_owned(),
                ));
            }
            validate_stored_text("origin.recordType", record_type, 1, 64)?;
        }
        LearningOriginSource::CapturedArtifact {
            artifact_snapshot_id,
            artifact_path,
            content_hash,
        } => {
            let path = Path::new(artifact_path);
            if artifact_path.is_empty()
                || path.is_absolute()
                || path.components().any(|component| {
                    matches!(
                        component,
                        std::path::Component::ParentDir
                            | std::path::Component::RootDir
                            | std::path::Component::Prefix(_)
                    )
                })
                || !valid_prefixed_hex(artifact_snapshot_id, "snp_", 64)
                || !is_sha256(content_hash)
            {
                return Err(LeyCoreError::InvalidLearningStore(
                    "captured-artifact origin metadata is invalid".to_owned(),
                ));
            }
        }
        LearningOriginSource::TurnEvidence {
            session_id,
            record_id,
        } => {
            if !valid_prefixed_hex(session_id, "ses_", 32)
                || !valid_prefixed_hex(record_id, "tev_", 32)
            {
                return Err(LeyCoreError::InvalidLearningStore(
                    "turn-evidence origin identity is invalid".to_owned(),
                ));
            }
        }
        LearningOriginSource::ToolEvidence {
            session_id,
            record_id,
        } => {
            if schema_version < LEARNING_SCHEMA_VERSION
                || !valid_prefixed_hex(session_id, "ses_", 32)
                || !valid_prefixed_hex(record_id, "toe_", 32)
            {
                return Err(LeyCoreError::InvalidLearningStore(
                    "tool-evidence origin identity is invalid for this learning schema".to_owned(),
                ));
            }
        }
        LearningOriginSource::RecoveryCandidate {
            session_id,
            candidate_fingerprint,
        } => {
            if !valid_prefixed_hex(session_id, "ses_", 32) || !is_sha256(candidate_fingerprint) {
                return Err(LeyCoreError::InvalidLearningStore(
                    "recovery-candidate origin identity is invalid".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_stored_evidence(evidence: &[LearningEvidence]) -> Result<(), LeyCoreError> {
    if evidence.is_empty() || evidence.len() > 20 {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning evidence count is invalid".to_owned(),
        ));
    }
    let mut unique = BTreeSet::new();
    for item in evidence {
        if !valid_prefixed_hex(&item.session_id, "ses_", 32)
            || !valid_record_id(&item.record_id)
            || !unique.insert((&item.session_id, &item.record_id))
        {
            return Err(LeyCoreError::InvalidLearningStore(
                "learning evidence identity is invalid".to_owned(),
            ));
        }
        validate_stored_text("evidence.recordType", &item.record_type, 1, 64)?;
        if !matches!(
            item.record_type.as_str(),
            "session"
                | "session-finish"
                | "checkpoint"
                | "plan-item"
                | "decision"
                | "task"
                | "command"
                | "verification"
                | "problem"
                | "attempt"
                | "resolution"
                | "unresolved"
                | "turn-user-prompt"
                | "turn-assistant-response"
        ) {
            return Err(LeyCoreError::InvalidLearningStore(
                "learning evidence record type is invalid".to_owned(),
            ));
        }
        validate_stored_text("evidence.note", &item.note, 0, 2_000)?;
        if item.session_updated_at_unix_ms == 0 || item.artifacts.len() > 200 {
            return Err(LeyCoreError::InvalidLearningStore(
                "learning evidence metadata is invalid".to_owned(),
            ));
        }
        let mut paths = BTreeSet::new();
        for citation in &item.artifacts {
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
                        std::path::Component::ParentDir
                            | std::path::Component::RootDir
                            | std::path::Component::Prefix(_)
                    )
                })
                || !valid_prefixed_hex(&citation.artifact_snapshot_id, "snp_", 64)
                || !is_sha256(&citation.content_hash)
                || !valid_range
                || !paths.insert(&citation.artifact_path)
            {
                return Err(LeyCoreError::InvalidLearningStore(
                    "learning artifact citation is invalid".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn sanitize_text(
    field: &str,
    value: &str,
    minimum: usize,
    maximum: usize,
    redactions: &mut Vec<LearningRedaction>,
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
        return Err(LeyCoreError::InvalidLearningRequest(format!(
            "{field} must contain {minimum} to {maximum} safe characters"
        )));
    }
    let (sanitized, findings) = redact_secrets(value);
    redactions.extend(
        findings
            .into_iter()
            .map(|RedactionFinding { kind, lines }| LearningRedaction {
                field: field.to_owned(),
                kind,
                lines,
            }),
    );
    Ok(sanitized)
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
        return Err(LeyCoreError::InvalidLearningStore(format!(
            "{field} contains invalid stored text"
        )));
    }
    let (redacted, _) = redact_secrets(value);
    if redacted != value {
        return Err(LeyCoreError::InvalidLearningStore(format!(
            "{field} contains unredacted secret material"
        )));
    }
    Ok(())
}

fn validate_confidence(value: u8) -> Result<(), LeyCoreError> {
    if value > 100 {
        return Err(LeyCoreError::InvalidLearningRequest(
            "confidencePercent must be between 0 and 100".to_owned(),
        ));
    }
    Ok(())
}

fn validate_stored_confidence(value: u8) -> Result<(), LeyCoreError> {
    if value > 100 {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning confidence is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_request_id(value: &str) -> Result<(), LeyCoreError> {
    if !valid_prefixed_hex(value, "req_", 32) {
        return Err(LeyCoreError::InvalidLearningRequest(
            "requestId must match req_ followed by 32 lowercase hexadecimal characters".to_owned(),
        ));
    }
    Ok(())
}

fn validate_request_id_store(value: &str) -> Result<(), LeyCoreError> {
    if !valid_prefixed_hex(value, "req_", 32) {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning request ID is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_learning_id(value: &str) -> Result<(), LeyCoreError> {
    if !valid_prefixed_hex(value, "lrn_", 32) {
        return Err(LeyCoreError::InvalidLearningRequest(
            "learningId must match lrn_ followed by 32 lowercase hexadecimal characters".to_owned(),
        ));
    }
    Ok(())
}

fn validate_learning_id_store(value: &str) -> Result<(), LeyCoreError> {
    if !valid_prefixed_hex(value, "lrn_", 32) {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning ID is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_event_id(value: &str) -> Result<(), LeyCoreError> {
    if !valid_prefixed_hex(value, "lev_", 64) {
        return Err(LeyCoreError::InvalidLearningStore(
            "learning event ID is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn valid_record_id(value: &str) -> bool {
    [
        ("ses_", 32),
        ("evt_", 64),
        ("ckp_", 32),
        ("pln_", 32),
        ("dec_", 32),
        ("tsk_", 32),
        ("prb_", 32),
        ("att_", 32),
        ("res_", 32),
        ("cmd_", 32),
        ("ver_", 32),
        ("unr_", 32),
        ("tev_", 32),
    ]
    .iter()
    .any(|(prefix, length)| valid_prefixed_hex(value, prefix, *length))
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

fn feedback_label(action: LearningFeedbackAction) -> &'static str {
    match action {
        LearningFeedbackAction::Confirm => "confirm",
        LearningFeedbackAction::Contest => "contest",
        LearningFeedbackAction::Reject => "reject",
        LearningFeedbackAction::MarkStale => "mark-stale",
        LearningFeedbackAction::Supersede => "supersede",
    }
}

fn state_label(state: LearningState) -> &'static str {
    match state {
        LearningState::Tentative => "tentative",
        LearningState::Verified => "verified",
        LearningState::Contested => "contested",
        LearningState::Superseded => "superseded",
        LearningState::Rejected => "rejected",
        LearningState::Stale => "stale",
    }
}

fn trust_label(state: LearningTrustState) -> &'static str {
    match state {
        LearningTrustState::ReviewRequired => "review-required",
        LearningTrustState::Trusted => "trusted",
        LearningTrustState::Contested => "contested",
        LearningTrustState::Superseded => "superseded",
        LearningTrustState::Rejected => "rejected",
        LearningTrustState::Stale => "stale",
    }
}

fn freshness_label(freshness: LearningFreshness) -> &'static str {
    match freshness {
        LearningFreshness::Current => "current",
        LearningFreshness::SourceChanged => "source-changed",
        LearningFreshness::Uncited => "uncited",
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

fn json_body<T: Serialize>(value: &T, limit: u64, name: &str) -> Result<Vec<u8>, LeyCoreError> {
    let mut body = serde_json::to_vec_pretty(value)
        .map_err(|error| LeyCoreError::InvalidLearningStore(error.to_string()))?;
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
        .map_err(|error| LeyCoreError::InvalidLearningStore(format!("{name}: {error}")))
}

fn open_existing_dir(parent: &Dir, name: &str) -> Result<Dir, LeyCoreError> {
    parent
        .open_dir_nofollow(name)
        .map_err(|source| learning_io(name, source))
}

fn open_or_create_private_dir(parent: &Dir, name: &str) -> Result<Dir, LeyCoreError> {
    match parent.open_dir_nofollow(name) {
        Ok(directory) => return Ok(directory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(learning_io(name, source)),
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
        Err(source) => return Err(learning_io(name, source)),
    }
    parent
        .open_dir_nofollow(name)
        .map_err(|source| learning_io(name, source))
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
        Err(source) => return Err(learning_io(name, source)),
    };
    ensure_private_file(&file, name)?;
    let metadata = file
        .metadata()
        .map_err(|source| learning_io(name, source))?;
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
        .map_err(|source| learning_io(name, source))?;
    if bytes.len() as u64 > limit {
        return Err(LeyCoreError::MetadataTooLarge {
            path: PathBuf::from(name),
            limit_bytes: limit,
        });
    }
    Ok(Some(bytes))
}

fn ensure_private_file(file: &cap_std::fs::File, name: &str) -> Result<(), LeyCoreError> {
    let metadata = file
        .metadata()
        .map_err(|source| learning_io(name, source))?;
    if !metadata.is_file() {
        return Err(LeyCoreError::InvalidLearningStore(format!(
            "{name} is not a regular file"
        )));
    }
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LeyCoreError::InvalidLearningStore(format!(
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
        return Err(LeyCoreError::InvalidLearningStore(format!(
            "immutable learning event collision at {name}"
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
        .map_err(|source| learning_io(name, source))?;
    file.write_all(body)
        .map_err(|source| learning_io(name, source))?;
    file.sync_all().map_err(|source| learning_io(name, source))
}

fn write_atomic_private(directory: &Dir, name: &str, body: &[u8]) -> Result<(), LeyCoreError> {
    let mut temporary =
        cap_tempfile::TempFile::new(directory).map_err(|source| learning_io(name, source))?;
    let mut permissions = temporary
        .as_file()
        .metadata()
        .map_err(|source| learning_io(name, source))?
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
        .map_err(|source| learning_io(name, source))?;
    temporary
        .write_all(body)
        .map_err(|source| learning_io(name, source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| learning_io(name, source))?;
    temporary
        .replace(name)
        .map_err(|source| learning_io(name, source))
}

fn learning_io(name: &str, source: std::io::Error) -> LeyCoreError {
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
        checkpoint_session, erase_session_memory, finish_session, ingest_project,
        initialize_project, project_memory_overview, read_session, record_session_prompt,
        record_session_response, record_session_tool_observation, start_session, AttemptInput,
        AttemptOutcome, CaptureMode, CheckpointInput, CommandInput, DecisionInput,
        EraseSessionMemoryInput, FinishSessionInput, ProblemInput, ResolutionInput, SessionSource,
        SessionSourceKind, SessionStatus, StartSessionInput, TaskInput, TaskStatus,
        ToolObservationInput, ToolObservationKind, TurnEvidenceInput, TurnEvidenceOrigin,
    };
    use std::sync::mpsc;
    use std::sync::{Arc, Barrier};
    use std::time::Duration;
    use tempfile::tempdir;

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
    }

    fn png_fixture() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes.extend_from_slice(&[0, 0, 0, 13]);
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(b"IEND");
        bytes.extend_from_slice(&[0xae, 0x42, 0x60, 0x82]);
        bytes
    }

    #[test]
    fn learning_evidence_preserves_multimodal_artifact_citations() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Media learning"), CaptureMode::FullEvidence).unwrap();
        std::fs::write(project.join("README.md"), "# Media learning\n").unwrap();
        std::fs::write(project.join("verification.png"), png_fixture()).unwrap();
        ingest_project(&project, &vault).unwrap();

        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('a'),
                name: "Verify the interface".to_owned(),
                goal: "Preserve reusable visual verification evidence".to_owned(),
                source: SessionSource {
                    kind: SessionSourceKind::HostHook,
                    host: Some("codex".to_owned()),
                    agent: Some("gpt-5".to_owned()),
                    source_reference: None,
                },
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            CheckpointInput {
                request_id: request_id('b'),
                summary: "Captured the verified interface state".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: vec!["verification.png".to_owned()],
                commands: Vec::new(),
                verification: Vec::new(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let checkpoint_id = checkpoint.session.checkpoints[0].id.clone();

        let proposed = propose_learning(
            &project,
            &vault,
            ProposeLearningInput {
                request_id: request_id('c'),
                actor: LearningActor::Agent,
                kind: LearningKind::Procedure,
                title: "Check the verified interface".to_owned(),
                guidance: "Compare the interface against the retained visual evidence.".to_owned(),
                confidence_percent: 80,
                provenance: LearningProvenance::Inferred,
                evidence: vec![LearningEvidenceInput {
                    session_id: started.session.session_id,
                    record_id: checkpoint_id,
                    note: "The checkpoint retained exact original image evidence.".to_owned(),
                }],
            },
        )
        .unwrap();

        let citation = &proposed.learning.evidence[0].artifacts[0];
        assert_eq!(citation.artifact_path, "verification.png");
        assert_eq!(citation.media_type, Some(crate::ArtifactMediaType::Png));
        assert_eq!(citation.start_line, 0);
        assert_eq!(citation.end_line, 0);
    }

    fn setup_learning() -> (tempfile::TempDir, PathBuf, PathBuf, String, String) {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Learning test"), CaptureMode::Structured).unwrap();
        std::fs::write(
            project.join("README.md"),
            "# Reliable builds\n\nRun the workspace checks before delivery.\n",
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('1'),
                name: "Fix the build".to_owned(),
                goal: "Find and preserve a reliable build procedure".to_owned(),
                source: SessionSource {
                    kind: SessionSourceKind::HostHook,
                    host: Some("codex".to_owned()),
                    agent: Some("gpt-5".to_owned()),
                    source_reference: None,
                },
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            CheckpointInput {
                request_id: request_id('2'),
                summary: "Found and verified the build sequence".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: vec![ProblemInput {
                    title: "Workspace build failed".to_owned(),
                    symptom: "A package was checked in isolation".to_owned(),
                    expected: "The complete workspace should compile".to_owned(),
                    attempts: vec![AttemptInput {
                        action: "Run the workspace check".to_owned(),
                        outcome: AttemptOutcome::Helped,
                        evidence: "All packages compiled".to_owned(),
                    }],
                    resolution: Some(ResolutionInput {
                        root_cause: "The wrong command omitted workspace members".to_owned(),
                        change: "Run cargo check --workspace".to_owned(),
                        verification: "The workspace check exited successfully".to_owned(),
                    }),
                }],
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: Vec::new(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let record_id = checkpoint.session.checkpoints[0].problems[0]
            .resolution
            .as_ref()
            .unwrap()
            .id
            .clone();
        (base, project, vault, started.session.session_id, record_id)
    }

    fn proposal(request_id: String, session_id: &str, record_id: &str) -> ProposeLearningInput {
        ProposeLearningInput {
            request_id,
            actor: LearningActor::Agent,
            kind: LearningKind::Procedure,
            title: "Check the whole workspace".to_owned(),
            guidance: "Run cargo check --workspace before delivery.".to_owned(),
            confidence_percent: 82,
            provenance: LearningProvenance::Inferred,
            evidence: vec![LearningEvidenceInput {
                session_id: session_id.to_owned(),
                record_id: record_id.to_owned(),
                note: "The resolution was verified in this session.".to_owned(),
            }],
        }
    }

    fn native_proposal_pending(
        project: &Path,
        vault: &Path,
        store: &ContinuityStore,
        input: ProposeLearningInput,
    ) -> (String, PendingLearningEvent) {
        let diagnostic = diagnose_project(project).unwrap();
        let learning_id = deterministic_id(
            "lrn",
            &format!("{}:{}", diagnostic.identity.project_id, input.request_id),
            32,
        );
        let event_id = deterministic_id(
            "lev",
            &format!("{learning_id}:{}:proposed", input.request_id),
            64,
        );
        let mut redactions = Vec::new();
        let (evidence, origin_lineage) = resolve_evidence(
            &diagnostic.root,
            vault,
            Some(store),
            input.evidence,
            &mut redactions,
        )
        .unwrap();
        (
            learning_id,
            PendingLearningEvent {
                event_id,
                request_id: input.request_id,
                expected_event_count: None,
                expected_replacement_event_count: None,
                redactions,
                payload: LearningEventPayload::Proposed {
                    actor: input.actor,
                    kind: input.kind,
                    title: input.title,
                    guidance: input.guidance,
                    confidence_percent: input.confidence_percent,
                    provenance: input.provenance,
                    evidence,
                    origin_lineage: Some(origin_lineage),
                },
                allow_create: true,
            },
        )
    }

    fn native_correction_pending(
        project: &Path,
        vault: &Path,
        store: &ContinuityStore,
        learning_id: &str,
        request_id: String,
        expected_event_count: Option<u64>,
        session_id: &str,
        record_id: &str,
        title: &str,
    ) -> PendingLearningEvent {
        let mut redactions = Vec::new();
        let (evidence, origin_lineage) = resolve_evidence(
            project,
            vault,
            Some(store),
            vec![LearningEvidenceInput {
                session_id: session_id.to_owned(),
                record_id: record_id.to_owned(),
                note: "Native learning correction evidence.".to_owned(),
            }],
            &mut redactions,
        )
        .unwrap();
        let event_id =
            deterministic_id("lev", &format!("{learning_id}:{request_id}:corrected"), 64);
        PendingLearningEvent {
            event_id,
            request_id,
            expected_event_count,
            expected_replacement_event_count: None,
            redactions,
            payload: LearningEventPayload::Corrected {
                actor: LearningActor::User,
                title: title.to_owned(),
                guidance: "Keep the native learning ledger transactionally consistent.".to_owned(),
                confidence_percent: 91,
                evidence,
                note: "Native correction.".to_owned(),
                origin_lineage: Some(origin_lineage),
            },
            allow_create: false,
        }
    }

    fn private_continuity_store(base: &tempfile::TempDir, name: &str) -> ContinuityStore {
        let directory = base.path().join(name);
        std::fs::create_dir(&directory).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        ContinuityStore::at(directory.join("continuity.sqlite3"))
    }

    fn add_learning_session(
        project: &Path,
        vault: &Path,
        start_request: String,
        checkpoint_request: String,
        name: &str,
    ) -> (String, String) {
        let started = start_session(
            project,
            vault,
            StartSessionInput {
                request_id: start_request,
                name: name.to_owned(),
                goal: "Preserve an independent project lesson".to_owned(),
                source: SessionSource {
                    kind: SessionSourceKind::HostHook,
                    host: Some("claude-code".to_owned()),
                    agent: Some("claude".to_owned()),
                    source_reference: None,
                },
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            project,
            vault,
            &started.session.session_id,
            CheckpointInput {
                request_id: checkpoint_request,
                summary: "Verified an independent workflow".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: vec![ProblemInput {
                    title: "Independent failure".to_owned(),
                    symptom: "A separate workflow failed".to_owned(),
                    expected: "The separate workflow should pass".to_owned(),
                    attempts: vec![AttemptInput {
                        action: "Run the independent check".to_owned(),
                        outcome: AttemptOutcome::Helped,
                        evidence: "The independent check passed".to_owned(),
                    }],
                    resolution: Some(ResolutionInput {
                        root_cause: "The independent command was incomplete".to_owned(),
                        change: "Run the complete independent command".to_owned(),
                        verification: "The complete command exited successfully".to_owned(),
                    }),
                }],
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: Vec::new(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let record_id = checkpoint.session.checkpoints[0].problems[0]
            .resolution
            .as_ref()
            .unwrap()
            .id
            .clone();
        (started.session.session_id, record_id)
    }

    fn session_directory(project: &Path, vault: &Path, session_id: &str) -> PathBuf {
        let project_id = diagnose_project(project).unwrap().identity.project_id;
        vault
            .join(STORE_ROOT)
            .join(AGENT_MEMORY_DIRECTORY)
            .join(PROJECTS_DIRECTORY)
            .join(project_id)
            .join("sessions")
            .join(session_id)
    }

    fn learning_directory(project: &Path, vault: &Path) -> PathBuf {
        let project_id = diagnose_project(project).unwrap().identity.project_id;
        vault
            .join(STORE_ROOT)
            .join(AGENT_MEMORY_DIRECTORY)
            .join(PROJECTS_DIRECTORY)
            .join(project_id)
            .join(LEARNINGS_DIRECTORY)
    }

    #[test]
    fn proposal_is_cited_reviewable_and_idempotent_across_source_changes() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let input = proposal(request_id('3'), &session_id, &record_id);
        let proposed = propose_learning(&project, &vault, input.clone()).unwrap();
        assert!(!proposed.replayed);
        assert_eq!(proposed.learning.state, LearningState::Tentative);
        assert_eq!(
            proposed.learning.trust_state,
            LearningTrustState::ReviewRequired
        );
        assert_eq!(proposed.learning.freshness, LearningFreshness::Current);
        assert_eq!(proposed.learning.corroborating_sessions, 1);
        assert_eq!(proposed.learning.evidence[0].record_id, record_id);
        assert!(proposed.learning.origin_lineage.mechanically_resolved);
        assert_eq!(proposed.learning.origin_lineage.omitted_sources, 0);
        assert_eq!(
            proposed.learning.origin_lineage.automatic_authority_ceiling,
            LearningTrustState::ReviewRequired
        );
        assert!(proposed
            .learning
            .origin_lineage
            .sources
            .iter()
            .any(|source| {
                matches!(
                    source,
                    LearningOriginSource::SessionRecord {
                        session_id: origin_session_id,
                        record_id: origin_record_id,
                        record_type,
                    } if origin_session_id == &session_id
                        && origin_record_id == &record_id
                        && record_type == "resolution"
                )
            }));
        assert!(proposed.learning.origin_lineage.sources.iter().any(|source| {
            matches!(source, LearningOriginSource::CapturedArtifact { artifact_path, .. } if artifact_path == "README.md")
        }));

        std::fs::write(
            project.join("README.md"),
            "# Reliable builds\n\nThe build process has changed.\n",
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let replayed = propose_learning(&project, &vault, input).unwrap();
        assert!(replayed.replayed);
        assert_eq!(replayed.event_id, proposed.event_id);
        assert_eq!(replayed.learning.event_count, 1);
        assert_eq!(
            replayed.learning.freshness,
            LearningFreshness::SourceChanged
        );
        let inbox = learning_review_inbox(&project, &vault).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].freshness, LearningFreshness::SourceChanged);

        let directory = learning_directory(&project, &vault);
        let review = std::fs::read_to_string(directory.join(LEARNING_REVIEW_FILE)).unwrap();
        assert!(review.contains("# Learning review"));
        assert!(review.contains("Check the whole workspace"));
        assert!(review.contains("source-changed"));
    }

    fn recovery_test_session(
        project: &Path,
        vault: &Path,
        request_digit: char,
        name: &str,
    ) -> crate::SessionMutation {
        start_session(
            project,
            vault,
            StartSessionInput {
                request_id: request_id(request_digit),
                name: name.to_owned(),
                goal: "Exercise persisted recovery compatibility".to_owned(),
                source: SessionSource {
                    kind: SessionSourceKind::HostHook,
                    host: Some("codex".to_owned()),
                    agent: Some("test-agent".to_owned()),
                    source_reference: None,
                },
            },
        )
        .unwrap()
    }

    fn recovery_test_turn(request_digit: char, text: &str) -> TurnEvidenceInput {
        TurnEvidenceInput {
            request_id: request_id(request_digit),
            origin: TurnEvidenceOrigin::HostHook,
            host: Some("codex".to_owned()),
            correlation_material: Some("fixture-turn".to_owned()),
            text: text.to_owned(),
        }
    }

    fn recovery_test_checkpoint(
        request_digit: char,
        summary: &str,
        decisions: Vec<DecisionInput>,
        tasks: Vec<TaskInput>,
        problems: Vec<ProblemInput>,
        commands: Vec<CommandInput>,
        unresolved: Vec<String>,
    ) -> CheckpointInput {
        CheckpointInput {
            request_id: request_id(request_digit),
            summary: summary.to_owned(),
            plan: Vec::new(),
            decisions,
            tasks,
            problems,
            touched_artifacts: Vec::new(),
            commands,
            verification: Vec::new(),
            unresolved,
        }
    }

    fn has_turn_origin(learning: &LearningRecord, record_id: &str) -> bool {
        learning.origin_lineage.sources.iter().any(|source| {
            matches!(source, LearningOriginSource::TurnEvidence { record_id: found, .. } if found == record_id)
        })
    }

    fn has_recovery_origin(learning: &LearningRecord, fingerprint: &str) -> bool {
        learning.origin_lineage.sources.iter().any(|source| {
            matches!(source, LearningOriginSource::RecoveryCandidate { candidate_fingerprint, .. } if candidate_fingerprint == fingerprint)
        })
    }

    #[test]
    fn schema_v3_persisted_recovery_lineage_reaches_candidate_and_exact_turn_evidence() {
        let (_base, project, vault, _, _) = setup_learning();
        let started = recovery_test_session(&project, &vault, 'a', "Interrupted derivation");
        let session_id = started.session.session_id.clone();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &session_id,
            recovery_test_turn('b', "Investigate the retry loop"),
        )
        .unwrap();
        let prompt_id = prompt.session.prompts.last().unwrap().record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &session_id,
            recovery_test_turn('c', "The retry investigation remains unresolved"),
        )
        .unwrap();
        let response_id = response.session.responses.last().unwrap().record_id.clone();
        let _ordinary = checkpoint_session(
            &project,
            &vault,
            &session_id,
            recovery_test_checkpoint(
                'd',
                "Retry investigation",
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec!["The retry investigation remains unresolved".to_owned()],
            ),
        )
        .unwrap();
        let mut evidence = vec![prompt_id.clone(), response_id.clone()];
        evidence.sort();
        let replayed =
            crate::session::recovery_fixture::replace_latest_checkpoint_with_persisted_recovery(
                &project,
                &vault,
                &session_id,
                crate::session::SESSION_RECOVERY_SCHEMA_VERSION,
                evidence,
                Vec::new(),
                None,
                None,
            )
            .unwrap();
        let checkpoint = replayed.checkpoints.last().unwrap();
        let unresolved_id = crate::session::unresolved_record_id(&checkpoint.event_id, 0);
        let origin = crate::session::read_recovery_derivation_origin(
            &project,
            &vault,
            &session_id,
            &checkpoint.event_id,
            &unresolved_id,
        )
        .unwrap()
        .unwrap();
        let mut input = proposal(request_id('e'), &session_id, &unresolved_id);
        input.title = "Retry investigation is unresolved".to_owned();
        input.guidance = "Do not assume the retry investigation was completed.".to_owned();
        let proposed = propose_learning(&project, &vault, input).unwrap();
        assert!(proposed.learning.origin_lineage.mechanically_resolved);
        assert!(has_recovery_origin(
            &proposed.learning,
            &origin.candidate_fingerprint
        ));
        assert!(has_turn_origin(&proposed.learning, &prompt_id));
        assert!(has_turn_origin(&proposed.learning, &response_id));
    }

    #[test]
    fn schema_v16_persisted_observed_command_lineage_keeps_tool_evidence_namespace() {
        let (_base, project, vault, _, _) = setup_learning();
        let started = recovery_test_session(&project, &vault, 'a', "Observed Command derivation");
        let session_id = started.session.session_id.clone();
        let observed = record_session_tool_observation(
            &project,
            &vault,
            &session_id,
            ToolObservationInput {
                request_id: request_id('b'),
                host: "codex".to_owned(),
                turn_correlation_material: None,
                tool_call_correlation_material: "learning-command-origin".to_owned(),
                tool_name: "Bash".to_owned(),
                observation_kind: ToolObservationKind::Returned,
                command: "cargo test -p ley-core lineage".to_owned(),
                result: "process returned".to_owned(),
            },
        )
        .unwrap();
        let tool = observed.session.tool_observations[0].clone();
        let direct_tool_error = propose_learning(
            &project,
            &vault,
            ProposeLearningInput {
                request_id: request_id('f'),
                actor: LearningActor::Agent,
                kind: LearningKind::Procedure,
                title: "Raw tool evidence".to_owned(),
                guidance: "Raw tool observations are provenance only.".to_owned(),
                confidence_percent: 50,
                provenance: LearningProvenance::Inferred,
                evidence: vec![LearningEvidenceInput {
                    session_id: session_id.clone(),
                    record_id: tool.record_id.clone(),
                    note: "Raw tool observation is not direct learning evidence.".to_owned(),
                }],
            },
        )
        .unwrap_err();
        assert!(matches!(
            direct_tool_error,
            LeyCoreError::InvalidLearningRequest(_)
        ));
        let _ordinary = checkpoint_session(
            &project,
            &vault,
            &session_id,
            recovery_test_checkpoint(
                'c',
                crate::recovery_compat::OBSERVED_COMMAND_CANDIDATE_SUMMARY,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                vec![CommandInput {
                    command: "cargo test -p ley-core lineage".to_owned(),
                    exit_code: None,
                    summary: crate::recovery_compat::OBSERVED_COMMAND_CANDIDATE_SUMMARY.to_owned(),
                }],
                Vec::new(),
            ),
        )
        .unwrap();
        let replayed =
            crate::session::recovery_fixture::replace_latest_checkpoint_with_persisted_recovery(
                &project,
                &vault,
                &session_id,
                crate::session::SESSION_OBSERVED_COMMAND_RECOVERY_SCHEMA_VERSION,
                vec![tool.record_id.clone()],
                vec![vec![tool.record_id.clone()]],
                Some(tool.event_id.clone()),
                Some(ToolObservationKind::Returned),
            )
            .unwrap();
        let checkpoint = replayed.checkpoints.last().unwrap();
        let command_id = checkpoint.commands[0].id.clone();
        let origin = crate::session::read_recovery_derivation_origin(
            &project,
            &vault,
            &session_id,
            &checkpoint.event_id,
            &command_id,
        )
        .unwrap()
        .unwrap();
        let mut input = proposal(request_id('d'), &session_id, &command_id);
        input.title = "Run the focused lineage check".to_owned();
        input.guidance =
            "Use the recorded focused lineage command when it remains applicable.".to_owned();
        let proposed = propose_learning(&project, &vault, input).unwrap();
        assert!(proposed.learning.origin_lineage.mechanically_resolved);
        assert!(has_recovery_origin(
            &proposed.learning,
            &origin.candidate_fingerprint
        ));
        assert!(proposed.learning.origin_lineage.sources.iter().any(|source| {
            matches!(source, LearningOriginSource::ToolEvidence { record_id, .. } if record_id == &tool.record_id)
        }));
        assert!(!proposed.learning.origin_lineage.sources.iter().any(|source| {
            matches!(source, LearningOriginSource::TurnEvidence { record_id, .. } if record_id == &tool.record_id)
        }));
    }

    #[test]
    fn schema_v11_persisted_recovery_lineage_uses_record_specific_evidence() {
        let (_base, project, vault, _, _) = setup_learning();
        let started = recovery_test_session(&project, &vault, 'a', "Atomic interrupted derivation");
        let session_id = started.session.session_id.clone();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &session_id,
            recovery_test_turn('b', "Use SQLite for local persistence"),
        )
        .unwrap();
        let prompt_id = prompt.session.prompts.last().unwrap().record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &session_id,
            recovery_test_turn('c', "Migration task completed"),
        )
        .unwrap();
        let response_id = response.session.responses.last().unwrap().record_id.clone();
        let _ordinary = checkpoint_session(
            &project,
            &vault,
            &session_id,
            recovery_test_checkpoint(
                'd',
                "Recovered persistence work",
                vec![DecisionInput {
                    title: "Storage engine".to_owned(),
                    decision: "Use SQLite".to_owned(),
                    rationale: String::new(),
                    alternatives: Vec::new(),
                }],
                vec![TaskInput {
                    title: "Migrate local state".to_owned(),
                    status: TaskStatus::Completed,
                    details: "Migration completed".to_owned(),
                }],
                Vec::new(),
                Vec::new(),
                vec!["Confirm SQLite backup behavior".to_owned()],
            ),
        )
        .unwrap();
        let mut evidence = vec![prompt_id.clone(), response_id.clone()];
        evidence.sort();
        let replayed =
            crate::session::recovery_fixture::replace_latest_checkpoint_with_persisted_recovery(
                &project,
                &vault,
                &session_id,
                crate::session::SESSION_BATCH_RECOVERY_SCHEMA_VERSION,
                evidence,
                vec![
                    vec![prompt_id.clone()],
                    vec![response_id.clone()],
                    vec![prompt_id.clone()],
                ],
                None,
                None,
            )
            .unwrap();
        let checkpoint = replayed.checkpoints.last().unwrap();
        let ids = [
            checkpoint.decisions[0].id.clone(),
            checkpoint.tasks[0].id.clone(),
            crate::session::unresolved_record_id(&checkpoint.event_id, 0),
        ];
        let mut learnings = Vec::new();
        for (digit, record_id) in ['e', 'f', '9'].into_iter().zip(ids.iter()) {
            let mut input = proposal(request_id(digit), &session_id, record_id);
            input.title = format!("Evidence-backed record {digit}");
            input.guidance =
                "Preserve only the evidence bound to this recovered record.".to_owned();
            learnings.push(propose_learning(&project, &vault, input).unwrap().learning);
        }
        let candidate_fingerprint = crate::session::read_recovery_derivation_origin(
            &project,
            &vault,
            &session_id,
            &checkpoint.event_id,
            &ids[0],
        )
        .unwrap()
        .unwrap()
        .candidate_fingerprint;
        for learning in &learnings {
            assert!(has_recovery_origin(learning, &candidate_fingerprint));
        }
        assert!(has_turn_origin(&learnings[0], &prompt_id));
        assert!(!has_turn_origin(&learnings[0], &response_id));
        assert!(has_turn_origin(&learnings[1], &response_id));
        assert!(!has_turn_origin(&learnings[1], &prompt_id));
        assert!(has_turn_origin(&learnings[2], &prompt_id));
        assert!(!has_turn_origin(&learnings[2], &response_id));
    }

    #[test]
    fn schema_v12_persisted_rich_problem_lineage_uses_component_specific_evidence() {
        let (_base, project, vault, _, _) = setup_learning();
        let started = recovery_test_session(&project, &vault, 'a', "Interrupted debugging episode");
        let session_id = started.session.session_id.clone();
        let symptom = record_session_prompt(
            &project,
            &vault,
            &session_id,
            recovery_test_turn(
                'b',
                "Refresh returns 401 although the session should survive.",
            ),
        )
        .unwrap();
        let symptom_id = symptom.session.prompts.last().unwrap().record_id.clone();
        let attempt = record_session_response(
            &project,
            &vault,
            &session_id,
            recovery_test_turn('c', "Clearing cookies had no effect; the 401 remained."),
        )
        .unwrap();
        let attempt_id = attempt.session.responses.last().unwrap().record_id.clone();
        let resolution = record_session_prompt(
            &project,
            &vault,
            &session_id,
            recovery_test_turn(
                'd',
                "The expired token was the root cause; refreshing it fixed repeated refreshes.",
            ),
        )
        .unwrap();
        let resolution_id = resolution.session.prompts.last().unwrap().record_id.clone();
        let _ordinary = checkpoint_session(
            &project,
            &vault,
            &session_id,
            recovery_test_checkpoint(
                'e',
                "Login refresh failure",
                Vec::new(),
                Vec::new(),
                vec![ProblemInput {
                    title: "Login refresh failure".to_owned(),
                    symptom: "Refreshing returns 401".to_owned(),
                    expected: "The authenticated session survives refresh".to_owned(),
                    attempts: vec![AttemptInput {
                        action: "Clear browser cookies".to_owned(),
                        outcome: AttemptOutcome::NoEffect,
                        evidence: "Refresh still returned 401".to_owned(),
                    }],
                    resolution: Some(ResolutionInput {
                        root_cause: "The client reused an expired access token".to_owned(),
                        change: "Refresh the token before protected navigation".to_owned(),
                        verification: "Repeated refreshes remained authenticated".to_owned(),
                    }),
                }],
                Vec::new(),
                Vec::new(),
            ),
        )
        .unwrap();
        let mut evidence = vec![
            symptom_id.clone(),
            attempt_id.clone(),
            resolution_id.clone(),
        ];
        evidence.sort();
        let replayed =
            crate::session::recovery_fixture::replace_latest_checkpoint_with_persisted_recovery(
                &project,
                &vault,
                &session_id,
                crate::session::SESSION_RICH_PROBLEM_RECOVERY_SCHEMA_VERSION,
                evidence,
                vec![
                    vec![symptom_id.clone()],
                    vec![attempt_id.clone()],
                    vec![resolution_id.clone()],
                ],
                None,
                None,
            )
            .unwrap();
        let checkpoint = replayed.checkpoints.last().unwrap();
        let problem = &checkpoint.problems[0];
        let attempt_learning = propose_learning(
            &project,
            &vault,
            proposal(request_id('f'), &session_id, &problem.attempts[0].id),
        )
        .unwrap()
        .learning;
        let resolution_learning = propose_learning(
            &project,
            &vault,
            proposal(
                request_id('7'),
                &session_id,
                &problem.resolution.as_ref().unwrap().id,
            ),
        )
        .unwrap()
        .learning;
        assert!(has_recovery_origin(
            &attempt_learning,
            &crate::session::read_recovery_derivation_origin(
                &project,
                &vault,
                &session_id,
                &checkpoint.event_id,
                &problem.attempts[0].id
            )
            .unwrap()
            .unwrap()
            .candidate_fingerprint
        ));
        assert!(has_turn_origin(&attempt_learning, &attempt_id));
        assert!(!has_turn_origin(&attempt_learning, &symptom_id));
        assert!(has_turn_origin(&resolution_learning, &resolution_id));
        assert!(!has_turn_origin(&resolution_learning, &attempt_id));
    }

    #[test]
    fn schema_v13_persisted_composite_lineage_keeps_each_child_binding() {
        let (_base, project, vault, _, _) = setup_learning();
        let started =
            recovery_test_session(&project, &vault, 'a', "Composite interrupted derivation");
        let session_id = started.session.session_id.clone();
        let symptom = record_session_prompt(
            &project,
            &vault,
            &session_id,
            recovery_test_turn(
                'b',
                "Refresh returns 401 although authentication should survive.",
            ),
        )
        .unwrap();
        let symptom_id = symptom.session.prompts.last().unwrap().record_id.clone();
        let attempt = record_session_response(
            &project,
            &vault,
            &session_id,
            recovery_test_turn('c', "Clearing cookies had no effect; the 401 remained."),
        )
        .unwrap();
        let attempt_id = attempt.session.responses.last().unwrap().record_id.clone();
        let decision = record_session_prompt(
            &project,
            &vault,
            &session_id,
            recovery_test_turn('d', "Adopt refresh-before-navigation for protected routes."),
        )
        .unwrap();
        let decision_evidence_id = decision.session.prompts.last().unwrap().record_id.clone();
        let _ordinary = checkpoint_session(
            &project,
            &vault,
            &session_id,
            recovery_test_checkpoint(
                'e',
                "Recovered login debugging and policy",
                vec![DecisionInput {
                    title: "Token refresh policy".to_owned(),
                    decision: "Refresh before protected navigation".to_owned(),
                    rationale: String::new(),
                    alternatives: Vec::new(),
                }],
                Vec::new(),
                vec![ProblemInput {
                    title: "Login refresh failure".to_owned(),
                    symptom: "Refreshing returns 401".to_owned(),
                    expected: "The authenticated session survives refresh".to_owned(),
                    attempts: vec![AttemptInput {
                        action: "Clear browser cookies".to_owned(),
                        outcome: AttemptOutcome::NoEffect,
                        evidence: "Refresh still returned 401".to_owned(),
                    }],
                    resolution: None,
                }],
                Vec::new(),
                Vec::new(),
            ),
        )
        .unwrap();
        let mut evidence = vec![
            symptom_id.clone(),
            attempt_id.clone(),
            decision_evidence_id.clone(),
        ];
        evidence.sort();
        let replayed =
            crate::session::recovery_fixture::replace_latest_checkpoint_with_persisted_recovery(
                &project,
                &vault,
                &session_id,
                crate::session::SESSION_COMPOSITE_RECOVERY_SCHEMA_VERSION,
                evidence,
                vec![
                    vec![decision_evidence_id.clone()],
                    vec![symptom_id.clone()],
                    vec![attempt_id.clone()],
                ],
                None,
                None,
            )
            .unwrap();
        let checkpoint = replayed.checkpoints.last().unwrap();
        let problem = &checkpoint.problems[0];
        let attempt_learning = propose_learning(
            &project,
            &vault,
            proposal(request_id('f'), &session_id, &problem.attempts[0].id),
        )
        .unwrap()
        .learning;
        let decision_learning = propose_learning(
            &project,
            &vault,
            proposal(request_id('7'), &session_id, &checkpoint.decisions[0].id),
        )
        .unwrap()
        .learning;
        let checkpoint_origin = crate::session::read_recovery_derivation_origin(
            &project,
            &vault,
            &session_id,
            &checkpoint.event_id,
            &checkpoint.id,
        )
        .unwrap()
        .unwrap();
        assert!(has_recovery_origin(
            &attempt_learning,
            &checkpoint_origin.candidate_fingerprint
        ));
        assert!(has_turn_origin(&attempt_learning, &attempt_id));
        assert!(!has_turn_origin(&attempt_learning, &decision_evidence_id));
        assert!(has_recovery_origin(
            &decision_learning,
            &checkpoint_origin.candidate_fingerprint
        ));
        assert!(has_turn_origin(&decision_learning, &decision_evidence_id));
        assert!(!has_turn_origin(&decision_learning, &symptom_id));
    }

    #[test]
    fn review_required_learning_can_cite_retained_terminal_turn_evidence_directly() {
        let (_base, project, vault, _session_id, _record_id) = setup_learning();
        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('a'),
                name: "Terminal consolidation".to_owned(),
                goal: "Preserve a reusable lesson from retained turn evidence".to_owned(),
                source: SessionSource {
                    kind: SessionSourceKind::HostHook,
                    host: Some("codex".to_owned()),
                    agent: Some("gpt-5".to_owned()),
                    source_reference: None,
                },
            },
        )
        .unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            TurnEvidenceInput {
                request_id: request_id('b'),
                origin: TurnEvidenceOrigin::HostHook,
                host: Some("codex".to_owned()),
                correlation_material: Some("terminal-turn".to_owned()),
                text: "Remember to run the complete workspace check before release.".to_owned(),
            },
        )
        .unwrap();
        let prompt_id = prompt.session.prompts.last().unwrap().record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &started.session.session_id,
            TurnEvidenceInput {
                request_id: request_id('c'),
                origin: TurnEvidenceOrigin::HostHook,
                host: Some("codex".to_owned()),
                correlation_material: Some("terminal-turn".to_owned()),
                text: "The complete workspace check passed and caught the missing package."
                    .to_owned(),
            },
        )
        .unwrap();
        let response_id = response.session.responses.last().unwrap().record_id.clone();
        finish_session(
            &project,
            &vault,
            &started.session.session_id,
            FinishSessionInput {
                request_id: request_id('d'),
                status: SessionStatus::Completed,
                summary: "Finished without a final structured checkpoint.".to_owned(),
                final_response: String::new(),
                handoff: "Review retained turn evidence before promoting a reusable lesson."
                    .to_owned(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();

        let proposed = propose_learning(
            &project,
            &vault,
            ProposeLearningInput {
                request_id: request_id('e'),
                actor: LearningActor::Agent,
                kind: LearningKind::Procedure,
                title: "Run the complete workspace check".to_owned(),
                guidance: "Run the complete workspace check before release.".to_owned(),
                confidence_percent: 70,
                provenance: LearningProvenance::Inferred,
                evidence: vec![
                    LearningEvidenceInput {
                        session_id: started.session.session_id.clone(),
                        record_id: prompt_id.clone(),
                        note: "Historical user request.".to_owned(),
                    },
                    LearningEvidenceInput {
                        session_id: started.session.session_id.clone(),
                        record_id: response_id.clone(),
                        note: "Historical agent outcome; review required.".to_owned(),
                    },
                ],
            },
        )
        .unwrap();

        assert_eq!(proposed.learning.state, LearningState::Tentative);
        assert_eq!(
            proposed.learning.trust_state,
            LearningTrustState::ReviewRequired
        );
        let mut record_types = proposed
            .learning
            .evidence
            .iter()
            .map(|item| item.record_type.as_str())
            .collect::<Vec<_>>();
        record_types.sort_unstable();
        assert_eq!(
            record_types,
            vec!["turn-assistant-response", "turn-user-prompt"]
        );
        for record_id in [&prompt_id, &response_id] {
            assert!(proposed
                .learning
                .origin_lineage
                .sources
                .iter()
                .any(|source| {
                    matches!(
                        source,
                        LearningOriginSource::TurnEvidence {
                            session_id,
                            record_id: source_record_id,
                        } if session_id == &started.session.session_id && source_record_id == record_id
                    )
                }));
        }
        assert!(proposed
            .learning
            .origin_lineage
            .sources
            .iter()
            .all(|source| !matches!(source, LearningOriginSource::SessionRecord { .. })));
        assert_eq!(
            proposed.learning.origin_lineage.automatic_authority_ceiling,
            LearningTrustState::ReviewRequired
        );
    }

    #[test]
    fn body_free_turn_evidence_cannot_support_a_learning_proposal() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Minimal learning"), CaptureMode::Minimal).unwrap();
        std::fs::write(project.join("README.md"), "# Minimal learning\n").unwrap();
        ingest_project(&project, &vault).unwrap();
        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('f'),
                name: "Metadata only".to_owned(),
                goal: "Do not infer semantics from omitted turn bodies".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &started.session.session_id,
            TurnEvidenceInput {
                request_id: request_id('7'),
                origin: TurnEvidenceOrigin::ManualCli,
                host: None,
                correlation_material: None,
                text: "This body must not be retained.".to_owned(),
            },
        )
        .unwrap();
        let prompt_id = prompt.session.prompts.last().unwrap().record_id.clone();

        let result = propose_learning(
            &project,
            &vault,
            ProposeLearningInput {
                request_id: request_id('8'),
                actor: LearningActor::Agent,
                kind: LearningKind::Fact,
                title: "Unsafe metadata inference".to_owned(),
                guidance: "Do not derive this from body-free evidence.".to_owned(),
                confidence_percent: 10,
                provenance: LearningProvenance::Inferred,
                evidence: vec![LearningEvidenceInput {
                    session_id: started.session.session_id,
                    record_id: prompt_id,
                    note: String::new(),
                }],
            },
        );
        assert!(matches!(
            result,
            Err(LeyCoreError::InvalidLearningRequest(message))
                if message.contains("body-free user turn evidence")
        ));
    }

    #[test]
    fn corrections_union_origin_lineage_instead_of_replacing_history() {
        let (_base, project, vault, first_session_id, first_record_id) = setup_learning();
        let proposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('3'), &first_session_id, &first_record_id),
        )
        .unwrap();
        let (second_session_id, second_record_id) = add_learning_session(
            &project,
            &vault,
            request_id('4'),
            request_id('5'),
            "Independent confirmation",
        );
        let corrected = correct_learning(
            &project,
            &vault,
            &proposed.learning.learning_id,
            CorrectLearningInput {
                request_id: request_id('6'),
                expected_event_count: Some(proposed.learning.event_count),
                actor: LearningActor::User,
                title: "Check both workflows".to_owned(),
                guidance: "Run both verified workspace workflows before delivery.".to_owned(),
                confidence_percent: 90,
                evidence: vec![LearningEvidenceInput {
                    session_id: second_session_id.clone(),
                    record_id: second_record_id,
                    note: "Independent confirmation.".to_owned(),
                }],
                note: "Broadened after a second verified session.".to_owned(),
            },
        )
        .unwrap();
        for expected_session in [&first_session_id, &second_session_id] {
            assert!(corrected
                .learning
                .origin_lineage
                .sources
                .iter()
                .any(|source| {
                    matches!(
                        source,
                        LearningOriginSource::SessionRecord { session_id, .. }
                            if session_id == expected_session
                    )
                }));
        }
        assert!(corrected.learning.origin_lineage.mechanically_resolved);
        assert!(!corrected.learning.origin_lineage.causal_completeness_proven);
        assert_eq!(
            corrected
                .learning
                .origin_lineage
                .automatic_authority_ceiling,
            LearningTrustState::ReviewRequired
        );
    }

    #[test]
    fn continuity_snapshot_replays_reviewed_learning_after_legacy_learning_store_disappears() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        let proposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('7'), &session_id, &record_id),
        )
        .unwrap();
        let corrected = correct_learning(
            &project,
            &vault,
            &proposed.learning.learning_id,
            CorrectLearningInput {
                request_id: request_id('8'),
                expected_event_count: Some(proposed.learning.event_count),
                actor: LearningActor::User,
                title: "Reviewed native continuity learning".to_owned(),
                guidance: "Replay the validated learning ledger from native continuity.".to_owned(),
                confidence_percent: 92,
                evidence: proposed
                    .learning
                    .evidence
                    .iter()
                    .map(|item| LearningEvidenceInput {
                        session_id: item.session_id.clone(),
                        record_id: item.record_id.clone(),
                        note: item.note.clone(),
                    })
                    .collect(),
                note: "Corrected before review.".to_owned(),
            },
        )
        .unwrap();
        let reviewed = review_learning(
            &project,
            &vault,
            &corrected.learning.learning_id,
            ReviewLearningInput {
                request_id: request_id('9'),
                expected_event_count: Some(corrected.learning.event_count),
                actor: LearningActor::User,
                action: LearningFeedbackAction::Confirm,
                note: "Verified for native replay.".to_owned(),
                replacement_learning_id: None,
            },
        )
        .unwrap();
        let continuity_dir = base.path().join("continuity");
        std::fs::create_dir(&continuity_dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&continuity_dir, std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let store = ContinuityStore::at(continuity_dir.join("continuity.sqlite3"));
        crate::import_legacy_continuity(&project, &vault, &store).unwrap();

        let native = read_learning_from_continuity_snapshot(
            &project,
            &vault,
            &store,
            &reviewed.learning.learning_id,
        )
        .unwrap();
        assert_eq!(native, reviewed.learning);

        std::fs::remove_dir_all(learning_directory(&project, &vault)).unwrap();
        assert!(read_learning(&project, &vault, &native.learning_id).is_err());
        assert_eq!(
            read_learning_from_continuity_snapshot(&project, &vault, &store, &native.learning_id)
                .unwrap(),
            native
        );
        let summaries = list_learnings_from_continuity_snapshot(&project, &vault, &store).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].learning_id, reviewed.learning.learning_id);
        assert_eq!(summaries[0].trust_state, LearningTrustState::Trusted);
    }

    #[test]
    fn continuity_learning_replay_rejects_mismatched_envelope_kind() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        let proposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('a'), &session_id, &record_id),
        )
        .unwrap();
        let identity = diagnose_project(&project).unwrap().identity;
        let mut events = continuity_events_for_migration(&project, &vault).unwrap();
        assert_eq!(events.len(), 1);
        let mut event = events.remove(0);
        event.kind = "learning-reviewed".to_owned();
        let continuity_dir = base.path().join("malformed-continuity");
        std::fs::create_dir(&continuity_dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&continuity_dir, std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let store = ContinuityStore::at(continuity_dir.join("continuity.sqlite3"));
        store.register_project(&identity).unwrap();
        store.append_event(&event).unwrap();

        assert!(matches!(
            read_learning_from_continuity_snapshot(
                &project,
                &vault,
                &store,
                &proposed.learning.learning_id,
            ),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("does not match its embedded learning event")
        ));
    }

    #[test]
    fn transition_learning_reads_track_legacy_learning_ledger_after_session_cutover() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        let proposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('b'), &session_id, &record_id),
        )
        .unwrap();
        let continuity_dir = base.path().join("transition-continuity");
        std::fs::create_dir(&continuity_dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&continuity_dir, std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let store = ContinuityStore::at(continuity_dir.join("continuity.sqlite3"));

        let first = read_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            &proposed.learning.learning_id,
        )
        .unwrap();
        assert_eq!(first, proposed.learning);

        crate::start_session_with_continuity_transition(
            &project,
            &vault,
            &store,
            crate::StartSessionInput {
                request_id: request_id('c'),
                name: "Native session authority".to_owned(),
                goal: "Freeze session authority before the learning-only resync".to_owned(),
                source: crate::SessionSource::default(),
            },
        )
        .unwrap();
        assert!(
            crate::session::session_authority_cutover_is_complete(&store, &first.project_id)
                .unwrap()
        );

        let corrected = correct_learning(
            &project,
            &vault,
            &first.learning_id,
            CorrectLearningInput {
                request_id: request_id('d'),
                expected_event_count: Some(first.event_count),
                actor: LearningActor::User,
                title: "Learning-only transition refresh".to_owned(),
                guidance: "Reconcile only the learning ledger after session cutover.".to_owned(),
                confidence_percent: 88,
                evidence: first
                    .evidence
                    .iter()
                    .map(|item| LearningEvidenceInput {
                        session_id: item.session_id.clone(),
                        record_id: item.record_id.clone(),
                        note: item.note.clone(),
                    })
                    .collect(),
                note: "Changed after session authority moved.".to_owned(),
            },
        )
        .unwrap();
        let refreshed =
            read_learning_with_continuity_transition(&project, &vault, &store, &first.learning_id)
                .unwrap();
        assert_eq!(refreshed, corrected.learning);

        std::fs::remove_dir_all(learning_directory(&project, &vault)).unwrap();
        assert!(
            list_learnings_with_continuity_transition(&project, &vault, &store)
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            read_learning_from_continuity_snapshot(&project, &vault, &store, &first.learning_id),
            Err(LeyCoreError::LearningNotFound(_))
        ));
        assert!(
            crate::session::session_authority_cutover_is_complete(&store, &first.project_id)
                .unwrap()
        );
    }

    #[test]
    fn native_learning_cutover_requires_native_session_authority_first() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        propose_learning(
            &project,
            &vault,
            proposal(request_id('e'), &session_id, &record_id),
        )
        .unwrap();
        let store = private_continuity_store(&base, "learning-cutover-order");
        assert!(matches!(
            ensure_native_learning_authority(&project, &vault, &store),
            Err(LeyCoreError::InvalidLearningStore(message))
                if message.contains("requires native session authority first")
        ));
        let project_id = diagnose_project(&project).unwrap().identity.project_id;
        assert!(!learning_authority_cutover_is_complete(&store, &project_id).unwrap());
    }

    #[test]
    fn native_learning_cutover_recovers_after_fence_and_resists_late_legacy_sync() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        let proposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('e'), &session_id, &record_id),
        )
        .unwrap();
        let identity = diagnose_project(&project).unwrap().identity;
        let store = private_continuity_store(&base, "learning-cutover-recovery");

        crate::start_session_with_continuity_transition(
            &project,
            &vault,
            &store,
            crate::StartSessionInput {
                request_id: request_id('f'),
                name: "Session authority prerequisite".to_owned(),
                goal: "Freeze session authority before learning cutover".to_owned(),
                source: crate::SessionSource::default(),
            },
        )
        .unwrap();
        assert!(crate::session::session_authority_cutover_is_complete(
            &store,
            &identity.project_id
        )
        .unwrap());

        assert!(fence_legacy_learning_writes(&project, &vault).unwrap());
        assert!(!fence_legacy_learning_writes(&project, &vault).unwrap());
        assert_eq!(
            read_learning(&project, &vault, &proposed.learning.learning_id)
                .unwrap()
                .event_count,
            1
        );
        assert!(correct_learning(
            &project,
            &vault,
            &proposed.learning.learning_id,
            CorrectLearningInput {
                request_id: request_id('a'),
                expected_event_count: Some(1),
                actor: LearningActor::User,
                title: "Legacy writer must remain fenced".to_owned(),
                guidance: "This mutation must not reach the legacy learning ledger.".to_owned(),
                confidence_percent: 90,
                evidence: proposed
                    .learning
                    .evidence
                    .iter()
                    .map(|item| LearningEvidenceInput {
                        session_id: item.session_id.clone(),
                        record_id: item.record_id.clone(),
                        note: item.note.clone(),
                    })
                    .collect(),
                note: "Fence verification.".to_owned(),
            },
        )
        .is_err());
        assert_eq!(
            read_learning(&project, &vault, &proposed.learning.learning_id)
                .unwrap()
                .event_count,
            1
        );

        assert_eq!(
            ensure_native_learning_authority(&project, &vault, &store).unwrap(),
            identity.project_id
        );
        assert!(learning_authority_cutover_is_complete(&store, &identity.project_id).unwrap());
        assert_eq!(
            read_learning_with_continuity_transition(
                &project,
                &vault,
                &store,
                &proposed.learning.learning_id,
            )
            .unwrap(),
            proposed.learning
        );

        // A full legacy sync already in flight when the marker commits can finish against the
        // same fenced snapshot without invalidating the learning-authority marker.
        crate::import_legacy_continuity(&project, &vault, &store).unwrap();
        assert!(learning_authority_cutover_is_complete(&store, &identity.project_id).unwrap());

        assert!(matches!(
            crate::continuity_import::import_legacy_learning_continuity(
                &project,
                &vault,
                &store,
            ),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("forbidden after native learning authority cutover")
        ));
        assert!(learning_authority_cutover_is_complete(&store, &identity.project_id).unwrap());

        let cited_session =
            crate::read_session_with_continuity_transition(&project, &vault, &store, &session_id)
                .unwrap();
        let erased = crate::erase_session_memory_with_continuity_transition(
            &project,
            &vault,
            &store,
            &session_id,
            EraseSessionMemoryInput {
                expected_event_count: cited_session.event_count,
                expected_name: cited_session.name,
            },
        )
        .unwrap();
        assert!(erased
            .erased_learning_ids
            .contains(&proposed.learning.learning_id));
        assert!(matches!(
            read_learning_with_continuity_transition(
                &project,
                &vault,
                &store,
                &proposed.learning.learning_id,
            ),
            Err(LeyCoreError::LearningNotFound(_))
        ));
        assert!(read_learning(&project, &vault, &proposed.learning.learning_id).is_err());
        assert!(learning_authority_cutover_is_complete(&store, &identity.project_id).unwrap());
    }

    #[test]
    fn transition_learning_writers_cut_over_once_and_append_only_native_events() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        let identity = diagnose_project(&project).unwrap().identity;
        let store = private_continuity_store(&base, "transition-learning-writes");

        let proposed = propose_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            proposal(request_id('1'), &session_id, &record_id),
        )
        .unwrap();
        assert!(!proposed.replayed);
        assert_eq!(proposed.learning.event_count, 1);
        assert!(crate::session::session_authority_cutover_is_complete(
            &store,
            &identity.project_id
        )
        .unwrap());
        assert!(learning_authority_cutover_is_complete(&store, &identity.project_id).unwrap());
        assert!(read_learning(&project, &vault, &proposed.learning.learning_id).is_err());

        let corrected = correct_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            &proposed.learning.learning_id,
            CorrectLearningInput {
                request_id: request_id('2'),
                expected_event_count: Some(1),
                actor: LearningActor::User,
                title: "Native transition correction".to_owned(),
                guidance: "Keep the learning authority entirely in continuity.".to_owned(),
                confidence_percent: 93,
                evidence: proposed
                    .learning
                    .evidence
                    .iter()
                    .map(|item| LearningEvidenceInput {
                        session_id: item.session_id.clone(),
                        record_id: item.record_id.clone(),
                        note: item.note.clone(),
                    })
                    .collect(),
                note: "Correction through transition authority.".to_owned(),
            },
        )
        .unwrap();
        assert_eq!(corrected.learning.event_count, 2);
        assert_eq!(corrected.learning.title, "Native transition correction");

        let reviewed = review_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            &proposed.learning.learning_id,
            ReviewLearningInput {
                request_id: request_id('3'),
                expected_event_count: Some(2),
                actor: LearningActor::User,
                action: LearningFeedbackAction::Confirm,
                note: "Confirmed through native transition authority.".to_owned(),
                replacement_learning_id: None,
            },
        )
        .unwrap();
        assert_eq!(reviewed.learning.event_count, 3);
        assert_eq!(reviewed.learning.state, LearningState::Verified);
        assert_eq!(reviewed.learning.trust_state, LearningTrustState::Trusted);

        let events = store.learning_events(&identity.project_id).unwrap();
        assert_eq!(
            events
                .iter()
                .map(|event| event.kind.as_str())
                .collect::<Vec<_>>(),
            vec![
                "learning-proposed",
                "learning-corrected",
                "learning-reviewed"
            ]
        );
        assert!(propose_learning(
            &project,
            &vault,
            proposal(request_id('4'), &session_id, &record_id),
        )
        .is_err());
    }

    #[test]
    fn transition_learning_supersession_records_native_link_and_rejects_terminal_replacement() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        let identity = diagnose_project(&project).unwrap().identity;
        let store = private_continuity_store(&base, "transition-learning-supersession");
        let obsolete = propose_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            proposal(request_id('5'), &session_id, &record_id),
        )
        .unwrap();
        let replacement = propose_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            proposal(request_id('6'), &session_id, &record_id),
        )
        .unwrap();

        let superseded = review_learning_with_continuity_transition_guarded(
            &project,
            &vault,
            &store,
            &obsolete.learning.learning_id,
            Some(replacement.learning.event_count),
            ReviewLearningInput {
                request_id: request_id('7'),
                expected_event_count: Some(1),
                actor: LearningActor::User,
                action: LearningFeedbackAction::Supersede,
                note: "Use the replacement learning instead.".to_owned(),
                replacement_learning_id: Some(replacement.learning.learning_id.clone()),
            },
        )
        .unwrap();
        assert_eq!(superseded.learning.state, LearningState::Superseded);
        assert_eq!(
            superseded.learning.superseded_by.as_deref(),
            Some(replacement.learning.learning_id.as_str())
        );
        let links = store
            .linked_events(&identity.project_id, &superseded.event_id, "supersedes")
            .unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].subject_id.as_deref(),
            Some(replacement.learning.learning_id.as_str())
        );

        assert!(matches!(
            review_learning_with_continuity_transition(
                &project,
                &vault,
                &store,
                &replacement.learning.learning_id,
                ReviewLearningInput {
                    request_id: request_id('8'),
                    expected_event_count: Some(1),
                    actor: LearningActor::User,
                    action: LearningFeedbackAction::Supersede,
                    note: "The prior learning is already terminal.".to_owned(),
                    replacement_learning_id: Some(obsolete.learning.learning_id.clone()),
                },
            ),
            Err(LeyCoreError::InvalidLearningRequest(message))
                if message.contains("already superseded")
        ));
    }

    #[test]
    fn guarded_supersession_rejects_changed_or_terminal_replacement() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        let store = private_continuity_store(&base, "guarded-learning-supersession");

        let source = propose_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            proposal(request_id('1'), &session_id, &record_id),
        )
        .unwrap();
        let replacement = propose_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            proposal(request_id('2'), &session_id, &record_id),
        )
        .unwrap();
        let observed_replacement_events = replacement.learning.event_count;
        let changed = review_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            &replacement.learning.learning_id,
            ReviewLearningInput {
                request_id: request_id('3'),
                expected_event_count: Some(observed_replacement_events),
                actor: LearningActor::User,
                action: LearningFeedbackAction::Contest,
                note: "Replacement changed after the Desktop list was read.".to_owned(),
                replacement_learning_id: None,
            },
        )
        .unwrap();
        assert_eq!(
            changed.learning.event_count,
            observed_replacement_events + 1
        );

        let stale_replacement = review_learning_with_continuity_transition_guarded(
            &project,
            &vault,
            &store,
            &source.learning.learning_id,
            Some(observed_replacement_events),
            ReviewLearningInput {
                request_id: request_id('4'),
                expected_event_count: Some(source.learning.event_count),
                actor: LearningActor::User,
                action: LearningFeedbackAction::Supersede,
                note: "This must fail against the stale replacement view.".to_owned(),
                replacement_learning_id: Some(replacement.learning.learning_id.clone()),
            },
        )
        .unwrap_err();
        assert!(matches!(
            stale_replacement,
            LeyCoreError::InvalidLearningRequest(ref message)
                if message.contains("replacement learning changed")
        ));

        let source_two = propose_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            proposal(request_id('5'), &session_id, &record_id),
        )
        .unwrap();
        let terminal_replacement = propose_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            proposal(request_id('6'), &session_id, &record_id),
        )
        .unwrap();
        let rejected = review_learning_with_continuity_transition(
            &project,
            &vault,
            &store,
            &terminal_replacement.learning.learning_id,
            ReviewLearningInput {
                request_id: request_id('7'),
                expected_event_count: Some(terminal_replacement.learning.event_count),
                actor: LearningActor::User,
                action: LearningFeedbackAction::Reject,
                note: "This replacement is no longer reusable.".to_owned(),
                replacement_learning_id: None,
            },
        )
        .unwrap();
        assert_eq!(rejected.learning.state, LearningState::Rejected);

        let terminal_error = review_learning_with_continuity_transition_guarded(
            &project,
            &vault,
            &store,
            &source_two.learning.learning_id,
            Some(rejected.learning.event_count),
            ReviewLearningInput {
                request_id: request_id('8'),
                expected_event_count: Some(source_two.learning.event_count),
                actor: LearningActor::User,
                action: LearningFeedbackAction::Supersede,
                note: "Terminal replacements must fail closed.".to_owned(),
                replacement_learning_id: Some(terminal_replacement.learning.learning_id.clone()),
            },
        )
        .unwrap_err();
        assert!(matches!(
            terminal_error,
            LeyCoreError::InvalidLearningRequest(ref message)
                if message.contains("already rejected")
        ));
    }

    #[test]
    fn native_learning_mutation_preserves_idempotency_expected_count_and_session_dependency() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        let identity = diagnose_project(&project).unwrap().identity;
        let store = private_continuity_store(&base, "native-learning");
        crate::continuity_import::import_legacy_session_continuity(&project, &vault, &store)
            .unwrap();

        let (learning_id, proposal_pending) = native_proposal_pending(
            &project,
            &vault,
            &store,
            proposal(request_id('e'), &session_id, &record_id),
        );
        let first = mutate_learning_in_continuity_store(
            &project,
            &vault,
            &store,
            &identity,
            &learning_id,
            proposal_pending,
        )
        .unwrap();
        assert!(!first.replayed);
        assert_eq!(first.learning.event_count, 1);

        let (_, replay_pending) = native_proposal_pending(
            &project,
            &vault,
            &store,
            proposal(request_id('e'), &session_id, &record_id),
        );
        let replay = mutate_learning_in_continuity_store(
            &project,
            &vault,
            &store,
            &identity,
            &learning_id,
            replay_pending,
        )
        .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.event_id, first.event_id);
        assert_eq!(replay.learning, first.learning);

        let (_, mut conflict_pending) = native_proposal_pending(
            &project,
            &vault,
            &store,
            proposal(request_id('e'), &session_id, &record_id),
        );
        if let LearningEventPayload::Proposed { guidance, .. } = &mut conflict_pending.payload {
            *guidance = "Conflicting retry content.".to_owned();
        }
        assert!(matches!(
            mutate_learning_in_continuity_store(
                &project,
                &vault,
                &store,
                &identity,
                &learning_id,
                conflict_pending,
            ),
            Err(LeyCoreError::LearningIdempotencyConflict(_))
        ));

        let corrected = mutate_learning_in_continuity_store(
            &project,
            &vault,
            &store,
            &identity,
            &learning_id,
            native_correction_pending(
                &project,
                &vault,
                &store,
                &learning_id,
                request_id('f'),
                Some(1),
                &session_id,
                &record_id,
                "Native corrected learning",
            ),
        )
        .unwrap();
        assert_eq!(corrected.learning.event_count, 2);
        assert_eq!(corrected.learning.title, "Native corrected learning");

        assert!(matches!(
            mutate_learning_in_continuity_store(
                &project,
                &vault,
                &store,
                &identity,
                &learning_id,
                native_correction_pending(
                    &project,
                    &vault,
                    &store,
                    &learning_id,
                    request_id('g'),
                    Some(1),
                    &session_id,
                    &record_id,
                    "Stale native correction",
                ),
            ),
            Err(LeyCoreError::InvalidLearningRequest(message))
                if message.contains("learning changed from 1 events to 2")
        ));

        let events = store.learning_events(&identity.project_id).unwrap();
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|event| event.request_id.is_none()));
        assert_eq!(events[0].kind, "learning-proposed");
        assert_eq!(events[1].kind, "learning-corrected");
        let dependencies = store
            .linked_events(&identity.project_id, &first.event_id, "depends-on-session")
            .unwrap();
        assert_eq!(dependencies.len(), 1);
        assert_eq!(
            dependencies[0].session_id.as_deref(),
            Some(session_id.as_str())
        );

        let preview = store
            .preview_session_erasure(&identity.project_id, &session_id)
            .unwrap();
        assert!(preview.dependent_subject_ids.contains(&learning_id));
    }

    #[test]
    fn concurrent_native_learning_corrections_serialize_into_contiguous_sequences() {
        let (base, project, vault, session_id, record_id) = setup_learning();
        let identity = diagnose_project(&project).unwrap().identity;
        let store = private_continuity_store(&base, "native-learning-concurrency");
        crate::continuity_import::import_legacy_session_continuity(&project, &vault, &store)
            .unwrap();
        let (learning_id, proposal_pending) = native_proposal_pending(
            &project,
            &vault,
            &store,
            proposal(request_id('a'), &session_id, &record_id),
        );
        mutate_learning_in_continuity_store(
            &project,
            &vault,
            &store,
            &identity,
            &learning_id,
            proposal_pending,
        )
        .unwrap();

        let pending = [
            native_correction_pending(
                &project,
                &vault,
                &store,
                &learning_id,
                request_id('b'),
                None,
                &session_id,
                &record_id,
                "Concurrent native correction A",
            ),
            native_correction_pending(
                &project,
                &vault,
                &store,
                &learning_id,
                request_id('c'),
                None,
                &session_id,
                &record_id,
                "Concurrent native correction B",
            ),
        ];
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();
        for pending in pending {
            let barrier = Arc::clone(&barrier);
            let project = project.clone();
            let vault = vault.clone();
            let store = store.clone();
            let identity = identity.clone();
            let learning_id = learning_id.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                mutate_learning_in_continuity_store(
                    &project,
                    &vault,
                    &store,
                    &identity,
                    &learning_id,
                    pending,
                )
                .unwrap()
            }));
        }
        barrier.wait();
        let results = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        assert!(results.iter().all(|result| !result.replayed));

        let events = store
            .learning_events(&identity.project_id)
            .unwrap()
            .into_iter()
            .map(learning_event_from_continuity)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut sequences = events
            .iter()
            .filter(|event| event.learning_id == learning_id)
            .map(|event| event.sequence)
            .collect::<Vec<_>>();
        sequences.sort_unstable();
        assert_eq!(sequences, vec![1, 2, 3]);
        let replayed = replay_one(&events, &identity.project_id, &learning_id).unwrap();
        assert_eq!(replayed.event_count, 3);
    }

    #[test]
    fn previous_schema_v2_lineage_remains_readable() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let proposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('2'), &session_id, &record_id),
        )
        .unwrap();
        let event_path = learning_directory(&project, &vault)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", proposed.event_id));
        let mut event: LearningEvent =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        event.schema_version = PREVIOUS_LEARNING_SCHEMA_VERSION;
        let artifact = proposed.learning.evidence[0].artifacts[0].clone();
        if let LearningEventPayload::Proposed { origin_lineage, .. } = &mut event.payload {
            *origin_lineage = Some(LearningOriginLineage {
                mechanically_resolved: true,
                causal_completeness_proven: false,
                omitted_sources: 0,
                automatic_authority_ceiling: LearningTrustState::ReviewRequired,
                sources: vec![
                    LearningOriginSource::SessionRecord {
                        session_id: session_id.clone(),
                        record_id: record_id.clone(),
                        record_type: proposed.learning.evidence[0].record_type.clone(),
                    },
                    LearningOriginSource::CapturedArtifact {
                        artifact_snapshot_id: artifact.artifact_snapshot_id,
                        artifact_path: artifact.artifact_path,
                        content_hash: artifact.content_hash,
                    },
                    LearningOriginSource::TurnEvidence {
                        session_id: session_id.clone(),
                        record_id: format!("tev_{}", "a".repeat(32)),
                    },
                    LearningOriginSource::RecoveryCandidate {
                        session_id: session_id.clone(),
                        candidate_fingerprint: format!("sha256:{}", "b".repeat(64)),
                    },
                ],
            });
        } else {
            panic!("expected proposed event");
        }
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.learning_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();

        let previous = read_learning(&project, &vault, &proposed.learning.learning_id).unwrap();
        assert_eq!(previous.schema_version, LEARNING_SCHEMA_VERSION);
        assert!(previous.origin_lineage.mechanically_resolved);
        assert!(previous
            .origin_lineage
            .sources
            .iter()
            .any(|source| matches!(source, LearningOriginSource::SessionRecord { .. })));
        assert!(previous
            .origin_lineage
            .sources
            .iter()
            .any(|source| matches!(source, LearningOriginSource::CapturedArtifact { .. })));
        assert!(previous
            .origin_lineage
            .sources
            .iter()
            .any(|source| matches!(source, LearningOriginSource::TurnEvidence { .. })));
        assert!(previous
            .origin_lineage
            .sources
            .iter()
            .any(|source| matches!(source, LearningOriginSource::RecoveryCandidate { .. })));
        assert!(!previous
            .origin_lineage
            .sources
            .iter()
            .any(|source| matches!(source, LearningOriginSource::ToolEvidence { .. })));
    }

    #[test]
    fn legacy_lineage_is_explicitly_incomplete_and_lineage_tampering_fails_closed() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let proposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('3'), &session_id, &record_id),
        )
        .unwrap();
        let event_path = learning_directory(&project, &vault)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", proposed.event_id));
        let mut event: LearningEvent =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        event.schema_version = LEGACY_LEARNING_SCHEMA_VERSION;
        if let LearningEventPayload::Proposed { origin_lineage, .. } = &mut event.payload {
            *origin_lineage = None;
        } else {
            panic!("expected proposed event");
        }
        event.request_fingerprint = request_fingerprint(
            &event.project_id,
            &event.learning_id,
            &event.request_id,
            &event.payload,
        )
        .unwrap();
        std::fs::write(&event_path, serde_json::to_vec_pretty(&event).unwrap()).unwrap();
        let legacy = read_learning(&project, &vault, &proposed.learning.learning_id).unwrap();
        assert!(!legacy.origin_lineage.mechanically_resolved);
        assert!(!legacy.origin_lineage.causal_completeness_proven);
        assert_eq!(
            legacy.origin_lineage.automatic_authority_ceiling,
            LearningTrustState::ReviewRequired
        );

        let reproposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('4'), &session_id, &record_id),
        )
        .unwrap();
        let tamper_path = learning_directory(&project, &vault)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", reproposed.event_id));
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&tamper_path).unwrap()).unwrap();
        let sources = value["data"]["originLineage"]["sources"]
            .as_array_mut()
            .unwrap();
        let session_source = sources
            .iter_mut()
            .find(|source| source["kind"] == "session-record")
            .unwrap();
        session_source["recordType"] = serde_json::json!("tampered-origin");
        std::fs::write(&tamper_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            read_learning(&project, &vault, &reproposed.learning.learning_id),
            Err(LeyCoreError::InvalidLearningStore(_))
        ));
    }

    #[test]
    fn origin_lineage_is_bounded_and_discloses_omitted_sources() {
        let mut builder = LearningOriginBuilder::new();
        for index in 0..(MAX_LEARNING_ORIGIN_SOURCES + 5) {
            builder.push(LearningOriginSource::SessionRecord {
                session_id: format!("ses_{:032x}", index + 1),
                record_id: format!("dec_{:032x}", index + 1),
                record_type: "decision".to_owned(),
            });
        }
        let lineage = builder.finish(true);
        assert_eq!(lineage.sources.len(), MAX_LEARNING_ORIGIN_SOURCES);
        assert_eq!(lineage.omitted_sources, 5);
        assert!(!lineage.mechanically_resolved);
        assert!(!lineage.causal_completeness_proven);
        assert_eq!(
            lineage.automatic_authority_ceiling,
            LearningTrustState::ReviewRequired
        );
    }

    #[test]
    fn only_user_authority_can_trust_or_terminally_review_a_learning() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let proposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('3'), &session_id, &record_id),
        )
        .unwrap();
        assert!(matches!(
            review_learning(
                &project,
                &vault,
                &proposed.learning.learning_id,
                ReviewLearningInput {
                    request_id: request_id('4'),
                    expected_event_count: None,
                    actor: LearningActor::Agent,
                    action: LearningFeedbackAction::Confirm,
                    note: String::new(),
                    replacement_learning_id: None,
                }
            ),
            Err(LeyCoreError::InvalidLearningRequest(_))
        ));
        let proposed_lineage = proposed.learning.origin_lineage.clone();
        let confirmed = review_learning(
            &project,
            &vault,
            &proposed.learning.learning_id,
            ReviewLearningInput {
                request_id: request_id('5'),
                expected_event_count: Some(proposed.learning.event_count),
                actor: LearningActor::User,
                action: LearningFeedbackAction::Confirm,
                note: "I verified this procedure.".to_owned(),
                replacement_learning_id: None,
            },
        )
        .unwrap();
        assert_eq!(confirmed.learning.state, LearningState::Verified);
        assert_eq!(confirmed.learning.trust_state, LearningTrustState::Trusted);
        assert_eq!(confirmed.learning.origin_lineage, proposed_lineage);
        assert!(!confirmed.learning.origin_lineage.causal_completeness_proven);

        assert!(matches!(
            correct_learning(
                &project,
                &vault,
                &proposed.learning.learning_id,
                CorrectLearningInput {
                    request_id: request_id('6'),
                    expected_event_count: Some(proposed.learning.event_count),
                    actor: LearningActor::User,
                    title: "Stale correction".to_owned(),
                    guidance: "This stale editor must not overwrite a newer review.".to_owned(),
                    confidence_percent: 90,
                    evidence: vec![LearningEvidenceInput {
                        session_id: session_id.clone(),
                        record_id: record_id.clone(),
                        note: String::new(),
                    }],
                    note: "Started before confirmation.".to_owned(),
                },
            ),
            Err(LeyCoreError::InvalidLearningRequest(message))
                if message.contains("reload before saving")
        ));

        let corrected = correct_learning(
            &project,
            &vault,
            &proposed.learning.learning_id,
            CorrectLearningInput {
                request_id: request_id('7'),
                expected_event_count: Some(confirmed.learning.event_count),
                actor: LearningActor::User,
                title: "Check and test the workspace".to_owned(),
                guidance: "Run cargo check --workspace and cargo test --workspace.".to_owned(),
                confidence_percent: 90,
                evidence: vec![LearningEvidenceInput {
                    session_id: session_id.clone(),
                    record_id: record_id.clone(),
                    note: String::new(),
                }],
                note: "A later correction expanded the verification.".to_owned(),
            },
        )
        .unwrap();
        assert_eq!(corrected.learning.state, LearningState::Tentative);
        assert_eq!(
            corrected.learning.trust_state,
            LearningTrustState::ReviewRequired
        );
        assert_eq!(corrected.learning.event_count, 3);
        assert!(matches!(
            review_learning(
                &project,
                &vault,
                &proposed.learning.learning_id,
                ReviewLearningInput {
                    request_id: request_id('8'),
                    expected_event_count: Some(confirmed.learning.event_count),
                    actor: LearningActor::User,
                    action: LearningFeedbackAction::Confirm,
                    note: "This stale review must not trust unseen text.".to_owned(),
                    replacement_learning_id: None,
                },
            ),
            Err(LeyCoreError::InvalidLearningRequest(message))
                if message.contains("reload before saving")
        ));
    }

    #[test]
    fn secrets_are_redacted_from_events_index_and_review_markdown() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let secret = "sk-abcdefghijklmnopqrstuvwxyz123456";
        let mut input = proposal(request_id('3'), &session_id, &record_id);
        input.guidance = format!("Run the check with token {secret}");
        input.evidence[0].note = format!("Observed using {secret}");
        propose_learning(&project, &vault, input).unwrap();

        let mut stored = String::new();
        collect_file_text(&learning_directory(&project, &vault), &mut stored);
        assert!(!stored.contains(secret));
        assert!(stored.contains("[REDACTED:provider-token]"));
        assert!(stored.contains("\"redactions\""));
    }

    #[test]
    fn concurrent_proposals_are_serialized_without_lost_events() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let barrier = Arc::new(Barrier::new(5));
        let mut workers = Vec::new();
        for digit in ['3', '4', '5', '6'] {
            let project = project.clone();
            let vault = vault.clone();
            let session_id = session_id.clone();
            let record_id = record_id.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                propose_learning(
                    project,
                    vault,
                    proposal(request_id(digit), &session_id, &record_id),
                )
                .unwrap();
            }));
        }
        barrier.wait();
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(list_learnings(&project, &vault).unwrap().len(), 4);
        assert_eq!(
            std::fs::read_dir(learning_directory(&project, &vault).join(EVENTS_DIRECTORY))
                .unwrap()
                .count(),
            4
        );
    }

    #[test]
    fn session_erasure_physically_removes_cited_and_dependent_learnings_only() {
        let (_base, project, vault, erased_session_id, erased_record_id) = setup_learning();
        let erased_session = read_session(&project, &vault, &erased_session_id).unwrap();
        let cited = propose_learning(
            &project,
            &vault,
            proposal(request_id('3'), &erased_session_id, &erased_record_id),
        )
        .unwrap();
        let (retained_session_id, retained_record_id) = add_learning_session(
            &project,
            &vault,
            request_id('4'),
            request_id('5'),
            "Independent session",
        );
        let mut retained_input =
            proposal(request_id('6'), &retained_session_id, &retained_record_id);
        retained_input.title = "Keep the independent workflow".to_owned();
        retained_input.guidance = "Run the independent workflow before delivery.".to_owned();
        let retained = propose_learning(&project, &vault, retained_input).unwrap();
        let mut dependent_input =
            proposal(request_id('7'), &retained_session_id, &retained_record_id);
        dependent_input.title = "Use the replaced workflow".to_owned();
        dependent_input.guidance =
            "This learning points to a replacement that will be erased.".to_owned();
        let dependent = propose_learning(&project, &vault, dependent_input).unwrap();
        let dependent_review = review_learning(
            &project,
            &vault,
            &dependent.learning.learning_id,
            ReviewLearningInput {
                request_id: request_id('8'),
                expected_event_count: Some(dependent.learning.event_count),
                actor: LearningActor::User,
                action: LearningFeedbackAction::Supersede,
                note: "Use the cited replacement.".to_owned(),
                replacement_learning_id: Some(cited.learning.learning_id.clone()),
            },
        )
        .unwrap();
        let memory_before = project_memory_overview(&project, &vault).unwrap();
        let erased_event_paths = [
            learning_directory(&project, &vault)
                .join(EVENTS_DIRECTORY)
                .join(format!("{}.json", cited.event_id)),
            learning_directory(&project, &vault)
                .join(EVENTS_DIRECTORY)
                .join(format!("{}.json", dependent.event_id)),
            learning_directory(&project, &vault)
                .join(EVENTS_DIRECTORY)
                .join(format!("{}.json", dependent_review.event_id)),
        ];

        let receipt = erase_session_memory(
            &project,
            &vault,
            &erased_session_id,
            EraseSessionMemoryInput {
                expected_event_count: erased_session.event_count,
                expected_name: erased_session.name.clone(),
            },
        )
        .unwrap();

        let mut expected_erased = vec![
            cited.learning.learning_id.clone(),
            dependent.learning.learning_id.clone(),
        ];
        expected_erased.sort();
        assert_eq!(receipt.erased_learning_ids, expected_erased);
        assert!(receipt.ordinary_notes_preserved);
        assert!(receipt.canvas_documents_preserved);
        assert!(receipt.project_evidence_preserved);
        assert!(!session_directory(&project, &vault, &erased_session_id).exists());
        assert!(matches!(
            read_session(&project, &vault, &erased_session_id),
            Err(LeyCoreError::SessionNotFound(_))
        ));
        assert_eq!(
            read_session(&project, &vault, &retained_session_id)
                .unwrap()
                .name,
            "Independent session"
        );
        assert!(matches!(
            read_learning(&project, &vault, &cited.learning.learning_id),
            Err(LeyCoreError::LearningNotFound(_))
        ));
        assert!(matches!(
            read_learning(&project, &vault, &dependent.learning.learning_id),
            Err(LeyCoreError::LearningNotFound(_))
        ));
        assert_eq!(
            read_learning(&project, &vault, &retained.learning.learning_id)
                .unwrap()
                .title,
            "Keep the independent workflow"
        );
        assert!(erased_event_paths.iter().all(|path| !path.exists()));
        assert!(learning_directory(&project, &vault)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", retained.event_id))
            .is_file());
        let projections = format!(
            "{}\n{}",
            std::fs::read_to_string(learning_directory(&project, &vault).join(LEARNING_INDEX_FILE))
                .unwrap(),
            std::fs::read_to_string(
                learning_directory(&project, &vault).join(LEARNING_REVIEW_FILE)
            )
            .unwrap()
        );
        assert!(!projections.contains(&cited.learning.learning_id));
        assert!(!projections.contains(&dependent.learning.learning_id));
        assert!(projections.contains(&retained.learning.learning_id));
        let memory_after = project_memory_overview(&project, &vault).unwrap();
        assert_eq!(
            memory_after.artifact_snapshot_id,
            memory_before.artifact_snapshot_id
        );
        assert!(project.join("README.md").is_file());
        assert!(project.join(".ley/project.json").is_file());
    }

    #[test]
    fn session_erasure_requires_current_event_count_and_exact_name_without_side_effects() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let learning = propose_learning(
            &project,
            &vault,
            proposal(request_id('3'), &session_id, &record_id),
        )
        .unwrap();
        let session = read_session(&project, &vault, &session_id).unwrap();

        assert!(matches!(
            erase_session_memory(
                &project,
                &vault,
                &session_id,
                EraseSessionMemoryInput {
                    expected_event_count: session.event_count.saturating_sub(1),
                    expected_name: session.name.clone(),
                },
            ),
            Err(LeyCoreError::InvalidSessionRequest(message))
                if message.contains("reload before erasing")
        ));
        assert!(matches!(
            erase_session_memory(
                &project,
                &vault,
                &session_id,
                EraseSessionMemoryInput {
                    expected_event_count: session.event_count,
                    expected_name: session.name.to_lowercase(),
                },
            ),
            Err(LeyCoreError::InvalidSessionRequest(message))
                if message.contains("type the current name")
        ));
        assert_eq!(
            read_session(&project, &vault, &session_id)
                .unwrap()
                .event_count,
            session.event_count
        );
        assert_eq!(
            read_learning(&project, &vault, &learning.learning.learning_id)
                .unwrap()
                .event_count,
            1
        );
    }

    #[test]
    fn corrupt_learning_evidence_aborts_session_erasure_before_any_deletion() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let learning = propose_learning(
            &project,
            &vault,
            proposal(request_id('3'), &session_id, &record_id),
        )
        .unwrap();
        let session = read_session(&project, &vault, &session_id).unwrap();
        let event_path = learning_directory(&project, &vault)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", learning.event_id));
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        value["sequence"] = serde_json::json!(7);
        std::fs::write(&event_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();

        let result = erase_session_memory(
            &project,
            &vault,
            &session_id,
            EraseSessionMemoryInput {
                expected_event_count: session.event_count,
                expected_name: session.name.clone(),
            },
        );
        assert!(
            matches!(result, Err(LeyCoreError::InvalidLearningStore(_))),
            "unexpected erasure result: {result:?}"
        );
        assert!(session_directory(&project, &vault, &session_id).is_dir());
        assert!(event_path.is_file());
    }

    #[test]
    fn session_erasure_waits_for_active_memory_users() {
        let (_base, project, vault, session_id, _record_id) = setup_learning();
        let session = read_session(&project, &vault, &session_id).unwrap();
        let project_id = diagnose_project(&project).unwrap().identity.project_id;
        let reader = lock_project_memory_lifecycle(&vault, &project_id, false, false).unwrap();
        let erase_project = project.clone();
        let erase_vault = vault.clone();
        let erase_session_id = session_id.clone();
        let (finished_tx, finished_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let result = erase_session_memory(
                erase_project,
                erase_vault,
                &erase_session_id,
                EraseSessionMemoryInput {
                    expected_event_count: session.event_count,
                    expected_name: session.name,
                },
            );
            finished_tx.send(result).unwrap();
        });

        assert!(finished_rx.recv_timeout(Duration::from_millis(80)).is_err());
        drop(reader);
        assert_eq!(
            finished_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap()
                .session_id,
            session_id
        );
        worker.join().unwrap();
    }

    #[test]
    fn terminal_reviews_and_corrupted_or_symlinked_events_are_rejected() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let proposed = propose_learning(
            &project,
            &vault,
            proposal(request_id('3'), &session_id, &record_id),
        )
        .unwrap();
        review_learning(
            &project,
            &vault,
            &proposed.learning.learning_id,
            ReviewLearningInput {
                request_id: request_id('4'),
                expected_event_count: None,
                actor: LearningActor::User,
                action: LearningFeedbackAction::Reject,
                note: "This is not a valid project rule.".to_owned(),
                replacement_learning_id: None,
            },
        )
        .unwrap();
        assert!(matches!(
            correct_learning(
                &project,
                &vault,
                &proposed.learning.learning_id,
                CorrectLearningInput {
                    request_id: request_id('5'),
                    expected_event_count: None,
                    actor: LearningActor::User,
                    title: "Changed".to_owned(),
                    guidance: "This must not be appended.".to_owned(),
                    confidence_percent: 100,
                    evidence: vec![LearningEvidenceInput {
                        session_id,
                        record_id,
                        note: String::new(),
                    }],
                    note: String::new(),
                }
            ),
            Err(LeyCoreError::InvalidLearningRequest(_))
        ));

        let event_path = learning_directory(&project, &vault)
            .join(EVENTS_DIRECTORY)
            .join(format!("{}.json", proposed.event_id));
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&event_path).unwrap()).unwrap();
        value["sequence"] = serde_json::json!(9);
        std::fs::write(&event_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            read_learning(&project, &vault, &proposed.learning.learning_id),
            Err(LeyCoreError::InvalidLearningStore(_))
        ));
    }

    #[test]
    fn supersession_requires_an_existing_acyclic_replacement_before_append() {
        let (_base, project, vault, session_id, record_id) = setup_learning();
        let original = propose_learning(
            &project,
            &vault,
            proposal(request_id('3'), &session_id, &record_id),
        )
        .unwrap();
        let mut replacement_input = proposal(request_id('4'), &session_id, &record_id);
        replacement_input.title = "Use the release verification workflow".to_owned();
        replacement_input.guidance = "Run the documented release verification workflow.".to_owned();
        let replacement = propose_learning(&project, &vault, replacement_input).unwrap();

        let superseded = review_learning(
            &project,
            &vault,
            &original.learning.learning_id,
            ReviewLearningInput {
                request_id: request_id('5'),
                expected_event_count: None,
                actor: LearningActor::User,
                action: LearningFeedbackAction::Supersede,
                note: "The replacement reflects the current workflow.".to_owned(),
                replacement_learning_id: Some(replacement.learning.learning_id.clone()),
            },
        )
        .unwrap();
        assert_eq!(superseded.learning.state, LearningState::Superseded);
        assert_eq!(
            superseded.learning.superseded_by.as_deref(),
            Some(replacement.learning.learning_id.as_str())
        );

        assert!(matches!(
            review_learning(
                &project,
                &vault,
                &replacement.learning.learning_id,
                ReviewLearningInput {
                    request_id: request_id('6'),
                    expected_event_count: None,
                    actor: LearningActor::User,
                    action: LearningFeedbackAction::Supersede,
                    note: "This cycle must never enter the ledger.".to_owned(),
                    replacement_learning_id: Some(original.learning.learning_id),
                },
            ),
            Err(LeyCoreError::InvalidLearningRequest(_))
        ));
        assert_eq!(
            read_learning(&project, &vault, &replacement.learning.learning_id)
                .unwrap()
                .event_count,
            1
        );
        assert_eq!(list_learnings(&project, &vault).unwrap().len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_learning_event_is_never_followed() {
        use std::os::unix::fs::symlink;

        let (_base, project, vault, session_id, record_id) = setup_learning();
        propose_learning(
            &project,
            &vault,
            proposal(request_id('3'), &session_id, &record_id),
        )
        .unwrap();
        symlink(
            "/etc/passwd",
            learning_directory(&project, &vault)
                .join(EVENTS_DIRECTORY)
                .join(format!("lev_{}.json", "f".repeat(64))),
        )
        .unwrap();
        assert!(matches!(
            list_learnings(&project, &vault),
            Err(LeyCoreError::InvalidLearningStore(_)) | Err(LeyCoreError::Io { .. })
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
