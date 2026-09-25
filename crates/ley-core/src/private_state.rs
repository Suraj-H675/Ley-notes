use crate::LeyCoreError;
use directories::BaseDirs;
use std::env;
use std::path::PathBuf;

#[cfg(feature = "eval-private-root")]
use std::ffi::OsStr;
#[cfg(feature = "eval-private-root")]
use std::fs;
#[cfg(feature = "eval-private-root")]
use std::path::{Component, Path};

pub const EVAL_PRIVATE_ROOT_ENV: &str = "LEY_EVAL_PRIVATE_ROOT";

pub(crate) fn default_private_config_dir() -> Result<PathBuf, LeyCoreError> {
    if let Some(root) = evaluation_private_root()? {
        return Ok(root.join("config"));
    }
    let base = BaseDirs::new().ok_or(LeyCoreError::ConfigDirectoryUnavailable)?;
    Ok(base.config_dir().to_path_buf())
}

pub(crate) fn default_private_cache_dir() -> Result<PathBuf, LeyCoreError> {
    if let Some(root) = evaluation_private_root()? {
        return Ok(root.join("cache"));
    }
    let base = BaseDirs::new().ok_or(LeyCoreError::ConfigDirectoryUnavailable)?;
    Ok(base.cache_dir().to_path_buf())
}

fn evaluation_private_root() -> Result<Option<PathBuf>, LeyCoreError> {
    let Some(value) = env::var_os(EVAL_PRIVATE_ROOT_ENV) else {
        return Ok(None);
    };

    #[cfg(not(feature = "eval-private-root"))]
    {
        let _ = value;
        return Err(LeyCoreError::InvalidEvalPrivateRoot(
            "LEY_EVAL_PRIVATE_ROOT is accepted only by builds with the eval-private-root feature"
                .to_owned(),
        ));
    }

    #[cfg(feature = "eval-private-root")]
    {
        validate_evaluation_private_root(value.as_os_str()).map(Some)
    }
}

#[cfg(feature = "eval-private-root")]
fn validate_evaluation_private_root(value: &OsStr) -> Result<PathBuf, LeyCoreError> {
    if value.is_empty() {
        return Err(invalid_root("must not be empty"));
    }
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(invalid_root("must be an absolute path"));
    }
    if path.parent().is_none() {
        return Err(invalid_root("must not be a filesystem root"));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(invalid_root("must not contain '.' or '..' components"));
    }

    validate_eval_directory(path, "root")?;
    validate_eval_directory(&path.join("config"), "config child")?;
    validate_eval_directory(&path.join("cache"), "cache child")?;

    path.canonicalize()
        .map_err(|_| invalid_root("could not be canonicalized"))
}

#[cfg(feature = "eval-private-root")]
fn validate_eval_directory(path: &Path, label: &str) -> Result<(), LeyCoreError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| invalid_root(&format!("{label} must already exist as a directory")))?;
    if metadata.file_type().is_symlink() || windows_reparse_point(&metadata) {
        return Err(invalid_root(&format!(
            "{label} must not be a symlink or reparse point"
        )));
    }
    if !metadata.is_dir() {
        return Err(invalid_root(&format!("{label} must be a directory")));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(invalid_root(&format!(
                "{label} must have owner-only permissions"
            )));
        }
    }
    Ok(())
}

#[cfg(all(feature = "eval-private-root", windows))]
fn windows_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(all(feature = "eval-private-root", not(windows)))]
fn windows_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(feature = "eval-private-root")]
fn invalid_root(message: &str) -> LeyCoreError {
    LeyCoreError::InvalidEvalPrivateRoot(message.to_owned())
}

#[cfg(all(test, feature = "eval-private-root"))]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(unix)]
    fn make_private(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }

    #[cfg(not(unix))]
    fn make_private(_path: &Path) {}

    fn valid_tree() -> (tempfile::TempDir, PathBuf) {
        let base = tempdir().unwrap();
        let root = base.path().join("private");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("config")).unwrap();
        fs::create_dir(root.join("cache")).unwrap();
        make_private(&root);
        make_private(&root.join("config"));
        make_private(&root.join("cache"));
        (base, root)
    }

    #[test]
    fn evaluation_private_root_requires_an_existing_absolute_private_tree() {
        let (_base, root) = valid_tree();
        assert_eq!(
            validate_evaluation_private_root(root.as_os_str()).unwrap(),
            root.canonicalize().unwrap()
        );

        assert!(matches!(
            validate_evaluation_private_root(OsStr::new("")),
            Err(LeyCoreError::InvalidEvalPrivateRoot(message))
                if message.contains("empty")
        ));
        assert!(matches!(
            validate_evaluation_private_root(OsStr::new("relative/private")),
            Err(LeyCoreError::InvalidEvalPrivateRoot(message))
                if message.contains("absolute")
        ));
        let parent_component = root.join("child").join("..");
        assert!(matches!(
            validate_evaluation_private_root(parent_component.as_os_str()),
            Err(LeyCoreError::InvalidEvalPrivateRoot(message))
                if message.contains("'.' or '..'")
        ));
        assert!(matches!(
            validate_evaluation_private_root(root.join("missing").as_os_str()),
            Err(LeyCoreError::InvalidEvalPrivateRoot(message))
                if message.contains("already exist")
        ));
    }

    #[test]
    fn evaluation_private_root_rejects_files_and_missing_children() {
        let base = tempdir().unwrap();
        let file = base.path().join("file");
        fs::write(&file, b"not a directory").unwrap();
        assert!(matches!(
            validate_evaluation_private_root(file.as_os_str()),
            Err(LeyCoreError::InvalidEvalPrivateRoot(message))
                if message.contains("directory")
        ));

        let root = base.path().join("private");
        fs::create_dir(&root).unwrap();
        make_private(&root);
        assert!(matches!(
            validate_evaluation_private_root(root.as_os_str()),
            Err(LeyCoreError::InvalidEvalPrivateRoot(message))
                if message.contains("config child")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn evaluation_private_root_rejects_non_private_or_symlinked_children() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let (_base, root) = valid_tree();
        fs::set_permissions(root.join("cache"), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            validate_evaluation_private_root(root.as_os_str()),
            Err(LeyCoreError::InvalidEvalPrivateRoot(message))
                if message.contains("owner-only")
        ));

        fs::remove_dir(root.join("cache")).unwrap();
        let target = root.join("cache-target");
        fs::create_dir(&target).unwrap();
        make_private(&target);
        symlink(&target, root.join("cache")).unwrap();
        assert!(matches!(
            validate_evaluation_private_root(root.as_os_str()),
            Err(LeyCoreError::InvalidEvalPrivateRoot(message))
                if message.contains("symlink")
        ));
    }
}
