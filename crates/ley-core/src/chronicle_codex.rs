use crate::chronicle::{
    append_observation_on, resolve_codex_capture_on, ActiveChronicleCapture, ChronicleCaptureRoute,
    ChronicleObservation, EpisodeKind, MAX_CHRONICLE_SESSION_BYTES, MAX_CHRONICLE_SESSION_EVENTS,
};
use crate::continuity_store::read_event_by_request;
use crate::project_brain::{
    invalid_brain, read_session_on, request_fingerprint, require_project_generation_on, u64_i64,
    unix_time_ms,
};
use crate::{
    CaptureMode, ContinuityStore, LeyCoreError, ProjectSession, ProjectSessionState,
    SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS, SESSION_RESPONSE_EVIDENCE_LIMIT_CHARACTERS,
    SESSION_TOOL_COMMAND_LIMIT_CHARACTERS, SESSION_TOOL_RESULT_LIMIT_CHARACTERS,
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

const MAX_HOST_IDENTIFIER_CHARACTERS: usize = 512;
const CHRONICLE_ADAPTER: &str = "codex-hooks-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ChronicleHookResult {
    NotProjectBrain,
    CaptureDisabled {
        reason: String,
    },
    Recorded {
        project_id: String,
        session_id: String,
        event_id: String,
        episode_kind: EpisodeKind,
        replayed: bool,
    },
}

impl ChronicleHookResult {
    pub fn handled(&self) -> bool {
        !matches!(self, Self::NotProjectBrain)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ChronicleHookPreflight {
    NotProjectBrain,
    CaptureDisabled { reason: String },
    CaptureEnabled,
}

pub fn preflight_codex_chronicle_hook(
    project_start: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<ChronicleHookPreflight, LeyCoreError> {
    let connection = store.open_connection()?;
    Ok(
        match resolve_codex_capture_on(&connection, project_start.as_ref(), store)? {
            ChronicleCaptureRoute::NotProjectBrain => ChronicleHookPreflight::NotProjectBrain,
            ChronicleCaptureRoute::Disabled(reason) => {
                ChronicleHookPreflight::CaptureDisabled { reason }
            }
            ChronicleCaptureRoute::Enabled(_) => ChronicleHookPreflight::CaptureEnabled,
        },
    )
}

struct PreparedObservation {
    request_id: String,
    request_fingerprint: String,
    observation: ChronicleObservation,
    turn_reference: Option<String>,
    tool_reference: Option<String>,
    gaps: Vec<String>,
    finish_session: bool,
}

pub fn process_codex_chronicle_hook(
    project_start: impl AsRef<Path>,
    payload: Value,
    store: &ContinuityStore,
) -> Result<ChronicleHookResult, LeyCoreError> {
    let mut connection = store.open_connection()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| store.database_error(error))?;
    let capture = match resolve_codex_capture_on(&transaction, project_start.as_ref(), store)? {
        ChronicleCaptureRoute::NotProjectBrain => return Ok(ChronicleHookResult::NotProjectBrain),
        ChronicleCaptureRoute::Disabled(reason) => {
            return Ok(ChronicleHookResult::CaptureDisabled { reason })
        }
        ChronicleCaptureRoute::Enabled(capture) => capture,
    };
    require_project_generation_on(&transaction, &capture.handle, store)?;

    let object = payload.as_object().ok_or_else(|| {
        LeyCoreError::InvalidSessionRequest("Codex hook payload must be a JSON object".into())
    })?;
    let event = required_identifier(object, "hook_event_name")?;
    let external_session_id = required_identifier(object, "session_id")?;
    let session_id = canonical_session_id(&capture.handle.project_id, &external_session_id);
    let (mut session, session_created) = ensure_session_on(
        &transaction,
        store,
        &capture,
        &session_id,
        &external_session_id,
    )?;

    let mut prepared = prepare_observation(object, &event, &external_session_id, capture.mode)?;
    if session_created && event != "SessionStart" {
        prepared.gaps.push(
            "This is the first retained event for the session; a preceding Codex SessionStart was not observed"
                .into(),
        );
    }
    if let Some(existing) = read_event_by_request(
        &transaction,
        &capture.handle.project_id,
        Some(&session_id),
        &prepared.request_id,
        store.path(),
    )? {
        if existing.request_fingerprint.as_deref() != Some(prepared.request_fingerprint.as_str()) {
            return Err(invalid_brain(
                "stable Codex hook identity was reused with different observable content",
            ));
        }
        let kind = prepared.observation.kind();
        transaction
            .commit()
            .map_err(|error| store.database_error(error))?;
        return Ok(ChronicleHookResult::Recorded {
            project_id: capture.handle.project_id,
            session_id,
            event_id: existing.event_id,
            episode_kind: kind,
            replayed: true,
        });
    }

    if session.state != ProjectSessionState::Active {
        transaction
            .commit()
            .map_err(|error| store.database_error(error))?;
        return Ok(ChronicleHookResult::CaptureDisabled {
            reason: "Codex session is already finished or erased".into(),
        });
    }

    let mut stats =
        session_capture_stats_on(&transaction, &capture.handle.project_id, &session_id, store)?;
    if stats.event_limit_reached || stats.event_count >= MAX_CHRONICLE_SESSION_EVENTS as u64 {
        transaction
            .commit()
            .map_err(|error| store.database_error(error))?;
        return Ok(ChronicleHookResult::CaptureDisabled {
            reason: "Chronicle session event limit has been reached".into(),
        });
    }

    let body_bytes = retained_body_bytes(&prepared.observation);
    if stats.body_limit_reached
        || stats.retained_bytes.saturating_add(body_bytes as u64)
            > MAX_CHRONICLE_SESSION_BYTES as u64
    {
        let newly_reached = !stats.body_limit_reached;
        omit_observation_bodies(&mut prepared.observation);
        if newly_reached {
            prepared.gaps.push(
                "Chronicle retained-body byte limit reached; this and later message/tool bodies are omitted"
                    .into(),
            );
        }
        stats.body_limit_reached = true;
    } else {
        stats.retained_bytes = stats.retained_bytes.saturating_add(body_bytes as u64);
    }
    if stats.event_count + 1 == MAX_CHRONICLE_SESSION_EVENTS as u64 {
        prepared.gaps.push(
            "Chronicle session event limit reached after this occurrence; later hook events are not retained"
                .into(),
        );
        stats.event_limit_reached = true;
    }

    // The request fingerprint is based on raw observable hook content, not the current
    // retention budget. This keeps exact retries idempotent even after later events exhaust
    // a session budget.
    let written = append_observation_on(
        &transaction,
        store,
        &capture.handle,
        &session_id,
        &prepared.request_id,
        &prepared.request_fingerprint,
        &prepared.observation,
        prepared.turn_reference.as_deref(),
        prepared.tool_reference.as_deref(),
        &prepared.gaps,
    )?;
    stats.event_count += 1;
    update_session_capture_stats_on(
        &transaction,
        &capture.handle.project_id,
        &session_id,
        &stats,
        store,
    )?;

    if prepared.finish_session {
        let now = unix_time_ms()?;
        transaction
            .execute(
                "UPDATE project_sessions
                 SET state='finished', ended_at_unix_ms=?1
                 WHERE project_id=?2 AND session_id=?3 AND state='active'",
                params![
                    u64_i64(now, "Chronicle session end time")?,
                    capture.handle.project_id,
                    session_id
                ],
            )
            .map_err(|error| store.database_error(error))?;
        session = read_session_on(&transaction, &capture.handle.project_id, &session_id, store)?
            .ok_or_else(|| invalid_brain("finished Chronicle session disappeared"))?;
        if session.state != ProjectSessionState::Finished {
            return Err(invalid_brain("Chronicle session end was not persisted"));
        }
    }

    let kind = prepared.observation.kind();
    transaction
        .commit()
        .map_err(|error| store.database_error(error))?;
    Ok(ChronicleHookResult::Recorded {
        project_id: capture.handle.project_id,
        session_id,
        event_id: written.event_id,
        episode_kind: kind,
        replayed: false,
    })
}

fn ensure_session_on(
    transaction: &Transaction<'_>,
    store: &ContinuityStore,
    capture: &ActiveChronicleCapture,
    session_id: &str,
    external_session_id: &str,
) -> Result<(ProjectSession, bool), LeyCoreError> {
    if let Some(session) =
        read_session_on(transaction, &capture.handle.project_id, session_id, store)?
    {
        if session.state == ProjectSessionState::Erased {
            return Ok((session, false));
        }
        if session.host_kind.as_deref() != Some("codex")
            || session.external_session_id.as_deref() != Some(external_session_id)
        {
            return Err(invalid_brain(
                "canonical Codex session identity conflicts with retained provenance",
            ));
        }
        return Ok((session, false));
    }

    let now = unix_time_ms()?;
    let gaps = default_session_gaps(capture.mode);
    let limits = json!({
        "adapter": CHRONICLE_ADAPTER,
        "captureMode": capture.mode,
        "eventLimit": MAX_CHRONICLE_SESSION_EVENTS,
        "retainedBodyByteLimit": MAX_CHRONICLE_SESSION_BYTES,
        "gaps": gaps,
    });
    transaction
        .execute(
            "INSERT INTO project_sessions(
                project_id, session_id, host_kind, external_session_id, state,
                started_at_unix_ms, ended_at_unix_ms, created_at_unix_ms,
                erased_at_unix_ms, observation_limits_json
             ) VALUES (?1, ?2, 'codex', ?3, 'active', ?4, NULL, ?4, NULL, ?5)",
            params![
                capture.handle.project_id,
                session_id,
                external_session_id,
                u64_i64(now, "Chronicle session start time")?,
                serde_json::to_string(&limits).map_err(|error| invalid_brain(&format!(
                    "Chronicle limits serialization failed: {error}"
                )))?
            ],
        )
        .map_err(|error| store.database_error(error))?;
    transaction
        .execute(
            "INSERT INTO chronicle_session_capture(
                project_id, session_id, retained_bytes, event_count,
                body_limit_reached, event_limit_reached
             ) VALUES (?1, ?2, 0, 0, 0, 0)",
            params![capture.handle.project_id, session_id],
        )
        .map_err(|error| store.database_error(error))?;
    let session = read_session_on(transaction, &capture.handle.project_id, session_id, store)?
        .ok_or_else(|| invalid_brain("Chronicle session was not persisted"))?;
    Ok((session, true))
}

#[derive(Default)]
struct SessionCaptureStats {
    retained_bytes: u64,
    event_count: u64,
    body_limit_reached: bool,
    event_limit_reached: bool,
}

fn session_capture_stats_on(
    transaction: &Transaction<'_>,
    project_id: &str,
    session_id: &str,
    store: &ContinuityStore,
) -> Result<SessionCaptureStats, LeyCoreError> {
    let row = transaction
        .query_row(
            "SELECT retained_bytes, event_count, body_limit_reached, event_limit_reached
             FROM chronicle_session_capture WHERE project_id=?1 AND session_id=?2",
            params![project_id, session_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| store.database_error(error))?;
    match row {
        Some((bytes, count, body_limit, event_limit)) => Ok(SessionCaptureStats {
            retained_bytes: u64::try_from(bytes)
                .map_err(|_| invalid_brain("Chronicle retained byte count is negative"))?,
            event_count: u64::try_from(count)
                .map_err(|_| invalid_brain("Chronicle event count is negative"))?,
            body_limit_reached: body_limit,
            event_limit_reached: event_limit,
        }),
        None => {
            // Migrated sessions may predate the M3 capture counter. Count only canonical M3
            // episodes; historical compatibility metadata is bounded by the migration itself.
            let count: i64 = transaction
                .query_row(
                    "SELECT count(*) FROM chronicle_episodes
                     WHERE project_id=?1 AND session_id=?2 AND origin=?3",
                    params![project_id, session_id, CHRONICLE_ADAPTER],
                    |row| row.get(0),
                )
                .map_err(|error| store.database_error(error))?;
            transaction
                .execute(
                    "INSERT INTO chronicle_session_capture(
                        project_id, session_id, retained_bytes, event_count,
                        body_limit_reached, event_limit_reached
                     ) VALUES (?1, ?2, 0, ?3, 0, ?4)",
                    params![
                        project_id,
                        session_id,
                        count,
                        count >= MAX_CHRONICLE_SESSION_EVENTS as i64
                    ],
                )
                .map_err(|error| store.database_error(error))?;
            Ok(SessionCaptureStats {
                event_count: u64::try_from(count)
                    .map_err(|_| invalid_brain("Chronicle event count is negative"))?,
                event_limit_reached: count >= MAX_CHRONICLE_SESSION_EVENTS as i64,
                ..Default::default()
            })
        }
    }
}

fn update_session_capture_stats_on(
    transaction: &Transaction<'_>,
    project_id: &str,
    session_id: &str,
    stats: &SessionCaptureStats,
    store: &ContinuityStore,
) -> Result<(), LeyCoreError> {
    transaction
        .execute(
            "UPDATE chronicle_session_capture
             SET retained_bytes=?1, event_count=?2, body_limit_reached=?3, event_limit_reached=?4
             WHERE project_id=?5 AND session_id=?6",
            params![
                u64_i64(stats.retained_bytes, "Chronicle retained bytes")?,
                u64_i64(stats.event_count, "Chronicle event count")?,
                stats.body_limit_reached,
                stats.event_limit_reached,
                project_id,
                session_id
            ],
        )
        .map_err(|error| store.database_error(error))?;
    Ok(())
}

fn prepare_observation(
    object: &Map<String, Value>,
    event: &str,
    external_session_id: &str,
    mode: CaptureMode,
) -> Result<PreparedObservation, LeyCoreError> {
    let mut gaps = Vec::new();
    let (identity_parts, observation, turn_reference, tool_reference, finish_session, raw_value) =
        match event {
            "SessionStart" => {
                let source =
                    optional_identifier(object, "source")?.unwrap_or_else(|| "unknown".into());
                gaps.extend(default_session_gaps(mode));
                (
                    vec!["session-start".to_owned(), source.clone()],
                    ChronicleObservation::SessionStart {
                        source: source.clone(),
                    },
                    None,
                    None,
                    false,
                    json!({"source": source}),
                )
            }
            "SessionEnd" => {
                let reason =
                    optional_identifier(object, "reason")?.unwrap_or_else(|| "unknown".into());
                (
                    vec!["session-end".to_owned()],
                    ChronicleObservation::SessionEnd {
                        reason: reason.clone(),
                    },
                    None,
                    None,
                    true,
                    json!({"reason": reason}),
                )
            }
            "UserPromptSubmit" => {
                let Some(turn_id) = optional_identifier(object, "turn_id")? else {
                    return missing_identity_gap(event, external_session_id, "turn_id", object);
                };
                let prompt = required_text(object, "prompt")?;
                let (text, retention, truncated) =
                    retain_text(mode, &prompt, SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS);
                let (fingerprint_text, _, _) = retain_text(
                    CaptureMode::Structured,
                    &prompt,
                    SESSION_PROMPT_EVIDENCE_LIMIT_CHARACTERS,
                );
                (
                    vec!["user-prompt".to_owned(), turn_id.clone()],
                    ChronicleObservation::UserPrompt {
                        text,
                        retention,
                        truncated,
                    },
                    Some(turn_id.clone()),
                    None,
                    false,
                    json!({"turnId": turn_id, "retainedPrompt": fingerprint_text}),
                )
            }
            "Stop" => {
                let Some(turn_id) = optional_identifier(object, "turn_id")? else {
                    return missing_identity_gap(event, external_session_id, "turn_id", object);
                };
                let response = object
                    .get("last_assistant_message")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty());
                match response {
                    Some(response) => {
                        let (text, retention, truncated) =
                            retain_text(mode, response, SESSION_RESPONSE_EVIDENCE_LIMIT_CHARACTERS);
                        let (fingerprint_text, _, _) = retain_text(
                            CaptureMode::Structured,
                            response,
                            SESSION_RESPONSE_EVIDENCE_LIMIT_CHARACTERS,
                        );
                        (
                            vec!["agent-response".to_owned(), turn_id.clone()],
                            ChronicleObservation::AgentResponse {
                                text,
                                retention,
                                truncated,
                            },
                            Some(turn_id.clone()),
                            None,
                            false,
                            json!({"turnId": turn_id, "retainedResponse": fingerprint_text}),
                        )
                    }
                    None => {
                        gaps.push(
                            "Stop hook did not include a visible assistant response body".into(),
                        );
                        (
                            vec!["stop-gap".to_owned(), turn_id.clone()],
                            ChronicleObservation::Gap {
                                reason: "assistant-response-unavailable".into(),
                            },
                            Some(turn_id.clone()),
                            None,
                            false,
                            json!({"turnId": turn_id, "responsePresent": false}),
                        )
                    }
                }
            }
            "PostToolUse" => {
                let Some(turn_id) = optional_identifier(object, "turn_id")? else {
                    return missing_identity_gap(event, external_session_id, "turn_id", object);
                };
                let Some(tool_use_id) = optional_identifier(object, "tool_use_id")? else {
                    return missing_identity_gap(event, external_session_id, "tool_use_id", object);
                };
                let tool = required_identifier(object, "tool_name")?;
                if tool != "Bash" {
                    gaps.push(format!(
                        "{tool} is outside M3's configured Codex tool-capture surface"
                    ));
                    (
                        vec!["unsupported-tool".to_owned(), tool_use_id.clone()],
                        ChronicleObservation::Gap {
                            reason: "unsupported-codex-tool".into(),
                        },
                        Some(turn_id.clone()),
                        Some(tool_use_id.clone()),
                        false,
                        json!({"turnId": turn_id, "toolUseId": tool_use_id, "tool": tool}),
                    )
                } else {
                    let command = object
                        .get("tool_input")
                        .and_then(Value::as_object)
                        .and_then(|input| input.get("command"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let result =
                        crate::host_adapter::serialize_tool_response(object.get("tool_response"))?;
                    let (command_body, command_retention, command_truncated) =
                        retain_text(mode, command, SESSION_TOOL_COMMAND_LIMIT_CHARACTERS);
                    let (result_body, result_retention, result_truncated) =
                        retain_text(mode, &result, SESSION_TOOL_RESULT_LIMIT_CHARACTERS);
                    let (fingerprint_command, _, _) = retain_text(
                        CaptureMode::Structured,
                        command,
                        SESSION_TOOL_COMMAND_LIMIT_CHARACTERS,
                    );
                    let (fingerprint_result, _, _) = retain_text(
                        CaptureMode::Structured,
                        &result,
                        SESSION_TOOL_RESULT_LIMIT_CHARACTERS,
                    );
                    let retention = if command_retention == result_retention {
                        command_retention
                    } else {
                        format!("command={command_retention};result={result_retention}")
                    };
                    gaps.push(
                        "Codex PostToolUse reports returned tool output; Ley does not treat it as proof that the command, test, or requested work succeeded"
                            .into(),
                    );
                    (
                        vec!["tool-result".to_owned(), tool_use_id.clone()],
                        ChronicleObservation::ToolResult {
                            tool: tool.clone(),
                            command: command_body,
                            result: result_body,
                            retention,
                            command_truncated,
                            result_truncated,
                        },
                        Some(turn_id.clone()),
                        Some(tool_use_id.clone()),
                        false,
                        json!({
                            "turnId": turn_id,
                            "toolUseId": tool_use_id,
                            "tool": tool,
                            "retainedCommand": fingerprint_command,
                            "retainedResult": fingerprint_result,
                        }),
                    )
                }
            }
            "Interrupt" => {
                let Some(turn_id) = optional_identifier(object, "turn_id")? else {
                    return missing_identity_gap(event, external_session_id, "turn_id", object);
                };
                (
                    vec!["interrupt".to_owned(), turn_id.clone()],
                    ChronicleObservation::Interruption,
                    Some(turn_id.clone()),
                    None,
                    false,
                    json!({"turnId": turn_id}),
                )
            }
            other => {
                gaps.push(format!("unsupported Codex hook event {other:?} was not interpreted as project activity"));
                (
                    vec!["unsupported-event".to_owned(), other.to_owned()],
                    ChronicleObservation::Gap {
                        reason: "unsupported-codex-hook-event".into(),
                    },
                    None,
                    None,
                    false,
                    json!({"hookEventName": other}),
                )
            }
        };

    let request_id = stable_request_id(
        std::iter::once("chronicle")
            .chain(std::iter::once("codex"))
            .chain(std::iter::once(external_session_id))
            .chain(identity_parts.iter().map(String::as_str)),
    );
    let request_fingerprint = request_fingerprint(&json!({
        "adapter": CHRONICLE_ADAPTER,
        "externalSessionId": external_session_id,
        "hookEventName": event,
        "observable": raw_value,
    }))?;
    Ok(PreparedObservation {
        request_id,
        request_fingerprint,
        observation,
        turn_reference,
        tool_reference,
        gaps,
        finish_session,
    })
}

fn missing_identity_gap(
    event: &str,
    external_session_id: &str,
    field: &str,
    object: &Map<String, Value>,
) -> Result<PreparedObservation, LeyCoreError> {
    let serialized = serde_json::to_string(&Value::Object(object.clone())).map_err(|error| {
        invalid_brain(&format!(
            "Codex hook identity serialization failed: {error}"
        ))
    })?;
    let (redacted, _) = crate::ingestion::redact_secrets(&serialized);
    let observable_hash = request_fingerprint(&Value::String(redacted))?;
    let request_id = stable_request_id([
        "chronicle",
        "codex",
        external_session_id,
        "missing-stable-identity",
        event,
        field,
        &observable_hash,
    ]);
    let reason = format!(
        "Codex {event} hook omitted stable {field}; exact occurrence identity is unavailable, so only byte-equivalent payload retries can be deduplicated"
    );
    Ok(PreparedObservation {
        request_id,
        request_fingerprint: request_fingerprint(&json!({
            "adapter": CHRONICLE_ADAPTER,
            "externalSessionId": external_session_id,
            "hookEventName": event,
            "missing": field,
            "observableHash": observable_hash,
        }))?,
        observation: ChronicleObservation::Gap {
            reason: "stable-host-occurrence-identity-unavailable".into(),
        },
        turn_reference: None,
        tool_reference: None,
        gaps: vec![reason],
        finish_session: false,
    })
}

fn default_session_gaps(mode: CaptureMode) -> Vec<String> {
    let mut gaps = vec![
        "Codex hooks do not expose hidden reasoning; Ley does not read transcript_path".into(),
        "M3 observes configured Bash PostToolUse only; other tool activity is not claimed as captured".into(),
    ];
    if mode == CaptureMode::Minimal {
        gaps.push(
            "Minimal retention omits visible message, command, and tool-result bodies".into(),
        );
    }
    gaps
}

fn retain_text(mode: CaptureMode, text: &str, limit: usize) -> (Option<String>, String, bool) {
    if mode == CaptureMode::Minimal {
        return (None, "metadata-only".into(), false);
    }
    let (redacted, _) = crate::ingestion::redact_secrets(text);
    let (bounded, truncated) = truncate_characters(&redacted, limit);
    (Some(bounded), "bounded-redacted".into(), truncated)
}

fn truncate_characters(value: &str, max: usize) -> (String, bool) {
    if value.chars().count() <= max {
        return (value.to_owned(), false);
    }
    const MARKER: &str = "\n[TRUNCATED BY LEY]";
    let marker_len = MARKER.chars().count();
    let mut output = value
        .chars()
        .take(max.saturating_sub(marker_len))
        .collect::<String>();
    output.push_str(MARKER);
    (output, true)
}

fn retained_body_bytes(observation: &ChronicleObservation) -> usize {
    match observation {
        ChronicleObservation::UserPrompt { text, .. }
        | ChronicleObservation::AgentResponse { text, .. } => {
            text.as_deref().map_or(0, |value| value.len())
        }
        ChronicleObservation::ToolCall { command, .. } => {
            command.as_deref().map_or(0, |value| value.len())
        }
        ChronicleObservation::ToolResult {
            command, result, ..
        } => {
            command.as_deref().map_or(0, |value| value.len())
                + result.as_deref().map_or(0, |value| value.len())
        }
        _ => 0,
    }
}

fn omit_observation_bodies(observation: &mut ChronicleObservation) {
    match observation {
        ChronicleObservation::UserPrompt {
            text, retention, ..
        }
        | ChronicleObservation::AgentResponse {
            text, retention, ..
        } => {
            *text = None;
            *retention = "session-body-limit".into();
        }
        ChronicleObservation::ToolCall {
            command, retention, ..
        } => {
            *command = None;
            *retention = "session-body-limit".into();
        }
        ChronicleObservation::ToolResult {
            command,
            result,
            retention,
            ..
        } => {
            *command = None;
            *result = None;
            *retention = "session-body-limit".into();
        }
        _ => {}
    }
}

fn required_identifier(object: &Map<String, Value>, field: &str) -> Result<String, LeyCoreError> {
    let value = required_text(object, field)?;
    validate_identifier(field, &value)?;
    Ok(value)
}

fn optional_identifier(
    object: &Map<String, Value>,
    field: &str,
) -> Result<Option<String>, LeyCoreError> {
    let Some(value) = object.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value.as_str().ok_or_else(|| {
        LeyCoreError::InvalidSessionRequest(format!("Codex hook {field} must be text"))
    })?;
    validate_identifier(field, value)?;
    Ok(Some(value.to_owned()))
}

fn required_text(object: &Map<String, Value>, field: &str) -> Result<String, LeyCoreError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            LeyCoreError::InvalidSessionRequest(format!(
                "Codex hook requires non-empty text field {field}"
            ))
        })
}

fn validate_identifier(field: &str, value: &str) -> Result<(), LeyCoreError> {
    if value.is_empty()
        || value.chars().count() > MAX_HOST_IDENTIFIER_CHARACTERS
        || value.chars().any(char::is_control)
    {
        return Err(LeyCoreError::InvalidSessionRequest(format!(
            "Codex hook {field} is empty, too long, or contains control characters"
        )));
    }
    Ok(())
}

fn canonical_session_id(project_id: &str, external_session_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ley-chronicle-session\0");
    digest.update(project_id.as_bytes());
    digest.update([0]);
    digest.update(b"codex\0");
    digest.update(external_session_id.as_bytes());
    format!("ses_{:x}", digest.finalize())[..36].to_owned()
}

fn stable_request_id<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(part.as_bytes());
        digest.update([0]);
    }
    let hash = format!("{:x}", digest.finalize());
    format!("req_{}", &hash[..32])
}
