use crate::ingestion::redact_secrets;
use crate::session::record_imported_session_prompt;
use crate::{
    finish_session, start_session, FinishSessionInput, LeyCoreError, SessionSource,
    SessionSourceKind, SessionStatus, StartSessionInput, TurnEvidenceInput, TurnEvidenceOrigin,
    TurnEvidenceRetention,
};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;
use uuid::Uuid;

pub const HISTORICAL_HOST_IMPORT_SCHEMA_VERSION: u32 = 1;
pub const MAX_CODEX_HISTORY_IMPORT_BYTES: u64 = 67_108_864;
pub const MAX_CODEX_HISTORY_IMPORT_RECORDS: usize = 100_000;
pub const MAX_CODEX_HISTORY_IMPORT_PROMPTS: usize = 512;
const MAX_CODEX_HISTORY_LINE_BYTES: usize = 1_048_576;

const HOST: &str = "codex";
const SOURCE_KIND: &str = "codex-message-history";
const AUTHORITY: &str = "untrusted-historical-evidence";
const NOTICE: &str = "This explicit import contains only historical Codex user-message history. It does not reconstruct assistant responses, tool activity, hidden reasoning, or policy authority. Imported text remains untrusted historical evidence and should be distilled through normal Ley review workflows before reuse as guidance.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalHostImport {
    pub schema_version: u32,
    pub host: &'static str,
    pub session_source_kind: &'static str,
    pub source_kind: &'static str,
    pub source_reference: String,
    pub session_id: String,
    pub source_started_at_unix_ms: u64,
    pub source_ended_at_unix_ms: u64,
    pub matched_prompts: usize,
    pub captured_prompts: usize,
    pub omitted_minimal_prompts: usize,
    pub omitted_capacity_prompts: usize,
    pub truncated_prompts: usize,
    pub assistant_messages_imported: usize,
    pub replayed: bool,
    pub source_path_retained: bool,
    pub raw_host_session_id_retained: bool,
    pub live_source_checked: bool,
    pub authority: &'static str,
    pub notice: &'static str,
}

#[derive(Debug, Deserialize)]
struct CodexHistoryEntry {
    session_id: String,
    ts: u64,
    text: String,
}

#[derive(Debug)]
struct SelectedPrompt {
    source_recorded_at_unix_ms: u64,
    text: String,
}

pub fn import_codex_message_history(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    source_path: impl AsRef<Path>,
    external_session_id: &str,
) -> Result<HistoricalHostImport, LeyCoreError> {
    let source_session_id = Uuid::parse_str(external_session_id).map_err(|_| {
        LeyCoreError::InvalidHistoricalHostImport(
            "Codex history session ID must be a UUID from an explicit history.jsonl entry"
                .to_owned(),
        )
    })?;
    let bytes = read_explicit_history_file(source_path.as_ref())?;
    let prompts = select_codex_prompts(&bytes, source_session_id)?;
    if prompts.is_empty() {
        return Err(LeyCoreError::InvalidHistoricalHostImport(
            "the explicit Codex history file contains no messages for that session ID".to_owned(),
        ));
    }
    if prompts.len() > MAX_CODEX_HISTORY_IMPORT_PROMPTS {
        return Err(LeyCoreError::InvalidHistoricalHostImport(format!(
            "one Codex history import may contain at most {MAX_CODEX_HISTORY_IMPORT_PROMPTS} selected messages"
        )));
    }

    let source_reference = opaque_source_reference(source_session_id);
    let snapshot_digest = selected_snapshot_digest(&source_reference, &prompts);
    let start_request_id = import_request_id("start", &snapshot_digest, None);
    let project_start = project_start.as_ref();
    let vault = vault.as_ref();
    let short_reference = &source_reference["hsi_".len().."hsi_".len() + 8];
    let started = start_session(
        project_start,
        vault,
        StartSessionInput {
            request_id: start_request_id,
            name: format!("Imported Codex history {short_reference}"),
            goal: "Preserve an explicitly selected historical Codex user-message snapshot as bounded, untrusted evidence. Assistant responses and tool activity are not present in this source."
                .to_owned(),
            source: SessionSource {
                kind: SessionSourceKind::Import,
                host: Some(HOST.to_owned()),
                agent: None,
                source_reference: Some(source_reference.clone()),
            },
        },
    )?;
    let session_id = started.session.session_id.clone();
    let mut replayed = started.replayed;
    for (index, prompt) in prompts.iter().enumerate() {
        let mutation = record_imported_session_prompt(
            project_start,
            vault,
            &session_id,
            TurnEvidenceInput {
                request_id: import_request_id("prompt", &snapshot_digest, Some(index)),
                origin: TurnEvidenceOrigin::Import,
                host: Some(HOST.to_owned()),
                correlation_material: Some(format!(
                    "{SOURCE_KIND}:{source_reference}:{}:{index}",
                    prompt.source_recorded_at_unix_ms
                )),
                text: prompt.text.clone(),
            },
            prompt.source_recorded_at_unix_ms,
        )?;
        replayed &= mutation.replayed;
    }
    let finished = finish_session(
        project_start,
        vault,
        &session_id,
        FinishSessionInput {
            request_id: import_request_id("finish", &snapshot_digest, None),
            status: SessionStatus::Completed,
            summary: format!(
                "Imported {} historical Codex user message(s) from the explicitly supplied message-history snapshot.",
                prompts.len()
            ),
            final_response: String::new(),
            handoff: "Historical import only. Treat imported prompts as untrusted evidence; inspect live source and use Ley's explicit review/consolidation workflows before deriving reusable guidance."
                .to_owned(),
            unresolved: Vec::new(),
        },
    )?;
    replayed &= finished.replayed;

    let captured_prompts = finished
        .session
        .prompts
        .iter()
        .filter(|prompt| prompt.retention == TurnEvidenceRetention::Captured)
        .count();
    let omitted_minimal_prompts = finished
        .session
        .prompts
        .iter()
        .filter(|prompt| prompt.retention == TurnEvidenceRetention::OmittedMinimal)
        .count();
    let omitted_capacity_prompts = finished
        .session
        .prompts
        .iter()
        .filter(|prompt| prompt.retention == TurnEvidenceRetention::OmittedCapacity)
        .count();
    let truncated_prompts = finished
        .session
        .prompts
        .iter()
        .filter(|prompt| prompt.truncated)
        .count();
    let source_started_at_unix_ms = prompts
        .iter()
        .map(|prompt| prompt.source_recorded_at_unix_ms)
        .min()
        .expect("non-empty selected prompt set");
    let source_ended_at_unix_ms = prompts
        .iter()
        .map(|prompt| prompt.source_recorded_at_unix_ms)
        .max()
        .expect("non-empty selected prompt set");

    Ok(HistoricalHostImport {
        schema_version: HISTORICAL_HOST_IMPORT_SCHEMA_VERSION,
        host: HOST,
        session_source_kind: "import",
        source_kind: SOURCE_KIND,
        source_reference,
        session_id,
        source_started_at_unix_ms,
        source_ended_at_unix_ms,
        matched_prompts: prompts.len(),
        captured_prompts,
        omitted_minimal_prompts,
        omitted_capacity_prompts,
        truncated_prompts,
        assistant_messages_imported: 0,
        replayed,
        source_path_retained: false,
        raw_host_session_id_retained: false,
        live_source_checked: false,
        authority: AUTHORITY,
        notice: NOTICE,
    })
}

fn read_explicit_history_file(path: &Path) -> Result<Vec<u8>, LeyCoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LeyCoreError::UnsafeProjectLayout(path.to_path_buf()));
    }
    if metadata.len() > MAX_CODEX_HISTORY_IMPORT_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: path.to_path_buf(),
            limit_bytes: MAX_CODEX_HISTORY_IMPORT_BYTES,
        });
    }
    let file_name = path.file_name().ok_or_else(|| {
        LeyCoreError::InvalidHistoricalHostImport(
            "Codex history source must identify one explicit regular file".to_owned(),
        )
    })?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let directory =
        Dir::open_ambient_dir(parent, ambient_authority()).map_err(|source| LeyCoreError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut file = directory
        .open_with(file_name, &options)
        .map_err(|source| LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    let opened_metadata = file.metadata().map_err(|source| LeyCoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if !opened_metadata.is_file() {
        return Err(LeyCoreError::UnsafeProjectLayout(path.to_path_buf()));
    }
    if opened_metadata.len() > MAX_CODEX_HISTORY_IMPORT_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: path.to_path_buf(),
            limit_bytes: MAX_CODEX_HISTORY_IMPORT_BYTES,
        });
    }
    let mut bytes = Vec::with_capacity(opened_metadata.len() as usize);
    file.by_ref()
        .take(MAX_CODEX_HISTORY_IMPORT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| LeyCoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() as u64 > MAX_CODEX_HISTORY_IMPORT_BYTES {
        return Err(LeyCoreError::MetadataTooLarge {
            path: path.to_path_buf(),
            limit_bytes: MAX_CODEX_HISTORY_IMPORT_BYTES,
        });
    }
    Ok(bytes)
}

fn select_codex_prompts(
    bytes: &[u8],
    selected_session_id: Uuid,
) -> Result<Vec<SelectedPrompt>, LeyCoreError> {
    let mut prompts = Vec::new();
    let mut records = 0usize;
    for (index, raw_line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        let line = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        records += 1;
        if records > MAX_CODEX_HISTORY_IMPORT_RECORDS {
            return Err(LeyCoreError::InvalidHistoricalHostImport(format!(
                "Codex history source exceeds the {MAX_CODEX_HISTORY_IMPORT_RECORDS}-record bound"
            )));
        }
        if line.len() > MAX_CODEX_HISTORY_LINE_BYTES {
            return Err(LeyCoreError::InvalidHistoricalHostImport(format!(
                "Codex history line {} exceeds the {}-byte record bound",
                index + 1,
                MAX_CODEX_HISTORY_LINE_BYTES
            )));
        }
        let entry: CodexHistoryEntry = serde_json::from_slice(line).map_err(|_| {
            LeyCoreError::InvalidHistoricalHostImport(format!(
                "Codex history line {} does not match the supported message-history JSONL shape",
                index + 1
            ))
        })?;
        let entry_session_id = Uuid::parse_str(&entry.session_id).map_err(|_| {
            LeyCoreError::InvalidHistoricalHostImport(format!(
                "Codex history line {} has an invalid session UUID",
                index + 1
            ))
        })?;
        if entry.ts == 0 {
            return Err(LeyCoreError::InvalidHistoricalHostImport(format!(
                "Codex history line {} has an invalid zero timestamp",
                index + 1
            )));
        }
        if entry_session_id != selected_session_id {
            continue;
        }
        if prompts.len() >= MAX_CODEX_HISTORY_IMPORT_PROMPTS {
            return Err(LeyCoreError::InvalidHistoricalHostImport(format!(
                "one Codex history import may contain at most {MAX_CODEX_HISTORY_IMPORT_PROMPTS} selected messages"
            )));
        }
        let trimmed = entry.text.trim();
        if trimmed.is_empty()
            || trimmed.chars().any(|character| {
                character == '\0'
                    || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
            })
        {
            return Err(LeyCoreError::InvalidHistoricalHostImport(format!(
                "Codex history line {} contains an empty or unsafe selected message",
                index + 1
            )));
        }
        let source_recorded_at_unix_ms = entry.ts.checked_mul(1_000).ok_or_else(|| {
            LeyCoreError::InvalidHistoricalHostImport(format!(
                "Codex history line {} timestamp cannot be represented in milliseconds",
                index + 1
            ))
        })?;
        prompts.push(SelectedPrompt {
            source_recorded_at_unix_ms,
            text: entry.text,
        });
    }
    Ok(prompts)
}

fn opaque_source_reference(session_id: Uuid) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-codex-history-source-v1");
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    format!("hsi_{:x}", hasher.finalize())
}

fn selected_snapshot_digest(source_reference: &str, prompts: &[SelectedPrompt]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-codex-history-import-snapshot-v1");
    hasher.update([0]);
    hasher.update(source_reference.as_bytes());
    for prompt in prompts {
        let (redacted, _) = redact_secrets(prompt.text.trim());
        hasher.update([0]);
        hasher.update(prompt.source_recorded_at_unix_ms.to_le_bytes());
        hasher.update((redacted.len() as u64).to_le_bytes());
        hasher.update(redacted.as_bytes());
    }
    hasher.finalize().into()
}

fn import_request_id(domain: &str, snapshot_digest: &[u8; 32], index: Option<usize>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ley-codex-history-import-request-v1");
    hasher.update([0]);
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(snapshot_digest);
    if let Some(index) = index {
        hasher.update([0]);
        hasher.update((index as u64).to_le_bytes());
    }
    let hex = format!("{:x}", hasher.finalize());
    format!("req_{}", &hex[..32])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compile_session_memory, import_codex_message_history, ingest_project, initialize_project,
        memory_health_report, project_resume_context, read_session, read_session_turns_context,
        search_project_memory, CaptureMode, MemoryHealthLimits, ProjectMemoryResultKind,
        ProjectMemorySearchLimits, SessionSourceKind, DEFAULT_MEMORY_COMPILE_CHARACTERS,
        DEFAULT_MEMORY_COMPILE_RESULTS, DEFAULT_RESUME_CHARACTERS, DEFAULT_RESUME_LEARNINGS,
        DEFAULT_RESUME_SESSIONS, DEFAULT_SESSION_TURN_CHARACTERS, DEFAULT_SESSION_TURN_RESULTS,
        SESSION_IMPORTED_TURN_SCHEMA_VERSION,
    };
    use tempfile::tempdir;

    fn setup(mode: CaptureMode) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        fs::create_dir(&project).unwrap();
        fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Import fixture"), mode).unwrap();
        fs::write(project.join("README.md"), "# Import fixture\n").unwrap();
        ingest_project(&project, &vault).unwrap();
        (base, project, vault)
    }

    #[test]
    fn explicit_codex_history_import_is_bounded_redacted_source_pinned_and_idempotent() {
        let (base, project, vault) = setup(CaptureMode::Structured);
        let selected = Uuid::new_v4();
        let other = Uuid::new_v4();
        let source = base.path().join("history.jsonl");
        let secret = "sk-test-super-secret-token-1234567890";
        let body = format!(
            "{{\"session_id\":\"{other}\",\"ts\":1700000000,\"text\":\"UNRELATED_IMPORT_CANARY\"}}\n{{\"session_id\":\"{selected}\",\"ts\":1700000001,\"text\":\"Investigate the cache with api_key={secret}\"}}\n{{\"session_id\":\"{selected}\",\"ts\":1700000002,\"text\":\"Verify the bounded import\"}}\n"
        );
        fs::write(&source, body).unwrap();

        let imported =
            import_codex_message_history(&project, &vault, &source, &selected.to_string()).unwrap();
        assert_eq!(imported.matched_prompts, 2);
        assert_eq!(imported.captured_prompts, 2);
        assert_eq!(imported.assistant_messages_imported, 0);
        assert_eq!(imported.source_started_at_unix_ms, 1_700_000_001_000);
        assert_eq!(imported.source_ended_at_unix_ms, 1_700_000_002_000);
        assert!(imported.source_reference.starts_with("hsi_"));
        assert!(!imported.replayed);
        assert!(!imported.live_source_checked);
        assert!(!imported.source_path_retained);
        assert!(!imported.raw_host_session_id_retained);

        let session = read_session(&project, &vault, &imported.session_id).unwrap();
        assert_eq!(session.schema_version, SESSION_IMPORTED_TURN_SCHEMA_VERSION);
        assert_eq!(session.status, SessionStatus::Completed);
        assert_eq!(session.source.kind, SessionSourceKind::Import);
        assert_eq!(session.source.host.as_deref(), Some("codex"));
        assert_eq!(
            session.source.source_reference.as_deref(),
            Some(imported.source_reference.as_str())
        );
        assert_eq!(session.prompts.len(), 2);
        assert!(session.responses.is_empty());
        assert!(session.checkpoints.is_empty());
        assert_eq!(
            session.prompts[0].source_recorded_at_unix_ms,
            Some(1_700_000_001_000)
        );
        let retained = session
            .prompts
            .iter()
            .filter_map(|prompt| prompt.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!retained.contains(secret));
        assert!(!retained.contains("UNRELATED_IMPORT_CANARY"));
        assert!(retained.contains("[REDACTED:"));
        let turns = read_session_turns_context(
            &project,
            &vault,
            &imported.session_id,
            DEFAULT_SESSION_TURN_RESULTS,
            DEFAULT_SESSION_TURN_CHARACTERS,
        )
        .unwrap();
        assert_eq!(
            turns.turns[0].source_recorded_at_unix_ms,
            Some(1_700_000_001_000)
        );
        assert_eq!(
            turns.turns[0].source_boundary,
            "untrusted-imported-host-history"
        );
        let session_dir = vault
            .join(".ley/agent-memory/projects")
            .join(&session.project_id)
            .join("sessions")
            .join(&session.session_id);
        assert!(session_dir.join("session-v7.json").exists());
        let markdown = fs::read_to_string(session_dir.join("session.md")).unwrap();
        assert!(markdown.contains(&imported.source_reference));
        assert!(markdown.contains("Source recorded at:"));

        let resume = project_resume_context(
            &project,
            &vault,
            DEFAULT_RESUME_SESSIONS,
            DEFAULT_RESUME_LEARNINGS,
            DEFAULT_RESUME_CHARACTERS,
        )
        .unwrap();
        assert_eq!(resume.total_sessions, 1);
        assert_eq!(resume.excluded_imported_sessions, 1);
        assert!(resume.sessions.is_empty());

        let compilation = compile_session_memory(
            &project,
            &vault,
            &imported.session_id,
            DEFAULT_MEMORY_COMPILE_RESULTS,
            DEFAULT_MEMORY_COMPILE_CHARACTERS,
        )
        .unwrap();
        assert!(!compilation.can_checkpoint);
        assert_eq!(compilation.evidence.len(), 2);
        assert_eq!(
            compilation.evidence[0].source_recorded_at_unix_ms,
            Some(1_700_000_001_000)
        );
        assert_eq!(
            compilation.evidence[0].source_boundary,
            "untrusted-imported-host-history"
        );

        let health = memory_health_report(&project, &vault, MemoryHealthLimits::default()).unwrap();
        assert!(health
            .signals
            .iter()
            .all(|signal| { !signal.related_session_ids.contains(&imported.session_id) }));

        let search = search_project_memory(
            &project,
            &vault,
            "Imported Codex history",
            ProjectMemorySearchLimits::default(),
            None,
        )
        .unwrap();
        let imported_session = search
            .results
            .iter()
            .find(|result| {
                result.kind == ProjectMemoryResultKind::Session
                    && result.entity_id == imported.session_id
            })
            .expect("explicit imported session remains discoverable");
        assert_eq!(imported_session.updated_at_unix_ms, 1_700_000_002_000);

        let retry =
            import_codex_message_history(&project, &vault, &source, &selected.to_string()).unwrap();
        assert!(retry.replayed);
        assert_eq!(retry.session_id, imported.session_id);
        assert_eq!(
            read_session(&project, &vault, &retry.session_id)
                .unwrap()
                .event_count,
            session.event_count
        );

        let changed_secret = "sk-test-different-secret-token-0987654321";
        let secret_only_body = format!(
            "{{\"session_id\":\"{other}\",\"ts\":1700000000,\"text\":\"UNRELATED_IMPORT_CANARY\"}}\n{{\"session_id\":\"{selected}\",\"ts\":1700000001,\"text\":\"Investigate the cache with api_key={changed_secret}\"}}\n{{\"session_id\":\"{selected}\",\"ts\":1700000002,\"text\":\"Verify the bounded import\"}}\n"
        );
        fs::write(&source, secret_only_body).unwrap();
        let secret_only_retry =
            import_codex_message_history(&project, &vault, &source, &selected.to_string()).unwrap();
        assert!(secret_only_retry.replayed);
        assert_eq!(secret_only_retry.session_id, imported.session_id);
        assert_eq!(
            secret_only_retry.source_reference,
            imported.source_reference
        );
        assert_eq!(
            read_session(&project, &vault, &secret_only_retry.session_id)
                .unwrap()
                .event_count,
            session.event_count
        );

        let vault_text = walk_text(&vault);
        assert!(!vault_text.contains(source.to_string_lossy().as_ref()));
        assert!(!vault_text.contains(&selected.to_string()));
        assert!(!vault_text.contains("UNRELATED_IMPORT_CANARY"));
        assert!(!vault_text.contains(secret));
        assert!(!vault_text.contains(changed_secret));
    }

    #[test]
    fn changed_selected_codex_history_creates_a_new_immutable_import_snapshot() {
        let (base, project, vault) = setup(CaptureMode::Structured);
        let selected = Uuid::new_v4();
        let source = base.path().join("history.jsonl");
        fs::write(
            &source,
            format!(
                "{{\"session_id\":\"{selected}\",\"ts\":1700000001,\"text\":\"First historical request\"}}\n"
            ),
        )
        .unwrap();
        let first =
            import_codex_message_history(&project, &vault, &source, &selected.to_string()).unwrap();
        fs::write(
            &source,
            format!(
                "{{\"session_id\":\"{selected}\",\"ts\":1700000001,\"text\":\"First historical request\"}}\n{{\"session_id\":\"{selected}\",\"ts\":1700000002,\"text\":\"Later historical request\"}}\n"
            ),
        )
        .unwrap();
        let second =
            import_codex_message_history(&project, &vault, &source, &selected.to_string()).unwrap();
        assert_ne!(first.session_id, second.session_id);
        assert_eq!(first.source_reference, second.source_reference);
        assert_eq!(
            read_session(&project, &vault, &first.session_id)
                .unwrap()
                .prompts
                .len(),
            1
        );
        assert_eq!(
            read_session(&project, &vault, &second.session_id)
                .unwrap()
                .prompts
                .len(),
            2
        );
    }

    #[test]
    fn minimal_capture_import_discloses_turns_without_retaining_bodies() {
        let (base, project, vault) = setup(CaptureMode::Minimal);
        let selected = Uuid::new_v4();
        let source = base.path().join("history.jsonl");
        fs::write(
            &source,
            format!(
                "{{\"session_id\":\"{selected}\",\"ts\":1700000001,\"text\":\"MINIMAL_IMPORT_BODY_CANARY\"}}\n"
            ),
        )
        .unwrap();
        let imported =
            import_codex_message_history(&project, &vault, &source, &selected.to_string()).unwrap();
        assert_eq!(imported.captured_prompts, 0);
        assert_eq!(imported.omitted_minimal_prompts, 1);
        let session = read_session(&project, &vault, &imported.session_id).unwrap();
        assert_eq!(
            session.prompts[0].retention,
            TurnEvidenceRetention::OmittedMinimal
        );
        assert!(session.prompts[0].text.is_none());
        assert!(!walk_text(&vault).contains("MINIMAL_IMPORT_BODY_CANARY"));
    }

    #[cfg(unix)]
    #[test]
    fn import_rejects_symlink_and_malformed_or_missing_selected_history() {
        use std::os::unix::fs::symlink;

        let (base, project, vault) = setup(CaptureMode::Structured);
        let selected = Uuid::new_v4();
        let source = base.path().join("history.jsonl");
        fs::write(&source, "not-json\n").unwrap();
        assert!(matches!(
            import_codex_message_history(&project, &vault, &source, &selected.to_string()),
            Err(LeyCoreError::InvalidHistoricalHostImport(_))
        ));

        fs::write(
            &source,
            format!(
                "{{\"session_id\":\"{}\",\"ts\":1700000001,\"text\":\"Other session\"}}\n",
                Uuid::new_v4()
            ),
        )
        .unwrap();
        assert!(matches!(
            import_codex_message_history(&project, &vault, &source, &selected.to_string()),
            Err(LeyCoreError::InvalidHistoricalHostImport(_))
        ));

        let link = base.path().join("history-link.jsonl");
        symlink(&source, &link).unwrap();
        assert!(matches!(
            import_codex_message_history(&project, &vault, &link, &selected.to_string()),
            Err(LeyCoreError::UnsafeProjectLayout(_))
        ));
    }

    fn walk_text(directory: &Path) -> String {
        let mut output = String::new();
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                output.push_str(&walk_text(&path));
            } else if let Ok(text) = fs::read_to_string(&path) {
                output.push_str(&text);
            }
        }
        output
    }
}
