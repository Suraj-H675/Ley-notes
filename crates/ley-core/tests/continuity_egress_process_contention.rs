use ley_core::{
    initialize_project, AgentEgressPolicy, AgentEgressTarget, CaptureMode, ContinuityEventInput,
    ContinuityStore, EgressPolicyRegistry, LeyCoreError,
};
use serde_json::json;
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

const DATABASE_ENV: &str = "LEY_CONTINUITY_EGRESS_DATABASE";
const PROJECT_ID_ENV: &str = "LEY_CONTINUITY_EGRESS_PROJECT_ID";
const HELD_ENV: &str = "LEY_CONTINUITY_EGRESS_HELD";
const RELEASE_ENV: &str = "LEY_CONTINUITY_EGRESS_RELEASE";
const REVOKER_CONTENDED_ENV: &str = "LEY_CONTINUITY_EGRESS_REVOKER_CONTENDED";
const REGISTRY_ENV: &str = "LEY_CONTINUITY_EGRESS_REGISTRY";
const PROJECT_ROOT_ENV: &str = "LEY_CONTINUITY_EGRESS_PROJECT_ROOT";
const TRANSITION_CONTENDED_ENV: &str = "LEY_CONTINUITY_EGRESS_TRANSITION_CONTENDED";

#[test]
fn continuity_egress_lock_holder_worker() {
    let Some(database) = env::var_os(DATABASE_ENV) else {
        return;
    };
    let project_id = env::var(PROJECT_ID_ENV).expect("worker project ID is configured");
    let held = PathBuf::from(env::var_os(HELD_ENV).expect("held marker is configured"));
    let release = PathBuf::from(env::var_os(RELEASE_ENV).expect("release marker is configured"));
    let store = ContinuityStore::at(database);

    store
        .with_project_egress_locked(&project_id, AgentEgressTarget::Cloud, || {
            fs::write(&held, b"held\n").unwrap();
            wait_for_path(&release, Duration::from_secs(10));
            Ok(())
        })
        .expect("allowed egress operation completes after release");
}

#[test]
fn continuity_egress_revoker_worker() {
    let Some(database) = env::var_os(DATABASE_ENV) else {
        return;
    };
    let database = PathBuf::from(database);
    let project_id = env::var(PROJECT_ID_ENV).expect("worker project ID is configured");
    let contended = PathBuf::from(
        env::var_os(REVOKER_CONTENDED_ENV).expect("revoker-contended marker is configured"),
    );
    let probe = File::open(database.with_file_name("continuity-egress.lock")).unwrap();
    assert!(matches!(
        probe.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    fs::write(&contended, b"contended\n").unwrap();

    ContinuityStore::at(database)
        .set_project_egress_policy(&project_id, AgentEgressPolicy::NeverSend)
        .expect("revocation succeeds after the in-flight operation releases authority");
}

#[test]
fn legacy_egress_lock_holder_worker() {
    let Some(registry) = env::var_os(REGISTRY_ENV) else {
        return;
    };
    let project = PathBuf::from(env::var_os(PROJECT_ROOT_ENV).expect("project root is configured"));
    let held = PathBuf::from(env::var_os(HELD_ENV).expect("held marker is configured"));
    let release = PathBuf::from(env::var_os(RELEASE_ENV).expect("release marker is configured"));
    EgressPolicyRegistry::at(registry)
        .with_project_egress_locked(&project, AgentEgressTarget::Cloud, || {
            fs::write(&held, b"held\n").unwrap();
            wait_for_path(&release, Duration::from_secs(10));
            Ok(())
        })
        .expect("legacy protected operation completes after release");
}

#[test]
fn transition_project_policy_setter_worker() {
    let Some(database) = env::var_os(DATABASE_ENV) else {
        return;
    };
    let registry = PathBuf::from(env::var_os(REGISTRY_ENV).expect("registry is configured"));
    let project = PathBuf::from(env::var_os(PROJECT_ROOT_ENV).expect("project root is configured"));
    let contended = PathBuf::from(
        env::var_os(TRANSITION_CONTENDED_ENV).expect("transition contention marker is configured"),
    );
    let probe = File::open(registry.with_file_name("agent-egress-v1.lock")).unwrap();
    assert!(matches!(
        probe.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    fs::write(&contended, b"contended\n").unwrap();

    EgressPolicyRegistry::at(registry)
        .set_project_policy_transition(
            &project,
            &ContinuityStore::at(database),
            AgentEgressPolicy::NeverSend,
        )
        .expect("transition setter converges after legacy operation releases authority");
}

#[test]
fn separate_process_revocation_waits_for_egress_without_blocking_continuity_writes() {
    let base = tempdir().unwrap();
    let project = base.path().join("project");
    fs::create_dir(&project).unwrap();
    let initialized =
        initialize_project(&project, Some("Egress lock"), CaptureMode::Structured).unwrap();
    let project_id = initialized.identity.project_id.clone();
    let database = base.path().join("private/continuity.sqlite3");
    let store = ContinuityStore::at(&database);
    store.register_project(&initialized.identity).unwrap();
    store
        .set_project_egress_policy(&project_id, AgentEgressPolicy::AgentOk)
        .unwrap();

    let held = base.path().join("held");
    let release = base.path().join("release");
    let revoker_contended = base.path().join("revoker-contended");
    let executable = env::current_exe().expect("integration-test executable is available");

    let mut holder = spawn_worker(
        &executable,
        "continuity_egress_lock_holder_worker",
        &database,
        &project_id,
        &[(HELD_ENV, held.as_path()), (RELEASE_ENV, release.as_path())],
        base.path().join("holder.log"),
    );
    wait_for_path(&held, Duration::from_secs(10));

    let mut revoker = spawn_worker(
        &executable,
        "continuity_egress_revoker_worker",
        &database,
        &project_id,
        &[(REVOKER_CONTENDED_ENV, revoker_contended.as_path())],
        base.path().join("revoker.log"),
    );
    wait_for_path(&revoker_contended, Duration::from_secs(10));
    assert!(
        revoker.child.try_wait().unwrap().is_none(),
        "revocation must still be pending after the revoker observed the held authority lock"
    );

    let write_started = Instant::now();
    store
        .append_event(&ContinuityEventInput {
            event_id: "evt_parallel_continuity_write".to_owned(),
            project_id: project_id.clone(),
            subject_id: None,
            session_id: None,
            session_sequence: None,
            request_id: None,
            request_fingerprint: None,
            kind: "native-lock-probe".to_owned(),
            payload_version: 1,
            recorded_at_unix_ms: 1,
            revision_head: None,
            revision_branch: None,
            payload: json!({"whileEgressLockHeld": true}),
        })
        .unwrap();
    assert!(
        write_started.elapsed() < Duration::from_secs(2),
        "the dedicated egress file lock must not hold SQLite's writer lock"
    );
    assert!(revoker.child.try_wait().unwrap().is_none());

    fs::write(&release, b"release\n").unwrap();
    holder.wait(Duration::from_secs(10));
    revoker.wait(Duration::from_secs(10));

    assert_eq!(
        store.project_egress_policy(&project_id).unwrap(),
        AgentEgressPolicy::NeverSend
    );
    assert!(matches!(
        store.with_project_egress_locked(&project_id, AgentEgressTarget::Cloud, || Ok(())),
        Err(LeyCoreError::AgentEgressDenied { .. })
    ));
}

#[test]
fn transition_setter_waits_for_legacy_reader_and_converges_both_stores() {
    let base = tempdir().unwrap();
    let project = base.path().join("project");
    fs::create_dir(&project).unwrap();
    let initialized = initialize_project(
        &project,
        Some("Mixed-version lock"),
        CaptureMode::Structured,
    )
    .unwrap();
    let project_id = initialized.identity.project_id.clone();
    let registry_path = base.path().join("private/egress.json");
    let database = base.path().join("private/continuity.sqlite3");
    let registry = EgressPolicyRegistry::at(&registry_path);
    let store = ContinuityStore::at(&database);
    registry
        .set_project_policy_transition(&project, &store, AgentEgressPolicy::AgentOk)
        .unwrap();

    let held = base.path().join("legacy-held");
    let release = base.path().join("legacy-release");
    let contended = base.path().join("transition-contended");
    let executable = env::current_exe().expect("integration-test executable is available");

    let mut holder = spawn_worker(
        &executable,
        "legacy_egress_lock_holder_worker",
        &database,
        &project_id,
        &[
            (REGISTRY_ENV, registry_path.as_path()),
            (PROJECT_ROOT_ENV, project.as_path()),
            (HELD_ENV, held.as_path()),
            (RELEASE_ENV, release.as_path()),
        ],
        base.path().join("legacy-holder.log"),
    );
    wait_for_path(&held, Duration::from_secs(10));

    let mut setter = spawn_worker(
        &executable,
        "transition_project_policy_setter_worker",
        &database,
        &project_id,
        &[
            (REGISTRY_ENV, registry_path.as_path()),
            (PROJECT_ROOT_ENV, project.as_path()),
            (TRANSITION_CONTENDED_ENV, contended.as_path()),
        ],
        base.path().join("transition-setter.log"),
    );
    wait_for_path(&contended, Duration::from_secs(10));
    assert!(setter.child.try_wait().unwrap().is_none());

    fs::write(&release, b"release\n").unwrap();
    holder.wait(Duration::from_secs(10));
    setter.wait(Duration::from_secs(10));

    assert_eq!(
        registry.list(&project).unwrap().project_policy,
        AgentEgressPolicy::NeverSend
    );
    assert_eq!(
        store.project_egress_policy(&project_id).unwrap(),
        AgentEgressPolicy::NeverSend
    );
    assert_eq!(
        registry
            .list_transition(&project, &store)
            .unwrap()
            .project_policy,
        AgentEgressPolicy::NeverSend
    );
}

struct Worker {
    child: Child,
    log: PathBuf,
}

impl Worker {
    fn wait(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                if !status.success() {
                    let detail = fs::read_to_string(&self.log).unwrap_or_default();
                    panic!("worker failed with {status}: {detail}");
                }
                return;
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                let detail = fs::read_to_string(&self.log).unwrap_or_default();
                panic!("worker timed out: {detail}");
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}

fn spawn_worker(
    executable: &Path,
    test_name: &str,
    database: &Path,
    project_id: &str,
    path_env: &[(&str, &Path)],
    log: PathBuf,
) -> Worker {
    let output = File::create(&log).unwrap();
    let stderr = output.try_clone().unwrap();
    let mut command = Command::new(executable);
    command
        .args(["--exact", test_name, "--nocapture"])
        .env(DATABASE_ENV, database)
        .env(PROJECT_ID_ENV, project_id)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(stderr));
    for (name, value) in path_env {
        command.env(name, value);
    }
    Worker {
        child: command.spawn().unwrap(),
        log,
    }
}

fn wait_for_path(path: &Path, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(5));
    }
}
