use crate::ingestion::{
    load_project_memory, lock_project_memory_lifecycle, redact_secrets, ProjectMemoryLifecycleLock,
};
use crate::{
    diagnose_project, project_memory_overview, read_session, LeyCoreError, RedactionFinding,
    SessionArtifactCitation, SessionStatus,
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

pub const LEARNING_SCHEMA_VERSION: u32 = 1;
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
    pub confidence_percent: u8,
    pub freshness: LearningFreshness,
    pub corroborating_sessions: usize,
    pub updated_at_unix_ms: u64,
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
    },
    Corrected {
        actor: LearningActor,
        title: String,
        guidance: String,
        confidence_percent: u8,
        evidence: Vec<LearningEvidence>,
        note: String,
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
    validate_request_id(&input.request_id)?;
    validate_confidence(input.confidence_percent)?;
    validate_provenance_authority(input.actor, input.provenance)?;
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
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
    let payload = LearningEventPayload::Proposed {
        actor: input.actor,
        kind: input.kind,
        title: sanitize_text("title", &input.title, 1, 256, &mut redactions)?,
        guidance: sanitize_text("guidance", &input.guidance, 1, 16_000, &mut redactions)?,
        confidence_percent: input.confidence_percent,
        provenance: input.provenance,
        evidence: resolve_evidence(
            &diagnostic.root,
            vault.as_ref(),
            input.evidence,
            &mut redactions,
        )?,
    };
    mutate_learning(
        &diagnostic.identity.project_id,
        &diagnostic.root,
        &learning_id,
        PendingLearningEvent {
            event_id,
            request_id: input.request_id,
            expected_event_count: None,
            redactions,
            payload,
            allow_create: true,
        },
        vault,
    )
}

pub fn correct_learning(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    learning_id: &str,
    input: CorrectLearningInput,
) -> Result<LearningMutation, LeyCoreError> {
    validate_learning_id(learning_id)?;
    validate_request_id(&input.request_id)?;
    validate_confidence(input.confidence_percent)?;
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
    let event_id = deterministic_id(
        "lev",
        &format!("{learning_id}:{}:corrected", input.request_id),
        64,
    );
    let mut redactions = Vec::new();
    let payload = LearningEventPayload::Corrected {
        actor: input.actor,
        title: sanitize_text("title", &input.title, 1, 256, &mut redactions)?,
        guidance: sanitize_text("guidance", &input.guidance, 1, 16_000, &mut redactions)?,
        confidence_percent: input.confidence_percent,
        evidence: resolve_evidence(
            &diagnostic.root,
            vault.as_ref(),
            input.evidence,
            &mut redactions,
        )?,
        note: sanitize_text("note", &input.note, 0, 4_000, &mut redactions)?,
    };
    mutate_learning(
        &diagnostic.identity.project_id,
        &diagnostic.root,
        learning_id,
        PendingLearningEvent {
            event_id,
            request_id: input.request_id,
            expected_event_count: input.expected_event_count,
            redactions,
            payload,
            allow_create: false,
        },
        vault,
    )
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
    project_memory_overview(&diagnostic.root, &vault)?;
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
            redactions,
            payload,
            allow_create: false,
        },
        vault,
    )
}

pub fn read_learning(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    learning_id: &str,
) -> Result<LearningRecord, LeyCoreError> {
    validate_learning_id(learning_id)?;
    let diagnostic = diagnose_project(&project_start)?;
    project_memory_overview(&diagnostic.root, &vault)?;
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
    project_memory_overview(&diagnostic.root, &vault)?;
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

pub fn learning_review_inbox(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
) -> Result<Vec<LearningSummary>, LeyCoreError> {
    Ok(list_learnings(project_start, vault)?
        .into_iter()
        .filter(summary_needs_review)
        .collect())
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
            confidence_percent: learning.confidence_percent,
            freshness: learning.freshness,
            corroborating_sessions: learning.corroborating_sessions,
            updated_at_unix_ms: learning.updated_at_unix_ms,
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

fn resolve_evidence(
    project: &Path,
    vault: &Path,
    inputs: Vec<LearningEvidenceInput>,
    redactions: &mut Vec<LearningRedaction>,
) -> Result<Vec<LearningEvidence>, LeyCoreError> {
    if inputs.is_empty() || inputs.len() > 20 {
        return Err(LeyCoreError::InvalidLearningRequest(
            "learning evidence must contain 1 to 20 session references".to_owned(),
        ));
    }
    let mut unique = BTreeSet::new();
    let mut evidence = Vec::new();
    for (index, input) in inputs.into_iter().enumerate() {
        if !unique.insert((input.session_id.clone(), input.record_id.clone())) {
            return Err(LeyCoreError::InvalidLearningRequest(
                "learning evidence contains a duplicate session record".to_owned(),
            ));
        }
        let session = read_session(project, vault, &input.session_id)?;
        let (record_type, artifacts) = locate_session_record(&session, &input.record_id)?;
        evidence.push(LearningEvidence {
            session_id: session.session_id,
            record_id: input.record_id,
            record_type,
            session_status: session.status,
            session_updated_at_unix_ms: session.updated_at_unix_ms,
            note: sanitize_text(
                &format!("evidence[{index}].note"),
                &input.note,
                0,
                2_000,
                redactions,
            )?,
            artifacts,
        });
    }
    evidence.sort_by(|left, right| {
        left.session_id
            .cmp(&right.session_id)
            .then_with(|| left.record_id.cmp(&right.record_id))
    });
    Ok(evidence)
}

fn locate_session_record(
    session: &crate::AgentSession,
    record_id: &str,
) -> Result<(String, Vec<SessionArtifactCitation>), LeyCoreError> {
    if record_id == session.session_id {
        return Ok(("session".to_owned(), Vec::new()));
    }
    if session
        .finish
        .as_ref()
        .is_some_and(|finish| finish.event_id == record_id)
    {
        return Ok(("session-finish".to_owned(), Vec::new()));
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
            return Ok((record_type.to_owned(), checkpoint.touched_artifacts.clone()));
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
    redactions: Vec<LearningRedaction>,
    payload: LearningEventPayload,
    allow_create: bool,
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
    let fingerprint = request_fingerprint(
        project_id,
        learning_id,
        &pending.request_id,
        &pending.payload,
    )?;
    let event_name = format!("{}.json", pending.event_id);
    if let Some(existing) =
        read_private_file(&store.events_dir, &event_name, LEARNING_EVENT_LIMIT_BYTES)?
    {
        let event: LearningEvent = parse_json(&event_name, &existing)?;
        validate_event(&event, project_id)?;
        if event.request_fingerprint != fingerprint {
            return Err(LeyCoreError::LearningIdempotencyConflict(
                pending.request_id,
            ));
        }
        let mut records = replay_all(&events, project_id)?;
        refresh_all_freshness(&mut records, project_root, &vault)?;
        store.persist_index(&records)?;
        let learning = records
            .into_iter()
            .find(|record| record.learning_id == learning_id)
            .ok_or_else(|| LeyCoreError::LearningNotFound(learning_id.to_owned()))?;
        return Ok(learning_mutation(learning, &pending.event_id, true));
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
        let current = replay_one(&events, project_id, learning_id)?;
        validate_transition(&current, &pending.payload)?;
        validate_pending_against_ledger(&events, project_id, learning_id, &pending.payload)?;
    }
    if events.len() >= LEARNING_EVENT_LIMIT {
        return Err(LeyCoreError::InvalidLearningStore(format!(
            "project exceeds {LEARNING_EVENT_LIMIT} learning events"
        )));
    }
    let sequence = learning_events.len() as u64 + 1;
    let minimum_time = learning_events
        .last()
        .map(|event| event.recorded_at_unix_ms)
        .unwrap_or(1);
    let recorded_at_unix_ms = unix_time_ms().max(minimum_time);
    let event = LearningEvent {
        schema_version: LEARNING_SCHEMA_VERSION,
        event_id: pending.event_id.clone(),
        project_id: project_id.to_owned(),
        learning_id: learning_id.to_owned(),
        request_id: pending.request_id,
        request_fingerprint: fingerprint,
        sequence,
        recorded_at_unix_ms,
        redactions: pending.redactions,
        payload: pending.payload,
    };
    let body = json_body(&event, LEARNING_EVENT_LIMIT_BYTES, &event_name)?;
    write_immutable_private(&store.events_dir, &event_name, &body)?;
    let events = store.read_events()?;
    let mut records = replay_all(&events, project_id)?;
    refresh_all_freshness(&mut records, project_root, &vault)?;
    store.persist_index(&records)?;
    let learning = records
        .into_iter()
        .find(|record| record.learning_id == learning_id)
        .ok_or_else(|| LeyCoreError::LearningNotFound(learning_id.to_owned()))?;
    Ok(learning_mutation(learning, &pending.event_id, false))
}

fn validate_pending_against_ledger(
    events: &[LearningEvent],
    project_id: &str,
    learning_id: &str,
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
            } => {
                learning.title = title.clone();
                learning.guidance = guidance.clone();
                learning.confidence_percent = *confidence_percent;
                learning.evidence = evidence.clone();
                learning.corroborating_sessions = corroboration_count(evidence);
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
    if index.schema_version != LEARNING_SCHEMA_VERSION || index.project_id != project_id {
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
    if event.schema_version != LEARNING_SCHEMA_VERSION
        || event.project_id != project_id
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
    validate_event_payload(&event.payload)?;
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

fn validate_event_payload(payload: &LearningEventPayload) -> Result<(), LeyCoreError> {
    match payload {
        LearningEventPayload::Proposed {
            actor,
            title,
            guidance,
            confidence_percent,
            provenance,
            evidence,
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
        }
        LearningEventPayload::Corrected {
            title,
            guidance,
            confidence_percent,
            evidence,
            note,
            ..
        } => {
            validate_stored_text("title", title, 1, 256)?;
            validate_stored_text("guidance", guidance, 1, 16_000)?;
            validate_stored_text("note", note, 0, 4_000)?;
            validate_stored_confidence(*confidence_percent)?;
            validate_stored_evidence(evidence)?;
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
                || citation.start_line == 0
                || citation.end_line < citation.start_line
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
        checkpoint_session, erase_session_memory, ingest_project, initialize_project,
        project_memory_overview, read_session, start_session, AttemptInput, AttemptOutcome,
        CaptureMode, CheckpointInput, EraseSessionMemoryInput, ProblemInput, ResolutionInput,
        SessionSource, SessionSourceKind, StartSessionInput,
    };
    use std::sync::mpsc;
    use std::sync::{Arc, Barrier};
    use std::time::Duration;
    use tempfile::tempdir;

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
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
