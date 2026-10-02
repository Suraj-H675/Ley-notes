mod common;

use common::seed_legacy_project;
use serde_json::Value;
use std::fs;
use std::io::Write;
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

fn run_ley_with_stdin(config: &Path, arguments: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
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

fn initialize_bound_project(config: &Path, project: &Path, vault: &Path, name: &str) {
    seed_legacy_project(project, vault, name);
    ley(
        config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            vault.to_str().unwrap(),
        ],
    );
    ley(config, &["ingest", project.to_str().unwrap()]);
}

#[test]
fn session_continuity_uses_proven_native_authority_after_vault_loss() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let project = base.path().join("project");
    let vault = base.path().join("vault");
    initialize_bound_project(&config, &project, &vault, "Native session reads");

    let started = json_stdout(ley(
        &config,
        &[
            "session",
            "start",
            project.to_str().unwrap(),
            "--name",
            "Imported session",
            "--goal",
            "Read bounded session history after the legacy vault disappears",
            "--json",
        ],
    ));
    let session_id = started["session"]["sessionId"].as_str().unwrap().to_owned();
    let prompt = run_ley_with_stdin(
        &config,
        &[
            "session",
            "prompt",
            &session_id,
            project.to_str().unwrap(),
            "--stdin",
            "--json",
        ],
        "Remember the bounded transition evidence.",
    );
    assert!(
        prompt.status.success(),
        "session prompt failed: {}",
        String::from_utf8_lossy(&prompt.stderr)
    );

    let imported = json_stdout(ley(
        &config,
        &["session", "list", project.to_str().unwrap(), "--json"],
    ));
    assert_eq!(imported.as_array().unwrap().len(), 1);
    assert_eq!(imported[0]["sessionId"], session_id);

    fs::remove_dir_all(&vault).unwrap();

    let checkpointed = json_stdout(ley(
        &config,
        &[
            "session",
            "checkpoint",
            &session_id,
            project.to_str().unwrap(),
            "--summary",
            "Continue writing through native session authority after vault loss",
            "--touched",
            "README.md",
            "--json",
        ],
    ));
    assert_eq!(checkpointed["session"]["eventCount"], 3);
    assert_eq!(checkpointed["replayed"], false);

    let renamed = json_stdout(ley(
        &config,
        &[
            "session",
            "rename",
            &session_id,
            project.to_str().unwrap(),
            "--name",
            "Native session after vault loss",
            "--note",
            "The legacy vault is gone but native authority is proven",
            "--expected-events",
            "3",
            "--json",
        ],
    ));
    assert_eq!(renamed["session"]["eventCount"], 4);

    let prompt_after_loss = run_ley_with_stdin(
        &config,
        &[
            "session",
            "prompt",
            &session_id,
            project.to_str().unwrap(),
            "--stdin",
            "--json",
        ],
        "Continue after the legacy vault is gone.",
    );
    assert!(
        prompt_after_loss.status.success(),
        "post-loss prompt failed: {}",
        String::from_utf8_lossy(&prompt_after_loss.stderr)
    );
    assert_eq!(json_stdout(prompt_after_loss)["session"]["eventCount"], 5);

    let response_after_loss = run_ley_with_stdin(
        &config,
        &[
            "session",
            "response",
            &session_id,
            project.to_str().unwrap(),
            "--stdin",
            "--json",
        ],
        "Native continuity is still writable.",
    );
    assert!(
        response_after_loss.status.success(),
        "post-loss response failed: {}",
        String::from_utf8_lossy(&response_after_loss.stderr)
    );
    assert_eq!(json_stdout(response_after_loss)["session"]["eventCount"], 6);

    let finished = json_stdout(ley(
        &config,
        &[
            "session",
            "finish",
            &session_id,
            project.to_str().unwrap(),
            "--summary",
            "Native session lifecycle completed after vault loss",
            "--json",
        ],
    ));
    assert_eq!(finished["session"]["eventCount"], 7);
    assert_eq!(finished["session"]["status"], "completed");

    let second = json_stdout(ley(
        &config,
        &[
            "session",
            "start",
            project.to_str().unwrap(),
            "--name",
            "Started after vault loss",
            "--goal",
            "Prove a new native session can start without the legacy vault",
            "--json",
        ],
    ));
    let second_session_id = second["session"]["sessionId"].as_str().unwrap().to_owned();
    assert_eq!(second["session"]["eventCount"], 1);

    let listed = json_stdout(ley(
        &config,
        &["session", "list", project.to_str().unwrap(), "--json"],
    ));
    let listed = listed.as_array().unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().any(|item| item["sessionId"] == session_id));
    assert!(listed
        .iter()
        .any(|item| item["sessionId"] == second_session_id));

    let turns = json_stdout(ley(
        &config,
        &[
            "session",
            "turns",
            &session_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(turns["sessionId"], session_id);
    assert_eq!(turns["promptCount"], 2);
    assert_eq!(turns["responseCount"], 1);
    assert_eq!(
        turns["turns"][0]["text"],
        "Remember the bounded transition evidence."
    );
    assert_eq!(
        turns["turns"][1]["text"],
        "Continue after the legacy vault is gone."
    );
    assert_eq!(
        turns["turns"][2]["text"],
        "Native continuity is still writable."
    );

    let shown = json_stdout(ley(
        &config,
        &[
            "session",
            "show",
            &session_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(shown["sessionId"], session_id);
    assert_eq!(
        shown["goal"],
        "Read bounded session history after the legacy vault disappears"
    );
    assert_eq!(shown["name"], "Native session after vault loss");
    assert_eq!(shown["status"], "completed");
    assert_eq!(shown["promptCount"], 2);
    assert_eq!(shown["responseCount"], 1);
    assert_eq!(shown["checkpointCount"], 1);
    assert_eq!(
        shown["checkpoints"][0]["touchedArtifacts"][0]["artifactPath"],
        "README.md"
    );
    assert!(
        shown["checkpoints"][0]["touchedArtifacts"][0]["artifactSnapshotId"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(
        shown["checkpoints"][0]["projectRevision"]["graphSnapshotId"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(shown["revisionFreshness"]["capturedHead"].is_null());

    let never_imported_project = base.path().join("never-imported-project");
    let never_imported_vault = base.path().join("never-imported-vault");
    initialize_bound_project(
        &config,
        &never_imported_project,
        &never_imported_vault,
        "Never imported",
    );
    let absent_session_id = format!("ses_{}", "f".repeat(32));
    fs::remove_dir_all(&never_imported_vault).unwrap();

    let unavailable = run_ley(
        &config,
        &[
            "session",
            "list",
            never_imported_project.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!unavailable.status.success());
    assert!(String::from_utf8_lossy(&unavailable.stderr).contains("bound Ley vault is unavailable"));

    let unavailable_start = run_ley(
        &config,
        &[
            "session",
            "start",
            never_imported_project.to_str().unwrap(),
            "--name",
            "Must not start",
            "--goal",
            "Do not create native session authority from an unavailable legacy source",
            "--json",
        ],
    );
    assert!(!unavailable_start.status.success());
    assert!(String::from_utf8_lossy(&unavailable_start.stderr)
        .contains("bound Ley vault is unavailable"));

    let unavailable_show = run_ley(
        &config,
        &[
            "session",
            "show",
            &absent_session_id,
            never_imported_project.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!unavailable_show.status.success());
    assert!(String::from_utf8_lossy(&unavailable_show.stderr)
        .contains("bound Ley vault is unavailable"));
}
