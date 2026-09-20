use ley_core::{
    generate_specification_id, initialize_project, CaptureMode, SpecificationRegistry,
    APP_IDENTIFIER, BOOTSTRAP_SPECIFICATION_REGISTRY_FILE, SPECIFICATION_REGISTRY_FILE,
};
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

fn run_ley_with_input(config: &Path, arguments: &[&str], input: &Value) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(input).unwrap().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn ley_with_input(config: &Path, arguments: &[&str], input: &Value) -> Output {
    let output = run_ley_with_input(config, arguments, input);
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

fn inactive_mcp_tools(config: &Path, target: &Path) -> (Output, Vec<Value>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .args(["mcp", target.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "bootstrap-cli-test", "version": "1"}
                }
            })
        )
        .unwrap();
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .unwrap();
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
        )
        .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    let messages = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();
    (output, messages)
}

fn bootstrap_mcp_surface(config: &Path, target: &Path) -> (Output, Vec<Value>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .args(["mcp", target.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for request in [
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "bootstrap-cli-test", "version": "1"}
                }
            }),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "resources/list", "params": {}}),
        ] {
            writeln!(stdin, "{request}").unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    let messages = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect();
    (output, messages)
}

#[cfg(unix)]
#[test]
fn cli_bootstrap_specification_is_explicit_read_only_and_retires_on_initialization() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let source = base.path().join("source");
    let source_vault = base.path().join("source-vault");
    let target = base.path().join("target");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(source_vault.join("Specs")).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::write(source.join("README.md"), "# Source\n").unwrap();
    fs::write(
        source_vault.join("Specs/Product.md"),
        "# Product\n\nbootstrap_cli_marker must stay exact.\n",
    )
    .unwrap();

    ley(
        &config,
        &[
            "init",
            source.to_str().unwrap(),
            "--name",
            "Bootstrap CLI source",
            "--json",
        ],
    );
    ley(
        &config,
        &[
            "bind",
            source.to_str().unwrap(),
            "--vault",
            source_vault.to_str().unwrap(),
            "--json",
        ],
    );

    let specification_id = generate_specification_id();
    let specifications = SpecificationRegistry::at(
        config
            .join(APP_IDENTIFIER)
            .join(SPECIFICATION_REGISTRY_FILE),
    );
    specifications
        .approve(
            &source,
            &source_vault,
            &specification_id,
            "Specs/Product.md",
        )
        .unwrap();

    let attached = json_stdout(ley(
        &config,
        &[
            "bootstrap-spec",
            "attach",
            source.to_str().unwrap(),
            &specification_id,
            target.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(attached["created"], true);
    assert_eq!(attached["grant"]["specificationId"], specification_id);
    assert!(attached["grant"]["grantId"]
        .as_str()
        .unwrap()
        .starts_with("bsg_"));
    assert!(!target.join(".ley").exists());
    let serialized = attached.to_string();
    assert!(!serialized.contains(source.to_str().unwrap()));
    assert!(!serialized.contains(source_vault.to_str().unwrap()));
    assert!(!serialized.contains(target.to_str().unwrap()));

    let listed = json_stdout(ley(
        &config,
        &["bootstrap-spec", "list", target.to_str().unwrap(), "--json"],
    ));
    assert_eq!(listed["targetInitialized"], false);
    assert_eq!(listed["totalGrants"], 1);
    assert_eq!(listed["grants"][0]["specificationId"], specification_id);
    assert!(!listed.to_string().contains(target.to_str().unwrap()));

    let (mcp_surface, messages) = bootstrap_mcp_surface(&config, &target);
    assert!(
        mcp_surface.status.success(),
        "bootstrap MCP failed: {}",
        String::from_utf8_lossy(&mcp_surface.stderr)
    );
    let tools = messages
        .iter()
        .find(|message| message.get("id") == Some(&json!(2)))
        .and_then(|message| message.pointer("/result/tools"))
        .and_then(Value::as_array)
        .expect("bootstrap MCP tools/list response");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "ley_compile_context");
    let resources_response = messages
        .iter()
        .find(|message| message.get("id") == Some(&json!(3)))
        .expect("bootstrap MCP resources/list response");
    let resources_empty = resources_response
        .pointer("/result/resources")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty);
    let resources_unsupported = resources_response.get("error").is_some();
    assert!(resources_empty || resources_unsupported);

    let session_start = json_stdout(ley_with_input(
        &config,
        &["hook", "--host", "codex", target.to_str().unwrap()],
        &json!({
            "hook_event_name": "SessionStart",
            "session_id": "bootstrap-cli-thread",
        }),
    ));
    assert_eq!(session_start, json!({}));
    let codex_stop = json_stdout(ley_with_input(
        &config,
        &["hook", "--host", "codex", target.to_str().unwrap()],
        &json!({
            "hook_event_name": "Stop",
            "session_id": "bootstrap-cli-thread",
            "last_assistant_message": "bootstrap_stop_private_marker",
        }),
    ));
    assert_eq!(codex_stop, json!({}));
    let claude_stop = json_stdout(ley_with_input(
        &config,
        &["hook", "--host", "claude", target.to_str().unwrap()],
        &json!({
            "hook_event_name": "Stop",
            "session_id": "bootstrap-cli-claude-thread",
            "last_assistant_message": "bootstrap_stop_private_marker",
        }),
    ));
    assert_eq!(claude_stop, json!({}));

    let prompt_marker = "BOOTSTRAP_CLI_PROMPT_ONLY_90af";
    let prompt = json_stdout(ley_with_input(
        &config,
        &["hook", "--host", "codex", target.to_str().unwrap()],
        &json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "bootstrap-cli-thread",
            "turn_id": "bootstrap-turn-1",
            "prompt": format!("implement bootstrap_cli_marker {prompt_marker}"),
        }),
    ));
    let context = prompt["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(context.starts_with("# Ley bootstrap task context (automatic)"));
    assert!(context.contains("bootstrap_cli_marker"));
    assert!(context.contains("No Ley project memory or Ley session is active here"));
    assert!(!context.contains(prompt_marker));
    assert!(context.len() <= 3_500);
    assert!(!target.join(".ley").exists());

    let initialized = json_stdout(ley(
        &config,
        &[
            "init",
            target.to_str().unwrap(),
            "--name",
            "Bootstrapped target",
            "--json",
        ],
    ));
    assert_eq!(initialized["created"], true);
    assert!(target.join(".ley").is_dir());

    let after = json_stdout(ley(
        &config,
        &["bootstrap-spec", "list", target.to_str().unwrap(), "--json"],
    ));
    assert_eq!(after["targetInitialized"], true);
    assert_eq!(after["totalGrants"], 0);
    assert!(after["grants"].as_array().unwrap().is_empty());

    let post_init_hook = json_stdout(ley_with_input(
        &config,
        &["hook", "--host", "codex", target.to_str().unwrap()],
        &json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "bootstrap-cli-thread",
            "turn_id": "bootstrap-turn-2",
            "prompt": "implement bootstrap_cli_marker",
        }),
    ));
    assert_eq!(post_init_hook, json!({}));

    fs::remove_dir_all(target.join(".ley")).unwrap();
    let after_metadata_removal = json_stdout(ley(
        &config,
        &["bootstrap-spec", "list", target.to_str().unwrap(), "--json"],
    ));
    assert_eq!(after_metadata_removal["targetInitialized"], false);
    assert_eq!(after_metadata_removal["totalGrants"], 0);
}

#[test]
fn ordinary_uninitialized_hook_and_mcp_do_not_create_bootstrap_config_state() {
    let base = tempdir().unwrap();
    let config = base.path().join("unused-config");
    let target = base.path().join("ordinary-target");
    fs::create_dir(&target).unwrap();
    let app_dir = config.join(APP_IDENTIFIER);

    let hook = json_stdout(ley_with_input(
        &config,
        &["hook", "--host", "codex", target.to_str().unwrap()],
        &json!({
            "hook_event_name": "SessionStart",
            "session_id": "ordinary-uninitialized",
        }),
    ));
    assert_eq!(hook, json!({}));
    assert!(!app_dir.exists());

    let (mcp, messages) = inactive_mcp_tools(&config, &target);
    assert!(
        mcp.status.success(),
        "inactive MCP failed: {}",
        String::from_utf8_lossy(&mcp.stderr)
    );
    let tools = messages
        .iter()
        .find(|message| message.get("id") == Some(&json!(2)))
        .and_then(|message| message.pointer("/result/tools"))
        .and_then(Value::as_array)
        .expect("inactive MCP tools/list response");
    assert!(tools.is_empty());
    assert!(!app_dir.exists());
}

#[cfg(unix)]
#[test]
fn bootstrap_hook_and_mcp_startup_errors_do_not_leak_private_paths() {
    use std::os::unix::fs::PermissionsExt;

    let base = tempdir().unwrap();
    let config = base.path().join("private-config-root");
    let app_dir = config.join(APP_IDENTIFIER);
    let target = base.path().join("private-target-path");
    fs::create_dir_all(&app_dir).unwrap();
    fs::set_permissions(&app_dir, fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir(&target).unwrap();
    let registry = app_dir.join(BOOTSTRAP_SPECIFICATION_REGISTRY_FILE);
    fs::write(&registry, b"{malformed-bootstrap-registry").unwrap();
    fs::set_permissions(&registry, fs::Permissions::from_mode(0o600)).unwrap();

    let hook = run_ley_with_input(
        &config,
        &["hook", "--host", "codex", target.to_str().unwrap()],
        &json!({
            "hook_event_name": "SessionStart",
            "session_id": "malformed-bootstrap",
        }),
    );
    assert!(!hook.status.success());
    let hook_error = String::from_utf8_lossy(&hook.stderr);
    assert!(hook_error.contains("bootstrap Specification authority is unavailable or invalid"));
    assert!(!hook_error.contains(target.to_str().unwrap()));
    assert!(!hook_error.contains(config.to_str().unwrap()));
    assert!(!hook_error.contains(registry.to_str().unwrap()));

    let mcp = run_ley(&config, &["mcp", target.to_str().unwrap()]);
    assert!(!mcp.status.success());
    let mcp_error = String::from_utf8_lossy(&mcp.stderr);
    assert!(mcp_error.contains("bootstrap Specification authority is unavailable or invalid"));
    assert!(!mcp_error.contains(target.to_str().unwrap()));
    assert!(!mcp_error.contains(config.to_str().unwrap()));
    assert!(!mcp_error.contains(registry.to_str().unwrap()));
}

#[cfg(unix)]
#[test]
fn initialized_unbound_project_never_falls_back_to_stale_bootstrap_authority() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let source = base.path().join("source");
    let source_vault = base.path().join("source-vault");
    let target = base.path().join("target");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(source_vault.join("Specs")).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::write(
        source_vault.join("Specs/Product.md"),
        "# Product\n\nstale_bootstrap_marker must not bypass an unbound project.\n",
    )
    .unwrap();

    ley(
        &config,
        &[
            "init",
            source.to_str().unwrap(),
            "--name",
            "Stale bootstrap source",
            "--json",
        ],
    );
    ley(
        &config,
        &[
            "bind",
            source.to_str().unwrap(),
            "--vault",
            source_vault.to_str().unwrap(),
            "--json",
        ],
    );
    let specification_id = generate_specification_id();
    let specifications = SpecificationRegistry::at(
        config
            .join(APP_IDENTIFIER)
            .join(SPECIFICATION_REGISTRY_FILE),
    );
    specifications
        .approve(
            &source,
            &source_vault,
            &specification_id,
            "Specs/Product.md",
        )
        .unwrap();
    ley(
        &config,
        &[
            "bootstrap-spec",
            "attach",
            source.to_str().unwrap(),
            &specification_id,
            target.to_str().unwrap(),
            "--json",
        ],
    );

    initialize_project(
        &target,
        Some("Raw initialized target"),
        CaptureMode::Structured,
    )
    .unwrap();
    let listed = json_stdout(ley(
        &config,
        &["bootstrap-spec", "list", target.to_str().unwrap(), "--json"],
    ));
    assert_eq!(listed["targetInitialized"], true);
    assert_eq!(listed["totalGrants"], 1);

    let (mcp, messages) = inactive_mcp_tools(&config, &target);
    assert!(
        mcp.status.success(),
        "initialized-unbound MCP failed: {}",
        String::from_utf8_lossy(&mcp.stderr)
    );
    let tools = messages
        .iter()
        .find(|message| message.get("id") == Some(&json!(2)))
        .and_then(|message| message.pointer("/result/tools"))
        .and_then(Value::as_array)
        .expect("initialized-unbound tools/list response");
    assert!(tools.is_empty());
}
