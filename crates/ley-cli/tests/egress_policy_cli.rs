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

fn init_and_bind(config: &Path, project: &Path, vault: &Path, name: &str) {
    fs::create_dir_all(project).unwrap();
    fs::create_dir_all(vault).unwrap();
    fs::write(project.join("README.md"), format!("# {name}\n")).unwrap();
    ley(
        config,
        &["init", project.to_str().unwrap(), "--name", name, "--json"],
    );
    ley(
        config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            vault.to_str().unwrap(),
            "--json",
        ],
    );
}

#[test]
fn cli_controls_private_project_and_mount_egress_without_path_leakage() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let active = base.path().join("active");
    let active_vault = base.path().join("active-vault");
    let reference = base.path().join("reference");
    let reference_vault = base.path().join("reference-vault");
    init_and_bind(&config, &active, &active_vault, "Active project");
    init_and_bind(&config, &reference, &reference_vault, "Reference project");

    let defaults = json_stdout(ley(
        &config,
        &["egress", "list", active.to_str().unwrap(), "--json"],
    ));
    assert_eq!(defaults["projectPolicy"], "agent-ok");
    assert!(defaults["specificationOverrides"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(defaults["mountOverrides"].as_array().unwrap().is_empty());

    let project_policy = json_stdout(ley(
        &config,
        &[
            "egress",
            "project",
            "local-model-only",
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(project_policy["scope"]["scopeKind"], "project");
    assert_eq!(project_policy["scope"]["policy"], "local-model-only");

    let mounted = json_stdout(ley(
        &config,
        &[
            "mount",
            "add",
            reference.to_str().unwrap(),
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    let mount_id = mounted["mount"]["mountId"].as_str().unwrap().to_owned();
    let mount_policy = json_stdout(ley(
        &config,
        &[
            "egress",
            "mount",
            &mount_id,
            "never-send",
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(mount_policy["scope"]["scopeKind"], "context-mount");
    assert_eq!(mount_policy["scope"]["scopeId"], mount_id);
    assert_eq!(mount_policy["scope"]["policy"], "never-send");

    let listed = json_stdout(ley(
        &config,
        &["egress", "list", active.to_str().unwrap(), "--json"],
    ));
    assert_eq!(listed["projectPolicy"], "local-model-only");
    assert_eq!(listed["mountOverrides"].as_array().unwrap().len(), 1);
    assert_eq!(listed["mountOverrides"][0]["scopeId"], mount_id);
    let serialized = listed.to_string();
    assert!(!serialized.contains(active.to_str().unwrap()));
    assert!(!serialized.contains(reference.to_str().unwrap()));
    assert!(!serialized.contains(active_vault.to_str().unwrap()));
    assert!(!serialized.contains(reference_vault.to_str().unwrap()));

    ley(
        &config,
        &[
            "mount",
            "remove",
            &mount_id,
            active.to_str().unwrap(),
            "--json",
        ],
    );
    let historical_reset = json_stdout(ley(
        &config,
        &[
            "egress",
            "mount",
            &mount_id,
            "agent-ok",
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(historical_reset["scope"]["policy"], "agent-ok");
    let after_historical_reset = json_stdout(ley(
        &config,
        &["egress", "list", active.to_str().unwrap(), "--json"],
    ));
    assert!(after_historical_reset["mountOverrides"]
        .as_array()
        .unwrap()
        .is_empty());

    let reset = json_stdout(ley(
        &config,
        &[
            "egress",
            "project",
            "agent-ok",
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(reset["scope"]["policy"], "agent-ok");

    let missing_mount = run_ley(
        &config,
        &[
            "egress",
            "mount",
            "mnt_11111111111111111111111111111111",
            "never-send",
            active.to_str().unwrap(),
        ],
    );
    assert!(!missing_mount.status.success());
    assert!(String::from_utf8_lossy(&missing_mount.stderr).contains("neither current/historical"));

    let invalid_target = run_ley(
        &config,
        &["mcp", active.to_str().unwrap(), "--egress-target", "remote"],
    );
    assert!(!invalid_target.status.success());
    assert!(String::from_utf8_lossy(&invalid_target.stderr).contains("cloud or local"));
}
