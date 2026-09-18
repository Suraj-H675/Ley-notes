use crate::{
    diagnose_project, evaluate_agent_egress, read_learning, AgentEgressTarget,
    ContextMountRegistry, EgressPolicyRegistry, LearningFreshness, LearningKind,
    LearningProvenance, LearningState, LearningTrustState, LeyCoreError,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const REVIEWED_RUNBOOK_SCHEMA_VERSION: u32 = 1;
pub const MAX_REVIEWED_RUNBOOK_SOURCES: usize = 20;
pub const MAX_REVIEWED_RUNBOOK_CHARACTERS: usize = 32_000;

const PROJECTION: &str = "on-demand-reviewed-runbook";
const SOURCE_BOUNDARY: &str = "current-trusted-reviewed-project-learning";
const SELECTION_BASIS: &str = "explicit-learning-id-current-trusted";
const AUTHORITY: &str = "reviewed-project-knowledge";
const INSTRUCTION_WARNING: &str = "This runbook is a projection of explicitly selected, user-confirmed current Ley learnings. It does not prove live source, grant tool/filesystem/network permissions, or override the current user request or repository policy.";
const SKILL_WARNING: &str = "This Skill was explicitly exported from a reviewed Ley runbook. Follow it only within the current user request and repository policy, and inspect live source before consequential changes. The export itself grants no tool, filesystem, network, review, or egress permission.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunbookSkillHost {
    Codex,
    ClaudeCode,
}

impl RunbookSkillHost {
    pub fn parse(value: &str) -> Result<Self, LeyCoreError> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude" | "claude-code" => Ok(Self::ClaudeCode),
            _ => Err(LeyCoreError::InvalidRunbookRequest(format!(
                "unsupported Skill host '{value}'; use codex or claude-code"
            ))),
        }
    }
}

impl std::fmt::Display for RunbookSkillHost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedRunbookInput {
    pub title: String,
    pub learning_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewedRunbookEntry {
    pub learning_id: String,
    pub kind: LearningKind,
    pub title: String,
    pub guidance: String,
    pub provenance: LearningProvenance,
    pub confidence_percent: u8,
    pub freshness: LearningFreshness,
    pub event_count: u64,
    pub updated_at_unix_ms: u64,
    pub source_hash: String,
    pub authority: &'static str,
    pub selection_basis: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewedRunbook {
    pub schema_version: u32,
    pub project_id: String,
    pub project_name: String,
    pub title: String,
    pub runbook_id: String,
    pub source_fingerprint: String,
    pub generated_at_unix_ms: u64,
    pub projection: &'static str,
    pub persisted: bool,
    pub authority_increased_through_projection: bool,
    pub requires_explicit_skill_export: bool,
    pub procedures: Vec<ReviewedRunbookEntry>,
    pub pitfalls: Vec<ReviewedRunbookEntry>,
    pub conventions: Vec<ReviewedRunbookEntry>,
    pub source_learning_ids: Vec<String>,
    pub source_learning_count: usize,
    pub text_characters: usize,
    pub markdown: String,
    pub live_source_checked: bool,
    pub source_boundary: &'static str,
    pub instruction_warning: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookSkillExport {
    pub schema_version: u32,
    pub host: RunbookSkillHost,
    pub egress_target: AgentEgressTarget,
    pub runbook_id: String,
    pub source_fingerprint: String,
    pub skill_name: String,
    pub file_name: &'static str,
    pub content: String,
    pub persisted: bool,
    pub installed: bool,
    pub explicit_user_action_required: bool,
    pub live_source_checked: bool,
    pub instruction_warning: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunbookSkillExportInput {
    pub runbook: ReviewedRunbookInput,
    pub expected_runbook_id: String,
    pub host: RunbookSkillHost,
    pub egress_target: AgentEgressTarget,
}

pub fn compile_reviewed_runbook(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    input: ReviewedRunbookInput,
) -> Result<ReviewedRunbook, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    let title = validate_title(&input.title)?;
    validate_learning_ids(&input.learning_ids)?;

    let mut procedures = Vec::new();
    let mut pitfalls = Vec::new();
    let mut conventions = Vec::new();
    let mut source_hashes = Vec::new();
    for learning_id in &input.learning_ids {
        let learning = read_learning(&diagnostic.root, &vault, learning_id)?;
        if learning.project_id != diagnostic.identity.project_id {
            return Err(LeyCoreError::InvalidRunbookRequest(format!(
                "learning {learning_id} belongs to another project"
            )));
        }
        if learning.state != LearningState::Verified
            || learning.trust_state != LearningTrustState::Trusted
            || learning.freshness != LearningFreshness::Current
        {
            return Err(LeyCoreError::InvalidRunbookRequest(format!(
                "learning {learning_id} is not verified, trusted, and current; review or refresh it before using it in a runbook"
            )));
        }
        if !matches!(
            learning.kind,
            LearningKind::Procedure | LearningKind::Pitfall | LearningKind::Convention
        ) {
            return Err(LeyCoreError::InvalidRunbookRequest(format!(
                "learning {learning_id} has kind {}; reviewed runbooks accept only procedure, pitfall, or convention learnings",
                learning_kind_label(learning.kind)
            )));
        }
        let source_hash = hash_json(&(
            learning.learning_id.as_str(),
            learning.kind,
            learning.title.as_str(),
            learning.guidance.as_str(),
            learning.provenance,
            learning.confidence_percent,
            learning.freshness,
            learning.event_count,
            learning.updated_at_unix_ms,
        ))?;
        source_hashes.push(source_hash.clone());
        let entry = ReviewedRunbookEntry {
            learning_id: learning.learning_id,
            kind: learning.kind,
            title: learning.title,
            guidance: learning.guidance,
            provenance: learning.provenance,
            confidence_percent: learning.confidence_percent,
            freshness: learning.freshness,
            event_count: learning.event_count,
            updated_at_unix_ms: learning.updated_at_unix_ms,
            source_hash,
            authority: AUTHORITY,
            selection_basis: SELECTION_BASIS,
        };
        match entry.kind {
            LearningKind::Procedure => procedures.push(entry),
            LearningKind::Pitfall => pitfalls.push(entry),
            LearningKind::Convention => conventions.push(entry),
            LearningKind::Constraint | LearningKind::Fact => unreachable!("validated above"),
        }
    }

    let source_fingerprint = hash_json(&source_hashes)?;
    let runbook_id = format!(
        "rbk_{}",
        hex_sha256(
            serde_json::to_vec(&(
                REVIEWED_RUNBOOK_SCHEMA_VERSION,
                diagnostic.identity.project_id.as_str(),
                title.as_str(),
                source_fingerprint.as_str(),
                input.learning_ids.as_slice(),
            ))
            .map_err(|error| LeyCoreError::InvalidRunbookRequest(format!(
                "could not fingerprint reviewed runbook: {error}"
            )))?
        )
    );
    let markdown = render_runbook_markdown(
        &title,
        &runbook_id,
        &source_fingerprint,
        &procedures,
        &pitfalls,
        &conventions,
    );
    let text_characters = markdown.chars().count();
    if text_characters > MAX_REVIEWED_RUNBOOK_CHARACTERS {
        return Err(LeyCoreError::InvalidRunbookRequest(format!(
            "reviewed runbook text is {text_characters} characters; reduce the selected learnings to stay at or below {MAX_REVIEWED_RUNBOOK_CHARACTERS}"
        )));
    }

    Ok(ReviewedRunbook {
        schema_version: REVIEWED_RUNBOOK_SCHEMA_VERSION,
        project_id: diagnostic.identity.project_id,
        project_name: diagnostic.identity.name,
        title,
        runbook_id,
        source_fingerprint,
        generated_at_unix_ms: unix_time_ms(),
        projection: PROJECTION,
        persisted: false,
        authority_increased_through_projection: false,
        requires_explicit_skill_export: true,
        procedures,
        pitfalls,
        conventions,
        source_learning_ids: input.learning_ids,
        source_learning_count: source_hashes.len(),
        text_characters,
        markdown,
        live_source_checked: false,
        source_boundary: SOURCE_BOUNDARY,
        instruction_warning: INSTRUCTION_WARNING,
    })
}

pub fn export_reviewed_runbook_skill(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    input: RunbookSkillExportInput,
    egress_registry: &EgressPolicyRegistry,
    context_mount_registry: &ContextMountRegistry,
) -> Result<RunbookSkillExport, LeyCoreError> {
    validate_runbook_id(&input.expected_runbook_id)?;
    enforce_historical_egress(
        project_start.as_ref(),
        input.egress_target,
        egress_registry,
        context_mount_registry,
    )?;
    let runbook = compile_reviewed_runbook(&project_start, vault, input.runbook)?;
    if runbook.runbook_id != input.expected_runbook_id {
        return Err(LeyCoreError::InvalidRunbookRequest(format!(
            "reviewed runbook changed: expected {}, current {}; compile and review the current runbook before exporting it",
            input.expected_runbook_id,
            runbook.runbook_id
        )));
    }
    let skill_name = skill_name(&runbook.title);
    let content = render_skill(&runbook, &skill_name);
    Ok(RunbookSkillExport {
        schema_version: REVIEWED_RUNBOOK_SCHEMA_VERSION,
        host: input.host,
        egress_target: input.egress_target,
        runbook_id: runbook.runbook_id,
        source_fingerprint: runbook.source_fingerprint,
        skill_name,
        file_name: "SKILL.md",
        content,
        persisted: false,
        installed: false,
        explicit_user_action_required: true,
        live_source_checked: false,
        instruction_warning: SKILL_WARNING,
    })
}

fn enforce_historical_egress(
    project_start: &Path,
    target: AgentEgressTarget,
    egress_registry: &EgressPolicyRegistry,
    context_mount_registry: &ContextMountRegistry,
) -> Result<(), LeyCoreError> {
    let project_id = diagnose_project(project_start)?.identity.project_id;
    egress_registry.with_snapshot_locked(|policies| {
        let project_decision = evaluate_agent_egress(policies.project_policy(&project_id), target);
        if !project_decision.allowed {
            return Err(LeyCoreError::AgentEgressDenied {
                policy: project_decision.policy.to_string(),
                target: target.to_string(),
            });
        }
        if policies.has_blocked_fine_grained_source(&project_id, target) {
            return Err(LeyCoreError::AgentDerivedEgressUnproven {
                target: target.to_string(),
            });
        }
        context_mount_registry.with_agent_context_sources_locked(project_start, |sources| {
            let source_blocked = sources.historical.iter().any(|source| {
                !evaluate_agent_egress(policies.project_policy(&source.source_project_id), target)
                    .allowed
            });
            if source_blocked {
                return Err(LeyCoreError::AgentDerivedEgressUnproven {
                    target: target.to_string(),
                });
            }
            Ok(())
        })
    })
}

fn validate_title(value: &str) -> Result<String, LeyCoreError> {
    let value = value.trim();
    let characters = value.chars().count();
    if characters == 0 || characters > 128 || value.chars().any(|character| character.is_control())
    {
        return Err(LeyCoreError::InvalidRunbookRequest(
            "runbook title must be between 1 and 128 visible characters".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn validate_learning_ids(values: &[String]) -> Result<(), LeyCoreError> {
    if values.is_empty() || values.len() > MAX_REVIEWED_RUNBOOK_SOURCES {
        return Err(LeyCoreError::InvalidRunbookRequest(format!(
            "reviewed runbook must select between 1 and {MAX_REVIEWED_RUNBOOK_SOURCES} learnings"
        )));
    }
    let mut unique = HashSet::new();
    for value in values {
        if !value.starts_with("lrn_") || !unique.insert(value.as_str()) {
            return Err(LeyCoreError::InvalidRunbookRequest(
                "runbook learning IDs must be unique lrn_ identifiers".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_runbook_id(value: &str) -> Result<(), LeyCoreError> {
    let Some(digest) = value.strip_prefix("rbk_") else {
        return Err(LeyCoreError::InvalidRunbookRequest(
            "expected runbook ID must use rbk_<64 lowercase hex>".to_owned(),
        ));
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(LeyCoreError::InvalidRunbookRequest(
            "expected runbook ID must use rbk_<64 lowercase hex>".to_owned(),
        ));
    }
    Ok(())
}

fn render_runbook_markdown(
    title: &str,
    runbook_id: &str,
    source_fingerprint: &str,
    procedures: &[ReviewedRunbookEntry],
    pitfalls: &[ReviewedRunbookEntry],
    conventions: &[ReviewedRunbookEntry],
) -> String {
    let mut output = format!(
        "# {title}\n\n> Reviewed Ley runbook. This is a source-bound projection of explicitly selected current trusted learnings, not a live-source check or a grant of tool permissions.\n\n"
    );
    append_section(&mut output, "Procedures", procedures);
    append_section(&mut output, "Pitfalls", pitfalls);
    append_section(&mut output, "Conventions", conventions);
    output.push_str("## Ley source binding\n\n");
    output.push_str(&format!("- Runbook ID: `{runbook_id}`\n"));
    output.push_str(&format!("- Source fingerprint: `{source_fingerprint}`\n"));
    for entry in procedures.iter().chain(pitfalls).chain(conventions) {
        output.push_str(&format!(
            "- `{}` — {} (event {})\n",
            entry.learning_id,
            learning_kind_label(entry.kind),
            entry.event_count
        ));
    }
    output.push_str("\nLive source was not checked. Inspect the current workspace before consequential changes.\n");
    output
}

fn append_section(output: &mut String, heading: &str, entries: &[ReviewedRunbookEntry]) {
    if entries.is_empty() {
        return;
    }
    output.push_str(&format!("## {heading}\n\n"));
    for entry in entries {
        output.push_str(&format!("### {}\n\n{}\n\n", entry.title, entry.guidance));
    }
}

fn render_skill(runbook: &ReviewedRunbook, skill_name: &str) -> String {
    let description = format!(
        "Explicitly exported reviewed Ley runbook for {}. Verify live source before consequential work.",
        runbook.title
    );
    let description =
        serde_json::to_string(&description).expect("Skill description is serializable");
    format!(
        "---\nname: {skill_name}\ndescription: {description}\n---\n\n{}\n\n{}",
        SKILL_WARNING, runbook.markdown
    )
}

fn skill_name(title: &str) -> String {
    let mut output = String::new();
    let mut dash = false;
    for character in title.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if dash && !output.is_empty() {
                output.push('-');
            }
            output.push(character);
            dash = false;
        } else if !output.is_empty() {
            dash = true;
        }
        if output.len() >= 56 {
            break;
        }
    }
    let output = output.trim_matches('-');
    if output.is_empty() {
        "ley-reviewed-runbook".to_owned()
    } else {
        format!("ley-{output}")
    }
}

fn learning_kind_label(kind: LearningKind) -> &'static str {
    match kind {
        LearningKind::Procedure => "procedure",
        LearningKind::Pitfall => "pitfall",
        LearningKind::Convention => "convention",
        LearningKind::Constraint => "constraint",
        LearningKind::Fact => "fact",
    }
}

fn hash_json(value: &impl Serialize) -> Result<String, LeyCoreError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        LeyCoreError::InvalidRunbookRequest(format!(
            "could not fingerprint runbook source: {error}"
        ))
    })?;
    Ok(format!("sha256:{}", hex_sha256(bytes)))
}

fn hex_sha256(bytes: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(bytes.as_ref());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        checkpoint_session, ingest_project, initialize_project, propose_learning, review_learning,
        start_session, AgentEgressPolicy, CaptureMode, CheckpointInput, LearningActor,
        LearningEvidenceInput, LearningFeedbackAction, ProposeLearningInput, ReviewLearningInput,
        SessionSource, StartSessionInput,
    };
    use tempfile::tempdir;

    fn request_id(seed: &str) -> String {
        format!("req_{}", &hex_sha256(seed.as_bytes())[..32])
    }

    fn confirm_learning(
        project: &Path,
        vault: &Path,
        evidence: (&str, &str),
        kind: LearningKind,
        title: &str,
        guidance: &str,
        seed: &str,
    ) -> String {
        let (session_id, checkpoint_id) = evidence;
        let proposed = propose_learning(
            project,
            vault,
            ProposeLearningInput {
                request_id: request_id(&format!("{seed}:propose")),
                actor: LearningActor::Agent,
                kind,
                title: title.to_owned(),
                guidance: guidance.to_owned(),
                confidence_percent: 90,
                provenance: LearningProvenance::AgentAuthored,
                evidence: vec![LearningEvidenceInput {
                    session_id: session_id.to_owned(),
                    record_id: checkpoint_id.to_owned(),
                    note: "private evidence note that must not enter a runbook".to_owned(),
                }],
            },
        )
        .unwrap();
        review_learning(
            project,
            vault,
            &proposed.learning.learning_id,
            ReviewLearningInput {
                request_id: request_id(&format!("{seed}:confirm")),
                expected_event_count: None,
                actor: LearningActor::User,
                action: LearningFeedbackAction::Confirm,
                note: "Explicitly reviewed for reuse.".to_owned(),
                replacement_learning_id: None,
            },
        )
        .unwrap();
        proposed.learning.learning_id
    }

    #[test]
    fn runbook_requires_current_trusted_operational_learnings_and_is_source_bound() {
        let temporary = tempdir().unwrap();
        let project = temporary.path().join("project");
        let vault = temporary.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Runbook project"), CaptureMode::Structured).unwrap();
        std::fs::write(
            project.join("README.md"),
            "# Release\n\nUse verified steps.\n",
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let session = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id("session"),
                name: "Verify release".to_owned(),
                goal: "Create reviewed operational knowledge".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &session.session.session_id,
            CheckpointInput {
                request_id: request_id("checkpoint"),
                summary: "Verified release workflow".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: Vec::new(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let checkpoint_id = checkpoint.session.checkpoints[0].id.clone();
        let procedure = confirm_learning(
            &project,
            &vault,
            (&session.session.session_id, &checkpoint_id),
            LearningKind::Procedure,
            "Release safely",
            "Run the verified release checks before publishing.",
            "procedure",
        );
        let pitfall = confirm_learning(
            &project,
            &vault,
            (&session.session.session_id, &checkpoint_id),
            LearningKind::Pitfall,
            "Avoid stale artifacts",
            "Do not publish from an old build directory.",
            "pitfall",
        );
        let convention = confirm_learning(
            &project,
            &vault,
            (&session.session.session_id, &checkpoint_id),
            LearningKind::Convention,
            "Use release tags",
            "Name release tags with the project release convention.",
            "convention",
        );
        let fact = confirm_learning(
            &project,
            &vault,
            (&session.session.session_id, &checkpoint_id),
            LearningKind::Fact,
            "Release fact",
            "The project has a release workflow.",
            "fact",
        );

        let input = ReviewedRunbookInput {
            title: "Release workflow".to_owned(),
            learning_ids: vec![procedure.clone(), pitfall.clone(), convention.clone()],
        };
        let runbook = compile_reviewed_runbook(&project, &vault, input.clone()).unwrap();
        let rebuilt = compile_reviewed_runbook(&project, &vault, input.clone()).unwrap();
        assert_eq!(runbook.schema_version, REVIEWED_RUNBOOK_SCHEMA_VERSION);
        assert_eq!(runbook.projection, PROJECTION);
        assert!(!runbook.persisted);
        assert!(!runbook.authority_increased_through_projection);
        assert!(runbook.requires_explicit_skill_export);
        assert!(!runbook.live_source_checked);
        assert!(runbook.runbook_id.starts_with("rbk_"));
        assert_eq!(runbook.runbook_id, rebuilt.runbook_id);
        assert_eq!(runbook.source_fingerprint, rebuilt.source_fingerprint);
        assert_eq!(runbook.procedures.len(), 1);
        assert_eq!(runbook.pitfalls.len(), 1);
        assert_eq!(runbook.conventions.len(), 1);
        assert!(runbook.markdown.contains("Run the verified release checks"));
        assert!(runbook
            .markdown
            .contains("Do not publish from an old build"));
        assert!(runbook.markdown.contains("Name release tags"));
        assert!(!runbook.markdown.contains("private evidence note"));
        assert!(!runbook
            .markdown
            .contains(project.to_string_lossy().as_ref()));
        assert!(!runbook.markdown.contains(vault.to_string_lossy().as_ref()));

        let unsupported = compile_reviewed_runbook(
            &project,
            &vault,
            ReviewedRunbookInput {
                title: "Bad runbook".to_owned(),
                learning_ids: vec![fact],
            },
        )
        .unwrap_err();
        assert!(unsupported
            .to_string()
            .contains("procedure, pitfall, or convention"));

        std::fs::write(
            project.join("README.md"),
            "# Release\n\nWorkflow changed.\n",
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let stale = compile_reviewed_runbook(&project, &vault, input).unwrap_err();
        assert!(stale
            .to_string()
            .contains("not verified, trusted, and current"));
    }

    #[test]
    fn skill_export_requires_exact_reviewed_runbook_and_historical_egress() {
        let temporary = tempdir().unwrap();
        let project = temporary.path().join("project");
        let vault = temporary.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(&project, Some("Skill export"), CaptureMode::Structured).unwrap();
        std::fs::write(
            project.join("README.md"),
            "# Test\n\nRun the verified suite.\n",
        )
        .unwrap();
        ingest_project(&project, &vault).unwrap();
        let session = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id("export-session"),
                name: "Verify tests".to_owned(),
                goal: "Prepare a reviewed runbook".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let checkpoint = checkpoint_session(
            &project,
            &vault,
            &session.session.session_id,
            CheckpointInput {
                request_id: request_id("export-checkpoint"),
                summary: "Verified tests".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: Vec::new(),
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let procedure = confirm_learning(
            &project,
            &vault,
            (
                &session.session.session_id,
                &checkpoint.session.checkpoints[0].id,
            ),
            LearningKind::Procedure,
            "Run verified tests",
            "Run the verified suite before delivery.",
            "export-procedure",
        );
        let input = ReviewedRunbookInput {
            title: "Delivery checks".to_owned(),
            learning_ids: vec![procedure],
        };
        let runbook = compile_reviewed_runbook(&project, &vault, input.clone()).unwrap();
        let config = temporary.path().join("config");
        std::fs::create_dir(&config).unwrap();
        let egress = EgressPolicyRegistry::at(config.join("agent-egress-v1.json"));
        let mounts = ContextMountRegistry::at(config.join("context-mounts-v1.json"));
        egress
            .set_project_policy(&project, AgentEgressPolicy::LocalModelOnly)
            .unwrap();

        let cloud = export_reviewed_runbook_skill(
            &project,
            &vault,
            RunbookSkillExportInput {
                runbook: input.clone(),
                expected_runbook_id: runbook.runbook_id.clone(),
                host: RunbookSkillHost::Codex,
                egress_target: AgentEgressTarget::Cloud,
            },
            &egress,
            &mounts,
        )
        .unwrap_err();
        assert!(matches!(cloud, LeyCoreError::AgentEgressDenied { .. }));

        let wrong = export_reviewed_runbook_skill(
            &project,
            &vault,
            RunbookSkillExportInput {
                runbook: input.clone(),
                expected_runbook_id: format!("rbk_{}", "0".repeat(64)),
                host: RunbookSkillHost::Codex,
                egress_target: AgentEgressTarget::Local,
            },
            &egress,
            &mounts,
        )
        .unwrap_err();
        assert!(wrong.to_string().contains("reviewed runbook changed"));

        let exported = export_reviewed_runbook_skill(
            &project,
            &vault,
            RunbookSkillExportInput {
                runbook: input,
                expected_runbook_id: runbook.runbook_id.clone(),
                host: RunbookSkillHost::ClaudeCode,
                egress_target: AgentEgressTarget::Local,
            },
            &egress,
            &mounts,
        )
        .unwrap();
        assert_eq!(exported.host, RunbookSkillHost::ClaudeCode);
        assert_eq!(exported.egress_target, AgentEgressTarget::Local);
        assert_eq!(exported.file_name, "SKILL.md");
        assert!(!exported.persisted);
        assert!(!exported.installed);
        assert!(exported.explicit_user_action_required);
        assert!(!exported.live_source_checked);
        assert!(exported.content.contains("name: ley-delivery-checks"));
        assert!(exported
            .content
            .contains("Run the verified suite before delivery."));
        assert!(exported.content.contains(&runbook.runbook_id));
        assert!(!exported
            .content
            .contains(project.to_string_lossy().as_ref()));
        assert!(!exported.content.contains(vault.to_string_lossy().as_ref()));
    }
}
