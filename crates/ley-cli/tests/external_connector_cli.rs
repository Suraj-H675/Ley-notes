use ley_core::{APP_IDENTIFIER, EXTERNAL_CONNECTOR_REGISTRY_FILE};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

const LEGACY_PROJECT_ID: &str = "prj_11111111111111111111111111111111";
const LEGACY_CONNECTOR_ID: &str = "ext_0673720492fd9e23cc54fe3901aceeef";
const LEGACY_SNAPSHOT_ID: &str =
    "exts_68fea0542aeb01b73110ae099b0d24e9f3dcd7bed271b1fc924c2717f48a347b";
const LEGACY_SOURCE_URL: &str = "https://github.com/openai/ley-test/issues/42";

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
    ley_core::initialize_project(
        project,
        Some("Connector compatibility"),
        ley_core::CaptureMode::Structured,
    )
    .unwrap();
    let project_file = project.join(".ley/project.json");
    let mut identity: Value = serde_json::from_slice(&fs::read(&project_file).unwrap()).unwrap();
    identity["projectId"] = Value::String(LEGACY_PROJECT_ID.to_owned());
    fs::write(&project_file, serde_json::to_vec_pretty(&identity).unwrap()).unwrap();
    ley_core::ingest_project(project, vault).unwrap();
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

fn write_private_json(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

fn make_private_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

fn seed_legacy_connector(config: &Path, vault: &Path) -> PathBuf {
    let registry_path = config
        .join(APP_IDENTIFIER)
        .join(EXTERNAL_CONNECTOR_REGISTRY_FILE);
    let source = serde_json::json!({
        "provider": "git-hub",
        "resourceKind": "issue",
        "owner": "openai",
        "repository": "ley-test",
        "number": 42,
        "canonicalUrl": LEGACY_SOURCE_URL
    });
    let registry = serde_json::json!({
        "schemaVersion": 1,
        "connectors": {
            LEGACY_PROJECT_ID: {
                LEGACY_CONNECTOR_ID: {
                    "source": source,
                    "createdAtUnixMs": 1,
                    "agentContextEnabled": true
                }
            }
        }
    });
    write_private_json(&registry_path, &registry);

    let connector_dir = vault
        .join(".ley/agent-memory/projects")
        .join(LEGACY_PROJECT_ID)
        .join("external-connectors")
        .join(LEGACY_CONNECTOR_ID);
    let snapshots_dir = connector_dir.join("snapshots");
    make_private_dir(&connector_dir);
    make_private_dir(&snapshots_dir);

    let pointer = serde_json::json!({
        "schemaVersion": 1,
        "projectId": LEGACY_PROJECT_ID,
        "connectorId": LEGACY_CONNECTOR_ID,
        "snapshotId": LEGACY_SNAPSHOT_ID,
        "refreshedAtUnixMs": 1
    });
    write_private_json(&connector_dir.join("current-v1.json"), &pointer);

    let snapshot = serde_json::json!({
        "schemaVersion": 1,
        "projectId": LEGACY_PROJECT_ID,
        "connectorId": LEGACY_CONNECTOR_ID,
        "snapshotId": LEGACY_SNAPSHOT_ID,
        "source": {
            "provider": "git-hub",
            "resourceKind": "issue",
            "owner": "openai",
            "repository": "ley-test",
            "number": 42,
            "canonicalUrl": LEGACY_SOURCE_URL
        },
        "title": "CLI legacy connector",
        "body": "Persisted legacy snapshot from CLI fixture.",
        "state": "closed",
        "labels": [],
        "sourceUpdatedAt": "2026-09-19T03:30:00Z",
        "redactions": []
    });
    let snapshot_path = snapshots_dir.join(format!("{LEGACY_SNAPSHOT_ID}.json"));
    write_private_json(&snapshot_path, &snapshot);
    snapshot_path
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

    let snapshot_path = seed_legacy_connector(&config, &vault);
    let connector_id = LEGACY_CONNECTOR_ID;
    let listed = json_stdout(ley(
        &config,
        &["connector", "list", project.to_str().unwrap(), "--json"],
    ));
    assert_eq!(listed["connectors"].as_array().unwrap().len(), 1);
    assert_eq!(listed["connectors"][0]["connectorId"], connector_id);
    let serialized = listed.to_string();
    assert!(!serialized.contains(project.to_str().unwrap()));
    assert!(!serialized.contains(vault.to_str().unwrap()));

    let shown = json_stdout(ley(
        &config,
        &[
            "connector",
            "show",
            connector_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(shown["snapshotId"], LEGACY_SNAPSHOT_ID);
    assert_eq!(shown["title"], "CLI legacy connector");
    assert_eq!(shown["body"], "Persisted legacy snapshot from CLI fixture.");
    assert_eq!(shown["liveSourceChecked"], false);

    let removed = json_stdout(ley(
        &config,
        &[
            "connector",
            "remove",
            connector_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(removed["connectorId"], connector_id);
    assert!(!snapshot_path.exists());

    let empty = json_stdout(ley(
        &config,
        &["connector", "list", project.to_str().unwrap(), "--json"],
    ));
    assert!(empty["connectors"].as_array().unwrap().is_empty());
}
