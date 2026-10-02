mod common;

use common::seed_legacy_project;
use ley_core::{
    diagnose_project, AgentEgressPolicy, EgressPolicyRegistry, SpecificationRegistry,
    APP_IDENTIFIER, CONTEXT_MOUNT_REGISTRY_FILE, EGRESS_POLICY_REGISTRY_FILE,
    SPECIFICATION_REGISTRY_FILE,
};
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
fn cli_uses_project_egress_and_cleans_legacy_overrides_without_path_leakage() {
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

    for arguments in [
        vec![
            "egress",
            "specification",
            "spec_11111111111111111111111111111111",
            "never-send",
            active.to_str().unwrap(),
        ],
        vec![
            "egress",
            "mount",
            "mnt_22222222222222222222222222222222",
            "local-model-only",
            active.to_str().unwrap(),
        ],
        vec![
            "egress",
            "connector",
            "ext_33333333333333333333333333333333",
            "confirm-per-use",
            active.to_str().unwrap(),
        ],
    ] {
        let rejected = run_ley(&config, &arguments);
        assert!(!rejected.status.success());
        let stderr = String::from_utf8_lossy(&rejected.stderr);
        assert!(stderr.contains("legacy compatibility state"));
        assert!(stderr.contains("only agent-ok is accepted"));
        assert!(stderr.contains("ley egress project"));
    }

    let mount_id = "mnt_22222222222222222222222222222222";
    seed_legacy_mount(&config, &active, &reference, mount_id);
    let legacy_registry = EgressPolicyRegistry::at(
        config
            .join(APP_IDENTIFIER)
            .join(EGRESS_POLICY_REGISTRY_FILE),
    );
    let specification_registry = SpecificationRegistry::at(
        config
            .join(APP_IDENTIFIER)
            .join(SPECIFICATION_REGISTRY_FILE),
    );
    fs::create_dir_all(active_vault.join("Specs")).unwrap();
    fs::write(
        active_vault.join("Specs/Legacy.md"),
        "# Legacy approved source\n\nCLI cleanup marker.\n",
    )
    .unwrap();
    let specification_id = "spec_44444444444444444444444444444444";
    specification_registry
        .approve(&active, &active_vault, specification_id, "Specs/Legacy.md")
        .unwrap();
    legacy_registry
        .set_specification_policy(&active, specification_id, AgentEgressPolicy::NeverSend)
        .unwrap();
    let specification_reset = json_stdout(ley(
        &config,
        &[
            "egress",
            "specification",
            specification_id,
            "agent-ok",
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(specification_reset["scope"]["policy"], "agent-ok");
    fs::remove_file(specification_registry.path()).unwrap();
    fs::remove_dir_all(active_vault.join("Specs")).unwrap();
    let migrated_source_cleanup = json_stdout(ley(
        &config,
        &[
            "egress",
            "specification",
            specification_id,
            "agent-ok",
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(migrated_source_cleanup["scope"]["policy"], "agent-ok");

    legacy_registry
        .set_mount_policy(&active, &mount_id, AgentEgressPolicy::NeverSend)
        .unwrap();

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
            "agent-ok",
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
