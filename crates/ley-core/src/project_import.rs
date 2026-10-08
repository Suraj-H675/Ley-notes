use crate::bounded_reader::read_scoped_file;
use crate::continuity_store::append_project_brain_event_on;
use crate::project_brain::{
    bump_project_generation_on, invalid_brain, read_source_version_on, read_working_copy_on,
    request_fingerprint, require_active_project_on, require_project_generation_on, unix_time_ms,
    validate_request_id,
};
use crate::{
    ContinuityEventInput, ContinuityStore, LeyCoreError, ProjectHandle, RetainedSourceVersion,
    SourceVersionInput, WorkingCopyState,
};
use cap_fs_ext::DirExt;
use cap_std::{ambient_authority, fs::Dir};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub const MAX_IMPORT_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_IMPORT_ENTRIES: usize = 20_000;
const MAX_IGNORE_BYTES: u64 = 64 * 1024;
const MAX_IGNORE_TOTAL: u64 = 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_PATH_BYTES: usize = 4096;
const MAX_FILE_BYTES: u64 = crate::MAX_PROJECT_BRAIN_SOURCE_BYTES as u64;
pub(crate) const IMPORT_SCHEMA: &str = r#"
ALTER TABLE working_copy_locators ADD COLUMN root_identity TEXT;
CREATE TABLE project_import_attempts (
 project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
 scan_id TEXT NOT NULL, locator_id TEXT NOT NULL, request_id TEXT NOT NULL,
 fingerprint TEXT NOT NULL, started_at INTEGER NOT NULL, completed_at INTEGER,
 failure_code TEXT,
 status TEXT NOT NULL CHECK(status IN ('running','published','failed','scrubbed')),
 receipt_json TEXT CHECK(receipt_json IS NULL OR json_valid(receipt_json)),
 PRIMARY KEY(project_id,scan_id), UNIQUE(project_id,request_id), UNIQUE(project_id,locator_id,scan_id),
 FOREIGN KEY(project_id,locator_id) REFERENCES working_copy_locators(project_id,locator_id) ON DELETE CASCADE
) STRICT;
CREATE TABLE working_copy_import_heads (
 project_id TEXT NOT NULL, locator_id TEXT NOT NULL, scan_id TEXT NOT NULL,
 PRIMARY KEY(project_id,locator_id),
 FOREIGN KEY(project_id,locator_id) REFERENCES working_copy_locators(project_id,locator_id) ON DELETE CASCADE,
 FOREIGN KEY(project_id,locator_id,scan_id) REFERENCES project_import_attempts(project_id,locator_id,scan_id) ON DELETE CASCADE
) STRICT;
CREATE TABLE working_copy_inventory (
 project_id TEXT NOT NULL, locator_id TEXT NOT NULL, relative_path TEXT NOT NULL,
 source_id TEXT, source_version_id TEXT, original_hash TEXT, entry_json TEXT NOT NULL CHECK(json_valid(entry_json)),
 PRIMARY KEY(project_id,locator_id,relative_path),
 FOREIGN KEY(project_id,locator_id) REFERENCES working_copy_locators(project_id,locator_id) ON DELETE CASCADE,
 FOREIGN KEY(project_id,source_id) REFERENCES project_sources(project_id,source_id) ON DELETE CASCADE,
 FOREIGN KEY(project_id,source_id,source_version_id) REFERENCES source_versions(project_id,source_id,source_version_id) ON DELETE CASCADE
) STRICT;
CREATE TABLE import_source_paths (
 project_id TEXT NOT NULL, locator_id TEXT NOT NULL, relative_path TEXT NOT NULL, source_id TEXT NOT NULL,
 PRIMARY KEY(project_id,locator_id,relative_path,source_id),
 FOREIGN KEY(project_id,source_id) REFERENCES project_sources(project_id,source_id) ON DELETE CASCADE,
 FOREIGN KEY(project_id,locator_id) REFERENCES working_copy_locators(project_id,locator_id) ON DELETE CASCADE
) STRICT;
CREATE TABLE import_erasure_fences (
 project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE, path_digest TEXT NOT NULL,
 PRIMARY KEY(project_id,path_digest)
) STRICT;
PRAGMA user_version = 13;
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalImportInput {
    pub locator_id: String,
    pub selected_root: PathBuf,
    pub request_id: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InventoryState {
    Retained,
    Omitted,
    Missing,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentsEntry {
    pub relative_path: String,
    pub state: InventoryState,
    pub source_id: Option<String>,
    pub source_version_id: Option<String>,
    pub occurrence_event_id: Option<String>,
    pub observed_bytes: Option<u64>,
    pub omission_reason: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReceipt {
    pub project_id: String,
    pub locator_id: String,
    pub scan_id: String,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub project_generation: u64,
    pub retained_count: usize,
    pub omitted_count: usize,
    pub deleted: Vec<String>,
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub renamed: Vec<Value>,
    pub git: Value,
    pub observation_digest: String,
    pub policy_digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAttempt {
    pub scan_id: String,
    pub status: String,
    pub failure_code: Option<String>,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectContents {
    pub project_id: String,
    pub locator_id: String,
    pub last_success: Option<ImportReceipt>,
    pub observation_status: String,
    pub current_generation: u64,
    pub locator_state: WorkingCopyState,
    pub latest_attempt: Option<ImportAttempt>,
    pub entries: Vec<ContentsEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceVersionMetadata {
    pub occurrences: Vec<Value>,
    pub occurrence_count: u64,
    pub version: RetainedSourceVersion,
    pub transformation: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEvidence {
    pub occurrences: Vec<Value>,
    pub occurrence_count: u64,
    pub version: RetainedSourceVersion,
    pub transformation: Value,
    pub bytes: Vec<u8>,
}
struct Scanned {
    entry: ContentsEntry,
    original: Option<String>,
    bytes: Vec<u8>,
    transformation: Value,
}
struct Scan {
    entries: BTreeMap<String, Scanned>,
    git: Value,
    ignore_hashes: BTreeMap<String, String>,
}
#[derive(Default)]
struct ScanExclusions {
    erased: BTreeSet<String>,
    removed: BTreeSet<String>,
}
impl ScanExclusions {
    fn reason(&self, locator: &str, path: &str) -> Option<&'static str> {
        let digest = fence(locator, path);
        if self.erased.contains(&digest) {
            Some("erased-source")
        } else if self.removed.contains(&digest) {
            Some("removed-source")
        } else {
            None
        }
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn fence(locator: &str, path: &str) -> String {
    hash(&serde_json::to_vec(&(locator, path)).expect("tuple serializable"))
}
fn io(path: &Path, source: std::io::Error) -> LeyCoreError {
    LeyCoreError::Io {
        path: path.to_owned(),
        source,
    }
}
pub(crate) fn safe_root(path: &Path) -> Result<PathBuf, LeyCoreError> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir().map_err(|e| io(path, e))?.join(path)
    };
    let mut normalized = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(part),
        }
    }
    open_root_nofollow(&normalized)?;
    Ok(normalized)
}
pub(crate) fn root_stamp(root: &Path) -> Result<String, LeyCoreError> {
    directory_stamp(&open_root_nofollow(root)?, root)
}
fn open_root_nofollow(root: &Path) -> Result<Dir, LeyCoreError> {
    let mut anchor = PathBuf::new();
    let mut components = root.components().peekable();
    while matches!(
        components.peek(),
        Some(Component::Prefix(_) | Component::RootDir)
    ) {
        anchor.push(components.next().unwrap());
    }
    if !anchor.is_absolute() {
        return Err(LeyCoreError::UnsafeProjectLayout(root.to_owned()));
    }
    let mut dir = Dir::open_ambient_dir(&anchor, ambient_authority()).map_err(|e| io(root, e))?;
    for component in components {
        let Component::Normal(name) = component else {
            return Err(LeyCoreError::UnsafeProjectLayout(root.to_owned()));
        };
        dir = dir.open_dir_nofollow(name).map_err(|e| io(root, e))?;
    }
    Ok(dir)
}
fn open_authorized_root(root: &Path, expected: &str) -> Result<Dir, LeyCoreError> {
    let dir = open_root_nofollow(root)?;
    if directory_stamp(&dir, root)? != expected {
        return Err(invalid_brain("authorized root directory was replaced"));
    }
    Ok(dir)
}

pub(crate) fn read_attached_source_nofollow(
    root: &Path,
    expected_root_stamp: &str,
    relative: &Path,
    limit: u64,
) -> Result<Option<Vec<u8>>, LeyCoreError> {
    let root_dir = open_authorized_root(root, expected_root_stamp)?;
    if relative.is_absolute() {
        return Err(LeyCoreError::UnsafeProjectLayout(relative.to_owned()));
    }
    let components = relative.components().collect::<Vec<_>>();
    if components.is_empty()
        || components
            .iter()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(LeyCoreError::UnsafeProjectLayout(relative.to_owned()));
    }
    let mut parent = root_dir
        .try_clone()
        .map_err(|source| io(relative, source))?;
    for component in &components[..components.len() - 1] {
        let Component::Normal(name) = component else {
            return Err(LeyCoreError::UnsafeProjectLayout(relative.to_owned()));
        };
        parent = parent
            .open_dir_nofollow(name)
            .map_err(|source| io(relative, source))?;
    }
    let Component::Normal(file_name) = components[components.len() - 1] else {
        return Err(LeyCoreError::UnsafeProjectLayout(relative.to_owned()));
    };
    let metadata = match parent.symlink_metadata(file_name) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(io(relative, source)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LeyCoreError::UnsafeProjectLayout(relative.to_owned()));
    }
    if metadata.len() > limit {
        return Ok(None);
    }
    let bytes =
        crate::bounded_reader::read_scoped_file(&root_dir, relative, metadata.len(), limit)?;
    Ok(Some(bytes))
}

fn directory_stamp(dir: &Dir, root: &Path) -> Result<String, LeyCoreError> {
    let metadata = dir.dir_metadata().map_err(|e| io(root, e))?;
    #[cfg(unix)]
    {
        use cap_std::fs::MetadataExt;
        Ok(format!("{}:{}", metadata.dev(), metadata.ino()))
    }
    #[cfg(windows)]
    {
        use cap_std::fs::MetadataExt;
        match (metadata.volume_serial_number(), metadata.file_index()) {
            (Some(volume), Some(index)) => Ok(format!("{volume}:{index}")),
            _ => Err(invalid_brain("directory identity unavailable")),
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(invalid_brain("directory identity unavailable"))
    }
}
impl ContinuityStore {
    pub fn import_local_project(
        &self,
        handle: &ProjectHandle,
        input: &LocalImportInput,
    ) -> Result<ImportReceipt, LeyCoreError> {
        validate_request_id(&input.request_id)?;
        let root = safe_root(&input.selected_root)?;
        self.reject_private_import_root(&root)?;
        let stamp;
        let fingerprint = request_fingerprint(
            &json!({"locator":input.locator_id,"root":root,"generation":handle.generation,"policy":1}),
        )?;
        let started = unix_time_ms()?;
        let scan_id = format!("imp_{}", Uuid::new_v4().simple());
        let base;
        let exclusions;
        {
            let mut conn = self.open_connection()?;
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| self.database_error(e))?;
            require_project_generation_on(&tx, handle, self)?;
            stamp = self.require_import_locator(&tx, handle, input, &root)?;
            if let Some((stored, status, receipt)) = tx
                .query_row(
                    "SELECT fingerprint,status,receipt_json
                 FROM project_import_attempts
                 WHERE project_id=?1 AND request_id=?2",
                    params![handle.project_id, input.request_id,],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, Option<String>>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| self.database_error(e))?
            {
                if stored != fingerprint {
                    return Err(invalid_brain(
                        "import request was reused with different scope",
                    ));
                }
                if status != "published" {
                    return Err(invalid_brain(
                        "import request did not publish; use a fresh request ID",
                    ));
                }
                return serde_json::from_str(
                    &receipt.ok_or_else(|| invalid_brain("missing import receipt"))?,
                )
                .map_err(|_| invalid_brain("invalid import receipt"));
            }
            base = import_head(&tx, &handle.project_id, &input.locator_id, self)?;
            let mut fences = tx
                .prepare("SELECT path_digest FROM import_erasure_fences WHERE project_id=?1")
                .map_err(|e| self.database_error(e))?;
            let erased = fences
                .query_map([&handle.project_id], |row| row.get::<_, String>(0))
                .map_err(|e| self.database_error(e))?
                .collect::<Result<BTreeSet<_>, _>>()
                .map_err(|e| self.database_error(e))?;
            drop(fences);
            let mut paths = tx
                .prepare(
                    "SELECT p.relative_path FROM import_source_paths p JOIN project_sources s
                 ON s.project_id=p.project_id AND s.source_id=p.source_id
                 WHERE p.project_id=?1 AND p.locator_id=?2 AND s.state='removed'",
                )
                .map_err(|e| self.database_error(e))?;
            let removed = paths
                .query_map(params![handle.project_id, input.locator_id], |r| {
                    r.get::<_, String>(0)
                })
                .map_err(|e| self.database_error(e))?
                .map(|path| path.map(|path| fence(&input.locator_id, &path)))
                .collect::<Result<BTreeSet<_>, _>>()
                .map_err(|e| self.database_error(e))?;
            drop(paths);
            exclusions = ScanExclusions { erased, removed };

            tx.execute(
                "INSERT INTO project_import_attempts(project_id,scan_id,locator_id,request_id,fingerprint,started_at,status)
                 VALUES(?1,?2,?3,?4,?5,?6,'running')",
                params![
                    handle.project_id,
                    scan_id,
                    input.locator_id,
                    input.request_id,
                    fingerprint,
                    started as i64,
                ],
            )
            .map_err(|e| self.database_error(e))?;
            tx.commit().map_err(|e| self.database_error(e))?;
        }
        let result = (|| {
            let dir = open_authorized_root(&root, &stamp)?;
            let scan = collect(&root, &dir, &input.locator_id, &exclusions)?;
            let check = collect(&root, &dir, &input.locator_id, &exclusions)?;
            if scan_signature(&scan) != scan_signature(&check)
                || root_stamp(&root)? != stamp
                || safe_root(&input.selected_root)? != root
            {
                return Err(invalid_brain("project changed during import"));
            }
            self.with_artifact_authority_lock(|| {
                self.garbage_collect_artifact_state_under_lock(&handle.project_id)?;
                let publication = (|| {
                    let mut conn = self.open_connection()?;
                    let tx = conn
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(|e| self.database_error(e))?;
                    require_project_generation_on(&tx, handle, self)?;
                    if self.require_import_locator(&tx, handle, input, &root)? != stamp {
                        return Err(invalid_brain("authorized root identity changed"));
                    }
                    if import_head(&tx, &handle.project_id, &input.locator_id, self)? != base
                        || root_stamp(&root)? != stamp
                    {
                        return Err(invalid_brain("import base observation changed"));
                    }
                    let receipt =
                        self.publish_import_on(&tx, handle, input, &scan_id, started, scan)?;
                    tx.commit().map_err(|e| self.database_error(e))?;
                    Ok(receipt)
                })();
                if publication.is_err() {
                    self.garbage_collect_artifact_state_under_lock(&handle.project_id)?;
                }
                publication
            })
        })();
        if result.is_err() {
            let conn = self.open_connection()?;
            conn.execute(
                "UPDATE project_import_attempts
                 SET status='failed',failure_code='import-failed',completed_at=?1
                 WHERE project_id=?2 AND scan_id=?3 AND status='running'",
                params![unix_time_ms()? as i64, handle.project_id, scan_id,],
            )
            .map_err(|e| self.database_error(e))?;
        }
        result
    }
    fn reject_private_import_root(&self, root: &Path) -> Result<(), LeyCoreError> {
        let parent = self
            .path()
            .parent()
            .ok_or_else(|| invalid_brain("private continuity store has no directory"))?;
        let private = parent.canonicalize().map_err(|e| io(parent, e))?;
        if root.starts_with(&private) || private.starts_with(root) {
            return Err(invalid_brain(
                "selected root overlaps private ContinuityStore storage",
            ));
        }
        Ok(())
    }
    fn require_import_locator(
        &self,
        conn: &rusqlite::Connection,
        handle: &ProjectHandle,
        input: &LocalImportInput,
        root: &Path,
    ) -> Result<String, LeyCoreError> {
        let locator = read_working_copy_on(conn, &handle.project_id, &input.locator_id, self)?
            .ok_or_else(|| invalid_brain("working copy not authorized"))?;
        let expected: Option<String> = conn
            .query_row(
                "SELECT root_identity
             FROM working_copy_locators
             WHERE project_id=?1 AND locator_id=?2",
                params![handle.project_id, input.locator_id,],
                |r| r.get(0),
            )
            .map_err(|e| self.database_error(e))?;
        let expected = expected.ok_or_else(|| {
            invalid_brain("working-copy identity unavailable; revoke and explicitly attach again")
        })?;
        if root_stamp(root)? != expected {
            return Err(invalid_brain("authorized root directory was replaced"));
        }
        if locator.state != WorkingCopyState::Authorized || locator.local_path != root {
            return Err(invalid_brain(
                "selected root does not match authorized working-copy locator",
            ));
        }
        Ok(expected)
    }
    pub fn project_contents(
        &self,
        project: &str,
        locator: &str,
    ) -> Result<ProjectContents, LeyCoreError> {
        crate::validate_project_id(project)?;
        let mut connection = self.open_connection()?;
        let conn = connection
            .transaction()
            .map_err(|e| self.database_error(e))?;
        let lifecycle = require_active_project_on(&conn, project, self)?;
        let locator_record = read_working_copy_on(&conn, project, locator, self)?
            .ok_or_else(|| invalid_brain("unknown working-copy locator"))?;
        let head = import_head(&conn, project, locator, self)?;
        let last_success: Option<ImportReceipt> = if let Some(head) = head {
            conn.query_row(
                "SELECT receipt_json
                 FROM project_import_attempts
                 WHERE project_id=?1 AND scan_id=?2",
                params![project, head,],
                |r| r.get::<_, Option<String>>(0),
            )
            .map_err(|e| self.database_error(e))?
            .map(|s| serde_json::from_str(&s).map_err(|_| invalid_brain("invalid import receipt")))
            .transpose()?
        } else {
            None
        };
        let latest_attempt = conn
            .query_row(
                "SELECT scan_id,status,started_at,completed_at,failure_code
             FROM project_import_attempts
             WHERE project_id=?1 AND locator_id=?2
             ORDER BY started_at DESC,rowid DESC LIMIT 1",
                params![project, locator,],
                |r| {
                    Ok(ImportAttempt {
                        scan_id: r.get(0)?,
                        status: r.get(1)?,
                        started_at_unix_ms: r.get::<_, i64>(2)? as u64,
                        completed_at_unix_ms: r.get::<_, Option<i64>>(3)?.map(|n| n as u64),
                        failure_code: r.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|e| self.database_error(e))?;
        let observation_status = if locator_record.state != WorkingCopyState::Authorized {
            "revoked"
        } else if last_success
            .as_ref()
            .is_some_and(|r| r.project_generation == lifecycle.generation)
        {
            "last-successful-observation"
        } else {
            "unobserved-or-invalidated"
        }
        .to_owned();
        Ok(ProjectContents {
            project_id: project.to_owned(),
            locator_id: locator.to_owned(),
            observation_status,
            current_generation: lifecycle.generation,
            locator_state: locator_record.state,
            last_success,
            latest_attempt,
            entries: inventory(&conn, project, locator, self)?
                .into_values()
                .map(|(e, _)| e)
                .collect(),
        })
    }
    pub fn source_versions(
        &self,
        project: &str,
        source: &str,
    ) -> Result<Vec<SourceVersionMetadata>, LeyCoreError> {
        self.open_project_brain(project)?;
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT source_version_id
             FROM source_versions
             WHERE project_id=?1 AND source_id=?2
             ORDER BY retained_at_unix_ms,source_version_id",
            )
            .map_err(|e| self.database_error(e))?;
        let ids = stmt
            .query_map(params![project, source,], |r| r.get::<_, String>(0))
            .map_err(|e| self.database_error(e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| self.database_error(e))?;
        ids.iter()
            .map(|id| {
                let version = read_source_version_on(&conn, project, source, id, self)?
                    .ok_or_else(|| invalid_brain("version disappeared"))?;
                let text: String = conn
                    .query_row(
                        "SELECT transformation_json
        FROM source_versions
        WHERE project_id=?1 AND source_id=?2 AND source_version_id=?3",
                        params![project, source, id,],
                        |r| r.get(0),
                    )
                    .map_err(|e| self.database_error(e))?;
                let (occurrences, occurrence_count) =
                    source_occurrences(&conn, project, source, id, self)?;
                Ok(SourceVersionMetadata {
                    occurrences,
                    occurrence_count,
                    version,
                    transformation: serde_json::from_str(&text)
                        .map_err(|_| invalid_brain("invalid transformation"))?,
                })
            })
            .collect()
    }
    pub fn source_evidence(
        &self,
        project: &str,
        source: &str,
        version: &str,
    ) -> Result<SourceEvidence, LeyCoreError> {
        self.open_project_brain(project)?;
        let conn = self.open_connection()?;
        let metadata = read_source_version_on(&conn, project, source, version, self)?
            .ok_or_else(|| invalid_brain("SourceVersion is not present"))?;
        let transform: String = conn
            .query_row(
                "SELECT transformation_json
             FROM source_versions
             WHERE project_id=?1 AND source_id=?2 AND source_version_id=?3",
                params![project, source, version,],
                |r| r.get(0),
            )
            .map_err(|e| self.database_error(e))?;
        let (occurrences, occurrence_count) =
            source_occurrences(&conn, project, source, version, self)?;
        Ok(SourceEvidence {
            occurrences,
            occurrence_count,
            version: metadata,
            transformation: serde_json::from_str(&transform)
                .map_err(|_| invalid_brain("invalid source transformation"))?,
            bytes: self.source_version_bytes(project, source, version)?,
        })
    }
    fn publish_import_on(
        &self,
        tx: &Transaction<'_>,
        handle: &ProjectHandle,
        input: &LocalImportInput,
        scan_id: &str,
        started: u64,
        mut scan: Scan,
    ) -> Result<ImportReceipt, LeyCoreError> {
        let policy_digest = hash(&serde_json::to_vec(&(1, &scan.ignore_hashes)).unwrap());
        let old = inventory(tx, &handle.project_id, &input.locator_id, self)?;
        let mut sources = tx
            .prepare(
                "SELECT DISTINCT s.source_id FROM project_sources s JOIN working_copy_inventory i
                      ON i.project_id=s.project_id AND i.source_id=s.source_id
                      WHERE s.project_id=?1 AND i.locator_id=?2 AND s.state='active'",
            )
            .map_err(|e| self.database_error(e))?;
        let active = sources
            .query_map(params![handle.project_id, input.locator_id], |r| {
                r.get::<_, String>(0)
            })
            .map_err(|e| self.database_error(e))?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(|e| self.database_error(e))?;
        drop(sources);
        let now = unix_time_ms()?;
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut renamed = Vec::new();
        let mut deleted = Vec::new();
        let mut old_hashes: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (path, (entry, original)) in &old {
            if entry.state == InventoryState::Retained
                && entry
                    .source_id
                    .as_ref()
                    .is_some_and(|id| active.contains(id))
            {
                if let Some(h) = original {
                    old_hashes.entry(h.clone()).or_default().push(path.clone());
                }
            }
        }
        let mut new_hashes: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (path, file) in &scan.entries {
            if let Some(h) = &file.original {
                new_hashes.entry(h.clone()).or_default().push(path.clone());
            }
        }
        let mut moved = BTreeSet::new();
        for (path, file) in &mut scan.entries {
            let blocked: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1
                 FROM import_erasure_fences
                 WHERE project_id=?1 AND path_digest=?2)",
                    params![handle.project_id, fence(&input.locator_id, path),],
                    |r| r.get(0),
                )
                .map_err(|e| self.database_error(e))?;
            if blocked {
                file.entry.state = InventoryState::Omitted;
                file.entry.omission_reason = Some("erased-source".into());
                file.original = None;
                file.bytes.clear();
                continue;
            }
            let mut source = old.get(path).and_then(|(e, _)| e.source_id.clone());
            file.entry.source_id = source.take();
        }
        let observation_digest = scan_signature(&scan);
        let paths: BTreeSet<_> = scan.entries.keys().cloned().collect();
        for (path, file) in &mut scan.entries {
            if file.entry.state != InventoryState::Retained {
                continue;
            }
            if file.entry.source_id.is_none() {
                if let Some(h) = &file.original {
                    if let (Some(previous), Some(current)) = (old_hashes.get(h), new_hashes.get(h))
                    {
                        if previous.len() == 1
                            && current.len() == 1
                            && !paths.contains(&previous[0])
                        {
                            file.entry.source_id = old[&previous[0]].0.source_id.clone();
                            moved.insert(previous[0].clone());
                            renamed.push(json!({"from":previous[0],"to":path,"sourceId":file.entry.source_id}));
                        }
                    }
                }
            }
            let source = file
                .entry
                .source_id
                .clone()
                .unwrap_or_else(|| format!("src_{}", Uuid::new_v4().simple()));
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1
                 FROM project_sources
                 WHERE project_id=?1 AND source_id=?2 AND state='active')",
                    params![handle.project_id, source,],
                    |r| r.get(0),
                )
                .map_err(|e| self.database_error(e))?;
            if !exists {
                tx.execute(
                    "INSERT INTO project_sources(project_id,source_id,source_kind,display_name,state,created_at_unix_ms)
                     VALUES(?1,?2,'repository-file',?3,'active',?4)",
                    params![
                        handle.project_id,
                        source,
                        Path::new(path).file_name().and_then(|name|name.to_str()).unwrap_or("Imported file"),
                        now as i64,
                    ],
                )
                .map_err(|e| self.database_error(e))?;
                added.push(path.clone());
            }
            let version = self.retain_source_version_on(
                tx,
                handle,
                &source,
                &SourceVersionInput {
                    request_id: format!("{scan_id}:{}", hash(path.as_bytes())),
                    representation_kind: "utf8-text".into(),
                    original_content_hash: file.original.clone(),
                    source_bytes: file.entry.observed_bytes.unwrap_or(0),
                    transformation: file.transformation.clone(),
                    retained_bytes: file.bytes.clone(),
                },
                Some(&json!({
                    "scanId":scan_id,
                    "locatorId":input.locator_id,
                    "relativePath":path,
                    "repositoryId":read_working_copy_on(tx,
                    &handle.project_id,
                    &input.locator_id,
                    self)?.ok_or_else(||invalid_brain("locator disappeared"))?.repository_id,
                    "git":scan.git})),
            )?;
            if old
                .get(path)
                .and_then(|(e, _)| e.source_version_id.as_deref())
                .is_some_and(|id| id != version.source_version_id)
            {
                modified.push(path.clone());
            }
            tx.execute(
                "INSERT OR IGNORE INTO import_source_paths(project_id,locator_id,relative_path,source_id)
                 VALUES(?1,?2,?3,?4)",
                params![
                    handle.project_id,
                    input.locator_id,
                    path,
                    source,
                ],
            )
            .map_err(|e| self.database_error(e))?;
            file.entry.source_id = Some(source);
            file.entry.source_version_id = Some(version.source_version_id);
            file.entry.occurrence_event_id = Some(version.occurrence_event_id);
        }
        let mut entries: BTreeMap<String, (ContentsEntry, Option<String>)> = scan
            .entries
            .into_iter()
            .map(|(path, file)| {
                let mut e = file.entry;
                if e.state == InventoryState::Omitted {
                    if let Some((prior, _)) = old.get(&path) {
                        e.source_id = prior.source_id.clone();
                        e.source_version_id = prior.source_version_id.clone();
                    }
                }
                (path, (e, file.original))
            })
            .collect();
        for (path, (prior, original)) in &old {
            if entries.contains_key(path) || moved.contains(path) {
                continue;
            }
            let ancestor_omission = Path::new(path)
                .ancestors()
                .skip(1)
                .filter_map(|ancestor| entries.get(ancestor.to_str()?))
                .filter(|(e, _)| e.state == InventoryState::Omitted)
                .filter_map(|(e, _)| e.omission_reason.clone())
                .last();
            let mut missing = prior.clone();
            missing.occurrence_event_id = None;
            if let Some(reason) = ancestor_omission {
                missing.state = InventoryState::Omitted;
                missing.omission_reason = Some(reason);
            } else {
                missing.state = InventoryState::Missing;
                missing.omission_reason = None;
                if prior.state != InventoryState::Missing {
                    deleted.push(path.clone());
                }
            }
            entries.insert(path.clone(), (missing, original.clone()));
        }
        if entries.len() > MAX_IMPORT_ENTRIES {
            return Err(invalid_brain("import combined inventory bound exceeded"));
        }
        tx.execute(
            "DELETE FROM working_copy_inventory WHERE project_id=?1 AND locator_id=?2",
            params![handle.project_id, input.locator_id,],
        )
        .map_err(|e| self.database_error(e))?;
        for (path, (entry, original)) in &entries {
            tx.execute(
                "INSERT INTO working_copy_inventory(project_id,locator_id,relative_path,source_id,source_version_id,original_hash,entry_json)
                 VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    handle.project_id,
                    input.locator_id,
                    path,
                    entry.source_id,
                    entry.source_version_id,
                    original,
                    serde_json::to_string(entry).unwrap(),
                ],
            )
            .map_err(|e| self.database_error(e))?;
        }
        let receipt = ImportReceipt {
            project_id: handle.project_id.clone(),
            locator_id: input.locator_id.clone(),
            scan_id: scan_id.to_owned(),
            started_at_unix_ms: started,
            completed_at_unix_ms: now,
            project_generation: handle.generation,
            retained_count: entries
                .values()
                .filter(|(e, _)| e.state == InventoryState::Retained)
                .count(),
            omitted_count: entries
                .values()
                .filter(|(e, _)| e.state == InventoryState::Omitted)
                .count(),
            added,
            modified,
            renamed,
            deleted,
            git: scan.git,
            observation_digest,
            policy_digest,
        };
        let occurrence = ContinuityEventInput {
            event_id: format!("evt_{}", Uuid::new_v4().simple()),
            project_id: handle.project_id.clone(),
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: Some(input.request_id.clone()),
            request_fingerprint: Some(request_fingerprint(&json!({"scanId":scan_id}))?),
            kind: "project-imported".into(),
            payload_version: 1,
            recorded_at_unix_ms: now,
            revision_head: receipt
                .git
                .get("head")
                .and_then(Value::as_str)
                .map(str::to_owned),
            revision_branch: receipt
                .git
                .get("branch")
                .and_then(Value::as_str)
                .map(str::to_owned),
            payload: json!({
                "scanId": scan_id,
                "locatorId": input.locator_id,
                "retainedCount": receipt.retained_count,
                "omittedCount": receipt.omitted_count,
            }),
        };
        append_project_brain_event_on(tx, &occurrence, self.path())?;
        tx.execute(
            "UPDATE working_copy_locators
             SET last_observed_at_unix_ms=?1,revision_head=?2,revision_branch=?3
             WHERE project_id=?4 AND locator_id=?5",
            params![
                now as i64,
                occurrence.revision_head,
                occurrence.revision_branch,
                handle.project_id,
                input.locator_id,
            ],
        )
        .map_err(|e| self.database_error(e))?;
        tx.execute(
            "UPDATE project_import_attempts
             SET status='published',completed_at=?1,receipt_json=?2
             WHERE project_id=?3 AND scan_id=?4 AND status='running'",
            params![
                now as i64,
                serde_json::to_string(&receipt).unwrap(),
                handle.project_id,
                scan_id,
            ],
        )
        .map_err(|e| self.database_error(e))?;
        tx.execute(
            "INSERT INTO working_copy_import_heads(project_id,locator_id,scan_id)
             VALUES(?1,?2,?3)
             ON CONFLICT(project_id,locator_id) DO UPDATE
             SET scan_id=excluded.scan_id",
            params![handle.project_id, input.locator_id, scan_id,],
        )
        .map_err(|e| self.database_error(e))?;
        Ok(receipt)
    }
    pub fn relocate_working_copy(
        &self,
        handle: &ProjectHandle,
        locator_id: &str,
        new_root: impl AsRef<Path>,
        request_id: &str,
    ) -> Result<ProjectHandle, LeyCoreError> {
        validate_request_id(request_id)?;
        let root = safe_root(new_root.as_ref())?;
        self.reject_private_import_root(&root)?;
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| self.database_error(e))?;
        let fingerprint = request_fingerprint(&json!({"locator":locator_id,"destination":root}))?;
        if let Some(event) = crate::continuity_store::read_event_by_request(
            &tx,
            &handle.project_id,
            None,
            request_id,
            self.path(),
        )? {
            if event.kind != "working-copy-relocated"
                || event.request_fingerprint.as_deref() != Some(fingerprint.as_str())
            {
                return Err(invalid_brain(
                    "relocation request was reused with different content",
                ));
            }
            let brain = self.open_project_brain(&handle.project_id)?;
            return Ok(ProjectHandle {
                project_id: handle.project_id.clone(),
                generation: brain.generation,
            });
        }
        require_project_generation_on(&tx, handle, self)?;
        let locator = read_working_copy_on(&tx, &handle.project_id, locator_id, self)?
            .ok_or_else(|| invalid_brain("unknown working-copy locator"))?;
        if locator.state != WorkingCopyState::Authorized {
            return Err(invalid_brain("locator is revoked"));
        }
        let moving = root != locator.local_path;
        if moving && locator.local_path.exists() {
            return Err(invalid_brain(
                "old root still exists; authorize a separate copy explicitly",
            ));
        }
        let conflict: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1
             FROM working_copy_locators
             WHERE local_path=?1 AND state='authorized' AND NOT(project_id=?2 AND locator_id=?3))",
                params![
                    root.to_str()
                        .ok_or_else(|| LeyCoreError::NonUtf8Path(root.clone()))?,
                    handle.project_id,
                    locator_id,
                ],
                |r| r.get(0),
            )
            .map_err(|e| self.database_error(e))?;
        if conflict {
            return Err(invalid_brain(
                "destination root already has an authorized locator",
            ));
        }
        let expected: Option<String> = tx
            .query_row(
                "SELECT root_identity
             FROM working_copy_locators
             WHERE project_id=?1 AND locator_id=?2",
                params![handle.project_id, locator_id,],
                |r| r.get(0),
            )
            .map_err(|e| self.database_error(e))?;
        if expected.as_deref() != Some(root_stamp(&root)?.as_str()) {
            return Err(invalid_brain(
                "destination is not the same authorized directory; authorize a separate locator",
            ));
        }
        let now = unix_time_ms()?;
        let generation = if moving {
            bump_project_generation_on(&tx, &handle.project_id, now, self)?
        } else {
            handle.generation
        };
        tx.execute(
            "UPDATE working_copy_locators
             SET local_path=?1,last_observed_at_unix_ms=?2
             WHERE project_id=?3 AND locator_id=?4",
            params![
                root.to_str().unwrap(),
                now as i64,
                handle.project_id,
                locator_id,
            ],
        )
        .map_err(|e| self.database_error(e))?;
        let event = ContinuityEventInput {
            event_id: format!("evt_{}", Uuid::new_v4().simple()),
            project_id: handle.project_id.clone(),
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: Some(request_id.into()),
            request_fingerprint: Some(fingerprint),
            kind: "working-copy-relocated".into(),
            payload_version: 1,
            recorded_at_unix_ms: now,
            revision_head: None,
            revision_branch: None,
            payload: json!({
                "locatorId":locator_id,
                "oldPathDigest":hash(locator.local_path.to_string_lossy().as_bytes()),
                "newPathDigest":hash(root.to_string_lossy().as_bytes()),
                "projectGeneration":generation,
                "origin":"local-user-control"}),
        };
        append_project_brain_event_on(&tx, &event, self.path())?;
        tx.commit().map_err(|e| self.database_error(e))?;
        Ok(ProjectHandle {
            project_id: handle.project_id.clone(),
            generation,
        })
    }
}
fn import_head(
    conn: &rusqlite::Connection,
    project: &str,
    locator: &str,
    store: &ContinuityStore,
) -> Result<Option<String>, LeyCoreError> {
    conn.query_row(
        "SELECT scan_id FROM working_copy_import_heads WHERE project_id=?1 AND locator_id=?2",
        params![project, locator,],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| store.database_error(e))
}
fn inventory(
    conn: &rusqlite::Connection,
    project: &str,
    locator: &str,
    store: &ContinuityStore,
) -> Result<BTreeMap<String, (ContentsEntry, Option<String>)>, LeyCoreError> {
    let mut stmt = conn
        .prepare(
            "SELECT relative_path,entry_json,original_hash
         FROM working_copy_inventory
         WHERE project_id=?1 AND locator_id=?2
         ORDER BY relative_path",
        )
        .map_err(|e| store.database_error(e))?;
    let rows = stmt
        .query_map(params![project, locator,], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| store.database_error(e))?;
    rows.map(|r| {
        let (path, text, original) = r.map_err(|e| store.database_error(e))?;
        Ok((
            path,
            (
                serde_json::from_str(&text).map_err(|_| invalid_brain("invalid inventory"))?,
                original,
            ),
        ))
    })
    .collect()
}
pub(crate) fn erase_import_source_on(
    tx: &Transaction<'_>,
    project: &str,
    source: &str,
    store: &ContinuityStore,
) -> Result<(), LeyCoreError> {
    let mut stmt = tx
        .prepare(
            "SELECT locator_id,relative_path FROM import_source_paths
         WHERE project_id=?1 AND source_id=?2",
        )
        .map_err(|e| store.database_error(e))?;
    let paths = stmt
        .query_map(params![project, source,], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| store.database_error(e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| store.database_error(e))?;
    drop(stmt);
    for (locator, path) in paths {
        tx.execute(
            "INSERT OR IGNORE INTO import_erasure_fences(project_id,path_digest)
             VALUES(?1,?2)",
            params![project, fence(&locator, &path),],
        )
        .map_err(|e| store.database_error(e))?;
    }
    tx.execute(
        "DELETE FROM import_source_paths WHERE project_id=?1 AND source_id=?2",
        params![project, source,],
    )
    .map_err(|e| store.database_error(e))?;
    tx.execute(
        "UPDATE project_import_attempts SET status='scrubbed',receipt_json=NULL WHERE project_id=?1",
        [project],
    ).map_err(|e|store.database_error(e))?;
    tx.execute(
        "DELETE FROM working_copy_import_heads WHERE project_id=?1",
        [project],
    )
    .map_err(|e| store.database_error(e))?;
    tx.execute(
        "DELETE FROM working_copy_inventory WHERE project_id=?1 AND source_id=?2",
        params![project, source,],
    )
    .map_err(|e| store.database_error(e))?;
    tx.execute(
        "DELETE FROM events WHERE project_id=?1 AND kind='project-imported'",
        [project],
    )
    .map_err(|e| store.database_error(e))?;
    Ok(())
}
fn scan_signature(scan: &Scan) -> String {
    hash(
        &serde_json::to_vec(&(
            scan.entries
                .iter()
                .map(|(p, f)| {
                    (
                        p,
                        f.entry.state,
                        f.entry.observed_bytes,
                        &f.entry.omission_reason,
                        &f.original,
                        &f.transformation,
                    )
                })
                .collect::<Vec<_>>(),
            &scan.git,
            &scan.ignore_hashes,
        ))
        .unwrap(),
    )
}
fn collect(
    root: &Path,
    dir: &Dir,
    locator: &str,
    exclusions: &ScanExclusions,
) -> Result<Scan, LeyCoreError> {
    let mut scan = Scan {
        entries: BTreeMap::new(),
        git: if exclusions.erased.is_empty() {
            git_observation(root, dir)
        } else {
            json!({"kind":"unavailable","reason":"erasure-restricted-git-observation"})
        },
        ignore_hashes: BTreeMap::new(),
    };
    let mut budget = ScanBudget {
        entries: 0,
        original: 0,
        retained: 0,
        ignores: 0,
    };
    walk(
        root,
        dir,
        dir,
        Path::new(""),
        &[],
        &mut budget,
        &mut scan,
        locator,
        exclusions,
    )?;
    Ok(scan)
}
struct ScanBudget {
    entries: usize,
    original: u64,
    retained: u64,
    ignores: u64,
}
fn omission(path: String, reason: &str, bytes: Option<u64>) -> Scanned {
    Scanned {
        entry: ContentsEntry {
            relative_path: path,
            state: InventoryState::Omitted,
            source_id: None,
            source_version_id: None,
            occurrence_event_id: None,
            observed_bytes: bytes,
            omission_reason: Some(reason.into()),
        },
        original: None,
        bytes: Vec::new(),
        transformation: Value::Null,
    }
}
fn hard_exclusion(name: &str, is_dir: bool) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        ".git"
            | ".ley"
            | "node_modules"
            | "vendor"
            | ".venv"
            | "venv"
            | "__pycache__"
            | ".cache"
            | "cache"
            | ".next"
            | ".nuxt"
            | ".svelte-kit"
            | ".turbo"
            | "target"
            | "dist"
            | "build"
            | "coverage"
            | ".gradle"
            | ".idea"
            | ".vscode"
    ) && is_dir
    {
        return Some("generated-or-private");
    }
    if matches!(
        lower.as_str(),
        ".aws" | ".ssh" | ".gnupg" | ".azure" | ".gcloud"
    ) {
        return Some("secret-file");
    }
    if matches!(lower.as_str(), ".git" | ".ley") {
        return Some("generated-or-private");
    }
    if lower == ".env"
        || lower.starts_with(".env.")
        || lower == ".envrc"
        || lower.starts_with("secrets.")
        || lower == ".netrc"
        || lower == ".npmrc"
        || lower == ".pypirc"
        || lower == "credentials"
        || lower == "credentials.json"
        || lower == "id_rsa"
        || lower == "id_dsa"
        || lower == "id_ecdsa"
        || lower == "id_ed25519"
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.ends_with(".p12")
        || lower.ends_with(".pfx")
        || lower.ends_with(".keystore")
        || lower.ends_with(".jks")
    {
        return Some("secret-file");
    }
    if [
        ".zip", ".gz", ".tar", ".tgz", ".bz2", ".xz", ".7z", ".rar", ".png", ".jpg", ".jpeg",
        ".gif", ".webp", ".ico", ".pdf", ".mp3", ".mp4", ".wav", ".woff", ".woff2",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension))
    {
        return Some("binary");
    }
    if lower.ends_with(".min.js")
        || lower.ends_with(".min.css")
        || lower.ends_with(".map")
        || lower.ends_with(".pyc")
        || lower.ends_with(".class")
        || lower.ends_with(".o")
        || lower.ends_with(".so")
        || lower.ends_with(".dll")
        || lower.ends_with(".exe")
    {
        return Some("generated-or-private");
    }
    None
}
fn walk(
    root: &Path,
    root_dir: &Dir,
    dir: &Dir,
    relative: &Path,
    inherited: &[Gitignore],
    budget: &mut ScanBudget,
    scan: &mut Scan,
    locator: &str,
    exclusions: &ScanExclusions,
) -> Result<(), LeyCoreError> {
    if relative.components().count() > MAX_DEPTH {
        return Err(invalid_brain("import directory depth bound exceeded"));
    }
    let mut names = dir
        .entries()
        .map_err(|e| io(relative, e))?
        .take(
            MAX_IMPORT_ENTRIES
                .saturating_sub(budget.entries)
                .saturating_add(1),
        )
        .map(|e| e.map(|e| e.file_name()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io(relative, e))?;
    names.sort();
    budget.entries = budget
        .entries
        .checked_add(names.len())
        .ok_or_else(|| invalid_brain("import work bound exceeded"))?;
    if budget.entries > MAX_IMPORT_ENTRIES {
        return Err(invalid_brain("import entry/work bound exceeded"));
    }
    let mut matchers = inherited.to_vec();
    let mut ignore_files = Vec::new();
    for ignore_name in [".gitignore", ".leyignore"] {
        if names.iter().any(|n| n == ignore_name) {
            let path = relative.join(ignore_name);
            ignore_files.push((
                path,
                dir.symlink_metadata(ignore_name)
                    .map_err(|e| io(relative, e))?,
            ));
        }
    }
    if relative.as_os_str().is_empty() && names.iter().any(|n| n == ".ley") {
        let metadata = dir
            .symlink_metadata(".ley")
            .map_err(|e| io(Path::new(".ley"), e))?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            let private = dir
                .open_dir_nofollow(".ley")
                .map_err(|e| io(Path::new(".ley"), e))?;
            match private.symlink_metadata(".leyignore") {
                Ok(metadata) => ignore_files.push((PathBuf::from(".ley/.leyignore"), metadata)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(io(Path::new(".ley/.leyignore"), error)),
            }
        }
    }
    for (path, metadata) in ignore_files {
        if exclusions.erased.contains(&fence(
            locator,
            path.to_str()
                .ok_or_else(|| LeyCoreError::NonUtf8Path(path.clone()))?,
        )) {
            return Err(invalid_brain(
                "an erased ignore policy cannot be reread by folder import",
            ));
        }

        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LeyCoreError::UnsafeProjectLayout(root.join(&path)));
        }
        if metadata.len() > MAX_IGNORE_BYTES {
            return Err(invalid_brain("import ignore-file bound exceeded"));
        }
        budget.ignores += metadata.len();
        if budget.ignores > MAX_IGNORE_TOTAL {
            return Err(invalid_brain("import total ignore-file bound exceeded"));
        }
        let bytes = read_scoped_file(root_dir, &path, metadata.len(), MAX_IGNORE_BYTES)?;
        scan.ignore_hashes.insert(
            path.to_str()
                .ok_or_else(|| LeyCoreError::NonUtf8Path(path.clone()))?
                .to_owned(),
            hash(&bytes),
        );
        let text =
            std::str::from_utf8(&bytes).map_err(|_| invalid_brain("ignore file must be UTF-8"))?;
        let mut builder = GitignoreBuilder::new(root.join(relative));
        for line in text.lines() {
            builder
                .add_line(Some(root.join(&path)), line)
                .map_err(|_| invalid_brain("invalid ignore rule"))?;
        }
        matchers.push(
            builder
                .build()
                .map_err(|_| invalid_brain("invalid ignore rules"))?,
        );
    }
    for name in names {
        let path = relative.join(&name);
        let text = path
            .to_str()
            .ok_or_else(|| LeyCoreError::NonUtf8Path(path.clone()))?
            .to_owned();
        if text.len() > MAX_PATH_BYTES {
            return Err(invalid_brain("import path bound exceeded"));
        }
        if let Some(reason) = exclusions.reason(locator, &text) {
            scan.entries
                .insert(text.clone(), omission(text, reason, None));
            continue;
        }
        let metadata = dir.symlink_metadata(&name).map_err(|e| io(&path, e))?;
        let is_dir = metadata.is_dir();
        let mut reason = hard_exclusion(
            name.to_str()
                .ok_or_else(|| LeyCoreError::NonUtf8Path(path.clone()))?,
            is_dir,
        );
        if reason.is_none() && metadata.file_type().is_symlink() {
            reason = Some("symlink");
        }
        if reason.is_none() {
            let mut ignored = false;
            for matcher in &matchers {
                let m = matcher.matched(root.join(&path), is_dir);
                if m.is_ignore() {
                    ignored = true;
                } else if m.is_whitelist() {
                    ignored = false;
                }
            }
            if ignored {
                reason = Some("ignored");
            }
        }
        if let Some(reason) = reason {
            scan.entries.insert(
                text.clone(),
                omission(
                    text,
                    reason,
                    if is_dir { None } else { Some(metadata.len()) },
                ),
            );
            continue;
        }
        if is_dir {
            let child = dir.open_dir_nofollow(&name).map_err(|e| io(&path, e))?;
            walk(
                root, root_dir, &child, &path, &matchers, budget, scan, locator, exclusions,
            )?;
            continue;
        }
        if !metadata.is_file() {
            return Err(invalid_brain("eligible source is not a regular file"));
        }
        if metadata.len() > MAX_FILE_BYTES {
            scan.entries.insert(
                text.clone(),
                omission(text, "oversized", Some(metadata.len())),
            );
            continue;
        }
        budget.original += metadata.len();
        if budget.original > MAX_IMPORT_TOTAL_BYTES {
            return Err(invalid_brain("import total original byte bound exceeded"));
        }
        let bytes = read_scoped_file(root_dir, &path, metadata.len(), MAX_FILE_BYTES)?;
        let original = hash(&bytes);
        if bytes.contains(&0) {
            scan.entries
                .insert(text.clone(), omission(text, "binary", Some(metadata.len())));
            continue;
        }
        let source = match std::str::from_utf8(&bytes) {
            Ok(s) => s,
            Err(_) => {
                scan.entries.insert(
                    text.clone(),
                    omission(text, "non-utf8", Some(metadata.len())),
                );
                continue;
            }
        };
        let (retained, redactions) = crate::ingestion::redact_secrets(source);
        if retained.len() as u64 > MAX_FILE_BYTES {
            scan.entries.insert(
                text.clone(),
                omission(text, "transformation-oversized", Some(metadata.len())),
            );
            continue;
        }
        budget.retained += retained.len() as u64;
        if budget.retained > MAX_IMPORT_TOTAL_BYTES {
            return Err(invalid_brain("import total retained byte bound exceeded"));
        }
        let transformed = !redactions.is_empty();
        scan.entries.insert(
            text.clone(),
            Scanned {
                entry: ContentsEntry {
                    relative_path: text,
                    state: InventoryState::Retained,
                    source_id: None,
                    source_version_id: None,
                    occurrence_event_id: None,
                    observed_bytes: Some(metadata.len()),
                    omission_reason: None,
                },
                original: Some(original),
                bytes: retained.into_bytes(),
                transformation: json!({
            "policyVersion":1,
            "kind":if transformed{
            "secret-redacted-utf8"}else{
            "utf8-text"},
            "redactions":redactions}),
            },
        );
    }
    Ok(())
}
fn git_observation(root: &Path, dir: &Dir) -> Value {
    let Some(cwd) = capability_cwd(dir) else {
        return json!({"kind":"unavailable","reason":"capability-cwd-unavailable"});
    };
    if dir
        .symlink_metadata(".git")
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return json!({"kind":"unavailable","reason":"unsafe-git-marker"});
    }
    // Reject parent-repository metadata when the selected root is a subdirectory.
    let Some(inside) = git_command(&cwd, &["rev-parse", "--show-toplevel"]) else {
        return if dir.symlink_metadata(".git").is_ok() {
            json!({"kind":"unavailable","reason":"git-metadata-failed"})
        } else {
            json!({"kind":"not-repository"})
        };
    };
    if Path::new(strip_git_newline(&inside)) != root {
        return json!({"kind":"unavailable","reason":"outside-selected-root"});
    }
    let head = git_command(&cwd, &["rev-parse", "--verify", "HEAD"])
        .map(|value| strip_git_newline(&value).to_owned());
    if head.as_ref().is_some_and(|oid| {
        !(oid.len() == 40 || oid.len() == 64) || !oid.bytes().all(|b| b.is_ascii_hexdigit())
    }) {
        return json!({"kind":"unavailable","reason":"git-head-invalid"});
    }
    let full_branch = git_command(&cwd, &["symbolic-ref", "--quiet", "HEAD"])
        .map(|value| strip_git_newline(&value).to_owned());
    let branch = full_branch.as_ref().map(|name| {
        crate::ingestion::redact_secrets(name.strip_prefix("refs/heads/").unwrap_or(name)).0
    });
    let unborn = if head.is_none() {
        let Some(name) = &full_branch else {
            return json!({"kind":"unavailable","reason":"git-head-unavailable"});
        };
        let Some(refs) = git_command(
            &cwd,
            &[
                "for-each-ref",
                "--format=%(refname)%00%(objectname)",
                "--",
                name,
            ],
        ) else {
            return json!({"kind":"unavailable","reason":"git-head-unavailable"});
        };
        if refs.lines().any(|line| {
            line.split_once('\0')
                .is_some_and(|(reference, _)| reference == name)
        }) {
            return json!({"kind":"unavailable","reason":"git-head-unavailable"});
        }
        true
    } else {
        false
    };
    let Some(index) = git_command(&cwd, &["ls-files", "--stage", "--debug", "-z"]) else {
        return json!({"kind":"unavailable","reason":"git-index-stat-unavailable"});
    };
    let Some(working_changes) = git_worktree_stat_changed(dir, &index) else {
        return json!({"kind":"unavailable","reason":"git-index-stat-unavailable"});
    };
    let staged_args: &[&str] = if unborn {
        &["ls-files", "--cached", "-z"]
    } else {
        &[
            "diff-index",
            "--cached",
            "--raw",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "--ignore-submodules=all",
            "HEAD",
        ]
    };
    let Some(staged_changes) = git_command(&cwd, staged_args) else {
        return json!({"kind":"unavailable","reason":"git-index-state-unavailable"});
    };
    let Some(untracked) = git_command(&cwd, &["ls-files", "--others", "--exclude-standard", "-z"])
    else {
        return json!({"kind":"unavailable","reason":"git-untracked-state-unavailable"});
    };
    let Some(common) = git_command(&cwd, &["rev-parse", "--git-common-dir"])
        .and_then(|s| git_path_identity(&cwd, &s))
    else {
        return json!({"kind":"unavailable","reason":"git-repository-identity-unavailable"});
    };
    let Some(git_dir) =
        git_command(&cwd, &["rev-parse", "--git-dir"]).and_then(|s| git_path_identity(&cwd, &s))
    else {
        return json!({"kind":"unavailable","reason":"git-worktree-identity-unavailable"});
    };
    json!({
        "kind":"observed",
        "head":head,
        "headState":if unborn{
        "unborn"}else{
        "resolved"},
        "branch":branch,
        "dirty":working_changes || !staged_changes.is_empty(),
        "worktreeStateBasis":"index-stat-comparison",
        "untracked":!untracked.is_empty(),
        "repositoryIdentity":common,
        "worktreeIdentity":git_dir})
}
fn git_worktree_stat_changed(dir: &Dir, mut index: &str) -> Option<bool> {
    #[cfg(target_os = "linux")]
    {
        use cap_std::fs::MetadataExt;
        let mut changed = false;
        while !index.is_empty() {
            let (header, rest) = index.split_once('\0')?;
            let (identity, path) = header.split_once('\t')?;
            let fields: Vec<_> = identity.split(' ').collect();
            if fields.len() != 3 {
                return None;
            }
            let mut details = Vec::with_capacity(5);
            index = rest;
            for _ in 0..5 {
                let (line, rest) = index.split_once('\n')?;
                details.push(line);
                index = rest;
            }
            let time = |line: &str, prefix: &str| -> Option<(i64, i64)> {
                let (seconds, nanos) = line.strip_prefix(prefix)?.split_once(':')?;
                Some((seconds.parse().ok()?, nanos.parse().ok()?))
            };
            let ctime = time(details[0], "  ctime: ")?;
            let mtime = time(details[1], "  mtime: ")?;
            let size: u64 = details[4]
                .strip_prefix("  size: ")?
                .split_once("\tflags: ")?
                .0
                .parse()
                .ok()?;
            if fields[2] != "0" {
                changed = true;
                continue;
            }
            if fields[0] == "160000" {
                continue;
            }
            if !matches!(fields[0], "100644" | "100755" | "120000") {
                return None;
            }
            let Some(metadata) = scoped_git_metadata(dir, Path::new(path)) else {
                changed = true;
                continue;
            };
            changed |= metadata.len() != size
                || (metadata.mtime(), metadata.mtime_nsec()) != mtime
                || (metadata.ctime(), metadata.ctime_nsec()) != ctime
                || if fields[0] == "120000" {
                    !metadata.file_type().is_symlink()
                } else {
                    !metadata.is_file() || (metadata.mode() & 0o100 != 0) != (fields[0] == "100755")
                };
        }
        Some(changed)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (dir, index);
        None
    }
}
#[cfg(target_os = "linux")]
fn scoped_git_metadata(root: &Dir, path: &Path) -> Option<cap_std::fs::Metadata> {
    let mut dir = root.try_clone().ok()?;
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else {
            return None;
        };
        if components.peek().is_none() {
            return dir.symlink_metadata(name).ok();
        }
        dir = dir.open_dir_nofollow(name).ok()?;
    }
    None
}
fn capability_cwd(dir: &Dir) -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        let path = PathBuf::from(format!(
            "/proc/{}/fd/{}",
            std::process::id(),
            dir.as_raw_fd()
        ));
        if path.is_dir() {
            Some(path)
        } else {
            None
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = dir;
        None
    }
}
fn git_command(root: &Path, args: &[&str]) -> Option<String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let mut child = Command::new(git_executable()?)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", "/nonexistent")
        .env("XDG_CONFIG_HOME", "/nonexistent")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_COUNT", "3")
        .env("GIT_CONFIG_KEY_0", "core.fsmonitor")
        .env("GIT_CONFIG_VALUE_0", "false")
        .env("GIT_CONFIG_KEY_1", "core.untrackedCache")
        .env("GIT_CONFIG_VALUE_1", "false")
        .env("GIT_CONFIG_KEY_2", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_2", "/nonexistent")
        .current_dir(root)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(65_537).read_to_end(&mut bytes).map(|_| bytes)
    });
    let start = Instant::now();
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(2) {
                    let _ = child.kill();
                    let _ = child.wait();
                    break false;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let bytes = reader.join().ok()?.ok()?;
    if !success || bytes.len() > 65_536 {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn strip_git_newline(value: &str) -> &str {
    let value = value.strip_suffix("\n").unwrap_or(value);
    #[cfg(windows)]
    {
        value.strip_suffix("\r").unwrap_or(value)
    }
    #[cfg(not(windows))]
    {
        value
    }
}

fn git_path_identity(root: &Path, value: &str) -> Option<String> {
    let path = Path::new(strip_git_newline(value));
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    };
    absolute
        .canonicalize()
        .ok()
        .map(|path| hash(path.to_string_lossy().as_bytes()))
}
fn git_executable() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path).filter(|dir| dir.is_absolute()) {
        let candidate = dir.join(if cfg!(windows) { "git.exe" } else { "git" });
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn source_occurrences(
    conn: &rusqlite::Connection,
    project: &str,
    source: &str,
    version: &str,
    store: &ContinuityStore,
) -> Result<(Vec<Value>, u64), LeyCoreError> {
    let count:i64=conn.query_row("SELECT count(*)
         FROM event_source_version_links
         WHERE project_id=?1 AND source_id=?2 AND source_version_id=?3 AND relation='retained-version'",
        params![project,
        source,
        version,],
        |r|r.get(0))
        .map_err(|e|store.database_error(e))?;
    let mut stmt=conn.prepare("SELECT e.event_id,e.recorded_at_unix_ms,e.revision_head,e.revision_branch,e.payload_json
         FROM events e
         JOIN event_source_version_links l ON l.project_id=e.project_id AND l.event_id=e.event_id
         WHERE l.project_id=?1 AND l.source_id=?2 AND l.source_version_id=?3 AND l.relation='retained-version'
         ORDER BY e.recorded_at_unix_ms DESC,e.rowid DESC LIMIT 1000")
        .map_err(|e|store.database_error(e))?;
    let rows = stmt
        .query_map(params![project, source, version,], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| store.database_error(e))?;
    let mut output = Vec::new();
    for row in rows {
        let (id, time, head, branch, payload) = row.map_err(|e| store.database_error(e))?;
        let value: Value =
            serde_json::from_str(&payload).map_err(|_| invalid_brain("invalid occurrence"))?;
        output.push(json!({
            "eventId": id,
            "recordedAtUnixMs": time,
            "revisionHead": head,
            "revisionBranch": branch,
            "import": value.get("import"),
        }));
    }
    Ok((output, count as u64))
}

#[cfg(all(test, unix))]
mod root_acquisition_tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn root_substitutions_between_validation_and_acquisition_fail_closed() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("parent");
        let root = parent.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let authorized = root_stamp(&root).unwrap();
        let validated = safe_root(&root).unwrap();
        let saved = temp.path().join("saved");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("canary.txt"), "UNAUTHORIZED").unwrap();
        std::fs::rename(&root, &saved).unwrap();
        symlink(&outside, &root).unwrap();
        assert!(open_authorized_root(&validated, &authorized).is_err());
        std::fs::remove_file(&root).unwrap();
        std::fs::create_dir(&root).unwrap();
        assert!(open_authorized_root(&validated, &authorized).is_err());
        std::fs::remove_dir(&root).unwrap();
        std::fs::rename(&saved, &root).unwrap();
        let saved_parent = temp.path().join("saved-parent");
        std::fs::rename(&parent, &saved_parent).unwrap();
        symlink(&saved_parent, &parent).unwrap();
        assert!(open_authorized_root(&validated, &authorized).is_err());
        std::fs::remove_file(&parent).unwrap();
        std::fs::rename(&saved_parent, &parent).unwrap();
        let opened = open_authorized_root(&validated, &authorized).unwrap();
        std::fs::rename(&root, &saved).unwrap();
        symlink(&outside, &root).unwrap();
        assert_eq!(directory_stamp(&opened, &root).unwrap(), authorized);
        assert!(opened.open("canary.txt").is_err());
        std::fs::remove_file(&root).unwrap();
        std::fs::rename(&saved, &root).unwrap();
        assert_eq!(root_stamp(&root).unwrap(), authorized);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod git_capability_tests {
    use super::*;

    #[test]
    fn git_commands_keep_the_opened_root_during_path_substitution() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let saved = temp.path().join("saved");
        std::fs::create_dir(&root).unwrap();
        assert!(std::process::Command::new("git")
            .args(["init", "-q", "--initial-branch=authorized"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success());
        let stamp = root_stamp(&root).unwrap();
        let opened = open_authorized_root(&root, &stamp).unwrap();
        let cwd = capability_cwd(&opened).unwrap();
        std::fs::rename(&root, &saved).unwrap();
        std::fs::create_dir(&root).unwrap();
        assert!(std::process::Command::new("git")
            .args(["init", "-q", "--initial-branch=substituted"])
            .current_dir(&root)
            .status()
            .unwrap()
            .success());
        assert_eq!(
            git_command(&cwd, &["symbolic-ref", "--short", "HEAD"]).as_deref(),
            Some("authorized\n")
        );
        assert_eq!(git_observation(&root, &opened)["kind"], "unavailable");
        std::fs::remove_dir_all(&root).unwrap();
        std::fs::rename(&saved, &root).unwrap();
        assert_eq!(git_observation(&root, &opened)["branch"], "authorized");
        assert_eq!(root_stamp(&root).unwrap(), stamp);
    }
}
