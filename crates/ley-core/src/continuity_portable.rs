use crate::continuity_store::project_events_from_portable_database;
use crate::ingestion::{
    collect_artifact_citations, read_portable_artifact_snapshots, validate_relative_artifact_path,
    PortableArtifactBlob, PortableArtifactCitation, PortableArtifactReference,
    PortableArtifactSnapshot,
};
use crate::{validate_project_id, ContinuityStore, LeyCoreError};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const PORTABLE_CONTINUITY_BUNDLE_VERSION: u32 = 1;
const BUNDLE_MANIFEST_FILE: &str = "manifest.json";
const BUNDLE_DATABASE_FILE: &str = "continuity.sqlite3";
const EVIDENCE_DIRECTORY: &str = "evidence";
const SNAPSHOTS_DIRECTORY: &str = "snapshots";
const CONTENT_DIRECTORY: &str = "content";
const BUNDLE_MANIFEST_LIMIT_BYTES: u64 = 16 * 1024 * 1024;
const ARTIFACT_MANIFEST_LIMIT_BYTES: u64 = 64 * 1024 * 1024;
const PORTABLE_EVIDENCE_SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PortableEvidenceArtifactEntry {
    artifact_path: String,
    content_hash: String,
    file_name: String,
    bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PortableEvidenceSnapshotFile {
    schema_version: u32,
    project_id: String,
    artifact_snapshot_id: String,
    artifacts: Vec<PortableEvidenceArtifactEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableContinuitySnapshotEntry {
    pub artifact_snapshot_id: String,
    pub manifest_sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableContinuityBlobEntry {
    pub content_hash: String,
    pub file_name: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableContinuityBundleManifest {
    pub schema_version: u32,
    pub project_id: String,
    pub database_sha256: String,
    pub database_bytes: u64,
    pub event_count: usize,
    pub artifact_snapshots: Vec<PortableContinuitySnapshotEntry>,
    pub evidence_blobs: Vec<PortableContinuityBlobEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableContinuityImport {
    pub project_id: String,
    pub root: PathBuf,
    pub database_path: PathBuf,
    pub evidence_root: PathBuf,
    pub event_count: usize,
}

pub fn export_portable_continuity(
    store: &ContinuityStore,
    legacy_vault: impl AsRef<Path>,
    project_id: &str,
    destination: impl AsRef<Path>,
) -> Result<PortableContinuityBundleManifest, LeyCoreError> {
    let legacy_vault = legacy_vault.as_ref();
    store.with_artifact_authority_lock(|| {
        export_portable_with_evidence(store, project_id, destination.as_ref(), |citations| {
            match store.read_native_portable_artifact_snapshots(project_id, citations)? {
                Some(snapshots) => Ok(snapshots),
                None => read_portable_artifact_snapshots(legacy_vault, project_id, citations),
            }
        })
    })
}

pub fn export_portable_continuity_from_evidence_root(
    store: &ContinuityStore,
    evidence_root: impl AsRef<Path>,
    project_id: &str,
    destination: impl AsRef<Path>,
) -> Result<PortableContinuityBundleManifest, LeyCoreError> {
    let evidence_root = evidence_root.as_ref();
    store.with_artifact_authority_lock(|| {
        export_portable_with_evidence(store, project_id, destination.as_ref(), |citations| {
            read_artifact_snapshots_from_evidence_root(evidence_root, project_id, citations)
        })
    })
}

fn export_portable_with_evidence<F>(
    store: &ContinuityStore,
    project_id: &str,
    destination: &Path,
    evidence_loader: F,
) -> Result<PortableContinuityBundleManifest, LeyCoreError>
where
    F: FnOnce(&[PortableArtifactCitation]) -> Result<Vec<PortableArtifactSnapshot>, LeyCoreError>,
{
    validate_project_id(project_id)?;
    ensure_destination_absent(destination)?;
    let private_database = PrivateDatabaseSnapshot::create(store, project_id)?;
    let staging = StagingDirectory::create(destination)?;
    let database_path = staging.path().join(BUNDLE_DATABASE_FILE);
    copy_regular_file(private_database.path(), &database_path)?;
    let events = project_events_from_portable_database(&database_path, project_id)?;
    let citations = collect_artifact_citations(&events)?;
    let snapshots = evidence_loader(&citations)?;

    let evidence_root = staging.path().join(EVIDENCE_DIRECTORY);
    let snapshots_root = evidence_root.join(SNAPSHOTS_DIRECTORY);
    let content_root = evidence_root.join(CONTENT_DIRECTORY);
    create_private_directory(&evidence_root)?;
    create_private_directory(&snapshots_root)?;
    create_private_directory(&content_root)?;

    let mut snapshot_entries = Vec::with_capacity(snapshots.len());
    let mut blob_entries = BTreeMap::<String, PortableContinuityBlobEntry>::new();
    for snapshot in snapshots {
        let PortableArtifactSnapshot {
            snapshot_id,
            artifacts,
            blobs,
        } = snapshot;
        let mut artifact_entries = artifacts
            .into_iter()
            .map(|artifact| PortableEvidenceArtifactEntry {
                artifact_path: artifact.artifact_path,
                content_hash: artifact.content_hash,
                file_name: artifact.file_name,
                bytes: artifact.bytes,
            })
            .collect::<Vec<_>>();
        artifact_entries.sort_by(|left, right| left.artifact_path.cmp(&right.artifact_path));
        let snapshot_file = PortableEvidenceSnapshotFile {
            schema_version: PORTABLE_EVIDENCE_SNAPSHOT_VERSION,
            project_id: project_id.to_owned(),
            artifact_snapshot_id: snapshot_id.clone(),
            artifacts: artifact_entries,
        };
        let snapshot_bytes = serialize_evidence_snapshot(&snapshot_file)?;
        let snapshot_path = snapshots_root.join(format!("{snapshot_id}.json"));
        write_private_file(&snapshot_path, &snapshot_bytes)?;
        snapshot_entries.push(PortableContinuitySnapshotEntry {
            artifact_snapshot_id: snapshot_id,
            manifest_sha256: sha256_digest(&snapshot_bytes),
            bytes: snapshot_bytes.len() as u64,
        });
        for blob in blobs {
            let path = content_root.join(&blob.file_name);
            if path.exists() {
                let existing = hash_file(&path)?;
                if existing.0 != blob.content_hash || existing.1 != blob.bytes.len() as u64 {
                    return Err(invalid_bundle(format!(
                        "content-addressed collision for {}",
                        blob.content_hash
                    )));
                }
            } else {
                write_private_file(&path, &blob.bytes)?;
            }
            match blob_entries.get(&blob.content_hash) {
                Some(existing)
                    if existing.file_name != blob.file_name
                        || existing.bytes != blob.bytes.len() as u64 =>
                {
                    return Err(invalid_bundle(format!(
                        "conflicting evidence metadata for {}",
                        blob.content_hash
                    )));
                }
                Some(_) => {}
                None => {
                    blob_entries.insert(
                        blob.content_hash.clone(),
                        PortableContinuityBlobEntry {
                            content_hash: blob.content_hash,
                            file_name: blob.file_name,
                            bytes: blob.bytes.len() as u64,
                        },
                    );
                }
            }
        }
    }
    snapshot_entries
        .sort_by(|left, right| left.artifact_snapshot_id.cmp(&right.artifact_snapshot_id));
    let (database_sha256, database_bytes) = hash_file(&database_path)?;
    let manifest = PortableContinuityBundleManifest {
        schema_version: PORTABLE_CONTINUITY_BUNDLE_VERSION,
        project_id: project_id.to_owned(),
        database_sha256,
        database_bytes,
        event_count: events.len(),
        artifact_snapshots: snapshot_entries,
        evidence_blobs: blob_entries.into_values().collect(),
    };
    let manifest_bytes = serialize_bundle_manifest(&manifest)?;
    write_private_file(&staging.path().join(BUNDLE_MANIFEST_FILE), &manifest_bytes)?;
    let validated = validate_portable_bundle(staging.path())?;
    if validated != manifest {
        return Err(invalid_bundle(
            "portable bundle changed during export validation".to_owned(),
        ));
    }
    staging.commit(destination)?;
    Ok(manifest)
}

pub fn import_portable_continuity(
    bundle: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> Result<PortableContinuityImport, LeyCoreError> {
    let bundle = bundle.as_ref();
    let destination = destination.as_ref();
    ensure_destination_absent(destination)?;
    let source = BundleSource::open(bundle)?;
    let manifest_bytes =
        source.read_root_file(BUNDLE_MANIFEST_FILE, BUNDLE_MANIFEST_LIMIT_BYTES)?;
    let manifest = parse_bundle_manifest(&manifest_bytes)?;
    validate_manifest_shape(&manifest)?;

    let staging = StagingDirectory::create(destination)?;
    source.copy_root_file(
        BUNDLE_DATABASE_FILE,
        &staging.path().join(BUNDLE_DATABASE_FILE),
    )?;
    let evidence_root = staging.path().join(EVIDENCE_DIRECTORY);
    let snapshots_root = evidence_root.join(SNAPSHOTS_DIRECTORY);
    let content_root = evidence_root.join(CONTENT_DIRECTORY);
    create_private_directory(&evidence_root)?;
    create_private_directory(&snapshots_root)?;
    create_private_directory(&content_root)?;
    for snapshot in &manifest.artifact_snapshots {
        source.copy_snapshot(
            &snapshot.artifact_snapshot_id,
            &snapshots_root.join(format!("{}.json", snapshot.artifact_snapshot_id)),
        )?;
    }
    for blob in &manifest.evidence_blobs {
        source.copy_blob(&blob.file_name, &content_root.join(&blob.file_name))?;
    }
    write_private_file(&staging.path().join(BUNDLE_MANIFEST_FILE), &manifest_bytes)?;

    let validated = validate_portable_bundle(staging.path())?;
    if validated != manifest {
        return Err(invalid_bundle(
            "portable bundle changed while being imported".to_owned(),
        ));
    }
    fs::remove_file(staging.path().join(BUNDLE_MANIFEST_FILE)).map_err(|source| {
        LeyCoreError::Io {
            path: staging.path().join(BUNDLE_MANIFEST_FILE),
            source,
        }
    })?;
    staging.commit(destination)?;
    Ok(PortableContinuityImport {
        project_id: manifest.project_id,
        root: destination.to_path_buf(),
        database_path: destination.join(BUNDLE_DATABASE_FILE),
        evidence_root: destination.join(EVIDENCE_DIRECTORY),
        event_count: manifest.event_count,
    })
}

pub fn read_portable_cited_evidence(
    bundle: impl AsRef<Path>,
    project_id: &str,
    artifact_snapshot_id: &str,
    artifact_path: &str,
    content_hash: &str,
) -> Result<Vec<u8>, LeyCoreError> {
    let bundle = bundle.as_ref();
    let manifest = validate_portable_bundle(bundle)?;
    if manifest.project_id != project_id {
        return Err(invalid_bundle(
            "requested project does not match portable bundle".to_owned(),
        ));
    }
    read_continuity_evidence(
        bundle.join(EVIDENCE_DIRECTORY),
        project_id,
        artifact_snapshot_id,
        artifact_path,
        content_hash,
    )
}

pub fn read_continuity_evidence(
    evidence_root: impl AsRef<Path>,
    project_id: &str,
    artifact_snapshot_id: &str,
    artifact_path: &str,
    content_hash: &str,
) -> Result<Vec<u8>, LeyCoreError> {
    validate_project_id(project_id)?;
    if !valid_snapshot_id(artifact_snapshot_id) || !is_sha256(content_hash) {
        return Err(invalid_bundle(
            "portable evidence citation identifiers are invalid".to_owned(),
        ));
    }
    let evidence_root = evidence_root.as_ref();
    validate_directory(evidence_root, "continuity evidence root")?;
    let snapshots_root = evidence_root.join(SNAPSHOTS_DIRECTORY);
    let content_root = evidence_root.join(CONTENT_DIRECTORY);
    validate_directory(&snapshots_root, "continuity snapshot directory")?;
    validate_directory(&content_root, "continuity content directory")?;
    let snapshot_path = snapshots_root.join(format!("{artifact_snapshot_id}.json"));
    let snapshot_bytes = read_regular_file(&snapshot_path, ARTIFACT_MANIFEST_LIMIT_BYTES)?;
    let snapshot = parse_evidence_snapshot(&snapshot_bytes, project_id, artifact_snapshot_id)?;
    let artifact = snapshot
        .artifacts
        .iter()
        .find(|artifact| artifact.artifact_path == artifact_path)
        .ok_or_else(|| invalid_bundle("cited artifact is absent from snapshot".to_owned()))?;
    if artifact.content_hash != content_hash {
        return Err(invalid_bundle(
            "cited artifact content hash does not match snapshot".to_owned(),
        ));
    }
    let bytes = read_regular_file(&content_root.join(&artifact.file_name), artifact.bytes)?;
    if bytes.len() as u64 != artifact.bytes || sha256_digest(&bytes) != content_hash {
        return Err(invalid_bundle(
            "portable evidence blob failed integrity verification".to_owned(),
        ));
    }
    Ok(bytes)
}

fn read_artifact_snapshots_from_evidence_root(
    evidence_root: &Path,
    project_id: &str,
    citations: &[PortableArtifactCitation],
) -> Result<Vec<PortableArtifactSnapshot>, LeyCoreError> {
    validate_project_id(project_id)?;
    if citations.is_empty() {
        return Ok(Vec::new());
    }
    validate_directory(evidence_root, "continuity evidence root")?;
    let snapshots_root = evidence_root.join(SNAPSHOTS_DIRECTORY);
    let content_root = evidence_root.join(CONTENT_DIRECTORY);
    validate_directory(&snapshots_root, "continuity snapshot directory")?;
    validate_directory(&content_root, "continuity content directory")?;

    let mut grouped = BTreeMap::<String, BTreeMap<String, String>>::new();
    for citation in citations {
        let paths = grouped
            .entry(citation.artifact_snapshot_id.clone())
            .or_default();
        if let Some(existing) = paths.insert(
            citation.artifact_path.clone(),
            citation.content_hash.clone(),
        ) {
            if existing != citation.content_hash {
                return Err(invalid_bundle(format!(
                    "conflicting cited hashes for {} in {}",
                    citation.artifact_path, citation.artifact_snapshot_id
                )));
            }
        }
    }

    let mut snapshots = Vec::with_capacity(grouped.len());
    for (snapshot_id, requested) in grouped {
        let snapshot_bytes = read_regular_file(
            &snapshots_root.join(format!("{snapshot_id}.json")),
            ARTIFACT_MANIFEST_LIMIT_BYTES,
        )?;
        let snapshot_file = parse_evidence_snapshot(&snapshot_bytes, project_id, &snapshot_id)?;
        let mut artifacts = Vec::with_capacity(requested.len());
        let mut blobs = BTreeMap::<String, PortableArtifactBlob>::new();
        for (artifact_path, content_hash) in requested {
            let artifact = snapshot_file
                .artifacts
                .iter()
                .find(|artifact| artifact.artifact_path == artifact_path)
                .ok_or_else(|| {
                    invalid_bundle(format!(
                        "cited artifact {artifact_path} is absent from snapshot {snapshot_id}"
                    ))
                })?;
            if artifact.content_hash != content_hash {
                return Err(invalid_bundle(format!(
                    "cited content hash does not match {artifact_path} in snapshot {snapshot_id}"
                )));
            }
            let bytes = read_regular_file(&content_root.join(&artifact.file_name), artifact.bytes)?;
            if bytes.len() as u64 != artifact.bytes || sha256_digest(&bytes) != content_hash {
                return Err(invalid_bundle(format!(
                    "installed evidence blob failed integrity verification for {artifact_path}"
                )));
            }
            artifacts.push(PortableArtifactReference {
                artifact_path,
                content_hash: content_hash.clone(),
                file_name: artifact.file_name.clone(),
                bytes: artifact.bytes,
            });
            match blobs.get(&content_hash) {
                Some(existing)
                    if existing.file_name != artifact.file_name || existing.bytes != bytes =>
                {
                    return Err(invalid_bundle(format!(
                        "content-addressed evidence collision for {content_hash}"
                    )));
                }
                Some(_) => {}
                None => {
                    blobs.insert(
                        content_hash.clone(),
                        PortableArtifactBlob {
                            content_hash,
                            file_name: artifact.file_name.clone(),
                            bytes,
                        },
                    );
                }
            }
        }
        snapshots.push(PortableArtifactSnapshot {
            snapshot_id,
            artifacts,
            blobs: blobs.into_values().collect(),
        });
    }
    Ok(snapshots)
}

fn validate_portable_bundle(root: &Path) -> Result<PortableContinuityBundleManifest, LeyCoreError> {
    validate_directory(root, "portable bundle root")?;
    let manifest_bytes = read_regular_file(
        &root.join(BUNDLE_MANIFEST_FILE),
        BUNDLE_MANIFEST_LIMIT_BYTES,
    )?;
    let manifest = parse_bundle_manifest(&manifest_bytes)?;
    validate_manifest_shape(&manifest)?;
    let database_path = root.join(BUNDLE_DATABASE_FILE);
    let (database_hash, database_bytes) = hash_file(&database_path)?;
    if database_hash != manifest.database_sha256 || database_bytes != manifest.database_bytes {
        return Err(invalid_bundle(
            "portable database hash/size does not match bundle manifest".to_owned(),
        ));
    }
    let events = project_events_from_portable_database(&database_path, &manifest.project_id)?;
    if events.len() != manifest.event_count {
        return Err(invalid_bundle(
            "portable database event count does not match bundle manifest".to_owned(),
        ));
    }
    let citations = collect_artifact_citations(&events)?;
    validate_evidence_tree(root, &manifest, &citations)?;
    Ok(manifest)
}

fn validate_evidence_tree(
    root: &Path,
    manifest: &PortableContinuityBundleManifest,
    citations: &[PortableArtifactCitation],
) -> Result<(), LeyCoreError> {
    let snapshots_root = root.join(EVIDENCE_DIRECTORY).join(SNAPSHOTS_DIRECTORY);
    let content_root = root.join(EVIDENCE_DIRECTORY).join(CONTENT_DIRECTORY);
    validate_directory(&snapshots_root, "portable snapshot directory")?;
    validate_directory(&content_root, "portable content directory")?;

    let required_snapshots = citations
        .iter()
        .map(|citation| citation.artifact_snapshot_id.clone())
        .collect::<BTreeSet<_>>();
    let declared_snapshots = manifest
        .artifact_snapshots
        .iter()
        .map(|entry| entry.artifact_snapshot_id.clone())
        .collect::<BTreeSet<_>>();
    if required_snapshots != declared_snapshots {
        return Err(invalid_bundle(
            "portable snapshot set does not match database citations".to_owned(),
        ));
    }

    let mut required_blobs = BTreeMap::<String, (String, u64)>::new();
    for entry in &manifest.artifact_snapshots {
        let path = snapshots_root.join(format!("{}.json", entry.artifact_snapshot_id));
        let bytes = read_regular_file(&path, ARTIFACT_MANIFEST_LIMIT_BYTES)?;
        if bytes.len() as u64 != entry.bytes || sha256_digest(&bytes) != entry.manifest_sha256 {
            return Err(invalid_bundle(format!(
                "artifact snapshot {} failed bundle integrity verification",
                entry.artifact_snapshot_id
            )));
        }
        let snapshot =
            parse_evidence_snapshot(&bytes, &manifest.project_id, &entry.artifact_snapshot_id)?;
        let expected_artifacts = citations
            .iter()
            .filter(|citation| citation.artifact_snapshot_id == entry.artifact_snapshot_id)
            .map(|citation| {
                (
                    citation.artifact_path.clone(),
                    citation.content_hash.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let declared_artifacts = snapshot
            .artifacts
            .iter()
            .map(|artifact| {
                (
                    artifact.artifact_path.clone(),
                    artifact.content_hash.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if declared_artifacts != expected_artifacts {
            return Err(invalid_bundle(format!(
                "portable snapshot {} does not exactly match database citations",
                entry.artifact_snapshot_id
            )));
        }
        for artifact in &snapshot.artifacts {
            match required_blobs.insert(
                artifact.content_hash.clone(),
                (artifact.file_name.clone(), artifact.bytes),
            ) {
                Some(existing) if existing != (artifact.file_name.clone(), artifact.bytes) => {
                    return Err(invalid_bundle(format!(
                        "conflicting blob metadata for {}",
                        artifact.content_hash
                    )));
                }
                _ => {}
            }
        }
    }

    let declared_blobs = manifest
        .evidence_blobs
        .iter()
        .map(|entry| {
            (
                entry.content_hash.clone(),
                (entry.file_name.clone(), entry.bytes),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if declared_blobs != required_blobs {
        return Err(invalid_bundle(
            "portable blob set does not match cited evidence".to_owned(),
        ));
    }
    for entry in &manifest.evidence_blobs {
        let path = content_root.join(&entry.file_name);
        let (hash, bytes) = hash_file(&path)?;
        if hash != entry.content_hash || bytes != entry.bytes {
            return Err(invalid_bundle(format!(
                "portable evidence blob {} failed integrity verification",
                entry.content_hash
            )));
        }
    }
    Ok(())
}

fn parse_bundle_manifest(bytes: &[u8]) -> Result<PortableContinuityBundleManifest, LeyCoreError> {
    serde_json::from_slice(bytes)
        .map_err(|error| invalid_bundle(format!("invalid bundle manifest JSON: {error}")))
}

fn parse_evidence_snapshot(
    bytes: &[u8],
    project_id: &str,
    artifact_snapshot_id: &str,
) -> Result<PortableEvidenceSnapshotFile, LeyCoreError> {
    let snapshot: PortableEvidenceSnapshotFile =
        serde_json::from_slice(bytes).map_err(|error| {
            invalid_bundle(format!("invalid portable evidence snapshot JSON: {error}"))
        })?;
    if snapshot.schema_version != PORTABLE_EVIDENCE_SNAPSHOT_VERSION
        || snapshot.project_id != project_id
        || snapshot.artifact_snapshot_id != artifact_snapshot_id
        || !valid_snapshot_id(&snapshot.artifact_snapshot_id)
        || snapshot.artifacts.is_empty()
    {
        return Err(invalid_bundle(
            "portable evidence snapshot identity is invalid".to_owned(),
        ));
    }
    let mut last_path: Option<&str> = None;
    for artifact in &snapshot.artifacts {
        validate_relative_artifact_path(&artifact.artifact_path)?;
        if !is_sha256(&artifact.content_hash)
            || !valid_blob_name(&artifact.file_name, &artifact.content_hash)
            || last_path.is_some_and(|last| last >= artifact.artifact_path.as_str())
        {
            return Err(invalid_bundle(
                "portable evidence artifact metadata is invalid".to_owned(),
            ));
        }
        last_path = Some(&artifact.artifact_path);
    }
    Ok(snapshot)
}

fn serialize_evidence_snapshot(
    snapshot: &PortableEvidenceSnapshotFile,
) -> Result<Vec<u8>, LeyCoreError> {
    let mut bytes = serde_json::to_vec_pretty(snapshot).map_err(|error| {
        invalid_bundle(format!(
            "portable evidence snapshot is not serializable: {error}"
        ))
    })?;
    bytes.push(b'\n');
    if bytes.len() as u64 > ARTIFACT_MANIFEST_LIMIT_BYTES {
        return Err(invalid_bundle(
            "portable evidence snapshot exceeds size limit".to_owned(),
        ));
    }
    Ok(bytes)
}

fn serialize_bundle_manifest(
    manifest: &PortableContinuityBundleManifest,
) -> Result<Vec<u8>, LeyCoreError> {
    let mut bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|error| invalid_bundle(format!("bundle manifest is not serializable: {error}")))?;
    bytes.push(b'\n');
    if bytes.len() as u64 > BUNDLE_MANIFEST_LIMIT_BYTES {
        return Err(invalid_bundle(
            "bundle manifest exceeds size limit".to_owned(),
        ));
    }
    Ok(bytes)
}

fn validate_manifest_shape(
    manifest: &PortableContinuityBundleManifest,
) -> Result<(), LeyCoreError> {
    if manifest.schema_version != PORTABLE_CONTINUITY_BUNDLE_VERSION {
        return Err(invalid_bundle(format!(
            "unsupported portable bundle schema {}",
            manifest.schema_version
        )));
    }
    validate_project_id(&manifest.project_id)?;
    if !is_sha256(&manifest.database_sha256) || manifest.database_bytes == 0 {
        return Err(invalid_bundle(
            "portable database metadata is invalid".to_owned(),
        ));
    }
    let mut last_snapshot: Option<&str> = None;
    for entry in &manifest.artifact_snapshots {
        if !valid_snapshot_id(&entry.artifact_snapshot_id)
            || !is_sha256(&entry.manifest_sha256)
            || entry.bytes == 0
            || last_snapshot.is_some_and(|last| last >= entry.artifact_snapshot_id.as_str())
        {
            return Err(invalid_bundle(
                "portable snapshot metadata is invalid".to_owned(),
            ));
        }
        last_snapshot = Some(&entry.artifact_snapshot_id);
    }
    let mut last_blob: Option<&str> = None;
    for entry in &manifest.evidence_blobs {
        if !is_sha256(&entry.content_hash)
            || !valid_blob_name(&entry.file_name, &entry.content_hash)
            || last_blob.is_some_and(|last| last >= entry.content_hash.as_str())
        {
            return Err(invalid_bundle(
                "portable blob metadata is invalid".to_owned(),
            ));
        }
        last_blob = Some(&entry.content_hash);
    }
    Ok(())
}

fn valid_snapshot_id(value: &str) -> bool {
    value.len() == 68
        && value.starts_with("snp_")
        && value[4..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_blob_name(file_name: &str, content_hash: &str) -> bool {
    let Some(hash) = content_hash.strip_prefix("sha256:") else {
        return false;
    };
    file_name == format!("{hash}.txt") || file_name == format!("{hash}.bin")
}

fn is_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn hash_file(path: &Path) -> Result<(String, u64), LeyCoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(invalid_bundle(format!(
            "portable bundle entry is not a regular file: {}",
            path.display()
        )));
    }
    let mut file = fs::File::open(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let read = file.read(&mut buffer).map_err(|source| LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total = total.saturating_add(read as u64);
    }
    Ok((format!("sha256:{:x}", hasher.finalize()), total))
}

fn read_regular_file(path: &Path, limit: u64) -> Result<Vec<u8>, LeyCoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > limit {
        return Err(invalid_bundle(format!(
            "portable bundle file is invalid or exceeds its limit: {}",
            path.display()
        )));
    }
    fs::read(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_directory(path: &Path, label: &str) -> Result<(), LeyCoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(invalid_bundle(format!(
            "{label} is not a regular directory"
        )));
    }
    Ok(())
}

fn ensure_destination_absent(path: &Path) -> Result<(), LeyCoreError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(invalid_bundle(format!(
            "destination already exists: {}",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn create_private_directory(path: &Path) -> Result<(), LeyCoreError> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), LeyCoreError> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    file.write_all(bytes).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    file.sync_all().map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn copy_regular_file(source: &Path, destination: &Path) -> Result<(), LeyCoreError> {
    let metadata = fs::symlink_metadata(source).map_err(|source_error| LeyCoreError::Io {
        path: source.to_path_buf(),
        source: source_error,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(invalid_bundle(format!(
            "portable database staging source is not a regular file: {}",
            source.display()
        )));
    }
    let mut input = fs::File::open(source).map_err(|source_error| LeyCoreError::Io {
        path: source.to_path_buf(),
        source: source_error,
    })?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output = options
        .open(destination)
        .map_err(|source_error| LeyCoreError::Io {
            path: destination.to_path_buf(),
            source: source_error,
        })?;
    std::io::copy(&mut input, &mut output).map_err(|source_error| LeyCoreError::Io {
        path: destination.to_path_buf(),
        source: source_error,
    })?;
    output.sync_all().map_err(|source_error| LeyCoreError::Io {
        path: destination.to_path_buf(),
        source: source_error,
    })
}

struct StagingDirectory {
    path: PathBuf,
    committed: bool,
}

struct PrivateDatabaseSnapshot {
    path: PathBuf,
}

impl PrivateDatabaseSnapshot {
    fn create(store: &ContinuityStore, project_id: &str) -> Result<Self, LeyCoreError> {
        let parent = store.path().parent().ok_or_else(|| {
            invalid_bundle("continuity database has no private parent directory".to_owned())
        })?;
        let path = parent.join(format!(
            ".ley-portable-db-{}.sqlite3",
            uuid::Uuid::new_v4().simple()
        ));
        if let Err(error) = store.export_project_database(project_id, &path) {
            cleanup_sqlite_snapshot(&path);
            return Err(error);
        }
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PrivateDatabaseSnapshot {
    fn drop(&mut self) {
        cleanup_sqlite_snapshot(&self.path);
    }
}

fn cleanup_sqlite_snapshot(path: &Path) {
    let _ = fs::remove_file(path);
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let _ = fs::remove_file(PathBuf::from(sidecar));
    }
}

impl StagingDirectory {
    fn create(destination: &Path) -> Result<Self, LeyCoreError> {
        let parent = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        validate_directory(parent, "portable bundle parent")?;
        let path = parent.join(format!(".ley-portable-{}", uuid::Uuid::new_v4().simple()));
        create_private_directory(&path)?;
        Ok(Self {
            path,
            committed: false,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn commit(mut self, destination: &Path) -> Result<(), LeyCoreError> {
        fs::rename(&self.path, destination).map_err(|source| LeyCoreError::Io {
            path: destination.to_path_buf(),
            source,
        })?;
        #[cfg(unix)]
        {
            let parent = destination
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let sync_result = fs::File::open(parent).and_then(|directory| directory.sync_all());
            if let Err(source) = sync_result {
                let cleanup = fs::remove_dir_all(destination);
                if let Err(cleanup_error) = cleanup {
                    return Err(invalid_bundle(format!(
                        "portable bundle rename was not durably synced ({source}) and rollback failed ({cleanup_error})"
                    )));
                }
                return Err(LeyCoreError::Io {
                    path: parent.to_path_buf(),
                    source,
                });
            }
        }
        self.committed = true;
        Ok(())
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

struct BundleSource {
    root: Dir,
    snapshots: Dir,
    content: Dir,
}

impl BundleSource {
    fn open(path: &Path) -> Result<Self, LeyCoreError> {
        validate_directory(path, "portable bundle source")?;
        let root = Dir::open_ambient_dir(path, ambient_authority()).map_err(|source| {
            LeyCoreError::Io {
                path: path.to_path_buf(),
                source,
            }
        })?;
        let evidence = root
            .open_dir_nofollow(EVIDENCE_DIRECTORY)
            .map_err(|source| {
                invalid_bundle(format!("cannot open evidence directory: {source}"))
            })?;
        let snapshots = evidence
            .open_dir_nofollow(SNAPSHOTS_DIRECTORY)
            .map_err(|source| {
                invalid_bundle(format!("cannot open snapshot directory: {source}"))
            })?;
        let content = evidence
            .open_dir_nofollow(CONTENT_DIRECTORY)
            .map_err(|source| invalid_bundle(format!("cannot open content directory: {source}")))?;
        Ok(Self {
            root,
            snapshots,
            content,
        })
    }

    fn read_root_file(&self, name: &str, limit: u64) -> Result<Vec<u8>, LeyCoreError> {
        read_cap_file(&self.root, name, limit)
    }

    fn copy_root_file(&self, name: &str, destination: &Path) -> Result<(), LeyCoreError> {
        copy_cap_file(&self.root, name, destination)
    }

    fn copy_snapshot(&self, snapshot_id: &str, destination: &Path) -> Result<(), LeyCoreError> {
        copy_cap_file(&self.snapshots, &format!("{snapshot_id}.json"), destination)
    }

    fn copy_blob(&self, file_name: &str, destination: &Path) -> Result<(), LeyCoreError> {
        copy_cap_file(&self.content, file_name, destination)
    }
}

fn read_cap_file(directory: &Dir, name: &str, limit: u64) -> Result<Vec<u8>, LeyCoreError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = directory
        .open_with(name, &options)
        .map_err(|source| invalid_bundle(format!("cannot open {name}: {source}")))?;
    let metadata = file
        .metadata()
        .map_err(|source| invalid_bundle(format!("cannot inspect {name}: {source}")))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(invalid_bundle(format!(
            "bundle file {name} is invalid or too large"
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| invalid_bundle(format!("cannot read {name}: {source}")))?;
    if bytes.len() as u64 > limit {
        return Err(invalid_bundle(format!(
            "bundle file {name} exceeds its limit"
        )));
    }
    Ok(bytes)
}

fn copy_cap_file(directory: &Dir, name: &str, destination: &Path) -> Result<(), LeyCoreError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut source = directory
        .open_with(name, &options)
        .map_err(|error| invalid_bundle(format!("cannot open {name}: {error}")))?;
    if !source
        .metadata()
        .map_err(|error| invalid_bundle(format!("cannot inspect {name}: {error}")))?
        .is_file()
    {
        return Err(invalid_bundle(format!(
            "bundle entry {name} is not a regular file"
        )));
    }
    let mut destination_options = fs::OpenOptions::new();
    destination_options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        destination_options.mode(0o600);
    }
    let mut target = destination_options
        .open(destination)
        .map_err(|source| LeyCoreError::Io {
            path: destination.to_path_buf(),
            source,
        })?;
    std::io::copy(&mut source, &mut target).map_err(|source| LeyCoreError::Io {
        path: destination.to_path_buf(),
        source,
    })?;
    target.sync_all().map_err(|source| LeyCoreError::Io {
        path: destination.to_path_buf(),
        source,
    })
}

fn invalid_bundle(message: String) -> LeyCoreError {
    LeyCoreError::InvalidPortableContinuityBundle(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuity_store::{
        ContinuityApprovedSourceSnapshotInput, ContinuityProjectObservation,
    };
    use crate::{
        checkpoint_session, checkpoint_session_with_continuity_transition,
        import_legacy_continuity, ingest_project, ingest_project_with_continuity_transition,
        initialize_project, start_session, start_session_with_continuity_transition,
        AgentEgressPolicy, CaptureMode, CheckpointInput, ContinuityEventInput, ProjectIdentity,
        SessionSource, SessionSourceKind, StartSessionInput, VerificationInput, VerificationStatus,
        CONTINUITY_SCHEMA_VERSION, PROJECT_SCHEMA_VERSION,
    };
    use rusqlite::Connection;
    use serde_json::json;
    use std::process::Command;
    use tempfile::tempdir;

    fn git(project: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(project)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
    }

    #[test]
    fn native_artifact_cas_exports_cited_evidence_after_legacy_vault_loss() {
        let base = tempdir().unwrap();
        let project = base.path().join("native-project");
        let vault = base.path().join("native-vault");
        let private = base.path().join("native-private");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::create_dir(&private).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        }
        initialize_project(
            &project,
            Some("Native portable continuity"),
            CaptureMode::Structured,
        )
        .unwrap();
        let readme = b"# Native portable evidence\nCAS survives vault removal\n";
        fs::write(project.join("README.md"), readme).unwrap();
        let store = ContinuityStore::at(private.join("continuity.sqlite3"));
        ingest_project(&project, &vault).unwrap();
        ingest_project_with_continuity_transition(&project, &vault, &store).unwrap();
        let started = start_session_with_continuity_transition(
            &project,
            &vault,
            &store,
            StartSessionInput {
                request_id: request_id('a'),
                name: "Native portable export".to_owned(),
                goal: "Keep cited evidence portable without the legacy vault".to_owned(),
                source: SessionSource {
                    kind: SessionSourceKind::HostHook,
                    host: Some("codex".to_owned()),
                    agent: Some("gpt-6-luna".to_owned()),
                    source_reference: None,
                },
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session_with_continuity_transition(
            &project,
            &vault,
            &store,
            &started.session.session_id,
            CheckpointInput {
                request_id: request_id('b'),
                summary: "Verified native portable evidence".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: vec![VerificationInput {
                    kind: "command".to_owned(),
                    status: VerificationStatus::Passed,
                    summary: "README evidence retained natively".to_owned(),
                    command: Some("cat README.md".to_owned()),
                    evidence_artifact_paths: vec!["README.md".to_owned()],
                }],
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let citation =
            checkpoint.session.checkpoints[0].verification[0].evidence_artifacts[0].clone();
        let project_id = checkpoint.session.project_id.clone();

        fs::remove_dir_all(&vault).unwrap();
        let bundle = base.path().join("native-portable-bundle");
        let manifest = export_portable_continuity(&store, &vault, &project_id, &bundle).unwrap();
        assert_eq!(manifest.artifact_snapshots.len(), 1);
        assert_eq!(manifest.evidence_blobs.len(), 1);
        assert_eq!(
            read_portable_cited_evidence(
                &bundle,
                &project_id,
                &citation.artifact_snapshot_id,
                &citation.artifact_path,
                &citation.content_hash,
            )
            .unwrap(),
            readme
        );
        let restored = base.path().join("native-restored");
        let imported = import_portable_continuity(&bundle, &restored).unwrap();
        assert_eq!(
            read_continuity_evidence(
                &imported.evidence_root,
                &project_id,
                &citation.artifact_snapshot_id,
                &citation.artifact_path,
                &citation.content_hash,
            )
            .unwrap(),
            readme
        );
    }

    #[test]
    fn portable_round_trip_is_project_scoped_and_preserves_cited_evidence() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        let private = base.path().join("private");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::create_dir(&private).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        }

        git(&project, &["init", "-b", "main"]);
        git(&project, &["config", "user.name", "Ley Test"]);
        git(&project, &["config", "user.email", "ley@example.invalid"]);
        initialize_project(
            &project,
            Some("Portable continuity"),
            CaptureMode::Structured,
        )
        .unwrap();
        let readme = b"# Portable evidence\nvalidated source bytes\n";
        fs::write(project.join("README.md"), readme).unwrap();
        fs::write(
            project.join("UNRELATED.md"),
            "uncited project metadata must not cross the portable boundary\n",
        )
        .unwrap();
        git(&project, &["add", "."]);
        git(&project, &["commit", "-m", "fixture"]);
        ingest_project(&project, &vault).unwrap();

        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('1'),
                name: "Portable export".to_owned(),
                goal: "Keep cited evidence portable".to_owned(),
                source: SessionSource {
                    kind: SessionSourceKind::HostHook,
                    host: Some("codex".to_owned()),
                    agent: Some("gpt-6-luna".to_owned()),
                    source_reference: None,
                },
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &started.session.session_id,
            CheckpointInput {
                request_id: request_id('2'),
                summary: "Verified portable evidence".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: vec![VerificationInput {
                    kind: "command".to_owned(),
                    status: VerificationStatus::Passed,
                    summary: "README evidence retained".to_owned(),
                    command: Some("cat README.md".to_owned()),
                    evidence_artifact_paths: vec!["README.md".to_owned()],
                }],
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let citation =
            checkpoint.session.checkpoints[0].verification[0].evidence_artifacts[0].clone();

        let store = ContinuityStore::at(private.join("continuity.sqlite3"));
        let imported = import_legacy_continuity(&project, &vault, &store).unwrap();
        store
            .set_project_egress_policy(&imported.project_id, AgentEgressPolicy::NeverSend)
            .unwrap();
        let snapshot_bytes =
            b"# Portable approved intent\nlegacy snapshot survives backup\n".to_vec();
        let snapshot_hash = format!("sha256:{:x}", Sha256::digest(&snapshot_bytes));
        store
            .import_legacy_approved_source_authority(
                &imported.project_id,
                &[ContinuityApprovedSourceSnapshotInput {
                    source_id: "spec_88888888888888888888888888888888".to_owned(),
                    display_name: "Legacy/PortableIntent.md".to_owned(),
                    content_hash: snapshot_hash.clone(),
                    approved_at_unix_ms: 124,
                    content_bytes: snapshot_bytes.clone(),
                }],
                &[],
            )
            .unwrap();
        store
            .approve_project_file_source(
                &imported.project_id,
                "spec_99999999999999999999999999999999",
                "AGENTS.md",
                "AGENTS.md",
                &format!("sha256:{:x}", Sha256::digest(b"# portable project file\n")),
                125,
            )
            .unwrap();
        store
            .sync_legacy_project_observations(&[ContinuityProjectObservation {
                project_id: imported.project_id.clone(),
                root_path: project.canonicalize().unwrap(),
                last_opened_at_unix_ms: 123,
                continuity_origin: crate::continuity_store::ContinuityProjectOrigin::LegacyUnknown,
            }])
            .unwrap();
        let other = ProjectIdentity {
            schema_version: PROJECT_SCHEMA_VERSION,
            project_id: format!("prj_{}", "9".repeat(32)),
            name: "Other private project".to_owned(),
            created_at_unix_ms: 1,
        };
        store.register_project(&other).unwrap();
        store
            .set_project_egress_policy(&other.project_id, AgentEgressPolicy::LocalModelOnly)
            .unwrap();
        store
            .append_event(&ContinuityEventInput {
                event_id: format!("evt_{}", "9".repeat(64)),
                project_id: other.project_id.clone(),
                subject_id: None,
                session_id: None,
                session_sequence: None,
                request_id: None,
                request_fingerprint: None,
                kind: "other-project-secret".to_owned(),
                payload_version: 1,
                recorded_at_unix_ms: 2,
                revision_head: None,
                revision_branch: None,
                payload: json!({"secret":"must not export"}),
            })
            .unwrap();

        let bundle = base.path().join("portable-bundle");
        let manifest =
            export_portable_continuity(&store, &vault, &imported.project_id, &bundle).unwrap();
        assert_eq!(manifest.project_id, imported.project_id);
        assert_eq!(manifest.event_count, imported.continuity_events_created);
        assert_eq!(manifest.artifact_snapshots.len(), 1);
        assert_eq!(manifest.evidence_blobs.len(), 1);
        let portable_snapshot_bytes = fs::read(
            bundle
                .join(EVIDENCE_DIRECTORY)
                .join(SNAPSHOTS_DIRECTORY)
                .join(format!(
                    "{}.json",
                    manifest.artifact_snapshots[0].artifact_snapshot_id
                )),
        )
        .unwrap();
        let portable_snapshot: PortableEvidenceSnapshotFile =
            serde_json::from_slice(&portable_snapshot_bytes).unwrap();
        assert_eq!(portable_snapshot.artifacts.len(), 1);
        assert_eq!(portable_snapshot.artifacts[0].artifact_path, "README.md");
        assert!(!String::from_utf8(portable_snapshot_bytes)
            .unwrap()
            .contains("UNRELATED.md"));
        assert!(!bundle.join("continuity.sqlite3-wal").exists());
        assert!(!bundle.join("continuity.sqlite3-shm").exists());
        assert!(!bundle.join("continuity.sqlite3-journal").exists());
        let exported_events = project_events_from_portable_database(
            &bundle.join(BUNDLE_DATABASE_FILE),
            &imported.project_id,
        )
        .unwrap();
        assert!(exported_events
            .iter()
            .all(|event| event.project_id == imported.project_id));
        assert_eq!(exported_events.len(), manifest.event_count);
        assert_eq!(
            read_portable_cited_evidence(
                &bundle,
                &imported.project_id,
                &citation.artifact_snapshot_id,
                &citation.artifact_path,
                &citation.content_hash,
            )
            .unwrap(),
            readme
        );

        let restored = base.path().join("restored");
        let result = import_portable_continuity(&bundle, &restored).unwrap();
        assert!(!restored.join(BUNDLE_MANIFEST_FILE).exists());
        fs::remove_dir_all(&vault).unwrap();
        let restored_store = ContinuityStore::at(&result.database_path);
        assert!(restored_store.project_observations().unwrap().is_empty());
        assert_eq!(
            restored_store
                .project_egress_policy(&result.project_id)
                .unwrap(),
            AgentEgressPolicy::NeverSend
        );
        assert!(!restored_store
            .import_legacy_project_egress_policy(&result.project_id, AgentEgressPolicy::NeverSend)
            .unwrap());
        assert!(!restored_store
            .import_legacy_approved_source_authority(&result.project_id, &[], &[])
            .unwrap());
        let restored_snapshot = restored_store
            .approved_source(&result.project_id, "spec_88888888888888888888888888888888")
            .unwrap()
            .unwrap();
        assert_eq!(restored_snapshot.content_hash, snapshot_hash);
        assert_eq!(
            restored_store
                .approved_snapshot_bytes(
                    &result.project_id,
                    "spec_88888888888888888888888888888888",
                )
                .unwrap()
                .unwrap(),
            snapshot_bytes
        );
        let restored_project_file = restored_store
            .approved_source(&result.project_id, "spec_99999999999999999999999999999999")
            .unwrap()
            .unwrap();
        assert_eq!(
            restored_project_file.source_kind,
            crate::continuity_store::ContinuityApprovedSourceKind::ProjectFile
        );
        assert_eq!(
            restored_project_file.project_relative_path.as_deref(),
            Some("AGENTS.md")
        );
        assert!(restored_store
            .approved_snapshot_bytes(&result.project_id, "spec_99999999999999999999999999999999",)
            .unwrap()
            .is_none());
        assert_eq!(
            restored_store
                .events_for_session(&result.project_id, &started.session.session_id)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            read_continuity_evidence(
                &result.evidence_root,
                &result.project_id,
                &citation.artifact_snapshot_id,
                &citation.artifact_path,
                &citation.content_hash,
            )
            .unwrap(),
            readme
        );

        let native_after_cutover = ContinuityEventInput {
            event_id: format!("evt_{}", "8".repeat(64)),
            project_id: result.project_id.clone(),
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "post-cutover-checkpoint".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 9_999,
            revision_head: None,
            revision_branch: None,
            payload: json!({"summary":"native runtime changed after import"}),
        };
        assert!(
            restored_store
                .append_event(&native_after_cutover)
                .unwrap()
                .created
        );
        let reexported = base.path().join("reexported-bundle");
        let reexport_manifest = export_portable_continuity_from_evidence_root(
            &restored_store,
            &result.evidence_root,
            &result.project_id,
            &reexported,
        )
        .unwrap();
        assert_eq!(reexport_manifest.event_count, manifest.event_count + 1);
        assert_eq!(reexport_manifest.artifact_snapshots.len(), 1);
        assert_eq!(reexport_manifest.evidence_blobs.len(), 1);
        assert_eq!(
            read_portable_cited_evidence(
                &reexported,
                &result.project_id,
                &citation.artifact_snapshot_id,
                &citation.artifact_path,
                &citation.content_hash,
            )
            .unwrap(),
            readme
        );

        let bundle_database = bundle.join(BUNDLE_DATABASE_FILE);
        let connection = Connection::open(&bundle_database).unwrap();
        connection
            .execute_batch(
                "DROP TABLE artifact_write_authority;
                 DROP TABLE project_artifact_state;
                 DROP TABLE artifact_files;
                 DROP TABLE artifact_snapshots;
                 DROP TABLE event_source_version_links;
                 DROP TABLE source_locators;
                 DROP TABLE source_versions;
                 DROP TABLE project_sources;
                 DROP TABLE working_copy_locators;
                 DROP TABLE project_repositories;
                 DROP TABLE project_sessions;
                 DROP TRIGGER projects_reject_terminal_lifecycle;
                 DROP TABLE project_lifecycle;
                 DROP TABLE local_migration_state;
                 DROP TABLE approved_sources;
                 DROP TABLE legacy_approved_source_issues;
                 DROP TABLE approved_source_blobs;
                 ALTER TABLE projects DROP COLUMN approved_source_authority_migrated;
                 DROP TABLE project_observations;
                 ALTER TABLE projects DROP COLUMN agent_egress_policy_migrated;
                 ALTER TABLE projects DROP COLUMN agent_egress_policy;
                 PRAGMA user_version = 2;",
            )
            .unwrap();
        drop(connection);
        let mut v2_manifest = manifest.clone();
        let (database_sha256, database_bytes) = hash_file(&bundle_database).unwrap();
        v2_manifest.database_sha256 = database_sha256;
        v2_manifest.database_bytes = database_bytes;
        fs::write(
            bundle.join(BUNDLE_MANIFEST_FILE),
            serialize_bundle_manifest(&v2_manifest).unwrap(),
        )
        .unwrap();
        let v2_restored = base.path().join("v2-restored");
        let v2_result = import_portable_continuity(&bundle, &v2_restored).unwrap();
        let v2_store = ContinuityStore::at(&v2_result.database_path);
        assert_eq!(
            v2_store.schema_version().unwrap(),
            CONTINUITY_SCHEMA_VERSION
        );
        assert_eq!(
            v2_store
                .project_egress_policy(&v2_result.project_id)
                .unwrap(),
            AgentEgressPolicy::AgentOk
        );
        assert!(v2_store
            .import_legacy_project_egress_policy(&v2_result.project_id, AgentEgressPolicy::AgentOk)
            .unwrap());

        let blob = &manifest.evidence_blobs[0];
        fs::write(
            bundle
                .join(EVIDENCE_DIRECTORY)
                .join(CONTENT_DIRECTORY)
                .join(&blob.file_name),
            b"corrupted evidence",
        )
        .unwrap();
        let rejected = base.path().join("rejected-import");
        assert!(matches!(
            import_portable_continuity(&bundle, &rejected),
            Err(LeyCoreError::InvalidPortableContinuityBundle(_))
                | Err(LeyCoreError::InvalidArtifactStore(_))
        ));
        assert!(!rejected.exists());
    }
}
