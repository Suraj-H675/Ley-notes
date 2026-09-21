use crate::session::read_session;
use crate::{
    derive_specification_acceptance_criteria, derive_specification_verification_methods,
    LeyCoreError, SpecificationAcceptanceCriteriaState, SpecificationAcceptanceCriterion,
    SpecificationRegistry, SpecificationVerificationMethod, SpecificationVerificationMethodsState,
    VerificationRecord,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

const RELATIONSHIP_BOUNDARY: &str = "caller-supplied-criterion-verification-review-link";
const METHOD_RELATIONSHIP_BOUNDARY: &str =
    "caller-supplied-criterion-method-verification-review-link";
const VERIFICATION_SOURCE_BOUNDARY: &str = "untrusted-historical-verification";
const REVIEW_NOTICE: &str = "Ley verified only that the acceptance criterion belongs to the current approved Specification revision, that any supplied Verification method belongs to that same exact revision, and that the cited Verification record exists in the fixed-project session ledger. The caller supplied every relationship between those handles. A method row does not prove execution or outcome, and a passed Verification does not prove semantic coverage, criterion satisfaction, or that current live source still passes.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptanceCriterionVerificationReviewInput<'a> {
    pub specification_id: &'a str,
    pub criterion_id: &'a str,
    pub verification_method_id: Option<&'a str>,
    pub session_id: &'a str,
    pub verification_record_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceCriterionVerificationReview {
    pub project_id: String,
    pub specification_id: String,
    pub specification_content_hash: String,
    pub criterion: SpecificationAcceptanceCriterion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_method: Option<SpecificationVerificationMethod>,
    pub session_id: String,
    pub checkpoint_id: String,
    pub checkpoint_event_id: String,
    pub checkpoint_recorded_at_unix_ms: u64,
    pub verification: VerificationRecord,
    pub link_fingerprint: String,
    pub specification_source_revision_checked: bool,
    pub verification_method_checked: bool,
    pub verification_record_checked: bool,
    pub relationship_supplied_by_caller: bool,
    pub verification_method_relationship_supplied_by_caller: bool,
    pub verification_method_execution_proven: bool,
    pub verification_method_outcome_proven: bool,
    pub verification_status_interpreted_as_satisfaction: bool,
    pub criterion_satisfaction_proven: bool,
    pub semantic_coverage_proven: bool,
    pub current_implementation_proven: bool,
    pub persisted: bool,
    pub automatic_write_allowed: bool,
    pub live_source_checked: bool,
    pub criterion_authority: &'static str,
    pub verification_source_boundary: &'static str,
    pub relationship_boundary: &'static str,
    pub review_notice: &'static str,
}

pub fn review_acceptance_criterion_verification(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    specifications: &SpecificationRegistry,
    specification_id: &str,
    criterion_id: &str,
    session_id: &str,
    verification_record_id: &str,
) -> Result<AcceptanceCriterionVerificationReview, LeyCoreError> {
    review_acceptance_criterion_verification_with_method(
        project_start,
        vault,
        specifications,
        AcceptanceCriterionVerificationReviewInput {
            specification_id,
            criterion_id,
            verification_method_id: None,
            session_id,
            verification_record_id,
        },
    )
}

pub fn review_acceptance_criterion_verification_with_method(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    specifications: &SpecificationRegistry,
    input: AcceptanceCriterionVerificationReviewInput<'_>,
) -> Result<AcceptanceCriterionVerificationReview, LeyCoreError> {
    if !valid_prefixed_hex(input.criterion_id, "acr_", 64) {
        return Err(LeyCoreError::InvalidSpecificationRequest(
            "criterionId must be an acr_ identifier".to_owned(),
        ));
    }
    if input
        .verification_method_id
        .is_some_and(|id| !valid_prefixed_hex(id, "vmd_", 64))
    {
        return Err(LeyCoreError::InvalidSpecificationRequest(
            "verificationMethodId must be a vmd_ identifier".to_owned(),
        ));
    }
    if !valid_prefixed_hex(input.verification_record_id, "ver_", 32) {
        return Err(LeyCoreError::InvalidSessionRequest(
            "verificationRecordId must be a ver_ identifier".to_owned(),
        ));
    }

    let source = specifications.read_approved_source(
        project_start.as_ref(),
        vault.as_ref(),
        input.specification_id,
    )?;
    let projection = derive_specification_acceptance_criteria(
        &source.specification_id,
        &source.content_hash,
        &source.source,
    );
    if projection.state != SpecificationAcceptanceCriteriaState::Available {
        return Err(LeyCoreError::InvalidSpecificationRequest(format!(
            "approved Specification {} has no available acceptance criteria projection",
            source.specification_id
        )));
    }
    let criterion = projection
        .criteria
        .into_iter()
        .find(|criterion| criterion.criterion_id == input.criterion_id)
        .ok_or_else(|| {
            LeyCoreError::InvalidSpecificationRequest(format!(
                "acceptance criterion is not part of the current approved Specification revision: {}",
                input.criterion_id
            ))
        })?;

    let verification_method = if let Some(verification_method_id) = input.verification_method_id {
        let projection = derive_specification_verification_methods(
            &source.specification_id,
            &source.content_hash,
            &source.source,
        );
        if projection.state != SpecificationVerificationMethodsState::Available {
            return Err(LeyCoreError::InvalidSpecificationRequest(format!(
                "approved Specification {} has no available Verification-method projection",
                source.specification_id
            )));
        }
        Some(
            projection
                .methods
                .into_iter()
                .find(|method| method.method_id == verification_method_id)
                .ok_or_else(|| {
                    LeyCoreError::InvalidSpecificationRequest(format!(
                        "Verification method is not part of the current approved Specification revision: {verification_method_id}"
                    ))
                })?,
        )
    } else {
        None
    };

    let session = read_session(project_start, vault, input.session_id)?;
    if session.project_id != source.project_id {
        return Err(LeyCoreError::InvalidProjectIdentity(
            "acceptance criterion and Verification session resolved to different projects"
                .to_owned(),
        ));
    }

    let mut matched = None;
    for checkpoint in &session.checkpoints {
        for verification in &checkpoint.verification {
            if verification.id != input.verification_record_id {
                continue;
            }
            if matched.is_some() {
                return Err(LeyCoreError::InvalidSessionStore(
                    "Verification record ID is duplicated across session checkpoints".to_owned(),
                ));
            }
            matched = Some((
                checkpoint.id.clone(),
                checkpoint.event_id.clone(),
                checkpoint.recorded_at_unix_ms,
                verification.clone(),
            ));
        }
    }
    let (checkpoint_id, checkpoint_event_id, checkpoint_recorded_at_unix_ms, verification) =
        matched.ok_or_else(|| {
            LeyCoreError::InvalidSessionRequest(format!(
                "Verification record is not present in this session: {}",
                input.verification_record_id
            ))
        })?;

    let link_fingerprint = match verification_method.as_ref() {
        Some(method) => method_link_fingerprint(
            &source.project_id,
            &source.content_hash,
            &checkpoint_id,
            AcceptanceCriterionVerificationReviewInput {
                verification_method_id: Some(&method.method_id),
                ..input
            },
        ),
        None => link_fingerprint(
            &source.project_id,
            &source.specification_id,
            &source.content_hash,
            &criterion.criterion_id,
            &session.session_id,
            &checkpoint_id,
            &verification.id,
        ),
    };
    let verification_method_checked = verification_method.is_some();

    Ok(AcceptanceCriterionVerificationReview {
        project_id: source.project_id,
        specification_id: source.specification_id,
        specification_content_hash: source.content_hash,
        criterion,
        verification_method,
        session_id: session.session_id,
        checkpoint_id,
        checkpoint_event_id,
        checkpoint_recorded_at_unix_ms,
        verification,
        link_fingerprint,
        specification_source_revision_checked: true,
        verification_method_checked,
        verification_record_checked: true,
        relationship_supplied_by_caller: true,
        verification_method_relationship_supplied_by_caller: verification_method_checked,
        verification_method_execution_proven: false,
        verification_method_outcome_proven: false,
        verification_status_interpreted_as_satisfaction: false,
        criterion_satisfaction_proven: false,
        semantic_coverage_proven: false,
        current_implementation_proven: false,
        persisted: false,
        automatic_write_allowed: false,
        live_source_checked: false,
        criterion_authority: "human-intent",
        verification_source_boundary: VERIFICATION_SOURCE_BOUNDARY,
        relationship_boundary: if verification_method_checked {
            METHOD_RELATIONSHIP_BOUNDARY
        } else {
            RELATIONSHIP_BOUNDARY
        },
        review_notice: REVIEW_NOTICE,
    })
}

fn valid_prefixed_hex(value: &str, prefix: &str, hex_length: usize) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        suffix.len() == hex_length
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn link_fingerprint(
    project_id: &str,
    specification_id: &str,
    specification_content_hash: &str,
    criterion_id: &str,
    session_id: &str,
    checkpoint_id: &str,
    verification_record_id: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ley-acceptance-criterion-verification-review-v1");
    for value in [
        project_id,
        specification_id,
        specification_content_hash,
        criterion_id,
        session_id,
        checkpoint_id,
        verification_record_id,
    ] {
        digest.update([0]);
        digest.update(value.as_bytes());
    }
    format!("sha256:{:x}", digest.finalize())
}

fn method_link_fingerprint(
    project_id: &str,
    specification_content_hash: &str,
    checkpoint_id: &str,
    input: AcceptanceCriterionVerificationReviewInput<'_>,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ley-acceptance-criterion-method-verification-review-v2");
    for value in [
        project_id,
        input.specification_id,
        specification_content_hash,
        input.criterion_id,
        input
            .verification_method_id
            .expect("method-aware fingerprint requires a Verification method"),
        input.session_id,
        checkpoint_id,
        input.verification_record_id,
    ] {
        digest.update([0]);
        digest.update(value.as_bytes());
    }
    format!("sha256:{:x}", digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        checkpoint_session, ingest_project, initialize_project, start_session, CaptureMode,
        CheckpointInput, SessionSource, StartSessionInput, VerificationInput, VerificationStatus,
    };
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
    }

    struct Fixture {
        _temporary: tempfile::TempDir,
        project: PathBuf,
        vault: PathBuf,
        registry: SpecificationRegistry,
        specification_id: String,
        criterion_id: String,
        verification_method_id: String,
        session_id: String,
        verification_record_id: String,
    }

    fn fixture() -> Fixture {
        let temporary = tempdir().unwrap();
        let project = temporary.path().join("project");
        let vault = temporary.path().join("vault");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(project.join("README.md"), "# App\n").unwrap();
        initialize_project(
            &project,
            Some("Acceptance verification"),
            CaptureMode::Structured,
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();

        fs::create_dir(vault.join("Specs")).unwrap();
        fs::write(
            vault.join("Specs/Offline.md"),
            "# Offline startup\n\n## Acceptance criteria\n\n- CLI startup works offline.\n\n## Verification method\n\n- Run the offline startup smoke test.\n",
        )
        .unwrap();
        let registry =
            SpecificationRegistry::at(temporary.path().join("config/specifications.json"));
        let specification_id = "spec_11111111111111111111111111111111".to_owned();
        registry
            .approve(&project, &vault, &specification_id, "Specs/Offline.md")
            .unwrap();
        let approved = registry
            .read_approved_source(&project, &vault, &specification_id)
            .unwrap();
        let criteria = derive_specification_acceptance_criteria(
            &approved.specification_id,
            &approved.content_hash,
            &approved.source,
        );
        let criterion_id = criteria.criteria[0].criterion_id.clone();
        let methods = derive_specification_verification_methods(
            &approved.specification_id,
            &approved.content_hash,
            &approved.source,
        );
        let verification_method_id = methods.methods[0].method_id.clone();

        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('1'),
                name: "Offline startup".to_owned(),
                goal: "Verify offline startup".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let session_id = started.session.session_id;
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &session_id,
            CheckpointInput {
                request_id: request_id('2'),
                summary: "Ran the offline startup smoke test.".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: Vec::new(),
                commands: Vec::new(),
                verification: vec![VerificationInput {
                    kind: "test".to_owned(),
                    status: VerificationStatus::Passed,
                    summary: "Offline startup smoke test passed.".to_owned(),
                    command: Some("cargo test offline_startup".to_owned()),
                    evidence_artifact_paths: Vec::new(),
                }],
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let verification_record_id = checkpoint.session.checkpoints[0].verification[0].id.clone();

        Fixture {
            _temporary: temporary,
            project,
            vault,
            registry,
            specification_id,
            criterion_id,
            verification_method_id,
            session_id,
            verification_record_id,
        }
    }

    #[test]
    fn review_binds_current_criterion_and_exact_verification_without_claiming_satisfaction() {
        let fixture = fixture();
        let event_count_before =
            crate::read_session(&fixture.project, &fixture.vault, &fixture.session_id)
                .unwrap()
                .event_count;
        let first = review_acceptance_criterion_verification(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            &fixture.specification_id,
            &fixture.criterion_id,
            &fixture.session_id,
            &fixture.verification_record_id,
        )
        .unwrap();
        let second = review_acceptance_criterion_verification(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            &fixture.specification_id,
            &fixture.criterion_id,
            &fixture.session_id,
            &fixture.verification_record_id,
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.criterion.criterion_id, fixture.criterion_id);
        assert_eq!(first.criterion.text, "- CLI startup works offline.");
        assert_eq!(first.verification.id, fixture.verification_record_id);
        assert_eq!(first.verification.status, VerificationStatus::Passed);
        assert!(first.link_fingerprint.starts_with("sha256:"));
        assert!(first.specification_source_revision_checked);
        assert!(!first.verification_method_checked);
        assert!(first.verification_record_checked);
        assert!(first.relationship_supplied_by_caller);
        assert!(!first.verification_method_relationship_supplied_by_caller);
        assert!(!first.verification_method_execution_proven);
        assert!(!first.verification_method_outcome_proven);
        assert!(first.verification_method.is_none());
        assert!(!first.verification_status_interpreted_as_satisfaction);
        assert!(!first.criterion_satisfaction_proven);
        assert!(!first.semantic_coverage_proven);
        assert!(!first.current_implementation_proven);
        assert!(!first.persisted);
        assert!(!first.automatic_write_allowed);
        assert!(!first.live_source_checked);
        assert_eq!(first.criterion_authority, "human-intent");
        assert_eq!(
            first.verification_source_boundary,
            "untrusted-historical-verification"
        );
        assert_eq!(
            first.relationship_boundary,
            "caller-supplied-criterion-verification-review-link"
        );
        let event_count_after =
            crate::read_session(&fixture.project, &fixture.vault, &fixture.session_id)
                .unwrap()
                .event_count;
        assert_eq!(event_count_before, event_count_after);
    }

    #[test]
    fn review_optionally_binds_exact_verification_method_without_inventing_semantics() {
        let fixture = fixture();
        let event_count_before =
            crate::read_session(&fixture.project, &fixture.vault, &fixture.session_id)
                .unwrap()
                .event_count;
        let first = review_acceptance_criterion_verification_with_method(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            AcceptanceCriterionVerificationReviewInput {
                specification_id: &fixture.specification_id,
                criterion_id: &fixture.criterion_id,
                verification_method_id: Some(&fixture.verification_method_id),
                session_id: &fixture.session_id,
                verification_record_id: &fixture.verification_record_id,
            },
        )
        .unwrap();
        let second = review_acceptance_criterion_verification_with_method(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            AcceptanceCriterionVerificationReviewInput {
                specification_id: &fixture.specification_id,
                criterion_id: &fixture.criterion_id,
                verification_method_id: Some(&fixture.verification_method_id),
                session_id: &fixture.session_id,
                verification_record_id: &fixture.verification_record_id,
            },
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.criterion.criterion_id, fixture.criterion_id);
        let method = first.verification_method.as_ref().unwrap();
        assert_eq!(method.method_id, fixture.verification_method_id);
        assert_eq!(method.text, "- Run the offline startup smoke test.");
        assert_eq!(first.verification.id, fixture.verification_record_id);
        assert!(first.link_fingerprint.starts_with("sha256:"));
        assert!(first.specification_source_revision_checked);
        assert!(first.verification_method_checked);
        assert!(first.verification_record_checked);
        assert!(first.relationship_supplied_by_caller);
        assert!(first.verification_method_relationship_supplied_by_caller);
        assert!(!first.verification_method_execution_proven);
        assert!(!first.verification_method_outcome_proven);
        assert!(!first.verification_status_interpreted_as_satisfaction);
        assert!(!first.criterion_satisfaction_proven);
        assert!(!first.semantic_coverage_proven);
        assert!(!first.current_implementation_proven);
        assert!(!first.persisted);
        assert!(!first.automatic_write_allowed);
        assert!(!first.live_source_checked);
        assert_eq!(
            first.relationship_boundary,
            "caller-supplied-criterion-method-verification-review-link"
        );

        let legacy = review_acceptance_criterion_verification(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            &fixture.specification_id,
            &fixture.criterion_id,
            &fixture.session_id,
            &fixture.verification_record_id,
        )
        .unwrap();
        assert_ne!(first.link_fingerprint, legacy.link_fingerprint);
        assert_eq!(
            crate::read_session(&fixture.project, &fixture.vault, &fixture.session_id)
                .unwrap()
                .event_count,
            event_count_before
        );
    }

    #[test]
    fn review_fails_closed_when_specification_revision_changes() {
        let fixture = fixture();
        fs::write(
            fixture.vault.join("Specs/Offline.md"),
            "# Offline startup\n\n## Acceptance criteria\n\n- CLI startup works offline and syncs later.\n",
        )
        .unwrap();

        let error = review_acceptance_criterion_verification(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            &fixture.specification_id,
            &fixture.criterion_id,
            &fixture.session_id,
            &fixture.verification_record_id,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            LeyCoreError::SpecificationApprovalStale { .. }
        ));
    }

    #[test]
    fn review_rejects_unknown_criterion_or_verification_handles() {
        let fixture = fixture();
        let unknown_criterion = format!("acr_{}", "a".repeat(64));
        let criterion_error = review_acceptance_criterion_verification(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            &fixture.specification_id,
            &unknown_criterion,
            &fixture.session_id,
            &fixture.verification_record_id,
        )
        .unwrap_err();
        assert!(matches!(
            criterion_error,
            LeyCoreError::InvalidSpecificationRequest(message)
                if message.contains("current approved Specification revision")
        ));

        let unknown_verification = format!("ver_{}", "b".repeat(32));
        let verification_error = review_acceptance_criterion_verification(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            &fixture.specification_id,
            &fixture.criterion_id,
            &fixture.session_id,
            &unknown_verification,
        )
        .unwrap_err();
        assert!(matches!(
            verification_error,
            LeyCoreError::InvalidSessionRequest(message)
                if message.contains("Verification record is not present")
        ));

        let malformed_verification_error = review_acceptance_criterion_verification(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            &fixture.specification_id,
            &fixture.criterion_id,
            &fixture.session_id,
            "ver_not_hex",
        )
        .unwrap_err();
        assert!(matches!(
            malformed_verification_error,
            LeyCoreError::InvalidSessionRequest(message)
                if message.contains("verificationRecordId")
        ));

        let unknown_method = format!("vmd_{}", "c".repeat(64));
        let method_error = review_acceptance_criterion_verification_with_method(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            AcceptanceCriterionVerificationReviewInput {
                specification_id: &fixture.specification_id,
                criterion_id: &fixture.criterion_id,
                verification_method_id: Some(&unknown_method),
                session_id: &fixture.session_id,
                verification_record_id: &fixture.verification_record_id,
            },
        )
        .unwrap_err();
        assert!(matches!(
            method_error,
            LeyCoreError::InvalidSpecificationRequest(message)
                if message.contains("Verification method is not part")
        ));

        let malformed_method_error = review_acceptance_criterion_verification_with_method(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            AcceptanceCriterionVerificationReviewInput {
                specification_id: &fixture.specification_id,
                criterion_id: &fixture.criterion_id,
                verification_method_id: Some("vmd_not_hex"),
                session_id: &fixture.session_id,
                verification_record_id: &fixture.verification_record_id,
            },
        )
        .unwrap_err();
        assert!(matches!(
            malformed_method_error,
            LeyCoreError::InvalidSpecificationRequest(message)
                if message.contains("verificationMethodId")
        ));
    }

    #[test]
    fn review_rejects_cross_project_session_substitution() {
        let fixture = fixture();
        let other_project = fixture._temporary.path().join("other-project");
        fs::create_dir(&other_project).unwrap();
        initialize_project(
            &other_project,
            Some("Other acceptance project"),
            CaptureMode::Structured,
        )
        .unwrap();
        ingest_project(&other_project, &fixture.vault).unwrap();
        let other = start_session(
            &other_project,
            &fixture.vault,
            StartSessionInput {
                request_id: request_id('3'),
                name: "Other session".to_owned(),
                goal: "Must not satisfy the first project's criterion".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();

        let error = review_acceptance_criterion_verification(
            &fixture.project,
            &fixture.vault,
            &fixture.registry,
            &fixture.specification_id,
            &fixture.criterion_id,
            &other.session.session_id,
            &fixture.verification_record_id,
        )
        .unwrap_err();
        assert!(matches!(error, LeyCoreError::SessionNotFound(_)));
    }
}
