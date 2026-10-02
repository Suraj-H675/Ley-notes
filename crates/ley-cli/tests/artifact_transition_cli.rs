mod common;

use common::seed_legacy_project;
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

fn initialize_bound_project(config: &Path, project: &Path, vault: &Path, name: &str) {
    fs::create_dir_all(project).unwrap();
    fs::write(project.join("README.md"), format!("# {name}\nfirst body\n")).unwrap();
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
}

#[test]
fn artifact_ingest_uses_native_authority_after_vault_loss_and_fails_closed_before_cutover() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let project = base.path().join("project");
    let vault = base.path().join("vault");
    initialize_bound_project(&config, &project, &vault, "Native artifact ingest");

    let first = json_stdout(ley(
        &config,
        &["ingest", project.to_str().unwrap(), "--json"],
    ));
    let first_snapshot = first["ingestion"]["snapshotId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(first["ingestion"]["manifestPath"].is_null());
    assert!(first["ingestion"]["graphPath"].is_null());

    fs::remove_dir_all(&vault).unwrap();
    fs::write(
        project.join("README.md"),
        "# Native artifact ingest\nsecond body after vault loss\n",
    )
    .unwrap();
    let second = json_stdout(ley(
        &config,
        &["ingest", project.to_str().unwrap(), "--json"],
    ));
    assert_ne!(second["ingestion"]["snapshotId"], first_snapshot);
    assert!(second["ingestion"]["manifestPath"].is_null());
    assert!(second["ingestion"]["graphPath"].is_null());
    assert_eq!(second["ingestion"]["changed"], true);

    let pending_project = base.path().join("pending-project");
    let pending_vault = base.path().join("pending-vault");
    initialize_bound_project(
        &config,
        &pending_project,
        &pending_vault,
        "Pending artifact cutover",
    );
    fs::remove_dir_all(&pending_vault).unwrap();
    let unavailable = run_ley(
        &config,
        &["ingest", pending_project.to_str().unwrap(), "--json"],
    );
    assert!(!unavailable.status.success());
    assert!(String::from_utf8_lossy(&unavailable.stderr).contains("bound Ley vault is unavailable"));
}
