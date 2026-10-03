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
fn fresh_cli_project_stays_native_without_a_vault_binding() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let project = base.path().join("project");
    let legacy_vault = base.path().join("legacy-vault");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("README.md"), "# Native CLI project\n").unwrap();

    ley(
        &config,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Native CLI project",
        ],
    );

    let missing_vault = base.path().join("missing-vault");
    let failed_bind = run_ley(
        &config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            missing_vault.to_str().unwrap(),
        ],
    );
    assert!(!failed_bind.status.success());

    fs::create_dir(&legacy_vault).unwrap();
    let failed_override_ingest = run_ley(
        &config,
        &[
            "ingest",
            project.to_str().unwrap(),
            "--vault",
            legacy_vault.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!failed_override_ingest.status.success());
    assert!(String::from_utf8_lossy(&failed_override_ingest.stderr)
        .contains("explicit legacy vault overrides are compatibility-only"));
    assert!(fs::read_dir(&legacy_vault).unwrap().next().is_none());

    let ingested = json_stdout(ley(
        &config,
        &["ingest", project.to_str().unwrap(), "--json"],
    ));
    assert!(ingested["binding"].is_null());
    assert_eq!(ingested["storage"], "native-continuity");
    assert!(ingested["ingestion"]["manifestPath"].is_null());
    assert!(ingested["ingestion"]["graphPath"].is_null());
    let ingestion = ingested["ingestion"].as_object().unwrap();
    for field in [
        "graphSnapshotId",
        "graphChanged",
        "graphNodes",
        "graphEdges",
    ] {
        assert!(!ingestion.contains_key(field), "unexpected {field}");
    }

    let started = json_stdout(ley(
        &config,
        &[
            "session",
            "start",
            project.to_str().unwrap(),
            "--name",
            "Native CLI session",
            "--goal",
            "Prove CLI continuity needs no vault",
            "--json",
        ],
    ));
    assert_eq!(started["storage"], "native-continuity");
    assert_eq!(started["session"]["eventCount"], 1);
    let session_id = started["session"]["sessionId"].as_str().unwrap();
    let checkpointed = json_stdout(ley(
        &config,
        &[
            "session",
            "checkpoint",
            session_id,
            project.to_str().unwrap(),
            "--summary",
            "Capture current native CLI evidence",
            "--touched",
            "README.md",
            "--json",
        ],
    ));
    assert_eq!(checkpointed["session"]["eventCount"], 2);
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
    let checkpoint_id = shown["checkpoints"][0]["checkpointId"].as_str().unwrap();
    assert!(shown["checkpoints"][0]["projectRevision"]
        .as_object()
        .is_some_and(|revision| !revision.contains_key("graphSnapshotId")));
    let evidence = format!("{session_id}:{checkpoint_id}");

    let proposed = json_stdout(ley(
        &config,
        &[
            "learning",
            "propose",
            project.to_str().unwrap(),
            "--actor",
            "user",
            "--kind",
            "convention",
            "--title",
            "Native CLI convention",
            "--guidance",
            "Keep continuity native",
            "--confidence",
            "95",
            "--provenance",
            "user-authored",
            "--evidence",
            &evidence,
            "--json",
        ],
    ));
    assert_eq!(proposed["learning"]["title"], "Native CLI convention");
    assert_eq!(proposed["replayed"], false);

    let learnings = json_stdout(ley(
        &config,
        &["learning", "list", project.to_str().unwrap(), "--json"],
    ));
    assert_eq!(learnings.as_array().unwrap().len(), 1);
    assert_eq!(learnings[0]["title"], "Native CLI convention");
    let learning_id = proposed["learning"]["learningId"].as_str().unwrap();

    let reviewed = json_stdout(ley(
        &config,
        &[
            "learning",
            "review",
            learning_id,
            project.to_str().unwrap(),
            "--actor",
            "user",
            "--action",
            "confirm",
            "--note",
            "Confirmed for native CLI coverage",
            "--json",
        ],
    ));
    assert_eq!(reviewed["learning"]["state"], "verified");

    let search = json_stdout(ley(
        &config,
        &[
            "search",
            "Native CLI project",
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(search["projectName"], "Native CLI project");
    assert!(!search["results"].as_array().unwrap().is_empty());

    let resume = json_stdout(ley(
        &config,
        &["resume", project.to_str().unwrap(), "--json"],
    ));
    assert_eq!(resume["projectName"], "Native CLI project");
    assert_eq!(resume["totalSessions"], 1);
    assert_eq!(resume["totalCurrentTrustedLearnings"], 1);
    assert_eq!(resume["learnings"][0]["learningId"], learning_id);

    let rejected = run_ley(
        &config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            legacy_vault.to_str().unwrap(),
        ],
    );
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains(
        "cannot bind a legacy vault to a project already registered for native continuity"
    ));
}
