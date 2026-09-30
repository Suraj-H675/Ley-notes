use ley_core::{
    checkpoint_session_with_continuity_transition, consolidation_inbox_with_continuity_transition,
    correct_learning_with_continuity_transition, diagnose_project,
    erase_session_memory_with_continuity_transition, establish_native_born_project_authorities,
    finish_session_with_continuity_transition, generate_learning_request_id, generate_request_id,
    import_codex_message_history_with_continuity_transition,
    ingest_project_with_continuity_transition, ingest_project_with_native_authority,
    initialize_project_retiring_bootstrap, learning_review_inbox_with_continuity_transition,
    list_learnings_with_continuity_transition, list_sessions_with_continuity_transition,
    native_born_project_registration_exists, prepare_legacy_project_binding, preview_capture,
    process_bootstrap_host_hook_for_agent_with_transition_registries,
    process_host_hook_for_agent_with_transition_registries,
    project_resume_context_with_continuity_transition, propose_learning_with_continuity_transition,
    read_external_connector_snapshot_with_registry, read_learning_with_continuity_transition,
    read_session_context_with_continuity_transition,
    read_session_turns_context_with_continuity_transition, read_session_with_continuity_transition,
    record_session_prompt_with_continuity_transition,
    record_session_response_with_continuity_transition, register_native_born_project,
    remove_external_connector_with_registry, rename_session_with_continuity_transition,
    review_learning_with_continuity_transition, search_project_memory_with_continuity_transition,
    semantic_model_status, start_session_with_continuity_transition, supported_semantic_model,
    AgentEgressPolicy, AgentEgressTarget, AgentHost, ApprovedSourceRegistry, BindingRegistry,
    BindingSource, BootstrapSpecificationRegistry, CaptureMode, CheckpointInput, CommandInput,
    ConsolidationInboxLimits, ContextMountRegistry, ContinuityStore, CorrectLearningInput,
    EgressPolicyRegistry, EraseSessionMemoryInput, ExternalConnectorRegistry, FinishSessionInput,
    HostAgentContextRegistries, KnowledgeScopeRegistry, LearningActor, LearningEvidenceInput,
    LearningFeedbackAction, LearningKind, LearningProvenance, LearningState, LearningTrustState,
    LeyCoreError, PolicyBundleRegistry, ProjectCatalog, ProjectMemorySearchLimits,
    ProjectVaultBinding, ProposeLearningInput, RenameSessionInput, ReviewLearningInput,
    RevisionCompatibility, SemanticModelStatus, SessionSource, SessionSourceKind, SessionStatus,
    SessionWriteResult, SpecificationRegistry, StartSessionInput, TurnEvidenceInput,
    TurnEvidenceOrigin, VerificationInput, VerificationStatus, DEFAULT_CONSOLIDATION_INBOX_ITEMS,
    DEFAULT_CONSOLIDATION_INBOX_SESSIONS, DEFAULT_PROJECT_MEMORY_SEARCH_RESULTS,
    DEFAULT_PROJECT_MEMORY_SEARCH_TOKENS, DEFAULT_RESUME_CHARACTERS, DEFAULT_RESUME_LEARNINGS,
    DEFAULT_RESUME_SESSIONS, DEFAULT_SESSION_CONTEXT_CHARACTERS,
    DEFAULT_SESSION_CONTEXT_CHECKPOINTS,
};
use ley_mcp::{
    run_bootstrap_stdio_with_egress_target, run_stdio_with_egress_target, run_unavailable_stdio,
};
use ley_semantic_installer::{
    install_supported_semantic_model_with_progress, SemanticModelInstallerError,
};
use std::env;
use std::io::Read;
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
        "hook" => hook(&arguments[1..]),
        "mcp" => mcp(&arguments[1..]),
        "egress" => egress(&arguments[1..]),
        "connector" => connector(&arguments[1..]),
        "bootstrap-spec" => bootstrap_specification(&arguments[1..]),
        "bootstrap-ref" => bootstrap_reference(&arguments[1..]),
        "mount" => mount(&arguments[1..]),
        "scope" => scope(&arguments[1..]),
        "policy-bundle" => policy_bundle(&arguments[1..]),
        "session" => session(&arguments[1..]),
        "consolidation" => consolidation(&arguments[1..]),
        "learning" => learning(&arguments[1..]),
        "resume" => resume(&arguments[1..]),
        "search" => search(&arguments[1..]),
        "semantic" => semantic(&arguments[1..]),
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

fn semantic(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "semantic requires status or install".to_owned(),
        ));
    };
    let mut json = false;
    for argument in &arguments[1..] {
        match argument.as_str() {
            "--json" => json = true,
            value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
        }
    }
    match command {
        "status" => print_semantic_status(semantic_model_status(), json),
        "install" => semantic_install(json),
        other => Err(CliError::Usage(format!(
            "unknown semantic command '{other}'"
        ))),
    }
}

fn print_semantic_status(status: SemanticModelStatus, json: bool) -> Result<(), CliError> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&status).expect("semantic status is serializable")
        );
        return Ok(());
    }
    match status {
        SemanticModelStatus::Ready { model } => {
            println!("Semantic retrieval: ready");
            println!("Model: {}", model.model_id);
            println!("Revision: {}", model.revision);
            println!("Dimension: {}", model.dimension);
            println!("Privacy: local inference only; normal search performs no network requests");
        }
        SemanticModelStatus::Uninstalled { reason } => {
            println!("Semantic retrieval: not installed");
            println!("Reason: {reason}");
            println!("Run 'ley semantic install' to explicitly download the pinned local model.");
        }
        SemanticModelStatus::Corrupt { reason } => {
            println!("Semantic retrieval: corrupt");
            println!("Reason: {reason}");
            println!("Ley will use lexical retrieval until the local model is repaired.");
        }
    }
    Ok(())
}

fn semantic_install(json: bool) -> Result<(), CliError> {
    if let SemanticModelStatus::Ready { model } = semantic_model_status() {
        if json {
            println!(
                "{}",
                serde_json::json!({ "model": model, "installed": false })
            );
        } else {
            println!("Semantic retrieval is already installed and verified.");
        }
        return Ok(());
    }

    let model = supported_semantic_model();
    if !json {
        let bytes = model.files.iter().map(|file| file.bytes).sum::<u64>();
        println!(
            "Installing {} ({} MiB, pinned revision {})",
            model.model_id,
            (bytes + 1_048_575) / 1_048_576,
            model.revision
        );
        println!("Downloads are explicit; inference and search remain fully local.");
    }
    let result = install_supported_semantic_model_with_progress(|file| {
        if !json {
            println!("Downloading {} ({} bytes)…", file.name, file.bytes);
        }
    })
    .map_err(|error| match error {
        SemanticModelInstallerError::Core(error) => CliError::Core(error),
        error => CliError::ModelDownload(error.to_string()),
    })?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("semantic installation is serializable")
        );
    } else {
        println!("Semantic retrieval installed and checksum-verified.");
        println!("No project content or query was uploaded.");
    }
    Ok(())
}

fn egress(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "egress requires list, project, specification, mount, or connector".to_owned(),
        ));
    };
    let registry = EgressPolicyRegistry::system_default()?;
    match command {
        "list" => {
            let mut project = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if project.is_none() => project = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let project =
                project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let store = ContinuityStore::system_default()?;
            let result = registry.list_transition(&project, &store)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).expect("egress policy is serializable")
                );
            } else {
                println!("Project egress: {}", result.project_policy);
                if result.specification_overrides.is_empty() {
                    println!("Specification overrides: none");
                } else {
                    println!("Specification overrides:");
                    for item in &result.specification_overrides {
                        println!("  {}  {}", item.scope_id, item.policy);
                    }
                }
                if result.mount_overrides.is_empty() {
                    println!("Context Mount overrides: none");
                } else {
                    println!("Context Mount overrides:");
                    for item in &result.mount_overrides {
                        println!("  {}  {}", item.scope_id, item.policy);
                    }
                }
                if result.connector_overrides.is_empty() {
                    println!("External connector overrides: none");
                } else {
                    println!("External connector overrides:");
                    for item in &result.connector_overrides {
                        println!("  {}  {}", item.scope_id, item.policy);
                    }
                }
                println!("Privacy: {}", result.privacy_notice);
            }
            Ok(())
        }
        "project" => {
            let (policy, project, json) = parse_egress_scope_arguments(
                &arguments[1..],
                "egress project requires POLICY [PROJECT]",
            )?;
            let project =
                project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let store = ContinuityStore::system_default()?;
            let result = registry.set_project_policy_transition(&project, &store, policy)?;
            print_egress_mutation(&result, json)
        }
        "specification" => {
            let specification_id = arguments.get(1).ok_or_else(|| {
                CliError::Usage(
                    "egress specification requires SPECIFICATION_ID POLICY [PROJECT]".to_owned(),
                )
            })?;
            let (policy, project, json) = parse_egress_scope_arguments(
                &arguments[2..],
                "egress specification requires SPECIFICATION_ID POLICY [PROJECT]",
            )?;
            require_legacy_scope_egress_cleanup("Specification", policy)?;
            let project =
                project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let retained_override =
                registry.has_specification_override(&project, specification_id)?;
            let approved = match approved_source_exists_for_cleanup(&project, specification_id) {
                Ok(approved) => approved,
                Err(_) if retained_override => false,
                Err(error) => return Err(error),
            };
            if !approved && !retained_override {
                return Err(CliError::Usage(format!(
                    "Approved source {specification_id} is neither current authority nor retained by legacy egress policy for this project"
                )));
            }
            let result = registry.set_specification_policy(&project, specification_id, policy)?;
            print_egress_mutation(&result, json)
        }
        "mount" => {
            let mount_id = arguments.get(1).ok_or_else(|| {
                CliError::Usage("egress mount requires MOUNT_ID POLICY [PROJECT]".to_owned())
            })?;
            let (policy, project, json) = parse_egress_scope_arguments(
                &arguments[2..],
                "egress mount requires MOUNT_ID POLICY [PROJECT]",
            )?;
            require_legacy_scope_egress_cleanup("Context Mount", policy)?;
            let project =
                project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let mounts = ContextMountRegistry::system_default()?;
            let known_mount = mounts.contains_mount_or_history(&project, mount_id)?;
            let retained_override = registry.has_mount_override(&project, mount_id)?;
            if !known_mount && !retained_override {
                return Err(CliError::Usage(format!(
                    "Context Mount {mount_id} is neither current/historical nor retained by egress policy for this project"
                )));
            }
            let result = registry.set_mount_policy(&project, mount_id, policy)?;
            print_egress_mutation(&result, json)
        }
        "connector" => {
            let connector_id = arguments.get(1).ok_or_else(|| {
                CliError::Usage(
                    "egress connector requires CONNECTOR_ID POLICY [PROJECT]".to_owned(),
                )
            })?;
            let (policy, project, json) = parse_egress_scope_arguments(
                &arguments[2..],
                "egress connector requires CONNECTOR_ID POLICY [PROJECT]",
            )?;
            require_legacy_scope_egress_cleanup("External Connector", policy)?;
            let project =
                project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let connectors = ExternalConnectorRegistry::system_default()?;
            let known_connector = connectors.contains_connector(&project, connector_id)?;
            let retained_override = registry.has_connector_override(&project, connector_id)?;
            if !known_connector && !retained_override {
                return Err(CliError::Usage(format!(
                    "External connector {connector_id} is neither current nor retained by egress policy for this project"
                )));
            }
            let result = registry.set_connector_policy(&project, connector_id, policy)?;
            print_egress_mutation(&result, json)
        }
        other => Err(CliError::Usage(format!(
            "unknown egress command '{other}'; use list, project, specification, mount, or connector"
        ))),
    }
}

fn approved_source_exists_for_cleanup(project: &Path, source_id: &str) -> Result<bool, CliError> {
    let approved_sources = ApprovedSourceRegistry::system_default()?;
    if !approved_sources.authority_ready(project)? {
        let binding = match BindingRegistry::system_default()?.resolve(project, None) {
            Ok(binding) => binding,
            Err(LeyCoreError::VaultNotBound(_))
            | Err(LeyCoreError::BoundVaultUnavailable { .. }) => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        let legacy = SpecificationRegistry::system_default()?;
        approved_sources.migrate_legacy_specifications(project, &binding.vault_path, &legacy)?;
    }
    Ok(approved_sources
        .authority(project)?
        .sources
        .iter()
        .any(|source| source.approval.source_id == source_id))
}

fn require_legacy_scope_egress_cleanup(
    scope_label: &str,
    policy: AgentEgressPolicy,
) -> Result<(), CliError> {
    if policy == AgentEgressPolicy::AgentOk {
        return Ok(());
    }
    Err(CliError::Usage(format!(
        "{scope_label} egress overrides are legacy compatibility state; only agent-ok is accepted to clear an existing override. Use 'ley egress project POLICY [PROJECT]' for new restrictions."
    )))
}

fn parse_egress_scope_arguments(
    arguments: &[String],
    usage: &str,
) -> Result<(AgentEgressPolicy, Option<PathBuf>, bool), CliError> {
    let policy = arguments
        .first()
        .ok_or_else(|| CliError::Usage(usage.to_owned()))
        .and_then(|value| AgentEgressPolicy::parse(value).map_err(CliError::Core))?;
    let mut project = None;
    let mut json = false;
    for argument in &arguments[1..] {
        match argument.as_str() {
            "--json" => json = true,
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option '{value}'")))
            }
            value if project.is_none() => project = Some(PathBuf::from(value)),
            value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
        }
    }
    Ok((policy, project, json))
}

fn print_egress_mutation(
    result: &ley_core::AgentEgressPolicyMutation,
    json: bool,
) -> Result<(), CliError> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(result).expect("egress mutation is serializable")
        );
    } else {
        println!(
            "Agent egress: {:?} {} -> {}",
            result.scope.scope_kind, result.scope.scope_id, result.scope.policy
        );
        if result.scope.policy == AgentEgressPolicy::ConfirmPerUse {
            println!(
                "confirm-per-use is fail-closed until Ley has an explicit local confirmation flow."
            );
        }
    }
    Ok(())
}

fn connector(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "connector requires list, show, or remove".to_owned(),
        ));
    };
    let registry = ExternalConnectorRegistry::system_default()?;
    match command {
        "list" => {
            let mut project = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if project.is_none() => project = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let project =
                project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let result = registry.list(&project)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result)
                        .expect("external connector list is serializable")
                );
            } else if result.connectors.is_empty() {
                println!("No external connectors.");
            } else {
                println!("External connectors: {}", result.connectors.len());
                for connector in &result.connectors {
                    println!(
                        "  {}  {:?}  {}",
                        connector.connector_id,
                        connector.source.resource_kind,
                        connector.source.canonical_url
                    );
                }
                println!("Privacy: {}", result.privacy_notice);
            }
            Ok(())
        }
        "show" => {
            let (connector_id, project, vault, json) = parse_connector_store_arguments(
                &arguments[1..],
                "connector show requires CONNECTOR_ID [PROJECT]",
            )?;
            let project =
                project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let binding = BindingRegistry::system_default()?.resolve(&project, vault.as_deref())?;
            let snapshot = read_external_connector_snapshot_with_registry(
                &project,
                &binding.vault_path,
                &registry,
                &connector_id,
            )?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&snapshot)
                        .expect("external connector snapshot is serializable")
                );
            } else {
                println!("External connector: {}", snapshot.connector_id);
                println!("Source: {}", snapshot.source.canonical_url);
                println!("Snapshot: {}", snapshot.snapshot_id);
                if let Some(state) = snapshot.state {
                    println!("State: {:?}", state);
                }
                if let Some(revision) = &snapshot.source.revision {
                    println!("Pinned revision: {}", revision);
                }
                if let Some(path) = &snapshot.source.path {
                    println!("Document path: {}", terminal_safe(path));
                }
                println!("Title: {}", terminal_safe(&snapshot.title));
                if !snapshot.body.is_empty() {
                    println!("Body:\n{}", terminal_safe(&snapshot.body));
                }
                if let Some(source_updated_at) = &snapshot.source_updated_at {
                    println!("Updated upstream: {}", terminal_safe(source_updated_at));
                }
                println!("Live source checked by this read: no");
                println!("Warning: {}", snapshot.instruction_warning);
            }
            Ok(())
        }
        "remove" => {
            let (connector_id, project, vault, json) = parse_connector_store_arguments(
                &arguments[1..],
                "connector remove requires CONNECTOR_ID [PROJECT]",
            )?;
            let project =
                project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let binding = BindingRegistry::system_default()?.resolve(&project, vault.as_deref())?;
            let removed = remove_external_connector_with_registry(
                &project,
                &binding.vault_path,
                &registry,
                &connector_id,
            )?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&removed)
                        .expect("external connector removal is serializable")
                );
            } else if removed.is_some() {
                println!("Removed external connector: {connector_id}");
                println!("Any connector-specific egress override remains retained until explicitly changed.");
            } else {
                println!("External connector not found: {connector_id}");
            }
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "unknown connector command '{other}'; use list, show, or remove"
        ))),
    }
}

fn parse_connector_store_arguments(
    arguments: &[String],
    usage: &str,
) -> Result<(String, Option<PathBuf>, Option<PathBuf>, bool), CliError> {
    let connector_id = arguments
        .first()
        .filter(|value| !value.starts_with('-'))
        .cloned()
        .ok_or_else(|| CliError::Usage(usage.to_owned()))?;
    let mut project = None;
    let mut vault = None;
    let mut json = false;
    let mut index = 1;
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
    Ok((connector_id, project, vault, json))
}

fn terminal_safe(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() && !matches!(character, '\n' | '\r' | '\t') {
                '\u{fffd}'
            } else {
                character
            }
        })
        .collect()
}

fn mount(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage("mount requires list or remove".to_owned()));
    };
    let registry = ContextMountRegistry::system_default()?;
    match command {
        "list" => {
            let mut active = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if active.is_none() => active = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let active = active.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let result = registry.list(active)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).expect("mount list is serializable")
                );
            } else if result.mounts.is_empty() {
                println!("No Context Mounts.");
            } else {
                println!("Context Mounts: {}", result.mounts.len());
                for mount in &result.mounts {
                    let name = mount
                        .source_project_name
                        .as_deref()
                        .unwrap_or(&mount.source_project_id);
                    println!(
                        "  {}  {}  {:?}  read-only  agent-context:{}",
                        mount.mount_id,
                        name,
                        mount.status,
                        if mount.agent_context_enabled {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                }
            }
            Ok(())
        }
        "remove" => {
            let mut mount_id = None;
            let mut active = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if mount_id.is_none() => mount_id = Some(value.to_owned()),
                    value if active.is_none() => active = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let mount_id = mount_id
                .ok_or_else(|| CliError::Usage("mount remove requires MOUNT_ID".to_owned()))?;
            let active = active.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let removed = registry.unmount(active, &mount_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&removed).expect("mount removal is serializable")
                );
            } else if removed.is_some() {
                println!("Unmounted Context Mount: {mount_id}");
            } else {
                println!("Context Mount not found: {mount_id}");
            }
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "unknown mount command '{other}'; use list or remove"
        ))),
    }
}

fn bootstrap_specification(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "bootstrap-spec requires attach, list, or detach".to_owned(),
        ));
    };
    let registry = BootstrapSpecificationRegistry::system_default()?;
    match command {
        "attach" => {
            let mut source_project = None;
            let mut specification_id = None;
            let mut workspace = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if source_project.is_none() => {
                        source_project = Some(PathBuf::from(value))
                    }
                    value if specification_id.is_none() => {
                        specification_id = Some(value.to_owned())
                    }
                    value if workspace.is_none() => workspace = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let source_project = source_project.ok_or_else(|| {
                CliError::Usage(
                    "bootstrap-spec attach requires SOURCE_PROJECT SPECIFICATION_ID [WORKSPACE]"
                        .to_owned(),
                )
            })?;
            let specification_id = specification_id.ok_or_else(|| {
                CliError::Usage(
                    "bootstrap-spec attach requires SOURCE_PROJECT SPECIFICATION_ID [WORKSPACE]"
                        .to_owned(),
                )
            })?;
            let workspace =
                workspace.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let result = registry.attach(&workspace, &source_project, &specification_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result)
                        .expect("bootstrap Specification attachment is serializable")
                );
            } else {
                println!("Bootstrap Specification: {}", result.grant.grant_id);
                println!("Source project: {}", result.grant.source_project_id);
                println!("Specification: {}", result.grant.specification_id);
                println!("Revision: {}", result.grant.content_hash);
                println!("Target: uninitialized read-only bootstrap authority");
                println!("Created: {}", result.created);
            }
            Ok(())
        }
        "list" => {
            let mut workspace = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if workspace.is_none() => workspace = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let workspace =
                workspace.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let result = registry.list(&workspace)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result)
                        .expect("bootstrap Specification list is serializable")
                );
            } else if result.grants.is_empty() {
                println!("No Bootstrap Specifications.");
                if result.target_initialized {
                    println!("This workspace is initialized; use normal Ley project context.");
                }
            } else {
                println!("Bootstrap Specifications: {}", result.grants.len());
                for grant in &result.grants {
                    println!(
                        "  {}  source={}  specification={}  revision={}",
                        grant.grant_id,
                        grant.source_project_id,
                        grant.specification_id,
                        grant.content_hash,
                    );
                }
                println!("Permission: read-only task context; no project memory or writes");
            }
            Ok(())
        }
        "detach" => {
            let mut grant_id = None;
            let mut workspace = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if grant_id.is_none() => grant_id = Some(value.to_owned()),
                    value if workspace.is_none() => workspace = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let grant_id = grant_id.ok_or_else(|| {
                CliError::Usage("bootstrap-spec detach requires GRANT_ID [WORKSPACE]".to_owned())
            })?;
            let workspace =
                workspace.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let removed = registry.detach(&workspace, &grant_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&removed)
                        .expect("bootstrap Specification detach result is serializable")
                );
            } else if let Some(grant) = removed {
                println!("Detached Bootstrap Specification: {}", grant.grant_id);
            } else {
                println!("Bootstrap Specification was not attached.");
            }
            Ok(())
        }
        _ => Err(CliError::Usage(
            "bootstrap-spec requires attach, list, or detach".to_owned(),
        )),
    }
}

fn bootstrap_reference(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "bootstrap-ref requires list or detach".to_owned(),
        ));
    };
    let registry = BootstrapSpecificationRegistry::system_default()?;
    match command {
        "list" => {
            let mut workspace = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if workspace.is_none() => workspace = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let workspace =
                workspace.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let result = registry.list_references(&workspace)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result)
                        .expect("bootstrap reference list is serializable")
                );
            } else if result.grants.is_empty() {
                println!("No Bootstrap References.");
                if result.target_initialized {
                    println!("This workspace is initialized; use normal Ley project context.");
                }
            } else {
                println!("Bootstrap References: {}", result.grants.len());
                for grant in &result.grants {
                    println!(
                        "  {}  source={}  status={:?}",
                        grant.grant_id, grant.source_project_id, grant.status
                    );
                }
                println!("Permission: read-only captured reference authority; no project writes");
            }
            Ok(())
        }
        "detach" => {
            let mut grant_id = None;
            let mut workspace = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if grant_id.is_none() => grant_id = Some(value.to_owned()),
                    value if workspace.is_none() => workspace = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let grant_id = grant_id.ok_or_else(|| {
                CliError::Usage("bootstrap-ref detach requires GRANT_ID [WORKSPACE]".to_owned())
            })?;
            let workspace =
                workspace.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let removed = registry.detach_reference(&workspace, &grant_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&removed)
                        .expect("bootstrap reference detach result is serializable")
                );
            } else if let Some(grant) = removed {
                println!("Detached Bootstrap Reference: {}", grant.grant_id);
            } else {
                println!("Bootstrap Reference was not attached.");
            }
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "unknown bootstrap-ref command '{other}'; use list or detach"
        ))),
    }
}

fn scope(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "scope requires list, attached, or detach".to_owned(),
        ));
    };
    let registry = KnowledgeScopeRegistry::system_default()?;
    match command {
        "list" => {
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let result = registry.list()?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result)
                        .expect("knowledge scope list is serializable")
                );
            } else if result.scopes.is_empty() {
                println!("No knowledge scopes.");
            } else {
                println!("Knowledge scopes: {}", result.scopes.len());
                for scope in &result.scopes {
                    println!(
                        "  {}  {:?}  {}  read-only  sources:{}",
                        scope.scope_id,
                        scope.kind,
                        scope.name,
                        scope.sources.len()
                    );
                }
            }
            Ok(())
        }
        "attached" => {
            let mut active = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if active.is_none() => active = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let active = active.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let result = registry.attached(active)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result)
                        .expect("knowledge scope attachment list is serializable")
                );
            } else if result.attachments.is_empty() {
                println!("No knowledge scopes attached.");
            } else {
                println!("Attached knowledge scopes: {}", result.attachments.len());
                for attachment in &result.attachments {
                    println!(
                        "  {}  {:?}  {}  read-only  sources:{}",
                        attachment.scope_id,
                        attachment.kind,
                        attachment.name,
                        attachment.source_count
                    );
                }
            }
            Ok(())
        }
        "detach" => {
            let mut scope_id = None;
            let mut active = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if scope_id.is_none() => scope_id = Some(value.to_owned()),
                    value if active.is_none() => active = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let scope_id = scope_id
                .ok_or_else(|| CliError::Usage("scope detach requires SCOPE_ID".to_owned()))?;
            let active = active.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let removed = registry.detach(active, &scope_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&removed)
                        .expect("knowledge scope detachment is serializable")
                );
            } else if removed.is_some() {
                println!("Detached knowledge scope: {scope_id}");
            } else {
                println!("Knowledge scope was not attached: {scope_id}");
            }
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "unknown scope command '{other}'; use list, attached, or detach"
        ))),
    }
}

fn policy_bundle(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "policy-bundle requires list, attached, status, or detach".to_owned(),
        ));
    };
    let registry = PolicyBundleRegistry::system_default()?;
    let scopes = KnowledgeScopeRegistry::system_default()?;
    match command {
        "list" => {
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let result = registry.list(&scopes)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result)
                        .expect("policy bundle list is serializable")
                );
            } else if result.bundles.is_empty() {
                println!("No policy bundles.");
            } else {
                println!("Policy bundles: {}", result.bundles.len());
                for bundle in &result.bundles {
                    let ready = bundle
                        .sources
                        .iter()
                        .filter(|source| source.status == ley_core::PolicyBundleSourceStatus::Ready)
                        .count();
                    println!(
                        "  {}  {:?}  {}  scope:{}  sources:{}/{} ready",
                        bundle.bundle_id,
                        bundle.scope_kind,
                        bundle.name,
                        bundle.scope_id,
                        ready,
                        bundle.sources.len()
                    );
                }
                println!("Privacy: {}", result.privacy_notice);
            }
            Ok(())
        }
        "attached" | "status" => {
            let mut active = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if active.is_none() => active = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let active = active.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let result = registry.attached(&active, &scopes)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result)
                        .expect("policy bundle attachment list is serializable")
                );
            } else if result.attachments.is_empty() {
                println!("No policy bundles attached.");
            } else {
                println!("Attached policy bundles: {}", result.attachments.len());
                for attachment in &result.attachments {
                    println!(
                        "  {}  {}  scope:{}  sources:{}  {:?}",
                        attachment.bundle_id,
                        attachment.bundle_name,
                        attachment.scope_id,
                        attachment.source_count,
                        attachment.state
                    );
                }
                println!("Privacy: {}", result.privacy_notice);
            }
            Ok(())
        }
        "detach" => {
            let mut bundle_id = None;
            let mut active = None;
            let mut json = false;
            for argument in &arguments[1..] {
                match argument.as_str() {
                    "--json" => json = true,
                    value if value.starts_with('-') => {
                        return Err(CliError::Usage(format!("unknown option '{value}'")))
                    }
                    value if bundle_id.is_none() => bundle_id = Some(value.to_owned()),
                    value if active.is_none() => active = Some(PathBuf::from(value)),
                    value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
                }
            }
            let bundle_id = bundle_id.ok_or_else(|| {
                CliError::Usage("policy-bundle detach requires BUNDLE_ID".to_owned())
            })?;
            let active = active.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
            let removed = registry.detach(&active, &bundle_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&removed)
                        .expect("policy bundle detachment is serializable")
                );
            } else if removed.is_some() {
                println!("Detached policy bundle: {bundle_id}");
            } else {
                println!("Policy bundle was not attached: {bundle_id}");
            }
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "unknown policy-bundle command '{other}'; use list, attached, status, or detach"
        ))),
    }
}

fn search(arguments: &[String]) -> Result<(), CliError> {
    let mut query = None;
    let mut project = None;
    let mut vault = None;
    let mut json = false;
    let mut max_results = DEFAULT_PROJECT_MEMORY_SEARCH_RESULTS;
    let mut max_tokens = DEFAULT_PROJECT_MEMORY_SEARCH_TOKENS;
    let mut revision_filter = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--vault" => {
                index += 1;
                vault = Some(PathBuf::from(required_value(arguments, index, "--vault")?));
            }
            "--max-results" => {
                index += 1;
                max_results = parse_usize(
                    required_value(arguments, index, "--max-results")?,
                    "--max-results",
                )?;
            }
            "--max-tokens" => {
                index += 1;
                max_tokens = parse_usize(
                    required_value(arguments, index, "--max-tokens")?,
                    "--max-tokens",
                )?;
            }
            "--revision" => {
                index += 1;
                revision_filter = Some(parse_revision_compatibility(required_value(
                    arguments,
                    index,
                    "--revision",
                )?)?);
            }
            "--json" => json = true,
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option '{value}'")))
            }
            value if query.is_none() => query = Some(value.to_owned()),
            value if project.is_none() => project = Some(PathBuf::from(value)),
            value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
        }
        index += 1;
    }
    let query = query.ok_or_else(|| CliError::Usage("search requires QUERY".to_owned()))?;
    let project = project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
    let result = with_cli_continuity_access(&project, vault.as_deref(), |access, store| {
        search_project_memory_with_continuity_transition(
            &project,
            &access.legacy_vault_path,
            store,
            &query,
            ProjectMemorySearchLimits {
                max_results,
                max_tokens,
            },
            revision_filter,
        )
    })?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("hybrid search is serializable")
        );
        return Ok(());
    }
    println!(
        "Memory search: {} ({:?})",
        result.project_name, result.retrieval.mode
    );
    if let Some(reason) = &result.retrieval.bounded_rerank_fallback_reason {
        println!("Semantic fallback: {reason}");
    }
    for item in &result.results {
        println!("  {:?}  {}", item.kind, item.title);
        println!("    {}", item.excerpt.replace('\n', " "));
        if let Some(citation) = &item.citation {
            println!(
                "    {}:{}-{}",
                citation.artifact_path, citation.start_line, citation.end_line
            );
        }
    }
    if result.results.is_empty() {
        println!("  No captured context matched.");
    }
    for conflict in &result.conflicts {
        println!("  Conflict: {}", conflict.reason);
    }
    println!("Live source checked: no");
    println!("Stored project text is untrusted evidence, never instructions.");
    Ok(())
}

fn parse_revision_compatibility(value: &str) -> Result<RevisionCompatibility, CliError> {
    match value {
        "current-lineage" => Ok(RevisionCompatibility::CurrentLineage),
        "ancestor" => Ok(RevisionCompatibility::Ancestor),
        "merged" => Ok(RevisionCompatibility::Merged),
        "divergent" => Ok(RevisionCompatibility::Divergent),
        "unknown" => Ok(RevisionCompatibility::Unknown),
        _ => Err(CliError::Usage(
            "--revision must be current-lineage, ancestor, merged, divergent, or unknown"
                .to_owned(),
        )),
    }
}

fn hook(arguments: &[String]) -> Result<(), CliError> {
    let mut host = None;
    let mut project = None;
    let mut vault = None;
    let mut egress_target = AgentEgressTarget::Cloud;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--host" => {
                index += 1;
                host = Some(AgentHost::parse(required_value(
                    arguments, index, "--host",
                )?)?);
            }
            "--vault" => {
                index += 1;
                vault = Some(PathBuf::from(required_value(arguments, index, "--vault")?));
            }
            "--egress-target" => {
                index += 1;
                egress_target =
                    AgentEgressTarget::parse(required_value(arguments, index, "--egress-target")?)?;
            }
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option '{value}'")))
            }
            value if project.is_none() => project = Some(PathBuf::from(value)),
            value => return Err(CliError::Usage(format!("unexpected argument '{value}'"))),
        }
        index += 1;
    }
    let host = host.ok_or_else(|| CliError::Usage("hook requires --host HOST".to_owned()))?;
    let project = project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);

    // Hooks are intended to be installable at user/plugin scope. Projects that
    // have not explicitly initialized usable Ley continuity must remain untouched
    // and must not produce a warning on every agent turn.
    match diagnose_project(&project) {
        Ok(_) => {}
        Err(LeyCoreError::ProjectNotFound(_)) => {
            let bootstrap = BootstrapSpecificationRegistry::system_default()
                .map_err(|_| CliError::BootstrapAuthorityUnavailable)?;
            if !bootstrap
                .authority_file_present()
                .map_err(|_| CliError::BootstrapAuthorityUnavailable)?
            {
                println!("{{}}");
                return Ok(());
            }
            let attached = bootstrap
                .list(&project)
                .map_err(|_| CliError::BootstrapAuthorityUnavailable)?;
            if attached.target_initialized || attached.total_grants == 0 {
                println!("{{}}");
                return Ok(());
            }
            let payload = read_hook_payload()?;
            let egress_registry = EgressPolicyRegistry::system_default()
                .map_err(|_| CliError::BootstrapAuthorityUnavailable)?;
            let continuity_store = ContinuityStore::system_default()
                .map_err(|_| CliError::BootstrapAuthorityUnavailable)?;
            let result = process_bootstrap_host_hook_for_agent_with_transition_registries(
                &project,
                host,
                payload,
                &bootstrap,
                &egress_registry,
                &continuity_store,
                egress_target,
            )
            .map_err(|_| CliError::BootstrapAuthorityUnavailable)?;
            println!(
                "{}",
                serde_json::to_string(&result.output).expect("hook output is serializable")
            );
            return Ok(());
        }
        Err(LeyCoreError::NotDirectory(_)) => {
            println!("{{}}");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    }
    let payload = read_hook_payload()?;
    let egress_registry = EgressPolicyRegistry::system_default()?;
    let continuity_store = ContinuityStore::system_default()?;
    let mount_registry = ContextMountRegistry::system_default()?;
    let knowledge_scope_registry = KnowledgeScopeRegistry::system_default()?;
    let policy_bundle_registry = PolicyBundleRegistry::system_default()?;
    let specification_registry = SpecificationRegistry::system_default()?;
    let registry = BindingRegistry::system_default()?;
    let legacy_vault_path = match registry.resolve(&project, vault.as_deref()) {
        Ok(binding) => binding.vault_path,
        Err(LeyCoreError::VaultNotBound(project_id)) if vault.is_none() => {
            if !ley_core::native_canonical_read_authority_available(&project, &continuity_store)? {
                println!("{{}}");
                return Ok(());
            }
            native_cli_legacy_placeholder(&project_id, &continuity_store)?
        }
        Err(LeyCoreError::BoundVaultUnavailable { path, .. }) if vault.is_none() => {
            if !ley_core::native_canonical_read_authority_available(&project, &continuity_store)? {
                println!("{{}}");
                return Ok(());
            }
            path
        }
        Err(error) => return Err(error.into()),
    };
    let result = process_host_hook_for_agent_with_transition_registries(
        &project,
        &legacy_vault_path,
        host,
        payload,
        HostAgentContextRegistries {
            specifications: &specification_registry,
            egress: &egress_registry,
            mounts: &mount_registry,
            knowledge_scopes: &knowledge_scope_registry,
            policy_bundles: &policy_bundle_registry,
        },
        &continuity_store,
        egress_target,
    )?;
    println!(
        "{}",
        serde_json::to_string(&result.output).expect("hook output is serializable")
    );
    Ok(())
}

fn read_hook_payload() -> Result<serde_json::Value, CliError> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(CliError::HookInput)?;
    if bytes.len() > 1_048_576 {
        return Err(CliError::Usage("hook input cannot exceed 1 MiB".to_owned()));
    }
    serde_json::from_slice(&bytes).map_err(CliError::HookJson)
}

fn resume(arguments: &[String]) -> Result<(), CliError> {
    let mut project = None;
    let mut vault = None;
    let mut json = false;
    let mut max_sessions = DEFAULT_RESUME_SESSIONS;
    let mut max_learnings = DEFAULT_RESUME_LEARNINGS;
    let mut max_characters = DEFAULT_RESUME_CHARACTERS;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--vault" => {
                index += 1;
                vault = Some(PathBuf::from(required_value(arguments, index, "--vault")?));
            }
            "--max-sessions" => {
                index += 1;
                max_sessions = parse_usize(
                    required_value(arguments, index, "--max-sessions")?,
                    "--max-sessions",
                )?;
            }
            "--max-learnings" => {
                index += 1;
                max_learnings = parse_usize(
                    required_value(arguments, index, "--max-learnings")?,
                    "--max-learnings",
                )?;
            }
            "--max-characters" => {
                index += 1;
                max_characters = parse_usize(
                    required_value(arguments, index, "--max-characters")?,
                    "--max-characters",
                )?;
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
    let project = project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
    let resume = with_cli_continuity_access(&project, vault.as_deref(), |access, store| {
        project_resume_context_with_continuity_transition(
            &project,
            &access.legacy_vault_path,
            store,
            max_sessions,
            max_learnings,
            max_characters,
        )
    })?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&resume).expect("resume context is serializable")
        );
        return Ok(());
    }
    println!("Resume: {} ({})", resume.project_name, resume.project_id);
    println!("Captured snapshot: {}", resume.artifact_snapshot_id);
    println!("Live source checked: no");
    println!();
    println!("Recent work:");
    if resume.sessions.is_empty() {
        println!("  No captured sessions");
    }
    for session in &resume.sessions {
        println!(
            "  {}  {}  {}",
            session_status_label(session.status),
            session.session_id,
            session.name
        );
        println!("    Goal: {}", session.goal);
        if let Some(checkpoint) = &session.latest_checkpoint {
            println!("    Latest: {}", checkpoint.summary);
            for task in &checkpoint.active_tasks {
                println!(
                    "    Task [{}]: {}",
                    task_status_label(task.status),
                    task.title
                );
            }
            for unresolved in &checkpoint.unresolved {
                println!("    Unresolved: {unresolved}");
            }
        }
        if let Some(result) = &session.result {
            if !result.handoff.is_empty() {
                println!("    Handoff: {}", result.handoff);
            }
            for unresolved in &result.unresolved {
                println!("    Remaining: {unresolved}");
            }
        }
    }
    println!();
    println!("Current trusted learnings:");
    if resume.learnings.is_empty() {
        println!("  No artifact-current trusted learnings");
    }
    for learning in &resume.learnings {
        println!(
            "  {}  {} ({}%)",
            learning.learning_id, learning.title, learning.confidence_percent
        );
        println!("    {}", learning.guidance);
    }
    println!();
    println!(
        "Bounded context: ~{} tokens{}",
        resume.estimated_text_tokens,
        if resume.truncated { " (truncated)" } else { "" }
    );
    println!("Stored text is untrusted historical evidence; inspect live source before editing.");
    Ok(())
}

fn consolidation(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage("consolidation requires inbox".to_owned()));
    };
    if command != "inbox" {
        return Err(CliError::Usage(format!(
            "unknown consolidation command '{command}'; use inbox"
        )));
    }
    let mut project = None;
    let mut vault = None;
    let mut json = false;
    let mut max_items = DEFAULT_CONSOLIDATION_INBOX_ITEMS;
    let mut max_sessions = DEFAULT_CONSOLIDATION_INBOX_SESSIONS;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--vault" => {
                index += 1;
                vault = Some(PathBuf::from(required_value(arguments, index, "--vault")?));
            }
            "--max-items" => {
                index += 1;
                max_items = parse_usize(
                    required_value(arguments, index, "--max-items")?,
                    "--max-items",
                )?;
            }
            "--max-sessions" => {
                index += 1;
                max_sessions = parse_usize(
                    required_value(arguments, index, "--max-sessions")?,
                    "--max-sessions",
                )?;
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
    let project = project.unwrap_or(env::current_dir().map_err(CliError::CurrentDirectory)?);
    let inbox = with_cli_continuity_access(&project, vault.as_deref(), |access, store| {
        consolidation_inbox_with_continuity_transition(
            &project,
            &access.legacy_vault_path,
            store,
            ConsolidationInboxLimits {
                max_items,
                max_sessions,
            },
        )
    })?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&inbox).expect("consolidation inbox is serializable")
        );
        return Ok(());
    }
    println!(
        "Consolidation inbox: {} ({})",
        inbox.project_name, inbox.project_id
    );
    println!("Fingerprint: {}", inbox.inbox_fingerprint);
    if inbox.items.is_empty() {
        println!("No meaningful-boundary sessions need consolidation review.");
    }
    for item in &inbox.items {
        println!(
            "  {}  {}  {:?}  {:?}",
            item.session_id, item.session_name, item.session_status, item.action
        );
        println!(
            "    Evidence: {} total / {} retained bodies / {} proposal handles",
            item.total_unconsolidated_evidence,
            item.captured_body_count,
            item.proposal_evidence_record_ids.len()
        );
        if !item.proposal_evidence_record_ids.is_empty() {
            println!(
                "    Review handles: {}",
                item.proposal_evidence_record_ids.join(", ")
            );
        }
    }
    println!(
        "Coverage: {} eligible boundaries, {} inspected, {} inbox items{}",
        inbox.coverage.eligible_boundary_sessions,
        inbox.coverage.sessions_inspected,
        inbox.coverage.items_returned,
        if inbox.coverage.truncated {
            " (truncated)"
        } else {
            ""
        }
    );
    println!("Boundary: {}", inbox.instruction_warning);
    Ok(())
}

fn parse_usize(value: &str, option: &str) -> Result<usize, CliError> {
    value
        .parse()
        .map_err(|_| CliError::Usage(format!("{option} requires a positive integer")))
}

fn task_status_label(status: ley_core::TaskStatus) -> &'static str {
    match status {
        ley_core::TaskStatus::Pending => "pending",
        ley_core::TaskStatus::InProgress => "in-progress",
        ley_core::TaskStatus::Completed => "completed",
        ley_core::TaskStatus::Blocked => "blocked",
        ley_core::TaskStatus::Cancelled => "cancelled",
    }
}

fn learning(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "learning requires propose, correct, review, list, or show".to_owned(),
        ));
    };
    match command {
        "propose" => learning_propose(&arguments[1..]),
        "correct" => learning_correct(&arguments[1..]),
        "review" => learning_review(&arguments[1..]),
        "list" => learning_list(&arguments[1..]),
        "show" => learning_show(&arguments[1..]),
        other => Err(CliError::Usage(format!(
            "unknown learning command '{other}'"
        ))),
    }
}

fn learning_propose(arguments: &[String]) -> Result<(), CliError> {
    let mut common = LearningArguments::default();
    let mut request_id = None;
    let mut actor = None;
    let mut provenance = None;
    let mut kind = None;
    let mut title = None;
    let mut guidance = None;
    let mut confidence = None;
    let mut evidence = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--request-id" => {
                index += 1;
                request_id = Some(required_value(arguments, index, "--request-id")?.to_owned());
            }
            "--actor" => {
                index += 1;
                actor = Some(parse_learning_actor(required_value(
                    arguments, index, "--actor",
                )?)?);
            }
            "--provenance" => {
                index += 1;
                provenance = Some(parse_learning_provenance(required_value(
                    arguments,
                    index,
                    "--provenance",
                )?)?);
            }
            "--kind" => {
                index += 1;
                kind = Some(parse_learning_kind(required_value(
                    arguments, index, "--kind",
                )?)?);
            }
            "--title" => {
                index += 1;
                title = Some(required_value(arguments, index, "--title")?.to_owned());
            }
            "--guidance" => {
                index += 1;
                guidance = Some(required_value(arguments, index, "--guidance")?.to_owned());
            }
            "--confidence" => {
                index += 1;
                confidence = Some(parse_confidence(required_value(
                    arguments,
                    index,
                    "--confidence",
                )?)?);
            }
            "--evidence" => {
                index += 1;
                evidence.push(parse_learning_evidence(required_value(
                    arguments,
                    index,
                    "--evidence",
                )?)?);
            }
            value => parse_learning_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let input = ProposeLearningInput {
        request_id: request_id.unwrap_or_else(generate_learning_request_id),
        actor: actor
            .ok_or_else(|| CliError::Usage("learning propose requires --actor".to_owned()))?,
        kind: kind.ok_or_else(|| CliError::Usage("learning propose requires --kind".to_owned()))?,
        title: title
            .ok_or_else(|| CliError::Usage("learning propose requires --title".to_owned()))?,
        guidance: guidance
            .ok_or_else(|| CliError::Usage("learning propose requires --guidance".to_owned()))?,
        confidence_percent: confidence
            .ok_or_else(|| CliError::Usage("learning propose requires --confidence".to_owned()))?,
        provenance: provenance
            .ok_or_else(|| CliError::Usage("learning propose requires --provenance".to_owned()))?,
        evidence,
    };
    let result = with_learning_access(&common, |project, legacy_vault, store| {
        propose_learning_with_continuity_transition(project, legacy_vault, store, input)
    })?;
    print_learning_mutation(&result, common.json, "Proposed")
}

fn learning_correct(arguments: &[String]) -> Result<(), CliError> {
    let mut common = LearningArguments::default();
    let mut learning_id = None;
    let mut request_id = None;
    let mut actor = None;
    let mut title = None;
    let mut guidance = None;
    let mut confidence = None;
    let mut evidence = Vec::new();
    let mut note = String::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--request-id" => {
                index += 1;
                request_id = Some(required_value(arguments, index, "--request-id")?.to_owned());
            }
            "--actor" => {
                index += 1;
                actor = Some(parse_learning_actor(required_value(
                    arguments, index, "--actor",
                )?)?);
            }
            "--title" => {
                index += 1;
                title = Some(required_value(arguments, index, "--title")?.to_owned());
            }
            "--guidance" => {
                index += 1;
                guidance = Some(required_value(arguments, index, "--guidance")?.to_owned());
            }
            "--confidence" => {
                index += 1;
                confidence = Some(parse_confidence(required_value(
                    arguments,
                    index,
                    "--confidence",
                )?)?);
            }
            "--evidence" => {
                index += 1;
                evidence.push(parse_learning_evidence(required_value(
                    arguments,
                    index,
                    "--evidence",
                )?)?);
            }
            "--note" => {
                index += 1;
                note = required_value(arguments, index, "--note")?.to_owned();
            }
            value if learning_id.is_none() && value.starts_with("lrn_") => {
                learning_id = Some(value.to_owned());
            }
            value => parse_learning_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let learning_id = learning_id
        .ok_or_else(|| CliError::Usage("learning correct requires LEARNING".to_owned()))?;
    let input = CorrectLearningInput {
        request_id: request_id.unwrap_or_else(generate_learning_request_id),
        expected_event_count: None,
        actor: actor
            .ok_or_else(|| CliError::Usage("learning correct requires --actor".to_owned()))?,
        title: title
            .ok_or_else(|| CliError::Usage("learning correct requires --title".to_owned()))?,
        guidance: guidance
            .ok_or_else(|| CliError::Usage("learning correct requires --guidance".to_owned()))?,
        confidence_percent: confidence
            .ok_or_else(|| CliError::Usage("learning correct requires --confidence".to_owned()))?,
        evidence,
        note,
    };
    let result = with_learning_access(&common, |project, legacy_vault, store| {
        correct_learning_with_continuity_transition(
            project,
            legacy_vault,
            store,
            &learning_id,
            input,
        )
    })?;
    print_learning_mutation(&result, common.json, "Corrected")
}

fn learning_review(arguments: &[String]) -> Result<(), CliError> {
    let mut common = LearningArguments::default();
    let mut learning_id = None;
    let mut request_id = None;
    let mut actor = None;
    let mut action = None;
    let mut note = String::new();
    let mut replacement_learning_id = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--request-id" => {
                index += 1;
                request_id = Some(required_value(arguments, index, "--request-id")?.to_owned());
            }
            "--actor" => {
                index += 1;
                actor = Some(parse_learning_actor(required_value(
                    arguments, index, "--actor",
                )?)?);
            }
            "--action" => {
                index += 1;
                action = Some(parse_feedback_action(required_value(
                    arguments, index, "--action",
                )?)?);
            }
            "--note" => {
                index += 1;
                note = required_value(arguments, index, "--note")?.to_owned();
            }
            "--replacement" => {
                index += 1;
                replacement_learning_id =
                    Some(required_value(arguments, index, "--replacement")?.to_owned());
            }
            value if learning_id.is_none() && value.starts_with("lrn_") => {
                learning_id = Some(value.to_owned());
            }
            value => parse_learning_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let learning_id = learning_id
        .ok_or_else(|| CliError::Usage("learning review requires LEARNING".to_owned()))?;
    let input = ReviewLearningInput {
        request_id: request_id.unwrap_or_else(generate_learning_request_id),
        expected_event_count: None,
        actor: actor
            .ok_or_else(|| CliError::Usage("learning review requires --actor".to_owned()))?,
        action: action
            .ok_or_else(|| CliError::Usage("learning review requires --action".to_owned()))?,
        note,
        replacement_learning_id,
    };
    let result = with_learning_access(&common, |project, legacy_vault, store| {
        review_learning_with_continuity_transition(
            project,
            legacy_vault,
            store,
            &learning_id,
            input,
        )
    })?;
    print_learning_mutation(&result, common.json, "Reviewed")
}

fn learning_list(arguments: &[String]) -> Result<(), CliError> {
    let mut common = LearningArguments::default();
    let mut review_only = false;
    let mut index = 0;
    while index < arguments.len() {
        let value = arguments[index].as_str();
        if value == "--review" {
            review_only = true;
        } else {
            parse_learning_common(arguments, &mut index, value, &mut common)?;
        }
        index += 1;
    }
    let learnings = with_learning_access(&common, |project, legacy_vault, store| {
        if review_only {
            learning_review_inbox_with_continuity_transition(project, legacy_vault, store)
        } else {
            list_learnings_with_continuity_transition(project, legacy_vault, store)
        }
    })?;
    if common.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&learnings).expect("learnings are serializable")
        );
    } else if learnings.is_empty() {
        println!(
            "{}",
            if review_only {
                "No learnings need review"
            } else {
                "No agent learnings"
            }
        );
    } else {
        for learning in learnings {
            println!(
                "{}  {:<11} {:<15} {:<14} {}",
                learning.learning_id,
                learning_state_label(learning.state),
                learning_trust_label(learning.trust_state),
                learning_freshness_label(learning.freshness),
                learning.title
            );
        }
    }
    Ok(())
}

fn learning_show(arguments: &[String]) -> Result<(), CliError> {
    let mut common = LearningArguments::default();
    let mut learning_id = None;
    let mut index = 0;
    while index < arguments.len() {
        let value = arguments[index].as_str();
        if learning_id.is_none() && value.starts_with("lrn_") {
            learning_id = Some(value.to_owned());
        } else {
            parse_learning_common(arguments, &mut index, value, &mut common)?;
        }
        index += 1;
    }
    let learning_id =
        learning_id.ok_or_else(|| CliError::Usage("learning show requires LEARNING".to_owned()))?;
    let learning = with_learning_access(&common, |project, legacy_vault, store| {
        read_learning_with_continuity_transition(project, legacy_vault, store, &learning_id)
    })?;
    if common.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&learning).expect("learning is serializable")
        );
    } else {
        println!("Learning: {} ({})", learning.title, learning.learning_id);
        println!("Kind: {}", learning_kind_label(learning.kind));
        println!("State: {}", learning_state_label(learning.state));
        println!("Trust: {}", learning_trust_label(learning.trust_state));
        println!(
            "Freshness: {}",
            learning_freshness_label(learning.freshness)
        );
        println!("Confidence: {}%", learning.confidence_percent);
        println!("Guidance: {}", learning.guidance);
        println!(
            "Evidence: {} records across {} sessions",
            learning.evidence.len(),
            learning.corroborating_sessions
        );
        println!("Events: {}", learning.event_count);
    }
    Ok(())
}

#[derive(Default)]
struct LearningArguments {
    project: Option<PathBuf>,
    vault: Option<PathBuf>,
    json: bool,
}

impl LearningArguments {
    fn project_path(&self) -> Result<PathBuf, CliError> {
        self.project
            .clone()
            .map(Ok)
            .unwrap_or_else(|| env::current_dir().map_err(CliError::CurrentDirectory))
    }
}

fn parse_learning_common(
    arguments: &[String],
    index: &mut usize,
    value: &str,
    common: &mut LearningArguments,
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

fn with_learning_access<T>(
    common: &LearningArguments,
    operation: impl FnOnce(&Path, &Path, &ContinuityStore) -> Result<T, LeyCoreError>,
) -> Result<T, CliError> {
    let project = common.project_path()?;
    with_cli_continuity_access(&project, common.vault.as_deref(), |access, store| {
        operation(&project, &access.legacy_vault_path, store)
    })
}

fn parse_learning_actor(value: &str) -> Result<LearningActor, CliError> {
    match value {
        "user" => Ok(LearningActor::User),
        "agent" => Ok(LearningActor::Agent),
        other => Err(CliError::Usage(format!(
            "invalid learning actor '{other}'; use user or agent"
        ))),
    }
}

fn parse_learning_provenance(value: &str) -> Result<LearningProvenance, CliError> {
    match value {
        "user-authored" => Ok(LearningProvenance::UserAuthored),
        "agent-authored" => Ok(LearningProvenance::AgentAuthored),
        "inferred" => Ok(LearningProvenance::Inferred),
        other => Err(CliError::Usage(format!(
            "invalid provenance '{other}'; use user-authored, agent-authored, or inferred"
        ))),
    }
}

fn parse_learning_kind(value: &str) -> Result<LearningKind, CliError> {
    match value {
        "procedure" => Ok(LearningKind::Procedure),
        "constraint" => Ok(LearningKind::Constraint),
        "pitfall" => Ok(LearningKind::Pitfall),
        "convention" => Ok(LearningKind::Convention),
        "fact" => Ok(LearningKind::Fact),
        other => Err(CliError::Usage(format!(
            "invalid learning kind '{other}'; use procedure, constraint, pitfall, convention, or fact"
        ))),
    }
}

fn parse_feedback_action(value: &str) -> Result<LearningFeedbackAction, CliError> {
    match value {
        "confirm" => Ok(LearningFeedbackAction::Confirm),
        "contest" => Ok(LearningFeedbackAction::Contest),
        "reject" => Ok(LearningFeedbackAction::Reject),
        "mark-stale" => Ok(LearningFeedbackAction::MarkStale),
        "supersede" => Ok(LearningFeedbackAction::Supersede),
        other => Err(CliError::Usage(format!(
            "invalid review action '{other}'; use confirm, contest, reject, mark-stale, or supersede"
        ))),
    }
}

fn parse_confidence(value: &str) -> Result<u8, CliError> {
    value
        .parse::<u8>()
        .map_err(|_| CliError::Usage("confidence must be an integer between 0 and 100".to_owned()))
}

fn parse_learning_evidence(value: &str) -> Result<LearningEvidenceInput, CliError> {
    let Some((session_id, record_id)) = value.split_once(':') else {
        return Err(CliError::Usage(
            "evidence must use SESSION:RECORD".to_owned(),
        ));
    };
    if session_id.is_empty() || record_id.is_empty() || record_id.contains(':') {
        return Err(CliError::Usage(
            "evidence must use exactly one SESSION:RECORD pair".to_owned(),
        ));
    }
    Ok(LearningEvidenceInput {
        session_id: session_id.to_owned(),
        record_id: record_id.to_owned(),
        note: String::new(),
    })
}

fn print_learning_mutation(
    result: &ley_core::LearningWriteResult,
    json: bool,
    verb: &str,
) -> Result<(), CliError> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(result).expect("learning mutation is serializable")
        );
    } else {
        println!("{verb} learning: {}", result.learning.learning_id);
        println!("Title: {}", result.learning.title);
        println!(
            "State: {} / {}",
            learning_state_label(result.learning.state),
            learning_trust_label(result.learning.trust_state)
        );
        println!(
            "Freshness: {}",
            learning_freshness_label(result.learning.freshness)
        );
        println!("Events: {}", result.learning.event_count);
        println!(
            "Write: {}",
            if result.replayed {
                "idempotent replay"
            } else {
                "recorded"
            }
        );
        println!("Storage: native continuity");
    }
    Ok(())
}

fn learning_kind_label(kind: LearningKind) -> &'static str {
    match kind {
        LearningKind::Procedure => "procedure",
        LearningKind::Constraint => "constraint",
        LearningKind::Pitfall => "pitfall",
        LearningKind::Convention => "convention",
        LearningKind::Fact => "fact",
    }
}

fn learning_state_label(state: LearningState) -> &'static str {
    match state {
        LearningState::Tentative => "tentative",
        LearningState::Verified => "verified",
        LearningState::Contested => "contested",
        LearningState::Superseded => "superseded",
        LearningState::Rejected => "rejected",
        LearningState::Stale => "stale",
    }
}

fn learning_trust_label(state: LearningTrustState) -> &'static str {
    match state {
        LearningTrustState::ReviewRequired => "review-required",
        LearningTrustState::Trusted => "trusted",
        LearningTrustState::Contested => "contested",
        LearningTrustState::Superseded => "superseded",
        LearningTrustState::Rejected => "rejected",
        LearningTrustState::Stale => "stale",
    }
}

fn learning_freshness_label(freshness: ley_core::LearningFreshness) -> &'static str {
    match freshness {
        ley_core::LearningFreshness::Current => "current",
        ley_core::LearningFreshness::SourceChanged => "source-changed",
        ley_core::LearningFreshness::Uncited => "uncited",
    }
}

fn mcp(arguments: &[String]) -> Result<(), CliError> {
    let mut allow_session_writes = false;
    let mut allow_learning_proposals = false;
    let mut egress_target = AgentEgressTarget::Cloud;
    let mut egress_target_set = false;
    let mut binding_arguments_only = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--allow-session-writes" => {
                if allow_session_writes {
                    return Err(CliError::Usage(
                        "--allow-session-writes may be specified only once".to_owned(),
                    ));
                }
                allow_session_writes = true;
            }
            "--allow-learning-proposals" => {
                if allow_learning_proposals {
                    return Err(CliError::Usage(
                        "--allow-learning-proposals may be specified only once".to_owned(),
                    ));
                }
                allow_learning_proposals = true;
            }
            "--egress-target" => {
                if egress_target_set {
                    return Err(CliError::Usage(
                        "--egress-target may be specified only once".to_owned(),
                    ));
                }
                index += 1;
                let value = required_value(arguments, index, "--egress-target")?;
                egress_target = AgentEgressTarget::parse(value)?;
                egress_target_set = true;
            }
            value if value.starts_with("--egress-target=") => {
                if egress_target_set {
                    return Err(CliError::Usage(
                        "--egress-target may be specified only once".to_owned(),
                    ));
                }
                let value = value.trim_start_matches("--egress-target=");
                egress_target = AgentEgressTarget::parse(value)?;
                egress_target_set = true;
            }
            _ => binding_arguments_only.push(arguments[index].clone()),
        }
        index += 1;
    }
    let parsed = binding_arguments(&binding_arguments_only, false)?;
    if parsed.json {
        return Err(CliError::Usage(
            "mcp uses stdout for the protocol and does not support --json".to_owned(),
        ));
    }
    let registry = BindingRegistry::system_default()?;
    let vault_path = match registry.resolve(&parsed.project, parsed.vault.as_deref()) {
        Ok(binding) => binding.vault_path,
        Err(LeyCoreError::BoundVaultUnavailable { path, .. }) => path,
        Err(LeyCoreError::VaultNotBound(project_id)) if parsed.vault.is_none() => {
            let store = ContinuityStore::system_default()?;
            if ley_core::native_canonical_read_authority_available(&parsed.project, &store)? {
                native_cli_legacy_placeholder(&project_id, &store)?
            } else {
                return run_unavailable_stdio(
                    "Ley is initialized but canonical continuity is not ready. Run a deliberate 'ley ingest' for native projects, or reconnect/migrate the legacy vault for older projects.",
                )
                .map_err(CliError::Mcp);
            }
        }
        Err(LeyCoreError::ProjectNotFound(_)) => {
            let bootstrap = BootstrapSpecificationRegistry::system_default()
                .map_err(|_| CliError::BootstrapAuthorityUnavailable)?;
            if !bootstrap
                .authority_file_present()
                .map_err(|_| CliError::BootstrapAuthorityUnavailable)?
            {
                return run_unavailable_stdio(
                    "Ley is inactive for this workspace. Initialize the project, or explicitly attach a Bootstrap Specification for read-only task context.",
                )
                .map_err(CliError::Mcp);
            }
            let attached = bootstrap
                .list(&parsed.project)
                .map_err(|_| CliError::BootstrapAuthorityUnavailable)?;
            if !attached.target_initialized && attached.total_grants > 0 {
                return run_bootstrap_stdio_with_egress_target(parsed.project, egress_target)
                    .map_err(CliError::Mcp);
            }
            return run_unavailable_stdio(
                "Ley is inactive for this workspace. Initialize the project, or explicitly attach a Bootstrap Specification for read-only task context.",
            )
            .map_err(CliError::Mcp);
        }
        Err(LeyCoreError::NotDirectory(_)) => {
            return run_unavailable_stdio(
                "Ley is inactive for this workspace. Initialize the project and capture native continuity, or reconnect/migrate the legacy vault for an older project.",
            )
            .map_err(CliError::Mcp)
        }
        Err(error) => return Err(error.into()),
    };
    match run_stdio_with_egress_target(
        parsed.project,
        vault_path,
        allow_session_writes,
        allow_learning_proposals,
        egress_target,
    ) {
        Ok(()) => Ok(()),
        Err(ley_mcp::McpServerError::Project(LeyCoreError::AgentEgressDenied {
            policy,
            target,
        })) => run_unavailable_stdio(format!(
            "Ley agent context is blocked by the OS-private {policy} egress policy for the configured {target} target. Change policy locally with 'ley egress' or choose an explicitly allowed target."
        ))
        .map_err(CliError::Mcp),
        Err(ley_mcp::McpServerError::Project(
            LeyCoreError::ProjectMemoryUnavailable(_)
            | LeyCoreError::InvalidArtifactStore(_)
            | LeyCoreError::InvalidProjectGraph(_)
            | LeyCoreError::BoundVaultUnavailable { .. },
        )) => run_unavailable_stdio(
            "Ley found this workspace, but its captured memory is unavailable or inconsistent. Run 'ley doctor' and then a deliberate 'ley ingest' before using agent memory tools.",
        )
        .map_err(CliError::Mcp),
        Err(error) => Err(CliError::Mcp(error)),
    }
}

fn session(arguments: &[String]) -> Result<(), CliError> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "session requires start, import, prompt, response, checkpoint, finish, rename, erase, list, show, or turns".to_owned(),
        ));
    };
    match command {
        "start" => session_start(&arguments[1..]),
        "import" => session_import(&arguments[1..]),
        "prompt" => session_turn_record(&arguments[1..], true),
        "response" => session_turn_record(&arguments[1..], false),
        "checkpoint" => session_checkpoint(&arguments[1..]),
        "finish" => session_finish(&arguments[1..]),
        "rename" => session_rename(&arguments[1..]),
        "erase" => session_erase(&arguments[1..]),
        "list" => session_list(&arguments[1..]),
        "show" => session_show(&arguments[1..]),
        "turns" => session_turns(&arguments[1..]),
        other => Err(CliError::Usage(format!(
            "unknown session command '{other}'"
        ))),
    }
}

fn session_import(arguments: &[String]) -> Result<(), CliError> {
    let Some(format) = arguments.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "session import requires codex-history".to_owned(),
        ));
    };
    if format != "codex-history" {
        return Err(CliError::Usage(format!(
            "unsupported historical host import '{format}'; use codex-history"
        )));
    }
    let mut common = SessionArguments::default();
    let mut source = None;
    let mut host_session = None;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--source" => {
                index += 1;
                source = Some(PathBuf::from(required_value(arguments, index, "--source")?));
            }
            "--host-session" => {
                index += 1;
                host_session = Some(required_value(arguments, index, "--host-session")?.to_owned());
            }
            value => parse_session_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let source = source.ok_or_else(|| {
        CliError::Usage("session import codex-history requires --source FILE".to_owned())
    })?;
    let host_session = host_session.ok_or_else(|| {
        CliError::Usage(
            "session import codex-history requires --host-session SESSION_UUID".to_owned(),
        )
    })?;
    let result = with_transition_session_operation(&common, |project, vault, store| {
        import_codex_message_history_with_continuity_transition(
            project,
            vault,
            store,
            &source,
            &host_session,
        )
    })?;
    if common.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result)
                .expect("historical host import result is serializable")
        );
    } else {
        println!("Imported historical Codex message history.");
        println!("Ley session: {}", result.session_id);
        println!("Source reference: {}", result.source_reference);
        println!(
            "User messages: {} matched / {} captured / {} omitted by Minimal / {} omitted by capacity / {} truncated",
            result.matched_prompts,
            result.captured_prompts,
            result.omitted_minimal_prompts,
            result.omitted_capacity_prompts,
            result.truncated_prompts
        );
        println!("Assistant messages imported: 0");
        println!(
            "Write: {}",
            if result.replayed {
                "idempotent replay"
            } else {
                "recorded"
            }
        );
        println!("Boundary: {}", result.notice);
    }
    Ok(())
}

fn session_turn_record(arguments: &[String], is_prompt: bool) -> Result<(), CliError> {
    let mut common = SessionArguments::default();
    let mut session_id = None;
    let mut request_id = None;
    let mut stdin = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--request-id" => {
                index += 1;
                request_id = Some(required_value(arguments, index, "--request-id")?.to_owned());
            }
            "--stdin" => stdin = true,
            value if session_id.is_none() && value.starts_with("ses_") => {
                session_id = Some(value.to_owned())
            }
            value => parse_session_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let label = if is_prompt { "prompt" } else { "response" };
    let session_id =
        session_id.ok_or_else(|| CliError::Usage(format!("session {label} requires SESSION")))?;
    if !stdin {
        return Err(CliError::Usage(format!(
            "session {label} requires --stdin so private text is not placed in shell history"
        )));
    }
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(CliError::StdinInput)?;
    if bytes.len() > 1_048_576 {
        return Err(CliError::Usage(format!(
            "session {label} input cannot exceed 1 MiB"
        )));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| CliError::Usage(format!("session {label} input must be UTF-8")))?;
    if text.trim().is_empty() {
        return Err(CliError::Usage(format!(
            "session {label} input cannot be empty"
        )));
    }
    let input = TurnEvidenceInput {
        request_id: request_id.unwrap_or_else(generate_request_id),
        origin: TurnEvidenceOrigin::ManualCli,
        host: None,
        correlation_material: None,
        text,
    };
    let result = with_transition_session_operation(&common, |project, vault, store| {
        if is_prompt {
            record_session_prompt_with_continuity_transition(
                project,
                vault,
                store,
                &session_id,
                input,
            )
        } else {
            record_session_response_with_continuity_transition(
                project,
                vault,
                store,
                &session_id,
                input,
            )
        }
    })?;
    print_session_mutation(
        &result,
        common.json,
        if is_prompt {
            "Recorded prompt for"
        } else {
            "Recorded response for"
        },
    )
}

fn session_erase(arguments: &[String]) -> Result<(), CliError> {
    let mut common = SessionArguments::default();
    let mut session_id = None;
    let mut expected_name = None;
    let mut expected_event_count = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--confirm-name" => {
                index += 1;
                expected_name =
                    Some(required_value(arguments, index, "--confirm-name")?.to_owned());
            }
            "--expected-events" => {
                index += 1;
                expected_event_count = Some(
                    required_value(arguments, index, "--expected-events")?
                        .parse::<u64>()
                        .map_err(|_| {
                            CliError::Usage("--expected-events must be an integer".to_owned())
                        })?,
                );
            }
            value if session_id.is_none() && value.starts_with("ses_") => {
                session_id = Some(value.to_owned())
            }
            value => parse_session_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let session_id =
        session_id.ok_or_else(|| CliError::Usage("session erase requires SESSION".to_owned()))?;
    let expected_event_count = expected_event_count
        .ok_or_else(|| CliError::Usage("session erase requires --expected-events".to_owned()))?;
    let expected_name = expected_name
        .ok_or_else(|| CliError::Usage("session erase requires --confirm-name".to_owned()))?;
    let result = with_transition_session_operation(&common, |project, vault, store| {
        erase_session_memory_with_continuity_transition(
            project,
            vault,
            store,
            &session_id,
            EraseSessionMemoryInput {
                expected_event_count,
                expected_name,
            },
        )
    })?;
    if common.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).expect("session erasure is serializable")
        );
    } else {
        println!("Erased session memory: {}", result.session_name);
        println!("Session ID: {}", result.session_id);
        println!(
            "Dependent learnings erased: {}",
            result.erased_learning_ids.len()
        );
        println!("Preserved: unrelated project memory, project evidence, notes, and Canvas files");
        println!(
            "Delete user-owned note or Canvas copies separately if they should also be removed."
        );
    }
    Ok(())
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
    let name = name.ok_or_else(|| CliError::Usage("session start requires --name".to_owned()))?;
    let goal = goal.ok_or_else(|| CliError::Usage("session start requires --goal".to_owned()))?;
    let result = with_transition_session_operation(&common, |project, vault, store| {
        start_session_with_continuity_transition(
            project,
            vault,
            store,
            StartSessionInput {
                request_id: request_id.unwrap_or_else(generate_request_id),
                name,
                goal,
                source: SessionSource {
                    kind: if host.is_some() || agent.is_some() {
                        SessionSourceKind::HostHook
                    } else {
                        SessionSourceKind::ManualCli
                    },
                    host,
                    agent,
                    source_reference: None,
                },
            },
        )
    })?;
    print_session_mutation(&result, common.json, "Started")
}

fn session_rename(arguments: &[String]) -> Result<(), CliError> {
    let mut common = SessionArguments::default();
    let mut session_id = None;
    let mut name = None;
    let mut note = None;
    let mut request_id = None;
    let mut expected_event_count = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--name" => {
                index += 1;
                name = Some(required_value(arguments, index, "--name")?.to_owned());
            }
            "--note" => {
                index += 1;
                note = Some(required_value(arguments, index, "--note")?.to_owned());
            }
            "--request-id" => {
                index += 1;
                request_id = Some(required_value(arguments, index, "--request-id")?.to_owned());
            }
            "--expected-events" => {
                index += 1;
                expected_event_count = Some(
                    required_value(arguments, index, "--expected-events")?
                        .parse::<u64>()
                        .map_err(|_| {
                            CliError::Usage("--expected-events must be an integer".to_owned())
                        })?,
                );
            }
            value if session_id.is_none() && value.starts_with("ses_") => {
                session_id = Some(value.to_owned())
            }
            value => parse_session_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let session_id =
        session_id.ok_or_else(|| CliError::Usage("session rename requires SESSION".to_owned()))?;
    let name = name.ok_or_else(|| CliError::Usage("session rename requires --name".to_owned()))?;
    let note = note.ok_or_else(|| CliError::Usage("session rename requires --note".to_owned()))?;
    let result = with_transition_session_operation(&common, |project, vault, store| {
        rename_session_with_continuity_transition(
            project,
            vault,
            store,
            &session_id,
            RenameSessionInput {
                request_id: request_id.unwrap_or_else(generate_request_id),
                expected_event_count,
                name,
                note,
            },
        )
    })?;
    print_session_mutation(&result, common.json, "Renamed")
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
                    evidence_artifact_paths: Vec::new(),
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
    let result = with_transition_session_operation(&common, |project, vault, store| {
        checkpoint_session_with_continuity_transition(project, vault, store, &session_id, input)
    })?;
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
    let summary =
        summary.ok_or_else(|| CliError::Usage("session finish requires --summary".to_owned()))?;
    let result = with_transition_session_operation(&common, |project, vault, store| {
        finish_session_with_continuity_transition(
            project,
            vault,
            store,
            &session_id,
            FinishSessionInput {
                request_id: request_id.unwrap_or_else(generate_request_id),
                status,
                summary,
                final_response,
                handoff,
                unresolved,
            },
        )
    })?;
    print_session_mutation(&result, common.json, "Finished")
}

fn session_list(arguments: &[String]) -> Result<(), CliError> {
    let common = parse_session_read_arguments(arguments, false)?;
    let sessions = with_transition_session_read(&common, |project, vault, store| {
        list_sessions_with_continuity_transition(project, vault, store)
    })?;
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
    if common.json {
        let session = with_transition_session_read(&common, |project, vault, store| {
            read_session_context_with_continuity_transition(
                project,
                vault,
                store,
                session_id,
                DEFAULT_SESSION_CONTEXT_CHECKPOINTS,
                DEFAULT_SESSION_CONTEXT_CHARACTERS,
            )
        })?;
        println!(
            "{}",
            serde_json::to_string_pretty(&session).expect("session is serializable")
        );
    } else {
        let session = with_transition_session_read(&common, |project, vault, store| {
            read_session_with_continuity_transition(project, vault, store, session_id)
        })?;
        println!("Session: {} ({})", session.name, session.session_id);
        println!("Status: {}", session_status_label(session.status));
        println!("Goal: {}", session.goal);
        println!("Events: {}", session.event_count);
        println!("Checkpoints: {}", session.checkpoints.len());
        println!("Prompts observed: {}", session.prompts.len());
        println!("Responses observed: {}", session.responses.len());
        if let Some(finish) = session.finish {
            println!("Result: {}", finish.summary);
            if !finish.handoff.is_empty() {
                println!("Handoff: {}", finish.handoff);
            }
        }
    }
    Ok(())
}

fn session_turns(arguments: &[String]) -> Result<(), CliError> {
    let mut common = SessionArguments::default();
    let mut session_id = None;
    let mut max_results = ley_core::DEFAULT_SESSION_TURN_RESULTS;
    let mut max_characters = ley_core::DEFAULT_SESSION_TURN_CHARACTERS;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--max-results" => {
                index += 1;
                max_results = parse_usize(
                    required_value(arguments, index, "--max-results")?,
                    "--max-results",
                )?;
            }
            "--max-characters" => {
                index += 1;
                max_characters = parse_usize(
                    required_value(arguments, index, "--max-characters")?,
                    "--max-characters",
                )?;
            }
            value if session_id.is_none() && value.starts_with("ses_") => {
                session_id = Some(value.to_owned())
            }
            value => parse_session_common(arguments, &mut index, value, &mut common)?,
        }
        index += 1;
    }
    let session_id =
        session_id.ok_or_else(|| CliError::Usage("session turns requires SESSION".to_owned()))?;
    let turns = with_transition_session_read(&common, |project, vault, store| {
        read_session_turns_context_with_continuity_transition(
            project,
            vault,
            store,
            &session_id,
            max_results,
            max_characters,
        )
    })?;
    if common.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&turns).expect("session turns are serializable")
        );
    } else {
        println!("Session: {}", turns.session_id);
        println!(
            "Observed: {} prompts / {} responses ({} retained)",
            turns.prompt_count, turns.response_count, turns.retained_turn_count
        );
        println!("Warning: {}", turns.instruction_warning);
        for turn in turns.turns {
            println!(
                "\n{:?} · {:?} · {}",
                turn.kind, turn.retention, turn.record_id
            );
            if let Some(text) = turn.text {
                println!("{text}");
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

fn with_transition_session_read<T>(
    common: &SessionArguments,
    operation: impl FnOnce(&Path, &Path, &ContinuityStore) -> Result<T, LeyCoreError>,
) -> Result<T, CliError> {
    with_transition_session_operation(common, operation)
}

struct CliContinuityAccess {
    legacy_vault_path: PathBuf,
    binding: Option<ProjectVaultBinding>,
    native_born_registered: bool,
}

fn native_cli_legacy_placeholder(
    project_id: &str,
    store: &ContinuityStore,
) -> Result<PathBuf, LeyCoreError> {
    let parent = store.path().parent().ok_or_else(|| {
        LeyCoreError::InvalidContinuityStore(
            "continuity database has no parent directory for native CLI access".to_owned(),
        )
    })?;
    let path = parent.join("native-no-legacy-vault").join(project_id);
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(source) => Err(LeyCoreError::Io { path, source }),
        Ok(_) => Err(LeyCoreError::UnsafeProjectLayout(path)),
    }
}

fn with_cli_continuity_access<T>(
    project: &Path,
    vault_override: Option<&Path>,
    operation: impl FnOnce(&CliContinuityAccess, &ContinuityStore) -> Result<T, LeyCoreError>,
) -> Result<T, CliError> {
    let registry = BindingRegistry::system_default()?;
    let store = ContinuityStore::system_default()?;
    match registry.resolve(project, vault_override) {
        Ok(binding) => {
            let access = CliContinuityAccess {
                legacy_vault_path: binding.vault_path.clone(),
                binding: Some(binding),
                native_born_registered: false,
            };
            Ok(operation(&access, &store)?)
        }
        Err(LeyCoreError::VaultNotBound(project_id)) if vault_override.is_none() => {
            let native_ready =
                ley_core::native_canonical_read_authority_available(project, &store)?;
            let native_born_registered = native_born_project_registration_exists(project, &store)?;
            if !native_ready && !native_born_registered {
                return Err(LeyCoreError::VaultNotBound(project_id).into());
            }
            let access = CliContinuityAccess {
                legacy_vault_path: native_cli_legacy_placeholder(&project_id, &store)?,
                binding: None,
                native_born_registered,
            };
            Ok(operation(&access, &store)?)
        }
        Err(LeyCoreError::BoundVaultUnavailable { project_id, path })
            if vault_override.is_none() =>
        {
            let unavailable_path = path.clone();
            let unavailable = LeyCoreError::BoundVaultUnavailable {
                project_id: project_id.clone(),
                path: path.clone(),
            };
            let access = CliContinuityAccess {
                legacy_vault_path: path.clone(),
                binding: Some(ProjectVaultBinding {
                    project_id,
                    vault_path: path,
                    source: BindingSource::Persisted,
                }),
                native_born_registered: false,
            };
            match operation(&access, &store) {
                Ok(value) => Ok(value),
                Err(LeyCoreError::Io { path, source })
                    if source.kind() == std::io::ErrorKind::NotFound
                        && (path == unavailable_path || path.starts_with(&unavailable_path)) =>
                {
                    Err(unavailable.into())
                }
                Err(error) => Err(error.into()),
            }
        }
        Err(error) => Err(error.into()),
    }
}

fn with_transition_session_operation<T>(
    common: &SessionArguments,
    operation: impl FnOnce(&Path, &Path, &ContinuityStore) -> Result<T, LeyCoreError>,
) -> Result<T, CliError> {
    let project = common.project_path()?;
    with_cli_continuity_access(&project, common.vault.as_deref(), |access, store| {
        operation(&project, &access.legacy_vault_path, store)
    })
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
    result: &SessionWriteResult,
    json: bool,
    verb: &str,
) -> Result<(), CliError> {
    if json {
        let session = &result.session;
        let receipt = serde_json::json!({
            "eventId": result.event_id,
            "replayed": result.replayed,
            "storage": "native-continuity",
            "session": {
                "schemaVersion": session.schema_version,
                "projectId": session.project_id,
                "sessionId": session.session_id,
                "name": session.name,
                "status": session.status,
                "eventCount": session.event_count,
                "checkpointCount": session.checkpoints.len(),
                "promptCount": session.prompts.len(),
                "responseCount": session.responses.len(),
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&receipt).expect("session receipt is serializable")
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
        println!("Storage: native continuity");
    }
    Ok(())
}

fn ingest(arguments: &[String]) -> Result<(), CliError> {
    let parsed = binding_arguments(arguments, false)?;
    let (result, binding, native_storage) =
        with_cli_continuity_access(&parsed.project, parsed.vault.as_deref(), |access, store| {
            let result = if access.binding.is_none() {
                let result = ingest_project_with_native_authority(&parsed.project, store)?;
                if access.native_born_registered {
                    establish_native_born_project_authorities(&parsed.project, store)?;
                }
                result
            } else {
                ingest_project_with_continuity_transition(
                    &parsed.project,
                    &access.legacy_vault_path,
                    store,
                )?
            };
            Ok((result, access.binding.clone(), access.binding.is_none()))
        })?;
    if parsed.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "binding": binding,
                "storage": if native_storage { "native-continuity" } else { "legacy-vault" },
                "ingestion": result,
            }))
            .expect("CLI result is serializable")
        );
    } else {
        println!("Ingested project: {}", result.project_id);
        println!("Snapshot: {}", result.snapshot_id);
        if let Some(binding) = &binding {
            if binding.vault_path.exists() {
                println!(
                    "Legacy vault: {} ({})",
                    binding.vault_path.display(),
                    binding.source
                );
            } else {
                println!("Storage: native continuity (legacy vault unavailable)");
            }
        } else {
            println!("Storage: native continuity");
        }
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
            if let Some(manifest_path) = &result.manifest_path {
                println!("Legacy manifest: {manifest_path}");
            }
        } else {
            println!("No source changes; the durable snapshot was left untouched");
        }
    }
    Ok(())
}

fn bind(arguments: &[String]) -> Result<(), CliError> {
    let parsed = binding_arguments(arguments, true)?;
    let registry = BindingRegistry::system_default()?;
    let store = ContinuityStore::system_default()?;
    prepare_legacy_project_binding(&parsed.project, &store)?;
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
    let result = initialize_project_retiring_bootstrap(&root, name.as_deref(), capture)?;
    if result.created {
        let store = ContinuityStore::system_default()?;
        let catalog = ProjectCatalog::system_default()?;
        catalog.observe(&result.root)?;
        register_native_born_project(&result.root, &store)?;
    }
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
    println!(
        "  ley hook [path] --host codex|claude [--vault TEMPORARY_VAULT] [--egress-target cloud|local]"
    );
    println!("  ley mcp [path] [--vault TEMPORARY_VAULT] [--allow-session-writes]");
    println!("      [--allow-learning-proposals] [--egress-target cloud|local]");
    println!("  ley egress list [PROJECT] [--json]");
    println!("  ley egress project POLICY [PROJECT] [--json]");
    println!("  ley egress specification SPECIFICATION_ID agent-ok [PROJECT] [--json]  # clear legacy override");
    println!("  ley egress mount MOUNT_ID agent-ok [PROJECT] [--json]  # clear legacy override");
    println!(
        "  ley egress connector CONNECTOR_ID agent-ok [PROJECT] [--json]  # clear legacy override"
    );
    println!("  ley connector list [PROJECT] [--json]");
    println!("  ley connector show CONNECTOR_ID [PROJECT] [--vault TEMPORARY_VAULT] [--json]");
    println!("  ley connector remove CONNECTOR_ID [PROJECT] [--vault TEMPORARY_VAULT] [--json]");
    println!("  ley bootstrap-spec attach SOURCE_PROJECT SPECIFICATION_ID [WORKSPACE] [--json]");
    println!("  ley bootstrap-spec list [WORKSPACE] [--json]");
    println!("  ley bootstrap-spec detach GRANT_ID [WORKSPACE] [--json]");
    println!("  ley bootstrap-ref list [WORKSPACE] [--json]");
    println!("  ley bootstrap-ref detach GRANT_ID [WORKSPACE] [--json]");
    println!("  ley mount list [ACTIVE_PROJECT] [--json]");
    println!("  ley mount remove MOUNT_ID [ACTIVE_PROJECT] [--json]");
    println!("  ley scope list [--json]");
    println!("  ley scope attached [ACTIVE_PROJECT] [--json]");
    println!("  ley scope detach SCOPE_ID [ACTIVE_PROJECT] [--json]");
    println!("  ley policy-bundle list [--json]");
    println!("  ley policy-bundle attached [ACTIVE_PROJECT] [--json]");
    println!("  ley policy-bundle status [ACTIVE_PROJECT] [--json]");
    println!("  ley policy-bundle detach BUNDLE_ID [ACTIVE_PROJECT] [--json]");
    println!("  ley session start [path] --name NAME --goal GOAL [--host HOST] [--agent AGENT]");
    println!(
        "  ley session import codex-history [path] --source FILE --host-session SESSION_UUID [--vault TEMPORARY_VAULT] [--json]"
    );
    println!("  ley session prompt SESSION [path] --stdin [--request-id REQUEST] [--json]");
    println!("  ley session response SESSION [path] --stdin [--request-id REQUEST] [--json]");
    println!("  ley session checkpoint SESSION [path] --summary TEXT [--touched PATH]...");
    println!("  ley session checkpoint SESSION [path] --data CHECKPOINT.json");
    println!("  ley session finish SESSION [path] --summary TEXT [--status STATUS]");
    println!("  ley session rename SESSION [path] --name NAME --note REASON [--expected-events N]");
    println!("  ley session erase SESSION [path] --confirm-name NAME --expected-events N [--json]");
    println!("  ley session list [path] [--json]");
    println!("  ley session show SESSION [path] [--json]");
    println!("  ley session turns SESSION [path] [--max-results N] [--max-characters N] [--json]");
    println!(
        "  ley consolidation inbox [path] [--max-items N] [--max-sessions N] [--vault TEMPORARY_VAULT] [--json]"
    );
    println!("  ley resume [path] [--max-sessions N] [--max-learnings N] [--json]");
    println!("  ley search QUERY [path] [--revision COMPATIBILITY] [--max-results N] [--max-tokens N] [--json]");
    println!("  ley semantic status [--json]");
    println!("  ley semantic install [--json]");
    println!(
        "  ley learning propose [path] --actor ACTOR --provenance SOURCE --kind KIND --title TITLE"
    );
    println!("      --guidance TEXT --confidence 0..100 --evidence SESSION:RECORD...");
    println!("  ley learning correct LEARNING [path] --actor ACTOR --title TITLE --guidance TEXT");
    println!("      --confidence 0..100 --evidence SESSION:RECORD... [--note TEXT]");
    println!("  ley learning review LEARNING [path] --actor ACTOR --action ACTION [--note TEXT]");
    println!("  ley learning list [path] [--review] [--json]");
    println!("  ley learning show LEARNING [path] [--json]");
    println!("  ley doctor [path] [--json]");
    println!("  ley preview [path] [--json]");
    println!();
    println!(
        "Structured capture is the default. Ley never reads or stores complete host transcripts automatically."
    );
    println!(
        "Historical host import is explicit. The first supported format reads Codex history.jsonl user messages only; it does not invent assistant/tool history."
    );
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    BootstrapAuthorityUnavailable,
    Core(LeyCoreError),
    Mcp(ley_mcp::McpServerError),
    CurrentDirectory(std::io::Error),
    HookInput(std::io::Error),
    HookJson(serde_json::Error),
    StdinInput(std::io::Error),
    ModelDownload(String),
    InputFile {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}; run 'ley help'"),
            Self::BootstrapAuthorityUnavailable => {
                write!(
                    formatter,
                    "bootstrap Specification authority is unavailable or invalid"
                )
            }
            Self::Core(error) => error.fmt(formatter),
            Self::Mcp(error) => error.fmt(formatter),
            Self::CurrentDirectory(error) => {
                write!(formatter, "could not read current directory: {error}")
            }
            Self::HookInput(error) => write!(formatter, "could not read hook input: {error}"),
            Self::HookJson(error) => write!(formatter, "hook input is not valid JSON: {error}"),
            Self::StdinInput(error) => {
                write!(formatter, "could not read session text from stdin: {error}")
            }
            Self::ModelDownload(message) => {
                write!(formatter, "semantic model install failed: {message}")
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
