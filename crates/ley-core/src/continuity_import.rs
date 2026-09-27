use crate::continuity_store::ContinuityEventLinkInput;
use crate::{diagnose_project, ContinuityEventInput, ContinuityStore, LeyCoreError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
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
    pub continuity_events_removed: usize,
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
    let links = legacy_event_links(&events)?;
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
        subject_id: None,
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

    let imported = store.sync_legacy_project_events(&diagnostic.identity, &events, &links)?;
    Ok(LegacyContinuityImportSummary {
        project_id: diagnostic.identity.project_id,
        session_events: session_event_count,
        learning_events: learning_event_count,
        source_digest,
        manifest_event_id,
        project_created: imported.project_created,
        continuity_events_created: imported.events_created,
        continuity_events_replayed: imported.events_replayed,
        continuity_events_removed: imported.events_removed,
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

fn legacy_event_links(
    events: &[ContinuityEventInput],
) -> Result<Vec<ContinuityEventLinkInput>, LeyCoreError> {
    let mut session_anchors = BTreeMap::<String, String>::new();
    let mut learning_roots = BTreeMap::<String, (u64, String)>::new();
    for event in events {
        if event.kind == "legacy-session-started" {
            let session_id = event.session_id.as_ref().ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(
                    "legacy session start event is missing session ID".to_owned(),
                )
            })?;
            if session_anchors
                .insert(session_id.clone(), event.event_id.clone())
                .is_some()
            {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "legacy session {session_id} has multiple start events"
                )));
            }
        }
        if let Some(subject_id) = &event.subject_id {
            let sequence = event.payload["sequence"].as_u64().ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "legacy learning event {} is missing sequence",
                    event.event_id
                ))
            })?;
            match learning_roots.get(subject_id) {
                Some((current, _)) if *current <= sequence => {}
                _ => {
                    learning_roots.insert(subject_id.clone(), (sequence, event.event_id.clone()));
                }
            }
        }
    }

    let mut links = BTreeSet::<(String, String, String)>::new();
    for event in events {
        match event.kind.as_str() {
            "legacy-learning-proposed" | "legacy-learning-corrected" => {
                let evidence = event.payload["data"]["evidence"]
                    .as_array()
                    .ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy learning event {} is missing evidence",
                            event.event_id
                        ))
                    })?;
                for item in evidence {
                    let session_id = item["sessionId"].as_str().ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy learning event {} has invalid session evidence",
                            event.event_id
                        ))
                    })?;
                    let target = session_anchors.get(session_id).ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy learning event {} cites missing session {session_id}",
                            event.event_id
                        ))
                    })?;
                    links.insert((
                        event.event_id.clone(),
                        target.clone(),
                        "depends-on-session".to_owned(),
                    ));
                }
            }
            "legacy-learning-reviewed"
                if event.payload["data"]["action"].as_str() == Some("supersede") =>
            {
                let replacement = event.payload["data"]["replacementLearningId"]
                    .as_str()
                    .ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy supersession event {} is missing replacement learning",
                            event.event_id
                        ))
                    })?;
                let (_, target) = learning_roots.get(replacement).ok_or_else(|| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "legacy supersession event {} references missing learning {replacement}",
                        event.event_id
                    ))
                })?;
                links.insert((
                    event.event_id.clone(),
                    target.clone(),
                    "supersedes".to_owned(),
                ));
            }
            _ => {}
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        checkpoint_session, erase_session_memory, ingest_project, initialize_project,
        propose_learning, read_learning, read_session, start_session, CaptureMode, CheckpointInput,
        EraseSessionMemoryInput, LearningActor, LearningEvidenceInput, LearningKind,
        LearningProvenance, ProposeLearningInput, SessionSource, SessionSourceKind,
        StartSessionInput, VerificationInput, VerificationStatus,
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
        assert_eq!(first.continuity_events_removed, 0);

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
        assert_eq!(
            imported_learning.subject_id.as_deref(),
            Some(learning.learning.learning_id.as_str())
        );
        assert!(imported_learning.session_id.is_none());
        assert!(imported_learning.request_id.is_none());
        assert_eq!(imported_learning.payload["kind"], "proposed");
        assert_eq!(imported_learning.payload["sequence"], 1);
        assert_eq!(
            imported_learning.payload["learningId"],
            learning.learning.learning_id
        );
        assert_eq!(imported_learning.payload["requestId"], request_id('3'));

        let native_preview = store
            .preview_session_erasure(&first.project_id, &started.session.session_id)
            .unwrap();
        assert_eq!(native_preview.session_event_count, 2);
        assert_eq!(native_preview.total_event_count, 3);
        assert_eq!(
            native_preview.dependent_subject_ids,
            vec![learning.learning.learning_id.clone()]
        );

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
        assert_eq!(replayed.continuity_events_removed, 0);

        let identity = diagnose_project(&project).unwrap().identity;
        let mut conflict = crate::session::continuity_events_for_migration(&project, &vault)
            .unwrap()
            .remove(0);
        conflict.payload["kind"] = json!("tampered");
        let probe = ContinuityEventInput {
            event_id: format!("evt_{}", "f".repeat(64)),
            project_id: identity.project_id.clone(),
            subject_id: None,
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

        let native = ContinuityEventInput {
            event_id: format!("evt_{}", "e".repeat(64)),
            project_id: identity.project_id.clone(),
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "native-continuity-probe".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 1_000,
            revision_head: None,
            revision_branch: None,
            payload: json!({"native":true}),
        };
        assert!(store.append_event(&native).unwrap().created);

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

        let erased = erase_session_memory(
            &project,
            &vault,
            &started.session.session_id,
            EraseSessionMemoryInput {
                expected_event_count: 2,
                expected_name: started.session.name.clone(),
            },
        )
        .unwrap();
        assert_eq!(
            erased.erased_learning_ids,
            vec![learning.learning.learning_id.clone()]
        );

        let after_erasure = import_legacy_continuity(&project, &vault, &store).unwrap();
        assert_eq!(after_erasure.session_events, 0);
        assert_eq!(after_erasure.learning_events, 0);
        assert_ne!(after_erasure.source_digest, first.source_digest);
        assert_ne!(after_erasure.manifest_event_id, first.manifest_event_id);
        assert_eq!(after_erasure.continuity_events_created, 1);
        assert_eq!(after_erasure.continuity_events_replayed, 0);
        assert_eq!(after_erasure.continuity_events_removed, 4);
        assert!(store
            .events_for_session(&first.project_id, &started.session.session_id)
            .unwrap()
            .is_empty());
        assert!(store
            .event(&first.project_id, &learning.event_id)
            .unwrap()
            .is_none());
        assert!(store
            .event(&first.project_id, &first.manifest_event_id)
            .unwrap()
            .is_none());
        assert!(store
            .event(&first.project_id, &after_erasure.manifest_event_id)
            .unwrap()
            .is_some());
        assert_eq!(
            store
                .event(&identity.project_id, &native.event_id)
                .unwrap()
                .unwrap()
                .kind,
            "native-continuity-probe"
        );
        assert!(read_session(&project, &vault, &started.session.session_id).is_err());
        assert!(read_learning(&project, &vault, &learning.learning.learning_id).is_err());
    }
}
