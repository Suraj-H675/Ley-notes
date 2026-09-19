use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

fn run_ley(config: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

fn ley(config: &Path, arguments: &[&str]) -> Output {
    let output = run_ley(config, arguments);
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
fn cli_imports_one_explicit_codex_message_history_snapshot_without_fabricating_transcript_data() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let project = base.path().join("project");
    let vault = base.path().join("vault");
    let history = base.path().join("history.jsonl");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&vault).unwrap();
    fs::write(project.join("README.md"), "# Historical import CLI\n").unwrap();
    ley(
        &config,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Historical import CLI",
            "--json",
        ],
    );
    ley(
        &config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            vault.to_str().unwrap(),
            "--json",
        ],
    );
    ley(&config, &["ingest", project.to_str().unwrap(), "--json"]);

    let selected = "11111111-1111-4111-8111-111111111111";
    let unrelated = "22222222-2222-4222-8222-222222222222";
    let secret = "HISTORICAL_IMPORT_SECRET_CANARY_8fe2";
    fs::write(
        &history,
        format!(
            "{{\"session_id\":\"{unrelated}\",\"ts\":1700000000,\"text\":\"UNRELATED_HISTORY_CANARY_4f31\"}}\n{{\"session_id\":\"{selected}\",\"ts\":1700000010,\"text\":\"Investigate historical cache api_key={secret}\"}}\n{{\"session_id\":\"{selected}\",\"ts\":1700000020,\"text\":\"Verify the import boundary\"}}\n"
        ),
    )
    .unwrap();

    let imported = json_stdout(ley(
        &config,
        &[
            "session",
            "import",
            "codex-history",
            project.to_str().unwrap(),
            "--source",
            history.to_str().unwrap(),
            "--host-session",
            selected,
            "--json",
        ],
    ));
    assert_eq!(imported["host"], "codex");
    assert_eq!(imported["sessionSourceKind"], "import");
    assert_eq!(imported["sourceKind"], "codex-message-history");
    assert_eq!(imported["matchedPrompts"], 2);
    assert_eq!(imported["capturedPrompts"], 2);
    assert_eq!(imported["assistantMessagesImported"], 0);
    assert_eq!(imported["sourceStartedAtUnixMs"], 1_700_000_010_000u64);
    assert_eq!(imported["sourceEndedAtUnixMs"], 1_700_000_020_000u64);
    assert_eq!(imported["sourcePathRetained"], false);
    assert_eq!(imported["rawHostSessionIdRetained"], false);
    assert_eq!(imported["liveSourceChecked"], false);
    assert_eq!(imported["authority"], "untrusted-historical-evidence");
    assert_eq!(imported["replayed"], false);
    let source_reference = imported["sourceReference"].as_str().unwrap();
    assert!(source_reference.starts_with("hsi_"));
    let session_id = imported["sessionId"].as_str().unwrap();
    assert!(session_id.starts_with("ses_"));

    let turns = json_stdout(ley(
        &config,
        &[
            "session",
            "turns",
            session_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(turns["schemaVersion"], 7);
    assert_eq!(turns["promptCount"], 2);
    assert_eq!(turns["responseCount"], 0);
    assert_eq!(turns["retainedTurnCount"], 2);
    assert_eq!(turns["liveSourceChecked"], false);
    let first = &turns["turns"][0];
    assert_eq!(first["origin"], "import");
    assert_eq!(first["host"], "codex");
    assert_eq!(first["sourceRecordedAtUnixMs"], 1_700_000_010_000u64);
    assert_eq!(first["sourceBoundary"], "untrusted-imported-host-history");
    assert!(first["text"]
        .as_str()
        .unwrap()
        .contains("Investigate historical cache"));
    let turns_json = turns.to_string();
    assert!(!turns_json.contains(secret));
    assert!(!turns_json.contains("UNRELATED_HISTORY_CANARY_4f31"));

    let retry = json_stdout(ley(
        &config,
        &[
            "session",
            "import",
            "codex-history",
            project.to_str().unwrap(),
            "--source",
            history.to_str().unwrap(),
            "--host-session",
            selected,
            "--json",
        ],
    ));
    assert_eq!(retry["replayed"], true);
    assert_eq!(retry["sessionId"], session_id);
    assert_eq!(retry["sourceReference"], source_reference);

    let vault_text = walk_text(&vault);
    for private in [
        history.to_str().unwrap(),
        selected,
        secret,
        "UNRELATED_HISTORY_CANARY_4f31",
    ] {
        assert!(!vault_text.contains(private));
    }

    let shown = json_stdout(ley(
        &config,
        &[
            "session",
            "show",
            session_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    let session_name = shown["name"].as_str().unwrap().to_owned();
    let event_count = shown["eventCount"].as_u64().unwrap().to_string();
    let external_history_before_erase = fs::read_to_string(&history).unwrap();
    let erased = json_stdout(ley(
        &config,
        &[
            "session",
            "erase",
            session_id,
            project.to_str().unwrap(),
            "--confirm-name",
            &session_name,
            "--expected-events",
            &event_count,
            "--json",
        ],
    ));
    assert_eq!(erased["sessionId"], session_id);
    assert_eq!(erased["sessionName"], session_name);
    assert_eq!(
        fs::read_to_string(&history).unwrap(),
        external_history_before_erase
    );
    let after_erase = run_ley(
        &config,
        &[
            "session",
            "show",
            session_id,
            project.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!after_erase.status.success());
    let vault_after_erase = walk_text(&vault);
    assert!(!vault_after_erase.contains("Investigate historical cache"));
    assert!(!vault_after_erase.contains("Verify the import boundary"));
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
