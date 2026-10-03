use crate::continuity_store::{
    ContinuityApprovedSourceIssueInput, ContinuityApprovedSourceIssueReason,
    ContinuityApprovedSourceSnapshotInput, ContinuityEventLinkInput, ContinuityProjectObservation,
    ContinuityProjectOrigin,
};
use crate::egress_policy::conservative_egress_join;
use crate::specification::LegacySpecificationMigrationState;
use crate::{
    diagnose_project, AgentEgressPolicy, ContinuityEventInput, ContinuityStore,
    EgressPolicyRegistry, LeyCoreError, ProjectCatalog, SpecificationRegistry,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyContinuityImportSummary {
    pub project_id: String,
    pub session_events: usize,
    pub learning_events: usize,
    pub source_digest: String,
    pub manifest_event_id: String,
    pub project_created: bool,
    pub continuity_events_created: usize,
    pub continuity_events_replayed: usize,
    pub continuity_events_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LegacySessionImportSummary {
    pub project_id: String,
    pub session_events: usize,
    pub source_digest: String,
    pub manifest_event_id: String,
    pub manifest_recorded_at_unix_ms: u64,
    pub continuity_events_created: usize,
    pub continuity_events_replayed: usize,
    pub continuity_events_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LegacyLearningImportSummary {
    pub project_id: String,
    pub learning_events: usize,
    pub source_digest: String,
    pub manifest_event_id: String,
    pub manifest_recorded_at_unix_ms: u64,
    pub continuity_events_created: usize,
    pub continuity_events_replayed: usize,
    pub continuity_events_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyProjectEgressImportSummary {
    pub project_id: String,
    pub source_project_policy: AgentEgressPolicy,
    pub migrated_project_policy: AgentEgressPolicy,
    pub specification_overrides_remaining_legacy: usize,
    pub mount_overrides_remaining_legacy: usize,
    pub connector_overrides_remaining_legacy: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyProjectCatalogImportSummary {
    pub project_observations: usize,
    pub catalog_migrated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyApprovedSourceImportSummary {
    pub project_id: String,
    pub imported_snapshots: usize,
    pub changed_approvals: usize,
    pub missing_approvals: usize,
    pub invalid_approvals: usize,
    pub authority_migrated: bool,
}

pub fn import_legacy_continuity(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<LegacyContinuityImportSummary, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    let session_events = crate::session::continuity_events_for_migration(&diagnostic.root, &vault)?;
    let learning_events =
        crate::learning::continuity_events_for_migration(&diagnostic.root, &vault)?;
    let session_event_count = session_events.len();
    let learning_event_count = learning_events.len();

    let mut events = session_events;
    events.extend(learning_events);
    events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    let links = legacy_event_links(&events)?;
    let source_digest = event_inventory_digest(&events)?;
    let digest_hex = source_digest
        .strip_prefix("sha256:")
        .expect("inventory digests always use sha256");
    let manifest_event_id = format!("mig_{digest_hex}");
    let manifest_timestamp = events
        .iter()
        .map(|event| event.recorded_at_unix_ms)
        .max()
        .unwrap_or(diagnostic.identity.created_at_unix_ms)
        .max(1);
    events.push(ContinuityEventInput {
        event_id: manifest_event_id.clone(),
        project_id: diagnostic.identity.project_id.clone(),
        subject_id: None,
        session_id: None,
        session_sequence: None,
        request_id: None,
        request_fingerprint: None,
        kind: "legacy-snapshot-imported".to_owned(),
        payload_version: LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION,
        recorded_at_unix_ms: manifest_timestamp,
        revision_head: None,
        revision_branch: None,
        payload: json!({
            "source": "legacy-agent-memory",
            "formatVersion": LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION,
            "eventDigest": source_digest,
            "sessionEventCount": session_event_count,
            "learningEventCount": learning_event_count,
            "cutover": false
        }),
    });

    let imported = store.sync_legacy_project_events(&diagnostic.identity, &events, &links)?;
    Ok(LegacyContinuityImportSummary {
        project_id: diagnostic.identity.project_id,
        session_events: session_event_count,
        learning_events: learning_event_count,
        source_digest,
        manifest_event_id,
        project_created: imported.project_created,
        continuity_events_created: imported.events_created,
        continuity_events_replayed: imported.events_replayed,
        continuity_events_removed: imported.events_removed,
    })
}

pub(crate) fn import_legacy_session_continuity(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<LegacySessionImportSummary, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    let mut events = crate::session::continuity_events_for_migration(&diagnostic.root, &vault)?;
    events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    let source_digest = event_inventory_digest(&events)?;
    let digest_hex = source_digest
        .strip_prefix("sha256:")
        .expect("inventory digests always use sha256");
    let manifest_event_id = format!("mig_session_{digest_hex}");
    let manifest_recorded_at_unix_ms = events
        .iter()
        .map(|event| event.recorded_at_unix_ms)
        .max()
        .unwrap_or(diagnostic.identity.created_at_unix_ms)
        .max(1);
    let manifest = ContinuityEventInput {
        event_id: manifest_event_id.clone(),
        project_id: diagnostic.identity.project_id.clone(),
        subject_id: None,
        session_id: None,
        session_sequence: None,
        request_id: None,
        request_fingerprint: None,
        kind: "legacy-session-snapshot-imported".to_owned(),
        payload_version: LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION,
        recorded_at_unix_ms: manifest_recorded_at_unix_ms,
        revision_head: None,
        revision_branch: None,
        payload: json!({
            "source": "legacy-session-memory",
            "formatVersion": LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION,
            "eventDigest": source_digest,
            "sessionEventCount": events.len(),
            "cutover": false
        }),
    };
    let imported = store.sync_legacy_session_events(&diagnostic.identity, &events, &manifest)?;
    Ok(LegacySessionImportSummary {
        project_id: diagnostic.identity.project_id,
        session_events: events.len(),
        source_digest,
        manifest_event_id,
        manifest_recorded_at_unix_ms,
        continuity_events_created: imported.events_created,
        continuity_events_replayed: imported.events_replayed,
        continuity_events_removed: imported.events_removed,
    })
}

pub(crate) fn import_legacy_learning_continuity(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    store: &ContinuityStore,
) -> Result<LegacyLearningImportSummary, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    let mut events = crate::learning::continuity_events_for_migration(&diagnostic.root, &vault)?;
    events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    let links = legacy_learning_event_links(store, &diagnostic.identity.project_id, &events)?;
    let source_digest = event_inventory_digest(&events)?;
    let digest_hex = source_digest
        .strip_prefix("sha256:")
        .expect("inventory digests always use sha256");
    let manifest_event_id = format!("mig_learning_{digest_hex}");
    let manifest_recorded_at_unix_ms = events
        .iter()
        .map(|event| event.recorded_at_unix_ms)
        .max()
        .unwrap_or(diagnostic.identity.created_at_unix_ms)
        .max(1);
    let manifest = ContinuityEventInput {
        event_id: manifest_event_id.clone(),
        project_id: diagnostic.identity.project_id.clone(),
        subject_id: None,
        session_id: None,
        session_sequence: None,
        request_id: None,
        request_fingerprint: None,
        kind: "legacy-learning-snapshot-imported".to_owned(),
        payload_version: LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION,
        recorded_at_unix_ms: manifest_recorded_at_unix_ms,
        revision_head: None,
        revision_branch: None,
        payload: json!({
            "source": "legacy-learning-memory",
            "formatVersion": LEGACY_CONTINUITY_IMPORT_FORMAT_VERSION,
            "eventDigest": source_digest,
            "learningEventCount": events.len(),
            "cutover": false
        }),
    };
    let imported =
        store.sync_legacy_learning_events(&diagnostic.identity, &events, &links, &manifest)?;
    Ok(LegacyLearningImportSummary {
        project_id: diagnostic.identity.project_id,
        learning_events: events.len(),
        source_digest,
        manifest_event_id,
        manifest_recorded_at_unix_ms,
        continuity_events_created: imported.events_created,
        continuity_events_replayed: imported.events_replayed,
        continuity_events_removed: imported.events_removed,
    })
}

fn legacy_learning_event_links(
    store: &ContinuityStore,
    project_id: &str,
    events: &[ContinuityEventInput],
) -> Result<Vec<ContinuityEventLinkInput>, LeyCoreError> {
    let mut learning_roots = BTreeMap::<String, (u64, String)>::new();
    for event in events {
        let learning_id = event.subject_id.as_ref().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(format!(
                "legacy learning event {} is missing learning ID",
                event.event_id
            ))
        })?;
        let sequence = event.payload["sequence"].as_u64().ok_or_else(|| {
            LeyCoreError::InvalidContinuityStore(format!(
                "legacy learning event {} is missing sequence",
                event.event_id
            ))
        })?;
        match learning_roots.get(learning_id) {
            Some((current, _)) if *current <= sequence => {}
            _ => {
                learning_roots.insert(learning_id.clone(), (sequence, event.event_id.clone()));
            }
        }
    }

    let mut session_anchors = BTreeMap::<String, String>::new();
    let cited_sessions = events
        .iter()
        .filter(|event| {
            matches!(
                event.kind.as_str(),
                "legacy-learning-proposed" | "legacy-learning-corrected"
            )
        })
        .flat_map(|event| {
            event.payload["data"]["evidence"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["sessionId"].as_str().map(str::to_owned))
        })
        .collect::<BTreeSet<_>>();
    for session_id in cited_sessions {
        let anchor = store
            .events_for_session(project_id, &session_id)?
            .into_iter()
            .find(|event| {
                matches!(
                    event.kind.as_str(),
                    "legacy-session-started" | "session-started"
                )
            })
            .ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "legacy learning snapshot cites missing session {session_id}"
                ))
            })?;
        session_anchors.insert(session_id, anchor.event_id);
    }

    let mut links = BTreeSet::<(String, String, String)>::new();
    for event in events {
        match event.kind.as_str() {
            "legacy-learning-proposed" | "legacy-learning-corrected" => {
                let evidence = event.payload["data"]["evidence"]
                    .as_array()
                    .ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy learning event {} is missing evidence",
                            event.event_id
                        ))
                    })?;
                for item in evidence {
                    let session_id = item["sessionId"].as_str().ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy learning event {} has invalid session evidence",
                            event.event_id
                        ))
                    })?;
                    let target = session_anchors.get(session_id).ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy learning event {} cites missing session {session_id}",
                            event.event_id
                        ))
                    })?;
                    links.insert((
                        event.event_id.clone(),
                        target.clone(),
                        "depends-on-session".to_owned(),
                    ));
                }
            }
            "legacy-learning-reviewed"
                if event.payload["data"]["action"].as_str() == Some("supersede") =>
            {
                let replacement = event.payload["data"]["replacementLearningId"]
                    .as_str()
                    .ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy supersession event {} is missing replacement learning",
                            event.event_id
                        ))
                    })?;
                let (_, target) = learning_roots.get(replacement).ok_or_else(|| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "legacy supersession event {} references missing learning {replacement}",
                        event.event_id
                    ))
                })?;
                links.insert((
                    event.event_id.clone(),
                    target.clone(),
                    "supersedes".to_owned(),
                ));
            }
            _ => {}
        }
    }
    Ok(links
        .into_iter()
        .map(
            |(from_event_id, to_event_id, relation)| ContinuityEventLinkInput {
                from_event_id,
                to_event_id,
                relation,
            },
        )
        .collect())
}

pub fn import_legacy_project_egress(
    project_start: impl AsRef<Path>,
    registry: &EgressPolicyRegistry,
    store: &ContinuityStore,
) -> Result<LegacyProjectEgressImportSummary, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    registry.with_snapshot_locked(|snapshot| {
        let project_id = diagnostic.identity.project_id.clone();
        let source_project_policy = snapshot.project_policy(&project_id);
        let fine_grained = snapshot.fine_grained_policies(&project_id);
        let migrated_project_policy = conservative_legacy_egress_policy(
            source_project_policy,
            fine_grained.iter().map(|scope| scope.policy),
        );
        store.register_project(&diagnostic.identity)?;
        store.import_legacy_project_egress_policy(&project_id, migrated_project_policy)?;
        Ok(LegacyProjectEgressImportSummary {
            project_id,
            source_project_policy,
            migrated_project_policy,
            specification_overrides_remaining_legacy: fine_grained
                .iter()
                .filter(|scope| {
                    matches!(scope.scope_kind, crate::AgentEgressScopeKind::Specification)
                })
                .count(),
            mount_overrides_remaining_legacy: fine_grained
                .iter()
                .filter(|scope| {
                    matches!(scope.scope_kind, crate::AgentEgressScopeKind::ContextMount)
                })
                .count(),
            connector_overrides_remaining_legacy: fine_grained
                .iter()
                .filter(|scope| {
                    matches!(
                        scope.scope_kind,
                        crate::AgentEgressScopeKind::ExternalConnector
                    )
                })
                .count(),
        })
    })
}

fn conservative_legacy_egress_policy(
    project_policy: AgentEgressPolicy,
    fine_grained: impl IntoIterator<Item = AgentEgressPolicy>,
) -> AgentEgressPolicy {
    fine_grained
        .into_iter()
        .fold(project_policy, conservative_egress_join)
}

pub fn import_legacy_project_catalog(
    catalog: &ProjectCatalog,
    store: &ContinuityStore,
) -> Result<LegacyProjectCatalogImportSummary, LeyCoreError> {
    let observations = catalog
        .all_for_migration()?
        .into_iter()
        .map(|observation| ContinuityProjectObservation {
            project_id: observation.project_id,
            root_path: observation.root_path,
            last_opened_at_unix_ms: observation.last_opened_at_unix_ms,
            continuity_origin: ContinuityProjectOrigin::LegacyUnknown,
        })
        .collect::<Vec<_>>();
    let catalog_migrated = store.import_legacy_project_observations_once(&observations)?;
    Ok(LegacyProjectCatalogImportSummary {
        project_observations: observations.len(),
        catalog_migrated,
    })
}

pub fn import_legacy_approved_sources(
    project_start: impl AsRef<Path>,
    vault: impl AsRef<Path>,
    registry: &SpecificationRegistry,
    store: &ContinuityStore,
) -> Result<LegacyApprovedSourceImportSummary, LeyCoreError> {
    let project_start = project_start.as_ref();
    let vault = vault.as_ref();
    store.with_approved_source_authority_lock(|| {
        import_legacy_approved_sources_with_native_authority_held(
            project_start,
            vault,
            registry,
            store,
            None,
        )
    })
}

pub(crate) fn import_legacy_approved_sources_for_expected_project(
    project_start: &Path,
    vault: &Path,
    registry: &SpecificationRegistry,
    store: &ContinuityStore,
    expected_project_id: &str,
) -> Result<LegacyApprovedSourceImportSummary, LeyCoreError> {
    store.with_approved_source_authority_lock(|| {
        import_legacy_approved_sources_with_native_authority_held(
            project_start,
            vault,
            registry,
            store,
            Some(expected_project_id),
        )
    })
}

pub(crate) fn import_legacy_approved_sources_with_native_authority_held(
    project_start: &Path,
    vault: &Path,
    registry: &SpecificationRegistry,
    store: &ContinuityStore,
    expected_project_id: Option<&str>,
) -> Result<LegacyApprovedSourceImportSummary, LeyCoreError> {
    let diagnostic = diagnose_project(&project_start)?;
    if expected_project_id.is_some_and(|expected| diagnostic.identity.project_id != expected) {
        return Err(LeyCoreError::InvalidProjectIdentity(
            "source project identity changed during approved-source migration".to_owned(),
        ));
    }
    let project_id = diagnostic.identity.project_id.clone();
    store.register_project(&diagnostic.identity)?;
    if store.approved_source_authority_ready(&project_id)? {
        return Ok(LegacyApprovedSourceImportSummary {
            project_id,
            imported_snapshots: 0,
            changed_approvals: 0,
            missing_approvals: 0,
            invalid_approvals: 0,
            authority_migrated: false,
        });
    }
    registry.with_legacy_approvals_for_migration_locked(&diagnostic.root, vault, |approvals| {
        let mut snapshots = Vec::new();
        let mut issues = Vec::new();
        let mut changed_approvals = 0usize;
        let mut missing_approvals = 0usize;
        let mut invalid_approvals = 0usize;

        for approval in approvals {
            let display_name = approval.relative_path.clone();
            match approval.state {
                LegacySpecificationMigrationState::Current(content_bytes) => {
                    snapshots.push(ContinuityApprovedSourceSnapshotInput {
                        source_id: approval.specification_id,
                        display_name,
                        content_hash: approval.content_hash,
                        approved_at_unix_ms: approval.approved_at_unix_ms,
                        content_bytes,
                    });
                }
                LegacySpecificationMigrationState::Changed => {
                    changed_approvals += 1;
                    issues.push(ContinuityApprovedSourceIssueInput {
                        source_id: approval.specification_id,
                        display_name,
                        approved_content_hash: approval.content_hash,
                        approved_at_unix_ms: approval.approved_at_unix_ms,
                        reason: ContinuityApprovedSourceIssueReason::Changed,
                    });
                }
                LegacySpecificationMigrationState::Missing => {
                    missing_approvals += 1;
                    issues.push(ContinuityApprovedSourceIssueInput {
                        source_id: approval.specification_id,
                        display_name,
                        approved_content_hash: approval.content_hash,
                        approved_at_unix_ms: approval.approved_at_unix_ms,
                        reason: ContinuityApprovedSourceIssueReason::Missing,
                    });
                }
                LegacySpecificationMigrationState::Invalid => {
                    invalid_approvals += 1;
                    issues.push(ContinuityApprovedSourceIssueInput {
                        source_id: approval.specification_id,
                        display_name,
                        approved_content_hash: approval.content_hash,
                        approved_at_unix_ms: approval.approved_at_unix_ms,
                        reason: ContinuityApprovedSourceIssueReason::Invalid,
                    });
                }
            }
        }

        let authority_migrated = store.import_legacy_approved_source_authority_unlocked(
            &project_id,
            &snapshots,
            &issues,
        )?;
        Ok(LegacyApprovedSourceImportSummary {
            project_id: project_id.clone(),
            imported_snapshots: snapshots.len(),
            changed_approvals,
            missing_approvals,
            invalid_approvals,
            authority_migrated,
        })
    })
}

fn event_inventory_digest(events: &[ContinuityEventInput]) -> Result<String, LeyCoreError> {
    let bytes = serde_json::to_vec(events).map_err(|error| {
        LeyCoreError::InvalidContinuityStore(format!(
            "legacy continuity inventory could not be serialized: {error}"
        ))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn legacy_event_links(
    events: &[ContinuityEventInput],
) -> Result<Vec<ContinuityEventLinkInput>, LeyCoreError> {
    let mut session_anchors = BTreeMap::<String, String>::new();
    let mut learning_roots = BTreeMap::<String, (u64, String)>::new();
    for event in events {
        if event.kind == "legacy-session-started" {
            let session_id = event.session_id.as_ref().ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(
                    "legacy session start event is missing session ID".to_owned(),
                )
            })?;
            if session_anchors
                .insert(session_id.clone(), event.event_id.clone())
                .is_some()
            {
                return Err(LeyCoreError::InvalidContinuityStore(format!(
                    "legacy session {session_id} has multiple start events"
                )));
            }
        }
        if let Some(subject_id) = &event.subject_id {
            let sequence = event.payload["sequence"].as_u64().ok_or_else(|| {
                LeyCoreError::InvalidContinuityStore(format!(
                    "legacy learning event {} is missing sequence",
                    event.event_id
                ))
            })?;
            match learning_roots.get(subject_id) {
                Some((current, _)) if *current <= sequence => {}
                _ => {
                    learning_roots.insert(subject_id.clone(), (sequence, event.event_id.clone()));
                }
            }
        }
    }

    let mut links = BTreeSet::<(String, String, String)>::new();
    for event in events {
        match event.kind.as_str() {
            "legacy-learning-proposed" | "legacy-learning-corrected" => {
                let evidence = event.payload["data"]["evidence"]
                    .as_array()
                    .ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy learning event {} is missing evidence",
                            event.event_id
                        ))
                    })?;
                for item in evidence {
                    let session_id = item["sessionId"].as_str().ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy learning event {} has invalid session evidence",
                            event.event_id
                        ))
                    })?;
                    let target = session_anchors.get(session_id).ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy learning event {} cites missing session {session_id}",
                            event.event_id
                        ))
                    })?;
                    links.insert((
                        event.event_id.clone(),
                        target.clone(),
                        "depends-on-session".to_owned(),
                    ));
                }
            }
            "legacy-learning-reviewed"
                if event.payload["data"]["action"].as_str() == Some("supersede") =>
            {
                let replacement = event.payload["data"]["replacementLearningId"]
                    .as_str()
                    .ok_or_else(|| {
                        LeyCoreError::InvalidContinuityStore(format!(
                            "legacy supersession event {} is missing replacement learning",
                            event.event_id
                        ))
                    })?;
                let (_, target) = learning_roots.get(replacement).ok_or_else(|| {
                    LeyCoreError::InvalidContinuityStore(format!(
                        "legacy supersession event {} references missing learning {replacement}",
                        event.event_id
                    ))
                })?;
                links.insert((
                    event.event_id.clone(),
                    target.clone(),
                    "supersedes".to_owned(),
                ));
            }
            _ => {}
        }
    }

    Ok(links
        .into_iter()
        .map(
            |(from_event_id, to_event_id, relation)| ContinuityEventLinkInput {
                from_event_id,
                to_event_id,
                relation,
            },
        )
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        checkpoint_session, erase_session_memory, generate_specification_id, ingest_project,
        initialize_project, propose_learning, read_learning, read_session, start_session,
        CaptureMode, CheckpointInput, EraseSessionMemoryInput, LearningActor,
        LearningEvidenceInput, LearningKind, LearningProvenance, ProposeLearningInput,
        SessionSource, SessionSourceKind, StartSessionInput, VerificationInput, VerificationStatus,
    };
    use std::process::Command;
    use tempfile::tempdir;

    fn request_id(digit: char) -> String {
        format!("req_{}", digit.to_string().repeat(32))
    }

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

    #[test]
    fn real_legacy_events_import_atomically_and_replay_without_mutating_source() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        git(&project, &["init", "-b", "main"]);
        git(&project, &["config", "user.name", "Ley Test"]);
        git(&project, &["config", "user.email", "ley@example.invalid"]);
        initialize_project(&project, Some("Legacy migration"), CaptureMode::Structured).unwrap();
        std::fs::write(project.join("README.md"), "# Migration fixture\n").unwrap();
        git(&project, &["add", "."]);
        git(&project, &["commit", "-m", "fixture"]);
        ingest_project(&project, &vault).unwrap();

        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('1'),
                name: "Verify migration".to_owned(),
                goal: "Preserve validated continuity history".to_owned(),
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
                summary: "Validated the migration boundary".to_owned(),
                plan: Vec::new(),
                decisions: Vec::new(),
                tasks: Vec::new(),
                problems: Vec::new(),
                touched_artifacts: vec!["README.md".to_owned()],
                commands: Vec::new(),
                verification: vec![VerificationInput {
                    kind: "command".to_owned(),
                    status: VerificationStatus::Passed,
                    summary: "Focused migration check passed".to_owned(),
                    command: Some("cargo test migration".to_owned()),
                    evidence_artifact_paths: vec!["README.md".to_owned()],
                }],
                unresolved: Vec::new(),
            },
        )
        .unwrap();
        let checkpoint_id = checkpoint.session.checkpoints[0].id.clone();
        let learning = propose_learning(
            &project,
            &vault,
            ProposeLearningInput {
                request_id: request_id('3'),
                actor: LearningActor::Agent,
                kind: LearningKind::Procedure,
                title: "Use the validated migration boundary".to_owned(),
                guidance: "Preserve validated raw events before retiring legacy storage."
                    .to_owned(),
                confidence_percent: 80,
                provenance: LearningProvenance::Inferred,
                evidence: vec![LearningEvidenceInput {
                    session_id: started.session.session_id.clone(),
                    record_id: checkpoint_id,
                    note: "The cited checkpoint contains the migration verification.".to_owned(),
                }],
            },
        )
        .unwrap();

        let egress_registry = EgressPolicyRegistry::at(base.path().join("agent-egress-v1.json"));
        egress_registry
            .set_project_policy(&project, AgentEgressPolicy::AgentOk)
            .unwrap();
        let legacy_specification_id = generate_specification_id();
        egress_registry
            .set_specification_policy(
                &project,
                &legacy_specification_id,
                AgentEgressPolicy::LocalModelOnly,
            )
            .unwrap();

        let store = ContinuityStore::at(base.path().join("private/continuity.sqlite3"));
        let first = import_legacy_continuity(&project, &vault, &store).unwrap();
        let egress = import_legacy_project_egress(&project, &egress_registry, &store).unwrap();
        assert_eq!(egress.source_project_policy, AgentEgressPolicy::AgentOk);
        assert_eq!(
            egress.migrated_project_policy,
            AgentEgressPolicy::LocalModelOnly
        );
        assert_eq!(egress.specification_overrides_remaining_legacy, 1);
        assert_eq!(egress.mount_overrides_remaining_legacy, 0);
        assert_eq!(egress.connector_overrides_remaining_legacy, 0);
        assert_eq!(
            store.project_egress_policy(&first.project_id).unwrap(),
            AgentEgressPolicy::LocalModelOnly
        );
        egress_registry
            .set_project_policy_transition(&project, &store, AgentEgressPolicy::AgentOk)
            .unwrap();
        let replayed_egress =
            import_legacy_project_egress(&project, &egress_registry, &store).unwrap();
        assert_eq!(
            replayed_egress.migrated_project_policy,
            AgentEgressPolicy::LocalModelOnly
        );
        assert_eq!(
            store.project_egress_policy(&first.project_id).unwrap(),
            AgentEgressPolicy::AgentOk
        );
        assert_eq!(
            egress_registry
                .list(&project)
                .unwrap()
                .specification_overrides
                .len(),
            1
        );
        assert_eq!(first.session_events, 2);
        assert_eq!(first.learning_events, 1);
        assert!(first.project_created);
        assert_eq!(first.continuity_events_created, 4);
        assert_eq!(first.continuity_events_replayed, 0);
        assert_eq!(first.continuity_events_removed, 0);

        let session_events = store
            .events_for_session(&first.project_id, &started.session.session_id)
            .unwrap();
        assert_eq!(session_events.len(), 2);
        assert_eq!(session_events[1].kind, "legacy-checkpoint-recorded");
        assert_eq!(session_events[1].payload_version, 1);
        assert_eq!(session_events[1].payload["schemaVersion"], 4);
        assert_eq!(session_events[1].payload["kind"], "checkpoint-recorded");
        assert_eq!(session_events[1].payload["requestId"], request_id('2'));
        assert_eq!(
            session_events[1].payload["data"]["verification"][0]["status"],
            "passed"
        );
        assert!(session_events[1].revision_head.is_some());
        assert!(
            session_events[1].payload["data"]["verification"][0]["evidenceArtifacts"][0]
                ["contentHash"]
                .as_str()
                .is_some_and(|hash| hash.starts_with("sha256:"))
        );

        let imported_learning = store
            .event(&first.project_id, &learning.event_id)
            .unwrap()
            .unwrap();
        assert_eq!(imported_learning.kind, "legacy-learning-proposed");
        assert_eq!(
            imported_learning.subject_id.as_deref(),
            Some(learning.learning.learning_id.as_str())
        );
        assert!(imported_learning.session_id.is_none());
        assert!(imported_learning.request_id.is_none());
        assert_eq!(imported_learning.payload["kind"], "proposed");
        assert_eq!(imported_learning.payload["sequence"], 1);
        assert_eq!(
            imported_learning.payload["learningId"],
            learning.learning.learning_id
        );
        assert_eq!(imported_learning.payload["requestId"], request_id('3'));

        let native_preview = store
            .preview_session_erasure(&first.project_id, &started.session.session_id)
            .unwrap();
        assert_eq!(native_preview.session_event_count, 2);
        assert_eq!(native_preview.total_event_count, 3);
        assert_eq!(
            native_preview.dependent_subject_ids,
            vec![learning.learning.learning_id.clone()]
        );

        let manifest = store
            .event(&first.project_id, &first.manifest_event_id)
            .unwrap()
            .unwrap();
        assert_eq!(manifest.kind, "legacy-snapshot-imported");
        assert_eq!(manifest.payload["eventDigest"], first.source_digest);
        assert_eq!(manifest.payload["cutover"], false);

        let replayed = import_legacy_continuity(&project, &vault, &store).unwrap();
        assert_eq!(replayed.source_digest, first.source_digest);
        assert_eq!(replayed.manifest_event_id, first.manifest_event_id);
        assert!(!replayed.project_created);
        assert_eq!(replayed.continuity_events_created, 0);
        assert_eq!(replayed.continuity_events_replayed, 4);
        assert_eq!(replayed.continuity_events_removed, 0);

        let identity = diagnose_project(&project).unwrap().identity;
        let mut conflict = crate::session::continuity_events_for_migration(&project, &vault)
            .unwrap()
            .remove(0);
        conflict.payload["kind"] = json!("tampered");
        let probe = ContinuityEventInput {
            event_id: format!("evt_{}", "f".repeat(64)),
            project_id: identity.project_id.clone(),
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "migration-probe".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 999,
            revision_head: None,
            revision_branch: None,
            payload: json!({"probe":true}),
        };
        assert!(store
            .import_project_events(&identity, &[probe.clone(), conflict])
            .is_err());
        assert!(store
            .event(&identity.project_id, &probe.event_id)
            .unwrap()
            .is_none());

        let native = ContinuityEventInput {
            event_id: format!("evt_{}", "e".repeat(64)),
            project_id: identity.project_id.clone(),
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "native-continuity-probe".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 1_000,
            revision_head: None,
            revision_branch: None,
            payload: json!({"native":true}),
        };
        assert!(store.append_event(&native).unwrap().created);

        assert_eq!(
            read_session(&project, &vault, &started.session.session_id)
                .unwrap()
                .event_count,
            2
        );
        assert_eq!(
            read_learning(&project, &vault, &learning.learning.learning_id)
                .unwrap()
                .event_count,
            1
        );

        let erased = erase_session_memory(
            &project,
            &vault,
            &started.session.session_id,
            EraseSessionMemoryInput {
                expected_event_count: 2,
                expected_name: started.session.name.clone(),
            },
        )
        .unwrap();
        assert_eq!(
            erased.erased_learning_ids,
            vec![learning.learning.learning_id.clone()]
        );

        let after_erasure = import_legacy_continuity(&project, &vault, &store).unwrap();
        assert_eq!(after_erasure.session_events, 0);
        assert_eq!(after_erasure.learning_events, 0);
        assert_ne!(after_erasure.source_digest, first.source_digest);
        assert_ne!(after_erasure.manifest_event_id, first.manifest_event_id);
        assert_eq!(after_erasure.continuity_events_created, 1);
        assert_eq!(after_erasure.continuity_events_replayed, 0);
        assert_eq!(after_erasure.continuity_events_removed, 4);
        assert!(store
            .events_for_session(&first.project_id, &started.session.session_id)
            .unwrap()
            .is_empty());
        assert!(store
            .event(&first.project_id, &learning.event_id)
            .unwrap()
            .is_none());
        assert!(store
            .event(&first.project_id, &first.manifest_event_id)
            .unwrap()
            .is_none());
        assert!(store
            .event(&first.project_id, &after_erasure.manifest_event_id)
            .unwrap()
            .is_some());
        assert_eq!(
            store
                .event(&identity.project_id, &native.event_id)
                .unwrap()
                .unwrap()
                .kind,
            "native-continuity-probe"
        );
        assert!(read_session(&project, &vault, &started.session.session_id).is_err());
        assert!(read_learning(&project, &vault, &learning.learning.learning_id).is_err());
    }

    #[test]
    fn session_only_import_reconciles_sessions_without_claiming_learning_authority() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&vault).unwrap();
        initialize_project(
            &project,
            Some("Session-only migration"),
            CaptureMode::Structured,
        )
        .unwrap();
        std::fs::write(project.join("README.md"), "# Session-only migration\n").unwrap();
        ingest_project(&project, &vault).unwrap();
        let started = start_session(
            &project,
            &vault,
            StartSessionInput {
                request_id: request_id('8'),
                name: "Native session boundary".to_owned(),
                goal: "Move only session authority into continuity storage".to_owned(),
                source: SessionSource::default(),
            },
        )
        .unwrap();
        let identity = diagnose_project(&project).unwrap().identity;
        let store = ContinuityStore::at(base.path().join("private/continuity.sqlite3"));
        store.register_project(&identity).unwrap();

        let learning_event_id = format!("evt_learning_{}", "a".repeat(48));
        store
            .append_event(&ContinuityEventInput {
                event_id: learning_event_id.clone(),
                project_id: identity.project_id.clone(),
                subject_id: Some(format!("lrn_{}", "b".repeat(32))),
                session_id: None,
                session_sequence: None,
                request_id: None,
                request_fingerprint: None,
                kind: "legacy-learning-proposed".to_owned(),
                payload_version: 1,
                recorded_at_unix_ms: 10,
                revision_head: None,
                revision_branch: None,
                payload: json!({"sequence": 1, "kind": "proposed"}),
            })
            .unwrap();
        let stale_session_event_id = format!("evt_stale_{}", "c".repeat(48));
        store
            .append_event(&ContinuityEventInput {
                event_id: stale_session_event_id.clone(),
                project_id: identity.project_id.clone(),
                subject_id: None,
                session_id: Some(format!("ses_{}", "d".repeat(32))),
                session_sequence: Some(1),
                request_id: Some(request_id('9')),
                request_fingerprint: Some(format!("sha256:{}", "e".repeat(64))),
                kind: "legacy-session-started".to_owned(),
                payload_version: 1,
                recorded_at_unix_ms: 11,
                revision_head: None,
                revision_branch: None,
                payload: json!({"stale": true}),
            })
            .unwrap();

        let imported = import_legacy_session_continuity(&project, &vault, &store).unwrap();
        assert_eq!(imported.session_events, 1);
        assert_eq!(imported.continuity_events_created, 2);
        assert_eq!(imported.continuity_events_replayed, 0);
        assert_eq!(imported.continuity_events_removed, 1);
        assert!(store
            .event(&identity.project_id, &learning_event_id)
            .unwrap()
            .is_some());
        assert!(store
            .event(&identity.project_id, &stale_session_event_id)
            .unwrap()
            .is_none());
        assert_eq!(
            store
                .events_for_session(&identity.project_id, &started.session.session_id)
                .unwrap()
                .len(),
            1
        );
        let manifest = store
            .event(&identity.project_id, &imported.manifest_event_id)
            .unwrap()
            .unwrap();
        assert_eq!(manifest.kind, "legacy-session-snapshot-imported");
        assert_eq!(manifest.payload["eventDigest"], imported.source_digest);
        assert_eq!(manifest.payload["sessionEventCount"], 1);
        assert_eq!(manifest.payload["cutover"], false);
    }

    #[test]
    fn legacy_approved_source_import_snapshots_only_exact_current_revisions() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        let vault = base.path().join("vault");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir_all(vault.join("Specs")).unwrap();
        git(&project, &["init", "-b", "main"]);
        git(&project, &["config", "user.name", "Ley Test"]);
        git(&project, &["config", "user.email", "ley@example.invalid"]);
        let initialized = initialize_project(
            &project,
            Some("Approved source migration"),
            CaptureMode::Structured,
        )
        .unwrap();

        let registry = SpecificationRegistry::at(base.path().join("specifications-v1.json"));
        let current_id = generate_specification_id();
        let changed_id = generate_specification_id();
        let missing_id = generate_specification_id();
        let invalid_id = generate_specification_id();

        std::fs::write(
            vault.join("Specs/Current.md"),
            "# Current\nexact approved bytes\n",
        )
        .unwrap();
        registry
            .approve(&project, &vault, &current_id, "Specs/Current.md")
            .unwrap();

        std::fs::write(
            vault.join("Specs/Changed.md"),
            "# Changed\napproved bytes\n",
        )
        .unwrap();
        registry
            .approve(&project, &vault, &changed_id, "Specs/Changed.md")
            .unwrap();
        std::fs::write(
            vault.join("Specs/Changed.md"),
            "# Changed\nnew unapproved bytes\n",
        )
        .unwrap();

        std::fs::write(
            vault.join("Specs/Missing.md"),
            "# Missing\napproved bytes\n",
        )
        .unwrap();
        registry
            .approve(&project, &vault, &missing_id, "Specs/Missing.md")
            .unwrap();
        std::fs::remove_file(vault.join("Specs/Missing.md")).unwrap();

        std::fs::write(vault.join("Specs/Invalid.md"), [0xff, 0xfe, 0xfd]).unwrap();
        registry
            .approve(&project, &vault, &invalid_id, "Specs/Invalid.md")
            .unwrap();

        let store = ContinuityStore::at(base.path().join("private/continuity.sqlite3"));
        let first = import_legacy_approved_sources(&project, &vault, &registry, &store).unwrap();
        assert_eq!(first.project_id, initialized.identity.project_id);
        assert_eq!(first.imported_snapshots, 1);
        assert_eq!(first.changed_approvals, 1);
        assert_eq!(first.missing_approvals, 1);
        assert_eq!(first.invalid_approvals, 1);
        assert!(first.authority_migrated);
        assert!(store
            .approved_source_authority_ready(&first.project_id)
            .unwrap());

        std::fs::remove_file(registry.path()).unwrap();
        std::fs::remove_dir_all(&vault).unwrap();
        let replay = import_legacy_approved_sources(&project, &vault, &registry, &store).unwrap();
        assert_eq!(replay.imported_snapshots, 0);
        assert_eq!(replay.changed_approvals, 0);
        assert_eq!(replay.missing_approvals, 0);
        assert_eq!(replay.invalid_approvals, 0);
        assert!(!replay.authority_migrated);
    }

    #[test]
    fn conservative_legacy_egress_fold_never_weakens_distinct_restrictions() {
        use AgentEgressPolicy::{AgentOk, ConfirmPerUse, LocalModelOnly, NeverSend};

        assert_eq!(conservative_legacy_egress_policy(AgentOk, []), AgentOk);
        assert_eq!(
            conservative_legacy_egress_policy(AgentOk, [LocalModelOnly]),
            LocalModelOnly
        );
        assert_eq!(
            conservative_legacy_egress_policy(AgentOk, [ConfirmPerUse]),
            ConfirmPerUse
        );
        assert_eq!(
            conservative_legacy_egress_policy(LocalModelOnly, [ConfirmPerUse]),
            NeverSend
        );
        assert_eq!(
            conservative_legacy_egress_policy(ConfirmPerUse, [LocalModelOnly]),
            NeverSend
        );
        assert_eq!(
            conservative_legacy_egress_policy(AgentOk, [LocalModelOnly, ConfirmPerUse]),
            NeverSend
        );
        assert_eq!(
            conservative_legacy_egress_policy(NeverSend, [LocalModelOnly, ConfirmPerUse]),
            NeverSend
        );
    }

    #[test]
    fn project_egress_import_validates_legacy_authority_before_writing_store() {
        let base = tempdir().unwrap();
        let project = base.path().join("project");
        std::fs::create_dir(&project).unwrap();
        git(&project, &["init", "-b", "main"]);
        git(&project, &["config", "user.name", "Ley Test"]);
        git(&project, &["config", "user.email", "ley@example.invalid"]);
        let initialized = initialize_project(
            &project,
            Some("Invalid egress migration"),
            CaptureMode::Structured,
        )
        .unwrap();

        let registry_path = base.path().join("agent-egress-v1.json");
        std::fs::write(&registry_path, r#"{"schemaVersion":999,"projects":{}}"#).unwrap();
        let registry = EgressPolicyRegistry::at(registry_path);
        let store = ContinuityStore::at(base.path().join("private/continuity.sqlite3"));

        assert!(matches!(
            import_legacy_project_egress(&project, &registry, &store),
            Err(LeyCoreError::InvalidEgressPolicyRegistry(_))
        ));
        assert!(matches!(
            store.project_egress_policy(&initialized.identity.project_id),
            Err(LeyCoreError::InvalidContinuityStore(_))
        ));
    }

    #[test]
    fn project_catalog_import_preserves_unavailable_projects_and_runs_only_once() {
        let base = tempdir().unwrap();
        let first = base.path().join("first");
        let second = base.path().join("second");
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&second).unwrap();
        let first_identity =
            initialize_project(&first, Some("First"), CaptureMode::Structured).unwrap();
        let second_identity =
            initialize_project(&second, Some("Second"), CaptureMode::Structured).unwrap();

        let catalog = ProjectCatalog::at(base.path().join("projects-v1.json"));
        let first_observed = catalog.observe(&first).unwrap();
        let second_observed = catalog.observe(&second).unwrap();
        std::fs::remove_dir_all(&second).unwrap();

        let store = ContinuityStore::at(base.path().join("private/continuity.sqlite3"));
        let imported = import_legacy_project_catalog(&catalog, &store).unwrap();
        assert_eq!(imported.project_observations, 2);
        assert!(imported.catalog_migrated);
        let observations = store.project_observations().unwrap();
        assert_eq!(observations.len(), 2);
        assert!(observations.iter().any(|observation| {
            observation.project_id == first_observed.project_id
                && observation.root_path == first_observed.root_path
        }));
        assert!(observations.iter().any(|observation| {
            observation.project_id == second_observed.project_id
                && observation.root_path == second_observed.root_path
        }));
        assert_eq!(
            second_observed.project_id,
            second_identity.identity.project_id
        );

        catalog.forget(&first_identity.identity.project_id).unwrap();
        let resynced = import_legacy_project_catalog(&catalog, &store).unwrap();
        assert_eq!(resynced.project_observations, 1);
        assert!(!resynced.catalog_migrated);
        let observations = store.project_observations().unwrap();
        assert_eq!(observations.len(), 2);
        assert!(observations.iter().any(|observation| {
            observation.project_id == first_observed.project_id
                && observation.root_path == first_observed.root_path
        }));
        assert!(observations.iter().any(|observation| {
            observation.project_id == second_observed.project_id
                && observation.root_path == second_observed.root_path
        }));
    }
}
