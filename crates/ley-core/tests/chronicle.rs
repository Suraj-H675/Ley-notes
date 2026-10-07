use ley_core::{
    preflight_codex_chronicle_hook, process_codex_chronicle_hook, CaptureMode,
    ChronicleHookPreflight, ChronicleHookResult, ContinuityEventInput, ContinuityStore,
    CreateProjectBrainInput, EpisodeEvidenceBasis, EpisodeKind, ProjectHandle, ProjectSessionState,
    SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct Fixture {
    _scratch: TempDir,
    root: PathBuf,
    store: ContinuityStore,
    project_id: String,
    locator_id: String,
}

impl Fixture {
    fn new() -> Self {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path().join("project");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("README.md"), "# Chronicle fixture\n").unwrap();
        let store = ContinuityStore::at(scratch.path().join("private/continuity.sqlite3"));
        let brain = store
            .create_project_brain(&CreateProjectBrainInput {
                name: "Chronicle fixture".into(),
                request_id: "req_chronicle_create".into(),
            })
            .unwrap();
        let (_, locator) = store
            .authorize_working_copy(
                &ProjectHandle {
                    project_id: brain.identity.project_id.clone(),
                    generation: brain.generation,
                },
                &root,
                "req_chronicle_attach",
            )
            .unwrap();
        Self {
            _scratch: scratch,
            root,
            store,
            project_id: brain.identity.project_id,
            locator_id: locator.locator_id,
        }
    }

    fn authorize(&self, mode: CaptureMode) -> ley_core::ChronicleCaptureGrant {
        let preview = self
            .store
            .preview_chronicle_capture(&self.project_id, &self.locator_id, mode)
            .unwrap();
        self.store.authorize_chronicle_capture(&preview).unwrap()
    }

    fn hook(&self, payload: Value) -> ChronicleHookResult {
        process_codex_chronicle_hook(&self.root, payload, &self.store).unwrap()
    }
}

fn recorded(result: ChronicleHookResult) -> (String, String, EpisodeKind, bool) {
    match result {
        ChronicleHookResult::Recorded {
            session_id,
            event_id,
            episode_kind,
            replayed,
            ..
        } => (session_id, event_id, episode_kind, replayed),
        other => panic!("expected recorded hook, got {other:?}"),
    }
}

fn start_payload(session: &str) -> Value {
    json!({
        "hook_event_name": "SessionStart",
        "session_id": session,
        "source": "startup",
        "transcript_path": "/private/transcript-MUST-NOT-BE-READ.jsonl"
    })
}

#[test]
fn attachment_is_not_capture_permission_and_cannot_fall_through() {
    let fixture = Fixture::new();
    assert!(matches!(
        preflight_codex_chronicle_hook(&fixture.root, &fixture.store).unwrap(),
        ChronicleHookPreflight::CaptureDisabled { .. }
    ));
    assert!(matches!(
        fixture.hook(start_payload("codex-disabled")),
        ChronicleHookResult::CaptureDisabled { .. }
    ));
    assert!(fixture
        .store
        .project_sessions(&fixture.project_id)
        .unwrap()
        .is_empty());
}

#[test]
fn codex_hooks_persist_ordered_observed_history_with_replay_redaction_and_gaps() {
    let fixture = Fixture::new();
    fixture.authorize(CaptureMode::Structured);
    assert_eq!(
        preflight_codex_chronicle_hook(&fixture.root, &fixture.store).unwrap(),
        ChronicleHookPreflight::CaptureEnabled
    );

    let start = fixture.hook(start_payload("codex-main"));
    let (session_id, start_event, start_kind, replayed) = recorded(start);
    assert_eq!(start_kind, EpisodeKind::SessionStart);
    assert!(!replayed);
    let replay = recorded(fixture.hook(start_payload("codex-main")));
    assert_eq!(replay.1, start_event);
    assert!(replay.3);

    let prompt = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "codex-main",
        "turn_id": "turn-1",
        "prompt": "Fix the watcher with api_key=NEVER_PERSIST_PROMPT"
    });
    let prompt_event = recorded(fixture.hook(prompt.clone()));
    assert_eq!(prompt_event.2, EpisodeKind::UserPrompt);
    assert!(!prompt_event.3);
    assert!(recorded(fixture.hook(prompt.clone())).3);
    let conflicting = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "codex-main",
        "turn_id": "turn-1",
        "prompt": "Different visible prompt"
    });
    let error = process_codex_chronicle_hook(&fixture.root, conflicting, &fixture.store)
        .unwrap_err()
        .to_string();
    assert!(error.contains("reused with different observable content"));

    let tool = recorded(fixture.hook(json!({
        "hook_event_name": "PostToolUse",
        "session_id": "codex-main",
        "turn_id": "turn-1",
        "tool_use_id": "tool-1",
        "tool_name": "Bash",
        "tool_input": {"command": "printf api_key=NEVER_PERSIST_COMMAND"},
        "tool_response": {"output": "token=NEVER_PERSIST_RESULT", "exit_code": 0}
    })));
    assert_eq!(tool.2, EpisodeKind::ToolResult);

    let response = recorded(fixture.hook(json!({
        "hook_event_name": "Stop",
        "session_id": "codex-main",
        "turn_id": "turn-1",
        "last_assistant_message": "Watcher fixed; token=NEVER_PERSIST_RESPONSE"
    })));
    assert_eq!(response.2, EpisodeKind::AgentResponse);

    let ended = recorded(fixture.hook(json!({
        "hook_event_name": "SessionEnd",
        "session_id": "codex-main",
        "reason": "exit"
    })));
    assert_eq!(ended.2, EpisodeKind::SessionEnd);

    let sessions = fixture
        .store
        .chronicle_sessions(&fixture.project_id, 0, 10)
        .unwrap();
    assert_eq!(sessions.items.len(), 1);
    assert_eq!(sessions.items[0].session.session_id, session_id);
    assert_eq!(
        sessions.items[0].session.state,
        ProjectSessionState::Finished
    );
    assert_eq!(sessions.items[0].episode_count, 5);
    assert!(!sessions.items[0].gaps.is_empty());

    let history = fixture
        .store
        .chronicle_history(&fixture.project_id, Some(&session_id), 0, 100)
        .unwrap();
    assert_eq!(
        history
            .items
            .iter()
            .map(|item| item.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    assert_eq!(
        history
            .items
            .iter()
            .map(|item| item.kind)
            .collect::<Vec<_>>(),
        vec![
            EpisodeKind::SessionStart,
            EpisodeKind::UserPrompt,
            EpisodeKind::ToolResult,
            EpisodeKind::AgentResponse,
            EpisodeKind::SessionEnd,
        ]
    );
    assert!(history
        .items
        .iter()
        .all(|item| item.evidence_basis == EpisodeEvidenceBasis::Observed));
    let serialized = serde_json::to_string(&history).unwrap();
    for secret in [
        "NEVER_PERSIST_PROMPT",
        "NEVER_PERSIST_COMMAND",
        "NEVER_PERSIST_RESULT",
        "NEVER_PERSIST_RESPONSE",
        "transcript-MUST-NOT-BE-READ",
    ] {
        assert!(!serialized.contains(secret), "retained secret {secret}");
    }
    assert!(serialized.contains("[REDACTED:"));
    assert!(history.items[2]
        .gaps
        .iter()
        .any(|gap| gap.contains("does not treat it as proof")));
}

#[test]
fn missing_stable_turn_identity_preserves_distinct_gaps_and_dedupes_exact_payload_retry() {
    let fixture = Fixture::new();
    fixture.authorize(CaptureMode::Structured);
    let (session_id, _, _, _) = recorded(fixture.hook(start_payload("codex-missing-id")));
    let first = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "codex-missing-id",
        "prompt": "first missing turn identity"
    });
    let second = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "codex-missing-id",
        "prompt": "second missing turn identity"
    });
    let first_event = recorded(fixture.hook(first.clone()));
    assert_eq!(first_event.2, EpisodeKind::Gap);
    assert!(recorded(fixture.hook(first)).3);
    let second_event = recorded(fixture.hook(second));
    assert_eq!(second_event.2, EpisodeKind::Gap);
    assert_ne!(first_event.1, second_event.1);

    let history = fixture
        .store
        .chronicle_history(&fixture.project_id, Some(&session_id), 0, 10)
        .unwrap();
    assert_eq!(history.items.len(), 3);
    assert!(history.items[1].gaps[0].contains("only byte-equivalent payload retries"));
}

#[test]
fn legacy_structured_checkpoint_remains_reported_and_incomplete() {
    let fixture = Fixture::new();
    fixture
        .store
        .append_event(&ContinuityEventInput {
            project_id: fixture.project_id.clone(),
            event_id: "evt_legacy_checkpoint_m3".into(),
            subject_id: None,
            session_id: Some("legacy-session".into()),
            session_sequence: Some(1),
            request_id: None,
            request_fingerprint: None,
            kind: "checkpoint-recorded".into(),
            payload_version: 1,
            recorded_at_unix_ms: 1_000,
            revision_head: None,
            revision_branch: None,
            payload: json!({"data": {"summary": "agent reported tests passed"}}),
        })
        .unwrap();

    let history = fixture
        .store
        .chronicle_history(&fixture.project_id, Some("legacy-session"), 0, 10)
        .unwrap();
    assert_eq!(history.items.len(), 1);
    assert_eq!(history.items[0].kind, EpisodeKind::StructuredSubmission);
    assert_eq!(
        history.items[0].evidence_basis,
        EpisodeEvidenceBasis::Reported
    );
    assert!(history.items[0]
        .gaps
        .iter()
        .any(|gap| gap.contains("complete observable chronology is unavailable")));
}

#[test]
fn minimal_and_bounded_capture_omit_or_truncate_bodies_truthfully() {
    let minimal = Fixture::new();
    minimal.authorize(CaptureMode::Minimal);
    let (session_id, _, _, _) = recorded(minimal.hook(start_payload("codex-minimal")));
    recorded(minimal.hook(json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "codex-minimal",
        "turn_id": "turn-minimal",
        "prompt": "visible but intentionally not retained"
    })));
    let history = minimal
        .store
        .chronicle_history(&minimal.project_id, Some(&session_id), 0, 10)
        .unwrap();
    assert_eq!(history.items[1].payload["observation"]["text"], Value::Null);
    assert_eq!(
        history.items[1].payload["observation"]["retention"],
        "metadata-only"
    );

    let bounded = Fixture::new();
    bounded.authorize(CaptureMode::Structured);
    let (bounded_session, _, _, _) = recorded(bounded.hook(start_payload("codex-bounded")));
    let oversized = "x".repeat(SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS + 500);
    recorded(bounded.hook(json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "codex-bounded",
        "turn_id": "turn-bounded",
        "prompt": oversized
    })));
    let history = bounded
        .store
        .chronicle_history(&bounded.project_id, Some(&bounded_session), 0, 10)
        .unwrap();
    let retained = history.items[1].payload["observation"]["text"]
        .as_str()
        .unwrap();
    assert!(retained.chars().count() <= SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS);
    assert_eq!(history.items[1].payload["observation"]["truncated"], true);
}

#[test]
fn session_erasure_and_capture_revocation_prevent_resurrection() {
    let fixture = Fixture::new();
    let grant = fixture.authorize(CaptureMode::Structured);
    let (session_id, _, _, _) = recorded(fixture.hook(start_payload("codex-erased")));
    recorded(fixture.hook(json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "codex-erased",
        "turn_id": "turn-before-erase",
        "prompt": "retained before erase"
    })));

    let brain = fixture
        .store
        .open_project_brain(&fixture.project_id)
        .unwrap();
    fixture
        .store
        .erase_project_session(
            &ProjectHandle {
                project_id: fixture.project_id.clone(),
                generation: brain.generation,
            },
            &session_id,
        )
        .unwrap();
    let sessions = fixture
        .store
        .chronicle_sessions(&fixture.project_id, 0, 10)
        .unwrap();
    assert_eq!(sessions.items[0].session.state, ProjectSessionState::Erased);
    assert_eq!(sessions.items[0].episode_count, 0);
    assert!(fixture
        .store
        .chronicle_history(&fixture.project_id, Some(&session_id), 0, 10)
        .is_err());
    assert!(matches!(
        fixture.hook(json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "codex-erased",
            "turn_id": "turn-after-erase",
            "prompt": "must not resurrect"
        })),
        ChronicleHookResult::CaptureDisabled { .. }
    ));

    let state = fixture
        .store
        .revoke_chronicle_capture(&fixture.project_id, &fixture.locator_id, &grant.grant_id)
        .unwrap();
    assert!(!state.authorized);
    assert!(!state.effective);
    assert!(matches!(
        preflight_codex_chronicle_hook(&fixture.root, &fixture.store).unwrap(),
        ChronicleHookPreflight::CaptureDisabled { .. }
    ));
}

#[test]
fn parallel_codex_sessions_keep_independent_sequence_spaces() {
    let fixture = Fixture::new();
    fixture.authorize(CaptureMode::Structured);
    let (first, _, _, _) = recorded(fixture.hook(start_payload("codex-a")));
    let (second, _, _, _) = recorded(fixture.hook(start_payload("codex-b")));
    recorded(fixture.hook(json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "codex-b",
        "turn_id": "b-1",
        "prompt": "second session"
    })));
    recorded(fixture.hook(json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "codex-a",
        "turn_id": "a-1",
        "prompt": "first session"
    })));

    for session in [first, second] {
        let history = fixture
            .store
            .chronicle_history(&fixture.project_id, Some(&session), 0, 10)
            .unwrap();
        assert_eq!(
            history
                .items
                .iter()
                .map(|item| item.sequence)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }
}

#[test]
fn replacing_the_authorized_directory_makes_the_grant_visibly_ineffective() {
    let fixture = Fixture::new();
    fixture.authorize(CaptureMode::Structured);
    let displaced = fixture._scratch.path().join("displaced-project");
    std::fs::rename(&fixture.root, &displaced).unwrap();
    std::fs::create_dir(&fixture.root).unwrap();

    let state = fixture
        .store
        .chronicle_capture_state(&fixture.project_id, &fixture.locator_id)
        .unwrap()
        .unwrap();
    assert!(state.authorized);
    assert!(!state.effective);
    assert_eq!(
        state.inactive_reason.as_deref(),
        Some("working-copy-directory-replaced")
    );
    assert!(matches!(
        preflight_codex_chronicle_hook(&fixture.root, &fixture.store).unwrap(),
        ChronicleHookPreflight::CaptureDisabled { .. }
    ));
}

#[test]
fn moved_working_copy_invalidates_old_capture_grant() {
    let fixture = Fixture::new();
    fixture.authorize(CaptureMode::Structured);
    let moved = fixture._scratch.path().join("moved-project");
    std::fs::rename(&fixture.root, &moved).unwrap();
    let brain = fixture
        .store
        .open_project_brain(&fixture.project_id)
        .unwrap();
    fixture
        .store
        .relocate_working_copy(
            &ProjectHandle {
                project_id: fixture.project_id.clone(),
                generation: brain.generation,
            },
            &fixture.locator_id,
            &moved,
            "req_chronicle_move",
        )
        .unwrap();
    assert!(matches!(
        preflight_codex_chronicle_hook(&moved, &fixture.store).unwrap(),
        ChronicleHookPreflight::CaptureDisabled { .. }
    ));
}

#[allow(dead_code)]
fn _assert_path(_: &Path) {}
