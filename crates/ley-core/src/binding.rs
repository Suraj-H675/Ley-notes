use crate::{
    canonical_directory, diagnose_project, validate_project_id, LeyCoreError, ProjectCatalog,
    METADATA_FILE_LIMIT_BYTES,
};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const APP_IDENTIFIER: &str = "app.leynotes.desktop";
pub const BINDING_REGISTRY_FILE: &str = "bindings-v1.json";
const BINDING_LOCK_FILE: &str = "bindings-v1.lock";
pub const BINDING_REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingSource {
    Persisted,
    Override,
}

impl std::fmt::Display for BindingSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Persisted => "persisted",
            Self::Override => "override",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectVaultBinding {
    pub project_id: String,
    pub vault_path: PathBuf,
    pub source: BindingSource,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RegistryDocument {
    schema_version: u32,
    bindings: BTreeMap<String, String>,
}

impl RegistryDocument {
    fn empty() -> Self {
        Self {
            schema_version: BINDING_REGISTRY_SCHEMA_VERSION,
            bindings: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), LeyCoreError> {
        if self.schema_version != BINDING_REGISTRY_SCHEMA_VERSION {
            return Err(LeyCoreError::InvalidBindingRegistry(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        for (project_id, vault_path) in &self.bindings {
            validate_project_id(project_id).map_err(|error| {
                LeyCoreError::InvalidBindingRegistry(format!(
                    "invalid project ID key '{project_id}': {error}"
                ))
            })?;
            if vault_path.is_empty() || !Path::new(vault_path).is_absolute() {
                return Err(LeyCoreError::InvalidBindingRegistry(format!(
                    "vault path for {project_id} must be an absolute UTF-8 path"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BindingRegistry {
    path: PathBuf,
    project_catalog: ProjectCatalog,
}

impl BindingRegistry {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        Ok(Self::at(default_binding_registry_path()?))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let project_catalog = ProjectCatalog::at(path.with_file_name(crate::PROJECT_CATALOG_FILE));
        Self {
            path,
            project_catalog,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bind(
        &self,
        project_start: impl AsRef<Path>,
        vault: impl AsRef<Path>,
    ) -> Result<ProjectVaultBinding, LeyCoreError> {
        let diagnostic = diagnose_project(project_start)?;
        self.project_catalog.observe_diagnostic(&diagnostic)?;
        let vault_path = canonical_directory(vault.as_ref())?;
        let vault_string = vault_path
            .to_str()
            .ok_or_else(|| LeyCoreError::NonUtf8Path(vault_path.clone()))?
            .to_owned();
        let project_id = diagnostic.identity.project_id.clone();

        self.mutate(|document| {
            document.bindings.insert(project_id.clone(), vault_string);
            Ok(())
        })?;

        Ok(ProjectVaultBinding {
            project_id,
            vault_path,
            source: BindingSource::Persisted,
        })
    }

    pub fn resolve(
        &self,
        project_start: impl AsRef<Path>,
        vault_override: Option<&Path>,
    ) -> Result<ProjectVaultBinding, LeyCoreError> {
        let diagnostic = diagnose_project(project_start)?;
        self.project_catalog.observe_diagnostic(&diagnostic)?;
        self.resolve_diagnostic(&diagnostic, vault_override)
    }

    pub fn resolve_observed(
        &self,
        diagnostic: &crate::ProjectDiagnostic,
    ) -> Result<ProjectVaultBinding, LeyCoreError> {
        self.resolve_diagnostic(diagnostic, None)
    }

    fn resolve_diagnostic(
        &self,
        diagnostic: &crate::ProjectDiagnostic,
        vault_override: Option<&Path>,
    ) -> Result<ProjectVaultBinding, LeyCoreError> {
        let project_id = diagnostic.identity.project_id.clone();

        if let Some(override_path) = vault_override {
            return Ok(ProjectVaultBinding {
                project_id: project_id.clone(),
                vault_path: canonical_directory(override_path)?,
                source: BindingSource::Override,
            });
        }

        let document = self.read_locked()?;
        let stored = document
            .bindings
            .get(&project_id)
            .ok_or_else(|| LeyCoreError::VaultNotBound(project_id.clone()))?;
        let stored_path = PathBuf::from(stored);
        let vault_path =
            canonical_directory(&stored_path).map_err(|_| LeyCoreError::BoundVaultUnavailable {
                project_id: project_id.clone(),
                path: stored_path.clone(),
            })?;
        Ok(ProjectVaultBinding {
            project_id,
            vault_path,
            source: BindingSource::Persisted,
        })
    }

    pub fn unbind(
        &self,
        project_start: impl AsRef<Path>,
    ) -> Result<Option<ProjectVaultBinding>, LeyCoreError> {
        let diagnostic = diagnose_project(project_start)?;
        self.project_catalog.observe_diagnostic(&diagnostic)?;
        let project_id = diagnostic.identity.project_id;
        let removed = self.mutate(|document| Ok(document.bindings.remove(&project_id)))?;
        Ok(removed.map(|vault_path| ProjectVaultBinding {
            project_id,
            vault_path: PathBuf::from(vault_path),
            source: BindingSource::Persisted,
        }))
    }

    fn read_locked(&self) -> Result<RegistryDocument, LeyCoreError> {
        let lock = self.acquire_lock()?;
        let result = self.read_document();
        let unlock_result = File::unlock(&lock);
        match (result, unlock_result) {
            (Ok(document), Ok(())) => Ok(document),
            (Err(error), _) => Err(error),
            (Ok(_), Err(source)) => Err(LeyCoreError::Io {
                path: self.lock_path(),
                source,
            }),
        }
    }

    fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut RegistryDocument) -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let lock = self.acquire_lock()?;
        let result = (|| {
            let mut document = self.read_document()?;
            let value = operation(&mut document)?;
            document.validate()?;
            self.write_document(&document)?;
            Ok(value)
        })();
        let unlock_result = File::unlock(&lock);
        match (result, unlock_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(source)) => Err(LeyCoreError::Io {
                path: self.lock_path(),
                source,
            }),
        }
    }

    fn acquire_lock(&self) -> Result<File, LeyCoreError> {
        self.prepare_parent()?;
        let lock_path = self.lock_path();
        reject_non_regular_if_present(&lock_path)?;
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options
            .open(&lock_path)
            .map_err(|source| LeyCoreError::Io {
                path: lock_path.clone(),
                source,
            })?;
        lock.lock().map_err(|source| LeyCoreError::Io {
            path: lock_path,
            source,
        })?;
        Ok(lock)
    }

    fn prepare_parent(&self) -> Result<(), LeyCoreError> {
        let parent = self.path.parent().ok_or_else(|| {
            LeyCoreError::InvalidBindingRegistry(
                "registry path must have a parent directory".to_owned(),
            )
        })?;
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(parent).map_err(|source| LeyCoreError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        let metadata = fs::symlink_metadata(parent).map_err(|source| LeyCoreError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LeyCoreError::UnsafeProjectLayout(parent.to_path_buf()));
        }
        Ok(())
    }

    fn read_document(&self) -> Result<RegistryDocument, LeyCoreError> {
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(RegistryDocument::empty())
            }
            Err(source) => {
                return Err(LeyCoreError::Io {
                    path: self.path.clone(),
                    source,
                })
            }
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(LeyCoreError::UnsafeProjectLayout(self.path.clone()))
            }
            Ok(metadata) if metadata.len() > METADATA_FILE_LIMIT_BYTES => {
                return Err(LeyCoreError::MetadataTooLarge {
                    path: self.path.clone(),
                    limit_bytes: METADATA_FILE_LIMIT_BYTES,
                })
            }
            Ok(_) => {}
        }
        reject_non_regular_if_present(&self.path)?;
        let bytes = fs::read(&self.path).map_err(|source| LeyCoreError::Io {
            path: self.path.clone(),
            source,
        })?;
        let document: RegistryDocument =
            serde_json::from_slice(&bytes).map_err(|source| LeyCoreError::Json {
                path: self.path.clone(),
                source,
            })?;
        document.validate()?;
        Ok(document)
    }

    fn write_document(&self, document: &RegistryDocument) -> Result<(), LeyCoreError> {
        reject_non_regular_if_present(&self.path)?;
        let mut body = serde_json::to_vec_pretty(document)
            .expect("validated binding registry is serializable");
        body.push(b'\n');
        if body.len() as u64 > METADATA_FILE_LIMIT_BYTES {
            return Err(LeyCoreError::MetadataTooLarge {
                path: self.path.clone(),
                limit_bytes: METADATA_FILE_LIMIT_BYTES,
            });
        }

        let mut options = atomic_write_file::OpenOptions::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&self.path)
            .map_err(|source| LeyCoreError::Io {
                path: self.path.clone(),
                source,
            })?;
        file.write_all(&body).map_err(|source| LeyCoreError::Io {
            path: self.path.clone(),
            source,
        })?;
        file.commit().map_err(|source| LeyCoreError::Io {
            path: self.path.clone(),
            source,
        })
    }

    fn lock_path(&self) -> PathBuf {
        if self.path.file_name().and_then(|name| name.to_str()) == Some(BINDING_REGISTRY_FILE) {
            self.path.with_file_name(BINDING_LOCK_FILE)
        } else {
            self.path.with_extension("lock")
        }
    }
}

pub fn default_binding_registry_path() -> Result<PathBuf, LeyCoreError> {
    let base = BaseDirs::new().ok_or(LeyCoreError::ConfigDirectoryUnavailable)?;
    Ok(base
        .config_dir()
        .join(APP_IDENTIFIER)
        .join(BINDING_REGISTRY_FILE))
}

fn reject_non_regular_if_present(path: &Path) -> Result<(), LeyCoreError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(LeyCoreError::UnsafeProjectLayout(path.to_path_buf()))
        }
        #[cfg(unix)]
        Ok(metadata) => {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(LeyCoreError::InvalidBindingRegistry(format!(
                    "private file permissions required for {}; use mode 600",
                    path.display()
                )));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        Ok(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{initialize_project, CaptureMode};
    use std::sync::{Arc, Barrier};
    use tempfile::tempdir;

    fn make_test_registry_private(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    #[test]
    fn binding_survives_project_moves_and_requires_rebind_after_vault_moves() {
        let base = tempdir().unwrap();
        let original_project = base.path().join("project-before");
        let moved_project = base.path().join("project-after");
        let original_vault = base.path().join("vault-before");
        let moved_vault = base.path().join("vault-after");
        fs::create_dir(&original_project).unwrap();
        fs::create_dir(&original_vault).unwrap();
        let initialized = initialize_project(
            &original_project,
            Some("Private project"),
            CaptureMode::Structured,
        )
        .unwrap();
        let registry_path = base.path().join("config/ley/bindings.json");
        let registry = BindingRegistry::at(&registry_path);

        let bound = registry.bind(&original_project, &original_vault).unwrap();
        assert_eq!(bound.project_id, initialized.identity.project_id);
        assert_eq!(bound.source, BindingSource::Persisted);

        fs::rename(&original_project, &moved_project).unwrap();
        assert_eq!(
            registry.resolve(&moved_project, None).unwrap().vault_path,
            original_vault.canonicalize().unwrap()
        );

        fs::rename(&original_vault, &moved_vault).unwrap();
        assert!(matches!(
            registry.resolve(&moved_project, None),
            Err(LeyCoreError::BoundVaultUnavailable { .. })
        ));
        assert_eq!(
            registry
                .bind(&moved_project, &moved_vault)
                .unwrap()
                .vault_path,
            moved_vault.canonicalize().unwrap()
        );

        let body = fs::read_to_string(registry_path).unwrap();
        assert!(!body.contains("project-before"));
        assert!(!body.contains("project-after"));
        assert!(!body.contains("Private project"));
        assert!(body.contains(&initialized.identity.project_id));
        assert!(body.contains(moved_vault.to_str().unwrap()));
    }

    #[test]
    fn override_is_temporary_and_unbind_is_explicit() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let persisted_vault = base.path().join("persisted");
        let override_vault = base.path().join("override");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&persisted_vault).unwrap();
        fs::create_dir(&override_vault).unwrap();
        initialize_project(&project, None, CaptureMode::Structured).unwrap();
        let registry = BindingRegistry::at(base.path().join("config/bindings.json"));
        registry.bind(&project, &persisted_vault).unwrap();

        let temporary = registry.resolve(&project, Some(&override_vault)).unwrap();
        assert_eq!(temporary.source, BindingSource::Override);
        assert_eq!(temporary.vault_path, override_vault.canonicalize().unwrap());
        assert_eq!(
            registry.resolve(&project, None).unwrap().vault_path,
            persisted_vault.canonicalize().unwrap()
        );

        let removed = registry.unbind(&project).unwrap().unwrap();
        assert_eq!(removed.vault_path, persisted_vault.canonicalize().unwrap());
        assert!(registry.unbind(&project).unwrap().is_none());
        assert!(matches!(
            registry.resolve(&project, None),
            Err(LeyCoreError::VaultNotBound(_))
        ));
    }

    #[test]
    fn concurrent_bindings_do_not_overwrite_each_other() {
        let base = tempdir().unwrap();
        let registry = BindingRegistry::at(base.path().join("config/bindings.json"));
        let barrier = Arc::new(Barrier::new(8));
        let mut workers = Vec::new();

        for index in 0..8 {
            let project = base.path().join(format!("project-{index}"));
            let vault = base.path().join(format!("vault-{index}"));
            fs::create_dir(&project).unwrap();
            fs::create_dir(&vault).unwrap();
            initialize_project(&project, None, CaptureMode::Structured).unwrap();
            let worker_registry = registry.clone();
            let worker_barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                worker_barrier.wait();
                worker_registry.bind(project, vault).unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }

        let document = registry.read_locked().unwrap();
        assert_eq!(document.bindings.len(), 8);
        let body = fs::read_to_string(registry.path()).unwrap();
        let keys = document.bindings.keys().collect::<Vec<_>>();
        let sorted = {
            let mut values = keys.clone();
            values.sort();
            values
        };
        assert_eq!(keys, sorted);
        assert!(body.ends_with('\n'));
    }

    #[test]
    fn corrupt_oversized_and_non_regular_registries_are_rejected() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&vault).unwrap();
        initialize_project(&project, None, CaptureMode::Structured).unwrap();
        let registry_path = base.path().join("config/bindings.json");
        fs::create_dir_all(registry_path.parent().unwrap()).unwrap();
        let registry = BindingRegistry::at(&registry_path);

        fs::write(
            &registry_path,
            b"{\"schemaVersion\":1,\"bindings\":[],\"extra\":true}",
        )
        .unwrap();
        make_test_registry_private(&registry_path);
        assert!(matches!(
            registry.resolve(&project, None),
            Err(LeyCoreError::Json { .. })
        ));
        fs::write(
            &registry_path,
            vec![b' '; (METADATA_FILE_LIMIT_BYTES + 1) as usize],
        )
        .unwrap();
        make_test_registry_private(&registry_path);
        assert!(matches!(
            registry.resolve(&project, None),
            Err(LeyCoreError::MetadataTooLarge { .. })
        ));

        fs::remove_file(&registry_path).unwrap();
        fs::create_dir(&registry_path).unwrap();
        assert!(matches!(
            registry.bind(&project, &vault),
            Err(LeyCoreError::UnsafeProjectLayout(_))
        ));
    }

    #[test]
    fn registry_rejects_noncanonical_project_ids_and_relative_vaults() {
        let mut document = RegistryDocument::empty();
        document.bindings.insert(
            "prj_01234567-89ab-cdef-0123-456789abcdef".to_owned(),
            "/vault".to_owned(),
        );
        assert!(matches!(
            document.validate(),
            Err(LeyCoreError::InvalidBindingRegistry(_))
        ));

        document.bindings.clear();
        document.bindings.insert(
            "prj_0123456789abcdef0123456789abcdef".to_owned(),
            "relative/vault".to_owned(),
        );
        assert!(matches!(
            document.validate(),
            Err(LeyCoreError::InvalidBindingRegistry(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn registry_files_are_private_and_symlinks_are_rejected() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&vault).unwrap();
        initialize_project(&project, None, CaptureMode::Structured).unwrap();
        let registry_path = base.path().join("private/config/bindings.json");
        let registry = BindingRegistry::at(&registry_path);
        registry.bind(&project, &vault).unwrap();

        assert_eq!(
            fs::metadata(registry_path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&registry_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(registry.lock_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );

        fs::remove_file(&registry_path).unwrap();
        let outside = base.path().join("outside.json");
        fs::write(&outside, b"do not replace").unwrap();
        symlink(&outside, &registry_path).unwrap();
        assert!(matches!(
            registry.bind(&project, &vault),
            Err(LeyCoreError::UnsafeProjectLayout(_))
        ));
        assert_eq!(fs::read_to_string(outside).unwrap(), "do not replace");

        fs::remove_file(&registry_path).unwrap();
        fs::write(&registry_path, b"{\"schemaVersion\":1,\"bindings\":{}}").unwrap();
        fs::set_permissions(&registry_path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            registry.resolve(&project, None),
            Err(LeyCoreError::InvalidBindingRegistry(_))
        ));
    }
}
