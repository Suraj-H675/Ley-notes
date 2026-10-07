use ley_core::{CaptureMode, ContinuityStore, APP_IDENTIFIER, CONTINUITY_DATABASE_FILE};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn run(config: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .env("XDG_DATA_HOME", config.join("data"))
        .env("XDG_CACHE_HOME", config.join("cache"))
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn json_output(config: &Path, cwd: &Path, args: &[&str]) -> Value {
    let output = run(config, cwd, args);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn hook(config: &Path, cwd: &Path, payload: Value) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .env("XDG_DATA_HOME", config.join("data"))
        .env("XDG_CACHE_HOME", config.join("cache"))
        .current_dir(cwd)
        .args(["hook", "--host", "codex"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(&payload).unwrap().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn real_cli_codex_hook_requires_capture_grant_and_persists_chronicle() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config");
    let root = temp.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("README.md"), "# M3 CLI hook\n").unwrap();

    let brain = json_output(
        &config,
        &root,
        &[
            "brain",
            "create",
            "--name",
            "M3 CLI",
            "--request",
            "req_m3_cli_create",
        ],
    );
    let project_id = brain["identity"]["projectId"].as_str().unwrap();
    let attached = json_output(
        &config,
        &root,
        &[
            "brain",
            "attach",
            "--project",
            project_id,
            "--root",
            root.to_str().unwrap(),
            "--request",
            "req_m3_cli_attach",
        ],
    );
    let locator_id = attached["locator"]["locatorId"].as_str().unwrap();

    let disabled = hook(
        &config,
        &root,
        json!({
            "hook_event_name": "SessionStart",
            "session_id": "cli-codex",
            "source": "startup"
        }),
    );
    assert!(disabled.status.success());
    assert_eq!(disabled.stdout, b"{}\n");
    let sessions = json_output(
        &config,
        &root,
        &["brain", "sessions", "--project", project_id],
    );
    assert_eq!(sessions["items"].as_array().unwrap().len(), 0);

    let store = ContinuityStore::at(config.join(APP_IDENTIFIER).join(CONTINUITY_DATABASE_FILE));
    let preview = store
        .preview_chronicle_capture(project_id, locator_id, CaptureMode::Structured)
        .unwrap();
    store.authorize_chronicle_capture(&preview).unwrap();

    for payload in [
        json!({
            "hook_event_name": "SessionStart",
            "session_id": "cli-codex",
            "source": "startup",
            "transcript_path": "/secret/raw-transcript.jsonl"
        }),
        json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "cli-codex",
            "turn_id": "turn-1",
            "prompt": "Investigate api_key=CLI_SECRET_MUST_BE_REDACTED"
        }),
        json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "cli-codex",
            "turn_id": "turn-1",
            "prompt": "Investigate api_key=CLI_SECRET_MUST_BE_REDACTED"
        }),
        json!({
            "hook_event_name": "SessionEnd",
            "session_id": "cli-codex",
            "reason": "exit"
        }),
    ] {
        let output = hook(&config, &root, payload);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"{}\n");
    }

    let sessions = json_output(
        &config,
        &root,
        &["brain", "sessions", "--project", project_id],
    );
    let item = &sessions["items"][0];
    let session_id = item["sessionId"].as_str().unwrap();
    assert_eq!(item["state"], "finished");
    assert_eq!(item["episodeCount"], 3);

    let history = json_output(
        &config,
        &root,
        &[
            "brain",
            "history",
            "--project",
            project_id,
            "--session",
            session_id,
        ],
    );
    let items = history["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["kind"], "session-start");
    assert_eq!(items[1]["kind"], "user-prompt");
    assert_eq!(items[2]["kind"], "session-end");
    let retained = serde_json::to_string(&history).unwrap();
    assert!(!retained.contains("CLI_SECRET_MUST_BE_REDACTED"));
    assert!(!retained.contains("raw-transcript.jsonl"));
    assert!(retained.contains("[REDACTED:"));
}
