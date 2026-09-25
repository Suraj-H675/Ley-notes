use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

fn run_ley(config: &Path, arguments: &[&str], stdin_body: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .args(arguments)
        .stdin(if stdin_body.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(body) = stdin_body {
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(body.as_bytes())
            .unwrap();
    }
    child.wait_with_output().unwrap()
}

fn ley(config: &Path, arguments: &[&str]) -> Output {
    ley_input(config, arguments, None)
}

fn ley_stdin(config: &Path, arguments: &[&str], body: &str) -> Output {
    ley_input(config, arguments, Some(body))
}

fn ley_input(config: &Path, arguments: &[&str], stdin_body: Option<&str>) -> Output {
    let output = run_ley(config, arguments, stdin_body);
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
fn cli_consolidation_inbox_is_read_only_and_feeds_review_required_learning() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let project = base.path().join("project");
    let vault = base.path().join("vault");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&vault).unwrap();
    fs::write(project.join("README.md"), "# Consolidation CLI\n").unwrap();
    ley(
        &config,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Consolidation CLI",
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

    let active = json_stdout(ley(
        &config,
        &[
            "session",
            "start",
            project.to_str().unwrap(),
            "--name",
            "Active work",
            "--goal",
            "Remain outside meaningful-boundary consolidation",
            "--json",
        ],
    ));
    let active_id = active["session"]["sessionId"].as_str().unwrap();
    ley_stdin(
        &config,
        &[
            "session",
            "prompt",
            active_id,
            project.to_str().unwrap(),
            "--stdin",
            "--json",
        ],
        "ACTIVE_CONSOLIDATION_CLI_CANARY",
    );

    let terminal = json_stdout(ley(
        &config,
        &[
            "session",
            "start",
            project.to_str().unwrap(),
            "--name",
            "Completed work",
            "--goal",
            "Create a bounded local consolidation review boundary",
            "--json",
        ],
    ));
    let terminal_id = terminal["session"]["sessionId"].as_str().unwrap();
    ley_stdin(
        &config,
        &[
            "session",
            "prompt",
            terminal_id,
            project.to_str().unwrap(),
            "--stdin",
            "--json",
        ],
        "TERMINAL_CONSOLIDATION_PROMPT_CANARY",
    );
    ley_stdin(
        &config,
        &[
            "session",
            "response",
            terminal_id,
            project.to_str().unwrap(),
            "--stdin",
            "--json",
        ],
        "TERMINAL_CONSOLIDATION_RESPONSE_CANARY",
    );
    let turns = json_stdout(ley(
        &config,
        &[
            "session",
            "turns",
            terminal_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    let prompt_id = turns["turns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|turn| turn["kind"] == "user-prompt")
        .and_then(|turn| turn["recordId"].as_str())
        .unwrap()
        .to_owned();
    let response_id = turns["turns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|turn| turn["kind"] == "assistant-response")
        .and_then(|turn| turn["recordId"].as_str())
        .unwrap()
        .to_owned();
    let finished = json_stdout(ley(
        &config,
        &[
            "session",
            "finish",
            terminal_id,
            project.to_str().unwrap(),
            "--summary",
            "Completed without a final structured checkpoint",
            "--status",
            "completed",
            "--json",
        ],
    ));
    let finished_event_count = finished["session"]["eventCount"].as_u64().unwrap();

    let inbox = json_stdout(ley(
        &config,
        &[
            "consolidation",
            "inbox",
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(inbox["schemaVersion"], 2);
    assert_eq!(inbox["persisted"], false);
    assert_eq!(inbox["modelInvoked"], false);
    assert_eq!(inbox["backgroundWorkStarted"], false);
    assert_eq!(inbox["destructiveActionsTaken"], false);
    assert_eq!(inbox["coverage"]["excludedActiveSessions"], 1);
    assert_eq!(inbox["items"].as_array().unwrap().len(), 1);
    let item = &inbox["items"][0];
    assert_eq!(item["sessionId"], terminal_id);
    assert_eq!(item["sessionStatus"], "completed");
    assert_eq!(item["action"], "review-partial-evidence");
    assert_eq!(item["semanticFaithfulnessProven"], false);
    assert_eq!(item["automaticWriteAllowed"], false);
    let handles = item["proposalEvidenceRecordIds"].as_array().unwrap();
    assert_eq!(handles.len(), 2);
    assert!(handles
        .iter()
        .any(|value| value == &Value::String(prompt_id.clone())));
    assert!(handles
        .iter()
        .any(|value| value == &Value::String(response_id.clone())));
    let inbox_text = inbox.to_string();
    assert!(!inbox_text.contains("ACTIVE_CONSOLIDATION_CLI_CANARY"));
    assert!(!inbox_text.contains("TERMINAL_CONSOLIDATION_PROMPT_CANARY"));
    assert!(!inbox_text.contains("TERMINAL_CONSOLIDATION_RESPONSE_CANARY"));
    assert!(!inbox_text.contains(project.to_str().unwrap()));
    assert!(!inbox_text.contains(vault.to_str().unwrap()));

    let prompt_evidence = format!("{terminal_id}:{prompt_id}");
    let response_evidence = format!("{terminal_id}:{response_id}");
    let proposed = json_stdout(ley(
        &config,
        &[
            "learning",
            "propose",
            project.to_str().unwrap(),
            "--actor",
            "agent",
            "--provenance",
            "inferred",
            "--kind",
            "procedure",
            "--title",
            "Review the completed workflow",
            "--guidance",
            "Review the completed workflow before reusing it.",
            "--confidence",
            "60",
            "--evidence",
            &prompt_evidence,
            "--evidence",
            &response_evidence,
            "--json",
        ],
    ));
    assert_eq!(proposed["learning"]["state"], "tentative");
    assert_eq!(proposed["learning"]["trustState"], "review-required");
    assert_eq!(
        proposed["learning"]["originLineage"]["automaticAuthorityCeiling"],
        "review-required"
    );
    assert_eq!(
        proposed["learning"]["originLineage"]["sources"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(proposed["learning"]["originLineage"]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .all(|source| source["kind"] == "turn-evidence"));

    let after = json_stdout(ley(
        &config,
        &[
            "session",
            "show",
            terminal_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(after["eventCount"], finished_event_count);
    let review = json_stdout(ley(
        &config,
        &[
            "learning",
            "list",
            project.to_str().unwrap(),
            "--review",
            "--json",
        ],
    ));
    assert_eq!(review.as_array().unwrap().len(), 1);
}
