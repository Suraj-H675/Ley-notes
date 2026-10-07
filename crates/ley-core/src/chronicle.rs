use crate::continuity_store::{append_project_brain_event_on, read_event_by_request};
use crate::project_brain::{
    bump_project_generation_on, invalid_brain, read_working_copy_on, request_fingerprint,
    require_active_project_on, require_project_generation_on, u64_i64, unix_time_ms,
};
use crate::{
    CaptureMode, ContinuityEvent, ContinuityEventInput, ContinuityStore, LeyCoreError,
    ProjectHandle, ProjectSession, WorkingCopyState,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const MAX_CHRONICLE_PAGE: usize = 100;
pub const MAX_CHRONICLE_SESSION_BYTES: usize = 1_048_576;
pub const MAX_CHRONICLE_SESSION_EVENTS: usize = 4096;
pub(crate) const CHRONICLE_EVENT_KIND: &str = "chronicle-episode";
const CAPTURE_ORIGIN: &str = "native-user-confirmation";

pub(crate) const CHRONICLE_SCHEMA: &str = r#"
CREATE TABLE chronicle_episodes (
 project_id TEXT NOT NULL, event_id TEXT NOT NULL, session_id TEXT,
 sequence INTEGER NOT NULL CHECK(sequence >= 1), episode_kind TEXT NOT NULL,
 evidence_basis TEXT NOT NULL CHECK(evidence_basis IN ('observed','reported','unknown')),
 producer TEXT NOT NULL, origin TEXT NOT NULL,
 external_turn_reference TEXT, external_tool_reference TEXT,
 occurred_at_unix_ms INTEGER, gaps_json TEXT NOT NULL CHECK(json_valid(gaps_json)),
 PRIMARY KEY(project_id,event_id),
 FOREIGN KEY(project_id,event_id) REFERENCES events(project_id,event_id) ON DELETE CASCADE
) STRICT;
CREATE UNIQUE INDEX chronicle_session_sequence
 ON chronicle_episodes(project_id,session_id,sequence) WHERE session_id IS NOT NULL;
CREATE UNIQUE INDEX chronicle_project_sequence
 ON chronicle_episodes(project_id,sequence) WHERE session_id IS NULL;
CREATE TABLE chronicle_capture_grants (
 project_id TEXT NOT NULL, locator_id TEXT NOT NULL, grant_id TEXT NOT NULL,
 generation INTEGER NOT NULL CHECK(generation >= 1), host TEXT NOT NULL CHECK(host='codex'),
 mode TEXT NOT NULL CHECK(mode IN ('minimal','structured','full-evidence')),
 root TEXT NOT NULL, root_stamp TEXT NOT NULL,
 origin TEXT NOT NULL CHECK(origin='native-user-confirmation'),
 authorized INTEGER NOT NULL CHECK(authorized IN (0,1)),
 authorized_at_unix_ms INTEGER NOT NULL, revoked_at_unix_ms INTEGER,
 PRIMARY KEY(project_id,locator_id), UNIQUE(grant_id),
 FOREIGN KEY(project_id,locator_id) REFERENCES working_copy_locators(project_id,locator_id) ON DELETE CASCADE
) STRICT;
CREATE TABLE chronicle_session_capture (
 project_id TEXT NOT NULL, session_id TEXT NOT NULL,
 retained_bytes INTEGER NOT NULL DEFAULT 0 CHECK(retained_bytes >= 0),
 event_count INTEGER NOT NULL DEFAULT 0 CHECK(event_count >= 0),
 body_limit_reached INTEGER NOT NULL DEFAULT 0 CHECK(body_limit_reached IN (0,1)),
 event_limit_reached INTEGER NOT NULL DEFAULT 0 CHECK(event_limit_reached IN (0,1)),
 PRIMARY KEY(project_id,session_id),
 FOREIGN KEY(project_id,session_id) REFERENCES project_sessions(project_id,session_id) ON DELETE CASCADE
) STRICT;
PRAGMA user_version = 14;
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EpisodeKind {
    SessionStart,
    SessionEnd,
    UserPrompt,
    AgentResponse,
    ToolCall,
    ToolResult,
    TurnBoundary,
    Interruption,
    Gap,
    StructuredSubmission,
    SourceCapture,
    ProjectImport,
    ProjectActivity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EpisodeEvidenceBasis {
    Observed,
    Reported,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Episode {
    pub project_id: String,
    pub event_id: String,
    pub session_id: Option<String>,
    pub sequence: u64,
    pub kind: EpisodeKind,
    pub evidence_basis: EpisodeEvidenceBasis,
    pub producer: String,
    pub origin: String,
    pub external_turn_reference: Option<String>,
    pub external_tool_reference: Option<String>,
    pub occurred_at_unix_ms: Option<u64>,
    pub recorded_at_unix_ms: u64,
    pub gaps: Vec<String>,
    pub event_kind: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChronicleSession {
    #[serde(flatten)]
    pub session: ProjectSession,
    pub episode_count: u64,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChroniclePage<T> {
    pub items: Vec<T>,
    pub next_offset: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChronicleCapturePreview {
    pub handle: ProjectHandle,
    pub locator_id: String,
    pub root: PathBuf,
    pub root_stamp: String,
    pub mode: CaptureMode,
    pub fingerprint: String,
    pub host: String,
    pub scope: Vec<String>,
    pub exclusions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChronicleCaptureState {
    pub project_id: String,
    pub locator_id: String,
    pub grant_id: String,
    pub generation: u64,
    pub mode: CaptureMode,
    pub root: PathBuf,
    pub root_stamp: String,
    pub origin: String,
    pub authorized: bool,
    pub effective: bool,
    pub inactive_reason: Option<String>,
    pub authorized_at_unix_ms: u64,
    pub revoked_at_unix_ms: Option<u64>,
}

pub type ChronicleCaptureGrant = ChronicleCaptureState;

#[derive(Debug, Clone)]
pub(crate) struct ActiveChronicleCapture {
    pub handle: ProjectHandle,
    pub mode: CaptureMode,
}

#[derive(Debug, Clone)]
pub(crate) enum ChronicleCaptureRoute {
    NotProjectBrain,
    Disabled(String),
    Enabled(ActiveChronicleCapture),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ChronicleObservation {
    SessionStart {
        source: String,
    },
    SessionEnd {
        reason: String,
    },
    UserPrompt {
        text: Option<String>,
        retention: String,
        truncated: bool,
    },
    AgentResponse {
        text: Option<String>,
        retention: String,
        truncated: bool,
    },
    ToolCall {
        tool: String,
        command: Option<String>,
        retention: String,
        truncated: bool,
    },
    ToolResult {
        tool: String,
        command: Option<String>,
        result: Option<String>,
        retention: String,
        command_truncated: bool,
        result_truncated: bool,
    },
    TurnBoundary,
    Interruption,
    Gap {
        reason: String,
    },
}
impl ChronicleObservation {
    pub(crate) fn kind(&self) -> EpisodeKind {
        match self {
            Self::SessionStart { .. } => EpisodeKind::SessionStart,
            Self::SessionEnd { .. } => EpisodeKind::SessionEnd,
            Self::UserPrompt { .. } => EpisodeKind::UserPrompt,
            Self::AgentResponse { .. } => EpisodeKind::AgentResponse,
            Self::ToolCall { .. } => EpisodeKind::ToolCall,
            Self::ToolResult { .. } => EpisodeKind::ToolResult,
            Self::TurnBoundary => EpisodeKind::TurnBoundary,
            Self::Interruption => EpisodeKind::Interruption,
            Self::Gap { .. } => EpisodeKind::Gap,
        }
    }
}

impl ContinuityStore {
    pub fn preview_chronicle_capture(
        &self,
        project_id: &str,
        locator_id: &str,
        mode: CaptureMode,
    ) -> Result<ChronicleCapturePreview, LeyCoreError> {
        let connection = self.open_connection()?;
        preview_chronicle_capture_on(&connection, project_id, locator_id, mode, self)
    }

    /// Called only by the trusted native Desktop control after native user confirmation.
    /// CLI, MCP and host hook adapters intentionally expose no grant operation.
    pub fn authorize_chronicle_capture(
        &self,
        preview: &ChronicleCapturePreview,
    ) -> Result<ChronicleCaptureGrant, LeyCoreError> {
        if preview.fingerprint != preview_fingerprint(preview)? {
            return Err(invalid_brain("capture preview was changed"));
        }
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        require_project_generation_on(&transaction, &preview.handle, self)?;
        let current = preview_chronicle_capture_on(
            &transaction,
            &preview.handle.project_id,
            &preview.locator_id,
            preview.mode,
            self,
        )?;
        if current != *preview {
            return Err(invalid_brain("capture scope changed since preview"));
        }
        require_locator_stamp(
            &transaction,
            &preview.handle.project_id,
            &preview.locator_id,
            &preview.root_stamp,
            self,
        )?;

        let existing = capture_state_on(
            &transaction,
            &preview.handle.project_id,
            &preview.locator_id,
            self,
        )?;
        let now = unix_time_ms()?;
        // Tightening/changing an existing active permission fences an in-flight hook that
        // began under the prior retention decision. Re-confirming the same mode does not
        // manufacture a project revision.
        let generation = if existing
            .as_ref()
            .is_some_and(|state| state.authorized && state.mode != preview.mode)
        {
            bump_project_generation_on(&transaction, &preview.handle.project_id, now, self)?
        } else {
            require_active_project_on(&transaction, &preview.handle.project_id, self)?.generation
        };
        let grant_id = format!("cgr_{}", Uuid::new_v4().simple());
        transaction
            .execute(
                "INSERT INTO chronicle_capture_grants(
                    project_id, locator_id, grant_id, generation, host, mode, root,
                    root_stamp, origin, authorized, authorized_at_unix_ms, revoked_at_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, 'codex', ?5, ?6, ?7, ?8, 1, ?9, NULL)
                 ON CONFLICT(project_id, locator_id) DO UPDATE SET
                    grant_id = excluded.grant_id,
                    generation = excluded.generation,
                    mode = excluded.mode,
                    root = excluded.root,
                    root_stamp = excluded.root_stamp,
                    authorized = 1,
                    authorized_at_unix_ms = excluded.authorized_at_unix_ms,
                    revoked_at_unix_ms = NULL",
                params![
                    preview.handle.project_id,
                    preview.locator_id,
                    grant_id,
                    u64_i64(generation, "capture generation")?,
                    preview.mode.to_string(),
                    preview
                        .root
                        .to_str()
                        .ok_or_else(|| { LeyCoreError::NonUtf8Path(preview.root.clone()) })?,
                    preview.root_stamp,
                    CAPTURE_ORIGIN,
                    u64_i64(now, "capture authorization time")?
                ],
            )
            .map_err(|error| self.database_error(error))?;
        let grant = capture_state_on(
            &transaction,
            &preview.handle.project_id,
            &preview.locator_id,
            self,
        )?
        .ok_or_else(|| invalid_brain("capture grant was not persisted"))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(grant)
    }

    pub fn chronicle_capture_state(
        &self,
        project_id: &str,
        locator_id: &str,
    ) -> Result<Option<ChronicleCaptureState>, LeyCoreError> {
        let connection = self.open_connection()?;
        require_active_project_on(&connection, project_id, self)?;
        let mut state = capture_state_on(&connection, project_id, locator_id, self)?;
        if let Some(current) = state
            .as_mut()
            .filter(|state| state.authorized && state.effective)
        {
            match crate::project_import::safe_root(&current.root) {
                Ok(root) => match crate::project_import::root_stamp(&root) {
                    Ok(stamp) if stamp == current.root_stamp => {}
                    Ok(_) => {
                        current.effective = false;
                        current.inactive_reason = Some("working-copy-directory-replaced".into());
                    }
                    Err(_) => {
                        current.effective = false;
                        current.inactive_reason = Some("working-copy-unavailable".into());
                    }
                },
                Err(_) => {
                    current.effective = false;
                    current.inactive_reason = Some("working-copy-unavailable".into());
                }
            }
        }
        Ok(state)
    }

    pub fn revoke_chronicle_capture(
        &self,
        project_id: &str,
        locator_id: &str,
        expected_grant_id: &str,
    ) -> Result<ChronicleCaptureState, LeyCoreError> {
        let mut connection = self.open_connection()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| self.database_error(e))?;
        require_active_project_on(&tx, project_id, self)?;
        let state = capture_state_on(&tx, project_id, locator_id, self)?
            .ok_or_else(|| invalid_brain("capture grant is absent"))?;
        if state.grant_id != expected_grant_id {
            return Err(invalid_brain("capture grant changed since review"));
        }
        if state.authorized {
            let now = unix_time_ms()?;
            let generation = bump_project_generation_on(&tx, project_id, now, self)?;
            tx.execute("UPDATE chronicle_capture_grants SET authorized=0,revoked_at_unix_ms=?1,generation=?2 WHERE project_id=?3 AND locator_id=?4",
                params![u64_i64(now,"capture revocation time")?,u64_i64(generation,"capture generation")?,project_id,locator_id])
                .map_err(|e| self.database_error(e))?;
        }
        let state = capture_state_on(&tx, project_id, locator_id, self)?
            .ok_or_else(|| invalid_brain("capture grant disappeared during revocation"))?;
        tx.commit().map_err(|e| self.database_error(e))?;
        Ok(state)
    }

    pub fn chronicle_sessions(
        &self,
        project_id: &str,
        offset: u64,
        max: usize,
    ) -> Result<ChroniclePage<ChronicleSession>, LeyCoreError> {
        validate_page(offset, max)?;
        let connection = self.open_connection()?;
        require_active_project_on(&connection, project_id, self)?;
        let mut statement = connection
            .prepare(
                "SELECT session_id FROM project_sessions WHERE project_id=?1
            ORDER BY created_at_unix_ms,session_id LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| self.database_error(e))?;
        let ids = statement
            .query_map(
                params![
                    project_id,
                    (max + 1) as i64,
                    u64_i64(offset, "session offset")?
                ],
                |r| r.get::<_, String>(0),
            )
            .map_err(|e| self.database_error(e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| self.database_error(e))?;
        let next_offset = (ids.len() > max).then_some(offset + max as u64);
        let mut items = Vec::new();
        for id in ids.into_iter().take(max) {
            let session =
                crate::project_brain::read_session_on(&connection, project_id, &id, self)?
                    .ok_or_else(|| invalid_brain("Chronicle session is absent"))?;
            let count: i64 = connection
                .query_row(
                    "SELECT count(*) FROM chronicle_episodes WHERE project_id=?1 AND session_id=?2",
                    params![project_id, id],
                    |r| r.get(0),
                )
                .map_err(|e| self.database_error(e))?;
            let gaps = if session.state == crate::ProjectSessionState::Erased {
                Vec::new()
            } else {
                vec!["Only supported observable events were retained; hidden reasoning and complete history are unavailable".into()]
            };
            items.push(ChronicleSession {
                session,
                episode_count: count as u64,
                gaps,
            });
        }
        Ok(ChroniclePage { items, next_offset })
    }

    pub fn chronicle_history(
        &self,
        project_id: &str,
        session_id: Option<&str>,
        after_sequence: u64,
        max: usize,
    ) -> Result<ChroniclePage<Episode>, LeyCoreError> {
        validate_page(after_sequence, max)?;
        let connection = self.open_connection()?;
        require_active_project_on(&connection, project_id, self)?;
        if let Some(id) = session_id {
            let session = crate::project_brain::read_session_on(&connection, project_id, id, self)?
                .ok_or_else(|| invalid_brain("Chronicle session is absent"))?;
            if session.state == crate::ProjectSessionState::Erased {
                return Err(invalid_brain("Chronicle session was erased"));
            }
        }
        let mut statement=connection.prepare("SELECT c.event_id,c.sequence,c.episode_kind,c.evidence_basis,c.producer,c.origin,
            c.external_turn_reference,c.external_tool_reference,c.occurred_at_unix_ms,c.gaps_json,e.recorded_at_unix_ms,e.kind,e.payload_json
            FROM chronicle_episodes c JOIN events e ON e.project_id=c.project_id AND e.event_id=c.event_id
            WHERE c.project_id=?1 AND c.session_id IS ?2 AND c.sequence>?3 ORDER BY c.sequence LIMIT ?4")
            .map_err(|e|self.database_error(e))?;
        let rows = statement
            .query_map(
                params![
                    project_id,
                    session_id,
                    u64_i64(after_sequence, "history sequence")?,
                    (max + 1) as i64
                ],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, Option<String>>(6)?,
                        r.get::<_, Option<String>>(7)?,
                        r.get::<_, Option<i64>>(8)?,
                        r.get::<_, String>(9)?,
                        r.get::<_, i64>(10)?,
                        r.get::<_, String>(11)?,
                        r.get::<_, String>(12)?,
                    ))
                },
            )
            .map_err(|e| self.database_error(e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| self.database_error(e))?;
        let has_more = rows.len() > max;
        let mut items = Vec::new();
        for (
            event_id,
            sequence,
            kind,
            basis,
            producer,
            origin,
            turn,
            tool,
            occurred,
            gaps,
            recorded,
            event_kind,
            payload,
        ) in rows.into_iter().take(max)
        {
            items.push(Episode {
                project_id: project_id.into(),
                event_id,
                session_id: session_id.map(str::to_owned),
                sequence: sequence as u64,
                kind: parse_enum(&kind)?,
                evidence_basis: parse_enum(&basis)?,
                producer,
                origin,
                external_turn_reference: turn,
                external_tool_reference: tool,
                occurred_at_unix_ms: occurred.map(|v| v as u64),
                recorded_at_unix_ms: recorded as u64,
                gaps: serde_json::from_str(&gaps).map_err(|e| invalid_brain(&e.to_string()))?,
                event_kind,
                payload: serde_json::from_str(&payload)
                    .map_err(|e| invalid_brain(&e.to_string()))?,
            });
        }
        let next_offset = if has_more {
            items.last().map(|e| e.sequence)
        } else {
            None
        };
        Ok(ChroniclePage { items, next_offset })
    }

    pub fn legacy_hook_root_matches(
        &self,
        project_id: &str,
        root: &Path,
    ) -> Result<bool, LeyCoreError> {
        let connection = self.open_connection()?;
        let observed: Option<String> = connection
            .query_row(
                "SELECT root_path FROM projects WHERE project_id=?1",
                [project_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| self.database_error(e))?
            .flatten();
        let Some(observed) = observed else {
            return Ok(false);
        };
        Ok(crate::project_import::safe_root(root)? == PathBuf::from(observed))
    }
}

pub(crate) fn resolve_codex_capture_on(
    connection: &Connection,
    project_start: &Path,
    store: &ContinuityStore,
) -> Result<ChronicleCaptureRoute, LeyCoreError> {
    let lexical = if project_start.is_absolute() {
        project_start.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| LeyCoreError::Io {
                path: project_start.to_path_buf(),
                source,
            })?
            .join(project_start)
    };
    let lexical = lexical
        .components()
        .fold(PathBuf::new(), |mut path, component| {
            use std::path::Component;
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    path.pop();
                }
                _ => path.push(component),
            }
            path
        });

    let canonical_detection = std::fs::canonicalize(&lexical).ok();
    let mut statement = connection
        .prepare(
            "SELECT project_id, locator_id, local_path
             FROM working_copy_locators
             WHERE state='authorized'
             ORDER BY length(local_path) DESC, project_id, locator_id",
        )
        .map_err(|error| store.database_error(error))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| store.database_error(error))?;
    let mut candidates = Vec::new();
    for row in rows {
        let (project_id, locator_id, root_text) =
            row.map_err(|error| store.database_error(error))?;
        let root = PathBuf::from(&root_text);
        let detected = lexical.starts_with(&root)
            || canonical_detection
                .as_ref()
                .is_some_and(|path| path.starts_with(&root));
        if detected {
            candidates.push((project_id, locator_id, root));
        }
    }
    drop(statement);

    if candidates.is_empty() {
        return Ok(ChronicleCaptureRoute::NotProjectBrain);
    }
    if candidates.len() != 1 {
        return Ok(ChronicleCaptureRoute::Disabled(
            "working directory overlaps multiple authorized Project Brain working copies".into(),
        ));
    }
    let (project_id, locator_id, root) = candidates
        .pop()
        .ok_or_else(|| invalid_brain("Chronicle capture candidate disappeared"))?;
    let safe_cwd = match crate::project_import::safe_root(&lexical) {
        Ok(path) => path,
        Err(_) => {
            return Ok(ChronicleCaptureRoute::Disabled(
                "working directory could not be acquired without following substitutions".into(),
            ))
        }
    };
    if !safe_cwd.starts_with(&root) {
        return Ok(ChronicleCaptureRoute::Disabled(
            "working directory no longer resolves inside the authorized working copy".into(),
        ));
    }
    let lifecycle = match require_active_project_on(connection, &project_id, store) {
        Ok(value) => value,
        Err(_) => {
            return Ok(ChronicleCaptureRoute::Disabled(
                "Project Brain is not active".into(),
            ))
        }
    };
    let state = match capture_state_on(connection, &project_id, &locator_id, store)? {
        Some(state) if state.authorized && state.effective => state,
        Some(state) => {
            return Ok(ChronicleCaptureRoute::Disabled(
                state
                    .inactive_reason
                    .unwrap_or_else(|| "Codex capture permission is inactive".into()),
            ))
        }
        None => {
            return Ok(ChronicleCaptureRoute::Disabled(
                "Codex capture has not been authorized for this working copy".into(),
            ))
        }
    };
    if state.root != root {
        return Ok(ChronicleCaptureRoute::Disabled(
            "capture permission does not match the current working-copy path".into(),
        ));
    }
    let observed_stamp = match crate::project_import::root_stamp(&root) {
        Ok(stamp) => stamp,
        Err(_) => {
            return Ok(ChronicleCaptureRoute::Disabled(
                "authorized working-copy identity is unavailable".into(),
            ))
        }
    };
    if observed_stamp != state.root_stamp {
        return Ok(ChronicleCaptureRoute::Disabled(
            "authorized working-copy directory was replaced".into(),
        ));
    }
    Ok(ChronicleCaptureRoute::Enabled(ActiveChronicleCapture {
        handle: ProjectHandle {
            project_id,
            generation: lifecycle.generation,
        },
        mode: state.mode,
    }))
}

fn preview_chronicle_capture_on(
    connection: &Connection,
    project_id: &str,
    locator_id: &str,
    mode: CaptureMode,
    store: &ContinuityStore,
) -> Result<ChronicleCapturePreview, LeyCoreError> {
    let lifecycle = require_active_project_on(connection, project_id, store)?;
    let locator = read_working_copy_on(connection, project_id, locator_id, store)?
        .ok_or_else(|| invalid_brain("capture locator is absent"))?;
    if locator.state != WorkingCopyState::Authorized {
        return Err(invalid_brain("capture locator is revoked"));
    }
    let root = crate::project_import::safe_root(&locator.local_path)?;
    let root_stamp = crate::project_import::root_stamp(&root)?;
    require_locator_stamp(connection, project_id, locator_id, &root_stamp, store)?;
    let mut preview = ChronicleCapturePreview {
        handle: ProjectHandle {
            project_id: project_id.to_owned(),
            generation: lifecycle.generation,
        },
        locator_id: locator_id.to_owned(),
        root,
        root_stamp,
        mode,
        fingerprint: String::new(),
        host: "codex".into(),
        scope: vec![
            "Future supported Codex session boundaries and visible gaps".into(),
            "Supported visible user prompts, assistant responses and Bash results".into(),
        ],
        exclusions: vec![
            "Hidden reasoning and raw transcript files".into(),
            "Historical host data and other projects".into(),
            "Model sharing and derived analysis".into(),
            "Tool activity not emitted by the configured Codex hooks".into(),
        ],
    };
    if mode == CaptureMode::Minimal {
        preview.scope = vec![
            "Future supported Codex activity metadata and visible gaps; no message, command or result bodies".into(),
        ];
    }
    preview.fingerprint = preview_fingerprint(&preview)?;
    Ok(preview)
}

fn validate_page(offset: u64, max: usize) -> Result<(), LeyCoreError> {
    if max == 0 || max > MAX_CHRONICLE_PAGE || offset > i64::MAX as u64 {
        return Err(invalid_brain(
            "Chronicle inspection requires max 1..100 and an in-range offset",
        ));
    }
    Ok(())
}
fn parse_enum<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, LeyCoreError> {
    serde_json::from_value(Value::String(value.into())).map_err(|e| invalid_brain(&e.to_string()))
}
fn enum_text<T: Serialize>(value: &T) -> Result<String, LeyCoreError> {
    serde_json::to_value(value)
        .map_err(|e| invalid_brain(&e.to_string()))?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid_brain("Chronicle enum is not a string"))
}
fn preview_fingerprint(preview: &ChronicleCapturePreview) -> Result<String, LeyCoreError> {
    let mut value = serde_json::to_value(preview).map_err(|e| invalid_brain(&e.to_string()))?;
    value
        .as_object_mut()
        .ok_or_else(|| invalid_brain("Chronicle capture preview is not an object"))?
        .remove("fingerprint");
    request_fingerprint(&value)
}
pub(crate) fn require_locator_stamp(
    connection: &Connection,
    project_id: &str,
    locator_id: &str,
    stamp: &str,
    store: &ContinuityStore,
) -> Result<(), LeyCoreError> {
    let stored:Option<String>=connection.query_row("SELECT root_identity FROM working_copy_locators WHERE project_id=?1 AND locator_id=?2 AND state='authorized'",
        params![project_id,locator_id],|r|r.get(0)).optional().map_err(|e|store.database_error(e))?.flatten();
    if stored.as_deref() != Some(stamp) {
        return Err(invalid_brain(
            "authorized capture root changed or locator revoked",
        ));
    }
    Ok(())
}
pub(crate) fn capture_state_on(
    connection: &Connection,
    project_id: &str,
    locator_id: &str,
    store: &ContinuityStore,
) -> Result<Option<ChronicleCaptureState>, LeyCoreError> {
    let row=connection.query_row("SELECT grant_id,generation,mode,root,root_stamp,origin,authorized,authorized_at_unix_ms,revoked_at_unix_ms
        FROM chronicle_capture_grants WHERE project_id=?1 AND locator_id=?2",params![project_id,locator_id],|r| {
        Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,
            r.get::<_,bool>(6)?,r.get::<_,i64>(7)?,r.get::<_,Option<i64>>(8)?))}).optional().map_err(|e|store.database_error(e))?;
    row.map(
        |(grant_id, generation, mode, root, root_stamp, origin, authorized, at, revoked)| {
            let locator = connection
                .query_row(
                    "SELECT local_path, state, root_identity
                     FROM working_copy_locators WHERE project_id=?1 AND locator_id=?2",
                    params![project_id, locator_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(|error| store.database_error(error))?;
            let inactive_reason = if !authorized {
                Some("revoked".to_owned())
            } else {
                match locator {
                    None => Some("working-copy-missing".to_owned()),
                    Some((_, state, _)) if state != "authorized" => {
                        Some("working-copy-revoked".to_owned())
                    }
                    Some((local_path, _, _)) if local_path != root => {
                        Some("working-copy-moved".to_owned())
                    }
                    Some((_, _, identity)) if identity.as_deref() != Some(root_stamp.as_str()) => {
                        Some("working-copy-identity-changed".to_owned())
                    }
                    Some(_) => None,
                }
            };
            Ok(ChronicleCaptureState {
                project_id: project_id.into(),
                locator_id: locator_id.into(),
                grant_id,
                generation: u64::try_from(generation)
                    .map_err(|_| invalid_brain("capture generation is negative"))?,
                mode: CaptureMode::parse(&mode)?,
                root: root.into(),
                root_stamp,
                origin,
                authorized,
                effective: inactive_reason.is_none(),
                inactive_reason,
                authorized_at_unix_ms: u64::try_from(at)
                    .map_err(|_| invalid_brain("capture authorization time is negative"))?,
                revoked_at_unix_ms: revoked
                    .map(|value| {
                        u64::try_from(value)
                            .map_err(|_| invalid_brain("capture revocation time is negative"))
                    })
                    .transpose()?,
            })
        },
    )
    .transpose()
}

pub(crate) fn normalize_event_on(
    connection: &Connection,
    event: &ContinuityEventInput,
    path: &Path,
) -> Result<(), LeyCoreError> {
    let store = ContinuityStore::at(path.to_owned());
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM chronicle_episodes WHERE project_id=?1 AND event_id=?2)",
            params![event.project_id, event.event_id],
            |r| r.get(0),
        )
        .map_err(|e| store.database_error(e))?;
    if exists {
        return Ok(());
    }
    let legacy_kind = event.kind.strip_prefix("legacy-").unwrap_or(&event.kind);
    let data = &event.payload["data"];
    let typed = event.kind == CHRONICLE_EVENT_KIND;
    let kind = if typed {
        serde_json::from_value::<ChronicleObservation>(event.payload["observation"].clone())
            .map_err(|e| invalid_brain(&format!("invalid typed Chronicle observation: {e}")))?
            .kind()
    } else {
        match legacy_kind {
            "session-started" => EpisodeKind::SessionStart,
            "session-finished" => EpisodeKind::SessionEnd,
            "user-prompt-observed" => EpisodeKind::UserPrompt,
            "assistant-response-observed" => EpisodeKind::AgentResponse,
            "tool-observed" => EpisodeKind::ToolResult,
            "checkpoint-recorded" | "recovery-checkpoint-recorded" => {
                EpisodeKind::StructuredSubmission
            }
            "episode-recorded" => match event.payload["episodeKind"].as_str() {
                Some("user-prompt") => EpisodeKind::UserPrompt,
                Some("agent-response") => EpisodeKind::AgentResponse,
                Some("tool-call") => EpisodeKind::ToolCall,
                Some("tool-result") => EpisodeKind::ToolResult,
                Some("gap") => EpisodeKind::Gap,
                _ => EpisodeKind::StructuredSubmission,
            },
            "source-version-retained" => EpisodeKind::SourceCapture,
            "project-imported" => EpisodeKind::ProjectImport,
            "source-created"
            | "source-locator-attached"
            | "human-action-recorded"
            | "working-copy-relocated" => EpisodeKind::ProjectActivity,
            _ => return Ok(()),
        }
    };
    let basis = if typed {
        EpisodeEvidenceBasis::Observed
    } else if matches!(kind, EpisodeKind::StructuredSubmission) {
        EpisodeEvidenceBasis::Reported
    } else if matches!(
        kind,
        EpisodeKind::SourceCapture | EpisodeKind::ProjectImport | EpisodeKind::ProjectActivity
    ) {
        EpisodeEvidenceBasis::Observed
    } else if matches!(
        legacy_kind,
        "user-prompt-observed" | "assistant-response-observed" | "tool-observed"
    ) && data["origin"].as_str() == Some("host-hook")
    {
        EpisodeEvidenceBasis::Observed
    } else {
        EpisodeEvidenceBasis::Reported
    };
    let producer = if typed {
        "codex"
    } else {
        data["source"]["host"]
            .as_str()
            .or_else(|| data["host"].as_str())
            .unwrap_or("unknown")
    };
    let origin = if typed {
        "codex-hooks-v1"
    } else if legacy_kind == "episode-recorded" {
        "m1-structured-recorder"
    } else if event.kind.starts_with("legacy-") {
        "legacy-structured-event"
    } else {
        "structured-event"
    };
    let gaps = if typed {
        event.payload["gaps"]
            .as_array()
            .map(|v| {
                v.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else if matches!(
        kind,
        EpisodeKind::StructuredSubmission | EpisodeKind::SessionStart | EpisodeKind::SessionEnd
    ) {
        vec![
            "Historical structured submission; complete observable chronology is unavailable"
                .into(),
        ]
    } else if legacy_kind == "tool-observed" {
        vec![
            "Historical tool observation is evidence of the retained observation only; it does not establish semantic success"
                .into(),
        ]
    } else {
        Vec::new()
    };
    if let Some(session_id) = &event.session_id {
        let started = if matches!(kind, EpisodeKind::SessionStart) {
            Some(event.recorded_at_unix_ms)
        } else {
            None
        };
        connection.execute("INSERT INTO project_sessions(project_id,session_id,host_kind,external_session_id,state,started_at_unix_ms,ended_at_unix_ms,created_at_unix_ms,erased_at_unix_ms,observation_limits_json)
            VALUES (?1,?2,?3,NULL,'active',?4,NULL,?5,NULL,'{}') ON CONFLICT(project_id,session_id) DO NOTHING",
            params![event.project_id,session_id,if producer=="unknown"{None}else{Some(producer)},started.map(|v|v as i64),u64_i64(event.recorded_at_unix_ms.max(1),"session creation time")?])
            .map_err(|e|store.database_error(e))?;
        if !typed && kind == EpisodeKind::SessionEnd {
            connection.execute("UPDATE project_sessions SET state='finished',ended_at_unix_ms=?1 WHERE project_id=?2 AND session_id=?3 AND state='active'",
                params![u64_i64(event.recorded_at_unix_ms.max(1),"session end time")?,event.project_id,session_id]).map_err(|e|store.database_error(e))?;
        }
    }
    let sequence:i64=connection.query_row("SELECT coalesce(max(sequence),0)+1 FROM chronicle_episodes WHERE project_id=?1 AND session_id IS ?2",
        params![event.project_id,event.session_id],|r|r.get(0)).map_err(|e|store.database_error(e))?;
    connection.execute("INSERT INTO chronicle_episodes(project_id,event_id,session_id,sequence,episode_kind,evidence_basis,producer,origin,external_turn_reference,external_tool_reference,occurred_at_unix_ms,gaps_json)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![event.project_id,event.event_id,event.session_id,sequence,enum_text(&kind)?,enum_text(&basis)?,producer,origin,
            if typed{event.payload["turnReference"].as_str()}else{None},if typed{event.payload["toolReference"].as_str()}else{None},
            event.payload["occurredAtUnixMs"].as_u64().map(|v|u64_i64(v,"episode occurrence time")).transpose()?,serde_json::to_string(&gaps).map_err(|e|invalid_brain(&e.to_string()))?])
        .map_err(|e|store.database_error(e))?;
    Ok(())
}

pub(crate) fn backfill_on(connection: &Connection, path: &Path) -> Result<(), LeyCoreError> {
    let store = ContinuityStore::at(path.to_owned());
    let mut statement=connection.prepare("SELECT project_id,event_id,subject_id,session_id,session_sequence,request_id,request_fingerprint,kind,payload_version,recorded_at_unix_ms,revision_head,revision_branch,payload_json
        FROM events ORDER BY project_id,session_id,coalesce(session_sequence,0),recorded_at_unix_ms,event_id")
        .map_err(|e|store.database_error(e))?;
    let mut rows = statement.query([]).map_err(|e| store.database_error(e))?;
    while let Some(r) = rows.next().map_err(|e| store.database_error(e))? {
        let event = ContinuityEventInput {
            project_id: r.get(0).map_err(|e| store.database_error(e))?,
            event_id: r.get(1).map_err(|e| store.database_error(e))?,
            subject_id: r.get(2).map_err(|e| store.database_error(e))?,
            session_id: r.get(3).map_err(|e| store.database_error(e))?,
            session_sequence: r
                .get::<_, Option<i64>>(4)
                .map_err(|e| store.database_error(e))?
                .map(|v| v as u64),
            request_id: r.get(5).map_err(|e| store.database_error(e))?,
            request_fingerprint: r.get(6).map_err(|e| store.database_error(e))?,
            kind: r.get(7).map_err(|e| store.database_error(e))?,
            payload_version: r.get::<_, i64>(8).map_err(|e| store.database_error(e))? as u32,
            recorded_at_unix_ms: r.get::<_, i64>(9).map_err(|e| store.database_error(e))? as u64,
            revision_head: r.get(10).map_err(|e| store.database_error(e))?,
            revision_branch: r.get(11).map_err(|e| store.database_error(e))?,
            payload: serde_json::from_str(
                &r.get::<_, String>(12)
                    .map_err(|e| store.database_error(e))?,
            )
            .map_err(|e| invalid_brain(&e.to_string()))?,
        };
        let erased = if let Some(id) = &event.session_id {
            connection.query_row("SELECT EXISTS(SELECT 1 FROM project_sessions WHERE project_id=?1 AND session_id=?2 AND state='erased')",params![event.project_id,id],|r|r.get::<_,bool>(0)).map_err(|e|store.database_error(e))?
        } else {
            false
        };
        if !erased {
            normalize_event_on(connection, &event, path)?;
        }
    }
    Ok(())
}

pub(crate) fn append_observation_on(
    connection: &Connection,
    store: &ContinuityStore,
    handle: &ProjectHandle,
    session_id: &str,
    request_id: &str,
    fingerprint: &str,
    observation: &ChronicleObservation,
    turn: Option<&str>,
    tool: Option<&str>,
    gaps: &[String],
) -> Result<ContinuityEvent, LeyCoreError> {
    if let Some(existing) = read_event_by_request(
        connection,
        &handle.project_id,
        Some(session_id),
        request_id,
        store.path(),
    )? {
        if existing.request_fingerprint.as_deref() != Some(fingerprint) {
            return Err(invalid_brain(
                "stable hook event ID was reused with different content",
            ));
        }
        return Ok(existing);
    }
    let sequence:i64=connection.query_row("SELECT coalesce(max(session_sequence),0)+1 FROM events WHERE project_id=?1 AND session_id=?2",params![handle.project_id,session_id],|r|r.get(0)).map_err(|e|store.database_error(e))?;
    let event = ContinuityEventInput {
        event_id: format!("evt_{}", Uuid::new_v4().simple()),
        project_id: handle.project_id.clone(),
        subject_id: None,
        session_id: Some(session_id.into()),
        session_sequence: Some(sequence as u64),
        request_id: Some(request_id.into()),
        request_fingerprint: Some(fingerprint.into()),
        kind: CHRONICLE_EVENT_KIND.into(),
        payload_version: 1,
        recorded_at_unix_ms: unix_time_ms()?,
        revision_head: None,
        revision_branch: None,
        payload: json!({"projectGeneration":handle.generation,"observation":observation,"turnReference":turn,"toolReference":tool,"gaps":gaps}),
    };
    Ok(append_project_brain_event_on(connection, &event, store.path())?.record)
}
