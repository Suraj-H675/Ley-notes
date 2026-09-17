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
fn cli_mounts_and_unmounts_one_explicit_reference_without_path_leakage() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let active = base.path().join("active");
    let active_vault = base.path().join("active-vault");
    let reference = base.path().join("reference");
    let reference_vault = base.path().join("reference-vault");
    let unrelated = base.path().join("unrelated");
    let unrelated_vault = base.path().join("unrelated-vault");
    init_and_bind(&config, &active, &active_vault, "Active project");
    init_and_bind(&config, &reference, &reference_vault, "Reference project");
    init_and_bind(&config, &unrelated, &unrelated_vault, "Unrelated project");

    let created = json_stdout(ley(
        &config,
        &[
            "mount",
            "add",
            reference.to_str().unwrap(),
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(created["created"], true);
    assert_eq!(created["mount"]["permission"], "read-only");
    assert_eq!(created["mount"]["agentContextEnabled"], true);
    assert_eq!(created["mount"]["status"], "ready");
    assert_eq!(created["mount"]["sourceProjectName"], "Reference project");
    let mount_id = created["mount"]["mountId"].as_str().unwrap().to_owned();

    let duplicate = json_stdout(ley(
        &config,
        &[
            "mount",
            "add",
            reference.to_str().unwrap(),
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(duplicate["created"], false);
    assert_eq!(duplicate["mount"]["mountId"], mount_id);
    let listed = json_stdout(ley(
        &config,
        &["mount", "list", active.to_str().unwrap(), "--json"],
    ));
    assert_eq!(listed["ready"], 1);
    assert_eq!(listed["unavailable"], 0);
    assert_eq!(listed["agentContextEnabled"], 1);
    assert_eq!(listed["mounts"].as_array().unwrap().len(), 1);
    assert_eq!(listed["mounts"][0]["mountId"], mount_id);
    assert_eq!(
        listed["mounts"][0]["sourceProjectName"],
        "Reference project"
    );
    let serialized = serde_json::to_string(&listed).unwrap();
    assert!(!serialized.contains(active.to_str().unwrap()));
    assert!(!serialized.contains(reference.to_str().unwrap()));
    assert!(!serialized.contains(unrelated.to_str().unwrap()));
    assert!(!serialized.contains("Unrelated project"));

    let unrelated_list = json_stdout(ley(
        &config,
        &["mount", "list", unrelated.to_str().unwrap(), "--json"],
    ));
    assert!(unrelated_list["mounts"].as_array().unwrap().is_empty());
    let removed = json_stdout(ley(
        &config,
        &[
            "mount",
            "remove",
            &mount_id,
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(removed["mountId"], mount_id);

    let empty = json_stdout(ley(
        &config,
        &["mount", "list", active.to_str().unwrap(), "--json"],
    ));
    assert_eq!(empty["ready"], 0);
    assert!(empty["mounts"].as_array().unwrap().is_empty());

    let second_remove = json_stdout(ley(
        &config,
        &[
            "mount",
            "remove",
            &mount_id,
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert!(second_remove.is_null());
}
