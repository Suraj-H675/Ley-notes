mod common;

use common::seed_legacy_project;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
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

fn run_ley_with_lines(config: &Path, arguments: &[&str], lines: &[Value]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for line in lines {
            writeln!(stdin, "{line}").unwrap();
        }
    }
    child.wait_with_output().unwrap()
}

#[test]
fn legacy_binding_is_reconnect_only_and_survives_a_moved_vault_after_cutover() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let project = base.path().join("project");
    let legacy_vault = base.path().join("legacy-vault");
    let moved_vault = base.path().join("moved-vault");
    let empty_vault = base.path().join("empty-vault");
    let wrong_project = base.path().join("wrong-project");
    let wrong_vault = base.path().join("wrong-vault");

    seed_legacy_project(&project, &legacy_vault, "Legacy reconnect");
    seed_legacy_project(&wrong_project, &wrong_vault, "Wrong legacy project");
    fs::create_dir(&empty_vault).unwrap();

    let empty = run_ley(
        &config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            empty_vault.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!empty.status.success());
    assert!(String::from_utf8_lossy(&empty.stderr).contains("reconnect-only"));
    assert!(fs::read_dir(&empty_vault).unwrap().next().is_none());

    let empty_inspection = run_ley(
        &config,
        &[
            "binding",
            project.to_str().unwrap(),
            "--vault",
            empty_vault.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!empty_inspection.status.success());
    assert!(String::from_utf8_lossy(&empty_inspection.stderr)
        .contains("explicit legacy vault overrides are compatibility-only"));

    let invalid_hook = run_ley_with_lines(
        &config,
        &[
            "hook",
            project.to_str().unwrap(),
            "--host",
            "codex",
            "--vault",
            empty_vault.to_str().unwrap(),
        ],
        &[json!({
            "session_id": "invalid-override-hook",
            "cwd": project,
            "hook_event_name": "SessionStart",
            "source": "startup"
        })],
    );
    assert!(invalid_hook.status.success());
    assert_eq!(String::from_utf8_lossy(&invalid_hook.stdout).trim(), "{}");
    assert!(fs::read_dir(&empty_vault).unwrap().next().is_none());

    let invalid_mcp = run_ley_with_lines(
        &config,
        &[
            "mcp",
            project.to_str().unwrap(),
            "--vault",
            empty_vault.to_str().unwrap(),
        ],
        &[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "legacy-binding-cli-test", "version": "1"}
                }
            }),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
        ],
    );
    assert!(invalid_mcp.status.success());
    let mcp_messages = String::from_utf8_lossy(&invalid_mcp.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>();
    let tools = mcp_messages
        .iter()
        .find(|message| message["id"] == 2)
        .and_then(|message| message["result"]["tools"].as_array())
        .expect("invalid-override MCP tools/list response");
    assert!(tools.is_empty());

    let wrong = run_ley(
        &config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            wrong_vault.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!wrong.status.success());
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("reconnect-only"));

    let transitioned = json_stdout(ley(
        &config,
        &[
            "ingest",
            project.to_str().unwrap(),
            "--vault",
            legacy_vault.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(transitioned["binding"]["source"], "override");
    assert_eq!(transitioned["storage"], "legacy-vault");
    assert!(transitioned["ingestion"]["manifestPath"].is_null());

    let bound = json_stdout(ley(
        &config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            legacy_vault.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(bound["source"], "persisted");
    assert_eq!(
        bound["vaultPath"],
        legacy_vault
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .as_ref()
    );

    let ingested = json_stdout(ley(
        &config,
        &["ingest", project.to_str().unwrap(), "--json"],
    ));
    assert_eq!(ingested["storage"], "legacy-vault");
    assert!(ingested["ingestion"]["manifestPath"].is_null());

    fs::rename(&legacy_vault, &moved_vault).unwrap();
    let rebound = json_stdout(ley(
        &config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            moved_vault.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(
        rebound["vaultPath"],
        moved_vault
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .as_ref()
    );
}
