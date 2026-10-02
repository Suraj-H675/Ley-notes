mod common;

use common::seed_legacy_project;
use ley_core::{diagnose_project, APP_IDENTIFIER, CONTEXT_MOUNT_REGISTRY_FILE};
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
    seed_legacy_project(project, vault, name);
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

fn seed_legacy_mount(config: &Path, active: &Path, reference: &Path, mount_id: &str) {
    let active_id = diagnose_project(active).unwrap().identity.project_id;
    let reference_id = diagnose_project(reference).unwrap().identity.project_id;
    let path = config
        .join(APP_IDENTIFIER)
        .join(CONTEXT_MOUNT_REGISTRY_FILE);
    let document = serde_json::json!({
        "schemaVersion": 3,
        "mounts": {
            active_id.clone(): {
                mount_id: {
                    "sourceProjectId": reference_id.clone(),
                    "createdAtUnixMs": 1_700_000_000_000_u64,
                    "agentContextEnabled": true,
                }
            }
        },
        "agentMountHistory": {
            active_id: { mount_id: reference_id }
        }
    });
    fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

#[test]
fn cli_retires_mount_creation_but_lists_and_removes_legacy_mount_without_path_leakage() {
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

    let rejected = run_ley(
        &config,
        &[
            "mount",
            "add",
            reference.to_str().unwrap(),
            active.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!rejected.status.success());
    let stderr = String::from_utf8_lossy(&rejected.stderr);
    assert!(stderr.contains("unknown mount command 'add'"));
    assert!(stderr.contains("use list or remove"));

    let empty_before = json_stdout(ley(
        &config,
        &["mount", "list", active.to_str().unwrap(), "--json"],
    ));
    assert!(empty_before["mounts"].as_array().unwrap().is_empty());

    let mount_id = "mnt_11111111111111111111111111111111";
    seed_legacy_mount(&config, &active, &reference, mount_id);

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
