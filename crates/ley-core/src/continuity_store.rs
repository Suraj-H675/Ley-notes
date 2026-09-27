use crate::{
    validate_identity, validate_project_id, LeyCoreError, ProjectIdentity, APP_IDENTIFIER,
};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const CONTINUITY_DATABASE_FILE: &str = "continuity.sqlite3";
pub const CONTINUITY_SCHEMA_VERSION: u32 = 2;
pub const CONTINUITY_EVENT_LIMIT_BYTES: usize = 1_048_576;
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct ContinuityStore {
    path: PathBuf,
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

    pub fn append_event(
        &self,
        input: &ContinuityEventInput,
    ) -> Result<ContinuityWrite<ContinuityEvent>, LeyCoreError> {
        let connection = self.open_connection()?;
        append_event_on(&connection, input, &self.path)
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
                    "SELECT event_id FROM events WHERE project_id = ?1 AND kind LIKE 'legacy-%' ORDER BY event_id",
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
        let mut statement = connection
            .prepare(
                "SELECT project_id, event_id, subject_id, session_id, session_sequence, request_id, request_fingerprint, kind, payload_version, recorded_at_unix_ms, revision_head, revision_branch, payload_json FROM events WHERE project_id = ?1 AND session_id = ?2 ORDER BY session_sequence IS NULL, session_sequence, recorded_at_unix_ms, event_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map(params![project_id, session_id], row_to_event)
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
        validate_project_id(project_id)?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        transaction
            .execute("DELETE FROM projects WHERE project_id = ?1", [project_id])
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        truncate_wal_after_erasure(&connection, &self.path)
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

        let source = self.open_connection()?;
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
            .execute("DELETE FROM projects WHERE project_id <> ?1", [project_id])
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

    fn open_connection(&self) -> Result<Connection, LeyCoreError> {
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

    fn database_error(&self, error: rusqlite::Error) -> LeyCoreError {
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
    if schema_version != CONTINUITY_SCHEMA_VERSION {
        return Err(LeyCoreError::InvalidContinuityStore(format!(
            "portable continuity database schema {schema_version} does not match supported schema {CONTINUITY_SCHEMA_VERSION}"
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

fn register_project_on(
    connection: &Connection,
    identity: &ProjectIdentity,
    path: &Path,
) -> Result<ContinuityWrite<ProjectIdentity>, LeyCoreError> {
    validate_identity(identity)?;
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
    Ok(ContinuityWrite {
        record: identity.clone(),
        created: false,
    })
}

fn append_event_on(
    connection: &Connection,
    input: &ContinuityEventInput,
    path: &Path,
) -> Result<ContinuityWrite<ContinuityEvent>, LeyCoreError> {
    validate_event_input(input)?;
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

fn read_event_by_request(
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

    #[test]
    fn existing_v1_store_migrates_to_subject_schema() {
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
                "DROP INDEX events_subject_time;
                 ALTER TABLE events DROP COLUMN subject_id;
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
        drop(connection);
        assert_eq!(
            store
                .event(&project.project_id, &original.event_id)
                .unwrap()
                .unwrap(),
            event_from_input(original)
        );
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
