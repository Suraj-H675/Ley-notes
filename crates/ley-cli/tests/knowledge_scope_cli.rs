use ley_core::{diagnose_project, APP_IDENTIFIER, KNOWLEDGE_SCOPE_REGISTRY_FILE};
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

fn seed_legacy_scope(config: &Path, active: &Path, sources: &[&Path], scope_id: &str, name: &str) {
    let active_id = diagnose_project(active).unwrap().identity.project_id;
    let source_ids = sources
        .iter()
        .map(|source| diagnose_project(source).unwrap().identity.project_id)
        .collect::<Vec<_>>();
    let path = config
        .join(APP_IDENTIFIER)
        .join(KNOWLEDGE_SCOPE_REGISTRY_FILE);
    let document = serde_json::json!({
        "schemaVersion": 1,
        "scopes": {
            scope_id: {
                "kind": "team",
                "name": name,
                "sourceProjectIds": source_ids.clone(),
                "createdAtUnixMs": 1_700_000_000_000_u64,
            }
        },
        "attachments": {
            active_id.clone(): {
                scope_id: 1_700_000_000_100_u64,
            }
        },
        "attachmentHistory": {
            active_id: {
                scope_id: source_ids,
            }
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
fn cli_retires_scope_growth_but_lists_and_detaches_legacy_scope_without_path_leakage() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let active = base.path().join("active");
    let active_vault = base.path().join("active-vault");
    let platform = base.path().join("platform");
    let platform_vault = base.path().join("platform-vault");
    let security = base.path().join("security");
    let security_vault = base.path().join("security-vault");
    let unrelated = base.path().join("unrelated");
    let unrelated_vault = base.path().join("unrelated-vault");
    init_and_bind(&config, &active, &active_vault, "Active project");
    init_and_bind(&config, &platform, &platform_vault, "Platform reference");
    init_and_bind(&config, &security, &security_vault, "Security reference");
    init_and_bind(&config, &unrelated, &unrelated_vault, "Unrelated project");

    let create_rejected = run_ley(
        &config,
        &[
            "scope",
            "create",
            "team",
            "Platform team",
            platform.to_str().unwrap(),
            security.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!create_rejected.status.success());
    let create_stderr = String::from_utf8_lossy(&create_rejected.stderr);
    assert!(create_stderr.contains("Knowledge Scope creation is retired"));
    assert!(create_stderr.contains("per-task source selection"));

    let scope_id = "ksc_11111111111111111111111111111111";
    seed_legacy_scope(
        &config,
        &active,
        &[platform.as_path(), security.as_path()],
        scope_id,
        "Platform team",
    );

    let listed = json_stdout(ley(&config, &["scope", "list", "--json"]));
    assert_eq!(listed["scopes"].as_array().unwrap().len(), 1);
    assert_eq!(listed["scopes"][0]["scopeId"], scope_id);
    let serialized = serde_json::to_string(&listed).unwrap();
    assert!(!serialized.contains(active.to_str().unwrap()));
    assert!(!serialized.contains(platform.to_str().unwrap()));
    assert!(!serialized.contains(security.to_str().unwrap()));
    assert!(!serialized.contains(unrelated.to_str().unwrap()));
    assert!(!serialized.contains("Unrelated project"));

    let attach_rejected = run_ley(
        &config,
        &[
            "scope",
            "attach",
            &scope_id,
            active.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!attach_rejected.status.success());
    let attach_stderr = String::from_utf8_lossy(&attach_rejected.stderr);
    assert!(attach_stderr.contains("Knowledge Scope attachment is retired"));
    assert!(attach_stderr.contains("per-task source selection"));

    let active_scopes = json_stdout(ley(
        &config,
        &["scope", "attached", active.to_str().unwrap(), "--json"],
    ));
    assert_eq!(active_scopes["attachments"].as_array().unwrap().len(), 1);
    assert_eq!(active_scopes["attachments"][0]["scopeId"], scope_id);

    let unrelated_scopes = json_stdout(ley(
        &config,
        &["scope", "attached", unrelated.to_str().unwrap(), "--json"],
    ));
    assert!(unrelated_scopes["attachments"]
        .as_array()
        .unwrap()
        .is_empty());

    let detached = json_stdout(ley(
        &config,
        &[
            "scope",
            "detach",
            &scope_id,
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(detached["scopeId"], scope_id);
    let empty = json_stdout(ley(
        &config,
        &["scope", "attached", active.to_str().unwrap(), "--json"],
    ));
    assert!(empty["attachments"].as_array().unwrap().is_empty());
}
