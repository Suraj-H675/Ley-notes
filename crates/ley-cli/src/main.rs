use ley_core::{diagnose_project, initialize_project, preview_capture, CaptureMode, LeyCoreError};
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
