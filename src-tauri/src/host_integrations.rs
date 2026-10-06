use atomic_write_file::AtomicWriteFile;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Manager};
use toml_edit::{value, Array, DocumentMut, Item, Table};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const MANAGED_MARKETPLACE: &str = "ley-desktop";
const CODEX_PLUGIN_MANIFEST: &str =
    include_str!("../../integrations/codex/plugins/ley-memory/.codex-plugin/plugin.json");
const CODEX_HOOKS: &str =
    include_str!("../../integrations/codex/plugins/ley-memory/hooks/hooks.json");
const CODEX_SKILL: &str =
    include_str!("../../integrations/codex/plugins/ley-memory/skills/ley/SKILL.md");
const CLAUDE_PLUGIN_MANIFEST: &str =
    include_str!("../../integrations/claude-code/ley-memory/.claude-plugin/plugin.json");
const CLAUDE_MCP: &str = include_str!("../../integrations/claude-code/ley-memory/.mcp.json");
const CLAUDE_HOOKS: &str =
    include_str!("../../integrations/claude-code/ley-memory/hooks/hooks.json");
const CLAUDE_SKILL: &str =
    include_str!("../../integrations/claude-code/ley-memory/skills/ley-memory/SKILL.md");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostIntegrationStatus {
    pub(crate) id: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) detected: bool,
    pub(crate) executable_path: Option<PathBuf>,
    pub(crate) version: Option<String>,
    pub(crate) configured: Option<bool>,
    pub(crate) enabled: Option<bool>,
    pub(crate) managed_by_ley_desktop: bool,
    pub(crate) restart_required: bool,
    pub(crate) review_required: bool,
    pub(crate) status_detail: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HostProbe {
    Codex,
    ClaudeCode,
}

pub(crate) fn inspect_host_integrations(
    project_path: Option<&Path>,
    helper_path: Option<&Path>,
) -> Vec<HostIntegrationStatus> {
    [
        inspect_host(
            "codex",
            "Codex",
            "codex",
            HostProbe::Codex,
            project_path,
            helper_path,
        ),
        inspect_host(
            "claude-code",
            "Claude Code",
            "claude",
            HostProbe::ClaudeCode,
            project_path,
            helper_path,
        ),
    ]
    .into_iter()
    .collect()
}

pub(crate) fn connect_host(
    app: &AppHandle,
    project_path: &Path,
    host_id: &str,
    helper_path: &Path,
) -> Result<HostIntegrationStatus, String> {
    let project_path = project_path
        .canonicalize()
        .map_err(|error| format!("could not resolve the selected project: {error}"))?;
    if !project_path.is_dir() {
        return Err("the selected Ley project is not an accessible directory".to_owned());
    }

    let (id, display_name, executable_name, probe) = match host_id {
        "codex" => ("codex", "Codex", "codex", HostProbe::Codex),
        "claude-code" => (
            "claude-code",
            "Claude Code",
            "claude",
            HostProbe::ClaudeCode,
        ),
        _ => return Err(format!("unsupported coding-agent host '{host_id}'")),
    };
    let executable = find_executable(executable_name)
        .ok_or_else(|| format!("{display_name} is not installed or could not be found."))?;

    let app_data = app
        .path()
        .app_local_data_dir()
        .map_err(|error| format!("could not resolve Ley's private application data: {error}"))?;
    let marketplace_root = app_data
        .join("host-integrations")
        .join(host_id)
        .join("marketplace");
    write_managed_marketplace(probe, &marketplace_root, helper_path)?;

    let was_managed = managed_plugin_status(probe, &executable, Some(&project_path))
        .map(|(configured, _)| configured)
        .unwrap_or(false);
    ensure_marketplace_registered(probe, &executable, &marketplace_root, &project_path)?;
    if !was_managed {
        install_managed_plugin(probe, &executable, &project_path)?;
    }

    if probe == HostProbe::Codex {
        if !was_managed {
            set_codex_global_plugin_enabled(false)?;
        }
        write_codex_project_config(&project_path, helper_path)?;
    }

    let mut status = inspect_host(
        id,
        display_name,
        executable_name,
        probe,
        Some(&project_path),
        Some(helper_path),
    );
    status.restart_required = true;
    status.review_required = true;
    status.status_detail = match probe {
        HostProbe::Codex => {
            "Ley is configured for this project. Restart Codex, trust the project configuration, then review the Ley hooks before they can run.".to_owned()
        }
        HostProbe::ClaudeCode => {
            "Ley is installed for this project. Restart Claude Code if it is already open, then review the plugin and its hooks before relying on capture.".to_owned()
        }
    };
    Ok(status)
}

pub(crate) fn disconnect_host(
    project_path: &Path,
    host_id: &str,
    helper_path: &Path,
) -> Result<HostIntegrationStatus, String> {
    let project_path = project_path
        .canonicalize()
        .map_err(|error| format!("could not resolve the selected project: {error}"))?;
    if !project_path.is_dir() {
        return Err("the selected Ley project is not an accessible directory".to_owned());
    }

    let (id, display_name, executable_name, probe) = match host_id {
        "codex" => ("codex", "Codex", "codex", HostProbe::Codex),
        "claude-code" => (
            "claude-code",
            "Claude Code",
            "claude",
            HostProbe::ClaudeCode,
        ),
        _ => return Err(format!("unsupported coding-agent host '{host_id}'")),
    };
    let changed = match probe {
        HostProbe::Codex => remove_codex_project_config(&project_path, helper_path)?,
        HostProbe::ClaudeCode => {
            let executable = find_executable(executable_name)
                .ok_or_else(|| format!("{display_name} is not installed or could not be found."))?;
            let managed = managed_plugin_status(probe, &executable, Some(&project_path))
                .map(|(configured, _)| configured)
                .unwrap_or(false);
            if managed {
                run_host_command(
                    &executable,
                    [
                        OsStr::new("plugin"),
                        OsStr::new("uninstall"),
                        OsStr::new("ley-memory@ley-desktop"),
                        OsStr::new("--scope"),
                        OsStr::new("project"),
                        OsStr::new("--json"),
                    ],
                    Some(&project_path),
                )?;
            }
            managed
        }
    };

    let mut status = inspect_host(
        id,
        display_name,
        executable_name,
        probe,
        Some(&project_path),
        Some(helper_path),
    );
    if changed {
        status.restart_required = true;
        status.review_required = false;
        status.status_detail = match probe {
            HostProbe::Codex => {
                "Ley removed this project's Codex binding. Restart Codex if it is already open. Ley's shared package may remain installed for other projects.".to_owned()
            }
            HostProbe::ClaudeCode => {
                "Ley removed its project-scoped Claude Code plugin. Restart Claude Code if it is already open. Ley's shared marketplace registration may remain for other projects.".to_owned()
            }
        };
    }
    Ok(status)
}

fn inspect_host(
    id: &'static str,
    display_name: &'static str,
    executable_name: &str,
    probe: HostProbe,
    project_path: Option<&Path>,
    helper_path: Option<&Path>,
) -> HostIntegrationStatus {
    let Some(executable_path) = find_executable(executable_name) else {
        return HostIntegrationStatus {
            id,
            display_name,
            detected: false,
            executable_path: None,
            version: None,
            configured: None,
            enabled: None,
            managed_by_ley_desktop: false,
            restart_required: false,
            review_required: false,
            status_detail: format!("{display_name} was not found on this device."),
        };
    };

    let version = command_stdout(&executable_path, &["--version"], project_path);
    let plugin_json = command_stdout(
        &executable_path,
        &["plugin", "list", "--json"],
        project_path,
    );
    let (installed, enabled) = plugin_json
        .as_deref()
        .and_then(|json| plugin_status(probe, json))
        .unwrap_or((false, false));
    let managed_package = plugin_json
        .as_deref()
        .and_then(|json| managed_plugin_status_from_json(probe, json))
        .map(|(configured, _)| configured)
        .unwrap_or(false);

    let managed_project_binding = match (probe, project_path, helper_path) {
        (HostProbe::Codex, Some(project_path), Some(helper_path)) => {
            codex_project_config_is_managed(project_path, helper_path)
        }
        (HostProbe::ClaudeCode, _, _) => managed_package,
        _ => false,
    };
    let configured = installed;
    let managed = match (probe, project_path) {
        (HostProbe::Codex, Some(_)) => managed_project_binding,
        _ => managed_package,
    };

    let status_detail = if managed_project_binding && enabled {
        "Ley Desktop's integration package is installed and enabled. This does not prove a hook or MCP request has run successfully yet.".to_owned()
    } else if managed_project_binding {
        "Ley Desktop's integration package is installed but is not enabled in this scope."
            .to_owned()
    } else if probe == HostProbe::Codex && managed_package {
        "Ley Desktop's shared Codex package is installed, but this project is not connected."
            .to_owned()
    } else if configured && enabled {
        "A Ley integration is installed and enabled, but it is not the package managed by this Ley Desktop installation.".to_owned()
    } else if configured {
        "A Ley integration is installed in this host but is not enabled.".to_owned()
    } else {
        format!("{display_name} is available. Ley has not been detected in its installed plugins.")
    };

    HostIntegrationStatus {
        id,
        display_name,
        detected: true,
        executable_path: Some(executable_path),
        version,
        configured: Some(configured),
        enabled: Some(enabled),
        managed_by_ley_desktop: managed,
        restart_required: false,
        review_required: managed,
        status_detail,
    }
}

fn plugin_status(probe: HostProbe, json: &str) -> Option<(bool, bool)> {
    let value: Value = serde_json::from_str(json).ok()?;
    let plugin = match probe {
        HostProbe::Codex => value
            .get("installed")?
            .as_array()?
            .iter()
            .find(|plugin| plugin.get("name").and_then(Value::as_str) == Some("ley-memory")),
        HostProbe::ClaudeCode => value.as_array()?.iter().find(|plugin| {
            plugin
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id == "ley-memory" || id.starts_with("ley-memory@"))
        }),
    };
    Some(match plugin {
        Some(plugin) => (
            true,
            plugin
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        None => (false, false),
    })
}

fn managed_plugin_status(
    probe: HostProbe,
    executable: &Path,
    project_path: Option<&Path>,
) -> Option<(bool, bool)> {
    let json = command_stdout(executable, &["plugin", "list", "--json"], project_path)?;
    managed_plugin_status_from_json(probe, &json)
}

fn managed_plugin_status_from_json(probe: HostProbe, json: &str) -> Option<(bool, bool)> {
    let value: Value = serde_json::from_str(json).ok()?;
    let plugin = match probe {
        HostProbe::Codex => value.get("installed")?.as_array()?.iter().find(|plugin| {
            plugin.get("name").and_then(Value::as_str) == Some("ley-memory")
                && plugin.get("marketplaceName").and_then(Value::as_str)
                    == Some(MANAGED_MARKETPLACE)
        }),
        HostProbe::ClaudeCode => value.as_array()?.iter().find(|plugin| {
            plugin.get("id").and_then(Value::as_str) == Some("ley-memory@ley-desktop")
        }),
    };
    Some(match plugin {
        Some(plugin) => (
            true,
            plugin
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        None => (false, false),
    })
}

fn ensure_marketplace_registered(
    probe: HostProbe,
    executable: &Path,
    marketplace_root: &Path,
    project_path: &Path,
) -> Result<(), String> {
    let listed = command_stdout(
        executable,
        &["plugin", "marketplace", "list", "--json"],
        Some(project_path),
    )
    .and_then(|json| marketplace_is_registered(probe, &json));
    if listed == Some(true) {
        return Ok(());
    }

    let source = marketplace_root.as_os_str();
    match probe {
        HostProbe::Codex => run_host_command(
            executable,
            [
                OsStr::new("plugin"),
                OsStr::new("marketplace"),
                OsStr::new("add"),
                source,
                OsStr::new("--json"),
            ],
            Some(project_path),
        ),
        HostProbe::ClaudeCode => run_host_command(
            executable,
            [
                OsStr::new("plugin"),
                OsStr::new("marketplace"),
                OsStr::new("add"),
                OsStr::new("--scope"),
                OsStr::new("user"),
                source,
            ],
            Some(project_path),
        ),
    }
}

fn marketplace_is_registered(probe: HostProbe, json: &str) -> Option<bool> {
    let value: Value = serde_json::from_str(json).ok()?;
    let marketplaces = match probe {
        HostProbe::Codex => value.get("marketplaces")?.as_array()?,
        HostProbe::ClaudeCode => value.as_array()?,
    };
    Some(marketplaces.iter().any(|marketplace| {
        marketplace.get("name").and_then(Value::as_str) == Some(MANAGED_MARKETPLACE)
    }))
}

fn install_managed_plugin(
    probe: HostProbe,
    executable: &Path,
    project_path: &Path,
) -> Result<(), String> {
    match probe {
        HostProbe::Codex => run_host_command(
            executable,
            [
                OsStr::new("plugin"),
                OsStr::new("add"),
                OsStr::new("ley-memory@ley-desktop"),
                OsStr::new("--json"),
            ],
            Some(project_path),
        ),
        HostProbe::ClaudeCode => run_host_command(
            executable,
            [
                OsStr::new("plugin"),
                OsStr::new("install"),
                OsStr::new("ley-memory@ley-desktop"),
                OsStr::new("--scope"),
                OsStr::new("project"),
                OsStr::new("--json"),
            ],
            Some(project_path),
        ),
    }
}

fn write_managed_marketplace(
    probe: HostProbe,
    root: &Path,
    helper_path: &Path,
) -> Result<(), String> {
    let plugin_root = root.join("plugins/ley-memory");
    create_private_dir(&plugin_root)?;

    match probe {
        HostProbe::Codex => {
            let mut manifest: Value = serde_json::from_str(CODEX_PLUGIN_MANIFEST)
                .map_err(|error| format!("invalid bundled Codex plugin manifest: {error}"))?;
            manifest
                .as_object_mut()
                .ok_or_else(|| "bundled Codex manifest is not an object".to_owned())?
                .remove("mcpServers");
            write_json(&plugin_root.join(".codex-plugin/plugin.json"), &manifest)?;

            let command = codex_hook_command(helper_path);
            let mut hooks: Value = serde_json::from_str(CODEX_HOOKS)
                .map_err(|error| format!("invalid bundled Codex hooks: {error}"))?;
            rewrite_hook_commands(&mut hooks, &command)?;
            write_json(&plugin_root.join("hooks/hooks.json"), &hooks)?;
            write_text(&plugin_root.join("skills/ley/SKILL.md"), CODEX_SKILL)?;

            let marketplace = json!({
                "name": MANAGED_MARKETPLACE,
                "interface": { "displayName": "Ley Desktop" },
                "plugins": [{
                    "name": "ley-memory",
                    "source": { "source": "local", "path": "./plugins/ley-memory" },
                    "policy": { "installation": "AVAILABLE", "authentication": "ON_INSTALL" },
                    "category": "Productivity"
                }]
            });
            write_json(&root.join(".agents/plugins/marketplace.json"), &marketplace)?;
        }
        HostProbe::ClaudeCode => {
            let manifest: Value = serde_json::from_str(CLAUDE_PLUGIN_MANIFEST)
                .map_err(|error| format!("invalid bundled Claude plugin manifest: {error}"))?;
            write_json(&plugin_root.join(".claude-plugin/plugin.json"), &manifest)?;

            let mut mcp: Value = serde_json::from_str(CLAUDE_MCP)
                .map_err(|error| format!("invalid bundled Claude MCP config: {error}"))?;
            mcp["mcpServers"]["ley"]["command"] =
                Value::String(helper_path.to_string_lossy().into_owned());
            write_json(&plugin_root.join(".mcp.json"), &mcp)?;

            let mut hooks: Value = serde_json::from_str(CLAUDE_HOOKS)
                .map_err(|error| format!("invalid bundled Claude hooks: {error}"))?;
            rewrite_hook_commands(&mut hooks, &helper_path.to_string_lossy())?;
            write_json(&plugin_root.join("hooks/hooks.json"), &hooks)?;
            write_text(
                &plugin_root.join("skills/ley-memory/SKILL.md"),
                CLAUDE_SKILL,
            )?;

            let marketplace = json!({
                "name": MANAGED_MARKETPLACE,
                "description": "Ley Desktop's local coding-agent integration package.",
                "owner": { "name": "Ley" },
                "plugins": [{
                    "name": "ley-memory",
                    "source": "./plugins/ley-memory",
                    "description": "Private local project memory and cited continuity for Claude Code.",
                    "category": "Productivity",
                    "tags": ["local-first", "memory", "mcp"]
                }]
            });
            write_json(&root.join(".claude-plugin/marketplace.json"), &marketplace)?;
        }
    }
    Ok(())
}

fn rewrite_hook_commands(hooks: &mut Value, command: &str) -> Result<(), String> {
    let events = hooks
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "bundled hook configuration is missing hooks".to_owned())?;
    for groups in events.values_mut() {
        let groups = groups
            .as_array_mut()
            .ok_or_else(|| "bundled hook event is not an array".to_owned())?;
        for group in groups {
            let handlers = group
                .get_mut("hooks")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| "bundled hook group is missing handlers".to_owned())?;
            for handler in handlers {
                handler["command"] = Value::String(command.to_owned());
            }
        }
    }
    Ok(())
}

fn codex_hook_command(helper_path: &Path) -> String {
    let path = helper_path.to_string_lossy();
    if cfg!(windows) {
        format!("\"{}\" hook --host codex", path.replace('"', "\"\""))
    } else {
        format!("{} hook --host codex", shell_single_quote(&path))
    }
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn set_codex_global_plugin_enabled(enabled: bool) -> Result<(), String> {
    let path = codex_home()?.join("config.toml");
    edit_toml(&path, |document| {
        ensure_table_path(document, &["plugins", "ley-memory@ley-desktop"])?;
        document["plugins"]["ley-memory@ley-desktop"]["enabled"] = value(enabled);
        Ok(())
    })
}

fn write_codex_project_config(project_path: &Path, helper_path: &Path) -> Result<(), String> {
    let path = project_path.join(".codex/config.toml");
    edit_toml(&path, |document| {
        ensure_table_path(document, &["plugins", "ley-memory@ley-desktop"])?;
        document["plugins"]["ley-memory@ley-desktop"]["enabled"] = value(true);

        ensure_table_path(document, &["mcp_servers", "ley"])?;
        let server = document["mcp_servers"]["ley"]
            .as_table_mut()
            .ok_or_else(|| "existing Codex mcp_servers.ley is not a table".to_owned())?;
        if let Some(existing) = server.get("command").and_then(Item::as_str) {
            if existing != helper_path.to_string_lossy() {
                return Err(
                    "this project already has a different Codex MCP server named 'ley'; Ley did not overwrite it"
                        .to_owned(),
                );
            }
        }
        server["command"] = value(helper_path.to_string_lossy().into_owned());
        let mut args = Array::new();
        args.push("mcp");
        args.push(project_path.to_string_lossy().into_owned());
        args.push("--allow-session-writes");
        server["args"] = value(args);
        Ok(())
    })
}

fn codex_project_config_is_managed(project_path: &Path, helper_path: &Path) -> bool {
    let canonical_project = project_path
        .canonicalize()
        .unwrap_or_else(|_| project_path.to_path_buf());
    let path = canonical_project.join(".codex/config.toml");
    let Ok(source) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(document) = source.parse::<DocumentMut>() else {
        return false;
    };
    codex_mcp_binding_is_managed(&document, &canonical_project, helper_path).unwrap_or(false)
}

fn codex_mcp_binding_is_managed(
    document: &DocumentMut,
    project_path: &Path,
    helper_path: &Path,
) -> Result<bool, String> {
    let Some(server) = document
        .get("mcp_servers")
        .and_then(Item::as_table)
        .and_then(|servers| servers.get("ley"))
    else {
        return Ok(false);
    };
    let server = server
        .as_table()
        .ok_or_else(|| "existing Codex mcp_servers.ley is not a table".to_owned())?;
    let command = server
        .get("command")
        .and_then(Item::as_str)
        .ok_or_else(|| "existing Codex mcp_servers.ley has no string command".to_owned())?;
    if command != helper_path.to_string_lossy() {
        return Ok(false);
    }
    let expected_args = [
        "mcp".to_owned(),
        project_path.to_string_lossy().into_owned(),
        "--allow-session-writes".to_owned(),
    ];
    let actual_args = server
        .get("args")
        .and_then(Item::as_array)
        .map(|args| {
            args.iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .flatten();
    Ok(actual_args.as_deref() == Some(expected_args.as_slice()))
}

fn remove_codex_project_config(project_path: &Path, helper_path: &Path) -> Result<bool, String> {
    let path = project_path.join(".codex/config.toml");
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("could not read {}: {error}", path.display())),
    };
    let mut document = source.parse::<DocumentMut>().map_err(|error| {
        format!(
            "could not parse {} without risking user settings: {error}",
            path.display()
        )
    })?;

    let mcp_present = document
        .get("mcp_servers")
        .and_then(Item::as_table)
        .is_some_and(|servers| servers.contains_key("ley"));
    if mcp_present && !codex_mcp_binding_is_managed(&document, project_path, helper_path)? {
        return Err(
            "this project has a Codex MCP server named 'ley' that no longer matches Ley Desktop's exact binding; Ley did not remove it"
                .to_owned(),
        );
    }

    let mut changed = false;
    if mcp_present {
        let root = document.as_table_mut();
        let remove_parent =
            if let Some(servers) = root.get_mut("mcp_servers").and_then(Item::as_table_mut) {
                changed = servers.remove("ley").is_some();
                servers.is_empty()
            } else {
                false
            };
        if remove_parent {
            root.remove("mcp_servers");
        }
    }

    {
        let root = document.as_table_mut();
        let mut remove_plugins_parent = false;
        if let Some(plugins) = root.get_mut("plugins").and_then(Item::as_table_mut) {
            let mut remove_plugin_table = false;
            if let Some(plugin) = plugins
                .get_mut("ley-memory@ley-desktop")
                .and_then(Item::as_table_mut)
            {
                if plugin.remove("enabled").is_some() {
                    changed = true;
                }
                remove_plugin_table = plugin.is_empty();
            }
            if remove_plugin_table {
                plugins.remove("ley-memory@ley-desktop");
            }
            remove_plugins_parent = plugins.is_empty();
        }
        if remove_plugins_parent {
            root.remove("plugins");
        }
    }

    if changed {
        write_text(&path, &document.to_string())?;
    }
    Ok(changed)
}

fn ensure_table_path(document: &mut DocumentMut, keys: &[&str]) -> Result<(), String> {
    let mut item: &mut Item = document.as_item_mut();
    for key in keys {
        if item.get(key).is_none() {
            item[key] = Item::Table(Table::new());
        }
        item = item
            .get_mut(key)
            .ok_or_else(|| format!("could not create TOML table {key}"))?;
        if !item.is_table() {
            return Err(format!("existing TOML key '{key}' is not a table"));
        }
    }
    Ok(())
}

fn edit_toml(
    path: &Path,
    edit: impl FnOnce(&mut DocumentMut) -> Result<(), String>,
) -> Result<(), String> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("could not read {}: {error}", path.display())),
    };
    let mut document = if source.trim().is_empty() {
        DocumentMut::new()
    } else {
        source.parse::<DocumentMut>().map_err(|error| {
            format!(
                "could not parse {} without risking user settings: {error}",
                path.display()
            )
        })?
    };
    edit(&mut document)?;
    write_text(path, &document.to_string())
}

fn codex_home() -> Result<PathBuf, String> {
    if let Some(home) = env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(home));
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .ok_or_else(|| "could not resolve the user home directory for Codex config".to_owned())?;
    Ok(PathBuf::from(home).join(".codex"))
}

fn command_stdout(executable: &Path, arguments: &[&str], cwd: Option<&Path>) -> Option<String> {
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let output = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!output.is_empty()).then_some(output)
}

fn run_host_command<'a>(
    executable: &Path,
    arguments: impl IntoIterator<Item = &'a OsStr>,
    cwd: Option<&Path>,
) -> Result<(), String> {
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command
        .output()
        .map_err(|error| format!("could not start {}: {error}", executable.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if !stderr.trim().is_empty() {
        stderr.trim()
    } else if !stdout.trim().is_empty() {
        stdout.trim()
    } else {
        "the host command failed without diagnostic output"
    };
    Err(format!(
        "{} integration command failed: {}",
        executable.display(),
        truncate(detail, 1200)
    ))
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("could not serialize Ley integration config: {error}"))?;
    write_text(path, &(text + "\n"))
}

fn write_text(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let mut file = AtomicWriteFile::open(path)
        .map_err(|error| format!("could not stage {}: {error}", path.display()))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    file.commit()
        .map_err(|error| format!("could not atomically replace {}: {error}", path.display()))?;
    Ok(())
}

fn create_private_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("could not protect {}: {error}", path.display()))?;
    Ok(())
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let candidates = executable_names(name);
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            for candidate in &candidates {
                let path = directory.join(candidate);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }

    let mut common_directories = Vec::new();
    if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
        let home = PathBuf::from(home);
        common_directories.push(home.join(".local/bin"));
        common_directories.push(home.join(".cargo/bin"));
    }
    common_directories.push(PathBuf::from("/usr/local/bin"));
    common_directories.push(PathBuf::from("/opt/homebrew/bin"));

    for directory in common_directories {
        for candidate in &candidates {
            let path = directory.join(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn executable_names(name: &str) -> Vec<OsString> {
    if cfg!(windows) {
        vec![format!("{name}.exe").into(), format!("{name}.cmd").into()]
    } else {
        vec![name.into()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_plugin_status_without_conflating_install_and_enablement() {
        let json = r#"{
          "installed": [
            {"name":"ley-memory","marketplaceName":"ley-desktop","installed":true,"enabled":false}
          ]
        }"#;
        assert_eq!(plugin_status(HostProbe::Codex, json), Some((true, false)));
        assert_eq!(
            managed_plugin_status_from_json(HostProbe::Codex, json),
            Some((true, false))
        );
    }

    #[test]
    fn parses_claude_scoped_plugin_id() {
        let json = r#"[
          {"id":"ley-memory@ley-desktop","scope":"project","enabled":true}
        ]"#;
        assert_eq!(
            plugin_status(HostProbe::ClaudeCode, json),
            Some((true, true))
        );
        assert_eq!(
            managed_plugin_status_from_json(HostProbe::ClaudeCode, json),
            Some((true, true))
        );
    }

    #[test]
    fn generated_codex_package_uses_stable_helper_and_no_ambiguous_plugin_mcp() {
        let root = tempfile::tempdir().unwrap();
        let helper = Path::new("/opt/Ley Engine/ley");
        write_managed_marketplace(HostProbe::Codex, root.path(), helper).unwrap();

        let manifest: Value = serde_json::from_str(
            &fs::read_to_string(
                root.path()
                    .join("plugins/ley-memory/.codex-plugin/plugin.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(manifest.get("mcpServers").is_none());
        let hooks =
            fs::read_to_string(root.path().join("plugins/ley-memory/hooks/hooks.json")).unwrap();
        assert!(hooks.contains("'/opt/Ley Engine/ley' hook --host codex"));
        assert!(!hooks.contains("\"ley hook --host codex\""));
    }

    #[test]
    fn codex_project_config_preserves_unrelated_settings_and_refuses_foreign_ley_server() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir_all(project.join(".codex")).unwrap();
        fs::write(
            project.join(".codex/config.toml"),
            "model = \"gpt-test\"\n[other]\nkeep = true\n",
        )
        .unwrap();
        let helper = root.path().join("engine/ley");
        write_codex_project_config(&project, &helper).unwrap();
        let configured = fs::read_to_string(project.join(".codex/config.toml")).unwrap();
        assert!(configured.contains("model = \"gpt-test\""));
        assert!(configured.contains("keep = true"));
        assert!(configured.contains("ley-memory@ley-desktop"));
        assert!(configured.contains(helper.to_string_lossy().as_ref()));

        fs::write(
            project.join(".codex/config.toml"),
            "[mcp_servers.ley]\ncommand = \"/someone/else/ley\"\n",
        )
        .unwrap();
        let error = write_codex_project_config(&project, &helper).unwrap_err();
        assert!(error.contains("did not overwrite"));
    }

    #[test]
    fn codex_disconnect_removes_only_owned_project_binding() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir_all(project.join(".codex")).unwrap();
        fs::write(
            project.join(".codex/config.toml"),
            "model = \"gpt-test\"\n[other]\nkeep = true\n",
        )
        .unwrap();
        let helper = root.path().join("engine/ley");
        write_codex_project_config(&project, &helper).unwrap();
        assert!(codex_project_config_is_managed(&project, &helper));

        let mut configured = fs::read_to_string(project.join(".codex/config.toml"))
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();
        configured["plugins"]["ley-memory@ley-desktop"]["keep_user_setting"] = value(true);
        fs::write(project.join(".codex/config.toml"), configured.to_string()).unwrap();

        assert!(remove_codex_project_config(&project, &helper).unwrap());
        assert!(!codex_project_config_is_managed(&project, &helper));
        let disconnected = fs::read_to_string(project.join(".codex/config.toml")).unwrap();
        assert!(disconnected.contains("model = \"gpt-test\""));
        assert!(disconnected.contains("keep = true"));
        assert!(disconnected.contains("keep_user_setting = true"));
        assert!(!disconnected.contains("mcp_servers.ley"));
        assert!(!disconnected.contains("enabled = true"));
    }

    #[test]
    fn codex_disconnect_refuses_foreign_or_modified_ley_server() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir_all(project.join(".codex")).unwrap();
        let helper = root.path().join("engine/ley");
        fs::write(
            project.join(".codex/config.toml"),
            format!(
                "[plugins.\"ley-memory@ley-desktop\"]\nenabled = true\n\n[mcp_servers.ley]\ncommand = {:?}\nargs = [\"mcp\", \"/different/project\", \"--allow-session-writes\"]\n",
                helper.to_string_lossy()
            ),
        )
        .unwrap();
        let before = fs::read_to_string(project.join(".codex/config.toml")).unwrap();

        let error = remove_codex_project_config(&project, &helper).unwrap_err();
        assert!(error.contains("did not remove"));
        assert_eq!(
            fs::read_to_string(project.join(".codex/config.toml")).unwrap(),
            before
        );
    }

    #[test]
    fn marketplace_registration_parser_is_scope_specific() {
        let codex = r#"{"marketplaces":[{"name":"ley-desktop"}]}"#;
        let claude = r#"[{"name":"ley-desktop"}]"#;
        assert_eq!(
            marketplace_is_registered(HostProbe::Codex, codex),
            Some(true)
        );
        assert_eq!(
            marketplace_is_registered(HostProbe::ClaudeCode, claude),
            Some(true)
        );
    }
}
