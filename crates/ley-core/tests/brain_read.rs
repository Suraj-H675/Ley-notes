use ley_core::{
    brain_read_brief, brain_read_evidence, brain_read_search, resolve_brain_workspace,
    AgentEgressPolicy, AgentEgressTarget, BrainEvidenceReference, BrainReadBinding,
    BrainWorkspaceResolution, CaptureMode, ContinuityStore, CreateProjectBrainInput,
    LocalImportInput, ProjectHandle, SourceVersionInput,
};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    store: ContinuityStore,
    handle: ProjectHandle,
    root: Option<PathBuf>,
    locator_id: Option<String>,
}

impl Fixture {
    fn source_only() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let store = ContinuityStore::at(temp.path().join("private/continuity.sqlite3"));
        let brain = store
            .create_project_brain(&CreateProjectBrainInput {
                name: "Source only fixture".into(),
                request_id: "req_brain_read_source_only".into(),
            })
            .unwrap();
        store
            .set_project_egress_policy(&brain.identity.project_id, AgentEgressPolicy::AgentOk)
            .unwrap();
        Self {
            _temp: temp,
            store,
            handle: ProjectHandle {
                project_id: brain.identity.project_id,
                generation: brain.generation,
            },
            root: None,
            locator_id: None,
        }
    }

    fn attached() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir(&root).unwrap();
        let store = ContinuityStore::at(temp.path().join("private/continuity.sqlite3"));
        let brain = store
            .create_project_brain(&CreateProjectBrainInput {
                name: "Attached fixture".into(),
                request_id: "req_brain_read_attached".into(),
            })
            .unwrap();
        store
            .set_project_egress_policy(&brain.identity.project_id, AgentEgressPolicy::AgentOk)
            .unwrap();
        let handle = ProjectHandle {
            project_id: brain.identity.project_id,
            generation: brain.generation,
        };
        let (_, locator) = store
            .authorize_working_copy(&handle, &root, "req_brain_read_attach")
            .unwrap();
        Self {
            _temp: temp,
            store,
            handle,
            root: Some(root),
            locator_id: Some(locator.locator_id),
        }
    }

    fn source(&self, display_name: &str, bytes: &[u8], suffix: &str) -> (String, String) {
        let source = self
            .store
            .create_project_source(
                &self.handle,
                "document",
                display_name,
                &format!("req_brain_source_{suffix}"),
            )
            .unwrap();
        let version = self
            .store
            .retain_source_version(
                &self.handle,
                &source.source_id,
                &SourceVersionInput {
                    request_id: format!("req_brain_version_{suffix}"),
                    representation_kind: "utf8-text".into(),
                    original_content_hash: None,
                    source_bytes: bytes.len() as u64,
                    transformation: json!({"kind":"identity"}),
                    retained_bytes: bytes.to_vec(),
                },
            )
            .unwrap();
        (source.source_id, version.source_version_id)
    }

    fn binding(&self) -> BrainReadBinding {
        BrainReadBinding::for_project_id(
            self.store.clone(),
            &self.handle.project_id,
            AgentEgressTarget::Local,
        )
        .unwrap()
    }
}

fn push_episode(
    fixture: &Fixture,
    session_id: &str,
    sequence: u64,
    kind: &str,
    observation: Value,
) -> String {
    fixture
        .store
        .record_project_episode(
            &fixture.handle,
            Some(session_id),
            Some(sequence),
            &format!("req_brain_episode_{sequence}_{kind}"),
            kind,
            None,
            observation,
        )
        .unwrap()
        .event_id
}

#[test]
fn source_only_brain_has_no_invented_path_and_historical_citations_revalidate_identity() {
    let fixture = Fixture::source_only();
    let (source_id, old_version_id) = fixture.source(
        "Specification.md",
        b"Requirement: stop using Redis because the service is unavailable.",
        "old",
    );
    let current = fixture
        .store
        .retain_source_version(
            &fixture.handle,
            &source_id,
            &SourceVersionInput {
                request_id: "req_brain_version_current".into(),
                representation_kind: "utf8-text".into(),
                original_content_hash: None,
                source_bytes: 24,
                transformation: json!({"kind":"identity"}),
                retained_bytes: b"Current local SQLite requirement".to_vec(),
            },
        )
        .unwrap();
    let unrelated = fixture
        .store
        .create_project_brain(&CreateProjectBrainInput {
            name: "Unrelated fixture".into(),
            request_id: "req_brain_read_other_project".into(),
        })
        .unwrap();
    let unrelated_handle = ProjectHandle {
        project_id: unrelated.identity.project_id.clone(),
        generation: unrelated.generation,
    };
    fixture
        .store
        .set_project_egress_policy(&unrelated.identity.project_id, AgentEgressPolicy::AgentOk)
        .unwrap();
    let unrelated_source = fixture
        .store
        .create_project_source(
            &unrelated_handle,
            "document",
            "unrelated.md",
            "req_brain_other_source",
        )
        .unwrap();
    fixture
        .store
        .retain_source_version(
            &unrelated_handle,
            &unrelated_source.source_id,
            &SourceVersionInput {
                request_id: "req_brain_other_version".into(),
                representation_kind: "utf8-text".into(),
                original_content_hash: None,
                source_bytes: 26,
                transformation: json!({"kind":"identity"}),
                retained_bytes: b"UNRELATED_PROJECT_SENTINEL".to_vec(),
            },
        )
        .unwrap();
    let binding = fixture.binding();
    assert!(brain_read_search(&binding, "sentinel", None, 8)
        .unwrap()
        .items
        .is_empty());
    let search = brain_read_search(&binding, "Redis", None, 8).unwrap();
    let old = search
        .items
        .iter()
        .find(|item| matches!(&item.reference, BrainEvidenceReference::SourceVersion { source_version_id, .. } if source_version_id == &old_version_id))
        .unwrap();
    assert_eq!(old.source.as_ref().unwrap().is_latest_version, false);
    let current_item = brain_read_search(&binding, "SQLite", None, 8)
        .unwrap()
        .items
        .into_iter()
        .find(|item| matches!(&item.reference, BrainEvidenceReference::SourceVersion { source_version_id, .. } if source_version_id == &current.source_version_id))
        .unwrap();
    assert_eq!(
        current_item.source.as_ref().unwrap().live_source_state,
        "unknown-no-authorized-repository-locator"
    );
    let citation = old.reference.clone();
    let evidence = brain_read_evidence(&binding, &citation).unwrap();
    assert_eq!(evidence.content, old.content);
    assert!(
        matches!(&citation, BrainEvidenceReference::SourceVersion { content_hash, .. } if content_hash == &evidence.content_hash)
    );
    assert!(matches!(
        brain_read_search(&binding, "SQLite", None, 8)
            .unwrap()
            .items
            .iter()
            .find(|item| matches!(&item.reference, BrainEvidenceReference::SourceVersion { source_version_id, .. } if source_version_id == &current.source_version_id))
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .is_latest_version,
        true
    ));

    let wrong_project = BrainEvidenceReference::SourceVersion {
        project_id: "prj_00000000000000000000000000000000".into(),
        source_id,
        source_version_id: old_version_id,
        content_hash: match &citation {
            BrainEvidenceReference::SourceVersion { content_hash, .. } => content_hash.clone(),
            _ => unreachable!(),
        },
        start_byte: 0,
        end_byte: 5,
    };
    assert!(brain_read_evidence(&binding, &wrong_project).is_err());
    let invalid_range = match citation {
        BrainEvidenceReference::SourceVersion {
            project_id,
            source_id,
            source_version_id,
            content_hash,
            ..
        } => BrainEvidenceReference::SourceVersion {
            project_id,
            source_id,
            source_version_id,
            content_hash,
            start_byte: 2,
            end_byte: 1,
        },
        _ => unreachable!(),
    };
    assert!(brain_read_evidence(&binding, &invalid_range).is_err());
}

#[test]
fn latest_source_version_follows_the_latest_retained_occurrence_when_content_returns() {
    let fixture = Fixture::source_only();
    let (source_id, version_a_id) = fixture.source("queue.md", b"Alpha queue decision", "a-first");
    let version_b = fixture
        .store
        .retain_source_version(
            &fixture.handle,
            &source_id,
            &SourceVersionInput {
                request_id: "req_brain_version_b".into(),
                representation_kind: "utf8-text".into(),
                original_content_hash: None,
                source_bytes: 19,
                transformation: json!({"kind":"identity"}),
                retained_bytes: b"Beta queue revision".to_vec(),
            },
        )
        .unwrap();
    let version_a_again = fixture
        .store
        .retain_source_version(
            &fixture.handle,
            &source_id,
            &SourceVersionInput {
                request_id: "req_brain_version_a-again".into(),
                representation_kind: "utf8-text".into(),
                original_content_hash: None,
                source_bytes: 20,
                transformation: json!({"kind":"identity"}),
                retained_bytes: b"Alpha queue decision".to_vec(),
            },
        )
        .unwrap();
    assert_eq!(version_a_again.source_version_id, version_a_id);

    let connection = rusqlite::Connection::open(fixture.store.path()).unwrap();
    connection
        .execute(
            "UPDATE source_versions SET retained_at_unix_ms=1000
             WHERE project_id=?1 AND source_id=?2 AND source_version_id=?3",
            rusqlite::params![fixture.handle.project_id, source_id, version_a_id],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE source_versions SET retained_at_unix_ms=2000
             WHERE project_id=?1 AND source_id=?2 AND source_version_id=?3",
            rusqlite::params![
                fixture.handle.project_id,
                source_id,
                version_b.source_version_id
            ],
        )
        .unwrap();

    let binding = fixture.binding();
    let result = brain_read_search(&binding, "Alpha Beta", None, 8).unwrap();
    let version_a = result
        .items
        .iter()
        .find(|item| {
            matches!(&item.reference, BrainEvidenceReference::SourceVersion { source_version_id, .. } if source_version_id == &version_a_id)
        })
        .unwrap();
    let version_b_item = result
        .items
        .iter()
        .find(|item| {
            matches!(&item.reference, BrainEvidenceReference::SourceVersion { source_version_id, .. } if source_version_id == &version_b.source_version_id)
        })
        .unwrap();

    assert!(version_a.source.as_ref().unwrap().is_latest_version);
    assert!(!version_b_item.source.as_ref().unwrap().is_latest_version);
    assert_eq!(
        version_a
            .source
            .as_ref()
            .unwrap()
            .occurrence_event_id
            .as_deref(),
        Some(version_a_again.occurrence_event_id.as_str())
    );
}

#[test]
fn orientation_preserves_the_chronological_failure_cause_solution_verification_and_source() {
    let fixture = Fixture::attached();
    let root = fixture.root.as_ref().unwrap();
    let source_text = "Requirement: local-only continuity. Decision: choose SQLite because no service is required. Approach A failed because Redis is absent. Root cause: external daemon dependency. Solution: implement local SQLite. Verification: cargo test passed. Unresolved: encrypted backup sync.";
    fs::write(
        root.join("README.md"),
        format!("{source_text}\npassword=fixture-secret\n"),
    )
    .unwrap();
    let receipt = fixture
        .store
        .import_local_project(
            &fixture.handle,
            &LocalImportInput {
                locator_id: fixture.locator_id.clone().unwrap(),
                selected_root: root.clone(),
                request_id: "req_brain_history_import".into(),
            },
        )
        .unwrap();
    assert_eq!(receipt.retained_count, 1);

    let capture_preview = fixture
        .store
        .preview_chronicle_capture(
            &fixture.handle.project_id,
            fixture.locator_id.as_deref().unwrap(),
            CaptureMode::Structured,
        )
        .unwrap();
    fixture
        .store
        .authorize_chronicle_capture(&capture_preview)
        .unwrap();
    let start = ley_core::process_codex_chronicle_hook(
        root,
        json!({
            "hook_event_name":"SessionStart",
            "session_id":"history-session",
            "source":"startup"
        }),
        &fixture.store,
    )
    .unwrap();
    let _ = start;
    ley_core::process_codex_chronicle_hook(
        root,
        json!({
            "hook_event_name":"UserPromptSubmit",
            "session_id":"history-session",
            "turn_id":"turn-1",
            "prompt":"Requirement: move the project off Redis. It must run locally because fresh machines have no Redis daemon."
        }),
        &fixture.store,
    )
    .unwrap();
    ley_core::process_codex_chronicle_hook(
        root,
        json!({
            "hook_event_name":"Stop",
            "session_id":"history-session",
            "turn_id":"turn-1",
            "last_assistant_message":"Decision: use SQLite because the Brain is local-first and must not depend on a service."
        }),
        &fixture.store,
    )
    .unwrap();
    ley_core::process_codex_chronicle_hook(
        root,
        json!({
            "hook_event_name":"PostToolUse",
            "session_id":"history-session",
            "turn_id":"turn-2",
            "tool_use_id":"redis-test",
            "tool_name":"Bash",
            "tool_input":{"command":"cargo test"},
            "tool_response":{"exit_code":1,"output":"Approach A failed: Redis daemon unavailable"}
        }),
        &fixture.store,
    )
    .unwrap();
    ley_core::process_codex_chronicle_hook(
        root,
        json!({
            "hook_event_name":"Stop",
            "session_id":"history-session",
            "turn_id":"turn-2",
            "last_assistant_message":"Root cause: approach A required an external Redis daemon. Solution: approach B uses embedded SQLite. Unresolved: encrypted backup sync still needs a design."
        }),
        &fixture.store,
    )
    .unwrap();
    ley_core::process_codex_chronicle_hook(
        root,
        json!({
            "hook_event_name":"PostToolUse",
            "session_id":"history-session",
            "turn_id":"turn-3",
            "tool_use_id":"sqlite-test",
            "tool_name":"Bash",
            "tool_input":{"command":"cargo test --locked --offline"},
            "tool_response":{"exit_code":0,"output":"Verification: cargo test passed"}
        }),
        &fixture.store,
    )
    .unwrap();

    let resolution =
        resolve_brain_workspace(fixture.store.clone(), root, AgentEgressTarget::Local).unwrap();
    let BrainWorkspaceResolution::Bound(binding) = resolution else {
        panic!("attached Brain workspace must resolve to its exact project");
    };
    let brief = brain_read_brief(&binding, None, 8).unwrap();
    let excerpts = brief
        .items
        .iter()
        .map(|item| item.content.to_ascii_lowercase())
        .collect::<Vec<_>>();
    for expected in [
        "requirement",
        "decision",
        "failed",
        "root cause",
        "solution",
        "verification",
        "unresolved",
    ] {
        assert!(
            excerpts.iter().any(|excerpt| excerpt.contains(expected)),
            "orientation lost {expected}: {excerpts:?}"
        );
    }
    let failure_tool = brief.items.iter().find(|item| {
        item.episode
            .as_ref()
            .is_some_and(|episode| episode.episode_kind == "tool-result")
            && item.content.contains("Approach A failed")
    });
    assert!(
        failure_tool.is_some(),
        "source wording must not replace the tool failure receipt"
    );
    assert!(brief.items.iter().any(|item| {
        item.source.as_ref().is_some_and(|source| {
            source.transformation["kind"] == "secret-redacted-utf8"
                && source.live_source_state == "original-bytes-match-at-attached-locator"
                && source.applicability.contains("may be transformed")
        })
    }));
    assert!(brief.items.iter().all(|item| {
        item.category_inference.contains("heuristic")
            && item.category_inference.contains("not a canonical claim")
    }));
    assert!(brief.items.iter().any(|item| {
        item.episode
            .as_ref()
            .is_some_and(|episode| episode.semantic_basis == "reported")
    }));
    assert!(brief.items.iter().any(|item| {
        item.episode.as_ref().is_some_and(|episode| {
            episode.evidence_basis == "observed" && episode.semantic_basis == "unknown"
        })
    }));

    let redis = brain_read_search(&binding, "why did we stop using Redis?", None, 8).unwrap();
    assert!(redis
        .items
        .iter()
        .any(|item| item.content.to_ascii_lowercase().contains("redis")));
    assert!(redis.items.len() <= 8);
    for item in brief.items.iter().filter(|item| item.source.is_some()) {
        let evidence = brain_read_evidence(&binding, &item.reference).unwrap();
        assert_eq!(evidence.content, item.content);
        assert!(matches!(
            (&item.reference, evidence.content_hash.as_str()),
            (BrainEvidenceReference::SourceVersion { content_hash, .. }, returned)
                if content_hash == returned
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let outside = root.parent().unwrap().join("outside-source.md");
        fs::write(&outside, "outside content must not be followed").unwrap();
        fs::remove_file(root.join("README.md")).unwrap();
        symlink(&outside, root.join("README.md")).unwrap();
        let after_replacement = brain_read_search(&binding, "local-only", None, 8).unwrap();
        assert!(after_replacement.items.iter().any(|item| {
            item.source
                .as_ref()
                .is_some_and(|source| source.live_source_state == "unknown-live-source-unavailable")
        }));
    }
}

#[test]
fn evidence_rejects_forged_session_hash_utf8_range_and_erased_episode() {
    let fixture = Fixture::source_only();
    let brain = fixture
        .store
        .open_project_brain(&fixture.handle.project_id)
        .unwrap();
    let handle = ProjectHandle {
        project_id: brain.identity.project_id,
        generation: brain.generation,
    };
    fixture
        .store
        .start_project_session(&handle, "exact-session", Some("codex"), Some("external-1"))
        .unwrap();
    let event_id = push_episode(
        &fixture,
        "exact-session",
        1,
        "agent-response",
        json!({"text":"Decision: Café storage is local."}),
    );
    let binding = fixture.binding();
    let result = brain_read_search(&binding, "Café", None, 8).unwrap();
    let item = result
        .items
        .iter()
        .find(|item| item.episode.is_some())
        .unwrap();
    let citation = item.reference.clone();
    let wrong_session = match citation.clone() {
        BrainEvidenceReference::Episode {
            project_id,
            event_id,
            content_hash,
            start_byte,
            end_byte,
            ..
        } => BrainEvidenceReference::Episode {
            project_id,
            event_id,
            session_id: Some("forged-session".into()),
            content_hash,
            start_byte,
            end_byte,
        },
        _ => unreachable!(),
    };
    assert!(brain_read_evidence(&binding, &wrong_session).is_err());
    let wrong_hash = match citation.clone() {
        BrainEvidenceReference::Episode {
            project_id,
            event_id,
            session_id,
            start_byte,
            end_byte,
            ..
        } => BrainEvidenceReference::Episode {
            project_id,
            event_id,
            session_id,
            content_hash: "sha256:forged".into(),
            start_byte,
            end_byte,
        },
        _ => unreachable!(),
    };
    assert!(brain_read_evidence(&binding, &wrong_hash).is_err());
    let split_character = item.content.find('é').unwrap() + 1;
    let bad_utf8_range = match citation.clone() {
        BrainEvidenceReference::Episode {
            project_id,
            event_id,
            session_id,
            content_hash,
            start_byte,
            ..
        } => BrainEvidenceReference::Episode {
            project_id,
            event_id,
            session_id,
            content_hash,
            start_byte: start_byte + split_character,
            end_byte: start_byte + split_character + 1,
        },
        _ => unreachable!(),
    };
    assert!(brain_read_evidence(&binding, &bad_utf8_range).is_err());
    assert!(brain_read_evidence(&binding, &citation).is_ok());

    fixture
        .store
        .erase_project_session(&handle, "exact-session")
        .unwrap();
    let current_binding = BrainReadBinding::for_project_id(
        fixture.store.clone(),
        &handle.project_id,
        AgentEgressTarget::Local,
    )
    .unwrap();
    assert!(brain_read_evidence(&current_binding, &citation).is_err());
    assert!(brain_read_search(&current_binding, "Café", None, 8)
        .unwrap()
        .items
        .iter()
        .all(|item| item
            .episode
            .as_ref()
            .is_none_or(|episode| episode.event_id != event_id)));
}

#[test]
fn locator_replacement_revocation_copy_and_overlap_fail_closed() {
    let fixture = Fixture::attached();
    let root = fixture.root.as_ref().unwrap().clone();
    let resolution =
        resolve_brain_workspace(fixture.store.clone(), &root, AgentEgressTarget::Local).unwrap();
    let BrainWorkspaceResolution::Bound(binding) = resolution else {
        panic!("expected exact locator binding");
    };
    fs::rename(&root, root.with_extension("replaced")).unwrap();
    fs::create_dir(&root).unwrap();
    assert!(brain_read_brief(&binding, None, 8).is_err());
    assert!(
        resolve_brain_workspace(fixture.store.clone(), &root, AgentEgressTarget::Local).is_err()
    );

    let second = Fixture::attached();
    let second_root = second.root.as_ref().unwrap().clone();
    let copied = second_root.with_extension("copy");
    fs::create_dir(&copied).unwrap();
    fs::create_dir(copied.join(".ley")).unwrap();
    let identity = second
        .store
        .open_project_brain(&second.handle.project_id)
        .unwrap()
        .identity;
    fs::write(
        copied.join(".ley/project.json"),
        serde_json::to_vec(&identity).unwrap(),
    )
    .unwrap();
    assert!(
        resolve_brain_workspace(second.store.clone(), &copied, AgentEgressTarget::Local).is_err()
    );

    let parent = second_root.join("nested");
    fs::create_dir(&parent).unwrap();
    let brain = second
        .store
        .create_project_brain(&CreateProjectBrainInput {
            name: "Overlapping fixture".into(),
            request_id: "req_brain_read_overlap".into(),
        })
        .unwrap();
    let nested_handle = ProjectHandle {
        project_id: brain.identity.project_id,
        generation: brain.generation,
    };
    second
        .store
        .authorize_working_copy(&nested_handle, &parent, "req_brain_read_nested")
        .unwrap();
    assert!(
        resolve_brain_workspace(second.store.clone(), &parent, AgentEgressTarget::Local).is_err()
    );
}

#[test]
fn egress_is_checked_again_for_each_read_and_retrieval_limits_are_enforced() {
    let fixture = Fixture::source_only();
    fixture.source("notes.md", b"local note", "egress");
    let binding = fixture.binding();
    fixture
        .store
        .set_project_egress_policy(&fixture.handle.project_id, AgentEgressPolicy::NeverSend)
        .unwrap();
    assert!(brain_read_brief(&binding, None, 8).is_err());
    fixture
        .store
        .set_project_egress_policy(&fixture.handle.project_id, AgentEgressPolicy::AgentOk)
        .unwrap();
    assert!(brain_read_brief(&binding, None, 8).is_ok());
    fixture
        .store
        .set_project_egress_policy(&fixture.handle.project_id, AgentEgressPolicy::NeverSend)
        .unwrap();
    assert!(brain_read_brief(&binding, None, 8).is_err());
    assert!(brain_read_search(&binding, "term", None, 13).is_err());
    assert!(brain_read_search(&binding, &"x".repeat(257), None, 8).is_err());
}

#[test]
fn oversized_chronicle_payload_is_disclosed_without_loading_it_into_the_index() {
    let fixture = Fixture::source_only();
    fixture
        .store
        .start_project_session(
            &fixture.handle,
            "bounded-session",
            Some("codex"),
            Some("bounded-session-external"),
        )
        .unwrap();
    push_episode(
        &fixture,
        "bounded-session",
        1,
        "tool-result",
        json!({"output": format!("oversized-marker {}", "x".repeat(40_000))}),
    );

    let result = brain_read_search(&fixture.binding(), "oversized-marker", None, 8).unwrap();
    assert!(result.items.is_empty());
    assert_eq!(result.omissions.omitted_records_for_byte_limit, 1);
    assert!(result.omissions.omitted_bytes > 32_000);
}

#[test]
fn attached_locator_revocation_invalidates_a_bound_reader() {
    let fixture = Fixture::attached();
    let binding = match resolve_brain_workspace(
        fixture.store.clone(),
        fixture.root.as_ref().unwrap(),
        AgentEgressTarget::Local,
    )
    .unwrap()
    {
        BrainWorkspaceResolution::Bound(binding) => binding,
        BrainWorkspaceResolution::NotAssociated => panic!("expected attached locator"),
    };
    assert!(brain_read_brief(&binding, None, 8).is_ok());
    fixture
        .store
        .revoke_working_copy(
            &fixture.handle,
            fixture.locator_id.as_deref().unwrap(),
            "req_brain_read_revoke",
        )
        .unwrap();
    assert!(brain_read_brief(&binding, None, 8).is_err());
}

#[test]
fn source_erasure_invalidates_historical_evidence() {
    let fixture = Fixture::source_only();
    let (source_id, version_id) = fixture.source("secret.md", b"erase this source", "erase");
    let binding = fixture.binding();
    let item = brain_read_search(&binding, "source", None, 8)
        .unwrap()
        .items
        .into_iter()
        .next()
        .unwrap();
    fixture
        .store
        .erase_project_source(&fixture.handle, &source_id)
        .unwrap();
    let current = fixture
        .store
        .open_project_brain(&fixture.handle.project_id)
        .unwrap();
    let current_binding = BrainReadBinding::for_project_id(
        fixture.store.clone(),
        &current.identity.project_id,
        AgentEgressTarget::Local,
    )
    .unwrap();
    assert!(brain_read_evidence(&current_binding, &item.reference).is_err());
    assert!(brain_read_search(&current_binding, "erase", None, 8)
        .unwrap()
        .items
        .is_empty());
    let erased = BrainEvidenceReference::SourceVersion {
        project_id: current.identity.project_id,
        source_id,
        source_version_id: version_id,
        content_hash: match item.reference {
            BrainEvidenceReference::SourceVersion { content_hash, .. } => content_hash,
            _ => unreachable!(),
        },
        start_byte: 0,
        end_byte: 5,
    };
    assert!(brain_read_evidence(&current_binding, &erased).is_err());
}
