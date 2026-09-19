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
fn cli_creates_and_attaches_reusable_team_scope_without_machine_path_leakage() {
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

    let created = json_stdout(ley(
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
    ));
    assert_eq!(created["created"], true);
    assert_eq!(created["scope"]["kind"], "team");
    assert_eq!(created["scope"]["name"], "Platform team");
    assert_eq!(created["scope"]["permission"], "read-only");
    assert_eq!(created["scope"]["sources"].as_array().unwrap().len(), 2);
    assert!(created["scope"]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .all(|source| source["status"] == "ready"));
    let scope_id = created["scope"]["scopeId"].as_str().unwrap().to_owned();

    let listed = json_stdout(ley(&config, &["scope", "list", "--json"]));
    assert_eq!(listed["scopes"].as_array().unwrap().len(), 1);
    assert_eq!(listed["scopes"][0]["scopeId"], scope_id);
    let serialized = serde_json::to_string(&listed).unwrap();
    assert!(!serialized.contains(active.to_str().unwrap()));
    assert!(!serialized.contains(platform.to_str().unwrap()));
    assert!(!serialized.contains(security.to_str().unwrap()));
    assert!(!serialized.contains(unrelated.to_str().unwrap()));
    assert!(!serialized.contains("Unrelated project"));

    let attached = json_stdout(ley(
        &config,
        &[
            "scope",
            "attach",
            &scope_id,
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(attached["created"], true);
    assert_eq!(attached["attachment"]["scopeId"], scope_id);
    assert_eq!(attached["attachment"]["permission"], "read-only");
    assert_eq!(attached["attachment"]["kind"], "team");
    assert_eq!(attached["attachment"]["name"], "Platform team");

    let duplicate = json_stdout(ley(
        &config,
        &[
            "scope",
            "attach",
            &scope_id,
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(duplicate["created"], false);

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
