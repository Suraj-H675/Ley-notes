use crate::{
    default_binding_registry_path, diagnose_project, validate_project_id, LeyCoreError,
    ProjectDiagnostic, METADATA_FILE_LIMIT_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROJECT_CATALOG_FILE: &str = "projects-v1.json";
const PROJECT_CATALOG_LOCK_FILE: &str = "projects-v1.lock";
pub const PROJECT_CATALOG_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_PROJECT_CATALOG_RESULTS: usize = 100;
pub const MAX_PROJECT_CATALOG_RESULTS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservedProject {
    pub project_id: String,
    pub root_path: PathBuf,
    pub last_opened_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservedProjectList {
    pub projects: Vec<ObservedProject>,
    pub total_projects: usize,
    pub omitted_projects: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectObservationEntry {
    root_path: String,
    last_opened_at_unix_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectCatalogDocument {
    schema_version: u32,
    projects: BTreeMap<String, ProjectObservationEntry>,
}

impl ProjectCatalogDocument {
    fn empty() -> Self {
        Self {
            schema_version: PROJECT_CATALOG_SCHEMA_VERSION,
            projects: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), LeyCoreError> {
        if self.schema_version != PROJECT_CATALOG_SCHEMA_VERSION {
            return Err(LeyCoreError::InvalidProjectCatalog(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        let mut roots = HashSet::new();
        for (project_id, observation) in &self.projects {
            validate_project_id(project_id).map_err(|error| {
                LeyCoreError::InvalidProjectCatalog(format!(
                    "invalid project ID key '{project_id}': {error}"
                ))
            })?;
            let root = Path::new(&observation.root_path);
            if observation.root_path.is_empty() || !root.is_absolute() {
                return Err(LeyCoreError::InvalidProjectCatalog(format!(
                    "project root for {project_id} must be an absolute UTF-8 path"
                )));
            }
            if root.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::CurDir | std::path::Component::ParentDir
                )
            }) {
                return Err(LeyCoreError::InvalidProjectCatalog(format!(
                    "project root for {project_id} must be normalized"
                )));
            }
            if !roots.insert(observation.root_path.as_str()) {
                return Err(LeyCoreError::InvalidProjectCatalog(
                    "one project root cannot claim multiple project IDs".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProjectCatalog {
    path: PathBuf,
}

impl ProjectCatalog {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        let binding_path = default_binding_registry_path()?;
        Ok(Self::at(binding_path.with_file_name(PROJECT_CATALOG_FILE)))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn observe(
        &self,
        project_start: impl AsRef<Path>,
    ) -> Result<ObservedProject, LeyCoreError> {
        let diagnostic = diagnose_project(project_start)?;
        self.observe_diagnostic_at(&diagnostic, unix_time_ms())
    }

    pub fn list(&self, max_results: usize) -> Result<ObservedProjectList, LeyCoreError> {
        if max_results == 0 || max_results > MAX_PROJECT_CATALOG_RESULTS {
            return Err(LeyCoreError::InvalidProjectCatalog(format!(
                "maxResults must be between 1 and {MAX_PROJECT_CATALOG_RESULTS}"
            )));
        }
        let document = self.read_locked()?;
        let total_projects = document.projects.len();
        let mut projects = document
            .projects
            .into_iter()
            .map(|(project_id, observation)| ObservedProject {
                project_id,
                root_path: PathBuf::from(observation.root_path),
                last_opened_at_unix_ms: observation.last_opened_at_unix_ms,
            })
            .collect::<Vec<_>>();
        projects.sort_by(|left, right| {
            right
                .last_opened_at_unix_ms
                .cmp(&left.last_opened_at_unix_ms)
                .then_with(|| left.project_id.cmp(&right.project_id))
        });
        projects.truncate(max_results);
        Ok(ObservedProjectList {
            omitted_projects: total_projects.saturating_sub(projects.len()),
            total_projects,
            projects,
        })
    }

    pub fn forget(&self, project_id: &str) -> Result<Option<ObservedProject>, LeyCoreError> {
        validate_project_id(project_id)?;
        let removed = self.mutate(|document| Ok(document.projects.remove(project_id)))?;
        Ok(removed.map(|observation| ObservedProject {
            project_id: project_id.to_owned(),
            root_path: PathBuf::from(observation.root_path),
            last_opened_at_unix_ms: observation.last_opened_at_unix_ms,
        }))
    }

    pub(crate) fn observe_diagnostic(
        &self,
        diagnostic: &ProjectDiagnostic,
    ) -> Result<ObservedProject, LeyCoreError> {
        self.observe_diagnostic_at(diagnostic, unix_time_ms())
    }

    fn observe_diagnostic_at(
        &self,
        diagnostic: &ProjectDiagnostic,
        opened_at_unix_ms: u64,
    ) -> Result<ObservedProject, LeyCoreError> {
        let project_id = diagnostic.identity.project_id.clone();
        let root_path = diagnostic.root.clone();
        let root_string = root_path
            .to_str()
            .ok_or_else(|| LeyCoreError::NonUtf8Path(root_path.clone()))?
            .to_owned();

        self.mutate(|document| {
            if let Some(existing) = document.projects.get(&project_id) {
                if existing.root_path != root_string
                    && path_still_claims_project(Path::new(&existing.root_path), &project_id)
                {
                    return Err(LeyCoreError::DuplicateProjectIdentity {
                        project_id: project_id.clone(),
                        observed_root: PathBuf::from(&existing.root_path),
                        requested_root: root_path.clone(),
                    });
                }
            }

            document.projects.retain(|existing_id, observation| {
                existing_id == &project_id || observation.root_path != root_string
            });
            document.projects.insert(
                project_id.clone(),
                ProjectObservationEntry {
                    root_path: root_string,
                    last_opened_at_unix_ms: opened_at_unix_ms,
                },
            );
            Ok(())
        })?;

        Ok(ObservedProject {
            project_id,
            root_path,
            last_opened_at_unix_ms: opened_at_unix_ms,
        })
    }

    fn read_locked(&self) -> Result<ProjectCatalogDocument, LeyCoreError> {
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
        operation: impl FnOnce(&mut ProjectCatalogDocument) -> Result<T, LeyCoreError>,
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
            LeyCoreError::InvalidProjectCatalog(
                "catalog path must have a parent directory".to_owned(),
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

    fn read_document(&self) -> Result<ProjectCatalogDocument, LeyCoreError> {
        match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ProjectCatalogDocument::empty())
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
        let document: ProjectCatalogDocument =
            serde_json::from_slice(&bytes).map_err(|source| LeyCoreError::Json {
                path: self.path.clone(),
                source,
            })?;
        document.validate()?;
        Ok(document)
    }

    fn write_document(&self, document: &ProjectCatalogDocument) -> Result<(), LeyCoreError> {
        reject_non_regular_if_present(&self.path)?;
        let mut body =
            serde_json::to_vec_pretty(document).expect("validated project catalog is serializable");
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
        self.path.with_file_name(PROJECT_CATALOG_LOCK_FILE)
    }
}

fn path_still_claims_project(path: &Path, project_id: &str) -> bool {
    diagnose_project(path).is_ok_and(|diagnostic| {
        diagnostic.root == path && diagnostic.identity.project_id == project_id
    })
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
                return Err(LeyCoreError::InvalidProjectCatalog(format!(
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

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after the Unix epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{initialize_project, CaptureMode};
    use std::sync::{Arc, Barrier};
    use tempfile::tempdir;

    #[test]
    fn catalog_is_explicit_recent_bounded_and_forgettable() {
        let base = tempdir().unwrap();
        let catalog = ProjectCatalog::at(base.path().join("config/projects.json"));
        let first = base.path().join("first");
        let second = base.path().join("second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        let first_identity =
            initialize_project(&first, Some("First"), CaptureMode::Structured).unwrap();
        let second_identity =
            initialize_project(&second, Some("Second"), CaptureMode::Minimal).unwrap();

        catalog
            .observe_diagnostic_at(&diagnose_project(&first).unwrap(), 100)
            .unwrap();
        catalog
            .observe_diagnostic_at(&diagnose_project(&second).unwrap(), 200)
            .unwrap();

        let bounded = catalog.list(1).unwrap();
        assert_eq!(bounded.total_projects, 2);
        assert_eq!(bounded.omitted_projects, 1);
        assert_eq!(
            bounded.projects[0].project_id,
            second_identity.identity.project_id
        );
        assert_eq!(
            catalog
                .forget(&first_identity.identity.project_id)
                .unwrap()
                .unwrap()
                .root_path,
            first.canonicalize().unwrap()
        );
        assert_eq!(
            catalog
                .list(DEFAULT_PROJECT_CATALOG_RESULTS)
                .unwrap()
                .total_projects,
            1
        );
        assert!(matches!(
            catalog.list(0),
            Err(LeyCoreError::InvalidProjectCatalog(_))
        ));
    }

    #[test]
    fn catalog_document_rejects_relative_unnormalized_and_duplicate_roots() {
        let mut document = ProjectCatalogDocument::empty();
        document.projects.insert(
            "prj_0123456789abcdef0123456789abcdef".to_owned(),
            ProjectObservationEntry {
                root_path: "relative/project".to_owned(),
                last_opened_at_unix_ms: 1,
            },
        );
        assert!(matches!(
            document.validate(),
            Err(LeyCoreError::InvalidProjectCatalog(_))
        ));
        document.projects.values_mut().next().unwrap().root_path = "/tmp/../project".to_owned();
        assert!(matches!(
            document.validate(),
            Err(LeyCoreError::InvalidProjectCatalog(_))
        ));
        document.projects.values_mut().next().unwrap().root_path = "/tmp/project".to_owned();
        document.projects.insert(
            "prj_fedcba9876543210fedcba9876543210".to_owned(),
            ProjectObservationEntry {
                root_path: "/tmp/project".to_owned(),
                last_opened_at_unix_ms: 2,
            },
        );
        assert!(matches!(
            document.validate(),
            Err(LeyCoreError::InvalidProjectCatalog(_))
        ));
    }

    #[test]
    fn moved_project_replaces_an_unavailable_observation() {
        let base = tempdir().unwrap();
        let before = base.path().join("before");
        let after = base.path().join("after");
        fs::create_dir(&before).unwrap();
        initialize_project(&before, None, CaptureMode::Structured).unwrap();
        let catalog = ProjectCatalog::at(base.path().join("config/projects.json"));
        catalog.observe(&before).unwrap();

        fs::rename(&before, &after).unwrap();
        let observed = catalog.observe(&after).unwrap();
        assert_eq!(observed.root_path, after.canonicalize().unwrap());
        assert_eq!(
            catalog
                .list(DEFAULT_PROJECT_CATALOG_RESULTS)
                .unwrap()
                .projects,
            vec![observed]
        );
    }

    #[test]
    fn live_duplicate_project_identities_are_rejected() {
        let base = tempdir().unwrap();
        let original = base.path().join("original");
        let duplicate = base.path().join("duplicate");
        fs::create_dir(&original).unwrap();
        fs::create_dir(&duplicate).unwrap();
        let initialized = initialize_project(&original, None, CaptureMode::Structured).unwrap();
        fs::create_dir(duplicate.join(crate::LEY_DIRECTORY)).unwrap();
        for file in [crate::PROJECT_FILE, crate::CAPTURE_FILE, crate::IGNORE_FILE] {
            fs::copy(
                original.join(crate::LEY_DIRECTORY).join(file),
                duplicate.join(crate::LEY_DIRECTORY).join(file),
            )
            .unwrap();
        }
        let catalog = ProjectCatalog::at(base.path().join("config/projects.json"));
        catalog.observe(&original).unwrap();

        let error = catalog.observe(&duplicate).unwrap_err();
        assert!(matches!(
            error,
            LeyCoreError::DuplicateProjectIdentity { .. }
        ));
        let listed = catalog.list(DEFAULT_PROJECT_CATALOG_RESULTS).unwrap();
        assert_eq!(listed.total_projects, 1);
        assert_eq!(
            listed.projects[0].project_id,
            initialized.identity.project_id
        );
        assert_eq!(
            listed.projects[0].root_path,
            original.canonicalize().unwrap()
        );
    }

    #[test]
    fn concurrent_observations_do_not_lose_projects() {
        let base = tempdir().unwrap();
        let catalog = ProjectCatalog::at(base.path().join("config/projects.json"));
        let barrier = Arc::new(Barrier::new(8));
        let mut workers = Vec::new();
        for index in 0..8 {
            let project = base.path().join(format!("project-{index}"));
            fs::create_dir(&project).unwrap();
            initialize_project(&project, None, CaptureMode::Structured).unwrap();
            let worker_catalog = catalog.clone();
            let worker_barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                worker_barrier.wait();
                worker_catalog.observe(project).unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(
            catalog
                .list(DEFAULT_PROJECT_CATALOG_RESULTS)
                .unwrap()
                .total_projects,
            8
        );
    }

    #[cfg(unix)]
    #[test]
    fn catalog_files_are_private_and_symlinks_are_rejected() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let base = tempdir().unwrap();
        let project = base.path().join("project");
        fs::create_dir(&project).unwrap();
        initialize_project(&project, None, CaptureMode::Structured).unwrap();
        let catalog_path = base.path().join("private/config/projects.json");
        let catalog = ProjectCatalog::at(&catalog_path);
        catalog.observe(&project).unwrap();
        assert_eq!(
            fs::metadata(&catalog_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(catalog.lock_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );

        fs::remove_file(&catalog_path).unwrap();
        let outside = base.path().join("outside.json");
        fs::write(&outside, b"do not replace").unwrap();
        symlink(&outside, &catalog_path).unwrap();
        assert!(matches!(
            catalog.observe(&project),
            Err(LeyCoreError::UnsafeProjectLayout(_))
        ));
        assert_eq!(fs::read_to_string(outside).unwrap(), "do not replace");
    }
}
