use ley_core::{
    checkpoint_session, diagnose_project, finish_session, generate_request_id, ingest_project,
    initialize_project, list_sessions, preview_capture, read_project_graph, read_session,
    start_session, BindingRegistry, CaptureMode, CheckpointInput, CommandInput, FinishSessionInput,
    GraphNodeKind, LeyCoreError, SessionSource, SessionSourceKind, SessionStatus,
    StartSessionInput, VerificationInput, VerificationStatus,
};
use ley_mcp::run_stdio;
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("ley: {error}");
        std::process::exit(1);
    }
}

fn run(arguments: Vec<String>) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };
    match command {
        "init" => initialize(&arguments[1..]),
        "bind" => bind(&arguments[1..]),
        "binding" => binding(&arguments[1..]),
        "unbind" => unbind(&arguments[1..]),
        "ingest" => ingest(&arguments[1..]),
        "graph" => graph(&arguments[1..]),
        "mcp" => mcp(&arguments[1..]),
        "session" => session(&arguments[1..]),
        "doctor" => doctor(&arguments[1..]),
        "preview" => preview(&arguments[1..]),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "--version" | "-V" => {
            println!("ley {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(CliError::Usage(format!("unknown command '{other}'"))),
    }
}

fn mcp(arguments: &[String]) -> Result<(), CliError> {
    let parsed = binding_arguments(arguments, false)?;
    if parsed.json {
        return Err(CliError::Usage(
            "mcp uses stdout for the protocol and does not support --json".to_owned(),
        ));
    }
    let registry = BindingRegistry::system_default()?;
    let binding = registry.resolve(&parsed.project, parsed.vault.as_deref())?;
    run_stdio(parsed.project, binding.vault_path).map_err(CliError::Mcp)
}

fn session(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "session requires start, checkpoint, finish, list, or show".to_owned(),
        ));
    };
    match command {
        "start" => session_start(&arguments[1..]),
        "checkpoint" => session_checkpoint(&arguments[1..]),
        "finish" => session_finish(&arguments[1..]),
        "list" => session_list(&arguments[1..]),
        "show" => session_show(&arguments[1..]),
        other => Err(CliError::Usage(format!(
            "unknown session command '{other}'"
        ))),
    }
}

fn session_start(arguments: &[String]) -> Result<(), CliError> {
    let mut common = SessionArguments::default();
    let mut name = None;
    let mut goal = None;
    let mut request_id = None;
    let mut host = None;
    let mut agent = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--name" => {
                index += 1;
                name = Some(required_value(arguments, index, "--name")?.to_owned());
            }
            "--goal" => {
                index += 1;
                goal = Some(required_value(arguments, index, "--goal")?.to_owned());
            }
            "--request-id" => {
                index += 1;
                request_id = Some(required_value(arguments, index, "--request-id")?.to_owned());
            }
            "--host" => {
                index += 1;
                host = Some(required_value(arguments, index, "--host")?.to_owned());
            }
            "--agent" => {
                index += 1;
                agent = Some(required_value(arguments, index, "--agent")?.to_owned());
            }
            value => parse_session_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let binding = resolve_session_binding(&common)?;
    let result = start_session(
        &common.project_path()?,
        &binding.vault_path,
        StartSessionInput {
            request_id: request_id.unwrap_or_else(generate_request_id),
            name: name
                .ok_or_else(|| CliError::Usage("session start requires --name".to_owned()))?,
            goal: goal
                .ok_or_else(|| CliError::Usage("session start requires --goal".to_owned()))?,
            source: SessionSource {
                kind: if host.is_some() || agent.is_some() {
                    SessionSourceKind::HostHook
                } else {
                    SessionSourceKind::ManualCli
                },
                host,
                agent,
            },
        },
    )?;
    print_session_mutation(&result, common.json, "Started")
}

fn session_checkpoint(arguments: &[String]) -> Result<(), CliError> {
    let mut common = SessionArguments::default();
    let mut session_id = None;
    let mut summary = None;
    let mut request_id = None;
    let mut data = None;
    let mut touched_artifacts = Vec::new();
    let mut commands = Vec::new();
    let mut verification = Vec::new();
    let mut unresolved = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--summary" => {
                index += 1;
                summary = Some(required_value(arguments, index, "--summary")?.to_owned());
            }
            "--request-id" => {
                index += 1;
                request_id = Some(required_value(arguments, index, "--request-id")?.to_owned());
            }
            "--data" => {
                index += 1;
                data = Some(PathBuf::from(required_value(arguments, index, "--data")?));
            }
            "--touched" => {
                index += 1;
                touched_artifacts.push(required_value(arguments, index, "--touched")?.to_owned());
            }
            "--command" => {
                index += 1;
                commands.push(CommandInput {
                    command: required_value(arguments, index, "--command")?.to_owned(),
                    exit_code: None,
                    summary: String::new(),
                });
            }
            "--verification-passed" | "--verification-failed" => {
                let passed = arguments[index] == "--verification-passed";
                index += 1;
                verification.push(VerificationInput {
                    kind: "manual".to_owned(),
                    status: if passed {
                        VerificationStatus::Passed
                    } else {
                        VerificationStatus::Failed
                    },
                    summary: required_value(
                        arguments,
                        index,
                        if passed {
                            "--verification-passed"
                        } else {
                            "--verification-failed"
                        },
                    )?
                    .to_owned(),
                    command: None,
                });
            }
            "--unresolved" => {
                index += 1;
                unresolved.push(required_value(arguments, index, "--unresolved")?.to_owned());
            }
            value if session_id.is_none() && value.starts_with("ses_") => {
                session_id = Some(value.to_owned())
            }
            value => parse_session_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let session_id = session_id
        .ok_or_else(|| CliError::Usage("session checkpoint requires SESSION".to_owned()))?;
    let input = if let Some(path) = data {
        if summary.is_some()
            || request_id.is_some()
            || !touched_artifacts.is_empty()
            || !commands.is_empty()
            || !verification.is_empty()
            || !unresolved.is_empty()
        {
            return Err(CliError::Usage(
                "--data cannot be combined with checkpoint content flags".to_owned(),
            ));
        }
        read_bounded_json(&path)?
    } else {
        CheckpointInput {
            request_id: request_id.unwrap_or_else(generate_request_id),
            summary: summary.ok_or_else(|| {
                CliError::Usage("session checkpoint requires --summary or --data".to_owned())
            })?,
            plan: Vec::new(),
            decisions: Vec::new(),
            tasks: Vec::new(),
            problems: Vec::new(),
            touched_artifacts,
            commands,
            verification,
            unresolved,
        }
    };
    let binding = resolve_session_binding(&common)?;
    let result = checkpoint_session(
        &common.project_path()?,
        &binding.vault_path,
        &session_id,
        input,
    )?;
    print_session_mutation(&result, common.json, "Checkpointed")
}

fn session_finish(arguments: &[String]) -> Result<(), CliError> {
    let mut common = SessionArguments::default();
    let mut session_id = None;
    let mut summary = None;
    let mut request_id = None;
    let mut status = SessionStatus::Completed;
    let mut final_response = String::new();
    let mut handoff = String::new();
    let mut unresolved = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--summary" => {
                index += 1;
                summary = Some(required_value(arguments, index, "--summary")?.to_owned());
            }
            "--request-id" => {
                index += 1;
                request_id = Some(required_value(arguments, index, "--request-id")?.to_owned());
            }
            "--status" => {
                index += 1;
                status = parse_finished_status(required_value(arguments, index, "--status")?)?;
            }
            "--final-response" => {
                index += 1;
                final_response = required_value(arguments, index, "--final-response")?.to_owned();
            }
            "--handoff" => {
                index += 1;
                handoff = required_value(arguments, index, "--handoff")?.to_owned();
            }
            "--unresolved" => {
                index += 1;
                unresolved.push(required_value(arguments, index, "--unresolved")?.to_owned());
            }
            value if session_id.is_none() && value.starts_with("ses_") => {
                session_id = Some(value.to_owned())
            }
            value => parse_session_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let session_id =
        session_id.ok_or_else(|| CliError::Usage("session finish requires SESSION".to_owned()))?;
    let binding = resolve_session_binding(&common)?;
    let result = finish_session(
        &common.project_path()?,
        &binding.vault_path,
        &session_id,
        FinishSessionInput {
            request_id: request_id.unwrap_or_else(generate_request_id),
            status,
            summary: summary
                .ok_or_else(|| CliError::Usage("session finish requires --summary".to_owned()))?,
            final_response,
            handoff,
            unresolved,
        },
    )?;
    print_session_mutation(&result, common.json, "Finished")
}

fn session_list(arguments: &[String]) -> Result<(), CliError> {
    let common = parse_session_read_arguments(arguments, false)?;
    let binding = resolve_session_binding(&common)?;
    let sessions = list_sessions(&common.project_path()?, &binding.vault_path)?;
    if common.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&sessions).expect("sessions are serializable")
        );
    } else if sessions.is_empty() {
        println!("No agent sessions");
    } else {
        for session in sessions {
            println!(
                "{}  {:<10}  {}  ({} checkpoints)",
                session.session_id,
                session_status_label(session.status),
                session.name,
                session.checkpoints
            );
        }
    }
    Ok(())
}

fn session_show(arguments: &[String]) -> Result<(), CliError> {
    let common = parse_session_read_arguments(arguments, true)?;
    let session_id = common
        .session_id
        .as_deref()
        .expect("show validation requires a session ID");
    let binding = resolve_session_binding(&common)?;
    let session = read_session(&common.project_path()?, &binding.vault_path, session_id)?;
    if common.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&session).expect("session is serializable")
        );
    } else {
        println!("Session: {} ({})", session.name, session.session_id);
        println!("Status: {}", session_status_label(session.status));
        println!("Goal: {}", session.goal);
        println!("Events: {}", session.event_count);
        println!("Checkpoints: {}", session.checkpoints.len());
        if let Some(finish) = session.finish {
            println!("Result: {}", finish.summary);
            if !finish.handoff.is_empty() {
                println!("Handoff: {}", finish.handoff);
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct SessionArguments {
    project: Option<PathBuf>,
    vault: Option<PathBuf>,
    session_id: Option<String>,
    json: bool,
}

impl SessionArguments {
    fn project_path(&self) -> Result<PathBuf, CliError> {
        self.project
            .clone()
            .map(Ok)
            .unwrap_or_else(|| env::current_dir().map_err(CliError::CurrentDirectory))
    }
}

fn parse_session_common(
    arguments: &[String],
    index: &mut usize,
    value: &str,
    common: &mut SessionArguments,
) -> Result<(), CliError> {
    match value {
        "--vault" => {
            *index += 1;
            common.vault = Some(PathBuf::from(required_value(arguments, *index, "--vault")?));
        }
        "--json" => common.json = true,
        value if value.starts_with('-') => {
            return Err(CliError::Usage(format!("unknown option '{value}'")))
        }
        value if common.project.is_none() => common.project = Some(PathBuf::from(value)),
        value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
    }
    Ok(())
}

fn parse_session_read_arguments(
    arguments: &[String],
    session_required: bool,
) -> Result<SessionArguments, CliError> {
    let mut common = SessionArguments::default();
    let mut index = 0;
    while index < arguments.len() {
        let value = arguments[index].as_str();
        if session_required && common.session_id.is_none() && value.starts_with("ses_") {
            common.session_id = Some(value.to_owned());
        } else {
            parse_session_common(arguments, &mut index, value, &mut common)?;
        }
        index += 1;
    }
    if session_required && common.session_id.is_none() {
        return Err(CliError::Usage("session show requires SESSION".to_owned()));
    }
    Ok(common)
}

fn resolve_session_binding(
    common: &SessionArguments,
) -> Result<ley_core::ProjectVaultBinding, CliError> {
    let registry = BindingRegistry::system_default()?;
    Ok(registry.resolve(common.project_path()?, common.vault.as_deref())?)
}

fn parse_finished_status(value: &str) -> Result<SessionStatus, CliError> {
    match value {
        "completed" => Ok(SessionStatus::Completed),
        "paused" => Ok(SessionStatus::Paused),
        "abandoned" => Ok(SessionStatus::Abandoned),
        other => Err(CliError::Usage(format!(
            "invalid finish status '{other}'; use completed, paused, or abandoned"
        ))),
    }
}

fn session_status_label(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Active => "active",
        SessionStatus::Completed => "completed",
        SessionStatus::Paused => "paused",
        SessionStatus::Abandoned => "abandoned",
    }
}

fn read_bounded_json(path: &Path) -> Result<CheckpointInput, CliError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| CliError::InputFile {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 1_048_576 {
        return Err(CliError::Usage(
            "--data must name a regular non-symlink JSON file no larger than 1 MiB".to_owned(),
        ));
    }
    let bytes = std::fs::read(path).map_err(|source| CliError::InputFile {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| CliError::Usage(format!("invalid checkpoint JSON: {error}")))
}

fn print_session_mutation(
    result: &ley_core::SessionMutation,
    json: bool,
    verb: &str,
) -> Result<(), CliError> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(result).expect("session mutation is serializable")
        );
    } else {
        println!("{verb} session: {}", result.session.session_id);
        println!("Name: {}", result.session.name);
        println!("Status: {}", session_status_label(result.session.status));
        println!("Events: {}", result.session.event_count);
        println!(
            "Write: {}",
            if result.replayed {
                "idempotent replay"
            } else {
                "recorded"
            }
        );
        println!("Projection: {}", result.session_path);
        println!("Markdown: {}", result.markdown_path);
    }
    Ok(())
}

fn ingest(arguments: &[String]) -> Result<(), CliError> {
    let parsed = binding_arguments(arguments, false)?;
    let registry = BindingRegistry::system_default()?;
    let binding = registry.resolve(&parsed.project, parsed.vault.as_deref())?;
    let result = ingest_project(&parsed.project, &binding.vault_path)?;
    if parsed.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "binding": binding,
                "ingestion": result,
            }))
            .expect("CLI result is serializable")
        );
    } else {
        println!("Ingested project: {}", result.project_id);
        println!("Snapshot: {}", result.snapshot_id);
        println!(
            "Vault: {} ({})",
            binding.vault_path.display(),
            binding.source
        );
        println!(
            "Artifacts: {} files / {} stored / {} redacted / {} skipped",
            result.files,
            result.stored_files,
            result.redacted_files,
            result.skipped.len()
        );
        println!(
            "Graph: {} nodes / {} edges / {}",
            result.graph_nodes,
            result.graph_edges,
            if result.graph_changed {
                "updated"
            } else {
                "unchanged"
            }
        );
        println!("Graph snapshot: {}", result.graph_snapshot_id);
        if result.changed {
            println!(
                "Changes: {} added / {} modified / {} renamed / {} deleted",
                result.added.len(),
                result.modified.len(),
                result.renamed.len(),
                result.deleted.len()
            );
            println!("Manifest: {}", result.manifest_path);
        } else {
            println!("No source changes; the durable snapshot was left untouched");
        }
    }
    Ok(())
}

fn graph(arguments: &[String]) -> Result<(), CliError> {
    let parsed = binding_arguments(arguments, false)?;
    let registry = BindingRegistry::system_default()?;
    let binding = registry.resolve(&parsed.project, parsed.vault.as_deref())?;
    let graph = read_project_graph(&parsed.project, &binding.vault_path)?;
    if parsed.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&graph).expect("project graph is serializable")
        );
        return Ok(());
    }

    let symbols = graph
        .nodes
        .iter()
        .filter(|node| node.kind == GraphNodeKind::Symbol)
        .count();
    let dependencies = graph
        .nodes
        .iter()
        .filter(|node| node.kind == GraphNodeKind::Dependency)
        .count();
    println!("Project graph: {}", graph.project_name);
    println!("Snapshot: {}", graph.graph_snapshot_id);
    println!("Source snapshot: {}", graph.artifact_snapshot_id);
    println!(
        "Nodes: {} total / {} symbols / {} dependencies",
        graph.nodes.len(),
        symbols,
        dependencies
    );
    println!("Edges: {}", graph.edges.len());
    println!("Diagnostics: {}", graph.diagnostics.len());
    if let Some(git) = &graph.git {
        println!(
            "Git: {} @ {} / {} tracked changes",
            git.branch.as_deref().unwrap_or("detached"),
            git.head
                .as_deref()
                .map(|head| &head[..head.len().min(12)])
                .unwrap_or("unborn"),
            git.changes.len()
        );
    } else {
        println!("Git: not a repository or Git is unavailable");
    }
    println!("Vault: {}", binding.vault_path.display());
    Ok(())
}

fn bind(arguments: &[String]) -> Result<(), CliError> {
    let parsed = binding_arguments(arguments, true)?;
    let registry = BindingRegistry::system_default()?;
    let vault = parsed
        .vault
        .expect("binding argument validation requires a vault");
    let result = registry.bind(parsed.project, vault)?;
    if parsed.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("CLI result is serializable")
        );
    } else {
        println!("Bound project: {}", result.project_id);
        println!("Vault: {}", result.vault_path.display());
        println!("Private registry: {}", registry.path().display());
    }
    Ok(())
}

fn binding(arguments: &[String]) -> Result<(), CliError> {
    let parsed = binding_arguments(arguments, false)?;
    let registry = BindingRegistry::system_default()?;
    let result = registry.resolve(parsed.project, parsed.vault.as_deref())?;
    if parsed.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("CLI result is serializable")
        );
    } else {
        println!("Project: {}", result.project_id);
        println!("Vault: {}", result.vault_path.display());
        println!("Source: {}", result.source);
    }
    Ok(())
}

fn unbind(arguments: &[String]) -> Result<(), CliError> {
    let mut project = None;
    let mut json = false;
    for argument in arguments {
        match argument.as_str() {
            "--json" => json = true,
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option '{value}'")))
            }
            value if project.is_none() => project = Some(PathBuf::from(value)),
            value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
        }
    }
    let project = project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
    let registry = BindingRegistry::system_default()?;
    let removed = registry.unbind(project)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "removed": removed.is_some(),
                "binding": removed,
            }))
            .expect("CLI result is serializable")
        );
    } else if let Some(binding) = removed {
        println!("Unbound project: {}", binding.project_id);
        println!("Former vault: {}", binding.vault_path.display());
    } else {
        println!("Project was not bound");
    }
    Ok(())
}

struct BindingArguments {
    project: PathBuf,
    vault: Option<PathBuf>,
    json: bool,
}

fn binding_arguments(
    arguments: &[String],
    vault_required: bool,
) -> Result<BindingArguments, CliError> {
    let mut project = None;
    let mut vault = None;
    let mut json = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--vault" => {
                index += 1;
                vault = Some(PathBuf::from(required_value(arguments, index, "--vault")?));
            }
            "--json" => json = true,
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option '{value}'")))
            }
            value if project.is_none() => project = Some(PathBuf::from(value)),
            value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
        }
        index += 1;
    }
    if vault_required && vault.is_none() {
        return Err(CliError::Usage("bind requires --vault <path>".to_owned()));
    }
    Ok(BindingArguments {
        project: project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?),
        vault,
        json,
    })
}

fn preview(arguments: &[String]) -> Result<(), CliError> {
    let mut path = None;
    let mut json = false;
    for argument in arguments {
        match argument.as_str() {
            "--json" => json = true,
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option '{value}'")))
            }
            value if path.is_none() => path = Some(PathBuf::from(value)),
            value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
        }
    }
    let start = path.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
    let result = preview_capture(start)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("CLI result is serializable")
        );
    } else {
        println!("Capture preview: {}", result.project_id);
        println!("Mode: {}", result.mode);
        println!(
            "Included: {} files / {} bytes",
            result.files.len(),
            result.included_bytes
        );
        for file in &result.files {
            println!("  {} ({} bytes)", file.path, file.bytes);
        }
        println!("Oversized: {}", result.skipped_oversized.len());
        println!("Over total limit: {}", result.skipped_total_limit.len());
        println!("Symlinks skipped: {}", result.skipped_symlinks.len());
    }
    Ok(())
}

fn initialize(arguments: &[String]) -> Result<(), CliError> {
    let mut path = None;
    let mut name = None;
    let mut capture = CaptureMode::Structured;
    let mut json = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--name" => {
                index += 1;
                name = Some(required_value(arguments, index, "--name")?.to_owned());
            }
            "--capture" => {
                index += 1;
                capture = CaptureMode::parse(required_value(arguments, index, "--capture")?)?;
            }
            "--json" => json = true,
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option '{value}'")))
            }
            value if path.is_none() => path = Some(PathBuf::from(value)),
            value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
        }
        index += 1;
    }
    let root = path.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
    let result = initialize_project(&root, name.as_deref(), capture)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("CLI result is serializable")
        );
    } else if result.created {
        println!(
            "Initialized {} ({})",
            result.identity.name, result.identity.project_id
        );
        println!("Capture: {}", result.capture.mode);
        println!("Created: {}/.ley", result.root.display());
    } else {
        println!(
            "Already initialized: {} ({})",
            result.identity.name, result.identity.project_id
        );
        println!("Capture remains: {}", result.capture.mode);
    }
    Ok(())
}

fn doctor(arguments: &[String]) -> Result<(), CliError> {
    let mut path = None;
    let mut json = false;
    for argument in arguments {
        match argument.as_str() {
            "--json" => json = true,
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option '{value}'")))
            }
            value if path.is_none() => path = Some(PathBuf::from(value)),
            value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
        }
    }
    let start = path.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
    let result = diagnose_project(start)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("CLI result is serializable")
        );
    } else {
        println!("Ley project: {}", result.identity.name);
        println!("Project ID: {}", result.identity.project_id);
        println!("Root: {}", result.root.display());
        println!("Capture: {}", result.capture.mode);
        println!(
            "Raw transcripts: {}",
            if result.capture.store_raw_transcripts {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!(
            "Ignore rules: {}",
            if result.ignore_file_present {
                "present"
            } else {
                "missing"
            }
        );
    }
    Ok(())
}

fn required_value<'a>(
    arguments: &'a [String],
    index: usize,
    option: &str,
) -> Result<&'a str, CliError> {
    arguments
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage(format!("{option} requires a value")))
}

fn print_help() {
    println!("Ley local project memory");
    println!();
    println!("Usage:");
    println!("  ley init [path] [--name NAME] [--capture minimal|structured|full] [--json]");
    println!("  ley bind [path] --vault VAULT [--json]");
    println!("  ley binding [path] [--vault TEMPORARY_VAULT] [--json]");
    println!("  ley unbind [path] [--json]");
    println!("  ley ingest [path] [--vault TEMPORARY_VAULT] [--json]");
    println!("  ley graph [path] [--vault TEMPORARY_VAULT] [--json]");
    println!("  ley mcp [path] [--vault TEMPORARY_VAULT]");
    println!("  ley session start [path] --name NAME --goal GOAL [--host HOST] [--agent AGENT]");
    println!("  ley session checkpoint SESSION [path] --summary TEXT [--touched PATH]...");
    println!("  ley session checkpoint SESSION [path] --data CHECKPOINT.json");
    println!("  ley session finish SESSION [path] --summary TEXT [--status STATUS]");
    println!("  ley session list [path] [--json]");
    println!("  ley session show SESSION [path] [--json]");
    println!("  ley doctor [path] [--json]");
    println!("  ley preview [path] [--json]");
    println!();
    println!(
        "Structured capture is the default. Full evidence explicitly enables raw transcripts."
    );
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Core(LeyCoreError),
    Mcp(ley_mcp::McpServerError),
    CurrentDirectory(std::io::Error),
    InputFile {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}; run 'ley help'"),
            Self::Core(error) => error.fmt(formatter),
            Self::Mcp(error) => error.fmt(formatter),
            Self::CurrentDirectory(error) => {
                write!(formatter, "could not read current directory: {error}")
            }
            Self::InputFile { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
        }
    }
}

impl From<LeyCoreError> for CliError {
    fn from(value: LeyCoreError) -> Self {
        Self::Core(value)
    }
}
