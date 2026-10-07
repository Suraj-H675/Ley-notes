use ley_core::{CaptureMode, ContinuityStore, LeyCoreError, WorkingCopyState};
use serde::Serialize;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureTarget {
    project_id: String,
    locator_id: String,
    local_path: std::path::PathBuf,
    mode: Option<CaptureMode>,
    grant_id: Option<String>,
    authorized: bool,
    effective: bool,
    inactive_reason: Option<String>,
}

#[tauri::command]
pub(crate) fn read_chronicle_capture_targets(
    project_id: String,
) -> Result<Vec<CaptureTarget>, String> {
    let store = ContinuityStore::system_default().map_err(|error| error.to_string())?;
    capture_targets(&store, &project_id).map_err(|error| error.to_string())
}

fn capture_targets(
    store: &ContinuityStore,
    project_id: &str,
) -> Result<Vec<CaptureTarget>, LeyCoreError> {
    store.open_project_brain(project_id)?;
    let mut targets = Vec::new();
    for locator in store.working_copies(project_id)? {
        if locator.state != WorkingCopyState::Authorized {
            continue;
        }
        let state = store.chronicle_capture_state(project_id, &locator.locator_id)?;
        targets.push(CaptureTarget {
            project_id: project_id.to_owned(),
            locator_id: locator.locator_id,
            local_path: locator.local_path,
            mode: state.as_ref().map(|state| state.mode),
            grant_id: state.as_ref().map(|state| state.grant_id.clone()),
            authorized: state.as_ref().is_some_and(|state| state.authorized),
            effective: state.as_ref().is_some_and(|state| state.effective),
            inactive_reason: state.and_then(|state| state.inactive_reason),
        });
        if targets.len() > 1_000 {
            return Err(LeyCoreError::InvalidContinuityStore(
                "capture permission listing exceeds 1,000 working copies".to_owned(),
            ));
        }
    }
    Ok(targets)
}

#[tauri::command]
pub(crate) async fn request_chronicle_capture(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    project_id: String,
    locator_id: String,
    mode: CaptureMode,
) -> Result<bool, String> {
    if window.label() != "main" {
        return Err("capture permission requires the Ley control window".to_owned());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let store = ContinuityStore::system_default().map_err(|error| error.to_string())?;
        let preview = store
            .preview_chronicle_capture(&project_id, &locator_id, mode)
            .map_err(|error| error.to_string())?;
        let brain = store
            .open_project_brain(&project_id)
            .map_err(|error| error.to_string())?;
        let retention = match mode {
            CaptureMode::Minimal => {
                "Record supported event boundaries and explicit gaps without message, command, or result bodies."
            }
            CaptureMode::Structured | CaptureMode::FullEvidence => {
                "Retain bounded, pattern-redacted visible prompts, responses, and supported Bash result evidence."
            }
        };
        let text = format!(
            "Allow future Codex activity capture for {}?\n\nProject: {}\nWorking copy: {}\nLocator: {}\nRetention: {}\n\n{}\n\nThis does not capture other projects, import old transcripts, read hidden reasoning, or grant model sharing. Hook observations can be incomplete. Codex hooks must also be installed and trusted.",
            brain.identity.name,
            project_id,
            preview.root.display(),
            locator_id,
            mode,
            retention
        );
        let confirmed = app
            .dialog()
            .message(text)
            .title("Authorize Codex capture")
            .parent(&window)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Enable capture".to_owned(),
                "Cancel".to_owned(),
            ))
            .blocking_show();
        if !confirmed {
            return Ok(false);
        }
        store
            .authorize_chronicle_capture(&preview)
            .map_err(|error| error.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) fn revoke_chronicle_capture(
    project_id: String,
    locator_id: String,
    expected_grant_id: String,
) -> Result<(), String> {
    let store = ContinuityStore::system_default().map_err(|error| error.to_string())?;
    store
        .revoke_chronicle_capture(&project_id, &locator_id, &expected_grant_id)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ley_core::{CreateProjectBrainInput, ProjectHandle};

    #[test]
    fn capture_targets_are_project_scoped_and_attachment_is_not_permission() {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path().join("project");
        let other_root = scratch.path().join("other");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&other_root).unwrap();
        let store = ContinuityStore::at(scratch.path().join("private/continuity.sqlite3"));
        let brain = store
            .create_project_brain(&CreateProjectBrainInput {
                name: "Capture scope".to_owned(),
                request_id: "capture-control-create".to_owned(),
            })
            .unwrap();
        let other = store
            .create_project_brain(&CreateProjectBrainInput {
                name: "Other".to_owned(),
                request_id: "capture-control-create-other".to_owned(),
            })
            .unwrap();
        let (_, locator) = store
            .authorize_working_copy(
                &ProjectHandle {
                    project_id: brain.identity.project_id.clone(),
                    generation: brain.generation,
                },
                &root,
                "capture-control-attach",
            )
            .unwrap();
        store
            .authorize_working_copy(
                &ProjectHandle {
                    project_id: other.identity.project_id.clone(),
                    generation: other.generation,
                },
                &other_root,
                "capture-control-attach-other",
            )
            .unwrap();

        let targets = capture_targets(&store, &brain.identity.project_id).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].project_id, brain.identity.project_id);
        assert_eq!(targets[0].locator_id, locator.locator_id);
        assert_eq!(targets[0].local_path, root);
        assert!(!targets[0].authorized);
        assert!(!targets[0].effective);
        assert!(targets[0].grant_id.is_none());
        assert!(targets[0].mode.is_none());
    }
}
