use crate::ingestion::{
    ArtifactManifest, PortableArtifactBlob, PortableArtifactCitation, PortableArtifactReference,
    PortableArtifactSnapshot,
};
use crate::{
    validate_identity, validate_project_id, ArtifactKind, ArtifactMediaType, LeyCoreError,
    ProjectIdentity, APP_IDENTIFIER,
};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const CONTINUITY_DATABASE_FILE: &str = "continuity.sqlite3";
pub const CONTINUITY_SCHEMA_VERSION: u32 = 13;
pub const CONTINUITY_EVENT_LIMIT_BYTES: usize = 1_048_576;
const CONTINUITY_APPROVED_SOURCE_LIMIT_BYTES: usize = 1_048_576;
const CONTINUITY_EGRESS_AUTHORITY_LOCK_FILE: &str = "continuity-egress.lock";
const CONTINUITY_APPROVED_SOURCE_AUTHORITY_LOCK_FILE: &str = "continuity-approved-source.lock";
const CONTINUITY_ARTIFACT_AUTHORITY_LOCK_FILE: &str = "continuity-artifacts.lock";
const CONTINUITY_ARTIFACT_CONTENT_DIRECTORY: &str = "artifact-content";
// Portable continuity bundles were introduced with schema v2. Their event layout is the
// compatibility floor for bundle validation; normal ContinuityStore opening migrates them forward.
const PORTABLE_DATABASE_SCHEMA_FLOOR: u32 = 2;
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

thread_local! {
    static ACTIVE_EGRESS_AUTHORITY_LOCKS: RefCell<HashSet<PathBuf>> = RefCell::new(HashSet::new());
    static ACTIVE_APPROVED_SOURCE_AUTHORITY_LOCKS: RefCell<HashSet<PathBuf>> = RefCell::new(HashSet::new());
}

#[derive(Debug, Clone)]
pub struct ContinuityStore {
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityProjectObservation {
    pub project_id: String,
    pub root_path: PathBuf,
    pub last_opened_at_unix_ms: u64,
    pub continuity_origin: ContinuityProjectOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContinuityProjectOrigin {
    LegacyUnknown,
    NativeBorn,
}

impl ContinuityProjectOrigin {
    fn as_str(self) -> &'static str {
        match self {
            Self::LegacyUnknown => "legacy-unknown",
            Self::NativeBorn => "native-born",
        }
    }

    fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "legacy-unknown" => Ok(Self::LegacyUnknown),
            "native-born" => Ok(Self::NativeBorn),
            _ => Err(LeyCoreError::InvalidContinuityStore(format!(
                "project continuity origin is invalid: {value:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityArtifactFileMetadata {
    pub artifact_path: String,
    pub kind: String,
    pub language: Option<String>,
    pub media_type: Option<String>,
    pub source_bytes: u64,
    pub stored_bytes: u64,
    pub line_count: u64,
    pub content_hash: String,
    pub content_captured: bool,
    pub redactions_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityArtifactSnapshotSync {
    pub project_id: String,
    pub snapshot_id: String,
    pub created: bool,
    pub previous_current_snapshot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityArtifactContent {
    pub artifact_path: String,
    pub content_hash: String,
    pub media_type: Option<String>,
    pub source_bytes: u64,
    pub stored_bytes: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityCurrentArtifactFile {
    pub artifact_path: String,
    pub kind: crate::ArtifactKind,
    pub language: Option<String>,
    pub media_type: Option<String>,
    pub source_bytes: u64,
    pub stored_bytes: u64,
    pub line_count: u64,
    pub content_hash: String,
    pub content_captured: bool,
    pub redactions: Vec<crate::RedactionFinding>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityCurrentArtifactSnapshot {
    pub project_id: String,
    pub project_name: String,
    pub capture_mode: crate::CaptureMode,
    pub snapshot_id: String,
    pub generated_at_unix_ms: u64,
    pub skipped_files: usize,
    pub skipped: Vec<crate::SkippedArtifact>,
    pub legacy_graph_snapshot_id: Option<String>,
    pub captured_git: Option<crate::GitState>,
    pub files: Vec<ContinuityCurrentArtifactFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityArtifactAuthorityFile {
    pub artifact_path: String,
    pub media_type: Option<String>,
    pub line_count: u64,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityArtifactAuthoritySnapshot {
    pub project_id: String,
    pub snapshot_id: String,
    pub generated_at_unix_ms: u64,
    pub legacy_graph_snapshot_id: Option<String>,
    pub captured_git: Option<crate::GitState>,
    pub files: Vec<ContinuityArtifactAuthorityFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityArtifactWriteBaseline {
    pub current_snapshot_id: Option<String>,
    pub legacy_graph_snapshot_id: Option<String>,
    pub content_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactWriteAuthorityOrigin {
    LegacyCutover,
    NativeBorn,
}

impl ArtifactWriteAuthorityOrigin {
    fn as_str(self) -> &'static str {
        match self {
            Self::LegacyCutover => "legacy-cutover",
            Self::NativeBorn => "native-born",
        }
    }

    fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "legacy-cutover" => Ok(Self::LegacyCutover),
            "native-born" => Ok(Self::NativeBorn),
            _ => Err(LeyCoreError::InvalidContinuityStore(
                "native artifact write-authority origin is invalid".to_owned(),
            )),
        }
    }

    fn project_origin(self) -> ContinuityProjectOrigin {
        match self {
            Self::LegacyCutover => ContinuityProjectOrigin::LegacyUnknown,
            Self::NativeBorn => ContinuityProjectOrigin::NativeBorn,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityApprovedSourceSnapshotInput {
    pub source_id: String,
    pub display_name: String,
    pub content_hash: String,
    pub approved_at_unix_ms: u64,
    pub content_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContinuityApprovedSourceKind {
    ProjectFile,
    ImportedSnapshot,
}

impl ContinuityApprovedSourceKind {
    fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "project-file" => Ok(Self::ProjectFile),
            "imported-snapshot" => Ok(Self::ImportedSnapshot),
            other => Err(LeyCoreError::InvalidContinuityStore(format!(
                "approved source has unsupported kind {other:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityApprovedSourceRecord {
    pub project_id: String,
    pub source_id: String,
    pub source_kind: ContinuityApprovedSourceKind,
    pub display_name: String,
    pub project_relative_path: Option<String>,
    pub content_hash: String,
    pub approved_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContinuityApprovedSourceIssueReason {
    Changed,
    Missing,
    Invalid,
}

impl ContinuityApprovedSourceIssueReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Changed => "changed",
            Self::Missing => "missing",
            Self::Invalid => "invalid",
        }
    }

    fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "changed" => Ok(Self::Changed),
            "missing" => Ok(Self::Missing),
            "invalid" => Ok(Self::Invalid),
            other => Err(LeyCoreError::InvalidContinuityStore(format!(
                "approved-source issue has unsupported reason {other:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityApprovedSourceIssueInput {
    pub source_id: String,
    pub display_name: String,
    pub approved_content_hash: String,
    pub approved_at_unix_ms: u64,
    pub reason: ContinuityApprovedSourceIssueReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityApprovedSourceIssueRecord {
    pub project_id: String,
    pub source_id: String,
    pub display_name: String,
    pub approved_content_hash: String,
    pub approved_at_unix_ms: u64,
    pub reason: ContinuityApprovedSourceIssueReason,
}

struct EgressAuthorityReentryGuard {
    path: PathBuf,
}

impl EgressAuthorityReentryGuard {
    fn enter(path: &Path) -> Result<Self, LeyCoreError> {
        let inserted = ACTIVE_EGRESS_AUTHORITY_LOCKS
            .with(|active| active.borrow_mut().insert(path.to_path_buf()));
        if !inserted {
            return Err(LeyCoreError::AgentEgressAuthorityReentrant);
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl Drop for EgressAuthorityReentryGuard {
    fn drop(&mut self) {
        ACTIVE_EGRESS_AUTHORITY_LOCKS.with(|active| {
            active.borrow_mut().remove(&self.path);
        });
    }
}

struct ApprovedSourceAuthorityReentryGuard {
    path: PathBuf,
}

impl ApprovedSourceAuthorityReentryGuard {
    fn enter(path: &Path) -> Result<Self, LeyCoreError> {
        let inserted = ACTIVE_APPROVED_SOURCE_AUTHORITY_LOCKS
            .with(|active| active.borrow_mut().insert(path.to_path_buf()));
        if !inserted {
            return Err(LeyCoreError::ApprovedSourceAuthorityReentrant);
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl Drop for ApprovedSourceAuthorityReentryGuard {
    fn drop(&mut self) {
        ACTIVE_APPROVED_SOURCE_AUTHORITY_LOCKS.with(|active| {
            active.borrow_mut().remove(&self.path);
        });
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuityEventInput {
    pub event_id: String,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_fingerprint: Option<String>,
    pub kind: String,
    pub payload_version: u32,
    pub recorded_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_branch: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuityEvent {
    pub event_id: String,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_fingerprint: Option<String>,
    pub kind: String,
    pub payload_version: u32,
    pub recorded_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_head: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_branch: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContinuityWrite<T> {
    pub record: T,
    pub created: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuityImportSummary {
    pub project_created: bool,
    pub events_created: usize,
    pub events_replayed: usize,
    pub events_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuitySessionErasurePreview {
    pub project_id: String,
    pub session_id: String,
    pub session_event_count: usize,
    pub dependent_subject_ids: Vec<String>,
    pub total_event_count: usize,
    pub confirmation_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuitySessionErasure {
    pub project_id: String,
    pub session_id: String,
    pub erased_event_count: usize,
    pub erased_subject_ids: Vec<String>,
    pub already_absent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContinuityEventLinkInput {
    pub from_event_id: String,
    pub to_event_id: String,
    pub relation: String,
}

pub fn default_continuity_database_path() -> Result<PathBuf, LeyCoreError> {
    Ok(crate::private_state::default_private_config_dir()?
        .join(APP_IDENTIFIER)
        .join(CONTINUITY_DATABASE_FILE))
}

impl ContinuityStore {
    pub fn system_default() -> Result<Self, LeyCoreError> {
        Ok(Self::at(default_continuity_database_path()?))
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return a path that is guaranteed not to exist and can stand in for a retired legacy
    /// vault when canonical native continuity is already authoritative.
    pub fn native_legacy_placeholder_path(
        &self,
        project_id: &str,
    ) -> Result<PathBuf, LeyCoreError> {
        validate_project_id(project_id)?;
        let parent = self.path.parent().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "continuity database has no parent directory for native continuity access"
                    .to_owned(),
            )
        })?;
        let path = parent.join("native-no-legacy-vault").join(project_id);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
            Err(source) => Err(LeyCoreError::Io { path, source }),
            Ok(_) => Err(LeyCoreError::UnsafeProjectLayout(path)),
        }
    }

    pub fn initialize(&self) -> Result<(), LeyCoreError> {
        let _ = self.open_connection()?;
        Ok(())
    }

    pub fn schema_version(&self) -> Result<u32, LeyCoreError> {
        let connection = self.open_connection()?;
        connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| self.database_error(error))
    }

    pub fn register_project(
        &self,
        identity: &ProjectIdentity,
    ) -> Result<ContinuityWrite<ProjectIdentity>, LeyCoreError> {
        validate_identity(identity)?;
        let connection = self.open_connection()?;
        register_project_on(&connection, identity, &self.path)
    }

    pub(crate) fn stage_artifact_snapshot_metadata(
        &self,
        identity: &ProjectIdentity,
        manifest: &ArtifactManifest,
    ) -> Result<ContinuityArtifactSnapshotSync, LeyCoreError> {
        validate_identity(identity)?;
        crate::ingestion::validate_manifest(manifest, &identity.project_id)?;
        if manifest.project_name != identity.name {
            return Err(LeyCoreError::InvalidContinuityStore(
                "artifact snapshot project name does not match project identity".to_owned(),
            ));
        }
        let capture_policy_json =
            serde_json::to_string(&manifest.capture_policy).map_err(|error| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "artifact capture policy could not be serialized: {error}"
                ))
            })?;
        let skipped_json = serde_json::to_string(&manifest.skipped).map_err(|error| {
            LeyCoreError::InvalidContinuityStore(format!(
                "artifact skipped inventory could not be serialized: {error}"
            ))
        })?;
        let expected_files = continuity_artifact_files(manifest)?;
        let generated_at_unix_ms = sqlite_u64(
            manifest.generated_at_unix_ms,
            "artifact generated-at timestamp",
        )?;

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        register_project_on(&transaction, identity, &self.path)?;
        let previous_current_snapshot_id = transaction
            .query_row(
                "SELECT current_snapshot_id FROM project_artifact_state WHERE project_id = ?1",
                [&identity.project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        let created = transaction
            .execute(
                "INSERT OR IGNORE INTO artifact_snapshots(
                    project_id, snapshot_id, project_name, generated_at_unix_ms, capture_mode,
                    capture_fingerprint, capture_policy_json, skipped_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    identity.project_id,
                    manifest.snapshot_id,
                    manifest.project_name,
                    generated_at_unix_ms,
                    manifest.capture_mode.to_string(),
                    manifest.capture_fingerprint,
                    capture_policy_json,
                    skipped_json,
                ],
            )
            .map_err(|error| self.database_error(error))?
            == 1;
        if created {
            for file in &expected_files {
                let source_bytes = sqlite_u64(file.source_bytes, "artifact source byte count")?;
                let stored_bytes = sqlite_u64(file.stored_bytes, "artifact stored byte count")?;
                let line_count = sqlite_u64(file.line_count, "artifact line count")?;
                transaction
                    .execute(
                        "INSERT INTO artifact_files(
                            project_id, snapshot_id, artifact_path, kind, language, media_type,
                            source_bytes, stored_bytes, line_count, content_hash, content_captured,
                            redactions_json
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        params![
                            identity.project_id,
                            manifest.snapshot_id,
                            file.artifact_path,
                            file.kind,
                            file.language,
                            file.media_type,
                            source_bytes,
                            stored_bytes,
                            line_count,
                            file.content_hash,
                            i64::from(file.content_captured),
                            file.redactions_json,
                        ],
                    )
                    .map_err(|error| self.database_error(error))?;
            }
        } else {
            validate_artifact_snapshot_matches(
                &transaction,
                &self.path,
                identity,
                manifest,
                &capture_policy_json,
                &skipped_json,
                &expected_files,
            )?;
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(ContinuityArtifactSnapshotSync {
            project_id: identity.project_id.clone(),
            snapshot_id: manifest.snapshot_id.clone(),
            created,
            previous_current_snapshot_id,
        })
    }

    pub(crate) fn activate_artifact_snapshot(
        &self,
        project_id: &str,
        snapshot_id: &str,
        captured_at_unix_ms: u64,
        legacy_graph_snapshot_id: Option<&str>,
        captured_git: Option<&crate::GitState>,
    ) -> Result<Option<String>, LeyCoreError> {
        validate_project_id(project_id)?;
        let captured_at_unix_ms = sqlite_u64(captured_at_unix_ms, "artifact capture timestamp")?;
        if legacy_graph_snapshot_id.is_some_and(|value| !valid_graph_snapshot_id(value)) {
            return Err(LeyCoreError::InvalidContinuityStore(
                "legacy graph snapshot ID is invalid".to_owned(),
            ));
        }
        let captured_git_json = captured_git
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "captured Git state could not be serialized: {error}"
                ))
            })?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM artifact_snapshots
                    WHERE project_id = ?1 AND snapshot_id = ?2
                 )",
                params![project_id, snapshot_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        if !exists {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "artifact snapshot {snapshot_id} is not staged for project {project_id}"
            )));
        }
        let previous = transaction
            .query_row(
                "SELECT current_snapshot_id FROM project_artifact_state WHERE project_id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        transaction
            .execute(
                "INSERT INTO project_artifact_state(
                    project_id, current_snapshot_id, legacy_graph_snapshot_id, captured_git_json,
                    captured_at_unix_ms
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(project_id) DO UPDATE
                     SET current_snapshot_id = excluded.current_snapshot_id,
                         legacy_graph_snapshot_id = excluded.legacy_graph_snapshot_id,
                         captured_git_json = excluded.captured_git_json,
                         captured_at_unix_ms = excluded.captured_at_unix_ms",
                params![
                    project_id,
                    snapshot_id,
                    legacy_graph_snapshot_id,
                    captured_git_json,
                    captured_at_unix_ms
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(previous)
    }

    pub(crate) fn activate_initial_artifact_write_authority(
        &self,
        manifest: &ArtifactManifest,
        captured_at_unix_ms: u64,
        legacy_graph_snapshot_id: Option<&str>,
        captured_git: Option<&crate::GitState>,
        origin: ArtifactWriteAuthorityOrigin,
    ) -> Result<(), LeyCoreError> {
        validate_project_id(&manifest.project_id)?;
        crate::ingestion::validate_manifest(manifest, &manifest.project_id)?;
        let captured_at_unix_ms = sqlite_u64(captured_at_unix_ms, "artifact capture timestamp")?;
        if legacy_graph_snapshot_id.is_some_and(|value| !valid_graph_snapshot_id(value)) {
            return Err(LeyCoreError::InvalidContinuityStore(
                "legacy graph snapshot ID is invalid".to_owned(),
            ));
        }
        let captured_git_json = captured_git
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "captured Git state could not be serialized: {error}"
                ))
            })?;
        let recorded_at_unix_ms: i64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                LeyCoreError::InvalidContinuityStore(
                    "system clock is before the Unix epoch".to_owned(),
                )
            })?
            .as_millis()
            .try_into()
            .map_err(|_| {
                LeyCoreError::InvalidContinuityStore(
                    "artifact authority timestamp exceeds SQLite range".to_owned(),
                )
            })?;
        let file_count = sqlite_u64(manifest.files.len() as u64, "artifact file count")?;

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let snapshot_exists: bool = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM artifact_snapshots
                    WHERE project_id = ?1 AND snapshot_id = ?2
                 )",
                params![manifest.project_id, manifest.snapshot_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        if !snapshot_exists {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "artifact snapshot {} is not staged for project {}",
                manifest.snapshot_id, manifest.project_id
            )));
        }
        let has_current: bool = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM project_artifact_state WHERE project_id = ?1
                 )",
                [&manifest.project_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        let has_authority: bool = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM artifact_write_authority WHERE project_id = ?1
                 )",
                [&manifest.project_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        if has_current || has_authority {
            return Err(LeyCoreError::InvalidContinuityStore(
                "initial artifact write authority cannot replace existing artifact authority"
                    .to_owned(),
            ));
        }
        transaction
            .execute(
                "INSERT INTO project_artifact_state(
                    project_id, current_snapshot_id, legacy_graph_snapshot_id, captured_git_json,
                    captured_at_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    manifest.project_id,
                    manifest.snapshot_id,
                    legacy_graph_snapshot_id,
                    captured_git_json,
                    captured_at_unix_ms
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .execute(
                "INSERT INTO artifact_write_authority(
                    project_id, origin, initial_snapshot_id, initial_capture_fingerprint,
                    initial_file_count, recorded_at_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    manifest.project_id,
                    origin.as_str(),
                    manifest.snapshot_id,
                    manifest.capture_fingerprint,
                    file_count,
                    recorded_at_unix_ms
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .execute(
                "UPDATE project_observations
                 SET continuity_origin = ?2
                 WHERE project_id = ?1",
                params![manifest.project_id, origin.project_origin().as_str()],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))
    }

    pub fn project_egress_policy(
        &self,
        project_id: &str,
    ) -> Result<crate::AgentEgressPolicy, LeyCoreError> {
        self.project_egress_state(project_id).map(|state| state.0)
    }

    pub fn project_egress_authority_ready(&self, project_id: &str) -> Result<bool, LeyCoreError> {
        self.project_egress_state(project_id).map(|state| state.1)
    }

    pub(crate) fn approved_source_authority_ready(
        &self,
        project_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let migrated: Option<i64> = connection
            .query_row(
                "SELECT approved_source_authority_migrated
                 FROM projects WHERE project_id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        match migrated {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            Some(value) => Err(LeyCoreError::InvalidContinuityStore(format!(
                "project {project_id} has invalid approved-source migration marker {value}"
            ))),
            None => Err(LeyCoreError::InvalidContinuityStore(format!(
                "project {project_id} is not present in the continuity database"
            ))),
        }
    }

    pub(crate) fn approved_sources(
        &self,
        project_id: &str,
    ) -> Result<Vec<ContinuityApprovedSourceRecord>, LeyCoreError> {
        validate_project_id(project_id)?;
        self.require_approved_source_authority_ready(project_id)?;
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT project_id, source_id, source_kind, display_name,
                        project_relative_path, content_hash, approved_at_unix_ms
                 FROM approved_sources
                 WHERE project_id = ?1
                 ORDER BY approved_at_unix_ms DESC, source_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map([project_id], row_to_approved_source)
            .map_err(|error| self.database_error(error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| self.database_error(error))
    }

    pub(crate) fn approved_source(
        &self,
        project_id: &str,
        source_id: &str,
    ) -> Result<Option<ContinuityApprovedSourceRecord>, LeyCoreError> {
        validate_project_id(project_id)?;
        self.require_approved_source_authority_ready(project_id)?;
        let connection = self.open_connection()?;
        connection
            .query_row(
                "SELECT project_id, source_id, source_kind, display_name,
                        project_relative_path, content_hash, approved_at_unix_ms
                 FROM approved_sources
                 WHERE project_id = ?1 AND source_id = ?2",
                params![project_id, source_id],
                row_to_approved_source,
            )
            .optional()
            .map_err(|error| self.database_error(error))
    }

    pub(crate) fn approved_source_issues(
        &self,
        project_id: &str,
    ) -> Result<Vec<ContinuityApprovedSourceIssueRecord>, LeyCoreError> {
        validate_project_id(project_id)?;
        self.require_approved_source_authority_ready(project_id)?;
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT project_id, source_id, display_name, approved_content_hash,
                        approved_at_unix_ms, reason
                 FROM legacy_approved_source_issues
                 WHERE project_id = ?1
                 ORDER BY approved_at_unix_ms DESC, source_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map([project_id], |row| {
                let reason: String = row.get(5)?;
                let reason =
                    ContinuityApprovedSourceIssueReason::parse(&reason).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            reason.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(ContinuityApprovedSourceIssueRecord {
                    project_id: row.get(0)?,
                    source_id: row.get(1)?,
                    display_name: row.get(2)?,
                    approved_content_hash: row.get(3)?,
                    approved_at_unix_ms: i64_to_u64_sql(
                        row.get(4)?,
                        "approved source issue approval time",
                    )?,
                    reason,
                })
            })
            .map_err(|error| self.database_error(error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| self.database_error(error))
    }

    pub(crate) fn approved_project_file_for_path(
        &self,
        project_id: &str,
        relative_path: &str,
    ) -> Result<Option<ContinuityApprovedSourceRecord>, LeyCoreError> {
        validate_project_id(project_id)?;
        self.require_approved_source_authority_ready(project_id)?;
        let connection = self.open_connection()?;
        connection
            .query_row(
                "SELECT project_id, source_id, source_kind, display_name,
                        project_relative_path, content_hash, approved_at_unix_ms
                 FROM approved_sources
                 WHERE project_id = ?1 AND source_kind = 'project-file'
                   AND project_relative_path = ?2",
                params![project_id, relative_path],
                row_to_approved_source,
            )
            .optional()
            .map_err(|error| self.database_error(error))
    }

    #[cfg(test)]
    pub(crate) fn approve_project_file_source(
        &self,
        project_id: &str,
        source_id: &str,
        display_name: &str,
        relative_path: &str,
        content_hash: &str,
        approved_at_unix_ms: u64,
    ) -> Result<ContinuityApprovedSourceRecord, LeyCoreError> {
        self.with_approved_source_authority_lock(|| {
            self.approve_project_file_source_unlocked(
                project_id,
                source_id,
                display_name,
                relative_path,
                content_hash,
                approved_at_unix_ms,
            )
        })
    }

    pub(crate) fn approve_project_file_source_unlocked(
        &self,
        project_id: &str,
        source_id: &str,
        display_name: &str,
        relative_path: &str,
        content_hash: &str,
        approved_at_unix_ms: u64,
    ) -> Result<ContinuityApprovedSourceRecord, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_approved_source_identity(
            source_id,
            display_name,
            content_hash,
            approved_at_unix_ms,
        )?;
        if relative_path.is_empty() || relative_path.chars().count() > 1_024 {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "approved source {source_id} has an invalid project-relative path"
            )));
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        require_approved_source_authority_ready_on(&transaction, project_id, &self.path)?;

        let existing_kind: Option<String> = transaction
            .query_row(
                "SELECT source_kind FROM approved_sources
                 WHERE project_id = ?1 AND source_id = ?2",
                params![project_id, source_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        if existing_kind.as_deref() == Some("imported-snapshot") {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "approved source {source_id} is an imported snapshot and cannot be silently relinked to a project file"
            )));
        }
        let path_owner: Option<String> = transaction
            .query_row(
                "SELECT source_id FROM approved_sources
                 WHERE project_id = ?1 AND source_kind = 'project-file'
                   AND project_relative_path = ?2",
                params![project_id, relative_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        if path_owner
            .as_deref()
            .is_some_and(|owner| owner != source_id)
        {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "project file {relative_path:?} is already approved under {}",
                path_owner.expect("checked as some")
            )));
        }

        transaction
            .execute(
                "INSERT INTO approved_sources(
                    project_id, source_id, source_kind, display_name,
                    project_relative_path, content_hash, snapshot_blob_hash,
                    approved_at_unix_ms
                 ) VALUES (?1, ?2, 'project-file', ?3, ?4, ?5, NULL, ?6)
                 ON CONFLICT(project_id, source_id) DO UPDATE SET
                    source_kind = 'project-file',
                    display_name = excluded.display_name,
                    project_relative_path = excluded.project_relative_path,
                    content_hash = excluded.content_hash,
                    snapshot_blob_hash = NULL,
                    approved_at_unix_ms = excluded.approved_at_unix_ms",
                params![
                    project_id,
                    source_id,
                    display_name,
                    relative_path,
                    content_hash,
                    u64_to_i64(approved_at_unix_ms, "approved source approval time")?
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .execute(
                "DELETE FROM legacy_approved_source_issues
                 WHERE project_id = ?1 AND source_id = ?2",
                params![project_id, source_id],
            )
            .map_err(|error| self.database_error(error))?;
        let record = transaction
            .query_row(
                "SELECT project_id, source_id, source_kind, display_name,
                        project_relative_path, content_hash, approved_at_unix_ms
                 FROM approved_sources
                 WHERE project_id = ?1 AND source_id = ?2",
                params![project_id, source_id],
                row_to_approved_source,
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(record)
    }

    #[cfg(test)]
    pub(crate) fn revoke_approved_source(
        &self,
        project_id: &str,
        source_id: &str,
    ) -> Result<bool, LeyCoreError> {
        self.with_approved_source_authority_lock(|| {
            self.revoke_approved_source_unlocked(project_id, source_id)
        })
    }

    pub(crate) fn revoke_approved_source_unlocked(
        &self,
        project_id: &str,
        source_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        require_approved_source_authority_ready_on(&transaction, project_id, &self.path)?;
        let snapshot_hash: Option<Option<String>> = transaction
            .query_row(
                "SELECT snapshot_blob_hash FROM approved_sources
                 WHERE project_id = ?1 AND source_id = ?2",
                params![project_id, source_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        let issue_deleted = transaction
            .execute(
                "DELETE FROM legacy_approved_source_issues
                 WHERE project_id = ?1 AND source_id = ?2",
                params![project_id, source_id],
            )
            .map_err(|error| self.database_error(error))?
            > 0;
        let deleted = transaction
            .execute(
                "DELETE FROM approved_sources
                 WHERE project_id = ?1 AND source_id = ?2",
                params![project_id, source_id],
            )
            .map_err(|error| self.database_error(error))?
            > 0;
        if let Some(Some(hash)) = snapshot_hash {
            transaction
                .execute(
                    "DELETE FROM approved_source_blobs
                     WHERE project_id = ?1 AND content_hash = ?2
                       AND NOT EXISTS(
                           SELECT 1 FROM approved_sources
                           WHERE project_id = ?1 AND snapshot_blob_hash = ?2
                       )",
                    params![project_id, hash],
                )
                .map_err(|error| self.database_error(error))?;
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(deleted || issue_deleted)
    }

    pub(crate) fn approved_snapshot_bytes(
        &self,
        project_id: &str,
        source_id: &str,
    ) -> Result<Option<Vec<u8>>, LeyCoreError> {
        validate_project_id(project_id)?;
        self.require_approved_source_authority_ready(project_id)?;
        let connection = self.open_connection()?;
        connection
            .query_row(
                "SELECT b.content_bytes
                 FROM approved_sources s
                 JOIN approved_source_blobs b
                   ON b.project_id = s.project_id AND b.content_hash = s.snapshot_blob_hash
                 WHERE s.project_id = ?1 AND s.source_id = ?2
                   AND s.source_kind = 'imported-snapshot'",
                params![project_id, source_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))
    }

    fn require_approved_source_authority_ready(
        &self,
        project_id: &str,
    ) -> Result<(), LeyCoreError> {
        let connection = self.open_connection()?;
        require_approved_source_authority_ready_on(&connection, project_id, &self.path)
    }

    pub(crate) fn establish_empty_approved_source_authority(
        &self,
        project_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        self.with_approved_source_authority_lock(|| {
            self.import_legacy_approved_source_authority_unlocked(project_id, &[], &[])
        })
    }

    #[cfg(test)]
    pub(crate) fn import_legacy_approved_source_authority(
        &self,
        project_id: &str,
        snapshots: &[ContinuityApprovedSourceSnapshotInput],
        issues: &[ContinuityApprovedSourceIssueInput],
    ) -> Result<bool, LeyCoreError> {
        self.with_approved_source_authority_lock(|| {
            self.import_legacy_approved_source_authority_unlocked(project_id, snapshots, issues)
        })
    }

    pub(crate) fn import_legacy_approved_source_authority_unlocked(
        &self,
        project_id: &str,
        snapshots: &[ContinuityApprovedSourceSnapshotInput],
        issues: &[ContinuityApprovedSourceIssueInput],
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_approved_source_import_inventory(snapshots, issues)?;

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let migrated: Option<i64> = transaction
            .query_row(
                "SELECT approved_source_authority_migrated
                 FROM projects WHERE project_id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        match migrated {
            None => {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "project {project_id} is not present in the continuity database"
                )))
            }
            Some(1) => {
                transaction
                    .commit()
                    .map_err(|error| self.database_error(error))?;
                return Ok(false);
            }
            Some(0) => {}
            Some(value) => {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "project {project_id} has invalid approved-source migration marker {value}"
                )))
            }
        }

        for table in [
            "approved_sources",
            "approved_source_blobs",
            "legacy_approved_source_issues",
        ] {
            let existing: i64 = transaction
                .query_row(
                    &format!("SELECT count(*) FROM {table} WHERE project_id = ?1"),
                    [project_id],
                    |row| row.get(0),
                )
                .map_err(|error| self.database_error(error))?;
            if existing != 0 {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "project {project_id} has pending approved-source rows before legacy authority migration"
                )));
            }
        }

        let mut inserted_blobs = BTreeSet::new();
        for snapshot in snapshots {
            if inserted_blobs.insert(snapshot.content_hash.as_str()) {
                transaction
                    .execute(
                        "INSERT INTO approved_source_blobs(project_id, content_hash, content_bytes)
                         VALUES (?1, ?2, ?3)",
                        params![project_id, snapshot.content_hash, snapshot.content_bytes],
                    )
                    .map_err(|error| self.database_error(error))?;
            }
            transaction
                .execute(
                    "INSERT INTO approved_sources(
                        project_id, source_id, source_kind, display_name,
                        project_relative_path, content_hash, snapshot_blob_hash,
                        approved_at_unix_ms
                     ) VALUES (?1, ?2, 'imported-snapshot', ?3, NULL, ?4, ?4, ?5)",
                    params![
                        project_id,
                        snapshot.source_id,
                        snapshot.display_name,
                        snapshot.content_hash,
                        u64_to_i64(
                            snapshot.approved_at_unix_ms,
                            "approved source approval time"
                        )?
                    ],
                )
                .map_err(|error| self.database_error(error))?;
        }
        for issue in issues {
            transaction
                .execute(
                    "INSERT INTO legacy_approved_source_issues(
                        project_id, source_id, display_name, approved_content_hash,
                        approved_at_unix_ms, reason
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        project_id,
                        issue.source_id,
                        issue.display_name,
                        issue.approved_content_hash,
                        u64_to_i64(
                            issue.approved_at_unix_ms,
                            "approved source issue approval time"
                        )?,
                        issue.reason.as_str()
                    ],
                )
                .map_err(|error| self.database_error(error))?;
        }
        transaction
            .execute(
                "UPDATE projects
                 SET approved_source_authority_migrated = 1
                 WHERE project_id = ?1",
                [project_id],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(true)
    }

    pub(crate) fn project_egress_state(
        &self,
        project_id: &str,
    ) -> Result<(crate::AgentEgressPolicy, bool), LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let state: Option<(String, i64)> = connection
            .query_row(
                "SELECT agent_egress_policy, agent_egress_policy_migrated
                 FROM projects WHERE project_id = ?1",
                [project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        let (policy, migrated) = state.ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(format!(
                "project {project_id} is not present in the continuity database"
            ))
        })?;
        let policy = continuity_egress_policy(&policy)?;
        match migrated {
            0 => Ok((policy, false)),
            1 => Ok((policy, true)),
            value => Err(LeyCoreError::InvalidContinuityStore(format!(
                "project {project_id} has invalid egress migration marker {value}"
            ))),
        }
    }

    pub fn set_project_egress_policy(
        &self,
        project_id: &str,
        policy: crate::AgentEgressPolicy,
    ) -> Result<(), LeyCoreError> {
        validate_project_id(project_id)?;
        self.with_egress_authority_lock(|| {
            self.set_project_egress_policy_unlocked(project_id, policy)
        })
    }

    pub(crate) fn establish_default_project_egress_authority(
        &self,
        project_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        self.with_egress_authority_lock(|| {
            let (policy, migrated) = self.project_egress_state(project_id)?;
            if migrated {
                return Ok(false);
            }
            if policy != crate::AgentEgressPolicy::AgentOk {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "native-born project {project_id} has staged non-default egress policy before authority initialization"
                )));
            }
            self.set_project_egress_policy_unlocked(project_id, crate::AgentEgressPolicy::AgentOk)?;
            Ok(true)
        })
    }

    /// Runs one egress-bearing operation while project egress authority is held.
    ///
    /// The callback must not call another egress-authority operation on this store. Re-entry is
    /// rejected rather than waiting on the lock held by the current thread.
    pub fn with_project_egress_locked<T>(
        &self,
        project_id: &str,
        target: crate::AgentEgressTarget,
        operation: impl FnOnce() -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        validate_project_id(project_id)?;
        self.with_egress_authority_lock(|| {
            let (policy, migrated) = self.project_egress_state(project_id)?;
            if !migrated {
                return Err(LeyCoreError::AgentEgressPolicyMigrationPending {
                    project_id: project_id.to_owned(),
                });
            }
            let decision = crate::evaluate_agent_egress(policy, target);
            if !decision.allowed {
                return Err(LeyCoreError::AgentEgressDenied {
                    policy: decision.policy.to_string(),
                    target: target.to_string(),
                });
            }
            operation()
        })
    }

    pub(crate) fn set_project_egress_policy_unlocked(
        &self,
        project_id: &str,
        policy: crate::AgentEgressPolicy,
    ) -> Result<(), LeyCoreError> {
        let connection = self.open_connection()?;
        let changed = connection
            .execute(
                "UPDATE projects
                 SET agent_egress_policy = ?1, agent_egress_policy_migrated = 1
                 WHERE project_id = ?2",
                params![policy.to_string(), project_id],
            )
            .map_err(|error| self.database_error(error))?;
        if changed != 1 {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "project {project_id} is not present in the continuity database"
            )));
        }
        Ok(())
    }

    pub(crate) fn import_legacy_project_egress_policy(
        &self,
        project_id: &str,
        policy: crate::AgentEgressPolicy,
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        self.with_egress_authority_lock(|| {
            self.import_legacy_project_egress_policy_unlocked(project_id, policy)
        })
    }

    fn import_legacy_project_egress_policy_unlocked(
        &self,
        project_id: &str,
        policy: crate::AgentEgressPolicy,
    ) -> Result<bool, LeyCoreError> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let current: Option<(String, i64)> = transaction
            .query_row(
                "SELECT agent_egress_policy, agent_egress_policy_migrated
                 FROM projects WHERE project_id = ?1",
                [project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        let (current_policy, migrated) = current.ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(format!(
                "project {project_id} is not present in the continuity database"
            ))
        })?;
        let current_policy = continuity_egress_policy(&current_policy)?;
        match migrated {
            0 => {
                if current_policy != crate::AgentEgressPolicy::AgentOk && current_policy != policy {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "project {project_id} has pending staged egress policy {current_policy} that conflicts with legacy policy {policy}"
                    )));
                }
                transaction
                    .execute(
                        "UPDATE projects
                         SET agent_egress_policy = ?1, agent_egress_policy_migrated = 1
                         WHERE project_id = ?2",
                        params![policy.to_string(), project_id],
                    )
                    .map_err(|error| self.database_error(error))?;
                transaction
                    .commit()
                    .map_err(|error| self.database_error(error))?;
                Ok(true)
            }
            1 => {
                transaction
                    .commit()
                    .map_err(|error| self.database_error(error))?;
                Ok(false)
            }
            value => Err(LeyCoreError::InvalidContinuityStore(format!(
                "project {project_id} has invalid egress migration marker {value}"
            ))),
        }
    }

    pub(crate) fn with_egress_authority_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let _reentry_guard =
            EgressAuthorityReentryGuard::enter(&self.egress_authority_lock_path())?;
        let lock = self.acquire_egress_authority_lock()?;
        let result = operation();
        let unlock_result = File::unlock(&lock);
        match (result, unlock_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(source)) => Err(LeyCoreError::Io {
                path: self.egress_authority_lock_path(),
                source,
            }),
        }
    }

    fn acquire_egress_authority_lock(&self) -> Result<File, LeyCoreError> {
        let lock_path = self.egress_authority_lock_path();
        let parent = lock_path.parent().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "egress authority lock has no parent directory".to_owned(),
            )
        })?;
        ensure_private_directory(parent)?;
        validate_private_lock_path_if_present(&lock_path, "egress authority lock")?;

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
        validate_private_lock_metadata(
            &lock_path,
            &lock.metadata().map_err(|source| LeyCoreError::Io {
                path: lock_path.clone(),
                source,
            })?,
            "egress authority lock",
        )?;
        lock.lock().map_err(|source| LeyCoreError::Io {
            path: lock_path,
            source,
        })?;
        Ok(lock)
    }

    fn egress_authority_lock_path(&self) -> PathBuf {
        self.path
            .with_file_name(CONTINUITY_EGRESS_AUTHORITY_LOCK_FILE)
    }

    pub(crate) fn with_approved_source_authority_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let lock_path = self.approved_source_authority_lock_path();
        let _reentry_guard = ApprovedSourceAuthorityReentryGuard::enter(&lock_path)?;
        let lock = self.acquire_approved_source_authority_lock()?;
        let result = operation();
        let unlock_result = File::unlock(&lock);
        match (result, unlock_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(source)) => Err(LeyCoreError::Io {
                path: lock_path,
                source,
            }),
        }
    }

    fn acquire_approved_source_authority_lock(&self) -> Result<File, LeyCoreError> {
        let lock_path = self.approved_source_authority_lock_path();
        let parent = lock_path.parent().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "approved-source authority lock has no parent directory".to_owned(),
            )
        })?;
        ensure_private_directory(parent)?;
        validate_private_lock_path_if_present(&lock_path, "approved-source authority lock")?;

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
        validate_private_lock_metadata(
            &lock_path,
            &lock.metadata().map_err(|source| LeyCoreError::Io {
                path: lock_path.clone(),
                source,
            })?,
            "approved-source authority lock",
        )?;
        lock.lock().map_err(|source| LeyCoreError::Io {
            path: lock_path,
            source,
        })?;
        Ok(lock)
    }

    fn approved_source_authority_lock_path(&self) -> PathBuf {
        self.path
            .with_file_name(CONTINUITY_APPROVED_SOURCE_AUTHORITY_LOCK_FILE)
    }

    pub(crate) fn with_artifact_authority_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, LeyCoreError>,
    ) -> Result<T, LeyCoreError> {
        let lock_path = self.artifact_authority_lock_path();
        let lock = self.acquire_artifact_authority_lock()?;
        let result = operation();
        let unlock_result = File::unlock(&lock);
        match (result, unlock_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(source)) => Err(LeyCoreError::Io {
                path: lock_path,
                source,
            }),
        }
    }

    fn acquire_artifact_authority_lock(&self) -> Result<File, LeyCoreError> {
        let lock_path = self.artifact_authority_lock_path();
        let parent = lock_path.parent().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "artifact authority lock has no parent directory".to_owned(),
            )
        })?;
        ensure_private_directory(parent)?;
        validate_private_lock_path_if_present(&lock_path, "artifact authority lock")?;
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
        validate_private_lock_metadata(
            &lock_path,
            &lock.metadata().map_err(|source| LeyCoreError::Io {
                path: lock_path.clone(),
                source,
            })?,
            "artifact authority lock",
        )?;
        lock.lock().map_err(|source| LeyCoreError::Io {
            path: lock_path,
            source,
        })?;
        Ok(lock)
    }

    fn artifact_authority_lock_path(&self) -> PathBuf {
        self.path
            .with_file_name(CONTINUITY_ARTIFACT_AUTHORITY_LOCK_FILE)
    }

    fn artifact_content_root(&self) -> Result<PathBuf, LeyCoreError> {
        let parent = self.path.parent().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "continuity database has no parent directory".to_owned(),
            )
        })?;
        Ok(parent.join(CONTINUITY_ARTIFACT_CONTENT_DIRECTORY))
    }

    fn artifact_project_content_dir(&self, project_id: &str) -> Result<PathBuf, LeyCoreError> {
        validate_project_id(project_id)?;
        Ok(self.artifact_content_root()?.join(project_id))
    }

    pub(crate) fn install_artifact_blob(
        &self,
        project_id: &str,
        content_hash: &str,
        expected_bytes: u64,
        bytes: &[u8],
    ) -> Result<(), LeyCoreError> {
        validate_project_id(project_id)?;
        {
            let connection = self.open_connection()?;
            crate::project_brain::ensure_project_not_terminal_on(&connection, project_id, self)?;
        }
        let digest = artifact_digest(content_hash)?;
        if bytes.len() as u64 != expected_bytes
            || format!("sha256:{:x}", Sha256::digest(bytes)) != content_hash
        {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "native artifact blob failed integrity verification for {content_hash}"
            )));
        }
        let root = self.artifact_content_root()?;
        ensure_private_directory(&root)?;
        let project_dir = self.artifact_project_content_dir(project_id)?;
        ensure_private_directory(&project_dir)?;
        let destination = project_dir.join(digest);
        write_immutable_private_blob(&project_dir, &destination, bytes)?;
        Ok(())
    }

    pub(crate) fn read_project_brain_blob(
        &self,
        project_id: &str,
        content_hash: &str,
        expected_bytes: u64,
    ) -> Result<Vec<u8>, LeyCoreError> {
        validate_project_id(project_id)?;
        let digest = artifact_digest(content_hash)?;
        let path = self.artifact_project_content_dir(project_id)?.join(digest);
        let bytes = read_private_blob(&path, expected_bytes)?;
        if format!("sha256:{:x}", Sha256::digest(&bytes)) != content_hash {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "Project Brain blob failed hash verification: {content_hash}"
            )));
        }
        Ok(bytes)
    }

    pub(crate) fn read_native_portable_artifact_snapshots(
        &self,
        project_id: &str,
        citations: &[PortableArtifactCitation],
    ) -> Result<Option<Vec<PortableArtifactSnapshot>>, LeyCoreError> {
        validate_project_id(project_id)?;
        if citations.is_empty() {
            return Ok(Some(Vec::new()));
        }
        let connection = self.open_connection()?;
        let mut grouped = BTreeMap::<String, Vec<&PortableArtifactCitation>>::new();
        for citation in citations {
            grouped
                .entry(citation.artifact_snapshot_id.clone())
                .or_default()
                .push(citation);
        }
        let project_dir = self.artifact_project_content_dir(project_id)?;
        let mut snapshots = Vec::with_capacity(grouped.len());
        for (snapshot_id, mut citations) in grouped {
            let snapshot_exists: bool = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM artifact_snapshots
                        WHERE project_id = ?1 AND snapshot_id = ?2
                     )",
                    params![project_id, snapshot_id],
                    |row| row.get(0),
                )
                .map_err(|error| self.database_error(error))?;
            if !snapshot_exists {
                return Ok(None);
            }
            citations.sort_by(|left, right| left.artifact_path.cmp(&right.artifact_path));
            let mut artifacts = Vec::with_capacity(citations.len());
            let mut blobs = BTreeMap::<String, PortableArtifactBlob>::new();
            for citation in citations {
                let row: Option<(String, String, i64, i64)> = connection
                    .query_row(
                        "SELECT kind, content_hash, content_captured, stored_bytes
                         FROM artifact_files
                         WHERE project_id = ?1 AND snapshot_id = ?2 AND artifact_path = ?3",
                        params![project_id, snapshot_id, citation.artifact_path],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )
                    .optional()
                    .map_err(|error| self.database_error(error))?;
                let Some((kind, content_hash, content_captured, stored_bytes)) = row else {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "durably cited artifact {} is missing from native snapshot {snapshot_id}",
                        citation.artifact_path
                    )));
                };
                if content_hash != citation.content_hash {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "durably cited artifact {} has a conflicting native content hash",
                        citation.artifact_path
                    )));
                }
                match content_captured {
                    0 => return Ok(None),
                    1 => {}
                    value => {
                        return Err(LeyCoreError::InvalidContinuityStore(format!(
                            "artifact content_captured flag is invalid: {value}"
                        )))
                    }
                }
                let stored_bytes = artifact_i64_to_u64(stored_bytes, "artifact stored_bytes")?;
                let digest = artifact_digest(&content_hash)?;
                let path = project_dir.join(digest);
                let bytes = read_private_blob(&path, stored_bytes)?;
                if format!("sha256:{:x}", Sha256::digest(&bytes)) != content_hash {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "native artifact blob failed hash verification: {content_hash}"
                    )));
                }
                let extension = if kind == "image" { "bin" } else { "txt" };
                let file_name = format!("{digest}.{extension}");
                artifacts.push(PortableArtifactReference {
                    artifact_path: citation.artifact_path.clone(),
                    content_hash: content_hash.clone(),
                    file_name: file_name.clone(),
                    bytes: stored_bytes,
                });
                match blobs.get(&content_hash) {
                    Some(existing)
                        if existing.file_name != file_name || existing.bytes != bytes =>
                    {
                        return Err(LeyCoreError::InvalidContinuityStore(format!(
                            "native artifact content collision for {content_hash}"
                        )));
                    }
                    Some(_) => {}
                    None => {
                        blobs.insert(
                            content_hash.clone(),
                            PortableArtifactBlob {
                                content_hash,
                                file_name,
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
        Ok(Some(snapshots))
    }

    pub(crate) fn current_artifact_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ContinuityCurrentArtifactSnapshot>, LeyCoreError> {
        self.with_artifact_authority_lock(|| self.current_artifact_snapshot_under_lock(project_id))
    }

    pub(crate) fn current_artifact_authority_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ContinuityArtifactAuthoritySnapshot>, LeyCoreError> {
        self.with_artifact_authority_lock(|| {
            validate_project_id(project_id)?;
            let connection = self.open_connection()?;
            let state: Option<(String, i64, Option<String>, Option<String>)> = connection
                .query_row(
                    "SELECT s.snapshot_id, st.captured_at_unix_ms,
                            st.legacy_graph_snapshot_id, st.captured_git_json
                     FROM project_artifact_state st
                     JOIN artifact_snapshots s
                       ON s.project_id = st.project_id
                      AND s.snapshot_id = st.current_snapshot_id
                     WHERE st.project_id = ?1",
                    [project_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()
                .map_err(|error| self.database_error(error))?;
            let Some((
                snapshot_id,
                generated_at_unix_ms,
                legacy_graph_snapshot_id,
                captured_git_json,
            )) = state
            else {
                return Ok(None);
            };
            if legacy_graph_snapshot_id
                .as_deref()
                .is_some_and(|value| !valid_graph_snapshot_id(value))
            {
                return Err(LeyCoreError::InvalidContinuityStore(
                    "legacy graph snapshot ID is invalid".to_owned(),
                ));
            }
            let captured_git = captured_git_json
                .map(|json| {
                    serde_json::from_str(&json).map_err(|error| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "captured Git state is invalid: {error}"
                        ))
                    })
                })
                .transpose()?;
            let mut statement = connection
                .prepare(
                    "SELECT artifact_path, media_type, line_count, content_hash
                     FROM artifact_files
                     WHERE project_id = ?1 AND snapshot_id = ?2
                     ORDER BY artifact_path",
                )
                .map_err(|error| self.database_error(error))?;
            let rows = statement
                .query_map(params![project_id, snapshot_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|error| self.database_error(error))?;
            let mut files = Vec::new();
            for row in rows {
                let (artifact_path, media_type, line_count, content_hash) =
                    row.map_err(|error| self.database_error(error))?;
                files.push(ContinuityArtifactAuthorityFile {
                    artifact_path,
                    media_type,
                    line_count: artifact_i64_to_u64(line_count, "artifact line count")?,
                    content_hash,
                });
            }
            if let Some(git) = &captured_git {
                crate::graph::validate_git_state(git).map_err(|error| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "captured Git state is invalid: {error}"
                    ))
                })?;
            }
            Ok(Some(ContinuityArtifactAuthoritySnapshot {
                project_id: project_id.to_owned(),
                snapshot_id,
                generated_at_unix_ms: artifact_i64_to_u64(
                    generated_at_unix_ms,
                    "artifact generated-at timestamp",
                )?,
                legacy_graph_snapshot_id,
                captured_git,
                files,
            }))
        })
    }

    pub(crate) fn current_artifact_hashes(
        &self,
        project_id: &str,
    ) -> Result<Option<BTreeMap<String, String>>, LeyCoreError> {
        self.with_artifact_authority_lock(|| {
            validate_project_id(project_id)?;
            let connection = self.open_connection()?;
            let snapshot_id: Option<String> = connection
                .query_row(
                    "SELECT current_snapshot_id
                     FROM project_artifact_state
                     WHERE project_id = ?1",
                    [project_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| self.database_error(error))?;
            let Some(snapshot_id) = snapshot_id else {
                return Ok(None);
            };
            let mut statement = connection
                .prepare(
                    "SELECT artifact_path, content_hash
                     FROM artifact_files
                     WHERE project_id = ?1 AND snapshot_id = ?2
                     ORDER BY artifact_path",
                )
                .map_err(|error| self.database_error(error))?;
            let rows = statement
                .query_map(params![project_id, snapshot_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| self.database_error(error))?;
            let mut hashes = BTreeMap::new();
            for row in rows {
                let (path, hash) = row.map_err(|error| self.database_error(error))?;
                hashes.insert(path, hash);
            }
            Ok(Some(hashes))
        })
    }

    pub(crate) fn artifact_read_authority_ready(
        &self,
        project_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let graph_snapshot_id: Option<Option<String>> = connection
            .query_row(
                "SELECT st.legacy_graph_snapshot_id
                 FROM project_artifact_state st
                 JOIN artifact_snapshots s
                   ON s.project_id = st.project_id
                  AND s.snapshot_id = st.current_snapshot_id
                 WHERE st.project_id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        match graph_snapshot_id {
            None => Ok(false),
            Some(Some(value)) if !valid_graph_snapshot_id(&value) => {
                Err(LeyCoreError::InvalidContinuityStore(
                    "legacy graph snapshot ID is invalid".to_owned(),
                ))
            }
            Some(_) => Ok(true),
        }
    }

    pub(crate) fn artifact_write_authority_ready(
        &self,
        project_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let marker: Option<(String, String, String, i64)> = connection
            .query_row(
                "SELECT origin, initial_snapshot_id, initial_capture_fingerprint, initial_file_count
                 FROM artifact_write_authority
                 WHERE project_id = ?1",
                [project_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        let Some((origin, initial_snapshot_id, capture_fingerprint, file_count)) = marker else {
            return Ok(false);
        };
        ArtifactWriteAuthorityOrigin::parse(&origin)?;
        if !valid_artifact_snapshot_id(&initial_snapshot_id)
            || capture_fingerprint.len() != 71
            || !capture_fingerprint.starts_with("sha256:")
            || !capture_fingerprint[7..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || file_count < 0
        {
            return Err(LeyCoreError::InvalidContinuityStore(
                "native artifact write-authority record is invalid".to_owned(),
            ));
        }
        let current_ready: bool = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1
                    FROM project_artifact_state st
                    JOIN artifact_snapshots s
                     ON s.project_id = st.project_id
                     AND s.snapshot_id = st.current_snapshot_id
                    WHERE st.project_id = ?1
                 )",
                [project_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        if !current_ready {
            return Err(LeyCoreError::InvalidContinuityStore(
                "native artifact write-authority record has no current artifact authority"
                    .to_owned(),
            ));
        }
        Ok(true)
    }

    pub(crate) fn artifact_write_authority_origin(
        &self,
        project_id: &str,
    ) -> Result<Option<ArtifactWriteAuthorityOrigin>, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let origin: Option<String> = connection
            .query_row(
                "SELECT origin FROM artifact_write_authority WHERE project_id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        origin
            .map(|value| ArtifactWriteAuthorityOrigin::parse(&value))
            .transpose()
    }

    pub(crate) fn mark_artifact_write_authority_under_lock(
        &self,
        manifest: &ArtifactManifest,
        origin: ArtifactWriteAuthorityOrigin,
    ) -> Result<(), LeyCoreError> {
        validate_project_id(&manifest.project_id)?;
        crate::ingestion::validate_manifest(manifest, &manifest.project_id)?;
        let recorded_at_unix_ms: i64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                LeyCoreError::InvalidContinuityStore(
                    "system clock is before the Unix epoch".to_owned(),
                )
            })?
            .as_millis()
            .try_into()
            .map_err(|_| {
                LeyCoreError::InvalidContinuityStore(
                    "artifact cutover timestamp exceeds SQLite range".to_owned(),
                )
            })?;
        let file_count = sqlite_u64(manifest.files.len() as u64, "artifact file count")?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let current: Option<String> = transaction
            .query_row(
                "SELECT current_snapshot_id
                 FROM project_artifact_state
                 WHERE project_id = ?1",
                [&manifest.project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        if current.as_deref() != Some(manifest.snapshot_id.as_str()) {
            return Err(LeyCoreError::InvalidContinuityStore(
                "artifact write-authority snapshot is not the current artifact authority"
                    .to_owned(),
            ));
        }
        let existing: Option<(String, String, String, i64)> = transaction
            .query_row(
                "SELECT origin, initial_snapshot_id, initial_capture_fingerprint, initial_file_count
                 FROM artifact_write_authority
                 WHERE project_id = ?1",
                [&manifest.project_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        if let Some(existing) = existing {
            if existing
                != (
                    origin.as_str().to_owned(),
                    manifest.snapshot_id.clone(),
                    manifest.capture_fingerprint.clone(),
                    file_count,
                )
            {
                return Err(LeyCoreError::InvalidContinuityStore(
                    "native artifact write-authority record conflicts with the established authority origin"
                        .to_owned(),
                ));
            }
        } else {
            transaction
                .execute(
                    "INSERT INTO artifact_write_authority(
                        project_id, origin, initial_snapshot_id, initial_capture_fingerprint,
                        initial_file_count, recorded_at_unix_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        manifest.project_id,
                        origin.as_str(),
                        manifest.snapshot_id,
                        manifest.capture_fingerprint,
                        file_count,
                        recorded_at_unix_ms
                    ],
                )
                .map_err(|error| self.database_error(error))?;
        }
        transaction
            .execute(
                "UPDATE project_observations
                 SET continuity_origin = ?2
                 WHERE project_id = ?1",
                params![manifest.project_id, origin.project_origin().as_str()],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))
    }

    pub(crate) fn artifact_write_baseline_under_lock(
        &self,
        project_id: &str,
    ) -> Result<ContinuityArtifactWriteBaseline, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let state: Option<(String, Option<String>)> = connection
            .query_row(
                "SELECT current_snapshot_id, legacy_graph_snapshot_id
                 FROM project_artifact_state
                 WHERE project_id = ?1",
                [project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        let (current_snapshot_id, legacy_graph_snapshot_id) = match state {
            Some((snapshot_id, graph_snapshot_id)) => (Some(snapshot_id), graph_snapshot_id),
            None => (None, None),
        };
        if legacy_graph_snapshot_id
            .as_deref()
            .is_some_and(|value| !valid_graph_snapshot_id(value))
        {
            return Err(LeyCoreError::InvalidContinuityStore(
                "legacy graph snapshot ID is invalid".to_owned(),
            ));
        }
        let mut content_hashes = BTreeMap::new();
        if let Some(snapshot_id) = &current_snapshot_id {
            let mut statement = connection
                .prepare(
                    "SELECT artifact_path, content_hash
                     FROM artifact_files
                     WHERE project_id = ?1 AND snapshot_id = ?2
                     ORDER BY artifact_path",
                )
                .map_err(|error| self.database_error(error))?;
            let rows = statement
                .query_map(params![project_id, snapshot_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| self.database_error(error))?;
            for row in rows {
                let (path, hash) = row.map_err(|error| self.database_error(error))?;
                content_hashes.insert(path, hash);
            }
        }
        Ok(ContinuityArtifactWriteBaseline {
            current_snapshot_id,
            legacy_graph_snapshot_id,
            content_hashes,
        })
    }

    fn current_artifact_snapshot_under_lock(
        &self,
        project_id: &str,
    ) -> Result<Option<ContinuityCurrentArtifactSnapshot>, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let state: Option<(
            String,
            String,
            String,
            String,
            i64,
            Option<String>,
            Option<String>,
        )> = connection
            .query_row(
                "SELECT s.snapshot_id, s.project_name, s.capture_mode, s.skipped_json,
                        st.captured_at_unix_ms, st.legacy_graph_snapshot_id,
                        st.captured_git_json
                 FROM project_artifact_state st
                 JOIN artifact_snapshots s
                   ON s.project_id = st.project_id
                  AND s.snapshot_id = st.current_snapshot_id
                 WHERE st.project_id = ?1",
                [project_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        let Some((
            snapshot_id,
            project_name,
            capture_mode,
            skipped_json,
            generated_at_unix_ms,
            legacy_graph_snapshot_id,
            captured_git_json,
        )) = state
        else {
            return Ok(None);
        };
        if legacy_graph_snapshot_id
            .as_deref()
            .is_some_and(|value| !valid_graph_snapshot_id(value))
        {
            return Err(LeyCoreError::InvalidContinuityStore(
                "legacy graph snapshot ID is invalid".to_owned(),
            ));
        }
        let capture_mode = crate::CaptureMode::parse(&capture_mode).map_err(|_| {
            LeyCoreError::InvalidContinuityStore(
                "native artifact snapshot has an invalid capture mode".to_owned(),
            )
        })?;
        let skipped = serde_json::from_str::<Vec<crate::SkippedArtifact>>(&skipped_json).map_err(
            |error| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "native artifact snapshot has invalid skipped metadata: {error}"
                ))
            },
        )?;
        let skipped_files = skipped.len();
        let captured_git = captured_git_json
            .map(|json| {
                serde_json::from_str(&json).map_err(|error| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "captured Git state is invalid: {error}"
                    ))
                })
            })
            .transpose()?;
        if let Some(git) = &captured_git {
            crate::graph::validate_git_state(git).map_err(|error| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "captured Git state is invalid: {error}"
                ))
            })?;
        }
        let mut statement = connection
            .prepare(
                "SELECT artifact_path, kind, language, media_type, source_bytes, stored_bytes,
                        line_count, content_hash, content_captured, redactions_json
                 FROM artifact_files
                 WHERE project_id = ?1 AND snapshot_id = ?2
                 ORDER BY artifact_path",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map(params![project_id, snapshot_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .map_err(|error| self.database_error(error))?;
        let project_dir = self.artifact_project_content_dir(project_id)?;
        let mut files = Vec::new();
        for row in rows {
            let (
                artifact_path,
                kind,
                language,
                media_type,
                source_bytes,
                stored_bytes,
                line_count,
                content_hash,
                content_captured,
                redactions_json,
            ) = row.map_err(|error| self.database_error(error))?;
            let kind = parse_artifact_kind_label(&kind)?;
            let redactions = serde_json::from_str::<Vec<crate::RedactionFinding>>(&redactions_json)
                .map_err(|error| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "native artifact redaction metadata is invalid for {artifact_path}: {error}"
                    ))
                })?;
            let text = match content_captured {
                0 => None,
                1 if media_type.is_some() => None,
                1 => {
                    let bytes = read_private_blob(
                        &project_dir.join(artifact_digest(&content_hash)?),
                        artifact_i64_to_u64(stored_bytes, "artifact stored_bytes")?,
                    )?;
                    if format!("sha256:{:x}", Sha256::digest(&bytes)) != content_hash {
                        return Err(LeyCoreError::InvalidContinuityStore(format!(
                            "native artifact blob failed hash verification: {content_hash}"
                        )));
                    }
                    Some(String::from_utf8(bytes).map_err(|_| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "native text artifact is not UTF-8: {artifact_path}"
                        ))
                    })?)
                }
                value => {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "artifact content_captured flag is invalid: {value}"
                    )))
                }
            };
            files.push(ContinuityCurrentArtifactFile {
                artifact_path,
                kind,
                language,
                media_type,
                source_bytes: artifact_i64_to_u64(source_bytes, "artifact source_bytes")?,
                stored_bytes: artifact_i64_to_u64(stored_bytes, "artifact stored_bytes")?,
                line_count: artifact_i64_to_u64(line_count, "artifact line count")?,
                content_hash,
                content_captured: content_captured == 1,
                redactions,
                text,
            });
        }
        Ok(Some(ContinuityCurrentArtifactSnapshot {
            project_id: project_id.to_owned(),
            project_name,
            capture_mode,
            snapshot_id,
            generated_at_unix_ms: artifact_i64_to_u64(
                generated_at_unix_ms,
                "artifact generated-at timestamp",
            )?,
            skipped_files,
            skipped,
            legacy_graph_snapshot_id,
            captured_git,
            files,
        }))
    }

    pub(crate) fn read_native_artifact_content(
        &self,
        project_id: &str,
        snapshot_id: &str,
        artifact_path: &str,
        content_hash: &str,
        max_bytes: Option<usize>,
    ) -> Result<Option<ContinuityArtifactContent>, LeyCoreError> {
        self.with_artifact_authority_lock(|| {
            validate_project_id(project_id)?;
            if !valid_artifact_snapshot_id(snapshot_id) {
                return Err(LeyCoreError::InvalidRetrievalRequest(
                    "artifactSnapshotId is invalid".to_owned(),
                ));
            }
            crate::ingestion::validate_relative_artifact_path(artifact_path)?;
            artifact_digest(content_hash)?;
            let connection = self.open_connection()?;
            let snapshot_exists: bool = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM artifact_snapshots
                        WHERE project_id = ?1 AND snapshot_id = ?2
                     )",
                    params![project_id, snapshot_id],
                    |row| row.get(0),
                )
                .map_err(|error| self.database_error(error))?;
            if !snapshot_exists {
                return Ok(None);
            }
            let row: Option<(String, Option<String>, String, i64, i64, i64)> = connection
                .query_row(
                    "SELECT kind, media_type, content_hash, content_captured, source_bytes, stored_bytes
                     FROM artifact_files
                     WHERE project_id = ?1 AND snapshot_id = ?2 AND artifact_path = ?3",
                    params![project_id, snapshot_id, artifact_path],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .optional()
                .map_err(|error| self.database_error(error))?;
            let Some((kind, media_type, stored_hash, content_captured, source_bytes, stored_bytes)) =
                row
            else {
                return Err(LeyCoreError::InvalidRetrievalRequest(format!(
                    "artifact is not in the cited snapshot: {artifact_path}"
                )));
            };
            if stored_hash != content_hash {
                return Err(LeyCoreError::InvalidArtifactStore(
                    "citation content hash does not match its captured artifact".to_owned(),
                ));
            }
            match content_captured {
                0 => {
                    return Err(LeyCoreError::ProjectMemoryUnavailable(format!(
                        "source bytes are not retained for {artifact_path} in Minimal capture mode"
                    )))
                }
                1 => {}
                value => {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "artifact content_captured flag is invalid: {value}"
                    )))
                }
            }
            if kind == "image" && media_type.is_none() {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "native image artifact is missing media type: {artifact_path}"
                )));
            }
            if kind != "image" && media_type.is_some() {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "native text artifact unexpectedly carries media type: {artifact_path}"
                )));
            }
            let source_bytes = artifact_i64_to_u64(source_bytes, "artifact source_bytes")?;
            let stored_bytes = artifact_i64_to_u64(stored_bytes, "artifact stored_bytes")?;
            if max_bytes.is_some_and(|limit| stored_bytes > limit as u64) {
                return Err(LeyCoreError::InvalidRetrievalRequest(format!(
                    "captured artifact is {stored_bytes} bytes and exceeds the {}-byte delivery limit",
                    max_bytes.expect("checked as present")
                )));
            }
            let digest = artifact_digest(content_hash)?;
            let path = self.artifact_project_content_dir(project_id)?.join(digest);
            let bytes = read_private_blob(&path, stored_bytes)?;
            if format!("sha256:{:x}", Sha256::digest(&bytes)) != content_hash {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "native artifact blob failed hash verification: {content_hash}"
                )));
            }
            Ok(Some(ContinuityArtifactContent {
                artifact_path: artifact_path.to_owned(),
                content_hash: content_hash.to_owned(),
                media_type,
                source_bytes,
                stored_bytes,
                bytes,
            }))
        })
    }

    pub(crate) fn garbage_collect_artifact_state_under_lock(
        &self,
        project_id: &str,
    ) -> Result<(), LeyCoreError> {
        validate_project_id(project_id)?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let project_exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE project_id = ?1)",
                [project_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        if !project_exists {
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            self.remove_native_artifact_project_dir(project_id)?;
            return Ok(());
        }

        let current_snapshot: Option<String> = transaction
            .query_row(
                "SELECT current_snapshot_id FROM project_artifact_state WHERE project_id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        let events = project_events_on(&transaction, project_id, &self.path)?;
        let citations = crate::ingestion::collect_artifact_citations(&events)?;
        let mut statement = transaction
            .prepare(
                "SELECT snapshot_id FROM artifact_snapshots
                 WHERE project_id = ?1 ORDER BY snapshot_id",
            )
            .map_err(|error| self.database_error(error))?;
        let snapshot_rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(|error| self.database_error(error))?;
        let all_snapshots = snapshot_rows
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(|error| self.database_error(error))?;
        drop(statement);

        let mut retained_snapshots = BTreeSet::new();
        let mut required_blobs = BTreeMap::<String, u64>::new();
        {
            let mut statement = transaction
                .prepare(
                    "SELECT content_hash, stored_bytes
                     FROM source_versions
                     WHERE project_id = ?1
                     ORDER BY source_id, source_version_id",
                )
                .map_err(|error| self.database_error(error))?;
            let rows = statement
                .query_map([project_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(|error| self.database_error(error))?;
            for row in rows {
                let (content_hash, stored_bytes) =
                    row.map_err(|error| self.database_error(error))?;
                record_required_artifact_blob(
                    &mut required_blobs,
                    &content_hash,
                    artifact_i64_to_u64(stored_bytes, "SourceVersion stored bytes")?,
                )?;
            }
        }
        if let Some(snapshot_id) = &current_snapshot {
            retained_snapshots.insert(snapshot_id.clone());
            let mut statement = transaction
                .prepare(
                    "SELECT content_hash, stored_bytes
                     FROM artifact_files
                     WHERE project_id = ?1 AND snapshot_id = ?2 AND content_captured = 1
                     ORDER BY artifact_path",
                )
                .map_err(|error| self.database_error(error))?;
            let rows = statement
                .query_map(params![project_id, snapshot_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(|error| self.database_error(error))?;
            for row in rows {
                let (content_hash, stored_bytes) =
                    row.map_err(|error| self.database_error(error))?;
                record_required_artifact_blob(
                    &mut required_blobs,
                    &content_hash,
                    artifact_i64_to_u64(stored_bytes, "artifact stored_bytes")?,
                )?;
            }
        }

        for citation in citations {
            if !all_snapshots.contains(&citation.artifact_snapshot_id) {
                continue;
            }
            retained_snapshots.insert(citation.artifact_snapshot_id.clone());
            let row: Option<(String, i64, i64)> = transaction
                .query_row(
                    "SELECT content_hash, content_captured, stored_bytes
                     FROM artifact_files
                     WHERE project_id = ?1 AND snapshot_id = ?2 AND artifact_path = ?3",
                    params![
                        project_id,
                        citation.artifact_snapshot_id,
                        citation.artifact_path
                    ],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|error| self.database_error(error))?;
            let Some((content_hash, content_captured, stored_bytes)) = row else {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "durably cited artifact {} is missing from native snapshot {}",
                    citation.artifact_path, citation.artifact_snapshot_id
                )));
            };
            if content_hash != citation.content_hash {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "durably cited artifact {} has a conflicting native content hash",
                    citation.artifact_path
                )));
            }
            match content_captured {
                0 => {}
                1 => record_required_artifact_blob(
                    &mut required_blobs,
                    &content_hash,
                    artifact_i64_to_u64(stored_bytes, "artifact stored_bytes")?,
                )?,
                value => {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "artifact content_captured flag is invalid: {value}"
                    )))
                }
            }
        }

        for snapshot_id in all_snapshots {
            if !retained_snapshots.contains(&snapshot_id) {
                transaction
                    .execute(
                        "DELETE FROM artifact_snapshots
                         WHERE project_id = ?1 AND snapshot_id = ?2",
                        params![project_id, snapshot_id],
                    )
                    .map_err(|error| self.database_error(error))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        self.reconcile_native_artifact_project_dir(project_id, &required_blobs)
    }

    fn reconcile_native_artifact_project_dir(
        &self,
        project_id: &str,
        required_blobs: &BTreeMap<String, u64>,
    ) -> Result<(), LeyCoreError> {
        let root = self.artifact_content_root()?;
        match fs::symlink_metadata(&root) {
            Ok(metadata) => validate_private_directory_metadata(&root, &metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if required_blobs.is_empty() {
                    return Ok(());
                }
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "native artifact content root is missing for project {project_id}"
                )));
            }
            Err(source) => return Err(LeyCoreError::Io { path: root, source }),
        }
        let project_dir = self.artifact_project_content_dir(project_id)?;
        match fs::symlink_metadata(&project_dir) {
            Ok(metadata) => validate_private_directory_metadata(&project_dir, &metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if required_blobs.is_empty() {
                    return Ok(());
                }
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "native artifact content is missing for project {project_id}"
                )));
            }
            Err(source) => {
                return Err(LeyCoreError::Io {
                    path: project_dir,
                    source,
                })
            }
        }

        let required_by_digest = required_blobs
            .iter()
            .map(|(hash, bytes)| Ok((artifact_digest(hash)?.to_owned(), (hash, *bytes))))
            .collect::<Result<BTreeMap<_, _>, LeyCoreError>>()?;
        let mut seen = BTreeSet::new();
        let entries = fs::read_dir(&project_dir).map_err(|source| LeyCoreError::Io {
            path: project_dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| LeyCoreError::Io {
                path: project_dir.clone(),
                source,
            })?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(|source| LeyCoreError::Io {
                path: path.clone(),
                source,
            })?;
            if name.starts_with(".tmp-artifact-") {
                validate_native_artifact_blob_metadata(&path, &metadata)?;
                fs::remove_file(&path).map_err(|source| LeyCoreError::Io { path, source })?;
                continue;
            }
            if name.len() != 64
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "unexpected entry in native artifact content directory: {name}"
                )));
            }
            validate_native_artifact_blob_metadata(&path, &metadata)?;
            if let Some((content_hash, expected_bytes)) = required_by_digest.get(&name) {
                let bytes = read_private_blob(&path, *expected_bytes)?;
                if format!("sha256:{:x}", Sha256::digest(&bytes)) != **content_hash {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "native artifact blob failed hash verification: {content_hash}"
                    )));
                }
                seen.insert(name);
            } else {
                fs::remove_file(&path).map_err(|source| LeyCoreError::Io { path, source })?;
            }
        }
        for digest in required_by_digest.keys() {
            if !seen.contains(digest) {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "required native artifact blob is missing: {digest}"
                )));
            }
        }
        #[cfg(unix)]
        File::open(&project_dir)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| LeyCoreError::Io {
                path: project_dir.clone(),
                source,
            })?;
        if required_blobs.is_empty() {
            fs::remove_dir(&project_dir).map_err(|source| LeyCoreError::Io {
                path: project_dir.clone(),
                source,
            })?;
            #[cfg(unix)]
            File::open(&root)
                .and_then(|directory| directory.sync_all())
                .map_err(|source| LeyCoreError::Io { path: root, source })?;
        }
        Ok(())
    }

    fn remove_native_artifact_project_dir(&self, project_id: &str) -> Result<(), LeyCoreError> {
        let root = self.artifact_content_root()?;
        let project_dir = self.artifact_project_content_dir(project_id)?;
        match fs::symlink_metadata(&project_dir) {
            Ok(metadata) => {
                validate_private_directory_metadata(&project_dir, &metadata)?;
                fs::remove_dir_all(&project_dir).map_err(|source| LeyCoreError::Io {
                    path: project_dir,
                    source,
                })?;
                #[cfg(unix)]
                if root.exists() {
                    File::open(&root)
                        .and_then(|directory| directory.sync_all())
                        .map_err(|source| LeyCoreError::Io { path: root, source })?;
                }
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(LeyCoreError::Io {
                path: project_dir,
                source,
            }),
        }
    }

    pub(crate) fn migrated_project_egress_policies_unlocked(
        &self,
    ) -> Result<BTreeMap<String, crate::AgentEgressPolicy>, LeyCoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT project_id, agent_egress_policy
                 FROM projects
                 WHERE agent_egress_policy_migrated = 1
                 ORDER BY project_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| self.database_error(error))?;
        let mut policies = BTreeMap::new();
        for row in rows {
            let (project_id, policy) = row.map_err(|error| self.database_error(error))?;
            validate_project_id(&project_id)?;
            policies.insert(project_id, continuity_egress_policy(&policy)?);
        }
        Ok(policies)
    }

    pub(crate) fn project_observations(
        &self,
    ) -> Result<Vec<ContinuityProjectObservation>, LeyCoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT project_id, root_path, last_opened_at_unix_ms, continuity_origin
                 FROM project_observations
                 ORDER BY last_opened_at_unix_ms DESC, project_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|error| self.database_error(error))?;
        rows.map(|row| {
            let (project_id, root_path, last_opened_at_unix_ms, continuity_origin) =
                row.map_err(|error| self.database_error(error))?;
            Ok(ContinuityProjectObservation {
                project_id,
                root_path: PathBuf::from(root_path),
                last_opened_at_unix_ms: i64_to_u64_sql(
                    last_opened_at_unix_ms,
                    "project last_opened_at_unix_ms",
                )
                .map_err(|error| self.database_error(error))?,
                continuity_origin: ContinuityProjectOrigin::parse(&continuity_origin)?,
            })
        })
        .collect()
    }

    pub(crate) fn project_catalog_migration_complete(&self) -> Result<bool, LeyCoreError> {
        let connection = self.open_connection()?;
        let migrated: i64 = connection
            .query_row(
                "SELECT project_catalog_migrated FROM local_migration_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        match migrated {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(LeyCoreError::InvalidContinuityStore(format!(
                "project catalog has invalid migration marker {value}"
            ))),
        }
    }

    pub(crate) fn import_legacy_project_observations_once(
        &self,
        observations: &[ContinuityProjectObservation],
    ) -> Result<bool, LeyCoreError> {
        validate_project_observation_set(observations)?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let migrated: i64 = transaction
            .query_row(
                "SELECT project_catalog_migrated FROM local_migration_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        match migrated {
            1 => return Ok(false),
            0 => {}
            value => {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "project catalog has invalid migration marker {value}"
                )))
            }
        }
        replace_project_observations_on(&transaction, observations, &self.path)?;
        transaction
            .execute(
                "UPDATE local_migration_state SET project_catalog_migrated = 1 WHERE singleton = 1",
                [],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(true)
    }

    pub(crate) fn upsert_project_observation(
        &self,
        observation: &ContinuityProjectObservation,
    ) -> Result<(), LeyCoreError> {
        validate_project_observation(observation)?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        crate::project_brain::ensure_project_not_terminal_on(
            &transaction,
            &observation.project_id,
            self,
        )?;
        let root = observation
            .root_path
            .to_str()
            .expect("validated UTF-8 project root");
        transaction
            .execute(
                "DELETE FROM project_observations WHERE root_path = ?1 AND project_id <> ?2",
                params![root, observation.project_id],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .execute(
                "INSERT INTO project_observations(
                    project_id, root_path, last_opened_at_unix_ms, continuity_origin
                 ) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(project_id) DO UPDATE SET
                    root_path = excluded.root_path,
                    last_opened_at_unix_ms = excluded.last_opened_at_unix_ms,
                    continuity_origin = excluded.continuity_origin",
                params![
                    observation.project_id,
                    root,
                    u64_to_i64(
                        observation.last_opened_at_unix_ms,
                        "project last_opened_at_unix_ms"
                    )?,
                    observation.continuity_origin.as_str()
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))
    }

    pub(crate) fn forget_project_observation(
        &self,
        project_id: &str,
    ) -> Result<Option<ContinuityProjectObservation>, LeyCoreError> {
        validate_project_id(project_id)?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let existing = transaction
            .query_row(
                "SELECT root_path, last_opened_at_unix_ms, continuity_origin
                 FROM project_observations WHERE project_id = ?1",
                [project_id],
                |row| {
                    let opened: i64 = row.get(1)?;
                    Ok((row.get::<_, String>(0)?, opened, row.get::<_, String>(2)?))
                },
            )
            .optional()
            .map_err(|error| self.database_error(error))?;
        transaction
            .execute(
                "DELETE FROM project_observations WHERE project_id = ?1",
                [project_id],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        existing
            .map(|(root_path, opened, origin)| {
                Ok(ContinuityProjectObservation {
                    project_id: project_id.to_owned(),
                    root_path: PathBuf::from(root_path),
                    last_opened_at_unix_ms: i64_to_u64_sql(
                        opened,
                        "project last_opened_at_unix_ms",
                    )
                    .map_err(|error| self.database_error(error))?,
                    continuity_origin: ContinuityProjectOrigin::parse(&origin)?,
                })
            })
            .transpose()
    }

    pub(crate) fn mark_project_native_born(
        &self,
        observation: &ContinuityProjectObservation,
    ) -> Result<(), LeyCoreError> {
        validate_project_observation(observation)?;
        let mut native = observation.clone();
        native.continuity_origin = ContinuityProjectOrigin::NativeBorn;
        self.upsert_project_observation(&native)
    }

    pub(crate) fn project_continuity_origin(
        &self,
        project_id: &str,
    ) -> Result<Option<ContinuityProjectOrigin>, LeyCoreError> {
        validate_project_id(project_id)?;
        Ok(self
            .project_observations()?
            .into_iter()
            .find(|observation| observation.project_id == project_id)
            .map(|observation| observation.continuity_origin))
    }

    #[cfg(test)]
    pub(crate) fn sync_legacy_project_observations(
        &self,
        observations: &[ContinuityProjectObservation],
    ) -> Result<(), LeyCoreError> {
        validate_project_observation_set(observations)?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        replace_project_observations_on(&transaction, observations, &self.path)?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))
    }

    pub fn append_event(
        &self,
        input: &ContinuityEventInput,
    ) -> Result<ContinuityWrite<ContinuityEvent>, LeyCoreError> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let write = append_event_on(&transaction, input, &self.path)?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(write)
    }

    pub(crate) fn append_project_event_if_count(
        &self,
        input: &ContinuityEventInput,
        expected_project_events: usize,
    ) -> Result<ContinuityWrite<ContinuityEvent>, LeyCoreError> {
        validate_event_input(input)?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let actual: i64 = transaction
            .query_row(
                "SELECT count(*) FROM events WHERE project_id = ?1",
                [&input.project_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        let actual = usize::try_from(actual).map_err(|_| {
            LeyCoreError::InvalidContinuityStore(
                "project event count exceeds process addressable range".to_owned(),
            )
        })?;
        if actual != expected_project_events {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "native-born authority expected {expected_project_events} project events, found {actual}"
            )));
        }
        let write = append_event_on(&transaction, input, &self.path)?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(write)
    }

    pub fn import_project_events(
        &self,
        identity: &ProjectIdentity,
        events: &[ContinuityEventInput],
    ) -> Result<ContinuityImportSummary, LeyCoreError> {
        validate_identity(identity)?;
        for event in events {
            validate_event_input(event)?;
            if event.project_id != identity.project_id {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "event {} belongs to project {}, not {}",
                    event.event_id, event.project_id, identity.project_id
                )));
            }
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let project = register_project_on(&transaction, identity, &self.path)?;
        let mut events_created = 0;
        let mut events_replayed = 0;
        for event in events {
            if append_event_on(&transaction, event, &self.path)?.created {
                events_created += 1;
            } else {
                events_replayed += 1;
            }
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(ContinuityImportSummary {
            project_created: project.created,
            events_created,
            events_replayed,
            events_removed: 0,
        })
    }

    pub(crate) fn sync_legacy_project_events(
        &self,
        identity: &ProjectIdentity,
        events: &[ContinuityEventInput],
        links: &[ContinuityEventLinkInput],
    ) -> Result<ContinuityImportSummary, LeyCoreError> {
        validate_identity(identity)?;
        let mut desired_ids = BTreeSet::new();
        for event in events {
            validate_event_input(event)?;
            if event.project_id != identity.project_id {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "event {} belongs to project {}, not {}",
                    event.event_id, event.project_id, identity.project_id
                )));
            }
            if !event.kind.starts_with("legacy-") {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "legacy snapshot contains non-legacy event kind {}",
                    event.kind
                )));
            }
            if !desired_ids.insert(event.event_id.clone()) {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "legacy snapshot repeats event {}",
                    event.event_id
                )));
            }
        }
        for link in links {
            validate_identifier(&link.from_event_id, "source event ID")?;
            validate_identifier(&link.to_event_id, "target event ID")?;
            validate_kind(&link.relation, "event relation")?;
            if link.from_event_id == link.to_event_id {
                return Err(LeyCoreError::InvalidContinuityStore(
                    "an event cannot link to itself".to_owned(),
                ));
            }
            if !desired_ids.contains(&link.from_event_id)
                || !desired_ids.contains(&link.to_event_id)
            {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "legacy link {} -> {} references an event outside the snapshot",
                    link.from_event_id, link.to_event_id
                )));
            }
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let project = register_project_on(&transaction, identity, &self.path)?;
        let mut events_created = 0;
        let mut events_replayed = 0;
        for event in events {
            if append_event_on(&transaction, event, &self.path)?.created {
                events_created += 1;
            } else {
                events_replayed += 1;
            }
        }
        transaction
            .execute(
                "DELETE FROM event_links
                 WHERE project_id = ?1
                   AND relation IN ('depends-on-session', 'supersedes')
                   AND from_event_id IN (
                       SELECT event_id FROM events
                       WHERE project_id = ?1 AND kind LIKE 'legacy-learning-%'
                   )",
                [&identity.project_id],
            )
            .map_err(|error| self.database_error(error))?;
        for link in links {
            link_events_on(
                &transaction,
                &identity.project_id,
                &link.from_event_id,
                &link.to_event_id,
                &link.relation,
                &self.path,
            )?;
        }

        let existing_legacy_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT event_id FROM events
                     WHERE project_id = ?1
                       AND kind LIKE 'legacy-%'
                       AND kind NOT IN (
                           'legacy-session-snapshot-imported',
                           'legacy-learning-snapshot-imported'
                       )
                     ORDER BY event_id",
                )
                .map_err(|error| self.database_error(error))?;
            let rows = statement
                .query_map([&identity.project_id], |row| row.get::<_, String>(0))
                .map_err(|error| self.database_error(error))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| self.database_error(error))?
        };
        let mut events_removed = 0;
        for event_id in existing_legacy_ids {
            if !desired_ids.contains(&event_id) {
                events_removed += transaction
                    .execute(
                        "DELETE FROM events WHERE project_id = ?1 AND event_id = ?2",
                        params![identity.project_id, event_id],
                    )
                    .map_err(|error| self.database_error(error))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(ContinuityImportSummary {
            project_created: project.created,
            events_created,
            events_replayed,
            events_removed,
        })
    }

    // Reconciles the final legacy session snapshot used by the one-way native authority cutover.
    pub(crate) fn sync_legacy_session_events(
        &self,
        identity: &ProjectIdentity,
        events: &[ContinuityEventInput],
        manifest: &ContinuityEventInput,
    ) -> Result<ContinuityImportSummary, LeyCoreError> {
        validate_identity(identity)?;
        let mut desired_ids = BTreeSet::new();
        for event in events {
            validate_event_input(event)?;
            if event.project_id != identity.project_id {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "session event {} belongs to project {}, not {}",
                    event.event_id, event.project_id, identity.project_id
                )));
            }
            if event.session_id.is_none()
                || event.session_sequence.is_none()
                || event.request_id.is_none()
                || event.request_fingerprint.is_none()
                || !event.kind.starts_with("legacy-")
            {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "legacy session snapshot contains invalid session event {}",
                    event.event_id
                )));
            }
            if !desired_ids.insert(event.event_id.clone()) {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "legacy session snapshot repeats event {}",
                    event.event_id
                )));
            }
        }
        validate_event_input(manifest)?;
        if manifest.project_id != identity.project_id
            || manifest.kind != "legacy-session-snapshot-imported"
            || manifest.session_id.is_some()
            || manifest.session_sequence.is_some()
            || manifest.request_id.is_some()
            || manifest.request_fingerprint.is_some()
        {
            return Err(LeyCoreError::InvalidContinuityStore(
                "legacy session snapshot manifest is invalid".to_owned(),
            ));
        }
        if !desired_ids.insert(manifest.event_id.clone()) {
            return Err(LeyCoreError::InvalidContinuityStore(
                "legacy session snapshot manifest collides with a session event".to_owned(),
            ));
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let project = register_project_on(&transaction, identity, &self.path)?;
        let mut events_created = 0;
        let mut events_replayed = 0;
        for event in events.iter().chain(std::iter::once(manifest)) {
            if append_event_on(&transaction, event, &self.path)?.created {
                events_created += 1;
            } else {
                events_replayed += 1;
            }
        }
        let existing_legacy_session_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT event_id FROM events
                     WHERE project_id = ?1
                       AND (
                           (session_id IS NOT NULL AND kind LIKE 'legacy-%')
                           OR kind = 'legacy-session-snapshot-imported'
                       )
                     ORDER BY event_id",
                )
                .map_err(|error| self.database_error(error))?;
            let rows = statement
                .query_map([&identity.project_id], |row| row.get::<_, String>(0))
                .map_err(|error| self.database_error(error))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| self.database_error(error))?
        };
        let mut events_removed = 0;
        for event_id in existing_legacy_session_ids {
            if !desired_ids.contains(&event_id) {
                events_removed += transaction
                    .execute(
                        "DELETE FROM events WHERE project_id = ?1 AND event_id = ?2",
                        params![identity.project_id, event_id],
                    )
                    .map_err(|error| self.database_error(error))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(ContinuityImportSummary {
            project_created: project.created,
            events_created,
            events_replayed,
            events_removed,
        })
    }

    pub(crate) fn sync_legacy_learning_events(
        &self,
        identity: &ProjectIdentity,
        events: &[ContinuityEventInput],
        links: &[ContinuityEventLinkInput],
        manifest: &ContinuityEventInput,
    ) -> Result<ContinuityImportSummary, LeyCoreError> {
        validate_identity(identity)?;
        let mut desired_ids = BTreeSet::new();
        for event in events {
            validate_event_input(event)?;
            if event.project_id != identity.project_id
                || event.subject_id.is_none()
                || event.session_id.is_some()
                || event.session_sequence.is_some()
                || !event.kind.starts_with("legacy-learning-")
            {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "legacy learning snapshot contains invalid event {}",
                    event.event_id
                )));
            }
            if !desired_ids.insert(event.event_id.clone()) {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "legacy learning snapshot repeats event {}",
                    event.event_id
                )));
            }
        }
        validate_event_input(manifest)?;
        if manifest.project_id != identity.project_id
            || manifest.kind != "legacy-learning-snapshot-imported"
            || manifest.subject_id.is_some()
            || manifest.session_id.is_some()
            || manifest.session_sequence.is_some()
            || manifest.request_id.is_some()
            || manifest.request_fingerprint.is_some()
        {
            return Err(LeyCoreError::InvalidContinuityStore(
                "legacy learning snapshot manifest is invalid".to_owned(),
            ));
        }
        if !desired_ids.insert(manifest.event_id.clone()) {
            return Err(LeyCoreError::InvalidContinuityStore(
                "legacy learning snapshot manifest collides with a learning event".to_owned(),
            ));
        }
        for link in links {
            validate_identifier(&link.from_event_id, "source event ID")?;
            validate_identifier(&link.to_event_id, "target event ID")?;
            validate_kind(&link.relation, "event relation")?;
            if !desired_ids.contains(&link.from_event_id)
                || !matches!(link.relation.as_str(), "depends-on-session" | "supersedes")
            {
                return Err(LeyCoreError::InvalidContinuityStore(
                    "legacy learning snapshot contains an invalid event link".to_owned(),
                ));
            }
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let cutover_count: i64 = transaction
            .query_row(
                "SELECT count(*) FROM events
                 WHERE project_id = ?1 AND kind = 'learning-authority-cutover'",
                [&identity.project_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        if cutover_count > 0 {
            return Err(LeyCoreError::InvalidContinuityStore(
                "legacy learning reconciliation is forbidden after native learning authority cutover"
                    .to_owned(),
            ));
        }
        let project = register_project_on(&transaction, identity, &self.path)?;
        let mut events_created = 0;
        let mut events_replayed = 0;
        for event in events.iter().chain(std::iter::once(manifest)) {
            if append_event_on(&transaction, event, &self.path)?.created {
                events_created += 1;
            } else {
                events_replayed += 1;
            }
        }
        transaction
            .execute(
                "DELETE FROM event_links
                 WHERE project_id = ?1
                   AND relation IN ('depends-on-session', 'supersedes')
                   AND from_event_id IN (
                       SELECT event_id FROM events
                       WHERE project_id = ?1 AND kind LIKE 'legacy-learning-%'
                   )",
                [&identity.project_id],
            )
            .map_err(|error| self.database_error(error))?;
        for link in links {
            link_events_on(
                &transaction,
                &identity.project_id,
                &link.from_event_id,
                &link.to_event_id,
                &link.relation,
                &self.path,
            )?;
        }
        let existing_ids = {
            let mut statement = transaction
                .prepare(
                    "SELECT event_id FROM events
                     WHERE project_id = ?1
                       AND (
                           kind LIKE 'legacy-learning-%'
                           OR kind = 'legacy-learning-snapshot-imported'
                       )
                     ORDER BY event_id",
                )
                .map_err(|error| self.database_error(error))?;
            let rows = statement
                .query_map([&identity.project_id], |row| row.get::<_, String>(0))
                .map_err(|error| self.database_error(error))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|error| self.database_error(error))?
        };
        let mut events_removed = 0;
        for event_id in existing_ids {
            if !desired_ids.contains(&event_id) {
                events_removed += transaction
                    .execute(
                        "DELETE FROM events WHERE project_id = ?1 AND event_id = ?2",
                        params![identity.project_id, event_id],
                    )
                    .map_err(|error| self.database_error(error))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(ContinuityImportSummary {
            project_created: project.created,
            events_created,
            events_replayed,
            events_removed,
        })
    }

    pub fn link_events(
        &self,
        project_id: &str,
        from_event_id: &str,
        to_event_id: &str,
        relation: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_identifier(from_event_id, "source event ID")?;
        validate_identifier(to_event_id, "target event ID")?;
        validate_kind(relation, "event relation")?;
        if from_event_id == to_event_id {
            return Err(LeyCoreError::InvalidContinuityStore(
                "an event cannot link to itself".to_owned(),
            ));
        }
        let connection = self.open_connection()?;
        link_events_on(
            &connection,
            project_id,
            from_event_id,
            to_event_id,
            relation,
            &self.path,
        )
    }

    pub fn events_for_session(
        &self,
        project_id: &str,
        session_id: &str,
    ) -> Result<Vec<ContinuityEvent>, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_identifier(session_id, "session ID")?;
        let connection = self.open_connection()?;
        session_events_on(&connection, project_id, session_id, &self.path)
    }

    pub(crate) fn events_for_subject(
        &self,
        project_id: &str,
        subject_id: &str,
    ) -> Result<Vec<ContinuityEvent>, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_identifier(subject_id, "subject ID")?;
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT project_id, event_id, subject_id, session_id, session_sequence, request_id, request_fingerprint, kind, payload_version, recorded_at_unix_ms, revision_head, revision_branch, payload_json
                 FROM events
                 WHERE project_id = ?1 AND subject_id = ?2
                 ORDER BY recorded_at_unix_ms, event_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map(params![project_id, subject_id], row_to_event)
            .map_err(|error| self.database_error(error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| self.database_error(error))
    }

    pub(crate) fn learning_events(
        &self,
        project_id: &str,
    ) -> Result<Vec<ContinuityEvent>, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        learning_events_on(&connection, project_id, &self.path)
    }

    pub(crate) fn append_learning_event_transactional(
        &self,
        project_id: &str,
        build: impl FnOnce(
            &[ContinuityEvent],
        ) -> Result<
            (ContinuityEventInput, Vec<ContinuityEventLinkInput>),
            LeyCoreError,
        >,
    ) -> Result<(ContinuityWrite<ContinuityEvent>, Vec<ContinuityEvent>), LeyCoreError> {
        validate_project_id(project_id)?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let mut events = learning_events_on(&transaction, project_id, &self.path)?;
        let (input, links) = build(&events)?;
        if input.project_id != project_id
            || input.subject_id.is_none()
            || input.session_id.is_some()
            || input.session_sequence.is_some()
            || input.request_id.is_some()
            || input.request_fingerprint.is_some()
            || input.payload_version != 1
            || !matches!(
                input.kind.as_str(),
                "learning-proposed" | "learning-corrected" | "learning-reviewed"
            )
        {
            return Err(LeyCoreError::InvalidContinuityStore(
                "transactional learning event has an invalid native continuity envelope".to_owned(),
            ));
        }
        let write = append_event_on(&transaction, &input, &self.path)?;
        for link in links {
            link_events_on(
                &transaction,
                project_id,
                &link.from_event_id,
                &link.to_event_id,
                &link.relation,
                &self.path,
            )?;
        }
        if write.created {
            events.push(write.record.clone());
            events.sort_by(|left, right| {
                left.recorded_at_unix_ms
                    .cmp(&right.recorded_at_unix_ms)
                    .then_with(|| left.event_id.cmp(&right.event_id))
            });
        } else if !events.iter().any(|event| event == &write.record) {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "replayed learning event {} was absent from the transaction snapshot",
                write.record.event_id
            )));
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok((write, events))
    }

    /// Serializes one native session append against the exact session snapshot in the same
    /// SQLite write transaction.
    pub(crate) fn append_session_event_transactional(
        &self,
        project_id: &str,
        session_id: &str,
        build: impl FnOnce(&[ContinuityEvent]) -> Result<ContinuityEventInput, LeyCoreError>,
    ) -> Result<(ContinuityWrite<ContinuityEvent>, Vec<ContinuityEvent>), LeyCoreError> {
        validate_project_id(project_id)?;
        validate_identifier(session_id, "session ID")?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let mut events = session_events_on(&transaction, project_id, session_id, &self.path)?;
        let input = build(&events)?;
        if input.project_id != project_id || input.session_id.as_deref() != Some(session_id) {
            return Err(LeyCoreError::InvalidContinuityStore(
                "transactional session event belongs to a different project or session".to_owned(),
            ));
        }
        if input.session_sequence.is_none() {
            return Err(LeyCoreError::InvalidContinuityStore(
                "transactional session event requires a session sequence".to_owned(),
            ));
        }
        let write = append_event_on(&transaction, &input, &self.path)?;
        if write.created {
            events.push(write.record.clone());
            events.sort_by(|left, right| {
                left.session_sequence
                    .cmp(&right.session_sequence)
                    .then_with(|| left.recorded_at_unix_ms.cmp(&right.recorded_at_unix_ms))
                    .then_with(|| left.event_id.cmp(&right.event_id))
            });
        } else if !events.iter().any(|event| event == &write.record) {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "replayed session event {} was absent from the transaction snapshot",
                write.record.event_id
            )));
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok((write, events))
    }

    pub(crate) fn has_legacy_snapshot_import(
        &self,
        project_id: &str,
    ) -> Result<bool, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let count: i64 = connection
            .query_row(
                "SELECT count(*) FROM events
                 WHERE project_id = ?1 AND kind = 'legacy-snapshot-imported'",
                [project_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        Ok(count > 0)
    }

    pub(crate) fn session_ids(&self, project_id: &str) -> Result<Vec<String>, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT DISTINCT session_id FROM events
                 WHERE project_id = ?1 AND session_id IS NOT NULL
                 ORDER BY session_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(|error| self.database_error(error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| self.database_error(error))
    }

    pub fn event(
        &self,
        project_id: &str,
        event_id: &str,
    ) -> Result<Option<ContinuityEvent>, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_identifier(event_id, "event ID")?;
        let connection = self.open_connection()?;
        read_event(&connection, project_id, event_id, &self.path)
    }

    pub fn linked_events(
        &self,
        project_id: &str,
        from_event_id: &str,
        relation: &str,
    ) -> Result<Vec<ContinuityEvent>, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_identifier(from_event_id, "source event ID")?;
        validate_kind(relation, "event relation")?;
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT e.project_id, e.event_id, e.subject_id, e.session_id, e.session_sequence, e.request_id, e.request_fingerprint, e.kind, e.payload_version, e.recorded_at_unix_ms, e.revision_head, e.revision_branch, e.payload_json FROM events e JOIN event_links l ON l.project_id = e.project_id AND l.to_event_id = e.event_id WHERE l.project_id = ?1 AND l.from_event_id = ?2 AND l.relation = ?3 ORDER BY e.recorded_at_unix_ms, e.event_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map(params![project_id, from_event_id, relation], row_to_event)
            .map_err(|error| self.database_error(error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| self.database_error(error))
    }

    pub fn preview_session_erasure(
        &self,
        project_id: &str,
        session_id: &str,
    ) -> Result<ContinuitySessionErasurePreview, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_identifier(session_id, "session ID")?;
        let connection = self.open_connection()?;
        let plan = session_erasure_plan_on(&connection, project_id, session_id, &self.path)?
            .ok_or_else(|| LeyCoreError::SessionNotFound(session_id.to_owned()))?;
        session_erasure_preview(project_id, session_id, &plan)
    }

    pub fn erase_session(
        &self,
        project_id: &str,
        session_id: &str,
        expected_confirmation_digest: &str,
    ) -> Result<ContinuitySessionErasure, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_identifier(session_id, "session ID")?;
        if !is_sha256_digest(expected_confirmation_digest) {
            return Err(LeyCoreError::InvalidContinuityStore(
                "session erasure confirmation digest must be sha256".to_owned(),
            ));
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        let Some(plan) = session_erasure_plan_on(&transaction, project_id, session_id, &self.path)?
        else {
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            truncate_wal_after_erasure(&connection, &self.path)?;
            return Ok(ContinuitySessionErasure {
                project_id: project_id.to_owned(),
                session_id: session_id.to_owned(),
                erased_event_count: 0,
                erased_subject_ids: Vec::new(),
                already_absent: true,
            });
        };
        let preview = session_erasure_preview(project_id, session_id, &plan)?;
        if preview.confirmation_digest != expected_confirmation_digest {
            return Err(LeyCoreError::InvalidContinuityStore(
                "session changed since erasure preview; refresh the preview before erasing"
                    .to_owned(),
            ));
        }

        let mut erased_event_count = 0;
        for subject_id in &plan.dependent_subject_ids {
            erased_event_count += transaction
                .execute(
                    "DELETE FROM events WHERE project_id = ?1 AND subject_id = ?2",
                    params![project_id, subject_id],
                )
                .map_err(|error| self.database_error(error))?;
        }
        erased_event_count += transaction
            .execute(
                "DELETE FROM events WHERE project_id = ?1 AND session_id = ?2",
                params![project_id, session_id],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        truncate_wal_after_erasure(&connection, &self.path)?;
        Ok(ContinuitySessionErasure {
            project_id: project_id.to_owned(),
            session_id: session_id.to_owned(),
            erased_event_count,
            erased_subject_ids: plan.dependent_subject_ids.into_iter().collect(),
            already_absent: false,
        })
    }

    pub fn erase_project(&self, project_id: &str) -> Result<(), LeyCoreError> {
        self.erase_project_with_cleanup(project_id, "native", None)
    }

    /// Transitional Desktop operation for the pre-Project-Brain "Agent Memory" surface.
    ///
    /// This is deliberately not Project Brain erasure: it keeps the active Project identity and
    /// path observation so the current Desktop can recapture focused continuity. It fails closed
    /// if new Project Brain canonical state is present rather than silently deleting that state
    /// under the legacy reset contract.
    pub fn reset_agent_memory_for_recapture(&self, project_id: &str) -> Result<(), LeyCoreError> {
        validate_project_id(project_id)?;
        self.with_artifact_authority_lock(|| {
            let mut connection = self.open_connection()?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| self.database_error(error))?;
            let Some(lifecycle) = crate::project_brain::lifecycle_on(&transaction, project_id, self)?
            else {
                let project_exists: bool = transaction
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM projects WHERE project_id = ?1)",
                        [project_id],
                        |row| row.get(0),
                    )
                    .map_err(|error| self.database_error(error))?;
                if project_exists {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "project {project_id} exists without a lifecycle row"
                    )));
                }
                transaction
                    .commit()
                    .map_err(|error| self.database_error(error))?;
                return Ok(());
            };
            match lifecycle.state {
                crate::project_brain::ProjectLifecycleState::Active => {}
                crate::project_brain::ProjectLifecycleState::Erasing => {
                    return Err(LeyCoreError::ProjectErasing {
                        project_id: project_id.to_owned(),
                    })
                }
                crate::project_brain::ProjectLifecycleState::Erased => {
                    return Err(LeyCoreError::ProjectErased {
                        project_id: project_id.to_owned(),
                    })
                }
            }

            let canonical_m1_rows: i64 = transaction
                .query_row(
                    "SELECT
                        (SELECT count(*) FROM project_repositories WHERE project_id = ?1) +
                        (SELECT count(*) FROM project_sources WHERE project_id = ?1) +
                        (SELECT count(*) FROM project_sessions WHERE project_id = ?1) +
                        (SELECT count(*) FROM project_import_attempts WHERE project_id = ?1) +
                        (SELECT count(*) FROM events
                         WHERE project_id = ?1
                           AND kind IN (
                               'source-version-retained', 'source-created',
                               'source-locator-attached', 'episode-recorded',
                               'human-action-recorded'
                           ))",
                    [project_id],
                    |row| row.get(0),
                )
                .map_err(|error| self.database_error(error))?;
            if canonical_m1_rows != 0 {
                return Err(LeyCoreError::InvalidContinuityStore(
                    "the transitional Agent Memory reset cannot erase Project Brain canonical state; use the Project Brain source/session/Brain erasure operations instead"
                        .to_owned(),
                ));
            }

            let next_generation = lifecycle.generation.checked_add(1).ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(
                    "project generation overflow during Agent Memory reset".to_owned(),
                )
            })?;
            let updated_at_unix_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| {
                    LeyCoreError::InvalidContinuityStore(
                        "system clock is before the Unix epoch".to_owned(),
                    )
                })?
                .as_millis();
            let updated_at_unix_ms = i64::try_from(updated_at_unix_ms).map_err(|_| {
                LeyCoreError::InvalidContinuityStore(
                    "Agent Memory reset timestamp exceeds SQLite range".to_owned(),
                )
            })?;
            transaction
                .execute(
                    "UPDATE project_lifecycle
                     SET generation = ?1, updated_at_unix_ms = ?2
                     WHERE project_id = ?3 AND state = 'active' AND generation = ?4",
                    params![
                        i64::try_from(next_generation).map_err(|_| {
                            LeyCoreError::InvalidContinuityStore(
                                "project generation exceeds SQLite range".to_owned(),
                            )
                        })?,
                        updated_at_unix_ms,
                        project_id,
                        i64::try_from(lifecycle.generation).map_err(|_| {
                            LeyCoreError::InvalidContinuityStore(
                                "project generation exceeds SQLite range".to_owned(),
                            )
                        })?,
                    ],
                )
                .map_err(|error| self.database_error(error))?;

            transaction
                .execute("DELETE FROM events WHERE project_id = ?1", [project_id])
                .map_err(|error| self.database_error(error))?;
            transaction
                .execute(
                    "DELETE FROM approved_sources WHERE project_id = ?1",
                    [project_id],
                )
                .map_err(|error| self.database_error(error))?;
            transaction
                .execute(
                    "DELETE FROM legacy_approved_source_issues WHERE project_id = ?1",
                    [project_id],
                )
                .map_err(|error| self.database_error(error))?;
            transaction
                .execute(
                    "DELETE FROM approved_source_blobs WHERE project_id = ?1",
                    [project_id],
                )
                .map_err(|error| self.database_error(error))?;
            transaction
                .execute(
                    "DELETE FROM artifact_write_authority WHERE project_id = ?1",
                    [project_id],
                )
                .map_err(|error| self.database_error(error))?;
            transaction
                .execute(
                    "DELETE FROM project_artifact_state WHERE project_id = ?1",
                    [project_id],
                )
                .map_err(|error| self.database_error(error))?;
            transaction
                .execute(
                    "DELETE FROM artifact_snapshots WHERE project_id = ?1",
                    [project_id],
                )
                .map_err(|error| self.database_error(error))?;
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            drop(connection);

            self.remove_native_artifact_project_dir(project_id)?;
            let connection = self.open_connection()?;
            truncate_wal_after_erasure(&connection, &self.path)
        })
    }

    pub(crate) fn erase_project_with_legacy_cleanup(
        &self,
        project_id: &str,
        legacy_vault: &Path,
    ) -> Result<(), LeyCoreError> {
        let legacy_vault = legacy_vault
            .canonicalize()
            .map_err(|source| LeyCoreError::Io {
                path: legacy_vault.to_path_buf(),
                source,
            })?;
        let legacy_vault = legacy_vault
            .to_str()
            .ok_or_else(|| LeyCoreError::NonUtf8Path(legacy_vault.clone()))?;
        let cleanup_json =
            serde_json::to_string(&json!({"legacyVault": legacy_vault})).map_err(|error| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "project erasure cleanup metadata could not be serialized: {error}"
                ))
            })?;
        self.erase_project_with_cleanup(
            project_id,
            "legacy-and-native",
            Some(cleanup_json.as_str()),
        )
    }

    fn erase_project_with_cleanup(
        &self,
        project_id: &str,
        cleanup_kind: &str,
        cleanup_json: Option<&str>,
    ) -> Result<(), LeyCoreError> {
        validate_project_id(project_id)?;
        self.with_artifact_authority_lock(|| {
            let mut connection = self.open_connection()?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| self.database_error(error))?;
            let lifecycle = crate::project_brain::lifecycle_on(&transaction, project_id, self)?;
            match lifecycle {
                Some(crate::project_brain::ProjectLifecycle {
                    state: crate::project_brain::ProjectLifecycleState::Erased,
                    ..
                }) => {
                    transaction
                        .commit()
                        .map_err(|error| self.database_error(error))?;
                    truncate_wal_after_erasure(&connection, &self.path)?;
                    return Ok(());
                }
                Some(crate::project_brain::ProjectLifecycle {
                    state: crate::project_brain::ProjectLifecycleState::Erasing,
                    cleanup_kind: existing_kind,
                    cleanup_json: existing_json,
                    ..
                }) => {
                    if existing_kind != cleanup_kind || existing_json.as_deref() != cleanup_json {
                        return Err(LeyCoreError::InvalidContinuityStore(format!(
                            "project {project_id} already has different pending erasure cleanup"
                        )));
                    }
                }
                Some(crate::project_brain::ProjectLifecycle {
                    generation,
                    state: crate::project_brain::ProjectLifecycleState::Active,
                    ..
                }) => {
                    let next_generation = generation.checked_add(1).ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(
                            "project generation overflow during erasure".to_owned(),
                        )
                    })?;
                    let updated_at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|_| {
                            LeyCoreError::InvalidContinuityStore(
                                "system clock is before the Unix epoch".to_owned(),
                            )
                        })?
                        .as_millis();
                    let updated_at = i64::try_from(updated_at).map_err(|_| {
                        LeyCoreError::InvalidContinuityStore(
                            "project erasure timestamp exceeds SQLite range".to_owned(),
                        )
                    })?;
                    transaction
                        .execute(
                            "UPDATE project_lifecycle
                             SET generation = ?1, state = 'erasing', updated_at_unix_ms = ?2,
                                 cleanup_kind = ?3, cleanup_json = ?4
                             WHERE project_id = ?5 AND generation = ?6 AND state = 'active'",
                            params![
                                i64::try_from(next_generation).map_err(|_| {
                                    LeyCoreError::InvalidContinuityStore(
                                        "project generation exceeds SQLite range".to_owned(),
                                    )
                                })?,
                                updated_at,
                                cleanup_kind,
                                cleanup_json,
                                project_id,
                                i64::try_from(generation).map_err(|_| {
                                    LeyCoreError::InvalidContinuityStore(
                                        "project generation exceeds SQLite range".to_owned(),
                                    )
                                })?,
                            ],
                        )
                        .map_err(|error| self.database_error(error))?;
                    transaction
                        .execute(
                            "DELETE FROM project_observations WHERE project_id = ?1",
                            [project_id],
                        )
                        .map_err(|error| self.database_error(error))?;
                    transaction
                        .execute("DELETE FROM projects WHERE project_id = ?1", [project_id])
                        .map_err(|error| self.database_error(error))?;
                }
                None => {
                    let project_exists: bool = transaction
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM projects WHERE project_id = ?1)",
                            [project_id],
                            |row| row.get(0),
                        )
                        .map_err(|error| self.database_error(error))?;
                    if !project_exists {
                        transaction
                            .execute(
                                "DELETE FROM project_observations WHERE project_id = ?1",
                                [project_id],
                            )
                            .map_err(|error| self.database_error(error))?;
                        if cleanup_kind == "legacy-and-native" {
                            let updated_at = i64::try_from(
                                SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .map_err(|_| {
                                        LeyCoreError::InvalidContinuityStore(
                                            "system clock is before the Unix epoch".to_owned(),
                                        )
                                    })?
                                    .as_millis(),
                            )
                            .map_err(|_| {
                                LeyCoreError::InvalidContinuityStore(
                                    "project erasure timestamp exceeds SQLite range".to_owned(),
                                )
                            })?;
                            transaction
                                .execute(
                                    "INSERT INTO project_lifecycle(
                                        project_id, generation, state, updated_at_unix_ms,
                                        create_request_id, create_request_fingerprint,
                                        cleanup_kind, cleanup_json
                                     ) VALUES (?1, 1, 'erasing', ?2, NULL, NULL, ?3, ?4)",
                                    params![project_id, updated_at, cleanup_kind, cleanup_json],
                                )
                                .map_err(|error| self.database_error(error))?;
                        } else {
                            transaction
                                .commit()
                                .map_err(|error| self.database_error(error))?;
                            self.remove_native_artifact_project_dir(project_id)?;
                            truncate_wal_after_erasure(&connection, &self.path)?;
                            return Ok(());
                        }
                    }
                    if project_exists {
                        return Err(LeyCoreError::InvalidContinuityStore(format!(
                            "project {project_id} exists without a lifecycle row"
                        )));
                    }
                }
            }
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            drop(connection);

            if cleanup_kind == "legacy-and-native" {
                let cleanup_json = cleanup_json.ok_or_else(|| {
                    LeyCoreError::InvalidContinuityStore(
                        "legacy project erasure is missing cleanup metadata".to_owned(),
                    )
                })?;
                let cleanup: Value = serde_json::from_str(cleanup_json).map_err(|error| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "project erasure cleanup metadata is invalid: {error}"
                    ))
                })?;
                let legacy_vault = cleanup
                    .get("legacyVault")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(
                            "project erasure cleanup metadata is missing legacyVault".to_owned(),
                        )
                    })?;
                crate::ingestion::erase_legacy_project_memory_by_id_terminal(
                    Path::new(legacy_vault),
                    project_id,
                )?;
            }
            self.remove_native_artifact_project_dir(project_id)?;

            let connection = self.open_connection()?;
            truncate_wal_after_erasure(&connection, &self.path)?;
            drop(connection);

            let mut connection = self.open_connection()?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| self.database_error(error))?;
            let changed = transaction
                .execute(
                    "UPDATE project_lifecycle
                     SET state = 'erased', updated_at_unix_ms = ?1,
                         cleanup_kind = 'none', cleanup_json = NULL
                     WHERE project_id = ?2 AND state = 'erasing'",
                    params![
                        i64::try_from(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map_err(|_| {
                                    LeyCoreError::InvalidContinuityStore(
                                        "system clock is before the Unix epoch".to_owned(),
                                    )
                                })?
                                .as_millis()
                        )
                        .map_err(|_| {
                            LeyCoreError::InvalidContinuityStore(
                                "project erasure timestamp exceeds SQLite range".to_owned(),
                            )
                        })?,
                        project_id
                    ],
                )
                .map_err(|error| self.database_error(error))?;
            if changed != 1 {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "project {project_id} erasure lifecycle changed during cleanup"
                )));
            }
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            truncate_wal_after_erasure(&connection, &self.path)
        })
    }

    pub(crate) fn export_project_database(
        &self,
        project_id: &str,
        destination: &Path,
    ) -> Result<(), LeyCoreError> {
        validate_project_id(project_id)?;
        if fs::symlink_metadata(destination).is_ok() {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "portable database destination already exists: {}",
                destination.display()
            )));
        }
        let parent = destination.parent().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(
                "portable database destination has no parent directory".to_owned(),
            )
        })?;
        validate_private_directory_metadata(
            parent,
            &fs::symlink_metadata(parent).map_err(|source| LeyCoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?,
        )?;

        let mut source_connection = self.open_connection()?;
        let source = source_connection
            .transaction()
            .map_err(|error| self.database_error(error))?;
        let project_exists: bool = source
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE project_id = ?1)",
                [project_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        if !project_exists {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "project {project_id} is not present in the continuity database"
            )));
        }
        let source_version_count: i64 = source
            .query_row(
                "SELECT (SELECT count(*) FROM source_versions WHERE project_id = ?1) + (SELECT count(*) FROM project_import_attempts WHERE project_id = ?1)",
                [project_id],
                |row| row.get(0),
            )
            .map_err(|error| self.database_error(error))?;
        if source_version_count != 0 {
            return Err(LeyCoreError::InvalidPortableContinuityBundle(
                "portable continuity v1 cannot represent retained Project Brain SourceVersions or canonical import state; export is disabled until the Project Brain bundle format includes their retained representations"
                    .to_owned(),
            ));
        }

        prepare_private_database_file(destination)?;
        let mut backup_destination = Connection::open_with_flags(
            destination,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|error| database_error(destination, error))?;
        let backup = rusqlite::backup::Backup::new(&source, &mut backup_destination)
            .map_err(|error| self.database_error(error))?;
        backup
            .run_to_completion(100, Duration::from_millis(20), None)
            .map_err(|error| self.database_error(error))?;
        drop(backup);
        drop(backup_destination);
        drop(source);

        let mut exported = Connection::open_with_flags(
            destination,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|error| database_error(destination, error))?;
        exported
            .busy_timeout(SQLITE_BUSY_TIMEOUT)
            .map_err(|error| database_error(destination, error))?;
        exported
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| database_error(destination, error))?;
        exported
            .pragma_update(None, "trusted_schema", "OFF")
            .map_err(|error| database_error(destination, error))?;
        exported
            .pragma_update(None, "secure_delete", "ON")
            .map_err(|error| database_error(destination, error))?;
        let mode: String = exported
            .query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))
            .map_err(|error| database_error(destination, error))?;
        if !mode.eq_ignore_ascii_case("delete") {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "SQLite refused standalone DELETE journal mode for {}",
                destination.display()
            )));
        }

        let transaction = exported
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| database_error(destination, error))?;
        transaction
            .execute("DELETE FROM project_observations", [])
            .map_err(|error| database_error(destination, error))?;
        transaction
            .execute("DELETE FROM working_copy_locators", [])
            .map_err(|error| database_error(destination, error))?;
        transaction
            .execute("DELETE FROM source_locators", [])
            .map_err(|error| database_error(destination, error))?;
        // Current artifact capture is rebuildable machine-local state. Portable bundles preserve
        // exact durably cited evidence through their dedicated evidence root; exporting the current
        // snapshot or its local write-authority marker would falsely imply that the restored machine
        // already has a complete current artifact capture.
        transaction
            .execute("DELETE FROM artifact_write_authority", [])
            .map_err(|error| database_error(destination, error))?;
        transaction
            .execute("DELETE FROM project_artifact_state", [])
            .map_err(|error| database_error(destination, error))?;
        transaction
            .execute("DELETE FROM artifact_snapshots", [])
            .map_err(|error| database_error(destination, error))?;
        transaction
            .execute(
                "UPDATE local_migration_state SET project_catalog_migrated = 1 WHERE singleton = 1",
                [],
            )
            .map_err(|error| database_error(destination, error))?;
        transaction
            .execute("DELETE FROM projects WHERE project_id <> ?1", [project_id])
            .map_err(|error| database_error(destination, error))?;
        transaction
            .execute(
                "DELETE FROM project_lifecycle WHERE project_id <> ?1",
                [project_id],
            )
            .map_err(|error| database_error(destination, error))?;
        transaction
            .execute(
                "UPDATE projects
                 SET agent_egress_policy = 'never-send', agent_egress_policy_migrated = 1
                 WHERE project_id = ?1",
                [project_id],
            )
            .map_err(|error| database_error(destination, error))?;
        transaction
            .commit()
            .map_err(|error| database_error(destination, error))?;
        exported
            .execute_batch("VACUUM;")
            .map_err(|error| database_error(destination, error))?;
        validate_exported_project_database(&exported, destination, project_id)?;
        drop(exported);
        ensure_no_sqlite_sidecars(destination)?;

        let file = fs::OpenOptions::new()
            .write(true)
            .open(destination)
            .map_err(|source| LeyCoreError::Io {
                path: destination.to_path_buf(),
                source,
            })?;
        file.sync_all().map_err(|source| LeyCoreError::Io {
            path: destination.to_path_buf(),
            source,
        })
    }

    pub(crate) fn open_connection(&self) -> Result<Connection, LeyCoreError> {
        prepare_private_database_file(&self.path)?;
        let mut connection = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|error| self.database_error(error))?;
        configure_connection(&connection, &self.path)?;
        migrate(&mut connection, &self.path)?;
        Ok(connection)
    }

    pub(crate) fn database_error(&self, error: rusqlite::Error) -> LeyCoreError {
        database_error(&self.path, error)
    }
}

#[derive(Debug, Clone)]
struct SessionErasurePlan {
    session_event_count: usize,
    dependent_subject_ids: BTreeSet<String>,
    event_ids: BTreeSet<String>,
}

fn session_erasure_plan_on(
    connection: &Connection,
    project_id: &str,
    session_id: &str,
    path: &Path,
) -> Result<Option<SessionErasurePlan>, LeyCoreError> {
    let session_event_ids = {
        let mut statement = connection
            .prepare(
                "SELECT event_id FROM events WHERE project_id = ?1 AND session_id = ?2 ORDER BY event_id",
            )
            .map_err(|error| database_error(path, error))?;
        let rows = statement
            .query_map(params![project_id, session_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|error| database_error(path, error))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| database_error(path, error))?
    };
    if session_event_ids.is_empty() {
        return Ok(None);
    }

    let mut dependent_subject_ids = {
        let mut statement = connection
            .prepare(
                "SELECT DISTINCT source.subject_id
                 FROM event_links link
                 JOIN events source
                   ON source.project_id = link.project_id
                  AND source.event_id = link.from_event_id
                 JOIN events target
                   ON target.project_id = link.project_id
                  AND target.event_id = link.to_event_id
                 WHERE link.project_id = ?1
                   AND link.relation = 'depends-on-session'
                   AND target.session_id = ?2
                   AND source.subject_id IS NOT NULL
                 ORDER BY source.subject_id",
            )
            .map_err(|error| database_error(path, error))?;
        let rows = statement
            .query_map(params![project_id, session_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|error| database_error(path, error))?;
        rows.collect::<Result<BTreeSet<_>, _>>()
            .map_err(|error| database_error(path, error))?
    };

    let supersession_predecessors = {
        let mut statement = connection
            .prepare(
                "SELECT DISTINCT source.subject_id, target.subject_id
                 FROM event_links link
                 JOIN events source
                   ON source.project_id = link.project_id
                  AND source.event_id = link.from_event_id
                 JOIN events target
                   ON target.project_id = link.project_id
                  AND target.event_id = link.to_event_id
                 WHERE link.project_id = ?1
                   AND link.relation = 'supersedes'
                   AND source.subject_id IS NOT NULL
                   AND target.subject_id IS NOT NULL",
            )
            .map_err(|error| database_error(path, error))?;
        let rows = statement
            .query_map([project_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| database_error(path, error))?;
        let mut predecessors = BTreeMap::<String, BTreeSet<String>>::new();
        for row in rows {
            let (source, target) = row.map_err(|error| database_error(path, error))?;
            predecessors.entry(target).or_default().insert(source);
        }
        predecessors
    };

    let mut frontier = dependent_subject_ids.iter().cloned().collect::<Vec<_>>();
    while let Some(target) = frontier.pop() {
        if let Some(predecessors) = supersession_predecessors.get(&target) {
            for predecessor in predecessors {
                if dependent_subject_ids.insert(predecessor.clone()) {
                    frontier.push(predecessor.clone());
                }
            }
        }
    }

    let mut event_ids = session_event_ids.iter().cloned().collect::<BTreeSet<_>>();
    for subject_id in &dependent_subject_ids {
        let mut statement = connection
            .prepare(
                "SELECT event_id FROM events WHERE project_id = ?1 AND subject_id = ?2 ORDER BY event_id",
            )
            .map_err(|error| database_error(path, error))?;
        let rows = statement
            .query_map(params![project_id, subject_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|error| database_error(path, error))?;
        for event_id in rows {
            event_ids.insert(event_id.map_err(|error| database_error(path, error))?);
        }
    }
    Ok(Some(SessionErasurePlan {
        session_event_count: session_event_ids.len(),
        dependent_subject_ids,
        event_ids,
    }))
}

fn session_erasure_preview(
    project_id: &str,
    session_id: &str,
    plan: &SessionErasurePlan,
) -> Result<ContinuitySessionErasurePreview, LeyCoreError> {
    let dependent_subject_ids = plan
        .dependent_subject_ids
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let event_ids = plan.event_ids.iter().cloned().collect::<Vec<_>>();
    let material = serde_json::to_vec(&json!({
        "version": 1,
        "projectId": project_id,
        "sessionId": session_id,
        "eventIds": event_ids,
        "dependentSubjectIds": dependent_subject_ids,
    }))
    .map_err(|error| {
        LeyCoreError::InvalidContinuityStore(format!(
            "session erasure preview could not be serialized: {error}"
        ))
    })?;
    Ok(ContinuitySessionErasurePreview {
        project_id: project_id.to_owned(),
        session_id: session_id.to_owned(),
        session_event_count: plan.session_event_count,
        dependent_subject_ids,
        total_event_count: plan.event_ids.len(),
        confirmation_digest: format!("sha256:{:x}", Sha256::digest(material)),
    })
}

fn truncate_wal_after_erasure(connection: &Connection, path: &Path) -> Result<(), LeyCoreError> {
    let (busy, _, _): (i64, i64, i64) = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|error| database_error(path, error))?;
    if busy != 0 {
        return Err(LeyCoreError::InvalidContinuityStore(
            "SQLite could not truncate the continuity WAL because another reader is active; retry erasure after the reader closes"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_exported_project_database(
    connection: &Connection,
    path: &Path,
    project_id: &str,
) -> Result<(), LeyCoreError> {
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| database_error(path, error))?;
    if integrity != "ok" {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "portable continuity database failed integrity_check: {integrity}"
        )));
    }
    let mut foreign_keys = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|error| database_error(path, error))?;
    if foreign_keys
        .query([])
        .map_err(|error| database_error(path, error))?
        .next()
        .map_err(|error| database_error(path, error))?
        .is_some()
    {
        return Err(LeyCoreError::InvalidContinuityStore(
            "portable continuity database failed foreign_key_check".to_owned(),
        ));
    }
    let schema_version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| database_error(path, error))?;
    if !(PORTABLE_DATABASE_SCHEMA_FLOOR..=CONTINUITY_SCHEMA_VERSION).contains(&schema_version) {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "portable continuity database schema {schema_version} is outside supported range {PORTABLE_DATABASE_SCHEMA_FLOOR}..={CONTINUITY_SCHEMA_VERSION}"
        )));
    }
    let project_count: i64 = connection
        .query_row("SELECT count(*) FROM projects", [], |row| row.get(0))
        .map_err(|error| database_error(path, error))?;
    let selected_count: i64 = connection
        .query_row(
            "SELECT count(*) FROM projects WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .map_err(|error| database_error(path, error))?;
    if project_count != 1 || selected_count != 1 {
        return Err(LeyCoreError::InvalidContinuityStore(
            "portable continuity database must contain exactly the selected project".to_owned(),
        ));
    }
    if schema_version >= 6 {
        validate_portable_approved_sources(connection, path, project_id)?;
    }
    if schema_version >= 8 {
        for table in [
            "artifact_snapshots",
            "artifact_files",
            "project_artifact_state",
        ] {
            let rows: i64 = connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .map_err(|error| database_error(path, error))?;
            if rows != 0 {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "portable continuity database must not contain rebuildable current artifact state in {table}"
                )));
            }
        }
    }
    if schema_version >= 10 {
        let authorities: i64 = connection
            .query_row("SELECT count(*) FROM artifact_write_authority", [], |row| {
                row.get(0)
            })
            .map_err(|error| database_error(path, error))?;
        if authorities != 0 {
            return Err(LeyCoreError::InvalidContinuityStore(
                "portable continuity database must not contain machine-local artifact write authority"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_portable_approved_sources(
    connection: &Connection,
    path: &Path,
    project_id: &str,
) -> Result<(), LeyCoreError> {
    let marker: i64 = connection
        .query_row(
            "SELECT approved_source_authority_migrated
             FROM projects WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .map_err(|error| database_error(path, error))?;
    if !matches!(marker, 0 | 1) {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "portable project {project_id} has invalid approved-source migration marker {marker}"
        )));
    }

    let mut statement = connection
        .prepare(
            "SELECT source_id, source_kind, display_name, project_relative_path,
                    content_hash, snapshot_blob_hash, approved_at_unix_ms
             FROM approved_sources WHERE project_id = ?1 ORDER BY source_id",
        )
        .map_err(|error| database_error(path, error))?;
    let rows = statement
        .query_map([project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|error| database_error(path, error))?;
    for row in rows {
        let (source_id, source_kind, display_name, relative_path, content_hash, snapshot_hash, at) =
            row.map_err(|error| database_error(path, error))?;
        validate_approved_source_identity(
            &source_id,
            &display_name,
            &content_hash,
            u64::try_from(at).map_err(|_| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "portable approved source {source_id} has invalid approval time {at}"
                ))
            })?,
        )?;
        match source_kind.as_str() {
            "project-file" => {
                let relative_path = relative_path.ok_or_else(|| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "portable project-file approved source {source_id} has no path"
                    ))
                })?;
                let normalized =
                    crate::approved_source::validate_and_normalize_project_relative_path(
                        &relative_path,
                    )
                    .map_err(|error| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "portable project-file approved source {source_id} has invalid path: {error}"
                        ))
                    })?;
                if normalized != relative_path || snapshot_hash.is_some() {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "portable project-file approved source {source_id} has invalid path/blob state"
                    )));
                }
            }
            "imported-snapshot" => {
                if relative_path.is_some() || snapshot_hash.as_deref() != Some(&content_hash) {
                    return Err(LeyCoreError::InvalidContinuityStore(format!(
                        "portable imported approved source {source_id} has invalid path/blob state"
                    )));
                }
            }
            other => {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "portable approved source {source_id} has unsupported kind {other:?}"
                )))
            }
        }
    }

    let mut blobs = connection
        .prepare(
            "SELECT content_hash, content_bytes
             FROM approved_source_blobs WHERE project_id = ?1 ORDER BY content_hash",
        )
        .map_err(|error| database_error(path, error))?;
    let blob_rows = blobs
        .query_map([project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(|error| database_error(path, error))?;
    for row in blob_rows {
        let (content_hash, bytes) = row.map_err(|error| database_error(path, error))?;
        if bytes.len() > CONTINUITY_APPROVED_SOURCE_LIMIT_BYTES
            || std::str::from_utf8(&bytes).is_err()
            || format!("sha256:{:x}", Sha256::digest(&bytes)) != content_hash
        {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "portable approved-source blob {content_hash} failed content validation"
            )));
        }
    }
    let orphan_blobs: i64 = connection
        .query_row(
            "SELECT count(*) FROM approved_source_blobs b
             WHERE b.project_id = ?1
               AND NOT EXISTS(
                   SELECT 1 FROM approved_sources s
                   WHERE s.project_id = b.project_id
                     AND s.snapshot_blob_hash = b.content_hash
               )",
            [project_id],
            |row| row.get(0),
        )
        .map_err(|error| database_error(path, error))?;
    if orphan_blobs != 0 {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "portable project {project_id} contains {orphan_blobs} orphan approved-source blobs"
        )));
    }
    Ok(())
}

fn ensure_no_sqlite_sidecars(path: &Path) -> Result<(), LeyCoreError> {
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let sidecar = PathBuf::from(sidecar);
        match fs::symlink_metadata(&sidecar) {
            Ok(_) => {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "portable continuity database left SQLite sidecar {}",
                    sidecar.display()
                )))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(LeyCoreError::Io {
                    path: sidecar,
                    source,
                })
            }
        }
    }
    Ok(())
}

pub(crate) fn project_events_from_portable_database(
    path: &Path,
    project_id: &str,
) -> Result<Vec<ContinuityEvent>, LeyCoreError> {
    validate_project_id(project_id)?;
    let metadata = fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    validate_private_database_metadata(path, &metadata)?;
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|error| database_error(path, error))?;
    validate_exported_project_database(&connection, path, project_id)?;
    let mut statement = connection
        .prepare(
            "SELECT project_id, event_id, subject_id, session_id, session_sequence, request_id, request_fingerprint, kind, payload_version, recorded_at_unix_ms, revision_head, revision_branch, payload_json FROM events WHERE project_id = ?1 ORDER BY event_id",
        )
        .map_err(|error| database_error(path, error))?;
    let rows = statement
        .query_map([project_id], row_to_event)
        .map_err(|error| database_error(path, error))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| database_error(path, error))
}

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_approved_source_import_inventory(
    snapshots: &[ContinuityApprovedSourceSnapshotInput],
    issues: &[ContinuityApprovedSourceIssueInput],
) -> Result<(), LeyCoreError> {
    let mut source_ids = BTreeSet::new();
    for snapshot in snapshots {
        validate_approved_source_identity(
            &snapshot.source_id,
            &snapshot.display_name,
            &snapshot.content_hash,
            snapshot.approved_at_unix_ms,
        )?;
        if snapshot.content_bytes.len() > CONTINUITY_APPROVED_SOURCE_LIMIT_BYTES {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "approved source {} exceeds {} bytes",
                snapshot.source_id, CONTINUITY_APPROVED_SOURCE_LIMIT_BYTES
            )));
        }
        if std::str::from_utf8(&snapshot.content_bytes).is_err() {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "approved source {} is not UTF-8 text",
                snapshot.source_id
            )));
        }
        let actual_hash = format!("sha256:{:x}", Sha256::digest(&snapshot.content_bytes));
        if actual_hash != snapshot.content_hash {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "approved source {} content does not match its approved hash",
                snapshot.source_id
            )));
        }
        if !source_ids.insert(snapshot.source_id.as_str()) {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "approved-source migration repeats source {}",
                snapshot.source_id
            )));
        }
    }
    for issue in issues {
        validate_approved_source_identity(
            &issue.source_id,
            &issue.display_name,
            &issue.approved_content_hash,
            issue.approved_at_unix_ms,
        )?;
        if !source_ids.insert(issue.source_id.as_str()) {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "approved-source migration repeats source {}",
                issue.source_id
            )));
        }
    }
    Ok(())
}

fn validate_approved_source_identity(
    source_id: &str,
    display_name: &str,
    content_hash: &str,
    approved_at_unix_ms: u64,
) -> Result<(), LeyCoreError> {
    if source_id.is_empty()
        || source_id.len() > 128
        || source_id.chars().any(|character| {
            character.is_control() || character.is_whitespace() || !character.is_ascii()
        })
    {
        return Err(LeyCoreError::InvalidContinuityStore(
            "approved source ID must contain 1 to 128 non-whitespace ASCII characters".to_owned(),
        ));
    }
    if display_name.trim().is_empty()
        || display_name.chars().count() > 1_024
        || display_name.chars().any(char::is_control)
    {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "approved source {source_id} has an invalid display name"
        )));
    }
    if !is_sha256_digest(content_hash) {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "approved source {source_id} has an invalid content hash"
        )));
    }
    if approved_at_unix_ms == 0 || i64::try_from(approved_at_unix_ms).is_err() {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "approved source {source_id} has an invalid approval time"
        )));
    }
    Ok(())
}

fn continuity_egress_policy(value: &str) -> Result<crate::AgentEgressPolicy, LeyCoreError> {
    crate::AgentEgressPolicy::parse(value).map_err(|_| {
        LeyCoreError::InvalidContinuityStore(format!(
            "continuity database contains unsupported agent egress policy {value:?}"
        ))
    })
}

fn validate_project_observation(
    observation: &ContinuityProjectObservation,
) -> Result<(), LeyCoreError> {
    validate_project_id(&observation.project_id)?;
    let root = observation
        .root_path
        .to_str()
        .ok_or_else(|| LeyCoreError::NonUtf8Path(observation.root_path.clone()))?;
    if root.is_empty() || !observation.root_path.is_absolute() {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "project observation root for {} must be an absolute UTF-8 path",
            observation.project_id
        )));
    }
    if observation.root_path.components().any(|component| {
        matches!(
            component,
            std::path::Component::CurDir | std::path::Component::ParentDir
        )
    }) {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "project observation root for {} must be normalized",
            observation.project_id
        )));
    }
    Ok(())
}

fn validate_project_observation_set(
    observations: &[ContinuityProjectObservation],
) -> Result<(), LeyCoreError> {
    let mut project_ids = BTreeSet::new();
    let mut root_paths = BTreeSet::new();
    for observation in observations {
        validate_project_observation(observation)?;
        if !project_ids.insert(observation.project_id.clone()) {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "legacy project catalog repeats project {}",
                observation.project_id
            )));
        }
        let root = observation
            .root_path
            .to_str()
            .expect("validated UTF-8 project root")
            .to_owned();
        if !root_paths.insert(root.clone()) {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "legacy project catalog repeats root path {root:?}"
            )));
        }
    }
    Ok(())
}

fn replace_project_observations_on(
    transaction: &rusqlite::Transaction<'_>,
    observations: &[ContinuityProjectObservation],
    path: &Path,
) -> Result<(), LeyCoreError> {
    let store = ContinuityStore::at(path.to_path_buf());
    for observation in observations {
        crate::project_brain::ensure_project_not_terminal_on(
            transaction,
            &observation.project_id,
            &store,
        )?;
    }
    transaction
        .execute("DELETE FROM project_observations", [])
        .map_err(|error| database_error(path, error))?;
    let store = ContinuityStore::at(path.to_path_buf());
    for observation in observations {
        match crate::project_brain::lifecycle_on(transaction, &observation.project_id, &store)? {
            Some(crate::project_brain::ProjectLifecycle {
                state:
                    crate::project_brain::ProjectLifecycleState::Erasing
                    | crate::project_brain::ProjectLifecycleState::Erased,
                ..
            }) => continue,
            _ => {}
        }
        transaction
            .execute(
                "INSERT INTO project_observations(
                    project_id, root_path, last_opened_at_unix_ms, continuity_origin
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    observation.project_id,
                    observation
                        .root_path
                        .to_str()
                        .expect("validated UTF-8 project root"),
                    u64_to_i64(
                        observation.last_opened_at_unix_ms,
                        "project last_opened_at_unix_ms"
                    )?,
                    observation.continuity_origin.as_str()
                ],
            )
            .map_err(|error| database_error(path, error))?;
    }
    Ok(())
}

fn register_project_on(
    connection: &Connection,
    identity: &ProjectIdentity,
    path: &Path,
) -> Result<ContinuityWrite<ProjectIdentity>, LeyCoreError> {
    validate_identity(identity)?;
    let store = ContinuityStore::at(path.to_path_buf());
    crate::project_brain::ensure_project_not_terminal_on(connection, &identity.project_id, &store)?;
    let changed = connection
        .execute(
            "INSERT INTO projects(project_id, name, created_at_unix_ms) VALUES (?1, ?2, ?3) ON CONFLICT(project_id) DO NOTHING",
            params![
                identity.project_id,
                identity.name,
                u64_to_i64(identity.created_at_unix_ms, "project created_at_unix_ms")?
            ],
        )
        .map_err(|error| database_error(path, error))?;
    if changed == 1 {
        crate::project_brain::ensure_registered_project_lifecycle_on(connection, identity, &store)?;
        return Ok(ContinuityWrite {
            record: identity.clone(),
            created: true,
        });
    }

    let existing = read_project(connection, &identity.project_id, path)?.ok_or_else(|| {
        LeyCoreError::InvalidContinuityStore(format!(
            "project {} disappeared during registration",
            identity.project_id
        ))
    })?;
    if existing.created_at_unix_ms != identity.created_at_unix_ms {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "project {} changed its creation timestamp",
            identity.project_id
        )));
    }
    if existing.name != identity.name {
        connection
            .execute(
                "UPDATE projects SET name = ?1 WHERE project_id = ?2",
                params![identity.name, identity.project_id],
            )
            .map_err(|error| database_error(path, error))?;
    }
    crate::project_brain::ensure_registered_project_lifecycle_on(connection, identity, &store)?;
    Ok(ContinuityWrite {
        record: identity.clone(),
        created: false,
    })
}

pub(crate) fn append_event_on(
    connection: &Connection,
    input: &ContinuityEventInput,
    path: &Path,
) -> Result<ContinuityWrite<ContinuityEvent>, LeyCoreError> {
    let store = ContinuityStore::at(path.to_path_buf());
    crate::project_brain::ensure_project_not_terminal_on(connection, &input.project_id, &store)?;
    if crate::project_brain::is_reserved_project_brain_event_kind(&input.kind) {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "Project Brain event kind {:?} requires its specialized write path",
            input.kind
        )));
    }
    append_event_on_inner(connection, input, path)
}

pub(crate) fn append_project_brain_event_on(
    connection: &Connection,
    input: &ContinuityEventInput,
    path: &Path,
) -> Result<ContinuityWrite<ContinuityEvent>, LeyCoreError> {
    if !crate::project_brain::is_reserved_project_brain_event_kind(&input.kind) {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "non-Project-Brain event kind {:?} cannot use the reserved write path",
            input.kind
        )));
    }
    append_event_on_inner(connection, input, path)
}

fn append_event_on_inner(
    connection: &Connection,
    input: &ContinuityEventInput,
    path: &Path,
) -> Result<ContinuityWrite<ContinuityEvent>, LeyCoreError> {
    validate_event_input(input)?;
    let store = ContinuityStore::at(path.to_path_buf());
    crate::project_brain::ensure_project_not_terminal_on(connection, &input.project_id, &store)?;
    crate::project_brain::ensure_event_references_not_erased_on(connection, input, &store)?;
    let payload_json = serde_json::to_string(&input.payload).map_err(|error| {
        LeyCoreError::InvalidContinuityStore(format!("event payload is not serializable: {error}"))
    })?;
    if payload_json.len() > CONTINUITY_EVENT_LIMIT_BYTES {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "event payload exceeds {CONTINUITY_EVENT_LIMIT_BYTES} bytes"
        )));
    }
    if let (Some(request_id), Some(request_fingerprint)) =
        (&input.request_id, &input.request_fingerprint)
    {
        if let Some(existing) = read_event_by_request(
            connection,
            &input.project_id,
            input.session_id.as_deref(),
            request_id,
            path,
        )? {
            if existing.request_fingerprint.as_deref() != Some(request_fingerprint.as_str())
                || !retry_event_matches(&existing, input)
            {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "request {request_id} was reused with different event content"
                )));
            }
            return Ok(ContinuityWrite {
                record: existing,
                created: false,
            });
        }
    }
    let changed = connection
        .execute(
            "INSERT INTO events(project_id, event_id, subject_id, session_id, session_sequence, request_id, request_fingerprint, kind, payload_version, recorded_at_unix_ms, revision_head, revision_branch, payload_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) ON CONFLICT DO NOTHING",
            params![
                input.project_id,
                input.event_id,
                input.subject_id,
                input.session_id,
                optional_u64_to_i64(input.session_sequence, "session sequence")?,
                input.request_id,
                input.request_fingerprint,
                input.kind,
                i64::from(input.payload_version),
                u64_to_i64(input.recorded_at_unix_ms, "event recorded_at_unix_ms")?,
                input.revision_head,
                input.revision_branch,
                payload_json,
            ],
        )
        .map_err(|error| database_error(path, error))?;
    let expected = event_from_input(input.clone());
    if changed == 1 {
        return Ok(ContinuityWrite {
            record: expected,
            created: true,
        });
    }
    if let (Some(request_id), Some(request_fingerprint)) =
        (&input.request_id, &input.request_fingerprint)
    {
        if let Some(existing) = read_event_by_request(
            connection,
            &input.project_id,
            input.session_id.as_deref(),
            request_id,
            path,
        )? {
            if existing.request_fingerprint.as_deref() == Some(request_fingerprint.as_str())
                && retry_event_matches(&existing, input)
            {
                return Ok(ContinuityWrite {
                    record: existing,
                    created: false,
                });
            }
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "request {request_id} was reused with different event content"
            )));
        }
    }
    let existing =
        read_event(connection, &input.project_id, &input.event_id, path)?.ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(format!(
                "event {} conflicted with another continuity constraint",
                input.event_id
            ))
        })?;
    if existing != expected {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "event {} was reused with different content",
            input.event_id
        )));
    }
    Ok(ContinuityWrite {
        record: existing,
        created: false,
    })
}

fn link_events_on(
    connection: &Connection,
    project_id: &str,
    from_event_id: &str,
    to_event_id: &str,
    relation: &str,
    path: &Path,
) -> Result<bool, LeyCoreError> {
    validate_project_id(project_id)?;
    validate_identifier(from_event_id, "source event ID")?;
    validate_identifier(to_event_id, "target event ID")?;
    validate_kind(relation, "event relation")?;
    if from_event_id == to_event_id {
        return Err(LeyCoreError::InvalidContinuityStore(
            "an event cannot link to itself".to_owned(),
        ));
    }
    let changed = connection
        .execute(
            "INSERT INTO event_links(project_id, from_event_id, to_event_id, relation) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(project_id, from_event_id, to_event_id, relation) DO NOTHING",
            params![project_id, from_event_id, to_event_id, relation],
        )
        .map_err(|error| database_error(path, error))?;
    Ok(changed == 1)
}

fn configure_connection(connection: &Connection, path: &Path) -> Result<(), LeyCoreError> {
    connection
        .busy_timeout(SQLITE_BUSY_TIMEOUT)
        .map_err(|error| database_error(path, error))?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| database_error(path, error))?;
    connection
        .pragma_update(None, "trusted_schema", "OFF")
        .map_err(|error| database_error(path, error))?;
    connection
        .pragma_update(None, "secure_delete", "ON")
        .map_err(|error| database_error(path, error))?;
    let current_mode: String = connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(|error| database_error(path, error))?;
    if !current_mode.eq_ignore_ascii_case("wal") {
        let mode: String = connection
            .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
            .map_err(|error| database_error(path, error))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "SQLite refused WAL mode for {}",
                path.display()
            )));
        }
    }
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| database_error(path, error))?;
    Ok(())
}

fn migrate(connection: &mut Connection, path: &Path) -> Result<(), LeyCoreError> {
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| database_error(path, error))?;
    if version > CONTINUITY_SCHEMA_VERSION {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "continuity database schema {version} is newer than supported schema {CONTINUITY_SCHEMA_VERSION}"
        )));
    }
    if version == CONTINUITY_SCHEMA_VERSION {
        return Ok(());
    }

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| database_error(path, error))?;
    let mut current_version = version;
    if current_version == 0 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE projects (
                    project_id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL CHECK(length(name) BETWEEN 1 AND 128),
                    created_at_unix_ms INTEGER NOT NULL CHECK(created_at_unix_ms >= 0)
                ) STRICT;

                CREATE TABLE events (
                    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
                    event_id TEXT NOT NULL,
                    session_id TEXT,
                    session_sequence INTEGER CHECK(session_sequence IS NULL OR session_sequence >= 0),
                    request_id TEXT,
                    request_fingerprint TEXT,
                    kind TEXT NOT NULL,
                    payload_version INTEGER NOT NULL CHECK(payload_version >= 1),
                    recorded_at_unix_ms INTEGER NOT NULL CHECK(recorded_at_unix_ms > 0),
                    revision_head TEXT,
                    revision_branch TEXT,
                    payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
                    PRIMARY KEY(project_id, event_id),
                    CHECK(session_id IS NULL OR length(session_id) > 0),
                    CHECK(session_sequence IS NULL OR session_id IS NOT NULL),
                    CHECK((request_id IS NULL) = (request_fingerprint IS NULL)),
                    CHECK(request_id IS NULL OR length(request_id) > 0),
                    CHECK(request_fingerprint IS NULL OR length(request_fingerprint) > 0)
                ) STRICT;
                CREATE UNIQUE INDEX events_session_sequence_unique
                    ON events(project_id, session_id, session_sequence)
                    WHERE session_id IS NOT NULL AND session_sequence IS NOT NULL;
                CREATE UNIQUE INDEX events_request_unique
                    ON events(project_id, ifnull(session_id, ''), request_id)
                    WHERE request_id IS NOT NULL;
                CREATE INDEX events_project_time
                    ON events(project_id, recorded_at_unix_ms, event_id);
                CREATE INDEX events_session_time
                    ON events(project_id, session_id, recorded_at_unix_ms, event_id);
                CREATE INDEX events_kind_time
                    ON events(project_id, kind, recorded_at_unix_ms, event_id);
                CREATE INDEX events_revision
                    ON events(project_id, revision_head, recorded_at_unix_ms, event_id)
                    WHERE revision_head IS NOT NULL;

                CREATE TABLE event_links (
                    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
                    from_event_id TEXT NOT NULL,
                    to_event_id TEXT NOT NULL,
                    relation TEXT NOT NULL,
                    PRIMARY KEY(project_id, from_event_id, to_event_id, relation),
                    FOREIGN KEY(project_id, from_event_id)
                        REFERENCES events(project_id, event_id) ON DELETE CASCADE,
                    FOREIGN KEY(project_id, to_event_id)
                        REFERENCES events(project_id, event_id) ON DELETE CASCADE,
                    CHECK(from_event_id <> to_event_id)
                ) STRICT;
                CREATE INDEX event_links_reverse
                    ON event_links(project_id, to_event_id, relation, from_event_id);

                PRAGMA user_version = 1;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 1;
    }
    if current_version == 1 {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE events ADD COLUMN subject_id TEXT;
                UPDATE events
                   SET subject_id = json_extract(payload_json, '$.learningId')
                 WHERE subject_id IS NULL
                   AND kind LIKE 'legacy-learning-%'
                   AND json_type(payload_json, '$.learningId') = 'text';
                CREATE INDEX events_subject_time
                    ON events(project_id, subject_id, recorded_at_unix_ms, event_id)
                    WHERE subject_id IS NOT NULL;
                PRAGMA user_version = 2;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 2;
    }
    if current_version == 2 {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE projects
                    ADD COLUMN agent_egress_policy TEXT NOT NULL DEFAULT 'agent-ok'
                    CHECK(agent_egress_policy IN ('agent-ok', 'confirm-per-use', 'local-model-only', 'never-send'));
                PRAGMA user_version = 3;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 3;
    }
    if current_version == 3 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE project_observations (
                    project_id TEXT PRIMARY KEY NOT NULL,
                    root_path TEXT NOT NULL UNIQUE CHECK(length(root_path) > 0),
                    last_opened_at_unix_ms INTEGER NOT NULL CHECK(last_opened_at_unix_ms >= 0)
                ) STRICT;
                PRAGMA user_version = 4;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 4;
    }
    if current_version == 4 {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE projects
                    ADD COLUMN agent_egress_policy_migrated INTEGER NOT NULL DEFAULT 0
                    CHECK(agent_egress_policy_migrated IN (0, 1));
                PRAGMA user_version = 5;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 5;
    }
    if current_version == 5 {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE projects
                    ADD COLUMN approved_source_authority_migrated INTEGER NOT NULL DEFAULT 0
                    CHECK(approved_source_authority_migrated IN (0, 1));

                CREATE TABLE approved_source_blobs (
                    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
                    content_hash TEXT NOT NULL
                        CHECK(length(content_hash) = 71 AND substr(content_hash, 1, 7) = 'sha256:'),
                    content_bytes BLOB NOT NULL CHECK(length(content_bytes) <= 1048576),
                    PRIMARY KEY(project_id, content_hash)
                ) STRICT;

                CREATE TABLE approved_sources (
                    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
                    source_id TEXT NOT NULL CHECK(length(source_id) > 0),
                    source_kind TEXT NOT NULL
                        CHECK(source_kind IN ('project-file', 'imported-snapshot')),
                    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 1024),
                    project_relative_path TEXT,
                    content_hash TEXT NOT NULL
                        CHECK(length(content_hash) = 71 AND substr(content_hash, 1, 7) = 'sha256:'),
                    snapshot_blob_hash TEXT,
                    approved_at_unix_ms INTEGER NOT NULL CHECK(approved_at_unix_ms > 0),
                    PRIMARY KEY(project_id, source_id),
                    FOREIGN KEY(project_id, snapshot_blob_hash)
                        REFERENCES approved_source_blobs(project_id, content_hash) ON DELETE RESTRICT,
                    CHECK(
                        (source_kind = 'project-file'
                            AND project_relative_path IS NOT NULL
                            AND length(project_relative_path) > 0
                            AND snapshot_blob_hash IS NULL)
                        OR
                        (source_kind = 'imported-snapshot'
                            AND project_relative_path IS NULL
                            AND snapshot_blob_hash = content_hash)
                    )
                ) STRICT;
                CREATE UNIQUE INDEX approved_sources_project_file_unique
                    ON approved_sources(project_id, project_relative_path)
                    WHERE source_kind = 'project-file';

                CREATE TABLE legacy_approved_source_issues (
                    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
                    source_id TEXT NOT NULL CHECK(length(source_id) > 0),
                    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 1024),
                    approved_content_hash TEXT NOT NULL
                        CHECK(length(approved_content_hash) = 71 AND substr(approved_content_hash, 1, 7) = 'sha256:'),
                    approved_at_unix_ms INTEGER NOT NULL CHECK(approved_at_unix_ms > 0),
                    reason TEXT NOT NULL CHECK(reason IN ('changed', 'missing', 'invalid')),
                    PRIMARY KEY(project_id, source_id)
                ) STRICT;
                PRAGMA user_version = 6;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 6;
    }
    if current_version == 6 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE local_migration_state (
                    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                    project_catalog_migrated INTEGER NOT NULL DEFAULT 0
                        CHECK(project_catalog_migrated IN (0, 1))
                ) STRICT;
                INSERT INTO local_migration_state(singleton, project_catalog_migrated)
                    VALUES (1, 0);
                PRAGMA user_version = 7;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 7;
    }
    if current_version == 7 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE artifact_snapshots (
                    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
                    snapshot_id TEXT NOT NULL
                        CHECK(length(snapshot_id) = 68 AND substr(snapshot_id, 1, 4) = 'snp_'),
                    project_name TEXT NOT NULL CHECK(length(project_name) BETWEEN 1 AND 128),
                    generated_at_unix_ms INTEGER NOT NULL CHECK(generated_at_unix_ms > 0),
                    capture_mode TEXT NOT NULL
                        CHECK(capture_mode IN ('minimal', 'structured', 'full-evidence')),
                    capture_fingerprint TEXT NOT NULL
                        CHECK(length(capture_fingerprint) = 71 AND substr(capture_fingerprint, 1, 7) = 'sha256:'),
                    capture_policy_json TEXT NOT NULL CHECK(json_valid(capture_policy_json)),
                    skipped_json TEXT NOT NULL CHECK(json_valid(skipped_json)),
                    PRIMARY KEY(project_id, snapshot_id)
                ) STRICT;

                CREATE TABLE artifact_files (
                    project_id TEXT NOT NULL,
                    snapshot_id TEXT NOT NULL,
                    artifact_path TEXT NOT NULL CHECK(length(artifact_path) > 0),
                    kind TEXT NOT NULL
                        CHECK(kind IN ('source', 'documentation', 'manifest', 'configuration', 'text', 'image')),
                    language TEXT,
                    media_type TEXT CHECK(media_type IS NULL OR media_type IN ('png', 'jpeg', 'webp')),
                    source_bytes INTEGER NOT NULL CHECK(source_bytes >= 0),
                    stored_bytes INTEGER NOT NULL CHECK(stored_bytes >= 0),
                    line_count INTEGER NOT NULL CHECK(line_count >= 0),
                    content_hash TEXT NOT NULL
                        CHECK(length(content_hash) = 71 AND substr(content_hash, 1, 7) = 'sha256:'),
                    content_captured INTEGER NOT NULL CHECK(content_captured IN (0, 1)),
                    redactions_json TEXT NOT NULL CHECK(json_valid(redactions_json)),
                    PRIMARY KEY(project_id, snapshot_id, artifact_path),
                    FOREIGN KEY(project_id, snapshot_id)
                        REFERENCES artifact_snapshots(project_id, snapshot_id) ON DELETE CASCADE
                ) STRICT;
                CREATE INDEX artifact_files_content_hash
                    ON artifact_files(project_id, content_hash, snapshot_id, artifact_path);

                CREATE TABLE project_artifact_state (
                    project_id TEXT PRIMARY KEY NOT NULL
                        REFERENCES projects(project_id) ON DELETE CASCADE,
                    current_snapshot_id TEXT NOT NULL,
                    legacy_graph_snapshot_id TEXT
                        CHECK(
                            legacy_graph_snapshot_id IS NULL
                            OR (length(legacy_graph_snapshot_id) = 68
                                AND substr(legacy_graph_snapshot_id, 1, 4) = 'grf_')
                        ),
                    captured_git_json TEXT
                        CHECK(captured_git_json IS NULL OR json_valid(captured_git_json)),
                    FOREIGN KEY(project_id, current_snapshot_id)
                        REFERENCES artifact_snapshots(project_id, snapshot_id) ON DELETE RESTRICT
                ) STRICT;

                PRAGMA user_version = 8;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 8;
    }
    if current_version == 8 {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE project_artifact_state
                    ADD COLUMN captured_at_unix_ms INTEGER NOT NULL DEFAULT 0
                    CHECK(captured_at_unix_ms >= 0);
                UPDATE project_artifact_state
                   SET captured_at_unix_ms = (
                       SELECT generated_at_unix_ms
                       FROM artifact_snapshots s
                       WHERE s.project_id = project_artifact_state.project_id
                         AND s.snapshot_id = project_artifact_state.current_snapshot_id
                   );

                CREATE TABLE artifact_authority_cutovers (
                    project_id TEXT PRIMARY KEY NOT NULL
                        REFERENCES projects(project_id) ON DELETE CASCADE,
                    legacy_snapshot_id TEXT NOT NULL
                        CHECK(length(legacy_snapshot_id) = 68 AND substr(legacy_snapshot_id, 1, 4) = 'snp_'),
                    legacy_capture_fingerprint TEXT NOT NULL
                        CHECK(length(legacy_capture_fingerprint) = 71 AND substr(legacy_capture_fingerprint, 1, 7) = 'sha256:'),
                    legacy_file_count INTEGER NOT NULL CHECK(legacy_file_count >= 0),
                    recorded_at_unix_ms INTEGER NOT NULL CHECK(recorded_at_unix_ms > 0)
                ) STRICT;

                PRAGMA user_version = 9;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 9;
    }
    if current_version == 9 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE artifact_write_authority (
                    project_id TEXT PRIMARY KEY NOT NULL
                        REFERENCES projects(project_id) ON DELETE CASCADE,
                    origin TEXT NOT NULL
                        CHECK(origin IN ('legacy-cutover', 'native-born')),
                    initial_snapshot_id TEXT NOT NULL
                        CHECK(length(initial_snapshot_id) = 68 AND substr(initial_snapshot_id, 1, 4) = 'snp_'),
                    initial_capture_fingerprint TEXT NOT NULL
                        CHECK(length(initial_capture_fingerprint) = 71 AND substr(initial_capture_fingerprint, 1, 7) = 'sha256:'),
                    initial_file_count INTEGER NOT NULL CHECK(initial_file_count >= 0),
                    recorded_at_unix_ms INTEGER NOT NULL CHECK(recorded_at_unix_ms > 0)
                ) STRICT;

                INSERT INTO artifact_write_authority(
                    project_id, origin, initial_snapshot_id, initial_capture_fingerprint,
                    initial_file_count, recorded_at_unix_ms
                )
                SELECT project_id, 'legacy-cutover', legacy_snapshot_id,
                       legacy_capture_fingerprint, legacy_file_count, recorded_at_unix_ms
                FROM artifact_authority_cutovers;

                DROP TABLE artifact_authority_cutovers;
                PRAGMA user_version = 10;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 10;
    }
    if current_version == 10 {
        transaction
            .execute_batch(
                r#"
                ALTER TABLE project_observations
                    ADD COLUMN continuity_origin TEXT NOT NULL DEFAULT 'legacy-unknown'
                    CHECK(continuity_origin IN ('legacy-unknown', 'native-born'));
                PRAGMA user_version = 11;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
        current_version = 11;
    }
    if current_version == 11 {
        transaction
            .execute_batch(
                r#"
                CREATE TABLE project_lifecycle (
                    project_id TEXT PRIMARY KEY NOT NULL,
                    generation INTEGER NOT NULL CHECK(generation >= 1),
                    state TEXT NOT NULL CHECK(state IN ('active', 'erasing', 'erased')),
                    updated_at_unix_ms INTEGER NOT NULL CHECK(updated_at_unix_ms >= 0),
                    create_request_id TEXT UNIQUE,
                    create_request_fingerprint TEXT,
                    cleanup_kind TEXT NOT NULL DEFAULT 'none'
                        CHECK(cleanup_kind IN ('none', 'native', 'legacy-and-native')),
                    cleanup_json TEXT CHECK(cleanup_json IS NULL OR json_valid(cleanup_json)),
                    CHECK((create_request_id IS NULL) = (create_request_fingerprint IS NULL)),
                    CHECK(
                        (state = 'active' AND cleanup_kind = 'none' AND cleanup_json IS NULL)
                        OR state IN ('erasing', 'erased')
                    )
                ) STRICT;
                INSERT INTO project_lifecycle(
                    project_id, generation, state, updated_at_unix_ms,
                    create_request_id, create_request_fingerprint, cleanup_kind, cleanup_json
                )
                SELECT project_id, 1, 'active', created_at_unix_ms,
                       NULL, NULL, 'none', NULL
                FROM projects;

                CREATE TRIGGER projects_reject_terminal_lifecycle
                BEFORE INSERT ON projects
                WHEN EXISTS(
                    SELECT 1 FROM project_lifecycle
                    WHERE project_id = NEW.project_id AND state <> 'active'
                )
                BEGIN
                    SELECT RAISE(ABORT, 'project lifecycle is not active');
                END;

                CREATE TABLE project_repositories (
                    project_id TEXT PRIMARY KEY NOT NULL
                        REFERENCES projects(project_id) ON DELETE CASCADE,
                    repository_id TEXT NOT NULL UNIQUE CHECK(length(repository_id) > 0),
                    attached_at_unix_ms INTEGER NOT NULL CHECK(attached_at_unix_ms > 0),
                    UNIQUE(project_id, repository_id)
                ) STRICT;

                CREATE TABLE working_copy_locators (
                    project_id TEXT NOT NULL,
                    repository_id TEXT NOT NULL,
                    locator_id TEXT NOT NULL CHECK(length(locator_id) > 0),
                    local_path TEXT NOT NULL CHECK(length(local_path) > 0),
                    state TEXT NOT NULL CHECK(state IN ('authorized', 'revoked')),
                    authorized_at_unix_ms INTEGER NOT NULL CHECK(authorized_at_unix_ms > 0),
                    revoked_at_unix_ms INTEGER CHECK(revoked_at_unix_ms IS NULL OR revoked_at_unix_ms > 0),
                    last_observed_at_unix_ms INTEGER NOT NULL CHECK(last_observed_at_unix_ms > 0),
                    revision_head TEXT,
                    revision_branch TEXT,
                    PRIMARY KEY(project_id, locator_id),
                    FOREIGN KEY(project_id, repository_id)
                        REFERENCES project_repositories(project_id, repository_id) ON DELETE CASCADE,
                    CHECK(
                        (state = 'authorized' AND revoked_at_unix_ms IS NULL)
                        OR (state = 'revoked' AND revoked_at_unix_ms IS NOT NULL)
                    )
                ) STRICT;
                CREATE UNIQUE INDEX working_copy_active_path_unique
                    ON working_copy_locators(local_path)
                    WHERE state = 'authorized';
                CREATE INDEX working_copy_project_state
                    ON working_copy_locators(project_id, state, last_observed_at_unix_ms);

                CREATE TABLE project_sources (
                    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
                    source_id TEXT NOT NULL CHECK(length(source_id) > 0),
                    source_kind TEXT NOT NULL CHECK(length(source_kind) > 0),
                    display_name TEXT NOT NULL CHECK(length(display_name) BETWEEN 1 AND 1024),
                    state TEXT NOT NULL CHECK(state IN ('active', 'removed', 'erased')),
                    created_at_unix_ms INTEGER NOT NULL CHECK(created_at_unix_ms > 0),
                    removed_at_unix_ms INTEGER CHECK(removed_at_unix_ms IS NULL OR removed_at_unix_ms > 0),
                    erased_at_unix_ms INTEGER CHECK(erased_at_unix_ms IS NULL OR erased_at_unix_ms > 0),
                    PRIMARY KEY(project_id, source_id),
                    CHECK(
                        (state = 'active' AND removed_at_unix_ms IS NULL AND erased_at_unix_ms IS NULL)
                        OR (state = 'removed' AND removed_at_unix_ms IS NOT NULL AND erased_at_unix_ms IS NULL)
                        OR (state = 'erased' AND erased_at_unix_ms IS NOT NULL)
                    )
                ) STRICT;
                CREATE INDEX project_sources_state
                    ON project_sources(project_id, state, created_at_unix_ms, source_id);

                CREATE TABLE source_versions (
                    project_id TEXT NOT NULL,
                    source_id TEXT NOT NULL,
                    source_version_id TEXT NOT NULL CHECK(length(source_version_id) > 0),
                    representation_kind TEXT NOT NULL CHECK(length(representation_kind) > 0),
                    content_hash TEXT NOT NULL
                        CHECK(length(content_hash) = 71 AND substr(content_hash, 1, 7) = 'sha256:'),
                    original_content_hash TEXT
                        CHECK(original_content_hash IS NULL OR (length(original_content_hash) = 71 AND substr(original_content_hash, 1, 7) = 'sha256:')),
                    source_bytes INTEGER NOT NULL CHECK(source_bytes >= 0),
                    stored_bytes INTEGER NOT NULL CHECK(stored_bytes >= 0),
                    transformation_json TEXT NOT NULL CHECK(json_valid(transformation_json)),
                    retained_at_unix_ms INTEGER NOT NULL CHECK(retained_at_unix_ms > 0),
                    PRIMARY KEY(project_id, source_id, source_version_id),
                    FOREIGN KEY(project_id, source_id)
                        REFERENCES project_sources(project_id, source_id) ON DELETE CASCADE
                ) STRICT;
                CREATE INDEX source_versions_content_hash
                    ON source_versions(project_id, content_hash, source_id, source_version_id);

                CREATE TABLE source_locators (
                    project_id TEXT NOT NULL,
                    source_id TEXT NOT NULL,
                    locator_id TEXT NOT NULL CHECK(length(locator_id) > 0),
                    locator_kind TEXT NOT NULL CHECK(length(locator_kind) > 0),
                    locator_value TEXT NOT NULL CHECK(length(locator_value) > 0),
                    state TEXT NOT NULL CHECK(state IN ('observed', 'revoked')),
                    observed_at_unix_ms INTEGER NOT NULL CHECK(observed_at_unix_ms > 0),
                    revoked_at_unix_ms INTEGER CHECK(revoked_at_unix_ms IS NULL OR revoked_at_unix_ms > 0),
                    PRIMARY KEY(project_id, source_id, locator_id),
                    FOREIGN KEY(project_id, source_id)
                        REFERENCES project_sources(project_id, source_id) ON DELETE CASCADE,
                    CHECK(
                        (state = 'observed' AND revoked_at_unix_ms IS NULL)
                        OR (state = 'revoked' AND revoked_at_unix_ms IS NOT NULL)
                    )
                ) STRICT;
                CREATE UNIQUE INDEX source_locator_observed_value_unique
                    ON source_locators(project_id, locator_kind, locator_value)
                    WHERE state = 'observed';

                CREATE TABLE project_sessions (
                    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
                    session_id TEXT NOT NULL CHECK(length(session_id) > 0),
                    host_kind TEXT,
                    external_session_id TEXT,
                    state TEXT NOT NULL CHECK(state IN ('active', 'finished', 'erased')),
                    started_at_unix_ms INTEGER CHECK(started_at_unix_ms IS NULL OR started_at_unix_ms > 0),
                    ended_at_unix_ms INTEGER CHECK(ended_at_unix_ms IS NULL OR ended_at_unix_ms > 0),
                    created_at_unix_ms INTEGER NOT NULL CHECK(created_at_unix_ms > 0),
                    erased_at_unix_ms INTEGER CHECK(erased_at_unix_ms IS NULL OR erased_at_unix_ms > 0),
                    observation_limits_json TEXT NOT NULL CHECK(json_valid(observation_limits_json)),
                    PRIMARY KEY(project_id, session_id),
                    CHECK(
                        (state = 'active' AND ended_at_unix_ms IS NULL AND erased_at_unix_ms IS NULL)
                        OR (state = 'finished' AND ended_at_unix_ms IS NOT NULL AND erased_at_unix_ms IS NULL)
                        OR (state = 'erased' AND erased_at_unix_ms IS NOT NULL)
                    )
                ) STRICT;
                CREATE INDEX project_sessions_state
                    ON project_sessions(project_id, state, created_at_unix_ms, session_id);

                CREATE TABLE event_source_version_links (
                    project_id TEXT NOT NULL,
                    event_id TEXT NOT NULL,
                    source_id TEXT NOT NULL,
                    source_version_id TEXT NOT NULL,
                    relation TEXT NOT NULL CHECK(length(relation) > 0),
                    selector_version INTEGER NOT NULL CHECK(selector_version >= 1),
                    selector_json TEXT NOT NULL CHECK(json_valid(selector_json)),
                    PRIMARY KEY(project_id, event_id, source_id, source_version_id, relation, selector_version),
                    FOREIGN KEY(project_id, event_id)
                        REFERENCES events(project_id, event_id) ON DELETE CASCADE,
                    FOREIGN KEY(project_id, source_id, source_version_id)
                        REFERENCES source_versions(project_id, source_id, source_version_id) ON DELETE CASCADE
                ) STRICT;
                CREATE INDEX event_source_version_links_reverse
                    ON event_source_version_links(project_id, source_id, source_version_id, relation, event_id);

                PRAGMA user_version = 12;
                "#,
            )
            .map_err(|error| database_error(path, error))?;
    }
    if version < 13 {
        transaction
            .execute_batch(crate::project_import::IMPORT_SCHEMA)
            .map_err(|error| database_error(path, error))?;
    }
    transaction
        .commit()
        .map_err(|error| database_error(path, error))?;
    Ok(())
}

fn prepare_private_database_file(path: &Path) -> Result<(), LeyCoreError> {
    let parent = path.parent().ok_or_else(|| {
        LeyCoreError::InvalidContinuityStore(
            "continuity database has no parent directory".to_owned(),
        )
    })?;
    ensure_private_directory(parent)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_private_database_metadata(path, &metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(path) {
                Ok(file) => {
                    file.sync_all().map_err(|source| LeyCoreError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
                    Ok(())
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let metadata =
                        fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
                            path: path.to_path_buf(),
                            source,
                        })?;
                    validate_private_database_metadata(path, &metadata)
                }
                Err(source) => Err(LeyCoreError::Io {
                    path: path.to_path_buf(),
                    source,
                }),
            }
        }
        Err(source) => Err(LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn ensure_private_directory(path: &Path) -> Result<(), LeyCoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_private_directory_metadata(path, &metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(path).map_err(|source| LeyCoreError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            let metadata = fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            validate_private_directory_metadata(path, &metadata)
        }
        Err(source) => Err(LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn validate_private_directory_metadata(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), LeyCoreError> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "private database directory is not a regular directory: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "private database directory must use mode 700: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn validate_private_database_metadata(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), LeyCoreError> {
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "continuity database is not a regular file: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "continuity database must use mode 600: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn validate_private_lock_path_if_present(path: &Path, label: &str) -> Result<(), LeyCoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_private_lock_metadata(path, &metadata, label),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn validate_private_lock_metadata(
    path: &Path,
    metadata: &fs::Metadata,
    label: &str,
) -> Result<(), LeyCoreError> {
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "{label} is not a regular file: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "{label} must use mode 600: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn read_project(
    connection: &Connection,
    project_id: &str,
    path: &Path,
) -> Result<Option<ProjectIdentity>, LeyCoreError> {
    connection
        .query_row(
            "SELECT project_id, name, created_at_unix_ms FROM projects WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok(ProjectIdentity {
                    schema_version: crate::PROJECT_SCHEMA_VERSION,
                    project_id: row.get(0)?,
                    name: row.get(1)?,
                    created_at_unix_ms: i64_to_u64(row.get(2)?, "project created_at_unix_ms")?,
                })
            },
        )
        .optional()
        .map_err(|error| database_error(path, error))
}

fn read_event(
    connection: &Connection,
    project_id: &str,
    event_id: &str,
    path: &Path,
) -> Result<Option<ContinuityEvent>, LeyCoreError> {
    connection
        .query_row(
            "SELECT project_id, event_id, subject_id, session_id, session_sequence, request_id, request_fingerprint, kind, payload_version, recorded_at_unix_ms, revision_head, revision_branch, payload_json FROM events WHERE project_id = ?1 AND event_id = ?2",
            params![project_id, event_id],
            row_to_event,
        )
        .optional()
        .map_err(|error| database_error(path, error))
}

fn project_events_on(
    connection: &Connection,
    project_id: &str,
    path: &Path,
) -> Result<Vec<ContinuityEvent>, LeyCoreError> {
    let mut statement = connection
        .prepare(
            "SELECT project_id, event_id, subject_id, session_id, session_sequence, request_id, request_fingerprint, kind, payload_version, recorded_at_unix_ms, revision_head, revision_branch, payload_json
             FROM events
             WHERE project_id = ?1
             ORDER BY recorded_at_unix_ms, event_id",
        )
        .map_err(|error| database_error(path, error))?;
    let rows = statement
        .query_map([project_id], row_to_event)
        .map_err(|error| database_error(path, error))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| database_error(path, error))
}

fn session_events_on(
    connection: &Connection,
    project_id: &str,
    session_id: &str,
    path: &Path,
) -> Result<Vec<ContinuityEvent>, LeyCoreError> {
    let mut statement = connection
        .prepare(
            "SELECT project_id, event_id, subject_id, session_id, session_sequence, request_id, request_fingerprint, kind, payload_version, recorded_at_unix_ms, revision_head, revision_branch, payload_json FROM events WHERE project_id = ?1 AND session_id = ?2 ORDER BY session_sequence IS NULL, session_sequence, recorded_at_unix_ms, event_id",
        )
        .map_err(|error| database_error(path, error))?;
    let rows = statement
        .query_map(params![project_id, session_id], row_to_event)
        .map_err(|error| database_error(path, error))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| database_error(path, error))
}

fn learning_events_on(
    connection: &Connection,
    project_id: &str,
    path: &Path,
) -> Result<Vec<ContinuityEvent>, LeyCoreError> {
    let mut statement = connection
        .prepare(
            "SELECT project_id, event_id, subject_id, session_id, session_sequence, request_id, request_fingerprint, kind, payload_version, recorded_at_unix_ms, revision_head, revision_branch, payload_json
             FROM events
             WHERE project_id = ?1
               AND kind IN (
                   'legacy-learning-proposed',
                   'legacy-learning-corrected',
                   'legacy-learning-reviewed',
                   'learning-proposed',
                   'learning-corrected',
                   'learning-reviewed'
               )
             ORDER BY recorded_at_unix_ms, event_id",
        )
        .map_err(|error| database_error(path, error))?;
    let rows = statement
        .query_map([project_id], row_to_event)
        .map_err(|error| database_error(path, error))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| database_error(path, error))
}

fn continuity_artifact_files(
    manifest: &ArtifactManifest,
) -> Result<Vec<ContinuityArtifactFileMetadata>, LeyCoreError> {
    let mut files = manifest
        .files
        .iter()
        .map(|artifact| {
            let redactions_json = serde_json::to_string(&artifact.redactions).map_err(|error| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "artifact redaction metadata could not be serialized: {error}"
                ))
            })?;
            Ok(ContinuityArtifactFileMetadata {
                artifact_path: artifact.path.clone(),
                kind: artifact_kind_label(artifact.kind).to_owned(),
                language: artifact.language.clone(),
                media_type: artifact
                    .media_type
                    .map(artifact_media_type_label)
                    .map(str::to_owned),
                source_bytes: artifact.source_bytes,
                stored_bytes: artifact.stored_bytes,
                line_count: artifact.line_count,
                content_hash: artifact.content_hash.clone(),
                content_captured: artifact.content_blob.is_some(),
                redactions_json,
            })
        })
        .collect::<Result<Vec<_>, LeyCoreError>>()?;
    files.sort_by(|left, right| left.artifact_path.cmp(&right.artifact_path));
    Ok(files)
}

fn validate_artifact_snapshot_matches(
    connection: &Connection,
    path: &Path,
    identity: &ProjectIdentity,
    manifest: &ArtifactManifest,
    capture_policy_json: &str,
    skipped_json: &str,
    expected_files: &[ContinuityArtifactFileMetadata],
) -> Result<(), LeyCoreError> {
    let stored = connection
        .query_row(
            "SELECT project_name, capture_mode, capture_fingerprint,
                    capture_policy_json, skipped_json
             FROM artifact_snapshots
             WHERE project_id = ?1 AND snapshot_id = ?2",
            params![identity.project_id, manifest.snapshot_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .map_err(|error| database_error(path, error))?;
    let expected = (
        manifest.project_name.clone(),
        manifest.capture_mode.to_string(),
        manifest.capture_fingerprint.clone(),
        capture_policy_json.to_owned(),
        skipped_json.to_owned(),
    );
    if stored != expected {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "artifact snapshot {} conflicts with immutable metadata already stored",
            manifest.snapshot_id
        )));
    }
    let stored_files = read_artifact_files_on(
        connection,
        &identity.project_id,
        &manifest.snapshot_id,
        path,
    )?;
    if stored_files != expected_files {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "artifact snapshot {} conflicts with immutable file metadata already stored",
            manifest.snapshot_id
        )));
    }
    Ok(())
}

fn read_artifact_files_on(
    connection: &Connection,
    project_id: &str,
    snapshot_id: &str,
    path: &Path,
) -> Result<Vec<ContinuityArtifactFileMetadata>, LeyCoreError> {
    let mut statement = connection
        .prepare(
            "SELECT artifact_path, kind, language, media_type, source_bytes, stored_bytes,
                    line_count, content_hash, content_captured, redactions_json
             FROM artifact_files
             WHERE project_id = ?1 AND snapshot_id = ?2
             ORDER BY artifact_path",
        )
        .map_err(|error| database_error(path, error))?;
    let rows = statement
        .query_map(params![project_id, snapshot_id], |row| {
            let source_bytes: i64 = row.get(4)?;
            let stored_bytes: i64 = row.get(5)?;
            let line_count: i64 = row.get(6)?;
            let retained: i64 = row.get(8)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                source_bytes,
                stored_bytes,
                line_count,
                row.get::<_, String>(7)?,
                retained,
                row.get::<_, String>(9)?,
            ))
        })
        .map_err(|error| database_error(path, error))?;
    rows.map(|row| {
        let (
            artifact_path,
            kind,
            language,
            media_type,
            source_bytes,
            stored_bytes,
            line_count,
            content_hash,
            retained,
            redactions_json,
        ) = row.map_err(|error| database_error(path, error))?;
        if retained != 0 && retained != 1 {
            return Err(LeyCoreError::InvalidContinuityStore(
                "artifact content-retained flag is invalid".to_owned(),
            ));
        }
        Ok(ContinuityArtifactFileMetadata {
            artifact_path,
            kind,
            language,
            media_type,
            source_bytes: u64::try_from(source_bytes).map_err(|_| {
                LeyCoreError::InvalidContinuityStore(
                    "artifact source byte count is invalid".to_owned(),
                )
            })?,
            stored_bytes: u64::try_from(stored_bytes).map_err(|_| {
                LeyCoreError::InvalidContinuityStore(
                    "artifact stored byte count is invalid".to_owned(),
                )
            })?,
            line_count: u64::try_from(line_count).map_err(|_| {
                LeyCoreError::InvalidContinuityStore("artifact line count is invalid".to_owned())
            })?,
            content_hash,
            content_captured: retained == 1,
            redactions_json,
        })
    })
    .collect()
}

fn artifact_kind_label(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Source => "source",
        ArtifactKind::Documentation => "documentation",
        ArtifactKind::Manifest => "manifest",
        ArtifactKind::Configuration => "configuration",
        ArtifactKind::Text => "text",
        ArtifactKind::Image => "image",
    }
}

fn parse_artifact_kind_label(value: &str) -> Result<ArtifactKind, LeyCoreError> {
    match value {
        "source" => Ok(ArtifactKind::Source),
        "documentation" => Ok(ArtifactKind::Documentation),
        "manifest" => Ok(ArtifactKind::Manifest),
        "configuration" => Ok(ArtifactKind::Configuration),
        "text" => Ok(ArtifactKind::Text),
        "image" => Ok(ArtifactKind::Image),
        _ => Err(LeyCoreError::InvalidContinuityStore(format!(
            "native artifact kind is invalid: {value:?}"
        ))),
    }
}

fn artifact_media_type_label(media_type: ArtifactMediaType) -> &'static str {
    match media_type {
        ArtifactMediaType::Png => "png",
        ArtifactMediaType::Jpeg => "jpeg",
        ArtifactMediaType::Webp => "webp",
    }
}

fn sqlite_u64(value: u64, label: &str) -> Result<i64, LeyCoreError> {
    i64::try_from(value).map_err(|_| {
        LeyCoreError::InvalidContinuityStore(format!(
            "{label} exceeds SQLite's signed integer range"
        ))
    })
}

fn artifact_i64_to_u64(value: i64, label: &str) -> Result<u64, LeyCoreError> {
    u64::try_from(value).map_err(|_| {
        LeyCoreError::InvalidContinuityStore(format!("{label} must be a non-negative integer"))
    })
}

fn artifact_digest(content_hash: &str) -> Result<&str, LeyCoreError> {
    let digest = content_hash.strip_prefix("sha256:").ok_or_else(|| {
        LeyCoreError::InvalidContinuityStore(
            "artifact content hash must use the sha256: prefix".to_owned(),
        )
    })?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(LeyCoreError::InvalidContinuityStore(
            "artifact content hash must contain a lowercase SHA-256 digest".to_owned(),
        ));
    }
    Ok(digest)
}

fn valid_artifact_snapshot_id(value: &str) -> bool {
    value.len() == 68
        && value.starts_with("snp_")
        && value[4..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_graph_snapshot_id(value: &str) -> bool {
    value.len() == 68
        && value.starts_with("grf_")
        && value[4..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn record_required_artifact_blob(
    required: &mut BTreeMap<String, u64>,
    content_hash: &str,
    stored_bytes: u64,
) -> Result<(), LeyCoreError> {
    artifact_digest(content_hash)?;
    match required.get(content_hash) {
        Some(existing) if *existing != stored_bytes => Err(LeyCoreError::InvalidContinuityStore(
            format!("artifact content hash {content_hash} has conflicting stored byte counts"),
        )),
        Some(_) => Ok(()),
        None => {
            required.insert(content_hash.to_owned(), stored_bytes);
            Ok(())
        }
    }
}

fn write_immutable_private_blob(
    project_dir: &Path,
    destination: &Path,
    bytes: &[u8],
) -> Result<(), LeyCoreError> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) => {
            validate_native_artifact_blob_metadata(destination, &metadata)?;
            let existing = read_private_blob(destination, bytes.len() as u64)?;
            if existing == bytes {
                return Ok(());
            }
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "native artifact blob collision at {}",
                destination.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LeyCoreError::Io {
                path: destination.to_path_buf(),
                source,
            })
        }
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            LeyCoreError::InvalidContinuityStore(
                "system clock is before the Unix epoch while staging artifact content".to_owned(),
            )
        })?
        .as_nanos();
    let temp_path = project_dir.join(format!(".tmp-artifact-{}-{nonce}", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut temporary = options
        .open(&temp_path)
        .map_err(|source| LeyCoreError::Io {
            path: temp_path.clone(),
            source,
        })?;
    let write_result = (|| {
        temporary
            .write_all(bytes)
            .map_err(|source| LeyCoreError::Io {
                path: temp_path.clone(),
                source,
            })?;
        temporary.sync_all().map_err(|source| LeyCoreError::Io {
            path: temp_path.clone(),
            source,
        })?;
        Ok::<(), LeyCoreError>(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    drop(temporary);

    match fs::symlink_metadata(destination) {
        Ok(metadata) => {
            validate_native_artifact_blob_metadata(destination, &metadata)?;
            let existing = read_private_blob(destination, bytes.len() as u64)?;
            let _ = fs::remove_file(&temp_path);
            if existing == bytes {
                return Ok(());
            }
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "native artifact blob collision at {}",
                destination.display()
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            let _ = fs::remove_file(&temp_path);
            return Err(LeyCoreError::Io {
                path: destination.to_path_buf(),
                source,
            });
        }
    }

    fs::rename(&temp_path, destination).map_err(|source| {
        let _ = fs::remove_file(&temp_path);
        LeyCoreError::Io {
            path: destination.to_path_buf(),
            source,
        }
    })?;
    let metadata = fs::symlink_metadata(destination).map_err(|source| LeyCoreError::Io {
        path: destination.to_path_buf(),
        source,
    })?;
    validate_native_artifact_blob_metadata(destination, &metadata)?;
    #[cfg(unix)]
    {
        File::open(project_dir)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| LeyCoreError::Io {
                path: project_dir.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn read_private_blob(path: &Path, expected_bytes: u64) -> Result<Vec<u8>, LeyCoreError> {
    let mut file = File::open(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let metadata = file.metadata().map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    validate_native_artifact_blob_metadata(path, &metadata)?;
    if metadata.len() != expected_bytes {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "native artifact blob length does not match metadata: {}",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(expected_bytes as usize);
    file.read_to_end(&mut bytes)
        .map_err(|source| LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(bytes)
}

fn validate_native_artifact_blob_metadata(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), LeyCoreError> {
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "native artifact blob is not a regular file: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "native artifact blob must use mode 600: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

pub(crate) fn read_event_by_request(
    connection: &Connection,
    project_id: &str,
    session_id: Option<&str>,
    request_id: &str,
    path: &Path,
) -> Result<Option<ContinuityEvent>, LeyCoreError> {
    connection
        .query_row(
            "SELECT project_id, event_id, subject_id, session_id, session_sequence, request_id, request_fingerprint, kind, payload_version, recorded_at_unix_ms, revision_head, revision_branch, payload_json FROM events WHERE project_id = ?1 AND request_id = ?2 AND ((session_id = ?3) OR (session_id IS NULL AND ?3 IS NULL))",
            params![project_id, request_id, session_id],
            row_to_event,
        )
        .optional()
        .map_err(|error| database_error(path, error))
}

fn row_to_approved_source(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ContinuityApprovedSourceRecord> {
    let source_kind: String = row.get(2)?;
    let source_kind = ContinuityApprovedSourceKind::parse(&source_kind).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            source_kind.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })?;
    Ok(ContinuityApprovedSourceRecord {
        project_id: row.get(0)?,
        source_id: row.get(1)?,
        source_kind,
        display_name: row.get(3)?,
        project_relative_path: row.get(4)?,
        content_hash: row.get(5)?,
        approved_at_unix_ms: i64_to_u64_sql(row.get(6)?, "approved source approval time")?,
    })
}

fn require_approved_source_authority_ready_on(
    connection: &Connection,
    project_id: &str,
    path: &Path,
) -> Result<(), LeyCoreError> {
    let migrated: Option<i64> = connection
        .query_row(
            "SELECT approved_source_authority_migrated
             FROM projects WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| database_error(path, error))?;
    match migrated {
        Some(1) => Ok(()),
        Some(0) => Err(LeyCoreError::ApprovedSourceAuthorityMigrationPending {
            project_id: project_id.to_owned(),
        }),
        Some(value) => Err(LeyCoreError::InvalidContinuityStore(format!(
            "project {project_id} has invalid approved-source migration marker {value}"
        ))),
        None => Err(LeyCoreError::InvalidContinuityStore(format!(
            "project {project_id} is not present in the continuity database"
        ))),
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContinuityEvent> {
    let payload_json: String = row.get(12)?;
    let payload = serde_json::from_str(&payload_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            payload_json.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })?;
    Ok(ContinuityEvent {
        project_id: row.get(0)?,
        event_id: row.get(1)?,
        subject_id: row.get(2)?,
        session_id: row.get(3)?,
        session_sequence: optional_i64_to_u64_sql(row.get(4)?, "session sequence")?,
        request_id: row.get(5)?,
        request_fingerprint: row.get(6)?,
        kind: row.get(7)?,
        payload_version: i64_to_u32_sql(row.get(8)?, "event payload_version")?,
        recorded_at_unix_ms: i64_to_u64_sql(row.get(9)?, "event recorded_at_unix_ms")?,
        revision_head: row.get(10)?,
        revision_branch: row.get(11)?,
        payload,
    })
}

fn validate_event_input(input: &ContinuityEventInput) -> Result<(), LeyCoreError> {
    validate_identifier(&input.event_id, "event ID")?;
    validate_project_id(&input.project_id)?;
    if let Some(subject_id) = &input.subject_id {
        validate_identifier(subject_id, "subject ID")?;
    }
    if let Some(session_id) = &input.session_id {
        validate_identifier(session_id, "session ID")?;
    }
    if input.session_sequence.is_some() && input.session_id.is_none() {
        return Err(LeyCoreError::InvalidContinuityStore(
            "session sequence requires a session ID".to_owned(),
        ));
    }
    match (&input.request_id, &input.request_fingerprint) {
        (Some(request_id), Some(fingerprint)) => {
            validate_identifier(request_id, "request ID")?;
            validate_optional_text(Some(fingerprint), "request fingerprint", 256)?;
        }
        (None, None) => {}
        _ => {
            return Err(LeyCoreError::InvalidContinuityStore(
                "request ID and request fingerprint must be supplied together".to_owned(),
            ))
        }
    }
    validate_kind(&input.kind, "event kind")?;
    if input.payload_version == 0 {
        return Err(LeyCoreError::InvalidContinuityStore(
            "event payload_version must be at least 1".to_owned(),
        ));
    }
    if input.recorded_at_unix_ms == 0 {
        return Err(LeyCoreError::InvalidContinuityStore(
            "event timestamp must be greater than zero".to_owned(),
        ));
    }
    validate_optional_text(input.revision_head.as_deref(), "revision head", 256)?;
    validate_optional_text(input.revision_branch.as_deref(), "revision branch", 512)?;
    Ok(())
}

fn validate_identifier(value: &str, label: &str) -> Result<(), LeyCoreError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
    {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "{label} must be 1-128 ASCII identifier characters"
        )));
    }
    Ok(())
}

fn validate_kind(value: &str, label: &str) -> Result<(), LeyCoreError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "{label} must be 1-64 lowercase kebab-case characters"
        )));
    }
    Ok(())
}

fn validate_optional_text(
    value: Option<&str>,
    label: &str,
    max_bytes: usize,
) -> Result<(), LeyCoreError> {
    if let Some(value) = value {
        if value.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "{label} must be non-empty, bounded text without control characters"
            )));
        }
    }
    Ok(())
}

fn event_from_input(input: ContinuityEventInput) -> ContinuityEvent {
    ContinuityEvent {
        event_id: input.event_id,
        project_id: input.project_id,
        subject_id: input.subject_id,
        session_id: input.session_id,
        session_sequence: input.session_sequence,
        request_id: input.request_id,
        request_fingerprint: input.request_fingerprint,
        kind: input.kind,
        payload_version: input.payload_version,
        recorded_at_unix_ms: input.recorded_at_unix_ms,
        revision_head: input.revision_head,
        revision_branch: input.revision_branch,
        payload: input.payload,
    }
}

fn retry_event_matches(existing: &ContinuityEvent, input: &ContinuityEventInput) -> bool {
    existing.project_id == input.project_id
        && existing.subject_id == input.subject_id
        && existing.session_id == input.session_id
        && existing.session_sequence == input.session_sequence
        && existing.request_id == input.request_id
        && existing.request_fingerprint == input.request_fingerprint
        && existing.kind == input.kind
        && existing.payload_version == input.payload_version
        && existing.revision_head == input.revision_head
        && existing.revision_branch == input.revision_branch
        && existing.payload == input.payload
}

fn u64_to_i64(value: u64, label: &str) -> Result<i64, LeyCoreError> {
    i64::try_from(value).map_err(|_| {
        LeyCoreError::InvalidContinuityStore(format!("{label} does not fit SQLite INTEGER"))
    })
}

fn optional_u64_to_i64(value: Option<u64>, label: &str) -> Result<Option<i64>, LeyCoreError> {
    value.map(|value| u64_to_i64(value, label)).transpose()
}

fn i64_to_u64(value: i64, label: &str) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|_| numeric_conversion_error(value, label))
}

fn i64_to_u64_sql(value: i64, label: &str) -> rusqlite::Result<u64> {
    i64_to_u64(value, label)
}

fn optional_i64_to_u64_sql(value: Option<i64>, label: &str) -> rusqlite::Result<Option<u64>> {
    value.map(|value| i64_to_u64(value, label)).transpose()
}

fn i64_to_u32_sql(value: i64, label: &str) -> rusqlite::Result<u32> {
    u32::try_from(value).map_err(|_| numeric_conversion_error(value, label))
}

fn numeric_conversion_error(value: i64, label: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Integer,
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{label} contains invalid integer {value}"),
        )
        .into(),
    )
}

fn database_error(path: &Path, error: rusqlite::Error) -> LeyCoreError {
    LeyCoreError::ContinuityDatabase {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Barrier};
    use tempfile::tempdir;

    fn private_store() -> (tempfile::TempDir, ContinuityStore) {
        let base = tempdir().unwrap();
        let directory = base.path().join("private");
        fs::create_dir(&directory).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let store = ContinuityStore::at(directory.join(CONTINUITY_DATABASE_FILE));
        (base, store)
    }

    fn project() -> ProjectIdentity {
        ProjectIdentity {
            schema_version: crate::PROJECT_SCHEMA_VERSION,
            project_id: format!("prj_{}", "1".repeat(32)),
            name: "Continuity test".to_owned(),
            created_at_unix_ms: 100,
        }
    }

    fn event(
        project_id: &str,
        event_id: &str,
        session_id: &str,
        sequence: u64,
        kind: &str,
        payload: Value,
    ) -> ContinuityEventInput {
        ContinuityEventInput {
            event_id: event_id.to_owned(),
            project_id: project_id.to_owned(),
            subject_id: None,
            session_id: Some(session_id.to_owned()),
            session_sequence: Some(sequence),
            request_id: Some(format!("req_{sequence}_{}", "a".repeat(24))),
            request_fingerprint: Some(format!(
                "sha256:{}",
                format!("{sequence:x}").repeat(64)[..64].to_owned()
            )),
            kind: kind.to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 200 + sequence,
            revision_head: Some("0123456789abcdef".to_owned()),
            revision_branch: Some("main".to_owned()),
            payload,
        }
    }

    #[test]
    fn fresh_store_enforces_policy_and_round_trips_linked_events() {
        let (_base, store) = private_store();
        store.initialize().unwrap();
        assert_eq!(store.schema_version().unwrap(), CONTINUITY_SCHEMA_VERSION);

        let connection = store.open_connection().unwrap();
        let journal: String = connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        let synchronous: i64 = connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .unwrap();
        let foreign_keys: i64 = connection
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        let secure_delete: i64 = connection
            .pragma_query_value(None, "secure_delete", |row| row.get(0))
            .unwrap();
        let trusted_schema: i64 = connection
            .pragma_query_value(None, "trusted_schema", |row| row.get(0))
            .unwrap();
        assert_eq!(journal.to_ascii_lowercase(), "wal");
        assert_eq!(synchronous, 2);
        assert_eq!(foreign_keys, 1);
        assert_eq!(secure_delete, 1);
        assert_eq!(trusted_schema, 0);
        drop(connection);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(store.path()).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(store.path().parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }

        let project = project();
        assert!(store.register_project(&project).unwrap().created);
        assert!(!store.register_project(&project).unwrap().created);
        let session_id = format!("ses_{}", "2".repeat(32));
        let evidence = event(
            &project.project_id,
            &format!("evt_{}", "3".repeat(64)),
            &session_id,
            0,
            "evidence-recorded",
            json!({
                "contentHash": format!("sha256:{}", "b".repeat(64)),
                "mediaType": "text/plain",
                "summary": "targeted test output"
            }),
        );
        let verification = event(
            &project.project_id,
            &format!("evt_{}", "4".repeat(64)),
            &session_id,
            1,
            "verification-recorded",
            json!({"status":"passed","summary":"targeted test passed"}),
        );
        assert!(store.append_event(&evidence).unwrap().created);
        assert!(store.append_event(&verification).unwrap().created);
        let mut retry = verification.clone();
        retry.event_id = format!("evt_{}", "c".repeat(64));
        retry.recorded_at_unix_ms += 99;
        let replayed = store.append_event(&retry).unwrap();
        assert!(!replayed.created);
        assert_eq!(replayed.record.event_id, verification.event_id);
        assert!(store
            .link_events(
                &project.project_id,
                &verification.event_id,
                &evidence.event_id,
                "supported-by",
            )
            .unwrap());
        assert!(!store
            .link_events(
                &project.project_id,
                &verification.event_id,
                &evidence.event_id,
                "supported-by",
            )
            .unwrap());

        let events = store
            .events_for_session(&project.project_id, &session_id)
            .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], event_from_input(evidence.clone()));
        assert_eq!(events[1], event_from_input(verification.clone()));
        assert_eq!(
            store
                .linked_events(&project.project_id, &verification.event_id, "supported-by")
                .unwrap(),
            vec![event_from_input(evidence)]
        );

        let mut conflicting = verification;
        conflicting.payload = json!({"status":"failed"});
        assert!(matches!(
            store.append_event(&conflicting),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("reused with different event content")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn egress_authority_lock_is_private_and_rejects_symlink_replacement() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let (base, store) = private_store();
        store.initialize().unwrap();
        let project = project();
        store.register_project(&project).unwrap();
        store
            .set_project_egress_policy(&project.project_id, crate::AgentEgressPolicy::AgentOk)
            .unwrap();

        let lock_path = store.egress_authority_lock_path();
        assert_eq!(
            fs::metadata(&lock_path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        fs::remove_file(&lock_path).unwrap();
        let outside = base.path().join("outside-lock");
        fs::write(&outside, b"do not lock through this path\n").unwrap();
        symlink(&outside, &lock_path).unwrap();
        assert!(matches!(
            store.set_project_egress_policy(
                &project.project_id,
                crate::AgentEgressPolicy::NeverSend
            ),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("egress authority lock is not a regular file")
        ));
        assert_eq!(
            fs::read_to_string(outside).unwrap(),
            "do not lock through this path\n"
        );
    }

    #[test]
    fn pending_egress_migration_fails_closed_before_protected_operation() {
        let (_base, store) = private_store();
        store.initialize().unwrap();
        let project = project();
        store.register_project(&project).unwrap();

        assert_eq!(
            store.project_egress_policy(&project.project_id).unwrap(),
            crate::AgentEgressPolicy::AgentOk
        );
        assert!(matches!(
            store.with_project_egress_locked(
                &project.project_id,
                crate::AgentEgressTarget::Cloud,
                || Ok(())
            ),
            Err(LeyCoreError::AgentEgressPolicyMigrationPending { project_id })
                if project_id == project.project_id
        ));
    }

    #[test]
    fn approved_source_legacy_import_is_hash_verified_atomic_and_idempotent() {
        let (_base, store) = private_store();
        store.initialize().unwrap();
        let project = project();
        store.register_project(&project).unwrap();

        assert!(!store
            .approved_source_authority_ready(&project.project_id)
            .unwrap());
        let bytes = b"legacy approved source snapshot".to_vec();
        let content_hash = format!("sha256:{:x}", Sha256::digest(&bytes));
        let snapshots = vec![ContinuityApprovedSourceSnapshotInput {
            source_id: "spec_22222222222222222222222222222222".to_owned(),
            display_name: "Specs/Legacy.md".to_owned(),
            content_hash: content_hash.clone(),
            approved_at_unix_ms: 101,
            content_bytes: bytes.clone(),
        }];
        let issues = vec![ContinuityApprovedSourceIssueInput {
            source_id: "spec_33333333333333333333333333333333".to_owned(),
            display_name: "Specs/Stale.md".to_owned(),
            approved_content_hash: format!("sha256:{}", "c".repeat(64)),
            approved_at_unix_ms: 102,
            reason: ContinuityApprovedSourceIssueReason::Changed,
        }];

        assert!(store
            .import_legacy_approved_source_authority(&project.project_id, &snapshots, &issues)
            .unwrap());
        assert!(store
            .approved_source_authority_ready(&project.project_id)
            .unwrap());

        let connection = store.open_connection().unwrap();
        let stored: (String, String, Vec<u8>) = connection
            .query_row(
                "SELECT s.source_kind, s.content_hash, b.content_bytes
                 FROM approved_sources s
                 JOIN approved_source_blobs b
                   ON b.project_id = s.project_id AND b.content_hash = s.snapshot_blob_hash
                 WHERE s.project_id = ?1 AND s.source_id = ?2",
                params![project.project_id, "spec_22222222222222222222222222222222"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(stored.0, "imported-snapshot");
        assert_eq!(stored.1, content_hash);
        assert_eq!(stored.2, bytes);
        let issue_reason: String = connection
            .query_row(
                "SELECT reason FROM legacy_approved_source_issues
                 WHERE project_id = ?1 AND source_id = ?2",
                params![project.project_id, "spec_33333333333333333333333333333333"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(issue_reason, "changed");
        drop(connection);

        assert!(!store
            .import_legacy_approved_source_authority(&project.project_id, &[], &[])
            .unwrap());

        store.erase_project(&project.project_id).unwrap();
        let connection = store.open_connection().unwrap();
        for table in [
            "approved_sources",
            "approved_source_blobs",
            "legacy_approved_source_issues",
        ] {
            let remaining: i64 = connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(remaining, 0, "{table} should cascade with project erasure");
        }
    }

    #[test]
    fn approved_source_legacy_import_rejects_bad_snapshot_without_partial_state() {
        let (_base, store) = private_store();
        store.initialize().unwrap();
        let project = project();
        store.register_project(&project).unwrap();
        let snapshots = vec![ContinuityApprovedSourceSnapshotInput {
            source_id: "spec_44444444444444444444444444444444".to_owned(),
            display_name: "Specs/Corrupt.md".to_owned(),
            content_hash: format!("sha256:{}", "d".repeat(64)),
            approved_at_unix_ms: 103,
            content_bytes: b"different bytes".to_vec(),
        }];

        assert!(matches!(
            store.import_legacy_approved_source_authority(&project.project_id, &snapshots, &[]),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("does not match its approved hash")
        ));
        assert!(!store
            .approved_source_authority_ready(&project.project_id)
            .unwrap());
        let connection = store.open_connection().unwrap();
        for table in [
            "approved_sources",
            "approved_source_blobs",
            "legacy_approved_source_issues",
        ] {
            let remaining: i64 = connection
                .query_row(
                    &format!("SELECT count(*) FROM {table} WHERE project_id = ?1"),
                    [&project.project_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(remaining, 0, "{table} must remain empty on failed import");
        }
    }

    #[test]
    fn portable_validation_rejects_corrupt_snapshot_and_unsafe_project_file_path() {
        let (_base, store) = private_store();
        store.initialize().unwrap();
        let project = project();
        store.register_project(&project).unwrap();
        let snapshot_bytes = b"portable approved snapshot\n".to_vec();
        let snapshot_hash = format!("sha256:{:x}", Sha256::digest(&snapshot_bytes));
        store
            .import_legacy_approved_source_authority(
                &project.project_id,
                &[ContinuityApprovedSourceSnapshotInput {
                    source_id: "spec_66666666666666666666666666666666".to_owned(),
                    display_name: "Legacy/Portable.md".to_owned(),
                    content_hash: snapshot_hash.clone(),
                    approved_at_unix_ms: 201,
                    content_bytes: snapshot_bytes,
                }],
                &[],
            )
            .unwrap();
        store
            .approve_project_file_source(
                &project.project_id,
                "spec_77777777777777777777777777777777",
                "AGENTS.md",
                "AGENTS.md",
                &format!("sha256:{}", "a".repeat(64)),
                202,
            )
            .unwrap();

        let corrupt_snapshot = store
            .path()
            .parent()
            .unwrap()
            .join("portable-corrupt.sqlite3");
        store
            .export_project_database(&project.project_id, &corrupt_snapshot)
            .unwrap();
        let connection = Connection::open_with_flags(
            &corrupt_snapshot,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .unwrap();
        connection
            .execute(
                "UPDATE approved_source_blobs SET content_bytes = ?1
                 WHERE project_id = ?2 AND content_hash = ?3",
                params![b"tampered".as_slice(), project.project_id, snapshot_hash],
            )
            .unwrap();
        assert!(matches!(
            validate_exported_project_database(&connection, &corrupt_snapshot, &project.project_id),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("failed content validation")
        ));
        drop(connection);

        let unsafe_path = store
            .path()
            .parent()
            .unwrap()
            .join("portable-unsafe.sqlite3");
        store
            .export_project_database(&project.project_id, &unsafe_path)
            .unwrap();
        let connection = Connection::open_with_flags(
            &unsafe_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .unwrap();
        connection
            .execute(
                "UPDATE approved_sources SET project_relative_path = '../escape'
                 WHERE project_id = ?1 AND source_id = ?2",
                params![project.project_id, "spec_77777777777777777777777777777777"],
            )
            .unwrap();
        assert!(matches!(
            validate_exported_project_database(&connection, &unsafe_path, &project.project_id),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("invalid path")
        ));
    }

    #[test]
    fn protected_egress_operation_rejects_same_thread_authority_reentry() {
        let (_base, store) = private_store();
        store.initialize().unwrap();
        let project = project();
        store.register_project(&project).unwrap();
        store
            .set_project_egress_policy(&project.project_id, crate::AgentEgressPolicy::AgentOk)
            .unwrap();

        assert!(matches!(
            store.with_project_egress_locked(
                &project.project_id,
                crate::AgentEgressTarget::Cloud,
                || store.set_project_egress_policy(
                    &project.project_id,
                    crate::AgentEgressPolicy::NeverSend
                )
            ),
            Err(LeyCoreError::AgentEgressAuthorityReentrant)
        ));
        assert_eq!(
            store.project_egress_policy(&project.project_id).unwrap(),
            crate::AgentEgressPolicy::AgentOk
        );
    }

    #[test]
    fn approved_source_authority_rejects_same_thread_reentry() {
        let (_base, store) = private_store();
        store.initialize().unwrap();
        let project = project();
        store.register_project(&project).unwrap();
        store
            .import_legacy_approved_source_authority(&project.project_id, &[], &[])
            .unwrap();

        assert!(matches!(
            store.with_approved_source_authority_lock(|| {
                store.revoke_approved_source(&project.project_id, "spec_missing")?;
                Ok(())
            }),
            Err(LeyCoreError::ApprovedSourceAuthorityReentrant)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn approved_source_authority_lock_is_private_and_rejects_symlink_replacement() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let (base, store) = private_store();
        store.initialize().unwrap();
        let project = project();
        store.register_project(&project).unwrap();
        store
            .import_legacy_approved_source_authority(&project.project_id, &[], &[])
            .unwrap();

        let lock_path = store.approved_source_authority_lock_path();
        assert_eq!(
            fs::metadata(&lock_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::remove_file(&lock_path).unwrap();
        let outside = base.path().join("outside-approved-source-lock");
        fs::write(&outside, b"do not lock through this path\n").unwrap();
        symlink(&outside, &lock_path).unwrap();

        assert!(matches!(
            store.revoke_approved_source(&project.project_id, "spec_missing"),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("approved-source authority lock is not a regular file")
        ));
        assert_eq!(
            fs::read_to_string(outside).unwrap(),
            "do not lock through this path\n"
        );
    }

    #[test]
    fn existing_v1_store_migrates_to_current_schema() {
        let (_base, store) = private_store();
        store.initialize().unwrap();
        let project = project();
        store.register_project(&project).unwrap();
        let original = ContinuityEventInput {
            event_id: format!("evt_{}", "a".repeat(64)),
            project_id: project.project_id.clone(),
            subject_id: Some("lrn_v1_migration".to_owned()),
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "legacy-learning-proposed".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 250,
            revision_head: None,
            revision_branch: None,
            payload: json!({
                "schemaVersion": 3,
                "learningId": "lrn_v1_migration",
                "sequence": 1,
                "kind": "proposed",
                "data": {"evidence": []}
            }),
        };
        store.append_event(&original).unwrap();
        let connection = store.open_connection().unwrap();
        connection
            .execute_batch(
                "DROP TABLE working_copy_import_heads;
                 DROP TABLE working_copy_inventory;
                 DROP TABLE import_source_paths;
                 DROP TABLE import_erasure_fences;
                 DROP TABLE project_import_attempts;
                 DROP TABLE event_source_version_links;
                 DROP TABLE source_locators;
                 DROP TABLE source_versions;
                 DROP TABLE project_sources;
                 DROP TABLE working_copy_locators;
                 DROP TABLE project_repositories;
                 DROP TABLE project_sessions;
                 DROP TRIGGER projects_reject_terminal_lifecycle;
                 DROP TABLE project_lifecycle;
                 DROP TABLE artifact_write_authority;
                 DROP TABLE project_artifact_state;
                 DROP TABLE artifact_files;
                 DROP TABLE artifact_snapshots;
                 DROP TABLE local_migration_state;
                 DROP TABLE approved_sources;
                 DROP TABLE legacy_approved_source_issues;
                 DROP TABLE approved_source_blobs;
                 ALTER TABLE projects DROP COLUMN approved_source_authority_migrated;
                 DROP INDEX events_subject_time;
                 ALTER TABLE events DROP COLUMN subject_id;
                 DROP TABLE project_observations;
                 ALTER TABLE projects DROP COLUMN agent_egress_policy_migrated;
                 ALTER TABLE projects DROP COLUMN agent_egress_policy;
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        drop(connection);

        assert_eq!(store.schema_version().unwrap(), CONTINUITY_SCHEMA_VERSION);
        let connection = store.open_connection().unwrap();
        let subject_columns: i64 = connection
            .query_row(
                "SELECT count(*) FROM pragma_table_info('events') WHERE name = 'subject_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(subject_columns, 1);
        for table in [
            "artifact_snapshots",
            "artifact_files",
            "project_artifact_state",
            "artifact_write_authority",
        ] {
            let present: i64 = connection
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(present, 1, "{table} should be created by schema migration");
        }
        let captured_at_columns: i64 = connection
            .query_row(
                "SELECT count(*) FROM pragma_table_info('project_artifact_state')
                 WHERE name = 'captured_at_unix_ms'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(captured_at_columns, 1);
        drop(connection);
        assert_eq!(
            store.project_egress_policy(&project.project_id).unwrap(),
            crate::AgentEgressPolicy::AgentOk
        );
        store
            .set_project_egress_policy(&project.project_id, crate::AgentEgressPolicy::NeverSend)
            .unwrap();
        assert_eq!(
            store.project_egress_policy(&project.project_id).unwrap(),
            crate::AgentEgressPolicy::NeverSend
        );
        assert_eq!(
            store
                .event(&project.project_id, &original.event_id)
                .unwrap()
                .unwrap(),
            event_from_input(original)
        );
    }

    #[test]
    fn existing_v9_artifact_cutover_migrates_to_write_authority_origin() {
        let (base, store) = private_store();
        let project_root = base.path().join("v9-artifact-project");
        let vault = base.path().join("v9-artifact-vault");
        fs::create_dir(&project_root).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(project_root.join("README.md"), "# v9 artifact authority\n").unwrap();
        let initialized = crate::initialize_project(
            &project_root,
            Some("v9 artifact authority"),
            crate::CaptureMode::Structured,
        )
        .unwrap();
        crate::ingest_project(&project_root, &vault).unwrap();
        crate::ingest_project_with_continuity_transition(&project_root, &vault, &store).unwrap();

        let connection = store.open_connection().unwrap();
        connection
            .execute_batch(
                "DROP TABLE working_copy_import_heads;
                 DROP TABLE working_copy_inventory;
                 DROP TABLE import_source_paths;
                 DROP TABLE import_erasure_fences;
                 DROP TABLE project_import_attempts;
                 DROP TABLE event_source_version_links;
                 DROP TABLE source_locators;
                 DROP TABLE source_versions;
                 DROP TABLE project_sources;
                 DROP TABLE working_copy_locators;
                 DROP TABLE project_repositories;
                 DROP TABLE project_sessions;
                 DROP TRIGGER projects_reject_terminal_lifecycle;
                 DROP TABLE project_lifecycle;
                 CREATE TABLE artifact_authority_cutovers (
                    project_id TEXT PRIMARY KEY NOT NULL
                        REFERENCES projects(project_id) ON DELETE CASCADE,
                    legacy_snapshot_id TEXT NOT NULL,
                    legacy_capture_fingerprint TEXT NOT NULL,
                    legacy_file_count INTEGER NOT NULL,
                    recorded_at_unix_ms INTEGER NOT NULL
                 ) STRICT;
                 INSERT INTO artifact_authority_cutovers(
                    project_id, legacy_snapshot_id, legacy_capture_fingerprint,
                    legacy_file_count, recorded_at_unix_ms
                 )
                 SELECT project_id, initial_snapshot_id, initial_capture_fingerprint,
                        initial_file_count, recorded_at_unix_ms
                 FROM artifact_write_authority;
                 DROP TABLE artifact_write_authority;
                 ALTER TABLE project_observations DROP COLUMN continuity_origin;
                 PRAGMA user_version = 9;",
            )
            .unwrap();
        drop(connection);

        assert_eq!(store.schema_version().unwrap(), CONTINUITY_SCHEMA_VERSION);
        let connection = store.open_connection().unwrap();
        let migrated: (String, String) = connection
            .query_row(
                "SELECT origin, initial_snapshot_id
                 FROM artifact_write_authority
                 WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(migrated.0, "legacy-cutover");
        let current: String = connection
            .query_row(
                "SELECT current_snapshot_id
                 FROM project_artifact_state
                 WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migrated.1, current);
        let legacy_table: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'artifact_authority_cutovers'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_table, 0);
    }

    #[test]
    fn transition_ingest_tracks_immutable_artifact_snapshots_and_erases_with_project() {
        let (base, store) = private_store();
        let project_root = base.path().join("artifact-project");
        let vault = base.path().join("artifact-vault");
        fs::create_dir(&project_root).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(
            project_root.join("README.md"),
            "# Native artifact metadata\nfirst captured body\n",
        )
        .unwrap();
        let initialized = crate::initialize_project(
            &project_root,
            Some("Artifact metadata"),
            crate::CaptureMode::Structured,
        )
        .unwrap();

        crate::ingest_project(&project_root, &vault).unwrap();
        let first = crate::ingest_project_with_continuity_transition(&project_root, &vault, &store)
            .unwrap();
        let connection = store.open_connection().unwrap();
        let current: String = connection
            .query_row(
                "SELECT current_snapshot_id FROM project_artifact_state WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(current, first.snapshot_id);
        let snapshot_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM artifact_snapshots WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(snapshot_count, 1);
        let retained_files: i64 = connection
            .query_row(
                "SELECT count(*) FROM artifact_files
                 WHERE project_id = ?1 AND snapshot_id = ?2 AND content_captured = 1",
                params![initialized.identity.project_id, first.snapshot_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(retained_files >= 1);
        let first_readme_hash: String = connection
            .query_row(
                "SELECT content_hash FROM artifact_files
                 WHERE project_id = ?1 AND snapshot_id = ?2 AND artifact_path = 'README.md'",
                params![initialized.identity.project_id, first.snapshot_id],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);
        let native_project_dir = store
            .artifact_project_content_dir(&initialized.identity.project_id)
            .unwrap();
        let first_blob = native_project_dir.join(artifact_digest(&first_readme_hash).unwrap());
        let first_blob_bytes = fs::read(&first_blob).unwrap();
        assert_eq!(
            format!("sha256:{:x}", Sha256::digest(&first_blob_bytes)),
            first_readme_hash
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&native_project_dir)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(&first_blob).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }

        let first_captured_at: i64 = store
            .open_connection()
            .unwrap()
            .query_row(
                "SELECT captured_at_unix_ms
                 FROM project_artifact_state
                 WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));

        let unchanged =
            crate::ingest_project_with_continuity_transition(&project_root, &vault, &store)
                .unwrap();
        assert_eq!(unchanged.snapshot_id, first.snapshot_id);
        let refreshed_captured_at: i64 = store
            .open_connection()
            .unwrap()
            .query_row(
                "SELECT captured_at_unix_ms
                 FROM project_artifact_state
                 WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(refreshed_captured_at > first_captured_at);
        assert!(store
            .artifact_write_authority_ready(&initialized.identity.project_id)
            .unwrap());
        assert!(crate::ingest_project(&project_root, &vault).is_err());
        let connection = store.open_connection().unwrap();
        let snapshot_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM artifact_snapshots WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(snapshot_count, 1);
        drop(connection);

        fs::write(
            project_root.join("README.md"),
            "# Native artifact metadata\nsecond captured body\n",
        )
        .unwrap();
        fs::remove_dir_all(&vault).unwrap();
        let changed =
            crate::ingest_project_with_continuity_transition(&project_root, &vault, &store)
                .unwrap();
        assert_ne!(changed.snapshot_id, first.snapshot_id);
        assert!(changed.manifest_path.is_none());
        assert!(changed.graph_path.is_none());
        let connection = store.open_connection().unwrap();
        let current: String = connection
            .query_row(
                "SELECT current_snapshot_id FROM project_artifact_state WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(current, changed.snapshot_id);
        let snapshot_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM artifact_snapshots WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(snapshot_count, 1);
        let historical_files: i64 = connection
            .query_row(
                "SELECT count(*) FROM artifact_files
                 WHERE project_id = ?1 AND snapshot_id = ?2",
                params![initialized.identity.project_id, first.snapshot_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(historical_files, 0);
        let changed_readme_hash: String = connection
            .query_row(
                "SELECT content_hash FROM artifact_files
                 WHERE project_id = ?1 AND snapshot_id = ?2 AND artifact_path = 'README.md'",
                params![initialized.identity.project_id, changed.snapshot_id],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);
        assert_ne!(changed_readme_hash, first_readme_hash);
        assert!(!first_blob.exists());
        assert!(native_project_dir
            .join(artifact_digest(&changed_readme_hash).unwrap())
            .is_file());

        let portable_db = store
            .path()
            .parent()
            .unwrap()
            .join("artifact-metadata-portable.sqlite3");
        store
            .export_project_database(&initialized.identity.project_id, &portable_db)
            .unwrap();
        let portable = Connection::open_with_flags(
            &portable_db,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .unwrap();
        for table in [
            "artifact_snapshots",
            "artifact_files",
            "project_artifact_state",
            "artifact_write_authority",
        ] {
            let exported_rows: i64 = portable
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(
                exported_rows, 0,
                "{table} is rebuildable machine-local artifact authority and must stay out of portable continuity"
            );
        }
        drop(portable);

        store
            .erase_project(&initialized.identity.project_id)
            .unwrap();
        assert!(!native_project_dir.exists());
        let connection = store.open_connection().unwrap();
        for table in [
            "artifact_snapshots",
            "artifact_files",
            "project_artifact_state",
        ] {
            let remaining: i64 = connection
                .query_row(
                    &format!("SELECT count(*) FROM {table} WHERE project_id = ?1"),
                    [&initialized.identity.project_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(remaining, 0, "{table} must cascade with project erasure");
        }
    }

    #[test]
    fn durable_citation_retains_only_cited_historical_artifact_bytes() {
        let (base, store) = private_store();
        let project_root = base.path().join("cited-artifact-project");
        let vault = base.path().join("cited-artifact-vault");
        fs::create_dir(&project_root).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(project_root.join("README.md"), "old cited body\n").unwrap();
        fs::write(project_root.join("stale.txt"), "old uncited body\n").unwrap();
        let initialized = crate::initialize_project(
            &project_root,
            Some("Cited artifact retention"),
            crate::CaptureMode::Structured,
        )
        .unwrap();
        crate::ingest_project(&project_root, &vault).unwrap();
        let first = crate::ingest_project_with_continuity_transition(&project_root, &vault, &store)
            .unwrap();
        let connection = store.open_connection().unwrap();
        let cited_hash: String = connection
            .query_row(
                "SELECT content_hash FROM artifact_files
                 WHERE project_id = ?1 AND snapshot_id = ?2 AND artifact_path = 'README.md'",
                params![initialized.identity.project_id, first.snapshot_id],
                |row| row.get(0),
            )
            .unwrap();
        let uncited_hash: String = connection
            .query_row(
                "SELECT content_hash FROM artifact_files
                 WHERE project_id = ?1 AND snapshot_id = ?2 AND artifact_path = 'stale.txt'",
                params![initialized.identity.project_id, first.snapshot_id],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);

        let session_id = format!("ses_{}", "9".repeat(32));
        store
            .append_event(&event(
                &initialized.identity.project_id,
                &format!("evt_{}", "9".repeat(64)),
                &session_id,
                1,
                "checkpoint-recorded",
                json!({
                    "evidence": {
                        "artifactSnapshotId": first.snapshot_id,
                        "artifactPath": "README.md",
                        "contentHash": cited_hash
                    }
                }),
            ))
            .unwrap();

        fs::write(project_root.join("README.md"), "new current body\n").unwrap();
        fs::write(project_root.join("stale.txt"), "new stale body\n").unwrap();
        let second =
            crate::ingest_project_with_continuity_transition(&project_root, &vault, &store)
                .unwrap();
        assert_ne!(second.snapshot_id, first.snapshot_id);

        let connection = store.open_connection().unwrap();
        let snapshots: i64 = connection
            .query_row(
                "SELECT count(*) FROM artifact_snapshots WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(snapshots, 2);
        drop(connection);
        let native_dir = store
            .artifact_project_content_dir(&initialized.identity.project_id)
            .unwrap();
        let cited_blob = native_dir.join(artifact_digest(&cited_hash).unwrap());
        let uncited_blob = native_dir.join(artifact_digest(&uncited_hash).unwrap());
        assert!(cited_blob.is_file());
        assert!(!uncited_blob.exists());

        let preview = store
            .preview_session_erasure(&initialized.identity.project_id, &session_id)
            .unwrap();
        store
            .erase_session(
                &initialized.identity.project_id,
                &session_id,
                &preview.confirmation_digest,
            )
            .unwrap();
        store
            .with_artifact_authority_lock(|| {
                store.garbage_collect_artifact_state_under_lock(&initialized.identity.project_id)
            })
            .unwrap();
        let connection = store.open_connection().unwrap();
        let old_snapshot: i64 = connection
            .query_row(
                "SELECT count(*) FROM artifact_snapshots
                 WHERE project_id = ?1 AND snapshot_id = ?2",
                params![initialized.identity.project_id, first.snapshot_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(old_snapshot, 0);
        assert!(!cited_blob.exists());
    }

    #[test]
    fn interrupted_artifact_install_recovers_before_activation() {
        let (base, store) = private_store();
        let project_root = base.path().join("interrupted-artifact-project");
        let vault = base.path().join("interrupted-artifact-vault");
        fs::create_dir(&project_root).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::write(project_root.join("README.md"), "first body\n").unwrap();
        let initialized = crate::initialize_project(
            &project_root,
            Some("Interrupted artifact install"),
            crate::CaptureMode::Structured,
        )
        .unwrap();
        crate::ingest_project(&project_root, &vault).unwrap();
        let first = crate::ingest_project_with_continuity_transition(&project_root, &vault, &store)
            .unwrap();

        fs::write(project_root.join("README.md"), "second body\n").unwrap();
        let diagnostic = crate::diagnose_project(&project_root).unwrap();
        let second_snapshot_id = store
            .with_artifact_authority_lock(|| {
                let prepared = crate::ingestion::prepare_artifact_capture(
                    &diagnostic,
                    None,
                    true,
                    |_name, content_hash, bytes| {
                        store.install_artifact_blob(
                            &initialized.identity.project_id,
                            content_hash,
                            bytes.len() as u64,
                            bytes,
                        )
                    },
                )?;
                assert_ne!(prepared.manifest.snapshot_id, first.snapshot_id);
                store
                    .stage_artifact_snapshot_metadata(&initialized.identity, &prepared.manifest)?;
                let project_dir =
                    store.artifact_project_content_dir(&initialized.identity.project_id)?;
                let stale_temp = project_dir.join(".tmp-artifact-interrupted-test");
                let mut options = OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                options
                    .open(&stale_temp)
                    .map_err(|source| LeyCoreError::Io {
                        path: stale_temp,
                        source,
                    })?
                    .sync_all()
                    .map_err(|source| LeyCoreError::Io {
                        path: project_dir,
                        source,
                    })?;
                Ok(prepared.manifest.snapshot_id)
            })
            .unwrap();
        let connection = store.open_connection().unwrap();
        let current: String = connection
            .query_row(
                "SELECT current_snapshot_id FROM project_artifact_state WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(current, first.snapshot_id);
        drop(connection);

        let recovered =
            crate::ingest_project_with_continuity_transition(&project_root, &vault, &store)
                .unwrap();
        assert_eq!(recovered.snapshot_id, second_snapshot_id);
        let connection = store.open_connection().unwrap();
        let current: String = connection
            .query_row(
                "SELECT current_snapshot_id FROM project_artifact_state WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(current, second_snapshot_id);
        let snapshots: i64 = connection
            .query_row(
                "SELECT count(*) FROM artifact_snapshots WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(snapshots, 1);
        drop(connection);
        let project_dir = store
            .artifact_project_content_dir(&initialized.identity.project_id)
            .unwrap();
        assert!(fs::read_dir(project_dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".tmp-artifact-")));
    }

    #[test]
    fn independent_connections_do_not_lose_concurrent_events() {
        let (_base, store) = private_store();
        let project = project();
        store.register_project(&project).unwrap();
        let session_id = format!("ses_{}", "5".repeat(32));

        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();
        for worker in 0..2_u8 {
            let store = store.clone();
            let project_id = project.project_id.clone();
            let session_id = session_id.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                let shared = ContinuityEventInput {
                    event_id: format!("evt_shared_{worker}_{}", "b".repeat(48)),
                    project_id: project_id.clone(),
                    subject_id: None,
                    session_id: Some(session_id.clone()),
                    session_sequence: Some(17),
                    request_id: Some(format!("req_shared_{}", "c".repeat(24))),
                    request_fingerprint: Some(format!("sha256:{}", "d".repeat(64))),
                    kind: "attempt-recorded".to_owned(),
                    payload_version: 1,
                    recorded_at_unix_ms: 10_000 + u64::from(worker),
                    revision_head: None,
                    revision_branch: None,
                    payload: json!({"shared":true}),
                };
                let shared_created = store.append_event(&shared).unwrap().created;
                for index in 0..8_u8 {
                    let sequence = u64::from(worker) * 8 + u64::from(index) + 1;
                    let input = event(
                        &project_id,
                        &format!("evt_{worker}_{index}_{}", "6".repeat(48)),
                        &session_id,
                        sequence,
                        "attempt-recorded",
                        json!({"worker":worker,"index":index}),
                    );
                    assert!(store.append_event(&input).unwrap().created);
                }
                shared_created
            }));
        }
        barrier.wait();
        let mut shared_creations = 0;
        for handle in handles {
            shared_creations += usize::from(handle.join().unwrap());
        }
        assert_eq!(shared_creations, 1);
        assert_eq!(
            store
                .events_for_session(&project.project_id, &session_id)
                .unwrap()
                .len(),
            17
        );
    }

    #[test]
    fn transactional_session_append_allocates_from_one_serialized_snapshot() {
        let (_base, store) = private_store();
        let project = project();
        store.register_project(&project).unwrap();
        let session_id = format!("ses_{}", "6".repeat(32));
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();

        for worker in 0..2_u8 {
            let store = store.clone();
            let project_id = project.project_id.clone();
            let session_id = session_id.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                store
                    .append_session_event_transactional(&project_id, &session_id, |existing| {
                        let sequence = existing.len() as u64 + 1;
                        Ok(ContinuityEventInput {
                            event_id: format!("evt_native_{worker}_{}", "7".repeat(48)),
                            project_id: project_id.clone(),
                            subject_id: None,
                            session_id: Some(session_id.clone()),
                            session_sequence: Some(sequence),
                            request_id: Some(format!("req_native_{worker}_{}", "8".repeat(20))),
                            request_fingerprint: Some(format!(
                                "sha256:{}",
                                char::from(b'a' + worker as u8).to_string().repeat(64)
                            )),
                            kind: "session-test-recorded".to_owned(),
                            payload_version: 1,
                            recorded_at_unix_ms: 20_000 + sequence,
                            revision_head: None,
                            revision_branch: None,
                            payload: json!({"worker": worker, "sequence": sequence}),
                        })
                    })
                    .unwrap()
            }));
        }

        barrier.wait();
        let mut snapshot_lengths = handles
            .into_iter()
            .map(|handle| {
                let (write, events) = handle.join().unwrap();
                assert!(write.created);
                events.len()
            })
            .collect::<Vec<_>>();
        snapshot_lengths.sort();
        assert_eq!(snapshot_lengths, vec![1, 2]);

        let events = store
            .events_for_session(&project.project_id, &session_id)
            .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].session_sequence, Some(1));
        assert_eq!(events[1].session_sequence, Some(2));
        assert_ne!(events[0].event_id, events[1].event_id);
    }

    #[test]
    fn project_erasure_cascades_events_and_links_and_truncates_wal() {
        let (_base, store) = private_store();
        let project = project();
        store.register_project(&project).unwrap();
        let session_id = format!("ses_{}", "7".repeat(32));
        let evidence = event(
            &project.project_id,
            &format!("evt_{}", "8".repeat(64)),
            &session_id,
            1,
            "evidence-recorded",
            json!({"summary":"private evidence"}),
        );
        let handoff = event(
            &project.project_id,
            &format!("evt_{}", "9".repeat(64)),
            &session_id,
            2,
            "handoff-recorded",
            json!({"summary":"continue from here"}),
        );
        store.append_event(&evidence).unwrap();
        store.append_event(&handoff).unwrap();
        store
            .link_events(
                &project.project_id,
                &handoff.event_id,
                &evidence.event_id,
                "supported-by",
            )
            .unwrap();

        store.erase_project(&project.project_id).unwrap();
        store.erase_project(&project.project_id).unwrap();

        let connection = store.open_connection().unwrap();
        for table in ["projects", "events", "event_links"] {
            let count: i64 = connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "{table} should be empty after project erasure");
        }
        drop(connection);
        let wal = store.path().with_extension("sqlite3-wal");
        if let Ok(metadata) = fs::metadata(wal) {
            assert_eq!(
                metadata.len(),
                0,
                "erasure must truncate retained WAL frames"
            );
        }
    }

    #[test]
    fn native_session_erasure_cascades_dependents_and_rejects_stale_preview() {
        let (_base, store) = private_store();
        let project = project();
        store.register_project(&project).unwrap();
        let session_id = format!("ses_{}", "c".repeat(32));
        let session_start = event(
            &project.project_id,
            &format!("evt_{}", "1".repeat(64)),
            &session_id,
            0,
            "session-started",
            json!({"name":"native session"}),
        );
        let checkpoint = event(
            &project.project_id,
            &format!("evt_{}", "2".repeat(64)),
            &session_id,
            1,
            "checkpoint-recorded",
            json!({"summary":"checkpoint"}),
        );
        store.append_event(&session_start).unwrap();
        store.append_event(&checkpoint).unwrap();

        let dependent = ContinuityEventInput {
            event_id: format!("evt_{}", "3".repeat(64)),
            project_id: project.project_id.clone(),
            subject_id: Some("lrn_dependent".to_owned()),
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "learning-proposed".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 300,
            revision_head: None,
            revision_branch: None,
            payload: json!({"guidance":"depends on session"}),
        };
        let superseder = ContinuityEventInput {
            event_id: format!("evt_{}", "4".repeat(64)),
            project_id: project.project_id.clone(),
            subject_id: Some("lrn_superseder".to_owned()),
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "learning-reviewed".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 301,
            revision_head: None,
            revision_branch: None,
            payload: json!({"action":"supersede"}),
        };
        let unrelated = ContinuityEventInput {
            event_id: format!("evt_{}", "5".repeat(64)),
            project_id: project.project_id.clone(),
            subject_id: Some("lrn_unrelated".to_owned()),
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "learning-proposed".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 302,
            revision_head: None,
            revision_branch: None,
            payload: json!({"guidance":"keep me"}),
        };
        let native = ContinuityEventInput {
            event_id: format!("evt_{}", "6".repeat(64)),
            project_id: project.project_id.clone(),
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "native-continuity-probe".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 303,
            revision_head: None,
            revision_branch: None,
            payload: json!({"keep":true}),
        };
        for input in [&dependent, &superseder, &unrelated, &native] {
            store.append_event(input).unwrap();
        }
        store
            .link_events(
                &project.project_id,
                &dependent.event_id,
                &session_start.event_id,
                "depends-on-session",
            )
            .unwrap();
        store
            .link_events(
                &project.project_id,
                &superseder.event_id,
                &dependent.event_id,
                "supersedes",
            )
            .unwrap();

        let stale = store
            .preview_session_erasure(&project.project_id, &session_id)
            .unwrap();
        assert_eq!(stale.session_event_count, 2);
        assert_eq!(
            stale.dependent_subject_ids,
            vec!["lrn_dependent".to_owned(), "lrn_superseder".to_owned()]
        );
        assert_eq!(stale.total_event_count, 4);

        let late_event = event(
            &project.project_id,
            &format!("evt_{}", "7".repeat(64)),
            &session_id,
            2,
            "session-finished",
            json!({"status":"completed"}),
        );
        store.append_event(&late_event).unwrap();
        assert!(matches!(
            store.erase_session(
                &project.project_id,
                &session_id,
                &stale.confirmation_digest
            ),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("changed since erasure preview")
        ));
        assert!(store
            .event(&project.project_id, &session_start.event_id)
            .unwrap()
            .is_some());

        let current = store
            .preview_session_erasure(&project.project_id, &session_id)
            .unwrap();
        assert_eq!(current.session_event_count, 3);
        assert_eq!(current.total_event_count, 5);
        let erased = store
            .erase_session(
                &project.project_id,
                &session_id,
                &current.confirmation_digest,
            )
            .unwrap();
        assert!(!erased.already_absent);
        assert_eq!(erased.erased_event_count, 5);
        assert_eq!(erased.erased_subject_ids, current.dependent_subject_ids);
        assert!(store
            .events_for_session(&project.project_id, &session_id)
            .unwrap()
            .is_empty());
        for event_id in [&dependent.event_id, &superseder.event_id] {
            assert!(store
                .event(&project.project_id, event_id)
                .unwrap()
                .is_none());
        }
        for event_id in [&unrelated.event_id, &native.event_id] {
            assert!(store
                .event(&project.project_id, event_id)
                .unwrap()
                .is_some());
        }

        let retried = store
            .erase_session(
                &project.project_id,
                &session_id,
                &current.confirmation_digest,
            )
            .unwrap();
        assert!(retried.already_absent);
        assert_eq!(retried.erased_event_count, 0);
    }
}
