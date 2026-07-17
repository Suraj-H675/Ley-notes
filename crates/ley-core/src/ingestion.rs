use crate::graph::{
    build_project_graph, graph_body, validate_project_graph, GraphSource, ProjectGraph,
    PROJECT_GRAPH_LIMIT_BYTES,
};
use crate::{diagnose_project, preview_capture, validate_project_id, CaptureMode, LeyCoreError};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

pub const AGENT_MEMORY_DIRECTORY: &str = "agent-memory";
pub const ARTIFACT_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const ARTIFACT_MANIFEST_LIMIT_BYTES: u64 = 67_108_864;
const STORE_ROOT: &str = ".ley";
const PROJECTS_DIRECTORY: &str = "projects";
const ARTIFACTS_DIRECTORY: &str = "artifacts";
const CONTENT_DIRECTORY: &str = "content";
const SNAPSHOTS_DIRECTORY: &str = "snapshots";
const MANIFEST_FILE: &str = "manifest-v1.json";
const INGEST_LOCK_FILE: &str = "ingest-v1.lock";
const GRAPH_DIRECTORY: &str = "graph";
const GRAPH_MANIFEST_FILE: &str = "graph-v1.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    Source,
    Documentation,
    Manifest,
    Configuration,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RedactionFinding {
    pub kind: String,
    pub lines: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactRecord {
    pub path: String,
    pub kind: ArtifactKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub source_bytes: u64,
    pub stored_bytes: u64,
    pub line_count: u64,
    pub content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_blob: Option<String>,
    pub redactions: Vec<RedactionFinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactSkipReason {
    Binary,
    NonUtf8,
    Oversized,
    TotalLimit,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkippedArtifact {
    pub path: String,
    pub reason: ArtifactSkipReason,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenamedArtifact {
    pub from: String,
    pub to: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionResult {
    pub project_id: String,
    pub snapshot_id: String,
    pub changed: bool,
    pub graph_snapshot_id: String,
    pub graph_changed: bool,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub files: usize,
    pub stored_files: usize,
    pub redacted_files: usize,
    pub skipped: Vec<SkippedArtifact>,
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub renamed: Vec<RenamedArtifact>,
    pub deleted: Vec<String>,
    pub manifest_path: String,
    pub graph_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArtifactManifest {
    schema_version: u32,
    project_id: String,
    project_name: String,
    snapshot_id: String,
    generated_at_unix_ms: u64,
    capture_mode: CaptureMode,
    capture_policy: crate::CapturePolicy,
    capture_fingerprint: String,
    files: Vec<ArtifactRecord>,
    skipped: Vec<SkippedArtifact>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotIdentity<'a> {
    schema_version: u32,
    project_id: &'a str,
    project_name: &'a str,
    capture_policy: &'a crate::CapturePolicy,
    capture_fingerprint: &'a str,
    files: &'a [ArtifactRecord],
    skipped: &'a [SkippedArtifact],
}

struct ArtifactStore {
    artifacts_dir: Dir,
    content_dir: Dir,
    snapshots_dir: Dir,
    graph_dir: Dir,
    graph_snapshots_dir: Dir,
}

struct IngestionLock {
    file: File,
}

impl Drop for IngestionLock {
    fn drop(&mut self) {
        let _ = File::unlock(&self.file);
    }
}

pub fn ingest_project(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
) -> Result<IngestionResult, LeyCoreError> {
    let diagnostic = diagnose_project(project_start)?;
    let vault_path = vault
        .as_ref()
        .canonicalize()
        .map_err(|source| LeyCoreError::Io {
            path: vault.as_ref().to_path_buf(),
            source,
        })?;
    if !vault_path.is_dir() {
        return Err(LeyCoreError::NotDirectory(vault.as_ref().to_path_buf()));
    }
    if vault_path.starts_with(&diagnostic.root) {
        return Err(LeyCoreError::OverlappingProjectVault(vault_path));
    }

    let store = ArtifactStore::open(&vault_path, &diagnostic.identity.project_id)?;
    let _lock = store.lock()?;
    let previous = store.read_manifest()?;
    if let Some(manifest) = &previous {
        validate_manifest(manifest, &diagnostic.identity.project_id)?;
        store.verify_snapshot(manifest)?;
        store.verify_content_blobs(manifest)?;
    }
    let previous_graph = store.read_graph()?;
    if let Some(graph) = &previous_graph {
        validate_project_graph(graph, &diagnostic.identity.project_id)?;
        store.verify_graph_snapshot(graph)?;
    }

    let preview = preview_capture(&diagnostic.root)?;
    let root_dir =
        Dir::open_ambient_dir(&diagnostic.root, ambient_authority()).map_err(|source| {
            LeyCoreError::Io {
                path: diagnostic.root.clone(),
                source,
            }
        })?;
    let mut files = Vec::new();
    let mut graph_sources = Vec::new();
    let mut skipped = preview
        .skipped_oversized
        .iter()
        .map(|file| SkippedArtifact {
            path: file.path.clone(),
            reason: ArtifactSkipReason::Oversized,
            bytes: file.bytes,
        })
        .chain(
            preview
                .skipped_total_limit
                .iter()
                .map(|file| SkippedArtifact {
                    path: file.path.clone(),
                    reason: ArtifactSkipReason::TotalLimit,
                    bytes: file.bytes,
                }),
        )
        .chain(preview.skipped_symlinks.iter().map(|path| SkippedArtifact {
            path: path.clone(),
            reason: ArtifactSkipReason::Symlink,
            bytes: 0,
        }))
        .collect::<Vec<_>>();
    for candidate in &preview.files {
        let bytes = read_scoped_file(
            &root_dir,
            &candidate.path,
            candidate.bytes,
            diagnostic.capture.max_file_bytes,
        )?;
        if bytes.contains(&0) {
            skipped.push(SkippedArtifact {
                path: candidate.path.clone(),
                reason: ArtifactSkipReason::Binary,
                bytes: candidate.bytes,
            });
            continue;
        }
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                skipped.push(SkippedArtifact {
                    path: candidate.path.clone(),
                    reason: ArtifactSkipReason::NonUtf8,
                    bytes: candidate.bytes,
                });
                continue;
            }
        };
        let (redacted, redactions) = redact_secrets(&text);
        let stored = redacted.as_bytes();
        let digest = sha256_hex(stored);
        let content_hash = format!("sha256:{digest}");
        let content_blob = if diagnostic.capture.mode == CaptureMode::Minimal {
            None
        } else {
            let name = format!("{digest}.txt");
            store.write_blob_if_absent(&name, stored)?;
            Some(format!("{CONTENT_DIRECTORY}/{name}"))
        };
        let (kind, language) = classify_artifact(&candidate.path);
        let artifact = ArtifactRecord {
            path: candidate.path.clone(),
            kind,
            language: language.map(str::to_owned),
            source_bytes: candidate.bytes,
            stored_bytes: stored.len() as u64,
            line_count: line_count(&redacted),
            content_hash,
            content_blob,
            redactions,
        };
        graph_sources.push(GraphSource {
            artifact: artifact.clone(),
            text: redacted,
        });
        files.push(artifact);
    }
    skipped.sort_by(|left, right| left.path.cmp(&right.path));

    let identity = SnapshotIdentity {
        schema_version: ARTIFACT_MANIFEST_SCHEMA_VERSION,
        project_id: &diagnostic.identity.project_id,
        project_name: &diagnostic.identity.name,
        capture_policy: &diagnostic.capture,
        capture_fingerprint: &preview.capture_fingerprint,
        files: &files,
        skipped: &skipped,
    };
    let snapshot_hash = sha256_hex(
        &serde_json::to_vec(&identity).expect("artifact snapshot identity is serializable"),
    );
    let snapshot_id = format!("snp_{snapshot_hash}");
    let graph = build_project_graph(
        &diagnostic.root,
        &diagnostic.identity.project_id,
        &diagnostic.identity.name,
        &snapshot_id,
        &graph_sources,
        unix_time_ms(),
    )?;
    if previous
        .as_ref()
        .is_some_and(|manifest| manifest.snapshot_id == snapshot_id)
    {
        let graph_changed = store.persist_graph(previous_graph.as_ref(), &graph)?;
        let old = previous.expect("checked as present");
        return Ok(IngestionResult {
            project_id: old.project_id,
            snapshot_id: old.snapshot_id,
            changed: false,
            graph_snapshot_id: graph.graph_snapshot_id,
            graph_changed,
            graph_nodes: graph.nodes.len(),
            graph_edges: graph.edges.len(),
            files: old.files.len(),
            stored_files: old
                .files
                .iter()
                .filter(|file| file.content_blob.is_some())
                .count(),
            redacted_files: old
                .files
                .iter()
                .filter(|file| !file.redactions.is_empty())
                .count(),
            skipped: old.skipped,
            added: Vec::new(),
            modified: Vec::new(),
            renamed: Vec::new(),
            deleted: Vec::new(),
            manifest_path: manifest_relative_path(&diagnostic.identity.project_id),
            graph_path: graph_relative_path(&diagnostic.identity.project_id),
        });
    }

    let manifest = ArtifactManifest {
        schema_version: ARTIFACT_MANIFEST_SCHEMA_VERSION,
        project_id: diagnostic.identity.project_id.clone(),
        project_name: diagnostic.identity.name,
        snapshot_id: snapshot_id.clone(),
        generated_at_unix_ms: unix_time_ms(),
        capture_mode: diagnostic.capture.mode,
        capture_policy: diagnostic.capture,
        capture_fingerprint: preview.capture_fingerprint,
        files,
        skipped,
    };
    validate_manifest(&manifest, &diagnostic.identity.project_id)?;
    let body = manifest_body(&manifest)?;
    let changes = calculate_changes(previous.as_ref(), &manifest);
    store.write_snapshot_if_absent(&snapshot_id, &body)?;
    let graph_changed = store.persist_graph(previous_graph.as_ref(), &graph)?;
    store.write_manifest(&body)?;

    Ok(IngestionResult {
        project_id: manifest.project_id,
        snapshot_id,
        changed: true,
        graph_snapshot_id: graph.graph_snapshot_id,
        graph_changed,
        graph_nodes: graph.nodes.len(),
        graph_edges: graph.edges.len(),
        files: manifest.files.len(),
        stored_files: manifest
            .files
            .iter()
            .filter(|file| file.content_blob.is_some())
            .count(),
        redacted_files: manifest
            .files
            .iter()
            .filter(|file| !file.redactions.is_empty())
            .count(),
        skipped: manifest.skipped,
        added: changes.added,
        modified: changes.modified,
        renamed: changes.renamed,
        deleted: changes.deleted,
        manifest_path: manifest_relative_path(&diagnostic.identity.project_id),
        graph_path: graph_relative_path(&diagnostic.identity.project_id),
    })
}

pub fn read_project_graph(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
) -> Result<ProjectGraph, LeyCoreError> {
    let diagnostic = diagnose_project(project_start)?;
    let vault_path = vault
        .as_ref()
        .canonicalize()
        .map_err(|source| LeyCoreError::Io {
            path: vault.as_ref().to_path_buf(),
            source,
        })?;
    if !vault_path.is_dir() {
        return Err(LeyCoreError::NotDirectory(vault.as_ref().to_path_buf()));
    }
    let store = ArtifactStore::open(&vault_path, &diagnostic.identity.project_id)?;
    let graph = store.read_graph()?.ok_or_else(|| {
        LeyCoreError::InvalidProjectGraph(
            "no project graph exists; run 'ley ingest' first".to_owned(),
        )
    })?;
    validate_project_graph(&graph, &diagnostic.identity.project_id)?;
    store.verify_graph_snapshot(&graph)?;
    Ok(graph)
}

impl ArtifactStore {
    fn open(vault: &Path, project_id: &str) -> Result<Self, LeyCoreError> {
        validate_project_id(project_id)?;
        let vault_dir = Dir::open_ambient_dir(vault, ambient_authority()).map_err(|source| {
            LeyCoreError::Io {
                path: vault.to_path_buf(),
                source,
            }
        })?;
        let ley_dir = open_or_create_private_dir(&vault_dir, STORE_ROOT, vault)?;
        let memory_dir = open_or_create_private_dir(&ley_dir, AGENT_MEMORY_DIRECTORY, vault)?;
        let projects_dir = open_or_create_private_dir(&memory_dir, PROJECTS_DIRECTORY, vault)?;
        let project_dir = open_or_create_private_dir(&projects_dir, project_id, vault)?;
        let artifacts_dir = open_or_create_private_dir(&project_dir, ARTIFACTS_DIRECTORY, vault)?;
        let content_dir = open_or_create_private_dir(&artifacts_dir, CONTENT_DIRECTORY, vault)?;
        let snapshots_dir = open_or_create_private_dir(&artifacts_dir, SNAPSHOTS_DIRECTORY, vault)?;
        let graph_dir = open_or_create_private_dir(&project_dir, GRAPH_DIRECTORY, vault)?;
        let graph_snapshots_dir =
            open_or_create_private_dir(&graph_dir, SNAPSHOTS_DIRECTORY, vault)?;
        Ok(Self {
            artifacts_dir,
            content_dir,
            snapshots_dir,
            graph_dir,
            graph_snapshots_dir,
        })
    }

    fn lock(&self) -> Result<IngestionLock, LeyCoreError> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create(true)
            .follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = self
            .artifacts_dir
            .open_with(INGEST_LOCK_FILE, &options)
            .map_err(|source| store_io(INGEST_LOCK_FILE, source))?;
        ensure_private_file_permissions(&lock, INGEST_LOCK_FILE)?;
        let file = lock.into_std();
        file.lock()
            .map_err(|source| store_io(INGEST_LOCK_FILE, source))?;
        Ok(IngestionLock { file })
    }

    fn read_manifest(&self) -> Result<Option<ArtifactManifest>, LeyCoreError> {
        let Some(bytes) = read_optional_store_file(
            &self.artifacts_dir,
            MANIFEST_FILE,
            ARTIFACT_MANIFEST_LIMIT_BYTES,
        )?
        else {
            return Ok(None);
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|source| LeyCoreError::InvalidArtifactStore(source.to_string()))
    }

    fn write_blob_if_absent(&self, name: &str, body: &[u8]) -> Result<(), LeyCoreError> {
        write_immutable_private(&self.content_dir, name, body)
    }

    fn write_snapshot_if_absent(&self, snapshot_id: &str, body: &[u8]) -> Result<(), LeyCoreError> {
        write_immutable_private(&self.snapshots_dir, &format!("{snapshot_id}.json"), body)
    }

    fn write_manifest(&self, body: &[u8]) -> Result<(), LeyCoreError> {
        write_atomic_private(&self.artifacts_dir, MANIFEST_FILE, body)
    }

    fn verify_snapshot(&self, manifest: &ArtifactManifest) -> Result<(), LeyCoreError> {
        let name = format!("{}.json", manifest.snapshot_id);
        let body = manifest_body(manifest)?;
        let stored =
            read_optional_store_file(&self.snapshots_dir, &name, ARTIFACT_MANIFEST_LIMIT_BYTES)?
                .ok_or_else(|| {
                    LeyCoreError::InvalidArtifactStore(format!(
                        "current manifest snapshot is missing: {name}"
                    ))
                })?;
        if stored != body {
            return Err(LeyCoreError::InvalidArtifactStore(format!(
                "current manifest does not match immutable snapshot {name}"
            )));
        }
        Ok(())
    }

    fn verify_content_blobs(&self, manifest: &ArtifactManifest) -> Result<(), LeyCoreError> {
        for artifact in &manifest.files {
            let Some(blob) = &artifact.content_blob else {
                continue;
            };
            let name = blob
                .strip_prefix(&format!("{CONTENT_DIRECTORY}/"))
                .expect("validated content blob prefix");
            let stored = read_optional_store_file(&self.content_dir, name, artifact.stored_bytes)?
                .ok_or_else(|| {
                    LeyCoreError::InvalidArtifactStore(format!(
                        "content blob is missing for {}",
                        artifact.path
                    ))
                })?;
            if stored.len() as u64 != artifact.stored_bytes
                || format!("sha256:{}", sha256_hex(&stored)) != artifact.content_hash
            {
                return Err(LeyCoreError::InvalidArtifactStore(format!(
                    "content blob failed integrity verification for {}",
                    artifact.path
                )));
            }
        }
        Ok(())
    }

    fn read_graph(&self) -> Result<Option<ProjectGraph>, LeyCoreError> {
        let Some(bytes) = read_optional_store_file(
            &self.graph_dir,
            GRAPH_MANIFEST_FILE,
            PROJECT_GRAPH_LIMIT_BYTES,
        )?
        else {
            return Ok(None);
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|source| LeyCoreError::InvalidProjectGraph(source.to_string()))
    }

    fn verify_graph_snapshot(&self, graph: &ProjectGraph) -> Result<(), LeyCoreError> {
        let name = format!("{}.json", graph.graph_snapshot_id);
        let body = graph_body(graph)?;
        let stored =
            read_optional_store_file(&self.graph_snapshots_dir, &name, PROJECT_GRAPH_LIMIT_BYTES)?
                .ok_or_else(|| {
                    LeyCoreError::InvalidProjectGraph(format!(
                        "current graph snapshot is missing: {name}"
                    ))
                })?;
        if stored != body {
            return Err(LeyCoreError::InvalidProjectGraph(format!(
                "current graph does not match immutable snapshot {name}"
            )));
        }
        Ok(())
    }

    fn persist_graph(
        &self,
        previous: Option<&ProjectGraph>,
        graph: &ProjectGraph,
    ) -> Result<bool, LeyCoreError> {
        if previous.is_some_and(|old| old.graph_snapshot_id == graph.graph_snapshot_id) {
            return Ok(false);
        }
        let body = graph_body(graph)?;
        write_immutable_private(
            &self.graph_snapshots_dir,
            &format!("{}.json", graph.graph_snapshot_id),
            &body,
        )?;
        write_atomic_private(&self.graph_dir, GRAPH_MANIFEST_FILE, &body)?;
        Ok(true)
    }
}

fn open_or_create_private_dir(parent: &Dir, name: &str, vault: &Path) -> Result<Dir, LeyCoreError> {
    use cap_fs_ext::DirExt;

    match parent.open_dir_nofollow(name) {
        Ok(directory) => return Ok(directory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LeyCoreError::Io {
                path: vault.join(name),
                source,
            })
        }
    }
    let mut builder = cap_std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use cap_std::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match parent.create_dir_with(name, &builder) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(source) => {
            return Err(LeyCoreError::Io {
                path: vault.join(name),
                source,
            })
        }
    }
    parent
        .open_dir_nofollow(name)
        .map_err(|source| LeyCoreError::Io {
            path: vault.join(name),
            source,
        })
}

fn read_scoped_file(
    root: &Dir,
    relative: &str,
    preview_bytes: u64,
    limit: u64,
) -> Result<Vec<u8>, LeyCoreError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = root
        .open_with(relative, &options)
        .map_err(|source| LeyCoreError::Io {
            path: PathBuf::from(relative),
            source,
        })?;
    let before = file.metadata().map_err(|source| LeyCoreError::Io {
        path: PathBuf::from(relative),
        source,
    })?;
    if !before.is_file() || before.len() != preview_bytes || before.len() > limit {
        return Err(LeyCoreError::ProjectChangedDuringIngestion(
            relative.to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    Read::by_ref(&mut file)
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| LeyCoreError::Io {
            path: PathBuf::from(relative),
            source,
        })?;
    let after = file.metadata().map_err(|source| LeyCoreError::Io {
        path: PathBuf::from(relative),
        source,
    })?;
    if bytes.len() as u64 != preview_bytes
        || after.len() != before.len()
        || after.modified().ok() != before.modified().ok()
    {
        return Err(LeyCoreError::ProjectChangedDuringIngestion(
            relative.to_owned(),
        ));
    }
    Ok(bytes)
}

fn read_optional_store_file(
    directory: &Dir,
    name: &str,
    limit: u64,
) -> Result<Option<Vec<u8>>, LeyCoreError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = match directory.open_with(name, &options) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(store_io(name, source)),
    };
    ensure_private_file_permissions(&file, name)?;
    let metadata = file.metadata().map_err(|source| store_io(name, source))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(LeyCoreError::InvalidArtifactStore(format!(
            "{name} must be a regular file no larger than {limit} bytes"
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    Read::by_ref(&mut file)
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| store_io(name, source))?;
    if bytes.len() as u64 > limit {
        return Err(LeyCoreError::InvalidArtifactStore(format!(
            "{name} exceeds {limit} bytes"
        )));
    }
    Ok(Some(bytes))
}

fn write_immutable_private(directory: &Dir, name: &str, body: &[u8]) -> Result<(), LeyCoreError> {
    if let Some(existing) = read_optional_store_file(directory, name, body.len() as u64)? {
        if existing == body {
            return Ok(());
        }
        return Err(LeyCoreError::InvalidArtifactStore(format!(
            "immutable artifact collision at {name}"
        )));
    }
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = match directory.open_with(name, &options) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = read_optional_store_file(directory, name, body.len() as u64)?
                .ok_or_else(|| {
                    LeyCoreError::InvalidArtifactStore(format!(
                        "artifact appeared and disappeared at {name}"
                    ))
                })?;
            if existing == body {
                return Ok(());
            }
            return Err(LeyCoreError::InvalidArtifactStore(format!(
                "immutable artifact collision at {name}"
            )));
        }
        Err(source) => return Err(store_io(name, source)),
    };
    file.write_all(body)
        .map_err(|source| store_io(name, source))?;
    file.sync_all().map_err(|source| store_io(name, source))
}

fn write_atomic_private(directory: &Dir, name: &str, body: &[u8]) -> Result<(), LeyCoreError> {
    let mut temporary =
        cap_tempfile::TempFile::new(directory).map_err(|source| store_io(name, source))?;
    let mut permissions = temporary
        .as_file()
        .metadata()
        .map_err(|source| store_io(name, source))?
        .permissions();
    permissions.set_readonly(false);
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        permissions.set_mode(0o600);
    }
    temporary
        .as_file()
        .set_permissions(permissions)
        .map_err(|source| store_io(name, source))?;
    temporary
        .write_all(body)
        .map_err(|source| store_io(name, source))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| store_io(name, source))?;
    temporary
        .replace(name)
        .map_err(|source| store_io(name, source))
}

fn ensure_private_file_permissions(
    file: &cap_std::fs::File,
    name: &str,
) -> Result<(), LeyCoreError> {
    let metadata = file.metadata().map_err(|source| store_io(name, source))?;
    if !metadata.is_file() {
        return Err(LeyCoreError::InvalidArtifactStore(format!(
            "{name} is not a regular file"
        )));
    }
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LeyCoreError::InvalidArtifactStore(format!(
                "{name} must use private mode 600"
            )));
        }
    }
    Ok(())
}

fn validate_manifest(
    manifest: &ArtifactManifest,
    expected_project_id: &str,
) -> Result<(), LeyCoreError> {
    if manifest.schema_version != ARTIFACT_MANIFEST_SCHEMA_VERSION {
        return Err(LeyCoreError::InvalidArtifactStore(format!(
            "unsupported manifest schema version {}",
            manifest.schema_version
        )));
    }
    validate_project_id(&manifest.project_id)?;
    if manifest.project_id != expected_project_id {
        return Err(LeyCoreError::InvalidArtifactStore(
            "manifest project ID does not match the initialized project".to_owned(),
        ));
    }
    if manifest.project_name.trim().is_empty()
        || manifest.project_name.chars().count() > 128
        || manifest.project_name.chars().any(char::is_control)
    {
        return Err(LeyCoreError::InvalidArtifactStore(
            "manifest project name is invalid".to_owned(),
        ));
    }
    if !manifest.snapshot_id.starts_with("snp_")
        || manifest.snapshot_id.len() != 68
        || !manifest.snapshot_id[4..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(LeyCoreError::InvalidArtifactStore(
            "manifest snapshot ID is invalid".to_owned(),
        ));
    }
    let identity = SnapshotIdentity {
        schema_version: manifest.schema_version,
        project_id: &manifest.project_id,
        project_name: &manifest.project_name,
        capture_policy: &manifest.capture_policy,
        capture_fingerprint: &manifest.capture_fingerprint,
        files: &manifest.files,
        skipped: &manifest.skipped,
    };
    let expected_snapshot = format!(
        "snp_{}",
        sha256_hex(
            &serde_json::to_vec(&identity).expect("artifact snapshot identity is serializable")
        )
    );
    if manifest.snapshot_id != expected_snapshot {
        return Err(LeyCoreError::InvalidArtifactStore(
            "manifest snapshot ID does not match its artifact contents".to_owned(),
        ));
    }
    if manifest.capture_mode != manifest.capture_policy.mode {
        return Err(LeyCoreError::InvalidArtifactStore(
            "manifest capture mode does not match its policy".to_owned(),
        ));
    }
    crate::validate_capture(&manifest.capture_policy).map_err(|error| {
        LeyCoreError::InvalidArtifactStore(format!("manifest capture policy is invalid: {error}"))
    })?;
    if !is_sha256(&manifest.capture_fingerprint) {
        return Err(LeyCoreError::InvalidArtifactStore(
            "manifest capture fingerprint is invalid".to_owned(),
        ));
    }

    let mut paths = BTreeSet::new();
    let mut last_path: Option<&str> = None;
    for file in &manifest.files {
        validate_relative_artifact_path(&file.path)?;
        if last_path.is_some_and(|previous| previous >= file.path.as_str()) {
            return Err(LeyCoreError::InvalidArtifactStore(
                "artifact files must be sorted by unique path".to_owned(),
            ));
        }
        last_path = Some(&file.path);
        if !paths.insert(file.path.as_str()) {
            return Err(LeyCoreError::InvalidArtifactStore(format!(
                "duplicate artifact path {}",
                file.path
            )));
        }
        let Some(hash) = file.content_hash.strip_prefix("sha256:") else {
            return Err(LeyCoreError::InvalidArtifactStore(format!(
                "invalid content hash for {}",
                file.path
            )));
        };
        if !is_sha256(&file.content_hash) {
            return Err(LeyCoreError::InvalidArtifactStore(format!(
                "invalid content hash for {}",
                file.path
            )));
        }
        if let Some(blob) = &file.content_blob {
            if blob != &format!("{CONTENT_DIRECTORY}/{hash}.txt") {
                return Err(LeyCoreError::InvalidArtifactStore(format!(
                    "content blob does not match hash for {}",
                    file.path
                )));
            }
        }
        if (manifest.capture_mode == CaptureMode::Minimal) != file.content_blob.is_none() {
            return Err(LeyCoreError::InvalidArtifactStore(format!(
                "content storage does not match capture mode for {}",
                file.path
            )));
        }
        for finding in &file.redactions {
            if finding.kind.is_empty()
                || finding.lines.is_empty()
                || finding.lines.contains(&0)
                || finding.lines.windows(2).any(|lines| lines[0] >= lines[1])
            {
                return Err(LeyCoreError::InvalidArtifactStore(format!(
                    "redaction evidence is invalid for {}",
                    file.path
                )));
            }
        }
    }
    last_path = None;
    for skipped in &manifest.skipped {
        validate_relative_artifact_path(&skipped.path)?;
        if last_path.is_some_and(|previous| previous >= skipped.path.as_str()) {
            return Err(LeyCoreError::InvalidArtifactStore(
                "skipped artifacts must be sorted by unique path".to_owned(),
            ));
        }
        last_path = Some(&skipped.path);
        if !paths.insert(skipped.path.as_str()) {
            return Err(LeyCoreError::InvalidArtifactStore(format!(
                "duplicate artifact path {}",
                skipped.path
            )));
        }
    }
    Ok(())
}

fn validate_relative_artifact_path(value: &str) -> Result<(), LeyCoreError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            !matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return Err(LeyCoreError::InvalidArtifactStore(format!(
            "unsafe artifact path {value}"
        )));
    }
    Ok(())
}

fn manifest_body(manifest: &ArtifactManifest) -> Result<Vec<u8>, LeyCoreError> {
    let mut body = serde_json::to_vec_pretty(manifest)
        .map_err(|error| LeyCoreError::InvalidArtifactStore(error.to_string()))?;
    body.push(b'\n');
    if body.len() as u64 > ARTIFACT_MANIFEST_LIMIT_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: PathBuf::from(MANIFEST_FILE),
            limit_bytes: ARTIFACT_MANIFEST_LIMIT_BYTES,
        });
    }
    Ok(body)
}

struct ArtifactChanges {
    added: Vec<String>,
    modified: Vec<String>,
    renamed: Vec<RenamedArtifact>,
    deleted: Vec<String>,
}

fn calculate_changes(
    previous: Option<&ArtifactManifest>,
    current: &ArtifactManifest,
) -> ArtifactChanges {
    let old = previous
        .map(|manifest| {
            manifest
                .files
                .iter()
                .map(|file| (file.path.as_str(), file))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let new = current
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let mut added = new
        .keys()
        .filter(|path| !old.contains_key(**path))
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    let mut deleted = old
        .keys()
        .filter(|path| !new.contains_key(**path))
        .map(|path| (*path).to_owned())
        .collect::<Vec<_>>();
    let modified = new
        .iter()
        .filter_map(|(path, file)| {
            old.get(path)
                .filter(|old_file| old_file.content_hash != file.content_hash)
                .map(|_| (*path).to_owned())
        })
        .collect::<Vec<_>>();

    let mut added_by_hash = BTreeMap::<&str, Vec<&str>>::new();
    for path in &added {
        added_by_hash
            .entry(new[path.as_str()].content_hash.as_str())
            .or_default()
            .push(path);
    }
    let mut deleted_by_hash = BTreeMap::<&str, Vec<&str>>::new();
    for path in &deleted {
        deleted_by_hash
            .entry(old[path.as_str()].content_hash.as_str())
            .or_default()
            .push(path);
    }
    let mut renamed = Vec::new();
    for (hash, new_paths) in added_by_hash {
        let Some(old_paths) = deleted_by_hash.get(hash) else {
            continue;
        };
        if new_paths.len() == 1 && old_paths.len() == 1 {
            renamed.push(RenamedArtifact {
                from: old_paths[0].to_owned(),
                to: new_paths[0].to_owned(),
                content_hash: hash.to_owned(),
            });
        }
    }
    let renamed_from = renamed
        .iter()
        .map(|rename| rename.from.as_str())
        .collect::<BTreeSet<_>>();
    let renamed_to = renamed
        .iter()
        .map(|rename| rename.to.as_str())
        .collect::<BTreeSet<_>>();
    added.retain(|path| !renamed_to.contains(path.as_str()));
    deleted.retain(|path| !renamed_from.contains(path.as_str()));

    ArtifactChanges {
        added,
        modified,
        renamed,
        deleted,
    }
}

struct RedactionRule {
    kind: &'static str,
    pattern: Regex,
    replacement: &'static str,
}

fn redaction_rules() -> &'static Vec<RedactionRule> {
    static RULES: OnceLock<Vec<RedactionRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            RedactionRule {
                kind: "private-key",
                pattern: Regex::new(
                    r"(?ms)-----BEGIN (?:[A-Z0-9 ]+ )?PRIVATE KEY-----.*?(?:-----END (?:[A-Z0-9 ]+ )?PRIVATE KEY-----|\z)",
                )
                .expect("private-key pattern is valid"),
                replacement: "[REDACTED:private-key]",
            },
            RedactionRule {
                kind: "credential-assignment",
                pattern: Regex::new(
                    r#"(?im)^(\s*(?:(?:export\s+)?(?:const|let|var|static|final)\s+(?:mut\s+)?|export\s+)?["']?[A-Z0-9_.-]*(?:password|passwd|secret|api[_-]?key|access[_-]?token|auth[_-]?token|private[_-]?key)[A-Z0-9_.-]*["']?\s*(?::|=)\s*)(?:"(?:\\.|[^"\\])*"|'[^'\r\n]*'|[^\s#,\r\n]+)"#,
                )
                .expect("credential assignment pattern is valid"),
                replacement: "${1}\"[REDACTED:credential-assignment]\"",
            },
            RedactionRule {
                kind: "credential-url",
                pattern: Regex::new(r"(?i)([a-z][a-z0-9+.-]*://[^/\s:@]+:)[^@\s/]+@")
                    .expect("credential URL pattern is valid"),
                replacement: "${1}[REDACTED:credential-url]@",
            },
            RedactionRule {
                kind: "provider-token",
                pattern: Regex::new(
                    r"(?i)(?:gh[pousr]_[a-z0-9]{20,}|github_pat_[a-z0-9_]{20,}|sk-[a-z0-9_-]{20,}|AKIA[0-9A-Z]{16})",
                )
                .expect("provider token pattern is valid"),
                replacement: "[REDACTED:provider-token]",
            },
        ]
    })
}

fn redact_secrets(text: &str) -> (String, Vec<RedactionFinding>) {
    let mut redacted = text.to_owned();
    let mut findings = Vec::new();
    for rule in redaction_rules() {
        let mut lines = rule
            .pattern
            .find_iter(&redacted)
            .map(|matched| {
                redacted.as_bytes()[..matched.start()]
                    .iter()
                    .filter(|byte| **byte == b'\n')
                    .count() as u64
                    + 1
            })
            .collect::<Vec<_>>();
        lines.sort_unstable();
        lines.dedup();
        if !lines.is_empty() {
            redacted = rule
                .pattern
                .replace_all(&redacted, |captures: &regex::Captures<'_>| {
                    let original = captures
                        .get(0)
                        .expect("a regex replacement always has a whole match")
                        .as_str();
                    let mut replacement = String::new();
                    captures.expand(rule.replacement, &mut replacement);
                    let original_newlines = original.bytes().filter(|byte| *byte == b'\n').count();
                    let replacement_newlines =
                        replacement.bytes().filter(|byte| *byte == b'\n').count();
                    if original_newlines > replacement_newlines {
                        replacement
                            .push_str(&"\n".repeat(original_newlines - replacement_newlines));
                    }
                    replacement
                })
                .into_owned();
            findings.push(RedactionFinding {
                kind: rule.kind.to_owned(),
                lines,
            });
        }
    }
    (redacted, findings)
}

fn classify_artifact(path: &str) -> (ArtifactKind, Option<&'static str>) {
    let path = Path::new(path);
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let lower_name = name.to_ascii_lowercase();
    if matches!(
        lower_name.as_str(),
        "cargo.toml"
            | "package.json"
            | "pyproject.toml"
            | "go.mod"
            | "go.sum"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "composer.json"
            | "gemfile"
            | "requirements.txt"
            | "deno.json"
            | "deno.jsonc"
    ) {
        return (ArtifactKind::Manifest, language_for_path(path));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "md" | "mdx" | "rst" | "adoc" | "txt")
        || lower_name.starts_with("readme")
        || lower_name.starts_with("changelog")
        || lower_name.starts_with("license")
    {
        return (ArtifactKind::Documentation, language_for_path(path));
    }
    if matches!(
        extension.as_str(),
        "json" | "jsonc" | "yaml" | "yml" | "toml" | "xml" | "ini" | "cfg" | "conf" | "properties"
    ) || lower_name.starts_with('.')
    {
        return (ArtifactKind::Configuration, language_for_path(path));
    }
    if let Some(language) = language_for_path(path) {
        return (ArtifactKind::Source, Some(language));
    }
    (ArtifactKind::Text, None)
}

fn language_for_path(path: &Path) -> Option<&'static str> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();
    match extension.as_str() {
        "rs" => Some("rust"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "py" | "pyi" => Some("python"),
        "go" => Some("go"),
        "java" => Some("java"),
        "kt" | "kts" => Some("kotlin"),
        "c" | "h" => Some("c"),
        "cc" | "cpp" | "cxx" | "hpp" | "hh" => Some("cpp"),
        "cs" => Some("csharp"),
        "rb" => Some("ruby"),
        "php" => Some("php"),
        "swift" => Some("swift"),
        "scala" => Some("scala"),
        "sh" | "bash" | "zsh" => Some("shell"),
        "sql" => Some("sql"),
        "html" | "htm" => Some("html"),
        "css" | "scss" | "sass" | "less" => Some("css"),
        "md" | "mdx" => Some("markdown"),
        "json" | "jsonc" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        "xml" => Some("xml"),
        _ => None,
    }
}

fn line_count(text: &str) -> u64 {
    if text.is_empty() {
        0
    } else {
        text.bytes().filter(|byte| *byte == b'\n').count() as u64 + u64::from(!text.ends_with('\n'))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn manifest_relative_path(project_id: &str) -> String {
    format!(
        "{STORE_ROOT}/{AGENT_MEMORY_DIRECTORY}/{PROJECTS_DIRECTORY}/{project_id}/{ARTIFACTS_DIRECTORY}/{MANIFEST_FILE}"
    )
}

fn graph_relative_path(project_id: &str) -> String {
    format!(
        "{STORE_ROOT}/{AGENT_MEMORY_DIRECTORY}/{PROJECTS_DIRECTORY}/{project_id}/{GRAPH_DIRECTORY}/{GRAPH_MANIFEST_FILE}"
    )
}

fn store_io(name: &str, source: std::io::Error) -> LeyCoreError {
    LeyCoreError::Io {
        path: PathBuf::from(name),
        source,
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
    use crate::{initialize_project, CaptureMode, LEY_DIRECTORY};
    use std::sync::{Arc, Barrier};
    use tempfile::tempdir;

    fn setup_project(mode: CaptureMode) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Ingestion test"), mode).unwrap();
        (base, project, vault)
    }

    fn manifest_path(project: &Path, vault: &Path) -> PathBuf {
        let project_id = diagnose_project(project).unwrap().identity.project_id;
        vault
            .join(STORE_ROOT)
            .join(AGENT_MEMORY_DIRECTORY)
            .join(PROJECTS_DIRECTORY)
            .join(project_id)
            .join(ARTIFACTS_DIRECTORY)
            .join(MANIFEST_FILE)
    }

    fn graph_path(project: &Path, vault: &Path) -> PathBuf {
        let project_id = diagnose_project(project).unwrap().identity.project_id;
        vault
            .join(STORE_ROOT)
            .join(AGENT_MEMORY_DIRECTORY)
            .join(PROJECTS_DIRECTORY)
            .join(project_id)
            .join(GRAPH_DIRECTORY)
            .join(GRAPH_MANIFEST_FILE)
    }

    #[test]
    fn ingestion_persists_redacted_text_and_reports_unsupported_files() {
        let (_base, project, vault) = setup_project(CaptureMode::Structured);
        std::fs::write(project.join("README.md"), "# Example\n").unwrap();
        std::fs::write(
            project.join("config.toml"),
            "api_key = \"sk-abcdefghijklmnopqrstuvwxyz123456\"\nurl = \"postgres://ley:hunter2@localhost/db\"\n",
        )
        .unwrap();
        std::fs::write(project.join("binary.dat"), b"hello\0world").unwrap();
        std::fs::write(project.join("legacy.txt"), [0xff, 0xfe, 0xfd]).unwrap();
        std::fs::write(project.join(".env"), "PASSWORD=must-never-appear").unwrap();

        let result = ingest_project(&project, &vault).unwrap();
        assert!(result.changed);
        assert_eq!(result.files, 2);
        assert_eq!(result.stored_files, 2);
        assert_eq!(result.redacted_files, 1);
        assert_eq!(
            result
                .skipped
                .iter()
                .map(|item| (item.path.as_str(), item.reason))
                .collect::<Vec<_>>(),
            vec![
                ("binary.dat", ArtifactSkipReason::Binary),
                ("legacy.txt", ArtifactSkipReason::NonUtf8),
            ]
        );

        let manifest_body = std::fs::read_to_string(manifest_path(&project, &vault)).unwrap();
        assert!(!manifest_body.contains("abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!manifest_body.contains("hunter2"));
        assert!(!manifest_body.contains("must-never-appear"));
        let graph_body = std::fs::read_to_string(graph_path(&project, &vault)).unwrap();
        assert!(!graph_body.contains("abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!graph_body.contains("hunter2"));
        assert!(!graph_body.contains("must-never-appear"));
        let manifest: ArtifactManifest = serde_json::from_str(&manifest_body).unwrap();
        let config = manifest
            .files
            .iter()
            .find(|file| file.path == "config.toml")
            .unwrap();
        assert_eq!(config.kind, ArtifactKind::Configuration);
        assert!(!config.redactions.is_empty());
        let blob = config.content_blob.as_ref().unwrap();
        let blob_body =
            std::fs::read_to_string(manifest_path(&project, &vault).parent().unwrap().join(blob))
                .unwrap();
        assert!(blob_body.contains("[REDACTED:credential-assignment]"));
        assert!(blob_body.contains("[REDACTED:credential-url]"));
        assert!(!blob_body.contains("hunter2"));
    }

    #[test]
    fn incremental_ingestion_is_idempotent_and_tracks_precise_changes() {
        let (_base, project, vault) = setup_project(CaptureMode::Structured);
        std::fs::write(project.join("modify.md"), "before\n").unwrap();
        std::fs::write(project.join("rename.md"), "stable rename content\n").unwrap();
        std::fs::write(project.join("delete.md"), "delete me\n").unwrap();

        let first = ingest_project(&project, &vault).unwrap();
        assert!(first.changed);
        let unchanged = ingest_project(&project, &vault).unwrap();
        assert!(!unchanged.changed);
        assert!(!unchanged.graph_changed);
        assert_eq!(unchanged.snapshot_id, first.snapshot_id);
        let snapshots = manifest_path(&project, &vault)
            .parent()
            .unwrap()
            .join(SNAPSHOTS_DIRECTORY);
        assert_eq!(std::fs::read_dir(&snapshots).unwrap().count(), 1);
        assert_eq!(
            std::fs::read_dir(
                graph_path(&project, &vault)
                    .parent()
                    .unwrap()
                    .join(SNAPSHOTS_DIRECTORY)
            )
            .unwrap()
            .count(),
            1
        );

        std::fs::write(project.join("modify.md"), "after\n").unwrap();
        std::fs::rename(project.join("rename.md"), project.join("renamed.md")).unwrap();
        std::fs::remove_file(project.join("delete.md")).unwrap();
        std::fs::write(project.join("added.md"), "new\n").unwrap();
        let changed = ingest_project(&project, &vault).unwrap();

        assert!(changed.changed);
        assert_eq!(changed.added, vec!["added.md"]);
        assert_eq!(changed.modified, vec!["modify.md"]);
        assert_eq!(changed.deleted, vec!["delete.md"]);
        assert_eq!(
            changed.renamed,
            vec![RenamedArtifact {
                from: "rename.md".to_owned(),
                to: "renamed.md".to_owned(),
                content_hash: format!("sha256:{}", sha256_hex(b"stable rename content\n")),
            }]
        );
        assert_ne!(changed.snapshot_id, first.snapshot_id);
        assert_eq!(std::fs::read_dir(&snapshots).unwrap().count(), 2);

        let capture_path = project.join(LEY_DIRECTORY).join(crate::CAPTURE_FILE);
        let mut capture: crate::CapturePolicy =
            serde_json::from_str(&std::fs::read_to_string(&capture_path).unwrap()).unwrap();
        capture.max_total_bytes -= 1;
        std::fs::write(&capture_path, serde_json::to_vec_pretty(&capture).unwrap()).unwrap();
        let policy_change = ingest_project(&project, &vault).unwrap();
        assert!(policy_change.changed);
        assert!(policy_change.added.is_empty());
        assert!(policy_change.modified.is_empty());
        assert_eq!(std::fs::read_dir(&snapshots).unwrap().count(), 3);

        let ignore_path = project.join(LEY_DIRECTORY).join(crate::IGNORE_FILE);
        let mut ignore = std::fs::OpenOptions::new()
            .append(true)
            .open(ignore_path)
            .unwrap();
        writeln!(ignore, "# audit-only rule comment").unwrap();
        let ignore_change = ingest_project(&project, &vault).unwrap();
        assert!(ignore_change.changed);
        assert_eq!(std::fs::read_dir(snapshots).unwrap().count(), 4);
    }

    #[test]
    fn minimal_capture_keeps_metadata_without_source_blobs() {
        let (_base, project, vault) = setup_project(CaptureMode::Minimal);
        std::fs::write(project.join("main.rs"), "fn main() {}\n").unwrap();

        let result = ingest_project(&project, &vault).unwrap();
        assert_eq!(result.files, 1);
        assert_eq!(result.stored_files, 0);
        assert!(result.graph_nodes >= 3);
        let manifest: ArtifactManifest = serde_json::from_str(
            &std::fs::read_to_string(manifest_path(&project, &vault)).unwrap(),
        )
        .unwrap();
        assert!(manifest.files[0].content_blob.is_none());
        let content = manifest_path(&project, &vault)
            .parent()
            .unwrap()
            .join(CONTENT_DIRECTORY);
        assert_eq!(std::fs::read_dir(content).unwrap().count(), 0);
        let graph = read_project_graph(&project, &vault).unwrap();
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == crate::GraphNodeKind::Symbol && node.name == "main"));
    }

    #[cfg(unix)]
    #[test]
    fn ingestion_audits_size_limits_and_symlink_skips() {
        use std::os::unix::fs::symlink;

        let (_base, project, vault) = setup_project(CaptureMode::Structured);
        std::fs::write(project.join("included.txt"), "aaa").unwrap();
        std::fs::write(project.join("over-total.txt"), "bbb").unwrap();
        std::fs::write(project.join("oversized.txt"), "ccccc").unwrap();
        let outside = tempdir().unwrap();
        std::fs::write(outside.path().join("outside.txt"), "outside").unwrap();
        symlink(
            outside.path().join("outside.txt"),
            project.join("linked.txt"),
        )
        .unwrap();
        let capture_path = project.join(LEY_DIRECTORY).join(crate::CAPTURE_FILE);
        let mut capture: crate::CapturePolicy =
            serde_json::from_str(&std::fs::read_to_string(&capture_path).unwrap()).unwrap();
        capture.max_file_bytes = 4;
        capture.max_total_bytes = 4;
        std::fs::write(&capture_path, serde_json::to_vec_pretty(&capture).unwrap()).unwrap();

        let result = ingest_project(&project, &vault).unwrap();
        assert_eq!(result.files, 1);
        assert_eq!(
            result
                .skipped
                .iter()
                .map(|item| (item.path.as_str(), item.reason))
                .collect::<Vec<_>>(),
            vec![
                ("linked.txt", ArtifactSkipReason::Symlink),
                ("over-total.txt", ArtifactSkipReason::TotalLimit),
                ("oversized.txt", ArtifactSkipReason::Oversized),
            ]
        );
    }

    #[test]
    fn redaction_covers_private_keys_assignments_provider_tokens_and_urls() {
        let source = concat!(
            "const password = \"weak-password\"\n",
            "\"apiKey\": \"sk-abcdefghijklmnopqrstuvwxyz123456\"\n",
            "token ghp_abcdefghijklmnopqrstuvwxyz123456\n",
            "db=postgres://user:pass@localhost/db\n",
            "-----BEGIN RSA PRIVATE KEY-----\n",
            "private material\n",
            "-----END RSA PRIVATE KEY-----\n"
        );
        let (redacted, findings) = redact_secrets(source);
        assert!(!redacted.contains("weak-password"));
        assert!(!redacted.contains("abcdefghijklmnopqrstuvwxyz123456"));
        assert!(!redacted.contains(":pass@"));
        assert!(!redacted.contains("private material"));
        assert_eq!(
            findings
                .iter()
                .map(|finding| finding.kind.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "credential-assignment",
                "credential-url",
                "private-key",
                "provider-token",
            ])
        );
        let provider = findings
            .iter()
            .find(|finding| finding.kind == "provider-token")
            .unwrap();
        assert_eq!(provider.lines, vec![3]);
        assert_eq!(line_count(&redacted), line_count(source));
    }

    #[test]
    fn refuses_self_ingestion_and_concurrent_writers_do_not_lose_state() {
        let (_base, project, vault) = setup_project(CaptureMode::Structured);
        std::fs::write(project.join("note.md"), "content\n").unwrap();
        assert!(matches!(
            ingest_project(&project, &project),
            Err(LeyCoreError::OverlappingProjectVault(_))
        ));
        let nested_vault = project.join("vault");
        std::fs::create_dir(&nested_vault).unwrap();
        assert!(matches!(
            ingest_project(&project, &nested_vault),
            Err(LeyCoreError::OverlappingProjectVault(_))
        ));

        let barrier = Arc::new(Barrier::new(4));
        let mut workers = Vec::new();
        for _ in 0..4 {
            let worker_project = project.clone();
            let worker_vault = vault.clone();
            let worker_barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                worker_barrier.wait();
                ingest_project(worker_project, worker_vault).unwrap()
            }));
        }
        let results = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.changed).count(), 1);
        assert_eq!(
            results
                .iter()
                .map(|result| result.snapshot_id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            1
        );
    }

    #[cfg(unix)]
    #[test]
    fn capability_reads_and_store_creation_refuse_symlink_escape() {
        use std::os::unix::fs::symlink;

        let (_base, project, vault) = setup_project(CaptureMode::Structured);
        let outside = tempdir().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "outside secret").unwrap();
        symlink(
            outside.path().join("secret.txt"),
            project.join("linked.txt"),
        )
        .unwrap();
        let root = Dir::open_ambient_dir(&project, ambient_authority()).unwrap();
        assert!(read_scoped_file(&root, "linked.txt", 14, 1024).is_err());

        symlink(outside.path(), vault.join(LEY_DIRECTORY)).unwrap();
        assert!(ingest_project(&project, &vault).is_err());
        assert!(!outside.path().join(AGENT_MEMORY_DIRECTORY).exists());
    }

    #[cfg(unix)]
    #[test]
    fn durable_evidence_uses_private_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let (_base, project, vault) = setup_project(CaptureMode::Structured);
        std::fs::write(project.join("note.md"), "private project content\n").unwrap();
        ingest_project(&project, &vault).unwrap();
        let manifest_path = manifest_path(&project, &vault);
        let manifest: ArtifactManifest =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let blob = manifest.files[0].content_blob.as_ref().unwrap();
        let graph_path = graph_path(&project, &vault);
        let graph: ProjectGraph =
            serde_json::from_str(&std::fs::read_to_string(&graph_path).unwrap()).unwrap();

        for path in [
            manifest_path.clone(),
            manifest_path.parent().unwrap().join(INGEST_LOCK_FILE),
            manifest_path.parent().unwrap().join(blob),
            manifest_path
                .parent()
                .unwrap()
                .join(SNAPSHOTS_DIRECTORY)
                .join(format!("{}.json", manifest.snapshot_id)),
            graph_path.clone(),
            graph_path
                .parent()
                .unwrap()
                .join(SNAPSHOTS_DIRECTORY)
                .join(format!("{}.json", graph.graph_snapshot_id)),
        ] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn ingestion_detects_durable_blob_corruption_before_a_noop() {
        let (_base, project, vault) = setup_project(CaptureMode::Structured);
        std::fs::write(project.join("note.md"), "trusted evidence\n").unwrap();
        ingest_project(&project, &vault).unwrap();
        let manifest_path = manifest_path(&project, &vault);
        let manifest: ArtifactManifest =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        let blob = manifest.files[0].content_blob.as_ref().unwrap();
        std::fs::write(
            manifest_path.parent().unwrap().join(blob),
            "corrupted evidence\n",
        )
        .unwrap();

        assert!(matches!(
            ingest_project(&project, &vault),
            Err(LeyCoreError::InvalidArtifactStore(_))
        ));
    }

    #[test]
    fn graph_reads_detect_current_and_immutable_snapshot_corruption() {
        let (_base, project, vault) = setup_project(CaptureMode::Structured);
        std::fs::write(project.join("main.py"), "def recall():\n    return True\n").unwrap();
        ingest_project(&project, &vault).unwrap();
        let graph_path = graph_path(&project, &vault);
        let graph: ProjectGraph =
            serde_json::from_str(&std::fs::read_to_string(&graph_path).unwrap()).unwrap();
        let snapshot_path = graph_path
            .parent()
            .unwrap()
            .join(SNAPSHOTS_DIRECTORY)
            .join(format!("{}.json", graph.graph_snapshot_id));
        std::fs::write(&snapshot_path, "{}\n").unwrap();

        assert!(matches!(
            read_project_graph(&project, &vault),
            Err(LeyCoreError::InvalidProjectGraph(_))
        ));
    }
}
