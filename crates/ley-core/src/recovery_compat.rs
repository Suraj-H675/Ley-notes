//! Private compatibility formats for fingerprints stored in historical recovery events.
//!
//! This module deliberately contains no verifier or writer policy. The records and hash inputs
//! below exist only so replay can validate persisted schema-v3, v8-v13, and v16 checkpoints.

use crate::{AttemptOutcome, PlanStatus, TaskStatus, ToolObservationKind};
use sha2::{Digest, Sha256};

pub(crate) const MAX_RECOVERY_CANDIDATES: usize = 50;
pub(crate) const MAX_RECOVERY_EVIDENCE_PER_RECORD: usize = 20;
pub(crate) const OBSERVED_COMMAND_CANDIDATE_SUMMARY: &str =
    "Observed Bash invocation; exit code, command success, test success, and verification outcome are unknown.";

// The discriminant order is part of the schema-v3 candidate fingerprint contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum MemoryCandidateKind {
    #[cfg(test)]
    Summary = 0,
    Plan = 1,
    Decision = 2,
    Task = 3,
    Problem = 4,
    Unresolved = 9,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TypedMemoryCandidateClaim {
    Plan {
        text: String,
        status: PlanStatus,
        evidence_record_ids: Vec<String>,
    },
    Task {
        title: String,
        status: TaskStatus,
        details: String,
        evidence_record_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypedMemoryTransitionInput {
    pub expected_event_count: u64,
    pub candidate: TypedMemoryCandidateClaim,
    pub deferred_evidence_record_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BatchMemoryCandidateClaim {
    Unresolved {
        text: String,
        evidence_record_ids: Vec<String>,
    },
    Decision {
        title: String,
        decision: String,
        evidence_record_ids: Vec<String>,
    },
    Problem {
        title: String,
        symptom: String,
        evidence_record_ids: Vec<String>,
    },
    Task {
        title: String,
        status: TaskStatus,
        details: String,
        evidence_record_ids: Vec<String>,
    },
    Plan {
        text: String,
        status: PlanStatus,
        evidence_record_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BatchMemoryTransitionInput {
    pub expected_event_count: u64,
    pub checkpoint_summary: String,
    pub candidates: Vec<BatchMemoryCandidateClaim>,
    pub deferred_evidence_record_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RichProblemAttemptCandidate {
    pub action: String,
    pub outcome: AttemptOutcome,
    pub evidence: String,
    pub evidence_record_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RichProblemResolutionCandidate {
    pub root_cause: String,
    pub change: String,
    pub verification: String,
    pub evidence_record_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RichProblemMemoryCandidate {
    pub title: String,
    pub symptom: String,
    pub expected: String,
    pub evidence_record_ids: Vec<String>,
    pub attempts: Vec<RichProblemAttemptCandidate>,
    pub resolution: Option<RichProblemResolutionCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RichProblemMemoryTransitionInput {
    pub expected_event_count: u64,
    pub candidate: RichProblemMemoryCandidate,
    pub deferred_evidence_record_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompositeMemoryTransitionInput {
    pub expected_event_count: u64,
    pub checkpoint_summary: String,
    pub rich_problem: RichProblemMemoryCandidate,
    pub siblings: Vec<BatchMemoryCandidateClaim>,
    pub deferred_evidence_record_ids: Vec<String>,
}

pub(crate) fn unresolved_candidate_fingerprint(
    session_id: &str,
    expected_event_count: u64,
    subject: &str,
    statement: &str,
    evidence_record_ids: &[String],
) -> String {
    recovery_candidate_fingerprint(
        session_id,
        expected_event_count,
        MemoryCandidateKind::Unresolved,
        subject,
        statement,
        evidence_record_ids,
    )
}

pub(crate) fn recovery_candidate_fingerprint(
    session_id: &str,
    expected_event_count: u64,
    kind: MemoryCandidateKind,
    subject: &str,
    statement: &str,
    evidence_record_ids: &[String],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-memory-transition-v1");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(expected_event_count.to_le_bytes());
    let mut evidence = evidence_record_ids.to_vec();
    evidence.sort();
    hasher.update([kind as u8]);
    hasher.update([0]);
    hasher.update(normalize(subject).as_bytes());
    hasher.update([0]);
    hasher.update(normalize(statement).as_bytes());
    for record_id in evidence {
        hasher.update([0]);
        hasher.update(record_id.as_bytes());
    }
    hasher.update([0xff]);
    format!("sha256:{:x}", hasher.finalize())
}

pub(crate) fn task_candidate_fingerprint(
    session_id: &str,
    expected_event_count: u64,
    title: &str,
    status: TaskStatus,
    details: &str,
    evidence_record_ids: &[String],
) -> String {
    typed_candidate_fingerprint(
        session_id,
        &TypedMemoryTransitionInput {
            expected_event_count,
            candidate: TypedMemoryCandidateClaim::Task {
                title: title.to_owned(),
                status,
                details: details.to_owned(),
                evidence_record_ids: evidence_record_ids.to_vec(),
            },
            deferred_evidence_record_ids: Vec::new(),
        },
    )
}

pub(crate) fn plan_candidate_fingerprint(
    session_id: &str,
    expected_event_count: u64,
    text: &str,
    status: PlanStatus,
    evidence_record_ids: &[String],
) -> String {
    typed_candidate_fingerprint(
        session_id,
        &TypedMemoryTransitionInput {
            expected_event_count,
            candidate: TypedMemoryCandidateClaim::Plan {
                text: text.to_owned(),
                status,
                evidence_record_ids: evidence_record_ids.to_vec(),
            },
            deferred_evidence_record_ids: Vec::new(),
        },
    )
}

fn typed_candidate_fingerprint(session_id: &str, input: &TypedMemoryTransitionInput) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-memory-transition-v2-typed");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(input.expected_event_count.to_le_bytes());
    match &input.candidate {
        TypedMemoryCandidateClaim::Plan {
            text,
            status,
            evidence_record_ids,
        } => {
            hasher.update([0]);
            hasher.update(b"plan");
            hasher.update([0]);
            hasher.update(normalize(text).as_bytes());
            hasher.update([0]);
            hasher.update(plan_status_label(*status).as_bytes());
            hash_evidence(&mut hasher, evidence_record_ids, 0);
        }
        TypedMemoryCandidateClaim::Task {
            title,
            status,
            details,
            evidence_record_ids,
        } => {
            hasher.update([0]);
            hasher.update(b"task");
            hasher.update([0]);
            hasher.update(normalize(title).as_bytes());
            hasher.update([0]);
            hasher.update(task_status_label(*status).as_bytes());
            hasher.update([0]);
            hasher.update(normalize(details).as_bytes());
            hash_evidence(&mut hasher, evidence_record_ids, 0);
        }
    }
    let mut deferred = input.deferred_evidence_record_ids.clone();
    deferred.sort();
    for record_id in deferred {
        hasher.update([0xfe]);
        hasher.update(record_id.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

pub(crate) fn batch_candidate_fingerprint(
    session_id: &str,
    input: &BatchMemoryTransitionInput,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-memory-transition-v3-batch");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(input.expected_event_count.to_le_bytes());
    hasher.update([0]);
    hasher.update(normalize(&input.checkpoint_summary).as_bytes());
    for kind in [
        MemoryCandidateKind::Plan,
        MemoryCandidateKind::Decision,
        MemoryCandidateKind::Task,
        MemoryCandidateKind::Problem,
        MemoryCandidateKind::Unresolved,
    ] {
        for candidate in input
            .candidates
            .iter()
            .filter(|item| batch_kind(item) == kind)
        {
            update_batch_candidate_hash(&mut hasher, candidate);
        }
    }
    hash_deferred(&mut hasher, &input.deferred_evidence_record_ids);
    format!("sha256:{:x}", hasher.finalize())
}

pub(crate) fn rich_problem_candidate_fingerprint(
    session_id: &str,
    input: &RichProblemMemoryTransitionInput,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-memory-transition-v4-rich-problem");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(input.expected_event_count.to_le_bytes());
    update_rich_problem_hash(&mut hasher, &input.candidate);
    hash_deferred(&mut hasher, &input.deferred_evidence_record_ids);
    format!("sha256:{:x}", hasher.finalize())
}

pub(crate) fn composite_candidate_fingerprint(
    session_id: &str,
    input: &CompositeMemoryTransitionInput,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-memory-transition-v5-composite");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(input.expected_event_count.to_le_bytes());
    hasher.update([0]);
    hasher.update(normalize(&input.checkpoint_summary).as_bytes());
    for kind in [
        MemoryCandidateKind::Plan,
        MemoryCandidateKind::Decision,
        MemoryCandidateKind::Task,
        MemoryCandidateKind::Problem,
        MemoryCandidateKind::Unresolved,
    ] {
        if kind == MemoryCandidateKind::Problem {
            hasher.update([0xa0]);
            update_rich_problem_hash(&mut hasher, &input.rich_problem);
        }
        for candidate in input
            .siblings
            .iter()
            .filter(|item| batch_kind(item) == kind)
        {
            update_batch_candidate_hash(&mut hasher, candidate);
        }
    }
    hash_deferred(&mut hasher, &input.deferred_evidence_record_ids);
    format!("sha256:{:x}", hasher.finalize())
}

fn batch_kind(candidate: &BatchMemoryCandidateClaim) -> MemoryCandidateKind {
    match candidate {
        BatchMemoryCandidateClaim::Unresolved { .. } => MemoryCandidateKind::Unresolved,
        BatchMemoryCandidateClaim::Decision { .. } => MemoryCandidateKind::Decision,
        BatchMemoryCandidateClaim::Problem { .. } => MemoryCandidateKind::Problem,
        BatchMemoryCandidateClaim::Task { .. } => MemoryCandidateKind::Task,
        BatchMemoryCandidateClaim::Plan { .. } => MemoryCandidateKind::Plan,
    }
}

fn update_batch_candidate_hash(hasher: &mut Sha256, candidate: &BatchMemoryCandidateClaim) {
    let (kind, fields, evidence) = match candidate {
        BatchMemoryCandidateClaim::Unresolved {
            text,
            evidence_record_ids,
        } => (
            b"unresolved".as_slice(),
            vec![normalize(text)],
            evidence_record_ids,
        ),
        BatchMemoryCandidateClaim::Decision {
            title,
            decision,
            evidence_record_ids,
        } => (
            b"decision".as_slice(),
            vec![normalize(title), normalize(decision)],
            evidence_record_ids,
        ),
        BatchMemoryCandidateClaim::Problem {
            title,
            symptom,
            evidence_record_ids,
        } => (
            b"problem".as_slice(),
            vec![normalize(title), normalize(symptom)],
            evidence_record_ids,
        ),
        BatchMemoryCandidateClaim::Task {
            title,
            status,
            details,
            evidence_record_ids,
        } => (
            b"task".as_slice(),
            vec![
                normalize(title),
                task_status_label(*status).to_owned(),
                normalize(details),
            ],
            evidence_record_ids,
        ),
        BatchMemoryCandidateClaim::Plan {
            text,
            status,
            evidence_record_ids,
        } => (
            b"plan".as_slice(),
            vec![normalize(text), plan_status_label(*status).to_owned()],
            evidence_record_ids,
        ),
    };
    hasher.update([0]);
    hasher.update(kind);
    for field in fields {
        hasher.update([0]);
        hasher.update(field.as_bytes());
    }
    hash_evidence(hasher, evidence, 0);
    hasher.update([0xff]);
}

fn update_rich_problem_hash(hasher: &mut Sha256, candidate: &RichProblemMemoryCandidate) {
    for field in [&candidate.title, &candidate.symptom, &candidate.expected] {
        hasher.update([0]);
        hasher.update(normalize(field).as_bytes());
    }
    hash_sorted_evidence(hasher, 0xe0, &candidate.evidence_record_ids);
    for attempt in &candidate.attempts {
        hasher.update([0xa1]);
        hasher.update(normalize(&attempt.action).as_bytes());
        hasher.update([0]);
        hasher.update(attempt_outcome_label(attempt.outcome).as_bytes());
        hasher.update([0]);
        hasher.update(normalize(&attempt.evidence).as_bytes());
        hash_sorted_evidence(hasher, 0xe1, &attempt.evidence_record_ids);
    }
    if let Some(resolution) = &candidate.resolution {
        hasher.update([0xa2]);
        for field in [
            &resolution.root_cause,
            &resolution.change,
            &resolution.verification,
        ] {
            hasher.update([0]);
            hasher.update(normalize(field).as_bytes());
        }
        hash_sorted_evidence(hasher, 0xe2, &resolution.evidence_record_ids);
    } else {
        hasher.update([0xa3]);
    }
}

pub(crate) fn observed_command_candidate_fingerprint(
    session_id: &str,
    expected_event_count: u64,
    source_record_id: &str,
    source_event_id: &str,
    observation_kind: Option<ToolObservationKind>,
    command: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-memory-transition-v6-observed-command");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(expected_event_count.to_le_bytes());
    hasher.update([0]);
    hasher.update(source_record_id.as_bytes());
    hasher.update([0]);
    hasher.update(source_event_id.as_bytes());
    hasher.update([0]);
    hasher.update(match observation_kind {
        Some(ToolObservationKind::Returned) => b"returned".as_slice(),
        Some(ToolObservationKind::ExplicitFailure) => b"explicit-failure".as_slice(),
        None => b"missing".as_slice(),
    });
    hasher.update([0]);
    hasher.update(command.as_bytes());
    hasher.update([0]);
    hasher.update(OBSERVED_COMMAND_CANDIDATE_SUMMARY.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn hash_evidence(hasher: &mut Sha256, evidence_record_ids: &[String], marker: u8) {
    let mut evidence = evidence_record_ids.to_vec();
    evidence.sort();
    for record_id in evidence {
        hasher.update([marker]);
        hasher.update(record_id.as_bytes());
    }
}

fn hash_sorted_evidence(hasher: &mut Sha256, marker: u8, evidence_record_ids: &[String]) {
    let mut evidence = evidence_record_ids.to_vec();
    evidence.sort();
    for record_id in evidence {
        hasher.update([marker]);
        hasher.update(record_id.as_bytes());
    }
    hasher.update([marker, 0xff]);
}

fn hash_deferred(hasher: &mut Sha256, evidence_record_ids: &[String]) {
    let mut deferred = evidence_record_ids.to_vec();
    deferred.sort();
    for record_id in deferred {
        hasher.update([0xfe]);
        hasher.update(record_id.as_bytes());
    }
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn plan_status_label(status: PlanStatus) -> &'static str {
    match status {
        PlanStatus::Pending => "pending",
        PlanStatus::InProgress => "in-progress",
        PlanStatus::Completed => "completed",
        PlanStatus::Blocked => "blocked",
    }
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::InProgress => "in-progress",
        TaskStatus::Completed => "completed",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Cancelled => "cancelled",
    }
}

fn attempt_outcome_label(outcome: AttemptOutcome) -> &'static str {
    match outcome {
        AttemptOutcome::Helped => "helped",
        AttemptOutcome::NoEffect => "no-effect",
        AttemptOutcome::Worsened => "worsened",
        AttemptOutcome::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(char: char) -> String {
        format!("tev_{}", char.to_string().repeat(32))
    }

    #[test]
    fn schema_v3_and_v8_candidate_fingerprints_are_byte_stable() {
        // Fixed inputs mirror the one-claim shapes persisted by schema-v3 and schema-v8.
        let session_id = format!("ses_{}", "1".repeat(32));
        assert_eq!(
            unresolved_candidate_fingerprint(
                &session_id,
                3,
                "Retry investigation",
                "The retry remains unresolved",
                &[evidence('b'), evidence('a')],
            ),
            "sha256:66bb0c578ada22e7b84facec127ea27e231aae0f62827cafe8dff2d580893561"
        );
        assert_eq!(
            recovery_candidate_fingerprint(
                &session_id,
                3,
                MemoryCandidateKind::Decision,
                "Storage engine",
                "Use SQLite",
                &[evidence('b'), evidence('a')],
            ),
            "sha256:336a7842de2ae26e4ae1b238e98a8997fe25466c3a72784efa2d9b38f1785130"
        );

        // Retain the original generic-v1 stability vector too.
        let fingerprint = recovery_candidate_fingerprint(
            &session_id,
            3,
            MemoryCandidateKind::Summary,
            "recovery",
            "state remains tentative",
            &[evidence('b'), evidence('a')],
        );
        assert_eq!(
            fingerprint,
            "sha256:60ec88a999de9f414565284ecb7a301990af31754d1ec22383023930fc8ce4f0"
        );
    }

    #[test]
    fn schema_v9_v10_and_v11_candidate_fingerprints_are_byte_stable() {
        let session_id = format!("ses_{}", "1".repeat(32));
        assert_eq!(
            task_candidate_fingerprint(
                &session_id,
                3,
                "Release build",
                TaskStatus::Completed,
                "Smoke test passed",
                &[evidence('b'), evidence('a')]
            ),
            "sha256:69a5a1839ba8559de288025d1f1e9b47e72e7206c2fe197332a43343a35856be"
        );
        assert_eq!(
            plan_candidate_fingerprint(
                &session_id,
                3,
                "Release build",
                PlanStatus::InProgress,
                &[evidence('b'), evidence('a')]
            ),
            "sha256:10ccf80c17447147aadd326f7981a493c68eab5397153c178d5577e882f7731c"
        );
        let batch = BatchMemoryTransitionInput {
            expected_event_count: 3,
            checkpoint_summary: "Recovered persistence work".to_owned(),
            candidates: vec![
                BatchMemoryCandidateClaim::Decision {
                    title: "Storage engine".to_owned(),
                    decision: "Use SQLite".to_owned(),
                    evidence_record_ids: vec![evidence('b'), evidence('a')],
                },
                BatchMemoryCandidateClaim::Task {
                    title: "Migrate local state".to_owned(),
                    status: TaskStatus::Completed,
                    details: "Migration completed".to_owned(),
                    evidence_record_ids: vec![evidence('b')],
                },
            ],
            deferred_evidence_record_ids: Vec::new(),
        };
        assert_eq!(
            batch_candidate_fingerprint(&session_id, &batch),
            "sha256:b1e1ca2d570c2385f53da5cb76cc4b1ab9dbc49f7da225f8fc7d358067b409c9"
        );
    }

    #[test]
    fn schema_v12_v13_v16_candidate_fingerprints_are_byte_stable() {
        let session_id = format!("ses_{}", "1".repeat(32));
        let rich = RichProblemMemoryCandidate {
            title: "Login refresh failure".to_owned(),
            symptom: "Refreshing returns 401".to_owned(),
            expected: "The authenticated session survives refresh".to_owned(),
            evidence_record_ids: vec![evidence('b'), evidence('a')],
            attempts: vec![RichProblemAttemptCandidate {
                action: "Clear browser cookies".to_owned(),
                outcome: AttemptOutcome::NoEffect,
                evidence: "Refresh still returned 401".to_owned(),
                evidence_record_ids: vec![evidence('b')],
            }],
            resolution: Some(RichProblemResolutionCandidate {
                root_cause: "The client reused an expired access token".to_owned(),
                change: "Refresh the token before protected navigation".to_owned(),
                verification: "Repeated refreshes remained authenticated".to_owned(),
                evidence_record_ids: vec![evidence('a')],
            }),
        };
        let rich_input = RichProblemMemoryTransitionInput {
            expected_event_count: 3,
            candidate: rich.clone(),
            deferred_evidence_record_ids: Vec::new(),
        };
        assert_eq!(
            rich_problem_candidate_fingerprint(&session_id, &rich_input),
            "sha256:d6e5f1742fcaf70a14ae6a9584f5da2931f5900f734646cf231718f5f589e3a6"
        );

        let composite = CompositeMemoryTransitionInput {
            expected_event_count: 23,
            checkpoint_summary: "Recovered protected-route debugging and policy".to_owned(),
            rich_problem: rich,
            siblings: vec![BatchMemoryCandidateClaim::Decision {
                title: "Protected route refresh policy".to_owned(),
                decision: "Refresh before every protected navigation".to_owned(),
                evidence_record_ids: vec![evidence('a')],
            }],
            deferred_evidence_record_ids: Vec::new(),
        };
        assert_eq!(
            composite_candidate_fingerprint(&session_id, &composite),
            "sha256:f0b578e6fdcbe3146cdbd11a2aa3287251fb041077cd1d8e13ae6b8d78bb89a1"
        );

        assert_eq!(
            observed_command_candidate_fingerprint(
                &session_id,
                4,
                &format!("toe_{}", "a".repeat(32)),
                &format!("evt_{}", "b".repeat(64)),
                Some(ToolObservationKind::Returned),
                "cargo test -p ley-core"
            ),
            "sha256:7a2fad1f586be66f0e0b0f50adc7a54848f4a099b13f22ca568d52ba8b97719b"
        );
    }
}
