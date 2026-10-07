use crate::LeyCoreError;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use std::ffi::OsStr;
use std::io::Read;
use std::path::{Component, Path};

pub(crate) fn read_scoped_file(
    root: &Dir,
    path: &Path,
    expected: u64,
    limit: u64,
) -> Result<Vec<u8>, LeyCoreError> {
    let fail = || LeyCoreError::ProjectChangedDuringIngestion(path.to_string_lossy().into_owned());
    let mut directory = root.try_clone().map_err(|source| LeyCoreError::Io {
        path: path.to_owned(),
        source,
    })?;
    let components: Vec<_> = path.components().collect();
    if components.is_empty() {
        return Err(fail());
    }
    for component in &components[..components.len() - 1] {
        let Component::Normal(name) = component else {
            return Err(fail());
        };
        directory = directory
            .open_dir_nofollow(name)
            .map_err(|source| LeyCoreError::Io {
                path: path.to_owned(),
                source,
            })?;
    }
    let Component::Normal(name) = components[components.len() - 1] else {
        return Err(fail());
    };
    let named_before = directory
        .symlink_metadata(name)
        .map_err(|source| LeyCoreError::Io {
            path: path.to_owned(),
            source,
        })?;
    if named_before.file_type().is_symlink() || !named_before.is_file() {
        return Err(fail());
    }
    let mut file = open_scoped_file(&directory, name, path)?;
    let before = file.metadata().map_err(|source| LeyCoreError::Io {
        path: path.to_owned(),
        source,
    })?;
    if !before.is_file()
        || before.len() != expected
        || expected > limit
        || !same_file(&before, &named_before)
    {
        return Err(fail());
    }
    let mut bytes = Vec::with_capacity(expected as usize);
    file.by_ref()
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| LeyCoreError::Io {
            path: path.to_owned(),
            source,
        })?;
    let after = file.metadata().map_err(|source| LeyCoreError::Io {
        path: path.to_owned(),
        source,
    })?;
    let named_after = directory
        .symlink_metadata(name)
        .map_err(|source| LeyCoreError::Io {
            path: path.to_owned(),
            source,
        })?;
    if !same_file(&after, &named_after)
        || !same_file(&before, &after)
        || bytes.len() as u64 != expected
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(fail());
    }
    Ok(bytes)
}

fn open_scoped_file(
    directory: &Dir,
    name: &OsStr,
    path: &Path,
) -> Result<cap_std::fs::File, LeyCoreError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    directory
        .open_with(name, &options)
        .map_err(|source| LeyCoreError::Io {
            path: path.to_owned(),
            source,
        })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn fifo_substitution_after_regular_file_check_cannot_block_open() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file.txt");
        std::fs::write(&path, "eligible").unwrap();
        let dir = Dir::open_ambient_dir(temp.path(), cap_std::ambient_authority()).unwrap();
        assert!(dir.symlink_metadata("file.txt").unwrap().is_file());
        std::fs::remove_file(&path).unwrap();
        assert!(std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success());
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let file = open_scoped_file(&dir, OsStr::new("file.txt"), &path).unwrap();
            send.send(file.metadata().unwrap().is_file()).unwrap();
        });
        assert!(!receive
            .recv_timeout(Duration::from_secs(2))
            .expect("FIFO substitution blocked acquisition"));
    }
}

fn same_file(left: &cap_std::fs::Metadata, right: &cap_std::fs::Metadata) -> bool {
    if left.len() != right.len() || left.modified().ok() != right.modified().ok() {
        return false;
    }
    #[cfg(unix)]
    {
        use cap_std::fs::MetadataExt;
        left.dev() == right.dev()
            && left.ino() == right.ino()
            && left.ctime() == right.ctime()
            && left.ctime_nsec() == right.ctime_nsec()
    }
    #[cfg(not(unix))]
    {
        left.created().ok() == right.created().ok()
    }
}
