use ley_core::{ExternalConnectorRegistry, APP_IDENTIFIER, EXTERNAL_CONNECTOR_REGISTRY_FILE};
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

fn init_and_bind(config: &Path, project: &Path, vault: &Path) {
    fs::create_dir_all(project).unwrap();
    fs::create_dir_all(vault).unwrap();
    fs::write(project.join("README.md"), "# Connector compatibility\n").unwrap();
    ley(
        config,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Connector compatibility",
            "--json",
        ],
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
fn cli_retires_connector_creation_and_refresh_but_lists_and_removes_legacy_authority() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let project = base.path().join("project");
    let vault = base.path().join("vault");
    init_and_bind(&config, &project, &vault);

    let add_rejected = run_ley(
        &config,
        &[
            "connector",
            "add",
            "https://github.com/openai/ley-test/issues/42",
            project.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!add_rejected.status.success());
    assert!(
        String::from_utf8_lossy(&add_rejected.stderr).contains("unknown connector command 'add'")
    );

    let refresh_rejected = run_ley(
        &config,
        &[
            "connector",
            "refresh",
            "ext_11111111111111111111111111111111",
            project.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!refresh_rejected.status.success());
    assert!(String::from_utf8_lossy(&refresh_rejected.stderr)
        .contains("unknown connector command 'refresh'"));

    let registry = ExternalConnectorRegistry::at(
        config
            .join(APP_IDENTIFIER)
            .join(EXTERNAL_CONNECTOR_REGISTRY_FILE),
    );
    let seeded = registry
        .add_public_github_reference(&project, "https://github.com/openai/ley-test/issues/42")
        .unwrap();
    let connector_id = seeded.connector.connector_id;

    let listed = json_stdout(ley(
        &config,
        &["connector", "list", project.to_str().unwrap(), "--json"],
    ));
    assert_eq!(listed["connectors"].as_array().unwrap().len(), 1);
    assert_eq!(listed["connectors"][0]["connectorId"], connector_id);
    let serialized = listed.to_string();
    assert!(!serialized.contains(project.to_str().unwrap()));
    assert!(!serialized.contains(vault.to_str().unwrap()));

    let removed = json_stdout(ley(
        &config,
        &[
            "connector",
            "remove",
            &connector_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(removed["connectorId"], connector_id);

    let empty = json_stdout(ley(
        &config,
        &["connector", "list", project.to_str().unwrap(), "--json"],
    ));
    assert!(empty["connectors"].as_array().unwrap().is_empty());
}
