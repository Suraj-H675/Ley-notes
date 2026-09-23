use crate::{
    compile_session_memory, diagnose_project, list_sessions, LeyCoreError, MemoryCompilationState,
    SessionSourceKind, SessionStatus, TurnEvidenceRetention, MAX_MEMORY_COMPILE_RESULTS,
    MIN_MEMORY_COMPILE_CHARACTERS,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

pub const CONSOLIDATION_INBOX_SCHEMA_VERSION: u32 = 2;
pub const DEFAULT_CONSOLIDATION_INBOX_ITEMS: usize = 20;
pub const MAX_CONSOLIDATION_INBOX_ITEMS: usize = 50;
pub const DEFAULT_CONSOLIDATION_INBOX_SESSIONS: usize = 30;
pub const MAX_CONSOLIDATION_INBOX_SESSIONS: usize = 50;
pub const MAX_CONSOLIDATION_PROPOSAL_EVIDENCE_IDS: usize = 20;

const SOURCE_BOUNDARY: &str = "derived-local-consolidation-advisory";
const INSTRUCTION_WARNING: &str = "Consolidation Inbox is a read-only local planning view over retained historical evidence. It does not interpret turn text, prove semantic faithfulness, create a checkpoint or learning, increase authority, or authorize background work. Review retained evidence and live source before proposing reusable memory.";
const PRIVACY_NOTICE: &str = "Ley returns stable session/turn IDs and bounded counts only; turn bodies, absolute project/vault paths, and imported-host sessions are omitted from this inbox. The projection is rebuilt on demand and is not persisted.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConsolidationAction {
    ProposeReviewRequiredLearning,
    ReviewPartialEvidence,
    InspectMetadataOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsolidationInboxLimits {
    pub max_items: usize,
    pub max_sessions: usize,
}

impl Default for ConsolidationInboxLimits {
    fn default() -> Self {
        Self {
            max_items: DEFAULT_CONSOLIDATION_INBOX_ITEMS,
            max_sessions: DEFAULT_CONSOLIDATION_INBOX_SESSIONS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsolidationInboxItem {
    pub session_id: String,
    pub session_name: String,
    pub session_status: SessionStatus,
    pub session_source_kind: SessionSourceKind,
    pub session_updated_at_unix_ms: u64,
    pub session_event_count: u64,
    pub checkpoint_count: usize,
    pub compilation_state: MemoryCompilationState,
    pub total_unconsolidated_evidence: usize,
    pub captured_body_count: usize,
    pub omitted_minimal_count: usize,
    pub omitted_capacity_count: usize,
    pub unpaired_or_uncorrelated_count: usize,
    pub proposal_evidence_record_ids: Vec<String>,
    pub omitted_proposal_evidence_record_ids: usize,
    pub action: ConsolidationAction,
    pub semantic_faithfulness_proven: bool,
    pub automatic_write_allowed: bool,
    pub source_boundary: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsolidationInboxCoverage {
    pub total_sessions: usize,
    pub eligible_boundary_sessions: usize,
    pub excluded_active_sessions: usize,
    pub excluded_imported_sessions: usize,
    pub sessions_inspected: usize,
    pub sessions_omitted: usize,
    pub all_eligible_sessions_inspected: bool,
    pub inspected_sessions_with_unconsolidated_evidence: usize,
    pub items_returned: usize,
    pub items_omitted: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsolidationInbox {
    pub schema_version: u32,
    pub project_id: String,
    pub project_name: String,
    pub generated_at_unix_ms: u64,
    pub inbox_fingerprint: String,
    pub items: Vec<ConsolidationInboxItem>,
    pub coverage: ConsolidationInboxCoverage,
    pub persisted: bool,
    pub model_invoked: bool,
    pub background_work_started: bool,
    pub destructive_actions_taken: bool,
    pub live_source_checked: bool,
    pub source_boundary: &'static str,
    pub instruction_warning: &'static str,
    pub privacy_notice: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintInput<'a> {
    schema_version: u32,
    project_id: &'a str,
    items: &'a [ConsolidationInboxItem],
    coverage: &'a ConsolidationInboxCoverage,
}

pub fn consolidation_inbox(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    limits: ConsolidationInboxLimits,
) -> Result<ConsolidationInbox, LeyCoreError> {
    validate_limits(limits)?;
    let project_start = project_start.as_ref();
    let vault = vault.as_ref();
    let diagnostic = diagnose_project(project_start)?;
    let mut sessions = list_sessions(project_start, vault)?;
    let total_sessions = sessions.len();
    let excluded_active_sessions = sessions
        .iter()
        .filter(|session| session.status == SessionStatus::Active)
        .count();
    let excluded_imported_sessions = sessions
        .iter()
        .filter(|session| session.source_kind == SessionSourceKind::Import)
        .count();
    sessions.retain(|session| {
        session.source_kind != SessionSourceKind::Import
            && matches!(
                session.status,
                SessionStatus::Paused | SessionStatus::Completed | SessionStatus::Abandoned
            )
    });
    sessions.sort_by(|left, right| {
        boundary_priority(left.status)
            .cmp(&boundary_priority(right.status))
            .then_with(|| right.updated_at_unix_ms.cmp(&left.updated_at_unix_ms))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    let eligible_boundary_sessions = sessions.len();
    let selected = sessions
        .into_iter()
        .take(limits.max_sessions)
        .collect::<Vec<_>>();
    let sessions_inspected = selected.len();
    let mut candidates = Vec::new();
    for summary in selected {
        let compilation = compile_session_memory(
            project_start,
            vault,
            &summary.session_id,
            MAX_MEMORY_COMPILE_RESULTS,
            MIN_MEMORY_COMPILE_CHARACTERS,
        )?;
        if compilation.total_unconsolidated_evidence == 0 {
            continue;
        }
        let mut proposal_evidence_record_ids = compilation
            .evidence
            .iter()
            .filter(|evidence| evidence.retention == TurnEvidenceRetention::Captured)
            .map(|evidence| evidence.record_id.clone())
            .collect::<Vec<_>>();
        if proposal_evidence_record_ids.len() > MAX_CONSOLIDATION_PROPOSAL_EVIDENCE_IDS {
            let first = proposal_evidence_record_ids
                .len()
                .saturating_sub(MAX_CONSOLIDATION_PROPOSAL_EVIDENCE_IDS);
            proposal_evidence_record_ids = proposal_evidence_record_ids.split_off(first);
        }
        let omitted_proposal_evidence_record_ids = compilation
            .captured_body_count
            .saturating_sub(proposal_evidence_record_ids.len());
        let action = match compilation.state {
            MemoryCompilationState::ReviewableEvidence => {
                ConsolidationAction::ProposeReviewRequiredLearning
            }
            MemoryCompilationState::PartialEvidence => ConsolidationAction::ReviewPartialEvidence,
            MemoryCompilationState::MetadataOnly => ConsolidationAction::InspectMetadataOnly,
            MemoryCompilationState::NoUnconsolidatedEvidence => continue,
        };
        candidates.push(ConsolidationInboxItem {
            session_id: summary.session_id,
            session_name: summary.name,
            session_status: summary.status,
            session_source_kind: summary.source_kind,
            session_updated_at_unix_ms: summary.updated_at_unix_ms,
            session_event_count: summary.event_count,
            checkpoint_count: summary.checkpoints,
            compilation_state: compilation.state,
            total_unconsolidated_evidence: compilation.total_unconsolidated_evidence,
            captured_body_count: compilation.captured_body_count,
            omitted_minimal_count: compilation.omitted_minimal_count,
            omitted_capacity_count: compilation.omitted_capacity_count,
            unpaired_or_uncorrelated_count: compilation.unpaired_or_uncorrelated_count,
            proposal_evidence_record_ids,
            omitted_proposal_evidence_record_ids,
            action,
            semantic_faithfulness_proven: false,
            automatic_write_allowed: false,
            source_boundary: SOURCE_BOUNDARY,
        });
    }
    let inspected_sessions_with_unconsolidated_evidence = candidates.len();
    let items = candidates
        .into_iter()
        .take(limits.max_items)
        .collect::<Vec<_>>();
    let sessions_omitted = eligible_boundary_sessions.saturating_sub(sessions_inspected);
    let coverage = ConsolidationInboxCoverage {
        total_sessions,
        eligible_boundary_sessions,
        excluded_active_sessions,
        excluded_imported_sessions,
        sessions_inspected,
        sessions_omitted,
        all_eligible_sessions_inspected: sessions_omitted == 0,
        inspected_sessions_with_unconsolidated_evidence,
        items_returned: items.len(),
        items_omitted: inspected_sessions_with_unconsolidated_evidence.saturating_sub(items.len()),
        truncated: eligible_boundary_sessions > sessions_inspected
            || inspected_sessions_with_unconsolidated_evidence > items.len()
            || items
                .iter()
                .any(|item| item.omitted_proposal_evidence_record_ids > 0),
    };
    let inbox_fingerprint = inbox_fingerprint(&diagnostic.identity.project_id, &items, &coverage)?;
    Ok(ConsolidationInbox {
        schema_version: CONSOLIDATION_INBOX_SCHEMA_VERSION,
        project_id: diagnostic.identity.project_id,
        project_name: diagnostic.identity.name,
        generated_at_unix_ms: crate::unix_time_ms(),
        inbox_fingerprint,
        items,
        coverage,
        persisted: false,
        model_invoked: false,
        background_work_started: false,
        destructive_actions_taken: false,
        live_source_checked: false,
        source_boundary: SOURCE_BOUNDARY,
        instruction_warning: INSTRUCTION_WARNING,
        privacy_notice: PRIVACY_NOTICE,
    })
}

fn validate_limits(limits: ConsolidationInboxLimits) -> Result<(), LeyCoreError> {
    if !(1..=MAX_CONSOLIDATION_INBOX_ITEMS).contains(&limits.max_items) {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "consolidation inbox maxItems must be between 1 and {MAX_CONSOLIDATION_INBOX_ITEMS}"
        )));
    }
    if !(1..=MAX_CONSOLIDATION_INBOX_SESSIONS).contains(&limits.max_sessions) {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "consolidation inbox maxSessions must be between 1 and {MAX_CONSOLIDATION_INBOX_SESSIONS}"
        )));
    }
    Ok(())
}

fn boundary_priority(status: SessionStatus) -> u8 {
    match status {
        SessionStatus::Paused => 0,
        SessionStatus::Completed => 1,
        SessionStatus::Abandoned => 2,
        SessionStatus::Active => 3,
    }
}

fn inbox_fingerprint(
    project_id: &str,
    items: &[ConsolidationInboxItem],
    coverage: &ConsolidationInboxCoverage,
) -> Result<String, LeyCoreError> {
    let bytes = serde_json::to_vec(&FingerprintInput {
        schema_version: CONSOLIDATION_INBOX_SCHEMA_VERSION,
        project_id,
        items,
        coverage,
    })
    .map_err(|error| LeyCoreError::InvalidSessionRequest(error.to_string()))?;
    Ok(format!("cin_{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        finish_session, ingest_project, initialize_project, propose_learning,
        record_session_prompt, record_session_response, start_session, CaptureMode,
        FinishSessionInput, LearningActor, LearningEvidenceInput, LearningKind, LearningProvenance,
        LearningTrustState, ProposeLearningInput, SessionSource, StartSessionInput,
        TurnEvidenceInput, TurnEvidenceOrigin,
    };
    use tempfile::tempdir;

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
    }

    #[test]
    fn inbox_surfaces_terminal_retained_evidence_without_writing_or_returning_bodies() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(
            &project,
            Some("Consolidation inbox"),
            CaptureMode::Structured,
        )
        .unwrap();
        std::fs::write(project.join("README.md"), "# Consolidation inbox\n").unwrap();
        ingest_project(&project, &vault).unwrap();

        let active = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('1'),
                name: "Still active".to_owned(),
                goal: "Active work is not a meaningful consolidation boundary".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        record_session_prompt(
            &project,
            &vault,
            &active.session.session_id,
            TurnEvidenceInput {
                request_id: request_id('2'),
                origin: TurnEvidenceOrigin::ManualCli,
                host: None,
                correlation_material: Some("active".to_owned()),
                text: "ACTIVE_BODY_CANARY".to_owned(),
            },
        )
        .unwrap();

        let terminal = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('3'),
                name: "Completed boundary".to_owned(),
                goal: "Review retained turn evidence after completion".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let prompt = record_session_prompt(
            &project,
            &vault,
            &terminal.session.session_id,
            TurnEvidenceInput {
                request_id: request_id('4'),
                origin: TurnEvidenceOrigin::ManualCli,
                host: None,
                correlation_material: Some("terminal".to_owned()),
                text: "TERMINAL_PROMPT_BODY_CANARY".to_owned(),
            },
        )
        .unwrap();
        let prompt_id = prompt.session.prompts.last().unwrap().record_id.clone();
        let response = record_session_response(
            &project,
            &vault,
            &terminal.session.session_id,
            TurnEvidenceInput {
                request_id: request_id('5'),
                origin: TurnEvidenceOrigin::ManualCli,
                host: None,
                correlation_material: Some("terminal".to_owned()),
                text: "TERMINAL_RESPONSE_BODY_CANARY".to_owned(),
            },
        )
        .unwrap();
        let response_id = response.session.responses.last().unwrap().record_id.clone();
        let finished = finish_session(
            &project,
            &vault,
            &terminal.session.session_id,
            FinishSessionInput {
                request_id: request_id('6'),
                status: SessionStatus::Completed,
                summary: "Completed without a final structured checkpoint".to_owned(),
                final_response: String::new(),
                handoff: "Review retained evidence before proposing reusable guidance.".to_owned(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let terminal_events = finished.session.event_count;

        let first =
            consolidation_inbox(&project, &vault, ConsolidationInboxLimits::default()).unwrap();
        let second =
            consolidation_inbox(&project, &vault, ConsolidationInboxLimits::default()).unwrap();
        assert_eq!(first.inbox_fingerprint, second.inbox_fingerprint);
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.coverage.total_sessions, 2);
        assert_eq!(first.coverage.excluded_active_sessions, 1);
        let item = &first.items[0];
        assert_eq!(item.session_id, terminal.session.session_id);
        assert_eq!(item.session_status, SessionStatus::Completed);
        assert_eq!(
            item.compilation_state,
            MemoryCompilationState::ReviewableEvidence
        );
        assert_eq!(
            item.action,
            ConsolidationAction::ProposeReviewRequiredLearning
        );
        assert_eq!(item.total_unconsolidated_evidence, 2);
        assert_eq!(item.captured_body_count, 2);
        assert_eq!(
            item.proposal_evidence_record_ids,
            vec![prompt_id.clone(), response_id.clone()]
        );
        assert!(!item.semantic_faithfulness_proven);
        assert!(!item.automatic_write_allowed);
        assert!(!first.persisted);
        assert!(!first.model_invoked);
        assert!(!first.background_work_started);
        assert!(!first.destructive_actions_taken);
        let serialized = serde_json::to_string(&first).unwrap();
        assert!(!serialized.contains("TERMINAL_PROMPT_BODY_CANARY"));
        assert!(!serialized.contains("TERMINAL_RESPONSE_BODY_CANARY"));
        assert!(!serialized.contains("ACTIVE_BODY_CANARY"));
        assert!(!serialized.contains(project.to_str().unwrap()));
        assert!(!serialized.contains(vault.to_str().unwrap()));

        let proposed = propose_learning(
            &project,
            &vault,
            ProposeLearningInput {
                request_id: request_id('7'),
                actor: LearningActor::Agent,
                kind: LearningKind::Procedure,
                title: "Review the completed workflow".to_owned(),
                guidance: "Review the completed workflow before reusing it.".to_owned(),
                confidence_percent: 60,
                provenance: LearningProvenance::Inferred,
                evidence: item
                    .proposal_evidence_record_ids
                    .iter()
                    .map(|record_id| LearningEvidenceInput {
                        session_id: item.session_id.clone(),
                        record_id: record_id.clone(),
                        note: "Selected from the local consolidation inbox.".to_owned(),
                    })
                    .collect(),
            },
        )
        .unwrap();
        assert_eq!(
            proposed.learning.trust_state,
            LearningTrustState::ReviewRequired
        );
        assert_eq!(
            crate::read_session(&project, &vault, &terminal.session.session_id)
                .unwrap()
                .event_count,
            terminal_events
        );
    }

    #[test]
    fn inbox_excludes_imports_and_marks_body_free_terminal_evidence_metadata_only() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Metadata inbox"), CaptureMode::Minimal).unwrap();
        std::fs::write(project.join("README.md"), "# Metadata inbox\n").unwrap();
        ingest_project(&project, &vault).unwrap();
        let terminal = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('8'),
                name: "Minimal terminal".to_owned(),
                goal: "Expose metadata-only consolidation state".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        record_session_prompt(
            &project,
            &vault,
            &terminal.session.session_id,
            TurnEvidenceInput {
                request_id: request_id('9'),
                origin: TurnEvidenceOrigin::ManualCli,
                host: None,
                correlation_material: None,
                text: "MINIMAL_CONSOLIDATION_BODY_CANARY".to_owned(),
            },
        )
        .unwrap();
        finish_session(
            &project,
            &vault,
            &terminal.session.session_id,
            FinishSessionInput {
                request_id: request_id('a'),
                status: SessionStatus::Abandoned,
                summary: "Stopped with body-free evidence".to_owned(),
                final_response: String::new(),
                handoff: String::new(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let imported = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('b'),
                name: "Imported source".to_owned(),
                goal: "Imported history stays outside local consolidation".to_owned(),
                source: SessionSource {
                    kind: SessionSourceKind::Import,
                    host: Some("codex".to_owned()),
                    agent: None,
                    source_reference: Some(format!("hsi_{}", "1".repeat(64))),
                },
            },
        )
        .unwrap();
        finish_session(
            &project,
            &vault,
            &imported.session.session_id,
            FinishSessionInput {
                request_id: request_id('c'),
                status: SessionStatus::Completed,
                summary: "Imported session remains explicit-only".to_owned(),
                final_response: String::new(),
                handoff: String::new(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();

        let inbox =
            consolidation_inbox(&project, &vault, ConsolidationInboxLimits::default()).unwrap();
        assert_eq!(inbox.coverage.excluded_imported_sessions, 1);
        assert_eq!(inbox.items.len(), 1);
        assert_eq!(
            inbox.items[0].action,
            ConsolidationAction::InspectMetadataOnly
        );
        assert!(inbox.items[0].proposal_evidence_record_ids.is_empty());
        assert_eq!(inbox.items[0].omitted_minimal_count, 1);
        assert!(!serde_json::to_string(&inbox)
            .unwrap()
            .contains("MINIMAL_CONSOLIDATION_BODY_CANARY"));
    }

    #[test]
    fn inbox_coverage_names_only_inspected_unconsolidated_sessions_under_bounds() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(
            &project,
            Some("Bounded consolidation coverage"),
            CaptureMode::Structured,
        )
        .unwrap();
        std::fs::write(project.join("README.md"), "# Consolidation coverage\n").unwrap();
        ingest_project(&project, &vault).unwrap();

        for (index, digit) in [('1', '2'), ('3', '4')].into_iter().enumerate() {
            let session = start_session(
                &project,
                &vault,
                StartSessionInput {
                    request_id: request_id(digit.0),
                    name: format!("Terminal boundary {index}"),
                    goal: "Retain evidence for bounded consolidation coverage".to_owned(),
                    source: SessionSource::default(),
                },
            )
            .unwrap();
            record_session_prompt(
                &project,
                &vault,
                &session.session.session_id,
                TurnEvidenceInput {
                    request_id: request_id(digit.1),
                    origin: TurnEvidenceOrigin::ManualCli,
                    host: None,
                    correlation_material: Some(format!("coverage-{index}")),
                    text: format!("CONSOLIDATION_COVERAGE_BODY_{index}"),
                },
            )
            .unwrap();
            finish_session(
                &project,
                &vault,
                &session.session.session_id,
                FinishSessionInput {
                    request_id: format!("req_{:032x}", 700 + index),
                    status: SessionStatus::Completed,
                    summary: "Terminal boundary retained unconsolidated evidence.".to_owned(),
                    final_response: String::new(),
                    handoff: String::new(),
                    unresolved: Vec::new(),
                },
            )
            .unwrap();
        }

        let inbox = consolidation_inbox(
            &project,
            &vault,
            ConsolidationInboxLimits {
                max_items: 20,
                max_sessions: 1,
            },
        )
        .unwrap();
        assert_eq!(inbox.schema_version, 2);
        assert_eq!(inbox.coverage.eligible_boundary_sessions, 2);
        assert_eq!(inbox.coverage.sessions_inspected, 1);
        assert_eq!(inbox.coverage.sessions_omitted, 1);
        assert!(!inbox.coverage.all_eligible_sessions_inspected);
        assert_eq!(
            inbox
                .coverage
                .inspected_sessions_with_unconsolidated_evidence,
            1
        );
        assert_eq!(inbox.coverage.items_returned, 1);
        assert!(inbox.coverage.truncated);
    }
}
