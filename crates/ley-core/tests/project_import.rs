use ley_core::{
    ContentsEntry, ContinuityStore, CreateProjectBrainInput, InventoryState, LocalImportInput,
    ProjectHandle, SourceVersionInput, MAX_IMPORT_ENTRIES,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
struct Fixture {
    temp: TempDir,
    root: PathBuf,
    store: ContinuityStore,
    handle: ProjectHandle,
    locator: String,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let store = ContinuityStore::at(temp.path().join("private/continuity.sqlite3"));
        let brain = store
            .create_project_brain(&CreateProjectBrainInput {
                name: "Import test".into(),
                request_id: "req_create".into(),
            })
            .unwrap();
        let handle = ProjectHandle {
            project_id: brain.identity.project_id,
            generation: brain.generation,
        };
        let (_, locator) = store
            .authorize_working_copy(&handle, &root, "req_attach")
            .unwrap();
        Self {
            temp,
            root,
            store,
            handle,
            locator: locator.locator_id,
        }
    }
    fn input(&self, request: &str) -> LocalImportInput {
        LocalImportInput {
            locator_id: self.locator.clone(),
            selected_root: self.root.clone(),
            request_id: request.into(),
        }
    }
    fn import(&self, request: &str) -> ley_core::ImportReceipt {
        self.store
            .import_local_project(&self.handle, &self.input(request))
            .unwrap()
    }
    fn entries(&self) -> Vec<ContentsEntry> {
        self.store
            .project_contents(&self.handle.project_id, &self.locator)
            .unwrap()
            .entries
    }
    fn entry(&self, path: &str) -> ContentsEntry {
        self.entries()
            .into_iter()
            .find(|e| e.relative_path == path)
            .unwrap()
    }
}
#[test]
fn non_git_retry_fresh_occurrences_changes_and_deletion_are_distinct() {
    let f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    let first = f.import("req_first");
    assert_eq!(first.git["kind"], "not-repository");
    let entry = f.entry("a.txt");
    let source = entry.source_id.clone().unwrap();
    let version = entry.source_version_id.clone().unwrap();
    fs::write(f.root.join("a.txt"), "second").unwrap();
    assert_eq!(
        f.store
            .import_local_project(&f.handle, &f.input("req_first"))
            .unwrap(),
        first
    );
    assert_eq!(f.entry("a.txt"), entry);
    let changed = f.import("req_changed");
    assert_eq!(changed.modified, vec!["a.txt"]);
    assert_eq!(f.entry("a.txt").source_id, Some(source.clone()));
    assert_ne!(f.entry("a.txt").source_version_id, Some(version.clone()));
    let same = f.import("req_same");
    assert_eq!(same.observation_digest, changed.observation_digest);
    assert_ne!(same.scan_id, changed.scan_id);
    let versions = f
        .store
        .source_versions(&f.handle.project_id, &source)
        .unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions.iter().map(|v| v.occurrence_count).sum::<u64>(), 3);
    assert!(versions.iter().any(|v| v
        .occurrences
        .iter()
        .any(|e| e["import"]["scanId"] == same.scan_id)));
    fs::remove_file(f.root.join("a.txt")).unwrap();
    let deleted = f.import("req_deleted");
    assert_eq!(deleted.deleted, vec!["a.txt"]);
    assert_eq!(f.entry("a.txt").state, InventoryState::Missing);
    assert_eq!(
        f.store
            .source_version_bytes(&f.handle.project_id, &source, &version)
            .unwrap(),
        b"first"
    );
    assert_eq!(
        f.store.project_sources(&f.handle.project_id).unwrap()[0].state,
        ley_core::ProjectSourceState::Active
    );
}
#[test]
fn conservative_renames_use_unique_original_hashes() {
    let f = Fixture::new();
    fs::write(f.root.join("a.txt"), "unique").unwrap();
    f.import("req_a");
    let source = f.entry("a.txt").source_id;
    fs::rename(f.root.join("a.txt"), f.root.join("b.txt")).unwrap();
    let renamed = f.import("req_b");
    assert_eq!(renamed.renamed.len(), 1);
    assert_eq!(f.entry("b.txt").source_id, source);
    fs::write(f.root.join("copy.txt"), "unique").unwrap();
    f.import("req_copy");
    let old = f.entry("b.txt").source_id;
    fs::rename(f.root.join("b.txt"), f.root.join("c.txt")).unwrap();
    let ambiguous = f.import("req_ambiguous");
    assert!(ambiguous.renamed.is_empty());
    assert_ne!(f.entry("c.txt").source_id, old);
}
#[test]
fn redaction_counts_transformation_and_identical_redacted_secrets_do_not_rename() {
    let f = Fixture::new();
    fs::write(f.root.join("a.txt"), "password=x\n").unwrap();
    f.import("req_a");
    let entry = f.entry("a.txt");
    let evidence = f
        .store
        .source_evidence(
            &f.handle.project_id,
            entry.source_id.as_deref().unwrap(),
            entry.source_version_id.as_deref().unwrap(),
        )
        .unwrap();
    assert!(!String::from_utf8_lossy(&evidence.bytes).contains("password=x"));
    assert_eq!(evidence.version.source_bytes, 11);
    assert!(evidence.version.stored_bytes > evidence.version.source_bytes);
    assert_eq!(evidence.transformation["kind"], "secret-redacted-utf8");
    fs::remove_file(f.root.join("a.txt")).unwrap();
    fs::write(f.root.join("b.txt"), "password=y\n").unwrap();
    let next = f.import("req_b");
    assert!(next.renamed.is_empty());
    assert_ne!(f.entry("b.txt").source_id, entry.source_id);
}
#[test]
fn hard_exclusions_ignore_negation_binary_oversized_and_unicode_paths() {
    let f = Fixture::new();
    fs::create_dir(f.root.join("node_modules")).unwrap();
    fs::write(f.root.join("node_modules/private.txt"), "secret").unwrap();
    fs::write(f.root.join(".env"), "password=secret").unwrap();
    fs::write(
        f.root.join(".gitignore"),
        "ignored.txt\n!node_modules/\n!.env\n",
    )
    .unwrap();
    fs::write(f.root.join("ignored.txt"), "ignored").unwrap();
    fs::write(f.root.join("binary.txt"), b"a\0b").unwrap();
    fs::write(f.root.join("huge.txt"), vec![b'x'; 1_048_577]).unwrap();
    fs::write(f.root.join("hello-世界\nback\\slash.txt"), "unicode").unwrap();
    let receipt = f.import("req_import");
    assert_eq!(receipt.retained_count, 2);
    assert_eq!(
        f.entry(".env").omission_reason.as_deref(),
        Some("secret-file")
    );
    assert_eq!(f.entry("node_modules").state, InventoryState::Omitted);
    assert_eq!(
        f.entry("ignored.txt").omission_reason.as_deref(),
        Some("ignored")
    );
    assert_eq!(
        f.entry("binary.txt").omission_reason.as_deref(),
        Some("binary")
    );
    assert_eq!(
        f.entry("huge.txt").omission_reason.as_deref(),
        Some("oversized")
    );
    assert_eq!(
        f.entry("hello-世界\nback\\slash.txt").state,
        InventoryState::Retained
    );
}
#[test]
fn ancestor_omission_is_not_deletion_and_path_source_identity_survives() {
    let f = Fixture::new();
    fs::create_dir(f.root.join("src")).unwrap();
    fs::write(f.root.join("src/a.txt"), "first").unwrap();
    f.import("req_a");
    let source = f.entry("src/a.txt").source_id;
    fs::write(f.root.join(".gitignore"), "src/\n").unwrap();
    let ignored = f.import("req_ignored");
    assert!(ignored.deleted.is_empty());
    assert_eq!(f.entry("src/a.txt").state, InventoryState::Omitted);
    assert_eq!(f.entry("src/a.txt").source_id, source);
    fs::write(f.root.join(".gitignore"), "").unwrap();
    f.import("req_unignore");
    assert_eq!(f.entry("src/a.txt").source_id, source);
}
#[cfg(unix)]
#[test]
fn symlinks_never_read_outside_and_ignore_failure_is_atomic() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = f.temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("canary.txt"), "OUTSIDE_CANARY").unwrap();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    symlink(outside.join("canary.txt"), f.root.join("link.txt")).unwrap();
    symlink(&outside, f.root.join("link-dir")).unwrap();
    let first = f.import("req_first");
    assert_eq!(
        f.entry("link.txt").omission_reason.as_deref(),
        Some("symlink")
    );
    assert_eq!(
        f.entry("link-dir").omission_reason.as_deref(),
        Some("symlink")
    );
    let before = f.entries();
    fs::write(f.root.join("a.txt"), "second").unwrap();
    symlink(outside.join("canary.txt"), f.root.join(".gitignore")).unwrap();
    assert!(f
        .store
        .import_local_project(&f.handle, &f.input("req_fail"))
        .is_err());
    let contents = f
        .store
        .project_contents(&f.handle.project_id, &f.locator)
        .unwrap();
    assert_eq!(contents.entries, before);
    assert_eq!(contents.last_success.unwrap(), first);
    assert_eq!(contents.latest_attempt.unwrap().status, "failed");
    assert_eq!(
        f.store.project_sources(&f.handle.project_id).unwrap().len(),
        1
    );
}
#[test]
fn total_bound_failure_preserves_prior_observation() {
    let f = Fixture::new();
    fs::write(f.root.join("first.txt"), "first").unwrap();
    let first = f.import("req_first");
    for index in 0..33 {
        fs::write(
            f.root.join(format!("{index:02}.txt")),
            vec![b'x'; 1_048_576],
        )
        .unwrap();
    }
    assert!(f
        .store
        .import_local_project(&f.handle, &f.input("req_large"))
        .is_err());
    let contents = f
        .store
        .project_contents(&f.handle.project_id, &f.locator)
        .unwrap();
    assert_eq!(contents.last_success.unwrap(), first);
    assert_eq!(contents.entries.len(), 1);
    assert_eq!(
        f.store.project_sources(&f.handle.project_id).unwrap().len(),
        1
    );
}
#[test]
fn entry_bound_fails_before_publication() {
    let f = Fixture::new();
    for index in 0..=MAX_IMPORT_ENTRIES {
        fs::write(f.root.join(format!("{index}.txt")), "").unwrap();
    }
    assert!(f
        .store
        .import_local_project(&f.handle, &f.input("req_many"))
        .is_err());
    assert!(f.entries().is_empty());
    assert!(f
        .store
        .project_sources(&f.handle.project_id)
        .unwrap()
        .is_empty());
}
#[test]
fn source_erasure_fences_current_and_historical_renamed_paths() {
    let mut f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    f.import("req_a");
    let source = f.entry("a.txt").source_id.unwrap();
    fs::rename(f.root.join("a.txt"), f.root.join("b.txt")).unwrap();
    f.import("req_b");
    let old = f.handle.clone();
    f.handle = f.store.erase_project_source(&f.handle, &source).unwrap();
    assert!(f
        .store
        .import_local_project(&old, &f.input("req_stale"))
        .is_err());
    let contents = f
        .store
        .project_contents(&f.handle.project_id, &f.locator)
        .unwrap();
    assert!(contents.last_success.is_none());
    assert!(contents.entries.is_empty());
    assert!(f
        .store
        .import_local_project(&f.handle, &f.input("req_b"))
        .is_err());
    fs::write(f.root.join("a.txt"), "first").unwrap();
    f.import("req_fresh");
    assert_eq!(
        f.entry("a.txt").omission_reason.as_deref(),
        Some("erased-source")
    );
    assert_eq!(
        f.entry("b.txt").omission_reason.as_deref(),
        Some("erased-source")
    );
    assert_eq!(
        f.store.project_sources(&f.handle.project_id).unwrap().len(),
        1
    );
}
#[test]
fn explicit_move_preserves_locator_and_sources_and_rejects_copied_authority() {
    let mut f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    f.import("req_a");
    let source = f.entry("a.txt").source_id;
    let moved = f.temp.path().join("moved");
    fs::rename(&f.root, &moved).unwrap();
    assert!(f
        .store
        .import_local_project(&f.handle, &f.input("req_old"))
        .is_err());
    let old = f.handle.clone();
    f.handle = f
        .store
        .relocate_working_copy(&f.handle, &f.locator, &moved, "req_move")
        .unwrap();
    assert!(f.handle.generation > old.generation);
    assert_eq!(
        f.store
            .relocate_working_copy(&old, &f.locator, &moved, "req_move")
            .unwrap(),
        f.handle
    );
    f.root = moved;
    f.import("req_moved");
    assert_eq!(f.entry("a.txt").source_id, source);
    let copy = f.temp.path().join("copy");
    fs::create_dir(&copy).unwrap();
    fs::write(copy.join("a.txt"), "first").unwrap();
    assert!(f
        .store
        .relocate_working_copy(&f.handle, &f.locator, &copy, "req_move_copy")
        .is_err());
    fs::remove_dir_all(&f.root).unwrap();
    assert!(f
        .store
        .relocate_working_copy(&f.handle, &f.locator, &copy, "req_copy_after_delete")
        .is_err());
}
#[test]
fn m1_retention_keeps_true_original_size_for_expanding_representation() {
    let f = Fixture::new();
    let source = f
        .store
        .create_project_source(&f.handle, "text", "Reference", "req_source")
        .unwrap();
    let version = f
        .store
        .retain_source_version(
            &f.handle,
            &source.source_id,
            &SourceVersionInput {
                request_id: "req_version".into(),
                representation_kind: "redacted-text".into(),
                original_content_hash: None,
                source_bytes: 1,
                transformation: json!({"kind":"redacted"}),
                retained_bytes: b"[REDACTED]".to_vec(),
            },
        )
        .unwrap();
    assert_eq!(version.source_bytes, 1);
    assert_eq!(version.stored_bytes, 10);
}
#[test]
fn git_observes_unborn_head_dirty_untracked_and_version_provenance() {
    use std::process::Command;
    let f = Fixture::new();
    let run = |args: &[&str]| {
        let result = Command::new("git")
            .args(args)
            .current_dir(&f.root)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    run(&["init", "-q"]);
    fs::write(f.root.join("a.txt"), "first").unwrap();
    let unborn = f.import("req_unborn");
    if !cfg!(target_os = "linux") {
        assert_eq!(unborn.git["kind"], "unavailable");
        assert_eq!(unborn.git["reason"], "capability-cwd-unavailable");
        return;
    }
    assert_eq!(unborn.git["kind"], "observed");
    assert!(unborn.git["head"].is_null());
    assert_eq!(unborn.git["untracked"], true);
    run(&["add", "a.txt"]);
    run(&[
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.invalid",
        "commit",
        "-qm",
        "First",
    ]);
    fs::write(f.root.join("a.txt"), "second").unwrap();
    let dirty = f.import("req_dirty");
    assert_eq!(dirty.git["dirty"], true);
    assert!(dirty.git["head"].as_str().is_some());
    let entry = f.entry("a.txt");
    let evidence = f
        .store
        .source_evidence(
            &f.handle.project_id,
            entry.source_id.as_deref().unwrap(),
            entry.source_version_id.as_deref().unwrap(),
        )
        .unwrap();
    assert_eq!(evidence.occurrences[0]["revisionHead"], dirty.git["head"]);
    assert_eq!(evidence.occurrences[0]["import"]["locatorId"], f.locator);
    assert_eq!(evidence.occurrences[0]["import"]["git"]["dirty"], true);
    run(&["checkout", "--detach", "-q"]);
    let detached = f.import("req_detached");
    assert!(detached.git["branch"].is_null());
}
#[test]
fn publication_collision_rolls_back_sources_versions_inventory_and_new_blobs() {
    let f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    let first = f.import("req_first");
    let before = f.entries();
    let source = f
        .store
        .create_project_source(&f.handle, "text", "Collision", "req_collision")
        .unwrap();
    fs::write(
        f.root.join("b.txt"),
        "new content prepared before collision",
    )
    .unwrap();
    assert!(f
        .store
        .import_local_project(&f.handle, &f.input("req_collision"))
        .is_err());
    let contents = f
        .store
        .project_contents(&f.handle.project_id, &f.locator)
        .unwrap();
    assert_eq!(contents.last_success.unwrap(), first);
    assert_eq!(contents.entries, before);
    assert_eq!(
        f.store.project_sources(&f.handle.project_id).unwrap().len(),
        2
    );
    assert!(f
        .store
        .source_versions(&f.handle.project_id, &source.source_id)
        .unwrap()
        .is_empty());
    let entry = f.entry("a.txt");
    assert_eq!(
        f.store
            .source_version_bytes(
                &f.handle.project_id,
                entry.source_id.as_deref().unwrap(),
                entry.source_version_id.as_deref().unwrap()
            )
            .unwrap(),
        b"first"
    );
}
#[test]
fn locator_inventories_do_not_globally_remove_other_sources() {
    let f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    f.import("req_a");
    let second_root = f.temp.path().join("second");
    fs::create_dir(&second_root).unwrap();
    fs::write(second_root.join("a.txt"), "first").unwrap();
    let (_, second) = f
        .store
        .authorize_working_copy(&f.handle, &second_root, "req_second_attach")
        .unwrap();
    f.store
        .import_local_project(
            &f.handle,
            &LocalImportInput {
                locator_id: second.locator_id.clone(),
                selected_root: second_root,
                request_id: "req_second_import".into(),
            },
        )
        .unwrap();
    let second_contents = f
        .store
        .project_contents(&f.handle.project_id, &second.locator_id)
        .unwrap();
    assert_ne!(
        second_contents.entries[0].source_id,
        f.entry("a.txt").source_id
    );
    fs::remove_file(f.root.join("a.txt")).unwrap();
    f.import("req_delete");
    assert_eq!(
        f.store
            .project_contents(&f.handle.project_id, &second.locator_id)
            .unwrap()
            .entries[0]
            .state,
        InventoryState::Retained
    );
    assert!(f
        .store
        .project_sources(&f.handle.project_id)
        .unwrap()
        .iter()
        .all(|s| s.state == ley_core::ProjectSourceState::Active));
}
#[test]
fn omission_only_import_disables_portable_v1_and_legacy_memory_reset() {
    let f = Fixture::new();
    fs::write(f.root.join(".env"), "password=x").unwrap();
    f.import("req_omission");
    assert!(f
        .store
        .project_sources(&f.handle.project_id)
        .unwrap()
        .is_empty());
    let error = ley_core::export_portable_continuity(
        &f.store,
        f.temp.path().join("unused-vault"),
        &f.handle.project_id,
        f.temp.path().join("bundle"),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("canonical import state"),
        "{error}"
    );
    assert!(f
        .store
        .reset_agent_memory_for_recapture(&f.handle.project_id)
        .is_err());
}
#[test]
fn forged_relocation_event_is_reserved() {
    let f = Fixture::new();
    let input = ley_core::ContinuityEventInput {
        event_id: "evt_forged_move".into(),
        project_id: f.handle.project_id.clone(),
        subject_id: None,
        session_id: None,
        session_sequence: None,
        request_id: Some("req_forged_move".into()),
        request_fingerprint: None,
        kind: "working-copy-relocated".into(),
        payload_version: 1,
        recorded_at_unix_ms: 1,
        revision_head: None,
        revision_branch: None,
        payload: json!({"origin":"local-user-control"}),
    };
    assert!(f.store.append_event(&input).is_err());
}
#[test]
fn concurrent_source_erasure_fences_inflight_import() {
    use std::time::{Duration, Instant};
    let f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    f.import("req_first");
    let source = f.entry("a.txt").source_id.unwrap();
    for i in 0..30 {
        fs::write(f.root.join(format!("{i:02}.txt")), vec![b'x'; 1_048_576]).unwrap();
    }
    let store = f.store.clone();
    let handle = f.handle.clone();
    let input = f.input("req_inflight");
    let worker = std::thread::spawn(move || store.import_local_project(&handle, &input));
    let start = Instant::now();
    loop {
        let conn = rusqlite::Connection::open(f.store.path()).unwrap();
        let running:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM project_import_attempts WHERE request_id='req_inflight' AND status='running')",[],|r|r.get(0)).unwrap();
        if running {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(1));
    }
    f.store.erase_project_source(&f.handle, &source).unwrap();
    assert!(worker.join().unwrap().is_err());
    assert!(f
        .store
        .project_sources(&f.handle.project_id)
        .unwrap()
        .iter()
        .all(|s| s.state == ley_core::ProjectSourceState::Erased));
    assert!(f.entries().is_empty());
}
#[test]
fn root_ley_ignore_is_bounded_scoped_exclusion_data() {
    let f = Fixture::new();
    fs::create_dir(f.root.join(".ley")).unwrap();
    fs::write(f.root.join(".ley/.leyignore"), "custom.txt\n!.env\n").unwrap();
    fs::write(f.root.join("custom.txt"), "excluded by Ley").unwrap();
    fs::write(f.root.join(".env"), "password=x").unwrap();
    let receipt = f.import("req_ley_ignore");
    assert_eq!(receipt.retained_count, 0);
    assert_eq!(
        f.entry("custom.txt").omission_reason.as_deref(),
        Some("ignored")
    );
    assert_eq!(
        f.entry(".env").omission_reason.as_deref(),
        Some("secret-file")
    );
}
#[cfg(unix)]
#[test]
fn excluded_symlink_ley_directory_never_loads_outside_ignore() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = f.temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join(".leyignore"), "keep.txt\n").unwrap();
    symlink(&outside, f.root.join(".ley")).unwrap();
    fs::write(f.root.join("keep.txt"), "keep").unwrap();
    f.import("req_ley_symlink");
    assert_eq!(f.entry("keep.txt").state, InventoryState::Retained);
}
#[cfg(unix)]
#[test]
fn fresh_erased_sources_are_not_read_or_hashed_again() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    f.import("req_first");
    let source = f.entry("a.txt").source_id.unwrap();
    f.handle = f.store.erase_project_source(&f.handle, &source).unwrap();
    fs::set_permissions(f.root.join("a.txt"), fs::Permissions::from_mode(0)).unwrap();
    let first = f.import("req_erased");
    fs::set_permissions(f.root.join("a.txt"), fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(
        f.root.join("a.txt"),
        "different original of a different size",
    )
    .unwrap();
    let changed = f.import("req_erased_changed");
    assert_eq!(first.observation_digest, changed.observation_digest);
    assert_eq!(f.entry("a.txt").observed_bytes, None);
}
#[test]
fn git_selected_root_with_newline_is_resolved_without_trimming_valid_path() {
    use std::process::Command;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root\n");
    fs::create_dir(&root).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status()
        .unwrap()
        .success());
    fs::write(root.join("a.txt"), "first").unwrap();
    let store = ContinuityStore::at(temp.path().join("private/continuity.sqlite3"));
    let brain = store
        .create_project_brain(&CreateProjectBrainInput {
            name: "Newline path".into(),
            request_id: "req_create".into(),
        })
        .unwrap();
    let handle = ProjectHandle {
        project_id: brain.identity.project_id,
        generation: brain.generation,
    };
    let (_, locator) = store
        .authorize_working_copy(&handle, &root, "req_attach")
        .unwrap();
    let receipt = store
        .import_local_project(
            &handle,
            &LocalImportInput {
                locator_id: locator.locator_id,
                selected_root: root,
                request_id: "req_import".into(),
            },
        )
        .unwrap();
    if cfg!(target_os = "linux") {
        assert_eq!(receipt.git["kind"], "observed");
        assert_eq!(receipt.git["headState"], "unborn");
    } else {
        assert_eq!(receipt.git["kind"], "unavailable");
        assert_eq!(receipt.git["reason"], "capability-cwd-unavailable");
    }
}
#[test]
fn import_rejects_roots_containing_or_inside_private_continuity_storage() {
    let f = Fixture::new();
    let (_, parent) = f
        .store
        .authorize_working_copy(&f.handle, f.temp.path(), "req_parent_attach")
        .unwrap();
    let parent_input = LocalImportInput {
        locator_id: parent.locator_id,
        selected_root: f.temp.path().to_owned(),
        request_id: "req_parent_import".into(),
    };
    assert!(f
        .store
        .import_local_project(&f.handle, &parent_input)
        .unwrap_err()
        .to_string()
        .contains("overlaps private"));
    let nested = f.store.path().parent().unwrap().join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("private.txt"), "private").unwrap();
    let (_, inside) = f
        .store
        .authorize_working_copy(&f.handle, &nested, "req_private_attach")
        .unwrap();
    assert!(f
        .store
        .import_local_project(
            &f.handle,
            &LocalImportInput {
                locator_id: inside.locator_id,
                selected_root: nested,
                request_id: "req_private_import".into()
            }
        )
        .unwrap_err()
        .to_string()
        .contains("overlaps private"));
    assert!(f
        .store
        .project_sources(&f.handle.project_id)
        .unwrap()
        .is_empty());
}

#[test]
fn removed_source_stays_inactive_while_other_files_import() {
    let mut f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    f.import("req_first");
    let entry = f.entry("a.txt");
    let source = entry.source_id.clone().unwrap();
    let version = entry.source_version_id.clone().unwrap();
    f.handle = f.store.remove_project_source(&f.handle, &source).unwrap();
    fs::write(f.root.join("a.txt"), "changed removed content").unwrap();
    fs::write(f.root.join("b.txt"), "unrelated").unwrap();
    let receipt = f.import("req_after_remove");
    assert_eq!(receipt.retained_count, 1);
    assert_eq!(f.entry("b.txt").state, InventoryState::Retained);
    let omitted = f.entry("a.txt");
    assert_eq!(omitted.state, InventoryState::Omitted);
    assert_eq!(omitted.omission_reason.as_deref(), Some("removed-source"));
    assert_eq!(omitted.source_id, Some(source.clone()));
    assert_eq!(omitted.source_version_id, Some(version.clone()));
    assert_eq!(omitted.observed_bytes, None);
    assert_eq!(
        f.store
            .source_versions(&f.handle.project_id, &source)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        f.store
            .source_version_bytes(&f.handle.project_id, &source, &version)
            .unwrap(),
        b"first"
    );
    f.import("req_again");
    assert_eq!(f.entry("a.txt"), omitted);
    fs::remove_file(f.root.join("a.txt")).unwrap();
    fs::write(f.root.join("renamed.txt"), "first").unwrap();
    let receipt = f.import("req_renamed_removed");
    assert!(receipt.renamed.is_empty());
    assert_ne!(
        f.entry("renamed.txt").source_id.as_deref(),
        Some(source.as_str())
    );
    assert_eq!(
        f.store
            .project_sources(&f.handle.project_id)
            .unwrap()
            .into_iter()
            .find(|s| s.source_id == source)
            .unwrap()
            .state,
        ley_core::ProjectSourceState::Removed
    );
}

#[test]
fn combined_inventory_bound_preserves_previous_observation() {
    let f = Fixture::new();
    fs::write(f.root.join(".env"), "excluded").unwrap();
    let first = f.import("req_first");
    let mut conn = rusqlite::Connection::open(f.store.path()).unwrap();
    let tx = conn.transaction().unwrap();
    let mut entry = f.entry(".env");
    entry.state = InventoryState::Missing;
    for i in 1..MAX_IMPORT_ENTRIES {
        entry.relative_path = format!("old-{i}.txt");
        tx.execute(
            "INSERT INTO working_copy_inventory(project_id,locator_id,relative_path,entry_json)
            VALUES(?1,?2,?3,?4)",
            rusqlite::params![
                f.handle.project_id,
                f.locator,
                entry.relative_path,
                serde_json::to_string(&entry).unwrap()
            ],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    fs::remove_file(f.root.join(".env")).unwrap();
    fs::write(f.root.join("new.txt"), "new retained bytes").unwrap();
    let error = f
        .store
        .import_local_project(&f.handle, &f.input("req_overflow"))
        .unwrap_err();
    assert!(
        error.to_string().contains("combined inventory bound"),
        "{error}"
    );
    let contents = f
        .store
        .project_contents(&f.handle.project_id, &f.locator)
        .unwrap();
    assert_eq!(contents.last_success.unwrap(), first);
    assert_eq!(contents.entries.len(), MAX_IMPORT_ENTRIES);
    assert_eq!(contents.latest_attempt.unwrap().status, "failed");
    assert!(f
        .store
        .project_sources(&f.handle.project_id)
        .unwrap()
        .is_empty());
}

#[test]
fn pre_m2_locator_without_directory_identity_requires_explicit_reattachment() {
    let f = Fixture::new();
    let conn = rusqlite::Connection::open(f.store.path()).unwrap();
    conn.execute(
        "UPDATE working_copy_locators SET root_identity=NULL WHERE project_id=?1 AND locator_id=?2",
        rusqlite::params![f.handle.project_id, f.locator],
    )
    .unwrap();
    let error = f
        .store
        .import_local_project(&f.handle, &f.input("req_identity_missing"))
        .unwrap_err();
    assert!(
        error.to_string().contains("explicitly attach again"),
        "{error}"
    );
    assert!(f.entries().is_empty());
}

#[test]
fn populated_v12_migration_preserves_m1_identity_and_exact_history() {
    let f = Fixture::new();
    let source = f
        .store
        .create_project_source(&f.handle, "text", "M1 reference", "req_m1_source")
        .unwrap();
    let input = SourceVersionInput {
        request_id: "req_m1_version".into(),
        representation_kind: "utf8-text".into(),
        original_content_hash: None,
        source_bytes: 12,
        transformation: json!({"kind":"exact"}),
        retained_bytes: b"M1 retained\n".to_vec(),
    };
    let version = f
        .store
        .retain_source_version(&f.handle, &source.source_id, &input)
        .unwrap();
    let before = f.store.open_project_brain(&f.handle.project_id).unwrap();
    let locator = f.store.working_copies(&f.handle.project_id).unwrap();
    let conn = rusqlite::Connection::open(f.store.path()).unwrap();
    let events: String = conn
        .query_row(
            "SELECT json_group_array(json(payload_json)) FROM events WHERE project_id=?1",
            [&f.handle.project_id],
            |r| r.get(0),
        )
        .unwrap();
    conn.execute_batch(
        "DROP TABLE working_copy_import_heads;
        DROP TABLE working_copy_inventory;
        DROP TABLE import_source_paths;
        DROP TABLE import_erasure_fences;
        DROP TABLE project_import_attempts;
        ALTER TABLE working_copy_locators DROP COLUMN root_identity;
        PRAGMA user_version=12;",
    )
    .unwrap();
    drop(conn);
    assert_eq!(f.store.schema_version().unwrap(), 13);
    assert_eq!(
        f.store.open_project_brain(&f.handle.project_id).unwrap(),
        before
    );
    assert_eq!(
        f.store.working_copies(&f.handle.project_id).unwrap(),
        locator
    );
    assert_eq!(
        f.store
            .source_version_bytes(
                &f.handle.project_id,
                &source.source_id,
                &version.source_version_id
            )
            .unwrap(),
        input.retained_bytes
    );
    let conn = rusqlite::Connection::open(f.store.path()).unwrap();
    let after: String = conn
        .query_row(
            "SELECT json_group_array(json(payload_json)) FROM events WHERE project_id=?1",
            [&f.handle.project_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(events, after);
    let identity: Option<String> = conn
        .query_row(
            "SELECT root_identity FROM working_copy_locators WHERE project_id=?1 AND locator_id=?2",
            rusqlite::params![f.handle.project_id, f.locator],
            |r| r.get(0),
        )
        .unwrap();
    assert!(identity.is_none());
    let violations: i64 = conn
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(violations, 0);
    drop(conn);
    assert!(f
        .store
        .import_local_project(&f.handle, &f.input("req_migrated_import"))
        .is_err());
}

#[test]
fn whole_brain_erasure_cascades_import_state_without_touching_originals() {
    let mut f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    f.import("req_first");
    let source = f.entry("a.txt").source_id.unwrap();
    f.handle = f.store.erase_project_source(&f.handle, &source).unwrap();
    fs::write(f.root.join("b.txt"), "second").unwrap();
    f.import("req_with_fences");
    f.store.erase_project(&f.handle.project_id).unwrap();
    let conn = rusqlite::Connection::open(f.store.path()).unwrap();
    for table in [
        "project_import_attempts",
        "working_copy_import_heads",
        "working_copy_inventory",
        "import_source_paths",
        "import_erasure_fences",
        "working_copy_locators",
        "source_versions",
        "project_sources",
    ] {
        let count: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE project_id=?1"),
                [&f.handle.project_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    assert_eq!(fs::read(f.root.join("a.txt")).unwrap(), b"first");
    assert_eq!(fs::read(f.root.join("b.txt")).unwrap(), b"second");
    assert!(f
        .store
        .import_local_project(&f.handle, &f.input("req_after_erasure"))
        .is_err());
}

#[test]
fn removed_ignore_source_still_filters_unrelated_imports_without_reactivation() {
    let mut f = Fixture::new();
    fs::write(f.root.join(".gitignore"), "secret.txt\n").unwrap();
    f.import("req_first");
    let source = f.entry(".gitignore").source_id.unwrap();
    f.handle = f.store.remove_project_source(&f.handle, &source).unwrap();
    fs::write(f.root.join("secret.txt"), "excluded").unwrap();
    fs::write(f.root.join("b.txt"), "unrelated").unwrap();
    f.import("req_after_remove");
    assert_eq!(
        f.entry(".gitignore").omission_reason.as_deref(),
        Some("removed-source")
    );
    assert_eq!(
        f.entry("secret.txt").omission_reason.as_deref(),
        Some("ignored")
    );
    assert_eq!(f.entry("b.txt").state, InventoryState::Retained);
    assert_eq!(
        f.store
            .source_versions(&f.handle.project_id, &source)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn contents_reads_one_observation_during_concurrent_publication() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let f = Fixture::new();
    fs::write(f.root.join("a.txt"), "first").unwrap();
    f.import("req_first");
    let store = f.store.clone();
    let handle = f.handle.clone();
    let input = f.input("req_writer");
    let running = Arc::new(AtomicBool::new(true));
    let writer_running = running.clone();
    let writer = std::thread::spawn(move || {
        for i in 0..24 {
            let path = input.selected_root.join("b.txt");
            if i % 2 == 0 {
                fs::write(&path, "second").unwrap();
            } else {
                fs::remove_file(&path).unwrap();
            }
            let mut input = input.clone();
            input.request_id = format!("req_writer_{i}");
            store.import_local_project(&handle, &input).unwrap();
        }
        writer_running.store(false, Ordering::Release);
    });
    let mut reads = 0;
    while running.load(Ordering::Acquire) || reads == 0 {
        let contents = f
            .store
            .project_contents(&f.handle.project_id, &f.locator)
            .unwrap();
        let receipt = contents.last_success.unwrap();
        assert_eq!(
            receipt.retained_count,
            contents
                .entries
                .iter()
                .filter(|e| e.state == InventoryState::Retained)
                .count()
        );
        assert_eq!(
            receipt.omitted_count,
            contents
                .entries
                .iter()
                .filter(|e| e.state == InventoryState::Omitted)
                .count()
        );
        assert_eq!(receipt.project_generation, contents.current_generation);
        reads += 1;
    }
    writer.join().unwrap();
    assert!(reads > 0);
}

#[cfg(unix)]
#[test]
fn attachment_never_canonicalizes_symlink_roots_into_authority() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let outside = f.temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("canary.txt"), "OUTSIDE").unwrap();
    let link = f.temp.path().join("selected-link");
    symlink(&outside, &link).unwrap();
    assert!(f
        .store
        .authorize_working_copy(&f.handle, &link, "req_link_attach")
        .is_err());
    let parent_link = f.temp.path().join("parent-link");
    symlink(f.temp.path(), &parent_link).unwrap();
    assert!(f
        .store
        .authorize_working_copy(
            &f.handle,
            parent_link.join("outside"),
            "req_ancestor_attach"
        )
        .is_err());
    let copies = f.store.working_copies(&f.handle.project_id).unwrap();
    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0].local_path, f.root);
}

#[cfg(all(unix, target_os = "linux"))]
#[test]
fn git_observation_never_executes_repository_content_filters() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    let f = Fixture::new();
    let marker = f.temp.path().join("filter-invoked");
    let script = f.temp.path().join("filter.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\nprintf invoked > '{}'\ncat\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let run = |args: &[&str]| {
        let result = Command::new("git")
            .args(args)
            .current_dir(&f.root)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    run(&["init", "-q"]);
    fs::write(
        f.root.join(".gitattributes"),
        "*.txt filter=trap diff=trap\n",
    )
    .unwrap();
    fs::write(f.root.join("a.txt"), "first\n").unwrap();
    run(&["config", "filter.trap.clean", script.to_str().unwrap()]);
    run(&["add", "."]);
    run(&[
        "-c",
        "user.name=Proof",
        "-c",
        "user.email=proof@example.invalid",
        "commit",
        "-qm",
        "Seed",
    ]);
    assert!(marker.exists());
    fs::remove_file(&marker).unwrap();
    run(&["config", "diff.external", script.to_str().unwrap()]);
    run(&["config", "diff.trap.textconv", script.to_str().unwrap()]);
    run(&["config", "core.fsmonitor", script.to_str().unwrap()]);
    fs::write(f.root.join("a.txt"), "other\n").unwrap();
    let receipt = f.import("req_filter_safe");
    assert!(
        !marker.exists(),
        "import executed repository-controlled code"
    );
    assert_eq!(receipt.git["kind"], "observed");
    assert_eq!(receipt.git["dirty"], true);
    f.import("req_filter_safe_again");
    assert!(!marker.exists());
}
