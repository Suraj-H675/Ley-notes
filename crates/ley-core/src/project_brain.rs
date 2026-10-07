use crate::continuity_store::{append_project_brain_event_on, read_event_by_request};
use crate::{
    validate_project_id, ContinuityEvent, ContinuityEventInput, ContinuityStore, LeyCoreError,
    ProjectIdentity, PROJECT_SCHEMA_VERSION,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const MAX_PROJECT_BRAIN_SOURCE_BYTES: usize = 1_048_576;

const EVENT_KIND_SOURCE_VERSION_RETAINED: &str = "source-version-retained";
const EVENT_KIND_SOURCE_CREATED: &str = "source-created";
const EVENT_KIND_SOURCE_LOCATOR_ATTACHED: &str = "source-locator-attached";
const EVENT_KIND_EPISODE_RECORDED: &str = "episode-recorded";
const EVENT_KIND_HUMAN_ACTION_RECORDED: &str = "human-action-recorded";
const HUMAN_CONTROL_ORIGIN: &str = "local-user-control";
const SUPPORTED_OBSERVED_EPISODE_KINDS: &[&str] = &[
    "user-prompt",
    "agent-response",
    "tool-call",
    "tool-result",
    "verification-result",
    "gap",
];

pub(crate) fn is_reserved_project_brain_event_kind(kind: &str) -> bool {
    matches!(
        kind,
        crate::chronicle::CHRONICLE_EVENT_KIND
            | EVENT_KIND_SOURCE_VERSION_RETAINED
            | EVENT_KIND_SOURCE_CREATED
            | EVENT_KIND_SOURCE_LOCATOR_ATTACHED
            | EVENT_KIND_EPISODE_RECORDED
            | "project-imported"
            | "working-copy-relocated"
            | EVENT_KIND_HUMAN_ACTION_RECORDED
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateProjectBrainInput {
    pub name: String,
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectHandle {
    pub project_id: String,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBrain {
    pub identity: ProjectIdentity,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectRepositoryAttachment {
    pub project_id: String,
    pub repository_id: String,
    pub attached_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkingCopyState {
    Authorized,
    Revoked,
}

impl WorkingCopyState {
    fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "authorized" => Ok(Self::Authorized),
            "revoked" => Ok(Self::Revoked),
            _ => Err(LeyCoreError::InvalidContinuityStore(format!(
                "invalid working-copy state {value:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkingCopyLocator {
    pub project_id: String,
    pub repository_id: String,
    pub locator_id: String,
    pub local_path: PathBuf,
    pub state: WorkingCopyState,
    pub authorized_at_unix_ms: u64,
    pub revoked_at_unix_ms: Option<u64>,
    pub last_observed_at_unix_ms: u64,
    pub revision_head: Option<String>,
    pub revision_branch: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectSourceState {
    Active,
    Removed,
    Erased,
}

impl ProjectSourceState {
    fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "active" => Ok(Self::Active),
            "removed" => Ok(Self::Removed),
            "erased" => Ok(Self::Erased),
            _ => Err(LeyCoreError::InvalidContinuityStore(format!(
                "invalid Project Brain source state {value:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceLocatorState {
    Observed,
    Revoked,
}

impl SourceLocatorState {
    fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "observed" => Ok(Self::Observed),
            "revoked" => Ok(Self::Revoked),
            _ => Err(LeyCoreError::InvalidContinuityStore(format!(
                "invalid Project Brain source locator state {value:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLocator {
    pub project_id: String,
    pub source_id: String,
    pub locator_id: String,
    pub locator_kind: String,
    pub locator_value: String,
    pub state: SourceLocatorState,
    pub observed_at_unix_ms: u64,
    pub revoked_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSource {
    pub project_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub display_name: String,
    pub state: ProjectSourceState,
    pub created_at_unix_ms: u64,
    pub removed_at_unix_ms: Option<u64>,
    pub erased_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceVersionInput {
    pub request_id: String,
    pub representation_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_content_hash: Option<String>,
    pub source_bytes: u64,
    #[serde(default)]
    pub transformation: Value,
    pub retained_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
// M1 persists this boundary but deliberately does not expose it through CLI/MCP; the human
// control surface arrives with the later Desktop review milestone.
#[allow(dead_code)]
pub(crate) enum HumanActionEvidenceInput {
    SourceVersion {
        source_id: String,
        source_version_id: String,
        selector: Value,
    },
    Episode {
        event_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct LocalHumanActionInput {
    pub request_id: String,
    pub action_kind: String,
    pub exact_target: Value,
    pub evidence: Vec<HumanActionEvidenceInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetainedSourceVersion {
    pub project_id: String,
    pub source_id: String,
    pub source_version_id: String,
    pub representation_kind: String,
    pub content_hash: String,
    pub original_content_hash: Option<String>,
    pub source_bytes: u64,
    pub stored_bytes: u64,
    pub retained_at_unix_ms: u64,
    pub occurrence_event_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectSessionState {
    Active,
    Finished,
    Erased,
}

impl ProjectSessionState {
    fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "active" => Ok(Self::Active),
            "finished" => Ok(Self::Finished),
            "erased" => Ok(Self::Erased),
            _ => Err(LeyCoreError::InvalidContinuityStore(format!(
                "invalid Project Brain session state {value:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSession {
    pub project_id: String,
    pub session_id: String,
    pub host_kind: Option<String>,
    pub external_session_id: Option<String>,
    pub state: ProjectSessionState,
    pub started_at_unix_ms: Option<u64>,
    pub ended_at_unix_ms: Option<u64>,
    pub created_at_unix_ms: u64,
    pub erased_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectLifecycleState {
    Active,
    Erasing,
    Erased,
}

impl ProjectLifecycleState {
    fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "active" => Ok(Self::Active),
            "erasing" => Ok(Self::Erasing),
            "erased" => Ok(Self::Erased),
            _ => Err(LeyCoreError::InvalidContinuityStore(format!(
                "invalid project lifecycle state {value:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectLifecycle {
    pub generation: u64,
    pub state: ProjectLifecycleState,
    pub cleanup_kind: String,
    pub cleanup_json: Option<String>,
}

impl ContinuityStore {
    pub fn create_project_brain(
        &self,
        input: &CreateProjectBrainInput,
    ) -> Result<ProjectBrain, LeyCoreError> {
        validate_display_name(&input.name, 128, "project name")?;
        validate_request_id(&input.request_id)?;
        let fingerprint = request_fingerprint(&json!({"name": input.name.trim()}))?;
        let now = unix_time_ms()?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;

        if let Some((project_id, stored_fingerprint, state, generation)) = transaction
            .query_row(
                "SELECT project_id, create_request_fingerprint, state, generation
                 FROM project_lifecycle WHERE create_request_id = ?1",
                [&input.request_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| self.database_error(error))?
        {
            if stored_fingerprint.as_deref() != Some(fingerprint.as_str()) {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "Project Brain create request {} was reused with different content",
                    input.request_id
                )));
            }
            match ProjectLifecycleState::parse(&state)? {
                ProjectLifecycleState::Active => {}
                ProjectLifecycleState::Erasing => {
                    return Err(LeyCoreError::ProjectErasing { project_id })
                }
                ProjectLifecycleState::Erased => {
                    return Err(LeyCoreError::ProjectErased { project_id })
                }
            }
            let identity =
                read_project_identity_on(&transaction, &project_id, self)?.ok_or_else(|| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "active Project Brain {project_id} is missing its project row"
                    ))
                })?;
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return Ok(ProjectBrain {
                identity,
                generation: sqlite_u64(generation, "project generation")?,
            });
        }

        let identity = ProjectIdentity {
            schema_version: PROJECT_SCHEMA_VERSION,
            project_id: format!("prj_{}", Uuid::new_v4().simple()),
            name: input.name.trim().to_owned(),
            created_at_unix_ms: now,
        };
        transaction
            .execute(
                "INSERT INTO projects(
                    project_id, name, created_at_unix_ms, agent_egress_policy,
                    agent_egress_policy_migrated, approved_source_authority_migrated
                 ) VALUES (?1, ?2, ?3, 'never-send', 1, 1)",
                params![
                    identity.project_id,
                    identity.name,
                    u64_i64(now, "project created time")?
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .execute(
                "INSERT INTO project_lifecycle(
                    project_id, generation, state, updated_at_unix_ms,
                    create_request_id, create_request_fingerprint, cleanup_kind, cleanup_json
                 ) VALUES (?1, 1, 'active', ?2, ?3, ?4, 'none', NULL)",
                params![
                    identity.project_id,
                    u64_i64(now, "project lifecycle time")?,
                    input.request_id,
                    fingerprint
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(ProjectBrain {
            identity,
            generation: 1,
        })
    }

    pub fn open_project_brain(&self, project_id: &str) -> Result<ProjectBrain, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        let lifecycle = require_active_project_on(&connection, project_id, self)?;
        let identity =
            read_project_identity_on(&connection, project_id, self)?.ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "active Project Brain {project_id} is missing its project row"
                ))
            })?;
        Ok(ProjectBrain {
            identity,
            generation: lifecycle.generation,
        })
    }

    pub fn list_project_brains(&self) -> Result<Vec<ProjectBrain>, LeyCoreError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT p.project_id, p.name, p.created_at_unix_ms, l.generation
                 FROM projects p
                 JOIN project_lifecycle l ON l.project_id = p.project_id
                 WHERE l.state = 'active'
                 ORDER BY p.created_at_unix_ms DESC, p.project_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|error| self.database_error(error))?;
        let mut brains = Vec::new();
        for row in rows {
            let (project_id, name, created_at, generation) =
                row.map_err(|error| self.database_error(error))?;
            brains.push(ProjectBrain {
                identity: ProjectIdentity {
                    schema_version: PROJECT_SCHEMA_VERSION,
                    project_id,
                    name,
                    created_at_unix_ms: sqlite_u64(created_at, "project created time")?,
                },
                generation: sqlite_u64(generation, "project generation")?,
            });
        }
        Ok(brains)
    }

    pub fn project_repository(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectRepositoryAttachment>, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        require_active_project_on(&connection, project_id, self)?;
        read_repository_on(&connection, project_id, self)
    }

    pub fn working_copies(
        &self,
        project_id: &str,
    ) -> Result<Vec<WorkingCopyLocator>, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        require_active_project_on(&connection, project_id, self)?;
        let mut statement = connection
            .prepare(
                "SELECT repository_id, locator_id, local_path, state, authorized_at_unix_ms,
                        revoked_at_unix_ms, last_observed_at_unix_ms, revision_head, revision_branch
                 FROM working_copy_locators
                 WHERE project_id = ?1
                 ORDER BY authorized_at_unix_ms, locator_id",
            )
            .map_err(|error| self.database_error(error))?;
        let rows = statement
            .query_map([project_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                ))
            })
            .map_err(|error| self.database_error(error))?;
        let mut locators = Vec::new();
        for row in rows {
            let (
                repository_id,
                locator_id,
                local_path,
                state,
                authorized_at,
                revoked_at,
                last_observed_at,
                revision_head,
                revision_branch,
            ) = row.map_err(|error| self.database_error(error))?;
            locators.push(WorkingCopyLocator {
                project_id: project_id.to_owned(),
                repository_id,
                locator_id,
                local_path: PathBuf::from(local_path),
                state: WorkingCopyState::parse(&state)?,
                authorized_at_unix_ms: sqlite_u64(
                    authorized_at,
                    "working-copy authorization time",
                )?,
                revoked_at_unix_ms: revoked_at
                    .map(|value| sqlite_u64(value, "working-copy revocation time"))
                    .transpose()?,
                last_observed_at_unix_ms: sqlite_u64(
                    last_observed_at,
                    "working-copy last-observed time",
                )?,
                revision_head,
                revision_branch,
            });
        }
        Ok(locators)
    }

    pub fn authorize_working_copy(
        &self,
        handle: &ProjectHandle,
        root: impl AsRef<Path>,
        request_id: &str,
    ) -> Result<(ProjectRepositoryAttachment, WorkingCopyLocator), LeyCoreError> {
        validate_request_id(request_id)?;
        let root = crate::project_import::safe_root(root.as_ref())?;
        let root_text = root
            .to_str()
            .ok_or_else(|| LeyCoreError::NonUtf8Path(root.clone()))?
            .to_owned();
        let now = unix_time_ms()?;
        let fingerprint = request_fingerprint(&json!({
            "action": "authorize-working-copy",
            "projectId": handle.project_id,
            "generation": handle.generation,
            "path": root_text,
        }))?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        require_project_generation_on(&transaction, handle, self)?;

        if let Some(existing_event) = read_event_by_request(
            &transaction,
            &handle.project_id,
            None,
            request_id,
            self.path(),
        )? {
            if existing_event.request_fingerprint.as_deref() != Some(fingerprint.as_str()) {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "working-copy authorization request {request_id} was reused with different content"
                )));
            }
            let repository_id = required_payload_string(&existing_event, "repositoryId")?;
            let locator_id = required_payload_string(&existing_event, "locatorId")?;
            let repository = read_repository_on(&transaction, &handle.project_id, self)?
                .ok_or_else(|| invalid_brain("authorized repository disappeared"))?;
            let locator =
                read_working_copy_on(&transaction, &handle.project_id, &locator_id, self)?
                    .ok_or_else(|| invalid_brain("authorized working copy disappeared"))?;
            if repository.repository_id != repository_id {
                return Err(invalid_brain("authorization replay repository changed"));
            }
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return Ok((repository, locator));
        }

        let repository = match read_repository_on(&transaction, &handle.project_id, self)? {
            Some(repository) => repository,
            None => {
                let repository = ProjectRepositoryAttachment {
                    project_id: handle.project_id.clone(),
                    repository_id: format!("repo_{}", Uuid::new_v4().simple()),
                    attached_at_unix_ms: now,
                };
                transaction
                    .execute(
                        "INSERT INTO project_repositories(project_id, repository_id, attached_at_unix_ms)
                         VALUES (?1, ?2, ?3)",
                        params![
                            repository.project_id,
                            repository.repository_id,
                            u64_i64(now, "repository attachment time")?
                        ],
                    )
                    .map_err(|error| self.database_error(error))?;
                repository
            }
        };

        if let Some((existing_project, existing_locator)) = transaction
            .query_row(
                "SELECT project_id, locator_id FROM working_copy_locators
                 WHERE local_path = ?1 AND state = 'authorized'",
                [&root_text],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| self.database_error(error))?
        {
            if existing_project != handle.project_id {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "working copy {} is already authorized for another Project Brain",
                    root.display()
                )));
            }
            return Err(LeyCoreError::InvalidContinuityStore(format!(
                "working copy {} is already authorized as {existing_locator}; reuse the original authorization request or revoke it explicitly",
                root.display()
            )));
        }

        let locator = WorkingCopyLocator {
            project_id: handle.project_id.clone(),
            repository_id: repository.repository_id.clone(),
            locator_id: format!("wcp_{}", Uuid::new_v4().simple()),
            local_path: root.clone(),
            state: WorkingCopyState::Authorized,
            authorized_at_unix_ms: now,
            revoked_at_unix_ms: None,
            last_observed_at_unix_ms: now,
            revision_head: None,
            revision_branch: None,
        };
        transaction
            .execute(
                "INSERT INTO working_copy_locators(
                    project_id, repository_id, locator_id, local_path, state,
                    authorized_at_unix_ms, revoked_at_unix_ms, last_observed_at_unix_ms,
                    revision_head, revision_branch
                 ) VALUES (?1, ?2, ?3, ?4, 'authorized', ?5, NULL, ?5, NULL, NULL)",
                params![
                    locator.project_id,
                    locator.repository_id,
                    locator.locator_id,
                    root_text,
                    u64_i64(now, "working-copy authorization time")?
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction.execute("UPDATE working_copy_locators SET root_identity=?1 WHERE project_id=?2 AND locator_id=?3",params![crate::project_import::root_stamp(&root)?,handle.project_id,locator.locator_id]).map_err(|e|self.database_error(e))?;
        let event = human_action_event(
            handle,
            request_id,
            &fingerprint,
            now,
            "authorize-working-copy",
            json!({
                "repositoryId": repository.repository_id,
                "locatorId": locator.locator_id,
                "pathDigest": format!("sha256:{:x}", Sha256::digest(root_text.as_bytes())),
            }),
        );
        append_project_brain_event_on(&transaction, &event, self.path())?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok((repository, locator))
    }

    pub fn revoke_working_copy(
        &self,
        handle: &ProjectHandle,
        locator_id: &str,
        request_id: &str,
    ) -> Result<ProjectHandle, LeyCoreError> {
        validate_brain_id(locator_id, "wcp_", "working-copy locator ID")?;
        validate_request_id(request_id)?;
        let fingerprint = request_fingerprint(&json!({
            "action": "revoke-working-copy",
            "locatorId": locator_id,
            "expectedGeneration": handle.generation,
        }))?;
        let now = unix_time_ms()?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        if let Some(event) = read_event_by_request(
            &transaction,
            &handle.project_id,
            None,
            request_id,
            self.path(),
        )? {
            if event.kind != EVENT_KIND_HUMAN_ACTION_RECORDED
                || event.request_fingerprint.as_deref() != Some(fingerprint.as_str())
                || event.payload.get("action").and_then(Value::as_str)
                    != Some("revoke-working-copy")
                || required_payload_string(&event, "locatorId")? != locator_id
            {
                return Err(invalid_brain(
                    "working-copy revocation request ID was reused with different content",
                ));
            }
            let generation = required_payload_u64(&event, "projectGeneration")?;
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return Ok(ProjectHandle {
                project_id: handle.project_id.clone(),
                generation,
            });
        }
        require_project_generation_on(&transaction, handle, self)?;
        let locator = read_working_copy_on(&transaction, &handle.project_id, locator_id, self)?
            .ok_or_else(|| invalid_brain("working-copy locator is not present"))?;
        if locator.state == WorkingCopyState::Revoked {
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return self
                .open_project_brain(&handle.project_id)
                .map(|brain| ProjectHandle {
                    project_id: brain.identity.project_id,
                    generation: brain.generation,
                });
        }
        let new_generation =
            bump_project_generation_on(&transaction, &handle.project_id, now, self)?;
        transaction
            .execute(
                "UPDATE working_copy_locators SET state = 'revoked', revoked_at_unix_ms = ?1
                 WHERE project_id = ?2 AND locator_id = ?3",
                params![
                    u64_i64(now, "working-copy revocation time")?,
                    handle.project_id,
                    locator_id
                ],
            )
            .map_err(|error| self.database_error(error))?;
        let next = ProjectHandle {
            project_id: handle.project_id.clone(),
            generation: new_generation,
        };
        let event = human_action_event(
            &next,
            request_id,
            &fingerprint,
            now,
            "revoke-working-copy",
            json!({"repositoryId": locator.repository_id, "locatorId": locator_id}),
        );
        append_project_brain_event_on(&transaction, &event, self.path())?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(next)
    }

    #[allow(dead_code)]
    pub(crate) fn record_local_human_action(
        &self,
        handle: &ProjectHandle,
        input: &LocalHumanActionInput,
    ) -> Result<ContinuityEvent, LeyCoreError> {
        validate_request_id(&input.request_id)?;
        validate_kind_text(&input.action_kind, "human action kind")?;
        let target_bytes = serde_json::to_vec(&input.exact_target).map_err(|error| {
            invalid_brain(&format!("human action target is not serializable: {error}"))
        })?;
        let target_digest = format!("sha256:{:x}", Sha256::digest(&target_bytes));
        let fingerprint = request_fingerprint(&json!({
            "action": input.action_kind,
            "exactTarget": input.exact_target,
            "evidence": input.evidence,
        }))?;
        let now = unix_time_ms()?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;

        if let Some(existing) = read_event_by_request(
            &transaction,
            &handle.project_id,
            None,
            &input.request_id,
            self.path(),
        )? {
            if existing.kind != EVENT_KIND_HUMAN_ACTION_RECORDED
                || existing.request_fingerprint.as_deref() != Some(fingerprint.as_str())
                || existing
                    .payload
                    .get("controlOrigin")
                    .and_then(Value::as_str)
                    != Some(HUMAN_CONTROL_ORIGIN)
            {
                return Err(invalid_brain(
                    "human action request ID was reused with different content",
                ));
            }
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return Ok(existing);
        }

        require_project_generation_on(&transaction, handle, self)?;
        for evidence in &input.evidence {
            match evidence {
                HumanActionEvidenceInput::SourceVersion {
                    source_id,
                    source_version_id,
                    selector,
                } => {
                    validate_brain_id(source_id, "src_", "source ID")?;
                    validate_brain_id(source_version_id, "ver_", "SourceVersion ID")?;
                    if read_source_version_on(
                        &transaction,
                        &handle.project_id,
                        source_id,
                        source_version_id,
                        self,
                    )?
                    .is_none()
                    {
                        return Err(invalid_brain(
                            "human action references a missing SourceVersion",
                        ));
                    }
                    serde_json::to_string(selector).map_err(|error| {
                        invalid_brain(&format!(
                            "human action evidence selector is not serializable: {error}"
                        ))
                    })?;
                }
                HumanActionEvidenceInput::Episode { event_id } => {
                    validate_event_reference_id(event_id)?;
                    let exists: bool = transaction
                        .query_row(
                            "SELECT EXISTS(
                                SELECT 1 FROM events
                                WHERE project_id = ?1 AND event_id = ?2
                             )",
                            params![handle.project_id, event_id],
                            |row| row.get(0),
                        )
                        .map_err(|error| self.database_error(error))?;
                    if !exists {
                        return Err(invalid_brain(
                            "human action references a missing Episode/event",
                        ));
                    }
                }
            }
        }

        let event = human_action_event(
            handle,
            &input.request_id,
            &fingerprint,
            now,
            &input.action_kind,
            json!({
                "snapshot": input.exact_target,
                "snapshotDigest": target_digest,
            }),
        );
        let event = append_project_brain_event_on(&transaction, &event, self.path())?.record;
        for evidence in &input.evidence {
            match evidence {
                HumanActionEvidenceInput::SourceVersion {
                    source_id,
                    source_version_id,
                    selector,
                } => {
                    let selector_json = serde_json::to_string(selector).map_err(|error| {
                        invalid_brain(&format!(
                            "human action evidence selector is not serializable: {error}"
                        ))
                    })?;
                    transaction
                        .execute(
                            "INSERT INTO event_source_version_links(
                                project_id, event_id, source_id, source_version_id,
                                relation, selector_version, selector_json
                             ) VALUES (?1, ?2, ?3, ?4, 'human-action-evidence', 1, ?5)",
                            params![
                                handle.project_id,
                                event.event_id,
                                source_id,
                                source_version_id,
                                selector_json,
                            ],
                        )
                        .map_err(|error| self.database_error(error))?;
                }
                HumanActionEvidenceInput::Episode { event_id } => {
                    transaction
                        .execute(
                            "INSERT INTO event_links(project_id, from_event_id, to_event_id, relation)
                             VALUES (?1, ?2, ?3, 'depends-on-episode')",
                            params![handle.project_id, event.event_id, event_id],
                        )
                        .map_err(|error| self.database_error(error))?;
                }
            }
        }
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(event)
    }

    pub fn create_project_source(
        &self,
        handle: &ProjectHandle,
        source_kind: &str,
        display_name: &str,
        request_id: &str,
    ) -> Result<ProjectSource, LeyCoreError> {
        validate_kind_text(source_kind, "source kind")?;
        validate_display_name(display_name, 1024, "source display name")?;
        validate_request_id(request_id)?;
        let display_name = display_name.trim();
        let fingerprint = request_fingerprint(&json!({
            "action": "create-source",
            "sourceKind": source_kind,
            "displayName": display_name,
        }))?;
        let now = unix_time_ms()?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        if let Some(event) = read_event_by_request(
            &transaction,
            &handle.project_id,
            None,
            request_id,
            self.path(),
        )? {
            if event.kind != EVENT_KIND_SOURCE_CREATED
                || event.request_fingerprint.as_deref() != Some(fingerprint.as_str())
            {
                return Err(invalid_brain(
                    "source creation request ID was reused with different content",
                ));
            }
            let source_id = required_payload_string(&event, "sourceId")?;
            let source = read_source_on(&transaction, &handle.project_id, &source_id, self)?
                .ok_or_else(|| invalid_brain("created source disappeared"))?;
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return Ok(source);
        }
        require_project_generation_on(&transaction, handle, self)?;
        let source = ProjectSource {
            project_id: handle.project_id.clone(),
            source_id: format!("src_{}", Uuid::new_v4().simple()),
            source_kind: source_kind.to_owned(),
            display_name: display_name.to_owned(),
            state: ProjectSourceState::Active,
            created_at_unix_ms: now,
            removed_at_unix_ms: None,
            erased_at_unix_ms: None,
        };
        transaction
            .execute(
                "INSERT INTO project_sources(
                    project_id, source_id, source_kind, display_name, state,
                    created_at_unix_ms, removed_at_unix_ms, erased_at_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, 'active', ?5, NULL, NULL)",
                params![
                    source.project_id,
                    source.source_id,
                    source.source_kind,
                    source.display_name,
                    u64_i64(now, "source creation time")?
                ],
            )
            .map_err(|error| self.database_error(error))?;
        let event = ContinuityEventInput {
            event_id: format!("evt_{}", Uuid::new_v4().simple()),
            project_id: handle.project_id.clone(),
            subject_id: Some(source.source_id.clone()),
            session_id: None,
            session_sequence: None,
            request_id: Some(request_id.to_owned()),
            request_fingerprint: Some(fingerprint),
            kind: EVENT_KIND_SOURCE_CREATED.to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: now,
            revision_head: None,
            revision_branch: None,
            payload: json!({
                "projectGeneration": handle.generation,
                "sourceId": source.source_id,
                "sourceKind": source.source_kind,
                "displayName": source.display_name,
            }),
        };
        append_project_brain_event_on(&transaction, &event, self.path())?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(source)
    }

    pub fn project_source(
        &self,
        project_id: &str,
        source_id: &str,
    ) -> Result<Option<ProjectSource>, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_brain_id(source_id, "src_", "source ID")?;
        let connection = self.open_connection()?;
        read_source_on(&connection, project_id, source_id, self)
    }

    pub fn project_sources(&self, project_id: &str) -> Result<Vec<ProjectSource>, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        require_active_project_on(&connection, project_id, self)?;
        let mut statement = connection
            .prepare(
                "SELECT source_id FROM project_sources
                 WHERE project_id = ?1
                 ORDER BY created_at_unix_ms, source_id",
            )
            .map_err(|error| self.database_error(error))?;
        let ids = statement
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(|error| self.database_error(error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| self.database_error(error))?;
        ids.into_iter()
            .map(|source_id| {
                read_source_on(&connection, project_id, &source_id, self)?
                    .ok_or_else(|| invalid_brain("Project Brain source disappeared during listing"))
            })
            .collect()
    }

    pub fn attach_source_path(
        &self,
        handle: &ProjectHandle,
        source_id: &str,
        path: impl AsRef<Path>,
        request_id: &str,
    ) -> Result<SourceLocator, LeyCoreError> {
        validate_brain_id(source_id, "src_", "source ID")?;
        validate_request_id(request_id)?;
        let path = path
            .as_ref()
            .canonicalize()
            .map_err(|source| LeyCoreError::Io {
                path: path.as_ref().to_path_buf(),
                source,
            })?;
        let metadata = std::fs::symlink_metadata(&path).map_err(|source| LeyCoreError::Io {
            path: path.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
            return Err(invalid_brain(
                "source locator must resolve to a regular file or directory",
            ));
        }
        let locator_kind = if metadata.is_dir() {
            "local-directory"
        } else {
            "local-file"
        };
        let locator_value = path
            .to_str()
            .ok_or_else(|| LeyCoreError::NonUtf8Path(path.clone()))?
            .to_owned();
        let fingerprint = request_fingerprint(&json!({
            "action": "attach-source-path",
            "sourceId": source_id,
            "locatorKind": locator_kind,
            "locatorValue": locator_value,
        }))?;
        let now = unix_time_ms()?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        if let Some(event) = read_event_by_request(
            &transaction,
            &handle.project_id,
            None,
            request_id,
            self.path(),
        )? {
            if event.kind != EVENT_KIND_SOURCE_LOCATOR_ATTACHED
                || event.request_fingerprint.as_deref() != Some(fingerprint.as_str())
            {
                return Err(invalid_brain(
                    "source locator request ID was reused with different content",
                ));
            }
            let locator_id = required_payload_string(&event, "locatorId")?;
            let locator = read_source_locator_on(
                &transaction,
                &handle.project_id,
                source_id,
                &locator_id,
                self,
            )?
            .ok_or_else(|| invalid_brain("source locator disappeared"))?;
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return Ok(locator);
        }
        require_project_generation_on(&transaction, handle, self)?;
        let source = read_source_on(&transaction, &handle.project_id, source_id, self)?
            .ok_or_else(|| invalid_brain("source is not present"))?;
        if source.state != ProjectSourceState::Active {
            return Err(invalid_brain(
                "a source must be active before attaching a live locator",
            ));
        }
        let locator = SourceLocator {
            project_id: handle.project_id.clone(),
            source_id: source_id.to_owned(),
            locator_id: format!("loc_{}", Uuid::new_v4().simple()),
            locator_kind: locator_kind.to_owned(),
            locator_value: locator_value.clone(),
            state: SourceLocatorState::Observed,
            observed_at_unix_ms: now,
            revoked_at_unix_ms: None,
        };
        transaction
            .execute(
                "INSERT INTO source_locators(
                    project_id, source_id, locator_id, locator_kind, locator_value,
                    state, observed_at_unix_ms, revoked_at_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'observed', ?6, NULL)",
                params![
                    locator.project_id,
                    locator.source_id,
                    locator.locator_id,
                    locator.locator_kind,
                    locator.locator_value,
                    u64_i64(now, "source locator observation time")?
                ],
            )
            .map_err(|error| self.database_error(error))?;
        let event = ContinuityEventInput {
            event_id: format!("evt_{}", Uuid::new_v4().simple()),
            project_id: handle.project_id.clone(),
            subject_id: Some(source_id.to_owned()),
            session_id: None,
            session_sequence: None,
            request_id: Some(request_id.to_owned()),
            request_fingerprint: Some(fingerprint),
            kind: EVENT_KIND_SOURCE_LOCATOR_ATTACHED.to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: now,
            revision_head: None,
            revision_branch: None,
            payload: json!({
                "projectGeneration": handle.generation,
                "sourceId": source_id,
                "locatorId": locator.locator_id,
                "locatorKind": locator_kind,
                "pathDigest": format!("sha256:{:x}", Sha256::digest(locator_value.as_bytes())),
            }),
        };
        append_project_brain_event_on(&transaction, &event, self.path())?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(locator)
    }

    pub fn remove_project_source(
        &self,
        handle: &ProjectHandle,
        source_id: &str,
    ) -> Result<ProjectHandle, LeyCoreError> {
        validate_brain_id(source_id, "src_", "source ID")?;
        let now = unix_time_ms()?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        require_project_generation_on(&transaction, handle, self)?;
        let source = read_source_on(&transaction, &handle.project_id, source_id, self)?
            .ok_or_else(|| invalid_brain("source is not present"))?;
        if source.state == ProjectSourceState::Erased {
            return Err(invalid_brain(
                "erased source cannot be reactivated or removed",
            ));
        }
        if source.state == ProjectSourceState::Removed {
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return Ok(handle.clone());
        }
        let generation = bump_project_generation_on(&transaction, &handle.project_id, now, self)?;
        transaction
            .execute(
                "UPDATE project_sources SET state = 'removed', removed_at_unix_ms = ?1
                 WHERE project_id = ?2 AND source_id = ?3",
                params![
                    u64_i64(now, "source removal time")?,
                    handle.project_id,
                    source_id
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(ProjectHandle {
            project_id: handle.project_id.clone(),
            generation,
        })
    }

    pub fn retain_source_version(
        &self,
        handle: &ProjectHandle,
        source_id: &str,
        input: &SourceVersionInput,
    ) -> Result<RetainedSourceVersion, LeyCoreError> {
        self.with_artifact_authority_lock(|| {
            let mut connection = self.open_connection()?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| self.database_error(error))?;
            let result =
                self.retain_source_version_on(&transaction, handle, source_id, input, None)?;
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            Ok(result)
        })
    }

    pub(crate) fn retain_source_version_on(
        &self,
        transaction: &Transaction<'_>,
        handle: &ProjectHandle,
        source_id: &str,
        input: &SourceVersionInput,
        import_provenance: Option<&Value>,
    ) -> Result<RetainedSourceVersion, LeyCoreError> {
        validate_brain_id(source_id, "src_", "source ID")?;
        validate_request_id(&input.request_id)?;
        validate_kind_text(&input.representation_kind, "representation kind")?;
        if input.retained_bytes.len() > MAX_PROJECT_BRAIN_SOURCE_BYTES {
            return Err(invalid_brain(&format!(
                "retained SourceVersion exceeds the M1 {}-byte limit",
                MAX_PROJECT_BRAIN_SOURCE_BYTES
            )));
        }
        validate_optional_hash(input.original_content_hash.as_deref())?;
        let transformation_json =
            serde_json::to_string(&input.transformation).map_err(|error| {
                invalid_brain(&format!(
                    "source transformation is not serializable: {error}"
                ))
            })?;
        let content_hash = format!("sha256:{:x}", Sha256::digest(&input.retained_bytes));
        let fingerprint = request_fingerprint(&json!({
            "sourceId": source_id,
            "representationKind": input.representation_kind,
            "originalContentHash": input.original_content_hash,
            "sourceBytes": input.source_bytes,
            "contentHash": content_hash,
            "transformation": input.transformation,
        }))?;
        let now = unix_time_ms()?;

        require_project_generation_on(transaction, handle, self)?;
        let source = read_source_on(transaction, &handle.project_id, source_id, self)?
            .ok_or_else(|| invalid_brain("source is not present"))?;
        if source.state != ProjectSourceState::Active {
            return Err(invalid_brain(
                "cannot retain a new SourceVersion unless the source is active",
            ));
        }

        if let Some(event) = read_event_by_request(
            transaction,
            &handle.project_id,
            None,
            &input.request_id,
            self.path(),
        )? {
            if event.request_fingerprint.as_deref() != Some(fingerprint.as_str()) {
                return Err(invalid_brain(
                    "SourceVersion request ID was reused with different content",
                ));
            }
            let version_id = required_payload_string(&event, "sourceVersionId")?;
            let version = read_source_version_on(
                transaction,
                &handle.project_id,
                source_id,
                &version_id,
                self,
            )?
            .ok_or_else(|| invalid_brain("retained SourceVersion disappeared"))?;
            return Ok(RetainedSourceVersion {
                occurrence_event_id: event.event_id,
                ..version
            });
        }

        let existing = transaction
            .query_row(
                "SELECT source_version_id
                     FROM source_versions
                     WHERE project_id = ?1 AND source_id = ?2
                       AND representation_kind = ?3 AND content_hash = ?4
                       AND ifnull(original_content_hash, '') = ifnull(?5, '')
                       AND source_bytes = ?6 AND stored_bytes = ?7
                       AND transformation_json = ?8
                     ORDER BY retained_at_unix_ms, source_version_id LIMIT 1",
                params![
                    handle.project_id,
                    source_id,
                    input.representation_kind,
                    content_hash,
                    input.original_content_hash,
                    u64_i64(input.source_bytes, "SourceVersion source bytes")?,
                    u64_i64(
                        input.retained_bytes.len() as u64,
                        "SourceVersion stored bytes"
                    )?,
                    transformation_json,
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| self.database_error(error))?;

        self.install_artifact_blob(
            &handle.project_id,
            &content_hash,
            input.retained_bytes.len() as u64,
            &input.retained_bytes,
        )?;
        let version_id = existing.unwrap_or_else(|| format!("ver_{}", Uuid::new_v4().simple()));
        if read_source_version_on(
            transaction,
            &handle.project_id,
            source_id,
            &version_id,
            self,
        )?
        .is_none()
        {
            transaction
                .execute(
                    "INSERT INTO source_versions(
                            project_id, source_id, source_version_id, representation_kind,
                            content_hash, original_content_hash, source_bytes, stored_bytes,
                            transformation_json, retained_at_unix_ms
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        handle.project_id,
                        source_id,
                        version_id,
                        input.representation_kind,
                        content_hash,
                        input.original_content_hash,
                        u64_i64(input.source_bytes, "SourceVersion source bytes")?,
                        u64_i64(
                            input.retained_bytes.len() as u64,
                            "SourceVersion stored bytes"
                        )?,
                        transformation_json,
                        u64_i64(now, "SourceVersion retention time")?,
                    ],
                )
                .map_err(|error| self.database_error(error))?;
        }
        let occurrence = ContinuityEventInput {
            event_id: format!("evt_{}", Uuid::new_v4().simple()),
            project_id: handle.project_id.clone(),
            subject_id: Some(source_id.to_owned()),
            session_id: None,
            session_sequence: None,
            request_id: Some(input.request_id.clone()),
            request_fingerprint: Some(fingerprint.clone()),
            kind: EVENT_KIND_SOURCE_VERSION_RETAINED.to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: now,
            revision_head: import_provenance
                .and_then(|p| p.get("git"))
                .and_then(|g| g.get("head"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            revision_branch: import_provenance
                .and_then(|p| p.get("git"))
                .and_then(|g| g.get("branch"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            payload: json!({
                "projectGeneration": handle.generation,
                "import": import_provenance,
                "sourceId": source_id,
                "sourceVersionId": version_id,
                "representationKind": input.representation_kind,
                "contentHash": content_hash,
                "storedBytes": input.retained_bytes.len(),
            }),
        };
        let event = append_project_brain_event_on(transaction, &occurrence, self.path())?.record;
        transaction
            .execute(
                "INSERT OR IGNORE INTO event_source_version_links(
                        project_id, event_id, source_id, source_version_id,
                        relation, selector_version, selector_json
                     ) VALUES (?1, ?2, ?3, ?4, 'retained-version', 1, '{\"kind\":\"whole\"}')",
                params![handle.project_id, event.event_id, source_id, version_id],
            )
            .map_err(|error| self.database_error(error))?;
        let version = read_source_version_on(
            transaction,
            &handle.project_id,
            source_id,
            &version_id,
            self,
        )?
        .ok_or_else(|| invalid_brain("retained SourceVersion was not persisted"))?;
        Ok(RetainedSourceVersion {
            occurrence_event_id: event.event_id,
            ..version
        })
    }

    pub fn source_version_bytes(
        &self,
        project_id: &str,
        source_id: &str,
        source_version_id: &str,
    ) -> Result<Vec<u8>, LeyCoreError> {
        validate_project_id(project_id)?;
        validate_brain_id(source_id, "src_", "source ID")?;
        validate_brain_id(source_version_id, "ver_", "SourceVersion ID")?;
        let connection = self.open_connection()?;
        let version =
            read_source_version_on(&connection, project_id, source_id, source_version_id, self)?
                .ok_or_else(|| invalid_brain("SourceVersion is not present"))?;
        self.read_project_brain_blob(project_id, &version.content_hash, version.stored_bytes)
    }

    pub fn erase_project_source(
        &self,
        handle: &ProjectHandle,
        source_id: &str,
    ) -> Result<ProjectHandle, LeyCoreError> {
        validate_brain_id(source_id, "src_", "source ID")?;
        let now = unix_time_ms()?;
        self.with_artifact_authority_lock(|| {
            let mut connection = self.open_connection()?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| self.database_error(error))?;
            require_project_generation_on(&transaction, handle, self)?;
            let source = read_source_on(&transaction, &handle.project_id, source_id, self)?
                .ok_or_else(|| invalid_brain("source is not present"))?;
            if source.state == ProjectSourceState::Erased {
                transaction
                    .commit()
                    .map_err(|error| self.database_error(error))?;
                self.garbage_collect_artifact_state_under_lock(&handle.project_id)?;
                return self
                    .open_project_brain(&handle.project_id)
                    .map(|brain| ProjectHandle {
                        project_id: brain.identity.project_id,
                        generation: brain.generation,
                    });
            }
            let generation =
                bump_project_generation_on(&transaction, &handle.project_id, now, self)?;
            let mut dependent = transaction
                .prepare(
                    "SELECT DISTINCT event_id FROM event_source_version_links
                     WHERE project_id = ?1 AND source_id = ?2",
                )
                .map_err(|error| self.database_error(error))?;
            let mut event_ids = dependent
                .query_map(params![handle.project_id, source_id], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|error| self.database_error(error))?
                .collect::<Result<BTreeSet<_>, _>>()
                .map_err(|error| self.database_error(error))?;
            drop(dependent);
            let mut source_events = transaction
                .prepare(
                    "SELECT event_id FROM events
                     WHERE project_id = ?1 AND (subject_id = ?2 OR kind = 'project-imported')",
                )
                .map_err(|error| self.database_error(error))?;
            event_ids.extend(
                source_events
                    .query_map(params![handle.project_id, source_id], |row| {
                        row.get::<_, String>(0)
                    })
                    .map_err(|error| self.database_error(error))?
                    .collect::<Result<BTreeSet<_>, _>>()
                    .map_err(|error| self.database_error(error))?,
            );
            drop(source_events);
            let event_ids =
                dependent_event_closure_on(&transaction, &handle.project_id, event_ids, self)?;
            for event_id in event_ids {
                transaction
                    .execute(
                        "DELETE FROM events WHERE project_id = ?1 AND event_id = ?2",
                        params![handle.project_id, event_id],
                    )
                    .map_err(|error| self.database_error(error))?;
            }
            crate::project_import::erase_import_source_on(
                &transaction,
                &handle.project_id,
                source_id,
                self,
            )?;
            transaction
                .execute(
                    "DELETE FROM source_versions WHERE project_id = ?1 AND source_id = ?2",
                    params![handle.project_id, source_id],
                )
                .map_err(|error| self.database_error(error))?;
            transaction
                .execute(
                    "DELETE FROM source_locators WHERE project_id = ?1 AND source_id = ?2",
                    params![handle.project_id, source_id],
                )
                .map_err(|error| self.database_error(error))?;
            transaction
                .execute(
                    "UPDATE project_sources
                     SET source_kind = 'erased', display_name = '[erased]', state = 'erased',
                         removed_at_unix_ms = NULL, erased_at_unix_ms = ?1
                     WHERE project_id = ?2 AND source_id = ?3",
                    params![
                        u64_i64(now, "source erasure time")?,
                        handle.project_id,
                        source_id
                    ],
                )
                .map_err(|error| self.database_error(error))?;
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            self.garbage_collect_artifact_state_under_lock(&handle.project_id)?;
            Ok(ProjectHandle {
                project_id: handle.project_id.clone(),
                generation,
            })
        })
    }

    pub fn start_project_session(
        &self,
        handle: &ProjectHandle,
        session_id: &str,
        host_kind: Option<&str>,
        external_session_id: Option<&str>,
    ) -> Result<ProjectSession, LeyCoreError> {
        validate_session_id(session_id)?;
        if host_kind.is_none() != external_session_id.is_none() {
            return Err(invalid_brain(
                "session host kind and external session ID must be supplied together",
            ));
        }
        if let Some(host) = host_kind {
            validate_kind_text(host, "session host kind")?;
        }
        let now = unix_time_ms()?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        require_project_generation_on(&transaction, handle, self)?;
        if let Some(existing) = read_session_on(&transaction, &handle.project_id, session_id, self)?
        {
            if existing.state == ProjectSessionState::Erased {
                return Err(invalid_brain("erased session ID cannot be reused"));
            }
            if existing.host_kind.as_deref() != host_kind
                || existing.external_session_id.as_deref() != external_session_id
            {
                return Err(invalid_brain(
                    "session ID was reused with different host provenance",
                ));
            }
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return Ok(existing);
        }
        transaction
            .execute(
                "INSERT INTO project_sessions(
                    project_id, session_id, host_kind, external_session_id, state,
                    started_at_unix_ms, ended_at_unix_ms, created_at_unix_ms,
                    erased_at_unix_ms, observation_limits_json
                 ) VALUES (?1, ?2, ?3, ?4, 'active', ?5, NULL, ?5, NULL, '{}')",
                params![
                    handle.project_id,
                    session_id,
                    host_kind,
                    external_session_id,
                    u64_i64(now, "session start time")?
                ],
            )
            .map_err(|error| self.database_error(error))?;
        let session = read_session_on(&transaction, &handle.project_id, session_id, self)?
            .ok_or_else(|| invalid_brain("Project Brain session was not persisted"))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(session)
    }

    pub fn project_sessions(&self, project_id: &str) -> Result<Vec<ProjectSession>, LeyCoreError> {
        validate_project_id(project_id)?;
        let connection = self.open_connection()?;
        require_active_project_on(&connection, project_id, self)?;
        let mut statement = connection
            .prepare(
                "SELECT session_id FROM project_sessions
                 WHERE project_id = ?1
                 ORDER BY created_at_unix_ms, session_id",
            )
            .map_err(|error| self.database_error(error))?;
        let ids = statement
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(|error| self.database_error(error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| self.database_error(error))?;
        ids.into_iter()
            .map(|session_id| {
                read_session_on(&connection, project_id, &session_id, self)?.ok_or_else(|| {
                    invalid_brain("Project Brain session disappeared during listing")
                })
            })
            .collect()
    }

    pub fn record_project_episode(
        &self,
        handle: &ProjectHandle,
        session_id: Option<&str>,
        session_sequence: Option<u64>,
        request_id: &str,
        episode_kind: &str,
        occurred_at_unix_ms: Option<u64>,
        payload: Value,
    ) -> Result<ContinuityEvent, LeyCoreError> {
        validate_request_id(request_id)?;
        validate_kind_text(episode_kind, "episode kind")?;
        if !SUPPORTED_OBSERVED_EPISODE_KINDS.contains(&episode_kind) {
            return Err(invalid_brain(
                "episode kind is not a supported observable Chronicle occurrence",
            ));
        }
        if session_id.is_some() != session_sequence.is_some() {
            return Err(invalid_brain(
                "session episode sequence must be supplied together with session ID",
            ));
        }
        let now = unix_time_ms()?;
        let fingerprint = request_fingerprint(&json!({
            "sessionId": session_id,
            "sessionSequence": session_sequence,
            "episodeKind": episode_kind,
            "occurredAtUnixMs": occurred_at_unix_ms,
            "payload": payload,
        }))?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        require_project_generation_on(&transaction, handle, self)?;
        if let Some(session_id) = session_id {
            validate_session_id(session_id)?;
            let session = read_session_on(&transaction, &handle.project_id, session_id, self)?
                .ok_or_else(|| invalid_brain("episode session is not registered"))?;
            if session.state != ProjectSessionState::Active {
                return Err(invalid_brain(
                    "cannot append an Episode unless the session is active",
                ));
            }
        }
        let event = ContinuityEventInput {
            event_id: format!("evt_{}", Uuid::new_v4().simple()),
            project_id: handle.project_id.clone(),
            subject_id: None,
            session_id: session_id.map(str::to_owned),
            session_sequence,
            request_id: Some(request_id.to_owned()),
            request_fingerprint: Some(fingerprint),
            kind: EVENT_KIND_EPISODE_RECORDED.to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: now,
            revision_head: None,
            revision_branch: None,
            payload: json!({
                "projectGeneration": handle.generation,
                "episodeKind": episode_kind,
                "occurredAtUnixMs": occurred_at_unix_ms,
                "observation": payload,
            }),
        };
        let written = append_project_brain_event_on(&transaction, &event, self.path())?.record;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(written)
    }

    pub fn finish_project_session(
        &self,
        handle: &ProjectHandle,
        session_id: &str,
    ) -> Result<ProjectSession, LeyCoreError> {
        validate_session_id(session_id)?;
        let now = unix_time_ms()?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        require_project_generation_on(&transaction, handle, self)?;
        let session = read_session_on(&transaction, &handle.project_id, session_id, self)?
            .ok_or_else(|| invalid_brain("Project Brain session is not present"))?;
        match session.state {
            ProjectSessionState::Finished => {
                transaction
                    .commit()
                    .map_err(|error| self.database_error(error))?;
                return Ok(session);
            }
            ProjectSessionState::Erased => {
                return Err(invalid_brain("erased session cannot be finished"));
            }
            ProjectSessionState::Active => {}
        }
        transaction
            .execute(
                "UPDATE project_sessions
                 SET state = 'finished', ended_at_unix_ms = ?1
                 WHERE project_id = ?2 AND session_id = ?3 AND state = 'active'",
                params![
                    u64_i64(now, "session finish time")?,
                    handle.project_id,
                    session_id
                ],
            )
            .map_err(|error| self.database_error(error))?;
        let session = read_session_on(&transaction, &handle.project_id, session_id, self)?
            .ok_or_else(|| invalid_brain("finished Project Brain session disappeared"))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(session)
    }

    pub fn erase_project_session(
        &self,
        handle: &ProjectHandle,
        session_id: &str,
    ) -> Result<ProjectHandle, LeyCoreError> {
        validate_session_id(session_id)?;
        let now = unix_time_ms()?;
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| self.database_error(error))?;
        require_project_generation_on(&transaction, handle, self)?;
        let session = read_session_on(&transaction, &handle.project_id, session_id, self)?
            .ok_or_else(|| invalid_brain("Project Brain session is not present"))?;
        if session.state == ProjectSessionState::Erased {
            transaction
                .commit()
                .map_err(|error| self.database_error(error))?;
            return Ok(handle.clone());
        }
        let generation = bump_project_generation_on(&transaction, &handle.project_id, now, self)?;
        let mut session_events = transaction
            .prepare(
                "SELECT event_id FROM events
                 WHERE project_id = ?1 AND session_id = ?2",
            )
            .map_err(|error| self.database_error(error))?;
        let event_ids = session_events
            .query_map(params![handle.project_id, session_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|error| self.database_error(error))?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(|error| self.database_error(error))?;
        drop(session_events);
        let event_ids =
            dependent_event_closure_on(&transaction, &handle.project_id, event_ids, self)?;
        for event_id in event_ids {
            transaction
                .execute(
                    "DELETE FROM events WHERE project_id = ?1 AND event_id = ?2",
                    params![handle.project_id, event_id],
                )
                .map_err(|error| self.database_error(error))?;
        }
        transaction
            .execute(
                "UPDATE project_sessions
                 SET host_kind = NULL, external_session_id = NULL, state = 'erased',
                     started_at_unix_ms = NULL, ended_at_unix_ms = NULL,
                     created_at_unix_ms = ?1, erased_at_unix_ms = ?1,
                     observation_limits_json = '{}'
                 WHERE project_id = ?2 AND session_id = ?3",
                params![
                    u64_i64(now, "session erasure time")?,
                    handle.project_id,
                    session_id
                ],
            )
            .map_err(|error| self.database_error(error))?;
        transaction
            .commit()
            .map_err(|error| self.database_error(error))?;
        Ok(ProjectHandle {
            project_id: handle.project_id.clone(),
            generation,
        })
    }
}

pub(crate) fn lifecycle_on(
    connection: &Connection,
    project_id: &str,
    store: &ContinuityStore,
) -> Result<Option<ProjectLifecycle>, LeyCoreError> {
    validate_project_id(project_id)?;
    connection
        .query_row(
            "SELECT generation, state, cleanup_kind, cleanup_json
             FROM project_lifecycle WHERE project_id = ?1",
            [project_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| store.database_error(error))?
        .map(|(generation, state, cleanup_kind, cleanup_json)| {
            Ok(ProjectLifecycle {
                generation: sqlite_u64(generation, "project generation")?,
                state: ProjectLifecycleState::parse(&state)?,
                cleanup_kind,
                cleanup_json,
            })
        })
        .transpose()
}

pub(crate) fn require_active_project_on(
    connection: &Connection,
    project_id: &str,
    store: &ContinuityStore,
) -> Result<ProjectLifecycle, LeyCoreError> {
    let lifecycle = lifecycle_on(connection, project_id, store)?
        .ok_or_else(|| invalid_brain(&format!("project {project_id} has no lifecycle record")))?;
    match lifecycle.state {
        ProjectLifecycleState::Active => Ok(lifecycle),
        ProjectLifecycleState::Erasing => Err(LeyCoreError::ProjectErasing {
            project_id: project_id.to_owned(),
        }),
        ProjectLifecycleState::Erased => Err(LeyCoreError::ProjectErased {
            project_id: project_id.to_owned(),
        }),
    }
}

pub(crate) fn ensure_registered_project_lifecycle_on(
    connection: &Connection,
    identity: &ProjectIdentity,
    store: &ContinuityStore,
) -> Result<(), LeyCoreError> {
    if let Some(lifecycle) = lifecycle_on(connection, &identity.project_id, store)? {
        return match lifecycle.state {
            ProjectLifecycleState::Active => Ok(()),
            ProjectLifecycleState::Erasing => Err(LeyCoreError::ProjectErasing {
                project_id: identity.project_id.clone(),
            }),
            ProjectLifecycleState::Erased => Err(LeyCoreError::ProjectErased {
                project_id: identity.project_id.clone(),
            }),
        };
    }
    connection
        .execute(
            "INSERT INTO project_lifecycle(
                project_id, generation, state, updated_at_unix_ms,
                create_request_id, create_request_fingerprint, cleanup_kind, cleanup_json
             ) VALUES (?1, 1, 'active', ?2, NULL, NULL, 'none', NULL)",
            params![
                identity.project_id,
                u64_i64(identity.created_at_unix_ms, "project lifecycle time")?
            ],
        )
        .map_err(|error| store.database_error(error))?;
    Ok(())
}

pub(crate) fn ensure_project_not_terminal_on(
    connection: &Connection,
    project_id: &str,
    store: &ContinuityStore,
) -> Result<(), LeyCoreError> {
    match lifecycle_on(connection, project_id, store)? {
        Some(ProjectLifecycle {
            state: ProjectLifecycleState::Erasing,
            ..
        }) => Err(LeyCoreError::ProjectErasing {
            project_id: project_id.to_owned(),
        }),
        Some(ProjectLifecycle {
            state: ProjectLifecycleState::Erased,
            ..
        }) => Err(LeyCoreError::ProjectErased {
            project_id: project_id.to_owned(),
        }),
        _ => Ok(()),
    }
}

pub(crate) fn ensure_event_references_not_erased_on(
    connection: &Connection,
    input: &ContinuityEventInput,
    store: &ContinuityStore,
) -> Result<(), LeyCoreError> {
    if let Some(session_id) = input.session_id.as_deref() {
        let state: Option<String> = connection
            .query_row(
                "SELECT state FROM project_sessions
                 WHERE project_id = ?1 AND session_id = ?2",
                params![input.project_id, session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| store.database_error(error))?;
        if state.as_deref() == Some("erased") {
            return Err(invalid_brain(
                "an erased Project Brain session ID cannot receive new events",
            ));
        }
    }
    if let Some(source_id) = input
        .subject_id
        .as_deref()
        .filter(|subject_id| subject_id.starts_with("src_"))
    {
        let state: Option<String> = connection
            .query_row(
                "SELECT state FROM project_sources
                 WHERE project_id = ?1 AND source_id = ?2",
                params![input.project_id, source_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| store.database_error(error))?;
        if state.as_deref() == Some("erased") {
            return Err(invalid_brain(
                "an erased Project Brain source ID cannot receive new events",
            ));
        }
    }
    Ok(())
}

fn dependent_event_closure_on(
    connection: &Connection,
    project_id: &str,
    seeds: BTreeSet<String>,
    store: &ContinuityStore,
) -> Result<BTreeSet<String>, LeyCoreError> {
    let mut closure = seeds;
    let mut frontier = closure.iter().cloned().collect::<Vec<_>>();
    while let Some(target_event_id) = frontier.pop() {
        let mut statement = connection
            .prepare(
                "SELECT from_event_id FROM event_links
                 WHERE project_id = ?1 AND relation = 'depends-on-episode'
                   AND to_event_id = ?2
                 ORDER BY from_event_id",
            )
            .map_err(|error| store.database_error(error))?;
        let rows = statement
            .query_map(params![project_id, target_event_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|error| store.database_error(error))?;
        for row in rows {
            let dependent = row.map_err(|error| store.database_error(error))?;
            if closure.insert(dependent.clone()) {
                frontier.push(dependent);
            }
        }
    }
    Ok(closure)
}

pub(crate) fn require_project_generation_on(
    connection: &Connection,
    handle: &ProjectHandle,
    store: &ContinuityStore,
) -> Result<ProjectLifecycle, LeyCoreError> {
    let lifecycle = require_active_project_on(connection, &handle.project_id, store)?;
    if lifecycle.generation != handle.generation {
        return Err(LeyCoreError::ProjectGenerationChanged {
            project_id: handle.project_id.clone(),
            expected: handle.generation,
            current: lifecycle.generation,
        });
    }
    Ok(lifecycle)
}

pub(crate) fn bump_project_generation_on(
    transaction: &Transaction<'_>,
    project_id: &str,
    updated_at_unix_ms: u64,
    store: &ContinuityStore,
) -> Result<u64, LeyCoreError> {
    let lifecycle = require_active_project_on(transaction, project_id, store)?;
    let next = lifecycle
        .generation
        .checked_add(1)
        .ok_or_else(|| invalid_brain("project generation overflow"))?;
    transaction
        .execute(
            "UPDATE project_lifecycle SET generation = ?1, updated_at_unix_ms = ?2
             WHERE project_id = ?3 AND state = 'active' AND generation = ?4",
            params![
                u64_i64(next, "project generation")?,
                u64_i64(updated_at_unix_ms, "project lifecycle time")?,
                project_id,
                u64_i64(lifecycle.generation, "project generation")?
            ],
        )
        .map_err(|error| store.database_error(error))?;
    Ok(next)
}

fn read_project_identity_on(
    connection: &Connection,
    project_id: &str,
    store: &ContinuityStore,
) -> Result<Option<ProjectIdentity>, LeyCoreError> {
    connection
        .query_row(
            "SELECT name, created_at_unix_ms FROM projects WHERE project_id = ?1",
            [project_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|error| store.database_error(error))?
        .map(|(name, created_at)| {
            Ok(ProjectIdentity {
                schema_version: PROJECT_SCHEMA_VERSION,
                project_id: project_id.to_owned(),
                name,
                created_at_unix_ms: sqlite_u64(created_at, "project created time")?,
            })
        })
        .transpose()
}

fn read_repository_on(
    connection: &Connection,
    project_id: &str,
    store: &ContinuityStore,
) -> Result<Option<ProjectRepositoryAttachment>, LeyCoreError> {
    connection
        .query_row(
            "SELECT repository_id, attached_at_unix_ms
             FROM project_repositories WHERE project_id = ?1",
            [project_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|error| store.database_error(error))?
        .map(|(repository_id, attached_at)| {
            Ok(ProjectRepositoryAttachment {
                project_id: project_id.to_owned(),
                repository_id,
                attached_at_unix_ms: sqlite_u64(attached_at, "repository attachment time")?,
            })
        })
        .transpose()
}

pub(crate) fn read_working_copy_on(
    connection: &Connection,
    project_id: &str,
    locator_id: &str,
    store: &ContinuityStore,
) -> Result<Option<WorkingCopyLocator>, LeyCoreError> {
    connection
        .query_row(
            "SELECT repository_id, local_path, state, authorized_at_unix_ms,
                    revoked_at_unix_ms, last_observed_at_unix_ms, revision_head, revision_branch
             FROM working_copy_locators WHERE project_id = ?1 AND locator_id = ?2",
            params![project_id, locator_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            },
        )
        .optional()
        .map_err(|error| store.database_error(error))?
        .map(
            |(
                repository_id,
                local_path,
                state,
                authorized_at,
                revoked_at,
                observed_at,
                head,
                branch,
            )| {
                Ok(WorkingCopyLocator {
                    project_id: project_id.to_owned(),
                    repository_id,
                    locator_id: locator_id.to_owned(),
                    local_path: PathBuf::from(local_path),
                    state: WorkingCopyState::parse(&state)?,
                    authorized_at_unix_ms: sqlite_u64(
                        authorized_at,
                        "working-copy authorization time",
                    )?,
                    revoked_at_unix_ms: revoked_at
                        .map(|value| sqlite_u64(value, "working-copy revocation time"))
                        .transpose()?,
                    last_observed_at_unix_ms: sqlite_u64(
                        observed_at,
                        "working-copy observation time",
                    )?,
                    revision_head: head,
                    revision_branch: branch,
                })
            },
        )
        .transpose()
}

fn read_source_on(
    connection: &Connection,
    project_id: &str,
    source_id: &str,
    store: &ContinuityStore,
) -> Result<Option<ProjectSource>, LeyCoreError> {
    connection
        .query_row(
            "SELECT source_kind, display_name, state, created_at_unix_ms,
                    removed_at_unix_ms, erased_at_unix_ms
             FROM project_sources WHERE project_id = ?1 AND source_id = ?2",
            params![project_id, source_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|error| store.database_error(error))?
        .map(
            |(kind, display_name, state, created_at, removed_at, erased_at)| {
                Ok(ProjectSource {
                    project_id: project_id.to_owned(),
                    source_id: source_id.to_owned(),
                    source_kind: kind,
                    display_name,
                    state: ProjectSourceState::parse(&state)?,
                    created_at_unix_ms: sqlite_u64(created_at, "source creation time")?,
                    removed_at_unix_ms: removed_at
                        .map(|value| sqlite_u64(value, "source removal time"))
                        .transpose()?,
                    erased_at_unix_ms: erased_at
                        .map(|value| sqlite_u64(value, "source erasure time"))
                        .transpose()?,
                })
            },
        )
        .transpose()
}

fn read_source_locator_on(
    connection: &Connection,
    project_id: &str,
    source_id: &str,
    locator_id: &str,
    store: &ContinuityStore,
) -> Result<Option<SourceLocator>, LeyCoreError> {
    connection
        .query_row(
            "SELECT locator_kind, locator_value, state, observed_at_unix_ms, revoked_at_unix_ms
             FROM source_locators
             WHERE project_id = ?1 AND source_id = ?2 AND locator_id = ?3",
            params![project_id, source_id, locator_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| store.database_error(error))?
        .map(
            |(locator_kind, locator_value, state, observed_at, revoked_at)| {
                Ok(SourceLocator {
                    project_id: project_id.to_owned(),
                    source_id: source_id.to_owned(),
                    locator_id: locator_id.to_owned(),
                    locator_kind,
                    locator_value,
                    state: SourceLocatorState::parse(&state)?,
                    observed_at_unix_ms: sqlite_u64(
                        observed_at,
                        "source locator observation time",
                    )?,
                    revoked_at_unix_ms: revoked_at
                        .map(|value| sqlite_u64(value, "source locator revocation time"))
                        .transpose()?,
                })
            },
        )
        .transpose()
}

pub(crate) fn read_source_version_on(
    connection: &Connection,
    project_id: &str,
    source_id: &str,
    source_version_id: &str,
    store: &ContinuityStore,
) -> Result<Option<RetainedSourceVersion>, LeyCoreError> {
    connection
        .query_row(
            "SELECT representation_kind, content_hash, original_content_hash,
                    source_bytes, stored_bytes, retained_at_unix_ms
             FROM source_versions
             WHERE project_id = ?1 AND source_id = ?2 AND source_version_id = ?3",
            params![project_id, source_id, source_version_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|error| store.database_error(error))?
        .map(
            |(
                representation_kind,
                content_hash,
                original_content_hash,
                source_bytes,
                stored_bytes,
                retained_at,
            )| {
                Ok(RetainedSourceVersion {
                    project_id: project_id.to_owned(),
                    source_id: source_id.to_owned(),
                    source_version_id: source_version_id.to_owned(),
                    representation_kind,
                    content_hash,
                    original_content_hash,
                    source_bytes: sqlite_u64(source_bytes, "SourceVersion source bytes")?,
                    stored_bytes: sqlite_u64(stored_bytes, "SourceVersion stored bytes")?,
                    retained_at_unix_ms: sqlite_u64(retained_at, "SourceVersion retention time")?,
                    occurrence_event_id: String::new(),
                })
            },
        )
        .transpose()
}

pub(crate) fn read_session_on(
    connection: &Connection,
    project_id: &str,
    session_id: &str,
    store: &ContinuityStore,
) -> Result<Option<ProjectSession>, LeyCoreError> {
    connection
        .query_row(
            "SELECT host_kind, external_session_id, state, started_at_unix_ms,
                    ended_at_unix_ms, created_at_unix_ms, erased_at_unix_ms
             FROM project_sessions WHERE project_id = ?1 AND session_id = ?2",
            params![project_id, session_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|error| store.database_error(error))?
        .map(
            |(host_kind, external_session_id, state, started, ended, created, erased)| {
                Ok(ProjectSession {
                    project_id: project_id.to_owned(),
                    session_id: session_id.to_owned(),
                    host_kind,
                    external_session_id,
                    state: ProjectSessionState::parse(&state)?,
                    started_at_unix_ms: started
                        .map(|value| sqlite_u64(value, "session start time"))
                        .transpose()?,
                    ended_at_unix_ms: ended
                        .map(|value| sqlite_u64(value, "session end time"))
                        .transpose()?,
                    created_at_unix_ms: sqlite_u64(created, "session creation time")?,
                    erased_at_unix_ms: erased
                        .map(|value| sqlite_u64(value, "session erasure time"))
                        .transpose()?,
                })
            },
        )
        .transpose()
}

fn human_action_event(
    handle: &ProjectHandle,
    request_id: &str,
    request_fingerprint: &str,
    recorded_at_unix_ms: u64,
    action: &str,
    exact_target: Value,
) -> ContinuityEventInput {
    ContinuityEventInput {
        event_id: format!("evt_{}", Uuid::new_v4().simple()),
        project_id: handle.project_id.clone(),
        subject_id: None,
        session_id: None,
        session_sequence: None,
        request_id: Some(request_id.to_owned()),
        request_fingerprint: Some(request_fingerprint.to_owned()),
        kind: EVENT_KIND_HUMAN_ACTION_RECORDED.to_owned(),
        payload_version: 1,
        recorded_at_unix_ms,
        revision_head: None,
        revision_branch: None,
        payload: json!({
            "projectGeneration": handle.generation,
            "controlOrigin": HUMAN_CONTROL_ORIGIN,
            "action": action,
            "exactTarget": exact_target,
        }),
    }
}

fn required_payload_string(event: &ContinuityEvent, key: &str) -> Result<String, LeyCoreError> {
    event
        .payload
        .get("exactTarget")
        .and_then(|target| target.get(key))
        .or_else(|| event.payload.get(key))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid_brain(&format!("event {} is missing {key}", event.event_id)))
}

fn required_payload_u64(event: &ContinuityEvent, key: &str) -> Result<u64, LeyCoreError> {
    event
        .payload
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_brain(&format!("event {} is missing {key}", event.event_id)))
}

fn validate_display_name(value: &str, max_chars: usize, label: &str) -> Result<(), LeyCoreError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars || value.chars().any(char::is_control)
    {
        return Err(invalid_brain(&format!(
            "{label} must be non-empty, at most {max_chars} characters, and contain no control characters"
        )));
    }
    Ok(())
}

fn validate_kind_text(value: &str, label: &str) -> Result<(), LeyCoreError> {
    if value.is_empty()
        || value.len() > 64
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(invalid_brain(&format!(
            "{label} must be 1-64 lowercase identifier characters"
        )));
    }
    Ok(())
}

pub(crate) fn validate_request_id(value: &str) -> Result<(), LeyCoreError> {
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(invalid_brain(
            "request ID must be non-empty, bounded text without control characters",
        ));
    }
    Ok(())
}

fn validate_session_id(value: &str) -> Result<(), LeyCoreError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
    {
        return Err(invalid_brain("session ID is invalid"));
    }
    Ok(())
}

#[allow(dead_code)]
fn validate_event_reference_id(value: &str) -> Result<(), LeyCoreError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
    {
        return Err(invalid_brain("event ID is invalid"));
    }
    Ok(())
}

fn validate_brain_id(value: &str, prefix: &str, label: &str) -> Result<(), LeyCoreError> {
    let Some(uuid) = value.strip_prefix(prefix) else {
        return Err(invalid_brain(&format!("{label} must start with {prefix}")));
    };
    if uuid.len() != 32
        || !uuid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || Uuid::parse_str(uuid).is_err()
    {
        return Err(invalid_brain(&format!(
            "{label} must contain a lowercase hexadecimal UUID"
        )));
    }
    Ok(())
}

fn validate_optional_hash(value: Option<&str>) -> Result<(), LeyCoreError> {
    if value.is_some_and(|hash| {
        hash.len() != 71
            || !hash.starts_with("sha256:")
            || !hash[7..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }) {
        return Err(invalid_brain(
            "content hash must be sha256:<64 lowercase hex>",
        ));
    }
    Ok(())
}

pub(crate) fn request_fingerprint(value: &Value) -> Result<String, LeyCoreError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        invalid_brain(&format!(
            "request fingerprint serialization failed: {error}"
        ))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

pub(crate) fn unix_time_ms() -> Result<u64, LeyCoreError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|_| invalid_brain("system clock is before the Unix epoch"))
}

pub(crate) fn u64_i64(value: u64, label: &str) -> Result<i64, LeyCoreError> {
    i64::try_from(value).map_err(|_| invalid_brain(&format!("{label} exceeds SQLite range")))
}

fn sqlite_u64(value: i64, label: &str) -> Result<u64, LeyCoreError> {
    u64::try_from(value).map_err(|_| invalid_brain(&format!("{label} is negative")))
}

pub(crate) fn invalid_brain(message: &str) -> LeyCoreError {
    LeyCoreError::InvalidContinuityStore(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuity_store::{ContinuityProjectObservation, ContinuityProjectOrigin};
    use crate::{CaptureMode, ContinuityEventInput};
    use rusqlite::Connection;
    use serde_json::json;
    use std::fs;
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
        let store = ContinuityStore::at(directory.join(crate::CONTINUITY_DATABASE_FILE));
        store.initialize().unwrap();
        (base, store)
    }

    fn create_brain(store: &ContinuityStore, name: &str, request_id: &str) -> ProjectBrain {
        store
            .create_project_brain(&CreateProjectBrainInput {
                name: name.to_owned(),
                request_id: request_id.to_owned(),
            })
            .unwrap()
    }

    fn handle(brain: &ProjectBrain) -> ProjectHandle {
        ProjectHandle {
            project_id: brain.identity.project_id.clone(),
            generation: brain.generation,
        }
    }

    #[test]
    fn source_only_brain_is_path_independent_replay_safe_and_restart_durable() {
        let (_base, store) = private_store();
        let brain = create_brain(&store, "Sources first", "req_create_sources_first");
        assert_eq!(brain.generation, 1);
        assert!(brain.identity.project_id.starts_with("prj_"));

        let replay = create_brain(&store, "Sources first", "req_create_sources_first");
        assert_eq!(replay, brain);
        let conflict = store.create_project_brain(&CreateProjectBrainInput {
            name: "Different".to_owned(),
            request_id: "req_create_sources_first".to_owned(),
        });
        assert!(matches!(
            conflict,
            Err(LeyCoreError::InvalidContinuityStore(_))
        ));

        let connection = store.open_connection().unwrap();
        let observations: i64 = connection
            .query_row("SELECT count(*) FROM project_observations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            observations, 0,
            "rootless Brain must not invent a project path"
        );
        drop(connection);

        let reopened = ContinuityStore::at(store.path().to_path_buf());
        assert_eq!(
            reopened
                .open_project_brain(&brain.identity.project_id)
                .unwrap(),
            brain
        );
        assert_eq!(reopened.list_project_brains().unwrap(), vec![brain]);
    }

    #[test]
    fn repository_working_copies_require_explicit_authorization_and_generation_fences_revocation() {
        let (base, store) = private_store();
        let brain = create_brain(&store, "Repo attach", "req_create_repo_attach");
        let first = base.path().join("worktree-a");
        let second = base.path().join("worktree-b");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();

        let (repository, locator_a) = store
            .authorize_working_copy(&handle(&brain), &first, "req_attach_a")
            .unwrap();
        let (same_repository, locator_b) = store
            .authorize_working_copy(&handle(&brain), &second, "req_attach_b")
            .unwrap();
        assert_eq!(repository.repository_id, same_repository.repository_id);
        assert_ne!(locator_a.locator_id, locator_b.locator_id);
        assert_eq!(
            store
                .project_repository(&brain.identity.project_id)
                .unwrap()
                .unwrap(),
            repository
        );
        assert_eq!(
            store
                .working_copies(&brain.identity.project_id)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            store
                .open_project_brain(&brain.identity.project_id)
                .unwrap()
                .identity,
            brain.identity
        );

        let other = create_brain(&store, "Other", "req_create_other_repo");
        assert!(store
            .authorize_working_copy(&handle(&other), &first, "req_other_attach_same_path")
            .is_err());

        let next = store
            .revoke_working_copy(&handle(&brain), &locator_a.locator_id, "req_revoke_a")
            .unwrap();
        assert_eq!(next.generation, 2);
        let replay = store
            .revoke_working_copy(&handle(&brain), &locator_a.locator_id, "req_revoke_a")
            .unwrap();
        assert_eq!(
            replay, next,
            "lost-response retry must replay the generation-changing write"
        );

        let stale = store.create_project_source(
            &handle(&brain),
            "reference",
            "stale",
            "req_stale_source_after_revoke",
        );
        assert!(matches!(
            stale,
            Err(LeyCoreError::ProjectGenerationChanged { .. })
        ));
    }

    #[test]
    fn source_versions_retain_exact_bytes_replay_safely_and_erase_dependencies() {
        let (base, store) = private_store();
        let brain = create_brain(&store, "Source versions", "req_create_source_versions");
        let source = store
            .create_project_source(
                &handle(&brain),
                "specification",
                "requirements.md",
                "req_create_requirements_source",
            )
            .unwrap();
        let live = base.path().join("requirements.md");
        fs::write(&live, b"live source").unwrap();
        let locator = store
            .attach_source_path(
                &handle(&brain),
                &source.source_id,
                &live,
                "req_attach_requirements_path",
            )
            .unwrap();
        assert_eq!(locator.state, SourceLocatorState::Observed);

        let input = SourceVersionInput {
            request_id: "req_retain_requirements_v1".to_owned(),
            representation_kind: "redacted-text".to_owned(),
            original_content_hash: Some(format!("sha256:{}", "1".repeat(64))),
            source_bytes: 20,
            transformation: json!({"redacted": true}),
            retained_bytes: b"retained evidence".to_vec(),
        };
        let version = store
            .retain_source_version(&handle(&brain), &source.source_id, &input)
            .unwrap();
        assert_eq!(
            store
                .source_version_bytes(
                    &brain.identity.project_id,
                    &source.source_id,
                    &version.source_version_id,
                )
                .unwrap(),
            b"retained evidence"
        );
        let replay = store
            .retain_source_version(&handle(&brain), &source.source_id, &input)
            .unwrap();
        assert_eq!(replay, version);

        let mut second_occurrence = input.clone();
        second_occurrence.request_id = "req_retain_requirements_v1_again".to_owned();
        let repeated = store
            .retain_source_version(&handle(&brain), &source.source_id, &second_occurrence)
            .unwrap();
        assert_eq!(repeated.source_version_id, version.source_version_id);
        assert_ne!(repeated.occurrence_event_id, version.occurrence_event_id);

        let action = store
            .record_local_human_action(
                &handle(&brain),
                &LocalHumanActionInput {
                    request_id: "req_adopt_requirements".to_owned(),
                    action_kind: "accept-candidate".to_owned(),
                    exact_target: json!({"statement":"Use retained requirement exactly"}),
                    evidence: vec![HumanActionEvidenceInput::SourceVersion {
                        source_id: source.source_id.clone(),
                        source_version_id: version.source_version_id.clone(),
                        selector: json!({"kind":"whole"}),
                    }],
                },
            )
            .unwrap();
        assert_eq!(
            action.payload["controlOrigin"].as_str(),
            Some(HUMAN_CONTROL_ORIGIN)
        );
        assert_eq!(
            action.payload["exactTarget"]["snapshot"]["statement"].as_str(),
            Some("Use retained requirement exactly")
        );

        let reopened = ContinuityStore::at(store.path().to_path_buf());
        assert_eq!(
            reopened
                .project_sources(&brain.identity.project_id)
                .unwrap(),
            vec![source.clone()]
        );
        assert_eq!(
            reopened
                .source_version_bytes(
                    &brain.identity.project_id,
                    &source.source_id,
                    &version.source_version_id,
                )
                .unwrap(),
            b"retained evidence"
        );

        let removed = reopened
            .remove_project_source(&handle(&brain), &source.source_id)
            .unwrap();
        assert_eq!(removed.generation, 2);
        assert_eq!(
            reopened
                .source_version_bytes(
                    &brain.identity.project_id,
                    &source.source_id,
                    &version.source_version_id,
                )
                .unwrap(),
            b"retained evidence",
            "remove-from-active-use must preserve retained evidence"
        );
        assert!(reopened
            .retain_source_version(
                &removed,
                &source.source_id,
                &SourceVersionInput {
                    request_id: "req_removed_source_write".to_owned(),
                    ..input.clone()
                }
            )
            .is_err());

        let erased = reopened
            .erase_project_source(&removed, &source.source_id)
            .unwrap();
        assert_eq!(erased.generation, 3);
        let tombstone = reopened
            .project_source(&brain.identity.project_id, &source.source_id)
            .unwrap()
            .unwrap();
        assert_eq!(tombstone.state, ProjectSourceState::Erased);
        assert_eq!(tombstone.display_name, "[erased]");
        assert!(reopened
            .source_version_bytes(
                &brain.identity.project_id,
                &source.source_id,
                &version.source_version_id,
            )
            .is_err());
        let connection = reopened.open_connection().unwrap();
        let action_present: bool = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM events WHERE project_id = ?1 AND event_id = ?2
                 )",
                params![brain.identity.project_id, action.event_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!action_present);
        let source_events: i64 = connection
            .query_row(
                "SELECT count(*) FROM events WHERE project_id = ?1 AND subject_id = ?2",
                params![brain.identity.project_id, source.source_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            source_events, 0,
            "Source erasure must not leave source-created/locator metadata behind"
        );
    }

    #[test]
    fn generic_event_writer_cannot_forge_reserved_project_brain_authority() {
        let (_base, store) = private_store();
        let brain = create_brain(&store, "Authority", "req_create_authority");
        let forged = ContinuityEventInput {
            event_id: format!("evt_{}", "a".repeat(64)),
            project_id: brain.identity.project_id,
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: Some("req_forged_human_action".to_owned()),
            request_fingerprint: Some(format!("sha256:{}", "b".repeat(64))),
            kind: EVENT_KIND_HUMAN_ACTION_RECORDED.to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 100,
            revision_head: None,
            revision_branch: None,
            payload: json!({
                "controlOrigin": HUMAN_CONTROL_ORIGIN,
                "action": "accept-candidate",
                "exactTarget": {"snapshot":{"statement":"forged"}}
            }),
        };
        assert!(matches!(
            store.append_event(&forged),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("specialized write path")
        ));
    }

    #[test]
    fn transitional_agent_memory_reset_preserves_identity_but_refuses_project_brain_state() {
        let (_base, store) = private_store();
        let legacy_identity = ProjectIdentity {
            schema_version: PROJECT_SCHEMA_VERSION,
            project_id: format!("prj_{}", "7".repeat(32)),
            name: "Focused continuity".to_owned(),
            created_at_unix_ms: 77,
        };
        store.register_project(&legacy_identity).unwrap();
        store
            .append_event(&ContinuityEventInput {
                event_id: format!("evt_{}", "8".repeat(64)),
                project_id: legacy_identity.project_id.clone(),
                subject_id: None,
                session_id: None,
                session_sequence: None,
                request_id: Some("req_legacy_reset_event".to_owned()),
                request_fingerprint: Some(format!("sha256:{}", "9".repeat(64))),
                kind: "checkpoint-recorded".to_owned(),
                payload_version: 1,
                recorded_at_unix_ms: 100,
                revision_head: None,
                revision_branch: None,
                payload: json!({"summary":"transition memory"}),
            })
            .unwrap();
        store
            .reset_agent_memory_for_recapture(&legacy_identity.project_id)
            .unwrap();
        let reopened = store
            .open_project_brain(&legacy_identity.project_id)
            .unwrap();
        assert_eq!(reopened.identity, legacy_identity);
        assert_eq!(reopened.generation, 2);
        let connection = store.open_connection().unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM events WHERE project_id = ?1",
                [&reopened.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 0);

        let brain = create_brain(&store, "Canonical", "req_create_reset_guard");
        store
            .create_project_source(
                &handle(&brain),
                "reference",
                "must survive",
                "req_reset_guard_source",
            )
            .unwrap();
        assert!(matches!(
            store.reset_agent_memory_for_recapture(&brain.identity.project_id),
            Err(LeyCoreError::InvalidContinuityStore(message))
                if message.contains("cannot erase Project Brain canonical state")
        ));
        assert_eq!(
            store
                .project_sources(&brain.identity.project_id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn sessions_and_episodes_are_replay_safe_and_session_erasure_tombstones_identity() {
        let (_base, store) = private_store();
        let brain = create_brain(&store, "Chronicle", "req_create_chronicle");
        let session_id = format!("ses_{}", "2".repeat(32));
        let session = store
            .start_project_session(&handle(&brain), &session_id, Some("codex"), Some("host-1"))
            .unwrap();
        assert_eq!(session.state, ProjectSessionState::Active);
        assert!(store
            .start_project_session(
                &handle(&brain),
                &session_id,
                Some("claude"),
                Some("other-host")
            )
            .is_err());
        assert!(store
            .start_project_session(
                &handle(&brain),
                &format!("ses_{}", "9".repeat(32)),
                None,
                Some("orphan-host")
            )
            .is_err());
        assert_eq!(
            store.project_sessions(&brain.identity.project_id).unwrap(),
            vec![session.clone()]
        );

        let episode = store
            .record_project_episode(
                &handle(&brain),
                Some(&session_id),
                Some(0),
                "req_episode_prompt",
                "user-prompt",
                Some(123),
                json!({"text":"inspect auth"}),
            )
            .unwrap();
        let replay = store
            .record_project_episode(
                &handle(&brain),
                Some(&session_id),
                Some(0),
                "req_episode_prompt",
                "user-prompt",
                Some(123),
                json!({"text":"inspect auth"}),
            )
            .unwrap();
        assert_eq!(replay.event_id, episode.event_id);
        assert!(store
            .record_project_episode(
                &handle(&brain),
                Some(&session_id),
                Some(0),
                "req_episode_prompt",
                "user-prompt",
                Some(123),
                json!({"text":"different"}),
            )
            .is_err());
        assert!(store
            .record_project_episode(
                &handle(&brain),
                Some(&session_id),
                Some(1),
                "req_episode_inferred",
                "authentication-debugging-group",
                Some(124),
                json!({"summary":"model-inferred grouping"}),
            )
            .is_err());

        let action = store
            .record_local_human_action(
                &handle(&brain),
                &LocalHumanActionInput {
                    request_id: "req_review_episode".to_owned(),
                    action_kind: "accept-candidate".to_owned(),
                    exact_target: json!({"statement":"Observed prompt belongs to auth work"}),
                    evidence: vec![HumanActionEvidenceInput::Episode {
                        event_id: episode.event_id.clone(),
                    }],
                },
            )
            .unwrap();

        let finished = store
            .finish_project_session(&handle(&brain), &session_id)
            .unwrap();
        assert_eq!(finished.state, ProjectSessionState::Finished);
        assert!(store
            .record_project_episode(
                &handle(&brain),
                Some(&session_id),
                Some(1),
                "req_after_finish",
                "agent-response",
                None,
                json!({"text":"late"}),
            )
            .is_err());

        let erased = store
            .erase_project_session(&handle(&brain), &session_id)
            .unwrap();
        assert_eq!(erased.generation, 2);
        let erased_session = store
            .project_sessions(&brain.identity.project_id)
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.session_id == session_id)
            .unwrap();
        assert_eq!(erased_session.state, ProjectSessionState::Erased);
        assert_eq!(erased_session.host_kind, None);
        assert_eq!(erased_session.external_session_id, None);
        assert_eq!(erased_session.started_at_unix_ms, None);
        assert_eq!(
            erased_session.created_at_unix_ms,
            erased_session.erased_at_unix_ms.unwrap()
        );
        let connection = store.open_connection().unwrap();
        for event_id in [&episode.event_id, &action.event_id] {
            let present: bool = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM events WHERE project_id = ?1 AND event_id = ?2
                     )",
                    params![brain.identity.project_id, event_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(
                !present,
                "erased session dependency event {event_id} survived"
            );
        }
        assert!(store
            .start_project_session(&erased, &session_id, Some("codex"), Some("host-1"))
            .is_err());
    }

    #[test]
    fn whole_brain_erasure_is_terminal_and_blocks_legacy_resurrection_paths() {
        let (base, store) = private_store();
        let brain = create_brain(&store, "Erase me", "req_create_erase_me");
        let source = store
            .create_project_source(
                &handle(&brain),
                "reference",
                "private.txt",
                "req_create_private_source",
            )
            .unwrap();
        let version = store
            .retain_source_version(
                &handle(&brain),
                &source.source_id,
                &SourceVersionInput {
                    request_id: "req_private_version".to_owned(),
                    representation_kind: "text".to_owned(),
                    original_content_hash: None,
                    source_bytes: 6,
                    transformation: json!({}),
                    retained_bytes: b"secret".to_vec(),
                },
            )
            .unwrap();
        let working = base.path().join("working");
        fs::create_dir(&working).unwrap();
        store
            .authorize_working_copy(&handle(&brain), &working, "req_attach_erase_working")
            .unwrap();

        store.erase_project(&brain.identity.project_id).unwrap();
        store.erase_project(&brain.identity.project_id).unwrap();
        let reopened = ContinuityStore::at(store.path().to_path_buf());
        assert!(matches!(
            reopened.open_project_brain(&brain.identity.project_id),
            Err(LeyCoreError::ProjectErased { .. })
        ));
        assert!(matches!(
            reopened.register_project(&brain.identity),
            Err(LeyCoreError::ProjectErased { .. })
        ));
        let stale_event = ContinuityEventInput {
            event_id: format!("evt_{}", "a".repeat(32)),
            project_id: brain.identity.project_id.clone(),
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: Some("req_resurrect_event".to_owned()),
            request_fingerprint: Some(format!("sha256:{}", "b".repeat(64))),
            kind: "episode-recorded".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 999,
            revision_head: None,
            revision_branch: None,
            payload: json!({"should":"fail"}),
        };
        assert!(matches!(
            reopened.append_event(&stale_event),
            Err(LeyCoreError::ProjectErased { .. })
        ));
        let observation = ContinuityProjectObservation {
            project_id: brain.identity.project_id.clone(),
            root_path: working,
            last_opened_at_unix_ms: 1000,
            continuity_origin: ContinuityProjectOrigin::NativeBorn,
        };
        assert!(matches!(
            reopened.upsert_project_observation(&observation),
            Err(LeyCoreError::ProjectErased { .. })
        ));
        assert!(matches!(
            reopened.sync_legacy_project_observations(&[observation]),
            Err(LeyCoreError::ProjectErased { .. })
        ));
        assert!(reopened
            .source_version_bytes(
                &brain.identity.project_id,
                &source.source_id,
                &version.source_version_id,
            )
            .is_err());
        let connection = reopened.open_connection().unwrap();
        let observations: i64 = connection
            .query_row(
                "SELECT count(*) FROM project_observations WHERE project_id = ?1",
                [&brain.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(observations, 0);
        let lifecycle: (i64, String) = connection
            .query_row(
                "SELECT generation, state FROM project_lifecycle WHERE project_id = ?1",
                [&brain.identity.project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(lifecycle, (2, "erased".to_owned()));
    }

    #[test]
    fn interrupted_brain_erasure_resumes_cleanup_without_reactivating_the_project() {
        let (_base, store) = private_store();
        let brain = create_brain(&store, "Crash erase", "req_create_crash_erase");
        let source = store
            .create_project_source(
                &handle(&brain),
                "reference",
                "retained.txt",
                "req_create_crash_source",
            )
            .unwrap();
        store
            .retain_source_version(
                &handle(&brain),
                &source.source_id,
                &SourceVersionInput {
                    request_id: "req_crash_source_version".to_owned(),
                    representation_kind: "text".to_owned(),
                    original_content_hash: None,
                    source_bytes: 8,
                    transformation: json!({}),
                    retained_bytes: b"retained".to_vec(),
                },
            )
            .unwrap();

        let artifact_dir = store
            .path()
            .parent()
            .unwrap()
            .join("artifact-content")
            .join(&brain.identity.project_id);
        assert!(artifact_dir.exists());

        let mut connection = store.open_connection().unwrap();
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        transaction
            .execute(
                "UPDATE project_lifecycle
                 SET generation = 2, state = 'erasing', cleanup_kind = 'native',
                     cleanup_json = NULL, updated_at_unix_ms = 1000
                 WHERE project_id = ?1",
                [&brain.identity.project_id],
            )
            .unwrap();
        transaction
            .execute(
                "DELETE FROM project_observations WHERE project_id = ?1",
                [&brain.identity.project_id],
            )
            .unwrap();
        transaction
            .execute(
                "DELETE FROM projects WHERE project_id = ?1",
                [&brain.identity.project_id],
            )
            .unwrap();
        transaction.commit().unwrap();
        drop(connection);

        assert!(matches!(
            store.open_project_brain(&brain.identity.project_id),
            Err(LeyCoreError::ProjectErasing { .. })
        ));
        assert!(
            artifact_dir.exists(),
            "simulated crash must leave cleanup pending"
        );

        store.erase_project(&brain.identity.project_id).unwrap();
        assert!(!artifact_dir.exists());
        assert!(matches!(
            store.open_project_brain(&brain.identity.project_id),
            Err(LeyCoreError::ProjectErased { .. })
        ));
        assert!(matches!(
            store.register_project(&brain.identity),
            Err(LeyCoreError::ProjectErased { .. })
        ));
    }

    #[test]
    fn project_isolation_and_portable_output_fail_closed_for_new_source_versions() {
        let (_base, store) = private_store();
        let first = create_brain(&store, "First", "req_create_first_isolation");
        let second = create_brain(&store, "Second", "req_create_second_isolation");
        let source = store
            .create_project_source(
                &handle(&first),
                "reference",
                "only-first",
                "req_first_source",
            )
            .unwrap();
        let version = store
            .retain_source_version(
                &handle(&first),
                &source.source_id,
                &SourceVersionInput {
                    request_id: "req_first_version".to_owned(),
                    representation_kind: "text".to_owned(),
                    original_content_hash: None,
                    source_bytes: 5,
                    transformation: json!({}),
                    retained_bytes: b"first".to_vec(),
                },
            )
            .unwrap();
        assert!(store
            .source_version_bytes(
                &second.identity.project_id,
                &source.source_id,
                &version.source_version_id,
            )
            .is_err());

        let destination = store.path().parent().unwrap().join("portable.sqlite3");
        let error = store
            .export_project_database(&first.identity.project_id, &destination)
            .unwrap_err();
        assert!(
            matches!(error, LeyCoreError::InvalidPortableContinuityBundle(_)),
            "unexpected export error: {error:?}"
        );
        assert!(!destination.exists());
    }

    #[test]
    fn v11_store_migrates_additively_to_project_brain_schema() {
        let (_base, store) = private_store();
        let legacy = crate::ProjectIdentity {
            schema_version: crate::PROJECT_SCHEMA_VERSION,
            project_id: format!("prj_{}", "9".repeat(32)),
            name: "v11 project".to_owned(),
            created_at_unix_ms: 77,
        };
        store.register_project(&legacy).unwrap();

        let connection = Connection::open(store.path()).unwrap();
        connection
            .execute_batch(
                r#"
                PRAGMA foreign_keys = OFF;
                DROP TABLE chronicle_session_capture;
                 DROP TABLE chronicle_capture_grants;
                 DROP TABLE chronicle_episodes;
                 DROP TABLE working_copy_import_heads;
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
                PRAGMA user_version = 11;
                "#,
            )
            .unwrap();
        drop(connection);

        let reopened = ContinuityStore::at(store.path().to_path_buf());
        assert_eq!(
            reopened.schema_version().unwrap(),
            crate::CONTINUITY_SCHEMA_VERSION
        );
        let brain = reopened.open_project_brain(&legacy.project_id).unwrap();
        assert_eq!(brain.identity, legacy);
        assert_eq!(brain.generation, 1);
    }

    #[test]
    fn legacy_project_observation_is_not_working_copy_authority() {
        let (base, store) = private_store();
        let initialized = crate::initialize_project(
            base.path().join("legacy-project"),
            Some("legacy"),
            CaptureMode::Structured,
        );
        assert!(matches!(initialized, Err(LeyCoreError::NotDirectory(_))));
        let root = base.path().join("legacy-project");
        fs::create_dir(&root).unwrap();
        let initialized =
            crate::initialize_project(&root, Some("legacy"), CaptureMode::Structured).unwrap();
        store.register_project(&initialized.identity).unwrap();
        store
            .upsert_project_observation(&ContinuityProjectObservation {
                project_id: initialized.identity.project_id.clone(),
                root_path: root,
                last_opened_at_unix_ms: 100,
                continuity_origin: ContinuityProjectOrigin::NativeBorn,
            })
            .unwrap();
        let connection = store.open_connection().unwrap();
        let authorized: i64 = connection
            .query_row(
                "SELECT count(*) FROM working_copy_locators WHERE project_id = ?1",
                [&initialized.identity.project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            authorized, 0,
            "historical path observation must not grant working-copy authority"
        );
    }
    #[test]
    fn source_erasure_removes_transitive_dependents_of_import_events() {
        let (base, store) = private_store();
        let brain = store
            .create_project_brain(&CreateProjectBrainInput {
                name: "Import dependency".into(),
                request_id: "req_create".into(),
            })
            .unwrap();
        let root = base.path().join("root");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("sensitive.txt"), "retained evidence").unwrap();
        let (_, locator) = store
            .authorize_working_copy(&handle(&brain), &root, "req_attach")
            .unwrap();
        store
            .import_local_project(
                &handle(&brain),
                &crate::LocalImportInput {
                    locator_id: locator.locator_id,
                    selected_root: root,
                    request_id: "req_import".into(),
                },
            )
            .unwrap();
        let connection = store.open_connection().unwrap();
        let imported: String = connection
            .query_row(
                "SELECT event_id FROM events WHERE project_id=?1 AND kind='project-imported'",
                [&brain.identity.project_id],
                |r| r.get(0),
            )
            .unwrap();
        drop(connection);
        let action = store
            .record_local_human_action(
                &handle(&brain),
                &LocalHumanActionInput {
                    request_id: "req_action".into(),
                    action_kind: "accept-candidate".into(),
                    exact_target: json!({"statement":"sensitive imported path"}),
                    evidence: vec![HumanActionEvidenceInput::Episode {
                        event_id: imported.clone(),
                    }],
                },
            )
            .unwrap();
        let dependent = store
            .record_local_human_action(
                &handle(&brain),
                &LocalHumanActionInput {
                    request_id: "req_dependent".into(),
                    action_kind: "accept-candidate".into(),
                    exact_target: json!({"statement":"dependent snapshot"}),
                    evidence: vec![HumanActionEvidenceInput::Episode {
                        event_id: action.event_id.clone(),
                    }],
                },
            )
            .unwrap();
        let source = store
            .project_sources(&brain.identity.project_id)
            .unwrap()
            .remove(0);
        store
            .erase_project_source(&handle(&brain), &source.source_id)
            .unwrap();
        let connection = store.open_connection().unwrap();
        for id in [imported, action.event_id, dependent.event_id] {
            let exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM events WHERE project_id=?1 AND event_id=?2)",
                    params![brain.identity.project_id, id],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(!exists, "erased import dependency survived: {id}");
        }
    }
}
