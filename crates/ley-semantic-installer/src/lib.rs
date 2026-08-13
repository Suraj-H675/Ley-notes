//! Explicit network installation for Ley's pinned semantic-retrieval model.
//!
//! This crate only requests the reviewed model files from Hugging Face after an explicit install
//! call. It never receives project content or search queries. `ley-core` remains the authority
//! for the supported descriptor, checksum verification, private cache layout, and atomic
//! promotion of verified staging files.

use ley_core::{
    install_semantic_model_from_staging, semantic_model_status, supported_semantic_model,
    LeyCoreError, SemanticModelDescriptor, SemanticModelFile, SemanticModelInstallation,
    SemanticModelStatus,
};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use tempfile::TempDir;
use thiserror::Error;

const SEMANTIC_MODEL_DOWNLOAD_ORIGIN: &str = "https://huggingface.co";
const DOWNLOAD_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_READ_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const DOWNLOAD_WRITE_TIMEOUT: Duration = Duration::from_secs(60);

/// Failures that occur before `ley-core` can verify and promote a staged model.
#[derive(Debug, Error)]
pub enum SemanticModelInstallerError {
    #[error("could not create a private temporary download directory")]
    StagingDirectory,
    #[error("could not make the temporary download directory private")]
    StagingDirectoryPermissions,
    #[error("the supported semantic model descriptor is invalid: {0}")]
    InvalidDescriptor(String),
    #[error("could not download {file}: {source}")]
    Download { file: String, source: ureq::Error },
    #[error("could not create staged {file}")]
    StagingFile { file: String },
    #[error("download of {file} was interrupted")]
    DownloadInterrupted { file: String },
    #[error("could not finish staging {file}")]
    StagingFlush { file: String },
    #[error("could not sync staged {file}")]
    StagingSync { file: String },
    #[error("downloaded {file} had an unexpected size")]
    UnexpectedSize { file: String },
    #[error(transparent)]
    Core(#[from] LeyCoreError),
}

/// Downloads and installs the only semantic model supported by Ley.
///
/// If the verified model is already installed, this does not issue a network request and returns
/// an installation result with `installed: false`.
pub fn install_supported_semantic_model(
) -> Result<SemanticModelInstallation, SemanticModelInstallerError> {
    install_supported_semantic_model_with_progress(|_| {})
}

/// Downloads and installs Ley's pinned semantic model, calling `on_download` once before each
/// fixed model file is requested. The callback receives metadata only, never file content.
pub fn install_supported_semantic_model_with_progress<F>(
    mut on_download: F,
) -> Result<SemanticModelInstallation, SemanticModelInstallerError>
where
    F: FnMut(SemanticModelFile),
{
    if let SemanticModelStatus::Ready { model } = semantic_model_status() {
        return Ok(SemanticModelInstallation {
            model,
            installed: false,
        });
    }

    let model = supported_semantic_model();
    validate_model_descriptor(&model)?;
    let staging = private_staging_directory()?;
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(DOWNLOAD_CONNECT_TIMEOUT)
        .timeout_read(DOWNLOAD_READ_TIMEOUT)
        .timeout_write(DOWNLOAD_WRITE_TIMEOUT)
        .build();

    for file in &model.files {
        on_download(*file);
        download_model_file(&agent, &model, *file, staging.path())?;
    }

    Ok(install_semantic_model_from_staging(staging.path())?)
}

fn private_staging_directory() -> Result<TempDir, SemanticModelInstallerError> {
    let staging = tempfile::Builder::new()
        .prefix("ley-semantic-model-")
        .tempdir()
        .map_err(|_| SemanticModelInstallerError::StagingDirectory)?;
    set_private_directory(staging.path())?;
    Ok(staging)
}

fn download_model_file(
    agent: &ureq::Agent,
    model: &SemanticModelDescriptor,
    file: SemanticModelFile,
    staging_directory: &Path,
) -> Result<(), SemanticModelInstallerError> {
    let url = model_file_url(model, file)?;
    let response =
        agent
            .get(&url)
            .call()
            .map_err(|source| SemanticModelInstallerError::Download {
                file: file.name.to_owned(),
                source,
            })?;
    let destination = staging_directory.join(file.name);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output =
        options
            .open(&destination)
            .map_err(|_| SemanticModelInstallerError::StagingFile {
                file: file.name.to_owned(),
            })?;
    let mut input = response.into_reader().take(file.bytes.saturating_add(1));
    let copied = std::io::copy(&mut input, &mut output).map_err(|_| {
        SemanticModelInstallerError::DownloadInterrupted {
            file: file.name.to_owned(),
        }
    })?;
    output
        .flush()
        .map_err(|_| SemanticModelInstallerError::StagingFlush {
            file: file.name.to_owned(),
        })?;
    output
        .sync_all()
        .map_err(|_| SemanticModelInstallerError::StagingSync {
            file: file.name.to_owned(),
        })?;
    if copied != file.bytes {
        return Err(SemanticModelInstallerError::UnexpectedSize {
            file: file.name.to_owned(),
        });
    }
    Ok(())
}

fn model_file_url(
    model: &SemanticModelDescriptor,
    file: SemanticModelFile,
) -> Result<String, SemanticModelInstallerError> {
    validate_model_id(&model.model_id)?;
    validate_url_segment(&model.revision, "revision")?;
    validate_url_segment(file.name, "file name")?;
    Ok(format!(
        "{SEMANTIC_MODEL_DOWNLOAD_ORIGIN}/{}/resolve/{}/{}",
        model.model_id, model.revision, file.name
    ))
}

fn validate_model_descriptor(
    model: &SemanticModelDescriptor,
) -> Result<(), SemanticModelInstallerError> {
    validate_model_id(&model.model_id)?;
    validate_url_segment(&model.revision, "revision")?;
    if model.files.is_empty() {
        return Err(SemanticModelInstallerError::InvalidDescriptor(
            "it has no model files".to_owned(),
        ));
    }
    for file in &model.files {
        validate_url_segment(file.name, "file name")?;
        if file.bytes == 0 {
            return Err(SemanticModelInstallerError::InvalidDescriptor(format!(
                "{} has no expected size",
                file.name
            )));
        }
        if file.sha256.len() != 64 || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(SemanticModelInstallerError::InvalidDescriptor(format!(
                "{} has an invalid checksum",
                file.name
            )));
        }
    }
    Ok(())
}

fn validate_model_id(model_id: &str) -> Result<(), SemanticModelInstallerError> {
    let mut components = model_id.split('/');
    let Some(owner) = components.next() else {
        return Err(SemanticModelInstallerError::InvalidDescriptor(
            "the model ID has no owner".to_owned(),
        ));
    };
    let Some(name) = components.next() else {
        return Err(SemanticModelInstallerError::InvalidDescriptor(
            "the model ID has no name".to_owned(),
        ));
    };
    if components.next().is_some() {
        return Err(SemanticModelInstallerError::InvalidDescriptor(
            "the model ID has an unsafe path".to_owned(),
        ));
    }
    validate_url_segment(owner, "model owner")?;
    validate_url_segment(name, "model name")
}

fn validate_url_segment(value: &str, label: &str) -> Result<(), SemanticModelInstallerError> {
    if value.is_empty()
        || value.len() > 256
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(SemanticModelInstallerError::InvalidDescriptor(format!(
            "the {label} is not a safe URL segment"
        )));
    }
    Ok(())
}

fn set_private_directory(path: &Path) -> Result<(), SemanticModelInstallerError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| SemanticModelInstallerError::StagingDirectoryPermissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
