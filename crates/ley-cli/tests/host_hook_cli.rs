use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

fn ley(config: &Path, arguments: &[&str], input: Option<&Value>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ley"));
    command
        .env("XDG_CONFIG_HOME", config)
        .args(arguments)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(serde_json::to_string(input).unwrap().as_bytes())
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "ley {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn json_stdout(output: Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn installed_hook_contract_survives_retry_and_carries_context_to_another_host() {
    let base = tempdir().unwrap();
    let project = base.path().join("project");
    let other = base.path().join("ordinary-project");
    let vault = base.path().join("vault");
    let config = base.path().join("config");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&other).unwrap();
    fs::create_dir(&vault).unwrap();
    fs::write(project.join("README.md"), "# Actual project\n").unwrap();

    assert_eq!(
        json_stdout(ley(
            &config,
            &["hook", "--host", "codex", other.to_str().unwrap()],
            Some(&json!({
                "session_id": "outside-ley",
                "cwd": other,
                "hook_event_name": "SessionStart",
                "source": "startup"
            }))
        )),
        json!({})
    );
    assert!(!other.join(".ley").exists());

    ley(
        &config,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "CLI integration",
        ],
        None,
    );
    ley(
        &config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            vault.to_str().unwrap(),
        ],
        None,
    );
    ley(&config, &["ingest", project.to_str().unwrap()], None);

    let transcript_path = base.path().join("private-transcript.jsonl");
    fs::write(&transcript_path, "NEVER_PERSIST_THIS_TRANSCRIPT").unwrap();
    let start = json_stdout(ley(
        &config,
        &["hook", "--host", "codex", project.to_str().unwrap()],
        Some(&json!({
            "session_id": "codex-thread",
            "transcript_path": transcript_path,
            "cwd": project,
            "hook_event_name": "SessionStart",
            "source": "startup",
            "model": "gpt-current"
        })),
    ));
    let context = start["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.contains("CLI integration"));
    assert!(context.contains("Live source checked: no"));

    let stop = json!({
        "session_id": "codex-thread",
        "transcript_path": transcript_path,
        "cwd": project,
        "hook_event_name": "Stop",
        "turn_id": "turn-one",
        "last_assistant_message": "The filesystem watcher works. Conflict recovery remains."
    });
    for _ in 0..2 {
        assert_eq!(
            json_stdout(ley(
                &config,
                &["hook", "--host", "codex", project.to_str().unwrap()],
                Some(&stop)
            )),
            json!({})
        );
    }

    let sessions = json_stdout(ley(
        &config,
        &["session", "list", project.to_str().unwrap(), "--json"],
        None,
    ));
    assert_eq!(sessions.as_array().unwrap().len(), 1);
    assert_eq!(sessions[0]["checkpoints"], 1);

    let next = json_stdout(ley(
        &config,
        &["hook", "--host", "claude", project.to_str().unwrap()],
        Some(&json!({
            "session_id": "claude-thread",
            "transcript_path": "/not/read/claude.jsonl",
            "cwd": "/forged/other/project",
            "hook_event_name": "SessionStart",
            "source": "startup"
        })),
    ));
    let next_context = next["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(next_context.contains("Conflict recovery remains"));

    let vault_text = walk_text(&vault);
    assert!(!vault_text.contains("NEVER_PERSIST_THIS_TRANSCRIPT"));
    assert!(!vault_text.contains("private-transcript.jsonl"));
    assert!(!vault_text.contains("/forged/other/project"));
}

fn walk_text(directory: &Path) -> String {
    let mut text = String::new();
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            text.push_str(&walk_text(&path));
        } else if let Ok(body) = fs::read_to_string(path) {
            text.push_str(&body);
        }
    }
    text
}
