use ley_core::{
    diagnose_project, ingest_project, initialize_project, preview_capture, BindingRegistry,
    CaptureMode, LeyCoreError,
};
use std::env;
use std::path::PathBuf;

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
    CurrentDirectory(std::io::Error),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}; run 'ley help'"),
            Self::Core(error) => error.fmt(formatter),
            Self::CurrentDirectory(error) => {
                write!(formatter, "could not read current directory: {error}")
            }
        }
    }
}

impl From<LeyCoreError> for CliError {
    fn from(value: LeyCoreError) -> Self {
        Self::Core(value)
    }
}
