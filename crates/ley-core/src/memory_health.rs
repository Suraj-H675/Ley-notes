use crate::revision::RevisionResolver;
use crate::{
    compile_session_memory, list_learnings, list_sessions, project_memory_overview, read_session,
    LearningFreshness, LearningSummary, LearningTrustState, LeyCoreError, MemoryCompilationState,
    ProjectRevisionFreshness, RevisionCompatibility, SessionStatus, TaskStatus,
    MAX_MEMORY_COMPILE_RESULTS, MIN_MEMORY_COMPILE_CHARACTERS,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const MEMORY_HEALTH_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_MEMORY_HEALTH_SIGNALS: usize = 100;
pub const MAX_MEMORY_HEALTH_SIGNALS: usize = 200;
pub const DEFAULT_MEMORY_HEALTH_SESSIONS: usize = 20;
pub const MAX_MEMORY_HEALTH_SESSIONS: usize = 50;
pub const DEFAULT_MEMORY_HEALTH_CHARACTERS: usize = 16_000;
pub const MIN_MEMORY_HEALTH_CHARACTERS: usize = 2_000;
pub const MAX_MEMORY_HEALTH_CHARACTERS: usize = 32_000;

const SOURCE_BOUNDARY: &str = "derived-memory-health-advisory";
const INSTRUCTION_WARNING: &str = "Memory Health is an advisory, rebuildable diagnostic over Ley's existing memory state. Signals do not grant authority, do not prove live source, and do not automatically delete, suppress, rewrite, confirm, contest, stale, or supersede any memory. Review the cited stable IDs and current source before taking maintenance action.";
const PRIVACY_NOTICE: &str = "Ley builds Memory Health on demand from the fixed project's learning index, structured sessions, recovery compiler metadata, and bounded Git freshness metadata. The report returns stable IDs and bounded diagnostic text, not absolute project/vault paths, and persists no health cache.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryHealthLimits {
    pub max_signals: usize,
    pub max_sessions: usize,
    pub max_characters: usize,
}

impl Default for MemoryHealthLimits {
    fn default() -> Self {
        Self {
            max_signals: DEFAULT_MEMORY_HEALTH_SIGNALS,
            max_sessions: DEFAULT_MEMORY_HEALTH_SESSIONS,
            max_characters: DEFAULT_MEMORY_HEALTH_CHARACTERS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryHealthSeverity {
    Info,
    Review,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryHealthSignalKind {
    ReviewRequiredLearning,
    ContestedLearning,
    StaleLearning,
    SourceChangedLearning,
    UncitedLearning,
    DuplicateLearning,
    SameSubjectDifferentGuidance,
    ConflictingTrustedClaims,
    IncompleteSession,
    UnconsolidatedEvidence,
    UnresolvedWorkingState,
    DivergentSessionMemory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryHealthSignal {
    pub signal_id: String,
    pub kind: MemoryHealthSignalKind,
    pub severity: MemoryHealthSeverity,
    pub title: String,
    pub detail: String,
    pub related_learning_ids: Vec<String>,
    pub related_session_ids: Vec<String>,
    pub related_record_ids: Vec<String>,
    pub updated_at_unix_ms: u64,
    pub recommended_action: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsupportedMemoryHealthSignal {
    pub signal: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryHealthSeverityCounts {
    pub high: usize,
    pub review: usize,
    pub info: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryHealthCoverage {
    pub total_learnings: usize,
    pub total_sessions: usize,
    pub sessions_inspected: usize,
    pub sessions_omitted: usize,
    pub candidate_signals: usize,
    pub signals_returned: usize,
    pub signals_omitted: usize,
    pub text_characters: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryHealthReport {
    pub schema_version: u32,
    pub project_id: String,
    pub project_name: String,
    pub artifact_snapshot_id: String,
    pub graph_snapshot_id: String,
    pub generated_at_unix_ms: u64,
    pub revision_freshness: ProjectRevisionFreshness,
    pub health_fingerprint: String,
    pub projection: &'static str,
    pub persisted: bool,
    pub destructive_actions_taken: bool,
    pub has_actionable_signals: bool,
    pub severity_counts: MemoryHealthSeverityCounts,
    pub signals: Vec<MemoryHealthSignal>,
    pub unsupported_signals: Vec<UnsupportedMemoryHealthSignal>,
    pub coverage: MemoryHealthCoverage,
    pub live_source_checked: bool,
    pub source_boundary: &'static str,
    pub instruction_warning: &'static str,
    pub privacy_notice: &'static str,
}

#[derive(Debug, Clone)]
struct SignalDraft {
    kind: MemoryHealthSignalKind,
    severity: MemoryHealthSeverity,
    title: String,
    detail: String,
    learning_ids: Vec<String>,
    session_ids: Vec<String>,
    record_ids: Vec<String>,
    updated_at_unix_ms: u64,
    recommended_action: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintInput<'a> {
    schema_version: u32,
    project_id: &'a str,
    artifact_snapshot_id: &'a str,
    graph_snapshot_id: &'a str,
    revision_freshness: &'a ProjectRevisionFreshness,
    signals: &'a [MemoryHealthSignal],
    unsupported_signals: &'a [UnsupportedMemoryHealthSignal],
    coverage: &'a MemoryHealthCoverage,
}

pub fn memory_health_report(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    limits: MemoryHealthLimits,
) -> Result<MemoryHealthReport, LeyCoreError> {
    validate_limits(limits)?;
    let project_start = project_start.as_ref();
    let vault = vault.as_ref();
    let overview = project_memory_overview(project_start, vault)?;
    let learnings = list_learnings(project_start, vault)?;
    let mut drafts = learning_health_signals(&learnings);

    let mut summaries = list_sessions(project_start, vault)?;
    summaries.sort_by(|left, right| {
        session_priority(left.status)
            .cmp(&session_priority(right.status))
            .then_with(|| right.updated_at_unix_ms.cmp(&left.updated_at_unix_ms))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    let total_sessions = summaries.len();
    let selected_sessions = summaries
        .into_iter()
        .take(limits.max_sessions)
        .collect::<Vec<_>>();
    let sessions_inspected = selected_sessions.len();
    let mut revision_resolver = RevisionResolver::new(project_start, overview.git.as_ref())?;
    for summary in selected_sessions {
        let session = read_session(project_start, vault, &summary.session_id)?;
        let latest = session.checkpoints.last();
        if matches!(
            session.status,
            SessionStatus::Active | SessionStatus::Paused
        ) {
            drafts.push(SignalDraft {
                kind: MemoryHealthSignalKind::IncompleteSession,
                severity: if session.status == SessionStatus::Paused {
                    MemoryHealthSeverity::Review
                } else {
                    MemoryHealthSeverity::Info
                },
                title: format!("Session '{}' is {}", session.name, session_status_name(session.status)),
                detail: "The session has not reached a completed/abandoned terminal state. This is an explicit lifecycle signal, not proof that the work is unhealthy.".to_owned(),
                learning_ids: Vec::new(),
                session_ids: vec![session.session_id.clone()],
                record_ids: latest.map(|checkpoint| vec![checkpoint.id.clone()]).unwrap_or_default(),
                updated_at_unix_ms: session.updated_at_unix_ms,
                recommended_action: "Review whether the session should remain active/paused, be resumed, or be explicitly finished.",
            });
        }

        if let Some(checkpoint) = latest {
            let active_tasks = checkpoint
                .tasks
                .iter()
                .filter(|task| {
                    matches!(
                        task.status,
                        TaskStatus::Pending | TaskStatus::InProgress | TaskStatus::Blocked
                    )
                })
                .count();
            let open_problems = checkpoint
                .problems
                .iter()
                .filter(|problem| problem.resolution.is_none())
                .count();
            let unresolved = checkpoint.unresolved.len();
            if matches!(
                session.status,
                SessionStatus::Active | SessionStatus::Paused
            ) && active_tasks
                .saturating_add(open_problems)
                .saturating_add(unresolved)
                > 0
            {
                drafts.push(SignalDraft {
                    kind: MemoryHealthSignalKind::UnresolvedWorkingState,
                    severity: MemoryHealthSeverity::Review,
                    title: format!("Open working state in session '{}'", session.name),
                    detail: format!(
                        "Latest checkpoint contains {active_tasks} active task(s), {open_problems} unresolved problem(s), and {unresolved} explicit unresolved item(s)."
                    ),
                    learning_ids: Vec::new(),
                    session_ids: vec![session.session_id.clone()],
                    record_ids: vec![checkpoint.id.clone()],
                    updated_at_unix_ms: checkpoint.recorded_at_unix_ms,
                    recommended_action: "Review the latest checkpoint and resolve, carry forward, or deliberately defer the remaining work.",
                });
            }
            if let Some(revision) = checkpoint.project_revision.as_ref() {
                let applicability = revision_resolver.applicability(revision);
                if applicability.compatibility == RevisionCompatibility::Divergent {
                    drafts.push(SignalDraft {
                        kind: MemoryHealthSignalKind::DivergentSessionMemory,
                        severity: MemoryHealthSeverity::Review,
                        title: format!("Divergent revision memory in session '{}'", session.name),
                        detail: "The latest checkpoint belongs to a Git revision classified as divergent from the current line. This makes the checkpoint historical/branch-specific evidence, not current implementation truth.".to_owned(),
                        learning_ids: Vec::new(),
                        session_ids: vec![session.session_id.clone()],
                        record_ids: vec![checkpoint.id.clone()],
                        updated_at_unix_ms: checkpoint.recorded_at_unix_ms,
                        recommended_action: "Review branch applicability before reusing this session state in current work.",
                    });
                }
            }
        }

        let compilation = compile_session_memory(
            project_start,
            vault,
            &session.session_id,
            1.min(MAX_MEMORY_COMPILE_RESULTS),
            MIN_MEMORY_COMPILE_CHARACTERS,
        )?;
        if compilation.total_unconsolidated_evidence > 0 {
            drafts.push(SignalDraft {
                kind: MemoryHealthSignalKind::UnconsolidatedEvidence,
                severity: if matches!(session.status, SessionStatus::Completed | SessionStatus::Abandoned) {
                    MemoryHealthSeverity::High
                } else {
                    MemoryHealthSeverity::Review
                },
                title: format!("Unconsolidated turn evidence in session '{}'", session.name),
                detail: format!(
                    "Memory Compiler reports {} unconsolidated record(s), {} unpaired/uncorrelated record(s), state '{}', and {} omitted record(s) under the health probe bounds.",
                    compilation.total_unconsolidated_evidence,
                    compilation.unpaired_or_uncorrelated_count,
                    memory_compilation_state_name(compilation.state),
                    compilation.omitted_evidence,
                ),
                learning_ids: Vec::new(),
                session_ids: vec![session.session_id.clone()],
                record_ids: compilation
                    .latest_checkpoint
                    .as_ref()
                    .map(|boundary| vec![boundary.checkpoint_id.clone()])
                    .unwrap_or_default(),
                updated_at_unix_ms: session.updated_at_unix_ms,
                recommended_action: "Inspect the recovery evidence and perform the explicit memory transition/checkpoint workflow if the evidence should become structured memory.",
            });
        }
    }

    drafts.sort_by(|left, right| {
        severity_priority(right.severity)
            .cmp(&severity_priority(left.severity))
            .then_with(|| right.updated_at_unix_ms.cmp(&left.updated_at_unix_ms))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.title.cmp(&right.title))
    });
    let candidate_signals = drafts.len();
    let mut budget = TextBudget::new(limits.max_characters);
    let mut signals = Vec::new();
    for draft in drafts.into_iter().take(limits.max_signals) {
        if budget.remaining() == 0 {
            budget.truncated = true;
            break;
        }
        let title = budget.take(&draft.title, 256);
        let detail = budget.take(&draft.detail, 768);
        signals.push(MemoryHealthSignal {
            signal_id: signal_id(
                draft.kind,
                &draft.learning_ids,
                &draft.session_ids,
                &draft.record_ids,
            ),
            kind: draft.kind,
            severity: draft.severity,
            title,
            detail,
            related_learning_ids: draft.learning_ids,
            related_session_ids: draft.session_ids,
            related_record_ids: draft.record_ids,
            updated_at_unix_ms: draft.updated_at_unix_ms,
            recommended_action: draft.recommended_action,
        });
    }
    let severity_counts = severity_counts(&signals);
    let unsupported_signals = unsupported_signals();
    let coverage = MemoryHealthCoverage {
        total_learnings: learnings.len(),
        total_sessions,
        sessions_inspected,
        sessions_omitted: total_sessions.saturating_sub(sessions_inspected),
        candidate_signals,
        signals_returned: signals.len(),
        signals_omitted: candidate_signals.saturating_sub(signals.len()),
        text_characters: budget.used,
        truncated: budget.truncated
            || candidate_signals > signals.len()
            || total_sessions > sessions_inspected,
    };
    let generated_at_unix_ms = crate::unix_time_ms();
    let mut report = MemoryHealthReport {
        schema_version: MEMORY_HEALTH_SCHEMA_VERSION,
        project_id: overview.project_id,
        project_name: overview.project_name,
        artifact_snapshot_id: overview.artifact_snapshot_id,
        graph_snapshot_id: overview.graph_snapshot_id,
        generated_at_unix_ms,
        revision_freshness: overview.revision_freshness,
        health_fingerprint: String::new(),
        projection: "on-demand-memory-health",
        persisted: false,
        destructive_actions_taken: false,
        has_actionable_signals: severity_counts.high > 0 || severity_counts.review > 0,
        severity_counts,
        signals,
        unsupported_signals,
        coverage,
        live_source_checked: false,
        source_boundary: SOURCE_BOUNDARY,
        instruction_warning: INSTRUCTION_WARNING,
        privacy_notice: PRIVACY_NOTICE,
    };
    report.health_fingerprint = health_fingerprint(&report);
    Ok(report)
}

fn learning_health_signals(learnings: &[LearningSummary]) -> Vec<SignalDraft> {
    let mut drafts = Vec::new();
    for learning in learnings {
        let (kind, severity, action) = match learning.trust_state {
            LearningTrustState::ReviewRequired => (
                Some(MemoryHealthSignalKind::ReviewRequiredLearning),
                MemoryHealthSeverity::Review,
                "Review the learning explicitly before allowing it to become trusted reusable knowledge.",
            ),
            LearningTrustState::Contested => (
                Some(MemoryHealthSignalKind::ContestedLearning),
                MemoryHealthSeverity::High,
                "Resolve the disagreement or keep the learning suppressed from current trusted reuse.",
            ),
            LearningTrustState::Stale => (
                Some(MemoryHealthSignalKind::StaleLearning),
                MemoryHealthSeverity::Review,
                "Revalidate against current source/evidence or leave the learning stale and suppressed.",
            ),
            LearningTrustState::Trusted if learning.freshness == LearningFreshness::SourceChanged => (
                Some(MemoryHealthSignalKind::SourceChangedLearning),
                MemoryHealthSeverity::High,
                "Revalidate the trusted learning against the changed source before current reuse.",
            ),
            _ => (None, MemoryHealthSeverity::Info, ""),
        };
        if let Some(kind) = kind {
            drafts.push(learning_signal(
                learning,
                kind,
                severity,
                format!(
                    "Learning '{}' is {} with freshness {}.",
                    learning.title,
                    trust_state_name(learning.trust_state),
                    freshness_name(learning.freshness)
                ),
                action,
            ));
        }
        if learning.freshness == LearningFreshness::Uncited {
            drafts.push(learning_signal(
                learning,
                MemoryHealthSignalKind::UncitedLearning,
                if learning.trust_state == LearningTrustState::Trusted {
                    MemoryHealthSeverity::High
                } else {
                    MemoryHealthSeverity::Review
                },
                format!(
                    "Learning '{}' has no current source citation/freshness anchor.",
                    learning.title
                ),
                "Add verifiable evidence/citations or keep the claim out of trusted current reuse.",
            ));
        }
    }

    let mut by_subject: BTreeMap<String, Vec<&LearningSummary>> = BTreeMap::new();
    for learning in learnings.iter().filter(|learning| {
        !matches!(
            learning.trust_state,
            LearningTrustState::Superseded | LearningTrustState::Rejected
        )
    }) {
        let subject = normalize_content(&learning.title);
        if !subject.is_empty() {
            by_subject.entry(subject).or_default().push(learning);
        }
    }
    for group in by_subject.values().filter(|group| group.len() > 1) {
        let mut by_guidance: BTreeMap<String, Vec<&LearningSummary>> = BTreeMap::new();
        for learning in group {
            by_guidance
                .entry(normalize_content(&learning.guidance_excerpt))
                .or_default()
                .push(*learning);
        }
        for duplicates in by_guidance.values().filter(|items| items.len() > 1) {
            let ids = sorted_learning_ids(duplicates);
            drafts.push(SignalDraft {
                kind: MemoryHealthSignalKind::DuplicateLearning,
                severity: MemoryHealthSeverity::Review,
                title: format!("Duplicate learning subject: {}", duplicates[0].title),
                detail: format!(
                    "{} active learning records have the same normalized subject and materially equivalent bounded guidance.",
                    duplicates.len()
                ),
                learning_ids: ids,
                session_ids: Vec::new(),
                record_ids: Vec::new(),
                updated_at_unix_ms: duplicates
                    .iter()
                    .map(|learning| learning.updated_at_unix_ms)
                    .max()
                    .unwrap_or(0),
                recommended_action: "Review whether these records should remain separate history or be explicitly consolidated/superseded; do not delete them automatically.",
            });
        }
        if by_guidance.len() > 1 {
            let trusted_current = group
                .iter()
                .filter(|learning| {
                    learning.trust_state == LearningTrustState::Trusted
                        && learning.freshness == LearningFreshness::Current
                })
                .copied()
                .collect::<Vec<_>>();
            let distinct_trusted = trusted_current
                .iter()
                .map(|learning| normalize_content(&learning.guidance_excerpt))
                .collect::<BTreeSet<_>>()
                .len();
            let (kind, severity, ids, action) = if distinct_trusted > 1 {
                (
                    MemoryHealthSignalKind::ConflictingTrustedClaims,
                    MemoryHealthSeverity::High,
                    sorted_learning_ids(&trusted_current),
                    "Resolve the conflicting trusted claims explicitly; until then do not infer a winner from recency.",
                )
            } else {
                (
                    MemoryHealthSignalKind::SameSubjectDifferentGuidance,
                    MemoryHealthSeverity::Review,
                    sorted_learning_ids(group),
                    "Review the same-subject records and explicitly preserve history, contest, stale, or supersede as appropriate; recency alone is not a winner.",
                )
            };
            drafts.push(SignalDraft {
                kind,
                severity,
                title: format!("Different guidance for learning subject: {}", group[0].title),
                detail: format!(
                    "{} active learning records share the same normalized subject but contain different bounded guidance; no winner is implied.",
                    group.len()
                ),
                learning_ids: ids,
                session_ids: Vec::new(),
                record_ids: Vec::new(),
                updated_at_unix_ms: group
                    .iter()
                    .map(|learning| learning.updated_at_unix_ms)
                    .max()
                    .unwrap_or(0),
                recommended_action: action,
            });
        }
    }
    drafts
}

fn learning_signal(
    learning: &LearningSummary,
    kind: MemoryHealthSignalKind,
    severity: MemoryHealthSeverity,
    detail: String,
    recommended_action: &'static str,
) -> SignalDraft {
    SignalDraft {
        kind,
        severity,
        title: format!("Learning needs attention: {}", learning.title),
        detail,
        learning_ids: vec![learning.learning_id.clone()],
        session_ids: Vec::new(),
        record_ids: Vec::new(),
        updated_at_unix_ms: learning.updated_at_unix_ms,
        recommended_action,
    }
}

fn unsupported_signals() -> Vec<UnsupportedMemoryHealthSignal> {
    vec![
        UnsupportedMemoryHealthSignal {
            signal: "old-procedure-never-successfully-reverified",
            reason: "Ley does not yet store a typed procedure-learning to verification-outcome linkage, so age/timestamps alone are insufficient to claim successful or failed reverification.",
        },
        UnsupportedMemoryHealthSignal {
            signal: "failed-consolidations",
            reason: "Ley does not yet persist consolidation-attempt outcome history as a typed ledger signal; current recovery state shows unconsolidated evidence, not failed attempts.",
        },
        UnsupportedMemoryHealthSignal {
            signal: "chronically-retrieved-but-unhelpful-memory",
            reason: "Ley does not yet record downstream retrieval utility/helpfulness feedback with enough provenance to classify memory as chronically unhelpful.",
        },
    ]
}

fn validate_limits(limits: MemoryHealthLimits) -> Result<(), LeyCoreError> {
    if !(1..=MAX_MEMORY_HEALTH_SIGNALS).contains(&limits.max_signals) {
        return Err(LeyCoreError::InvalidRetrievalRequest(format!(
            "memory health maxSignals must be between 1 and {MAX_MEMORY_HEALTH_SIGNALS}"
        )));
    }
    if !(1..=MAX_MEMORY_HEALTH_SESSIONS).contains(&limits.max_sessions) {
        return Err(LeyCoreError::InvalidRetrievalRequest(format!(
            "memory health maxSessions must be between 1 and {MAX_MEMORY_HEALTH_SESSIONS}"
        )));
    }
    if !(MIN_MEMORY_HEALTH_CHARACTERS..=MAX_MEMORY_HEALTH_CHARACTERS)
        .contains(&limits.max_characters)
    {
        return Err(LeyCoreError::InvalidRetrievalRequest(format!(
            "memory health maxCharacters must be between {MIN_MEMORY_HEALTH_CHARACTERS} and {MAX_MEMORY_HEALTH_CHARACTERS}"
        )));
    }
    Ok(())
}

fn sorted_learning_ids(learnings: &[&LearningSummary]) -> Vec<String> {
    let mut ids = learnings
        .iter()
        .map(|learning| learning.learning_id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn normalize_content(value: &str) -> String {
    let mut output = String::new();
    let mut pending_space = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if pending_space && !output.is_empty() {
                output.push(' ');
            }
            output.push(character);
            pending_space = false;
        } else {
            pending_space = true;
        }
    }
    output
}

fn severity_counts(signals: &[MemoryHealthSignal]) -> MemoryHealthSeverityCounts {
    let mut counts = MemoryHealthSeverityCounts {
        high: 0,
        review: 0,
        info: 0,
    };
    for signal in signals {
        match signal.severity {
            MemoryHealthSeverity::High => counts.high += 1,
            MemoryHealthSeverity::Review => counts.review += 1,
            MemoryHealthSeverity::Info => counts.info += 1,
        }
    }
    counts
}

fn signal_id(
    kind: MemoryHealthSignalKind,
    learning_ids: &[String],
    session_ids: &[String],
    record_ids: &[String],
) -> String {
    let payload = serde_json::to_vec(&(kind, learning_ids, session_ids, record_ids))
        .expect("memory health signal identity is serializable");
    format!("mhs_{:x}", Sha256::digest(payload))
}

fn health_fingerprint(report: &MemoryHealthReport) -> String {
    let input = FingerprintInput {
        schema_version: report.schema_version,
        project_id: &report.project_id,
        artifact_snapshot_id: &report.artifact_snapshot_id,
        graph_snapshot_id: &report.graph_snapshot_id,
        revision_freshness: &report.revision_freshness,
        signals: &report.signals,
        unsupported_signals: &report.unsupported_signals,
        coverage: &report.coverage,
    };
    let bytes =
        serde_json::to_vec(&input).expect("memory health fingerprint input is serializable");
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn session_priority(status: SessionStatus) -> u8 {
    match status {
        SessionStatus::Active => 0,
        SessionStatus::Paused => 1,
        SessionStatus::Completed => 2,
        SessionStatus::Abandoned => 3,
    }
}

fn severity_priority(severity: MemoryHealthSeverity) -> u8 {
    match severity {
        MemoryHealthSeverity::Info => 0,
        MemoryHealthSeverity::Review => 1,
        MemoryHealthSeverity::High => 2,
    }
}

fn trust_state_name(state: LearningTrustState) -> &'static str {
    match state {
        LearningTrustState::ReviewRequired => "review-required",
        LearningTrustState::Trusted => "trusted",
        LearningTrustState::Contested => "contested",
        LearningTrustState::Superseded => "superseded",
        LearningTrustState::Rejected => "rejected",
        LearningTrustState::Stale => "stale",
    }
}

fn freshness_name(freshness: LearningFreshness) -> &'static str {
    match freshness {
        LearningFreshness::Current => "current",
        LearningFreshness::SourceChanged => "source-changed",
        LearningFreshness::Uncited => "uncited",
    }
}

fn session_status_name(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Active => "active",
        SessionStatus::Completed => "completed",
        SessionStatus::Paused => "paused",
        SessionStatus::Abandoned => "abandoned",
    }
}

fn memory_compilation_state_name(state: MemoryCompilationState) -> &'static str {
    match state {
        MemoryCompilationState::NoUnconsolidatedEvidence => "no-unconsolidated-evidence",
        MemoryCompilationState::ReviewableEvidence => "reviewable-evidence",
        MemoryCompilationState::PartialEvidence => "partial-evidence",
        MemoryCompilationState::MetadataOnly => "metadata-only",
    }
}

struct TextBudget {
    remaining: usize,
    used: usize,
    truncated: bool,
}

impl TextBudget {
    fn new(maximum: usize) -> Self {
        Self {
            remaining: maximum,
            used: 0,
            truncated: false,
        }
    }

    fn remaining(&self) -> usize {
        self.remaining
    }

    fn take(&mut self, value: &str, per_field_maximum: usize) -> String {
        let allowed = self.remaining.min(per_field_maximum);
        let mut characters = value.chars();
        let mut output = characters.by_ref().take(allowed).collect::<String>();
        let omitted = characters.next().is_some();
        if omitted && allowed > 0 {
            output.pop();
            output.push('…');
        }
        let used = output.chars().count();
        self.remaining = self.remaining.saturating_sub(used);
        self.used = self.used.saturating_add(used);
        self.truncated |= omitted;
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        checkpoint_session, ingest_project, initialize_project, propose_learning, read_session,
        record_session_prompt, record_session_response, review_learning, start_session,
        AttemptInput, CaptureMode, CheckpointInput, DecisionInput, LearningActor,
        LearningEvidenceInput, LearningFeedbackAction, LearningKind, LearningProvenance,
        PlanItemInput, ProblemInput, ProposeLearningInput, ReviewLearningInput, SessionSource,
        StartSessionInput, TaskInput, TurnEvidenceInput, TurnEvidenceOrigin, VerificationInput,
    };
    use std::fs;
    use tempfile::tempdir;

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
    }

    fn base_fixture() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        std::path::PathBuf,
        String,
        String,
    ) {
        let temporary = tempdir().unwrap();
        let project = temporary.path().join("project");
        let vault = temporary.path().join("vault");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(project.join("README.md"), "# Storage\n\nUse SQLite WAL.\n").unwrap();
        initialize_project(&project, Some("Health fixture"), CaptureMode::Structured).unwrap();
        ingest_project(&project, &vault).unwrap();
        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('1'),
                name: "Storage migration".to_owned(),
                goal: "Keep migration memory healthy".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            CheckpointInput {
                request_id: request_id('2'),
                summary: "Migration remains in progress.".to_owned(),
                plan: Vec::<PlanItemInput>::new(),
                decisions: vec![DecisionInput {
                    title: "Storage backend".to_owned(),
                    decision: "Use SQLite WAL.".to_owned(),
                    rationale: "Durable local state.".to_owned(),
                    alternatives: Vec::new(),
                }],
                tasks: vec![TaskInput {
                    title: "Finish migration".to_owned(),
                    status: TaskStatus::InProgress,
                    details: "Migrate the remaining records.".to_owned(),
                }],
                problems: vec![ProblemInput {
                    title: "Legacy rows remain".to_owned(),
                    symptom: "Some rows are not converted.".to_owned(),
                    expected: "All rows migrate.".to_owned(),
                    attempts: Vec::<AttemptInput>::new(),
                    resolution: None,
                }],
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: Vec::<VerificationInput>::new(),
                unresolved: vec!["Verify rollback behavior.".to_owned()],
            },
        )
        .unwrap();
        (
            temporary,
            project,
            vault,
            started.session.session_id,
            checkpoint.session.checkpoints.last().unwrap().id.clone(),
        )
    }

    #[test]
    fn reports_learning_hygiene_and_source_change_without_rewriting_memory() {
        let (_temporary, project, vault, session_id, checkpoint_id) = base_fixture();

        for (index, digit) in [('3', '4'), ('5', '6')].into_iter().enumerate() {
            let proposed = propose_learning(
                &project,
                &vault,
                ProposeLearningInput {
                    request_id: request_id(digit.0),
                    actor: LearningActor::Agent,
                    kind: LearningKind::Constraint,
                    title: "Database migration rule".to_owned(),
                    guidance: if index == 0 {
                        "Migrate inside one transaction.".to_owned()
                    } else {
                        "Migrate in resumable batches.".to_owned()
                    },
                    confidence_percent: 90,
                    provenance: LearningProvenance::AgentAuthored,
                    evidence: vec![LearningEvidenceInput {
                        session_id: session_id.clone(),
                        record_id: checkpoint_id.clone(),
                        note: "Migration checkpoint evidence.".to_owned(),
                    }],
                },
            )
            .unwrap();
            review_learning(
                &project,
                &vault,
                &proposed.learning.learning_id,
                ReviewLearningInput {
                    request_id: request_id(digit.1),
                    expected_event_count: None,
                    actor: LearningActor::User,
                    action: LearningFeedbackAction::Confirm,
                    note: "Confirmed for hygiene conflict fixture.".to_owned(),
                    replacement_learning_id: None,
                },
            )
            .unwrap();
        }

        let uncited_checkpoint = checkpoint_session(
            &project,
            &vault,
            &session_id,
            CheckpointInput {
                request_id: request_id('b'),
                summary: "Batch-size note without artifact citation.".to_owned(),
                plan: Vec::<PlanItemInput>::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: Vec::new(),
                commands: Vec::new(),
                verification: Vec::<VerificationInput>::new(),
                unresolved: Vec::new(),
            },
        )
        .unwrap()
        .session
        .checkpoints
        .last()
        .unwrap()
        .id
        .clone();
        let mut duplicate_ids = Vec::new();
        for digit in ['7', '8'] {
            let proposed = propose_learning(
                &project,
                &vault,
                ProposeLearningInput {
                    request_id: request_id(digit),
                    actor: LearningActor::Agent,
                    kind: LearningKind::Fact,
                    title: "Migration batch size".to_owned(),
                    guidance: "Use batches of 100 records.".to_owned(),
                    confidence_percent: 50,
                    provenance: LearningProvenance::AgentAuthored,
                    evidence: vec![LearningEvidenceInput {
                        session_id: session_id.clone(),
                        record_id: uncited_checkpoint.clone(),
                        note: "Valid structured evidence with no artifact citation.".to_owned(),
                    }],
                },
            )
            .unwrap();
            duplicate_ids.push(proposed.learning.learning_id);
        }

        let before_records = list_learnings(&project, &vault).unwrap();
        let first = memory_health_report(&project, &vault, MemoryHealthLimits::default()).unwrap();
        let second = memory_health_report(&project, &vault, MemoryHealthLimits::default()).unwrap();
        let after_records = list_learnings(&project, &vault).unwrap();

        assert_eq!(before_records, after_records);
        assert_eq!(first.health_fingerprint, second.health_fingerprint);
        assert!(!first.persisted);
        assert!(!first.destructive_actions_taken);
        assert!(first.has_actionable_signals);
        assert!(first.signals.iter().any(|signal| {
            signal.kind == MemoryHealthSignalKind::ConflictingTrustedClaims
                && signal.severity == MemoryHealthSeverity::High
                && signal.related_learning_ids.len() == 2
        }));
        assert!(first.signals.iter().any(|signal| {
            signal.kind == MemoryHealthSignalKind::DuplicateLearning
                && duplicate_ids
                    .iter()
                    .all(|id| signal.related_learning_ids.contains(id))
        }));
        assert!(
            first
                .signals
                .iter()
                .filter(|signal| { signal.kind == MemoryHealthSignalKind::ReviewRequiredLearning })
                .count()
                >= 2
        );
        assert!(
            first
                .signals
                .iter()
                .filter(|signal| { signal.kind == MemoryHealthSignalKind::UncitedLearning })
                .count()
                >= 2
        );
        assert_eq!(first.unsupported_signals.len(), 3);

        fs::write(
            project.join("README.md"),
            "# Storage\n\nThe migration source changed after review.\n",
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let changed =
            memory_health_report(&project, &vault, MemoryHealthLimits::default()).unwrap();
        assert!(
            changed
                .signals
                .iter()
                .filter(|signal| {
                    signal.kind == MemoryHealthSignalKind::SourceChangedLearning
                        && signal.severity == MemoryHealthSeverity::High
                })
                .count()
                >= 2
        );
        assert_ne!(first.health_fingerprint, changed.health_fingerprint);
    }

    #[test]
    fn reports_incomplete_unresolved_and_unconsolidated_session_without_mutation() {
        let (_temporary, project, vault, session_id, checkpoint_id) = base_fixture();
        record_session_prompt(
            &project,
            &vault,
            &session_id,
            TurnEvidenceInput {
                request_id: request_id('9'),
                origin: TurnEvidenceOrigin::HostHook,
                host: Some("codex".to_owned()),
                correlation_material: Some("health-turn".to_owned()),
                text: "Continue migration after the checkpoint.".to_owned(),
            },
        )
        .unwrap();
        record_session_response(
            &project,
            &vault,
            &session_id,
            TurnEvidenceInput {
                request_id: request_id('a'),
                origin: TurnEvidenceOrigin::HostHook,
                host: Some("codex".to_owned()),
                correlation_material: Some("health-turn".to_owned()),
                text: "I changed migration behavior after the checkpoint.".to_owned(),
            },
        )
        .unwrap();

        let before = read_session(&project, &vault, &session_id).unwrap();
        let report = memory_health_report(&project, &vault, MemoryHealthLimits::default()).unwrap();
        let after = read_session(&project, &vault, &session_id).unwrap();
        assert_eq!(before.event_count, after.event_count);
        assert_eq!(before.prompts, after.prompts);
        assert_eq!(before.responses, after.responses);
        assert!(report.signals.iter().any(|signal| {
            signal.kind == MemoryHealthSignalKind::IncompleteSession
                && signal.related_session_ids == vec![session_id.clone()]
        }));
        assert!(report.signals.iter().any(|signal| {
            signal.kind == MemoryHealthSignalKind::UnresolvedWorkingState
                && signal.related_record_ids == vec![checkpoint_id.clone()]
        }));
        assert!(report.signals.iter().any(|signal| {
            signal.kind == MemoryHealthSignalKind::UnconsolidatedEvidence
                && signal.related_session_ids == vec![session_id.clone()]
                && signal.detail.contains("2 unconsolidated")
        }));
        assert!(report.coverage.text_characters <= DEFAULT_MEMORY_HEALTH_CHARACTERS);
        assert!(!report.live_source_checked);
    }
}
