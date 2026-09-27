use crate::{diagnose_project, ContinuityEventInput, ContinuityStore, LeyCoreError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;

pub const LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyContinuityImportSummary {
    pub project_id: String,
    pub session_events: usize,
    pub learning_events: usize,
    pub source_digest: String,
    pub manifest_event_id: String,
    pub project_created: bool,
    pub continuity_events_created: usize,
    pub continuity_events_replayed: usize,
}

pub fn import_legacy_continuity(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<LegacyContinuityImportSummary, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    let session_events = crate::session::continuity_events_for_migration(&diagnostic.root, &vault)?;
    let learning_events =
        crate::learning::continuity_events_for_migration(&diagnostic.root, &vault)?;
    let session_event_count = session_events.len();
    let learning_event_count = learning_events.len();

    let mut events = session_events;
    events.extend(learning_events);
    events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    let source_digest = event_inventory_digest(&events)?;
    let digest_hex = source_digest
        .strip_prefix("sha256:")
        .expect("inventory digests always use sha256");
    let manifest_event_id = format!("mig_{digest_hex}");
    let manifest_timestamp = events
        .iter()
        .map(|event| event.recorded_at_unix_ms)
        .max()
        .unwrap_or(diagnostic.identity.created_at_unix_ms)
        .max(1);
    events.push(ContinuityEventInput {
        event_id: manifest_event_id.clone(),
        project_id: diagnostic.identity.project_id.clone(),
        session_id: None,
        session_sequence: None,
        request_id: None,
        request_fingerprint: None,
        kind: "legacy-snapshot-imported".to_owned(),
        payload_version: LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION,
        recorded_at_unix_ms: manifest_timestamp,
        revision_head: None,
        revision_branch: None,
        payload: json!({
            "source": "legacy-agent-memory",
            "formatVersion": LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION,
            "eventDigest": source_digest,
            "sessionEventCount": session_event_count,
            "learningEventCount": learning_event_count,
            "cutover": false
        }),
    });

    let imported = store.import_project_events(&diagnostic.identity, &events)?;
    Ok(LegacyContinuityImportSummary {
        project_id: diagnostic.identity.project_id,
        session_events: session_event_count,
        learning_events: learning_event_count,
        source_digest,
        manifest_event_id,
        project_created: imported.project_created,
        continuity_events_created: imported.events_created,
        continuity_events_replayed: imported.events_replayed,
    })
}

fn event_inventory_digest(events: &[ContinuityEventInput]) -> Result<String, LeyCoreError> {
    let bytes = serde_json::to_vec(events).map_err(|error| {
        LeyCoreError::InvalidContinuityStore(format!(
            "legacy continuity inventory could not be serialized: {error}"
        ))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        checkpoint_session, ingest_project, initialize_project, propose_learning, read_learning,
        read_session, start_session, CaptureMode, CheckpointInput, LearningActor,
        LearningEvidenceInput, LearningKind, LearningProvenance, ProposeLearningInput,
        SessionSource, SessionSourceKind, StartSessionInput, VerificationInput, VerificationStatus,
    };
    use std::process::Command;
    use tempfile::tempdir;

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
    }

    fn git(project: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(project)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn real_legacy_events_import_atomically_and_replay_without_mutating_source() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        git(&project, &["init", "-b", "main"]);
        git(&project, &["config", "user.name", "Ley Test"]);
        git(&project, &["config", "user.email", "ley@example.invalid"]);
        initialize_project(&project, Some("Legacy migration"), CaptureMode::Structured).unwrap();
        std::fs::write(project.join("README.md"), "# Migration fixture\n").unwrap();
        git(&project, &["add", "."]);
        git(&project, &["commit", "-m", "fixture"]);
        ingest_project(&project, &vault).unwrap();

        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('1'),
                name: "Verify migration".to_owned(),
                goal: "Preserve validated continuity history".to_owned(),
                source: SessionSource {
                    kind: SessionSourceKind::HostHook,
                    host: Some("codex".to_owned()),
                    agent: Some("gpt-6-luna".to_owned()),
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
                summary: "Validated the migration boundary".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: vec![VerificationInput {
                    kind: "command".to_owned(),
                    status: VerificationStatus::Passed,
                    summary: "Focused migration check passed".to_owned(),
                    command: Some("cargo test migration".to_owned()),
                    evidence_artifact_paths: vec!["README.md".to_owned()],
                }],
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let checkpoint_id = checkpoint.session.checkpoints[0].id.clone();
        let learning = propose_learning(
            &project,
            &vault,
            ProposeLearningInput {
                request_id: request_id('3'),
                actor: LearningActor::Agent,
                kind: LearningKind::Procedure,
                title: "Use the validated migration boundary".to_owned(),
                guidance: "Preserve validated raw events before retiring legacy storage."
                    .to_owned(),
                confidence_percent: 80,
                provenance: LearningProvenance::Inferred,
                evidence: vec![LearningEvidenceInput {
                    session_id: started.session.session_id.clone(),
                    record_id: checkpoint_id,
                    note: "The cited checkpoint contains the migration verification.".to_owned(),
                }],
            },
        )
        .unwrap();

        let store = ContinuityStore::at(base.path().join("private/continuity.sqlite3"));
        let first = import_legacy_continuity(&project, &vault, &store).unwrap();
        assert_eq!(first.session_events, 2);
        assert_eq!(first.learning_events, 1);
        assert!(first.project_created);
        assert_eq!(first.continuity_events_created, 4);
        assert_eq!(first.continuity_events_replayed, 0);

        let session_events = store
            .events_for_session(&first.project_id, &started.session.session_id)
            .unwrap();
        assert_eq!(session_events.len(), 2);
        assert_eq!(session_events[1].kind, "legacy-checkpoint-recorded");
        assert_eq!(session_events[1].payload_version, 1);
        assert_eq!(session_events[1].payload["schemaVersion"], 4);
        assert_eq!(session_events[1].payload["kind"], "checkpoint-recorded");
        assert_eq!(session_events[1].payload["requestId"], request_id('2'));
        assert_eq!(
            session_events[1].payload["data"]["verification"][0]["status"],
            "passed"
        );
        assert!(session_events[1].revision_head.is_some());
        assert!(
            session_events[1].payload["data"]["verification"][0]["evidenceArtifacts"][0]
                ["contentHash"]
                .as_str()
                .is_some_and(|hash| hash.starts_with("sha256:"))
        );

        let imported_learning = store
            .event(&first.project_id, &learning.event_id)
            .unwrap()
            .unwrap();
        assert_eq!(imported_learning.kind, "legacy-learning-proposed");
        assert!(imported_learning.session_id.is_none());
        assert!(imported_learning.request_id.is_none());
        assert_eq!(imported_learning.payload["kind"], "proposed");
        assert_eq!(imported_learning.payload["sequence"], 1);
        assert_eq!(
            imported_learning.payload["learningId"],
            learning.learning.learning_id
        );
        assert_eq!(imported_learning.payload["requestId"], request_id('3'));

        let manifest = store
            .event(&first.project_id, &first.manifest_event_id)
            .unwrap()
            .unwrap();
        assert_eq!(manifest.kind, "legacy-snapshot-imported");
        assert_eq!(manifest.payload["eventDigest"], first.source_digest);
        assert_eq!(manifest.payload["cutover"], false);

        let replayed = import_legacy_continuity(&project, &vault, &store).unwrap();
        assert_eq!(replayed.source_digest, first.source_digest);
        assert_eq!(replayed.manifest_event_id, first.manifest_event_id);
        assert!(!replayed.project_created);
        assert_eq!(replayed.continuity_events_created, 0);
        assert_eq!(replayed.continuity_events_replayed, 4);

        let identity = diagnose_project(&project).unwrap().identity;
        let mut conflict = crate::session::continuity_events_for_migration(&project, &vault)
            .unwrap()
            .remove(0);
        conflict.payload["kind"] = json!("tampered");
        let probe = ContinuityEventInput {
            event_id: format!("evt_{}", "f".repeat(64)),
            project_id: identity.project_id.clone(),
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "migration-probe".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 999,
            revision_head: None,
            revision_branch: None,
            payload: json!({"probe":true}),
        };
        assert!(store
            .import_project_events(&identity, &[probe.clone(), conflict])
            .is_err());
        assert!(store
            .event(&identity.project_id, &probe.event_id)
            .unwrap()
            .is_none());

        assert_eq!(
            read_session(&project, &vault, &started.session.session_id)
                .unwrap()
                .event_count,
            2
        );
        assert_eq!(
            read_learning(&project, &vault, &learning.learning.learning_id)
                .unwrap()
                .event_count,
            1
        );
    }
}
