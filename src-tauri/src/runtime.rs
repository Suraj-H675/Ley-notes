use atomic_write_file::AtomicWriteFile;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tauri::{path::BaseDirectory, AppHandle, Manager};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentRuntimeStatus {
    pub(crate) helper_path: PathBuf,
    pub(crate) version: &'static str,
    pub(crate) sha256: String,
}

pub(crate) fn ensure_bundled_helper(app: &AppHandle) -> Result<AgentRuntimeStatus, String> {
    let resource_name = helper_resource_name();
    let packaged = app
        .path()
        .resolve(resource_name, BaseDirectory::Resource)
        .map_err(|error| format!("could not resolve the packaged Ley helper: {error}"))?;
    let packaged = if packaged.is_file() {
        packaged
    } else {
        #[cfg(debug_assertions)]
        {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("generated")
                .join(helper_file_name())
        }
        #[cfg(not(debug_assertions))]
        {
            packaged
        }
    };

    let destination = installed_helper_path(app)?;
    install_helper(&packaged, &destination)?;

    let bytes = fs::read(&destination)
        .map_err(|error| format!("could not verify installed Ley helper: {error}"))?;
    Ok(AgentRuntimeStatus {
        helper_path: destination,
        version: env!("CARGO_PKG_VERSION"),
        sha256: hex_sha256(&bytes),
    })
}

pub(crate) fn installed_helper_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|error| format!("could not resolve Ley's private application data: {error}"))?
        .join("engine")
        .join(installed_helper_name()))
}

fn install_helper(source: &Path, destination: &Path) -> Result<(), String> {
    let source_bytes = fs::read(source).map_err(|error| {
        format!(
            "the packaged Ley helper is unavailable at {}: {error}",
            source.display()
        )
    })?;

    if let Ok(existing) = fs::read(destination) {
        if existing == source_bytes {
            secure_helper_permissions(destination)?;
            return Ok(());
        }
    }

    let parent = destination
        .parent()
        .ok_or_else(|| "Ley helper destination has no parent directory".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create Ley's private engine directory: {error}"))?;
    secure_directory_permissions(parent)?;

    let mut options = AtomicWriteFile::options();
    #[cfg(unix)]
    options.mode(0o700);
    let mut file = options
        .open(destination)
        .map_err(|error| format!("could not stage the Ley helper update: {error}"))?;
    file.write_all(&source_bytes)
        .map_err(|error| format!("could not write the Ley helper update: {error}"))?;
    file.commit()
        .map_err(|error| format!("could not atomically install the Ley helper: {error}"))?;
    secure_helper_permissions(destination)
}

#[cfg(unix)]
fn secure_directory_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("could not protect Ley's private engine directory: {error}"))
}

#[cfg(not(unix))]
fn secure_directory_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn secure_helper_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("could not protect the installed Ley helper: {error}"))
}

#[cfg(not(unix))]
fn secure_helper_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn helper_resource_name() -> &'static str {
    if cfg!(windows) {
        "generated/ley-helper.exe"
    } else {
        "generated/ley-helper"
    }
}

#[cfg(any(test, debug_assertions))]
fn helper_file_name() -> &'static str {
    if cfg!(windows) {
        "ley-helper.exe"
    } else {
        "ley-helper"
    }
}

fn installed_helper_name() -> &'static str {
    if cfg!(windows) {
        "ley.exe"
    } else {
        "ley"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_install_is_private_and_idempotent() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join(helper_file_name());
        let destination = root
            .path()
            .join("private/engine")
            .join(installed_helper_name());
        fs::write(&source, b"ley helper v1").unwrap();

        install_helper(&source, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"ley helper v1");
        let first_hash = hex_sha256(&fs::read(&destination).unwrap());

        install_helper(&source, &destination).unwrap();
        assert_eq!(hex_sha256(&fs::read(&destination).unwrap()), first_hash);

        fs::write(&source, b"ley helper v2").unwrap();
        install_helper(&source, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"ley helper v2");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(destination.parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }
}
