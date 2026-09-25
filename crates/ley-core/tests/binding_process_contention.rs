use ley_core::{initialize_project, BindingRegistry, CaptureMode, PROJECT_CATALOG_FILE};
use serde_json::Value;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

const REGISTRY_ENV: &str = "LEY_BINDING_CONTENTION_REGISTRY";
const PROJECT_ENV: &str = "LEY_BINDING_CONTENTION_PROJECT";
const VAULT_ENV: &str = "LEY_BINDING_CONTENTION_VAULT";
const GATE_ENV: &str = "LEY_BINDING_CONTENTION_GATE";
const WORKERS: usize = 8;

#[test]
fn binding_process_contention_worker() {
    let Some(registry) = env::var_os(REGISTRY_ENV) else {
        return;
    };
    let project = PathBuf::from(env::var_os(PROJECT_ENV).expect("worker project is configured"));
    let vault = PathBuf::from(env::var_os(VAULT_ENV).expect("worker vault is configured"));
    let gate = PathBuf::from(env::var_os(GATE_ENV).expect("worker gate is configured"));

    let deadline = Instant::now() + Duration::from_secs(10);
    while !gate.exists() {
        assert!(
            Instant::now() < deadline,
            "worker start gate was never released"
        );
        thread::sleep(Duration::from_millis(2));
    }

    BindingRegistry::at(registry)
        .bind(&project, &vault)
        .expect("cross-process binding succeeds");
}

#[test]
fn separate_process_bindings_preserve_binding_and_catalog_state() {
    let base = tempdir().unwrap();
    let registry_path = base.path().join("private/bindings-v1.json");
    let gate = base.path().join("start-gate");
    let executable = env::current_exe().expect("integration-test executable is available");
    let mut fixtures = Vec::with_capacity(WORKERS);
    let mut children = Vec::with_capacity(WORKERS);

    for index in 0..WORKERS {
        let project = base.path().join(format!("project-{index}"));
        let vault = base.path().join(format!("vault-{index}"));
        fs::create_dir(&project).unwrap();
        fs::create_dir(&vault).unwrap();
        initialize_project(&project, None, CaptureMode::Structured).unwrap();
        let output = base.path().join(format!("worker-{index}.log"));
        let output_file = File::create(&output).unwrap();
        let stderr_file = output_file.try_clone().unwrap();
        let child = Command::new(&executable)
            .args([
                "--exact",
                "binding_process_contention_worker",
                "--nocapture",
            ])
            .env(REGISTRY_ENV, &registry_path)
            .env(PROJECT_ENV, &project)
            .env(VAULT_ENV, &vault)
            .env(GATE_ENV, &gate)
            .stdin(Stdio::null())
            .stdout(Stdio::from(output_file))
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .unwrap();
        fixtures.push((project, vault));
        children.push((index, child, output));
    }

    fs::write(&gate, b"go\n").unwrap();
    wait_for_workers(&mut children, Duration::from_secs(20));

    let registry = BindingRegistry::at(&registry_path);
    for (project, vault) in &fixtures {
        let binding = registry.resolve(project, None).unwrap();
        assert_eq!(binding.vault_path, vault.canonicalize().unwrap());
    }

    let binding_body = fs::read_to_string(&registry_path).unwrap();
    let binding_document: Value = serde_json::from_str(&binding_body).unwrap();
    assert_eq!(binding_document["schemaVersion"], 1);
    assert_eq!(
        binding_document["bindings"].as_object().unwrap().len(),
        WORKERS
    );
    assert!(binding_body.ends_with('\n'));

    let catalog_path = registry_path.with_file_name(PROJECT_CATALOG_FILE);
    let catalog_body = fs::read_to_string(&catalog_path).unwrap();
    let catalog_document: Value = serde_json::from_str(&catalog_body).unwrap();
    assert_eq!(catalog_document["schemaVersion"], 1);
    assert_eq!(
        catalog_document["projects"].as_object().unwrap().len(),
        WORKERS
    );
    assert!(catalog_body.ends_with('\n'));
}

fn wait_for_workers(children: &mut [(usize, Child, PathBuf)], timeout: Duration) {
    let deadline = Instant::now() + timeout;
    let mut remaining = children.len();
    let mut finished = vec![false; children.len()];

    while remaining > 0 && Instant::now() < deadline {
        for (slot, (index, child, output)) in children.iter_mut().enumerate() {
            if finished[slot] {
                continue;
            }
            let Some(status) = child.try_wait().unwrap() else {
                continue;
            };
            finished[slot] = true;
            remaining -= 1;
            if !status.success() {
                let detail = fs::read_to_string(output).unwrap_or_default();
                panic!("binding worker {index} failed with {status}: {detail}");
            }
        }
        if remaining > 0 {
            thread::sleep(Duration::from_millis(5));
        }
    }

    if remaining == 0 {
        return;
    }
    let mut unfinished = Vec::new();
    for (slot, (index, child, output)) in children.iter_mut().enumerate() {
        if finished[slot] {
            continue;
        }
        unfinished.push(*index);
        let _ = child.kill();
        let _ = child.wait();
        let detail = fs::read_to_string(output).unwrap_or_default();
        if !detail.is_empty() {
            eprintln!("binding worker {index} output before timeout: {detail}");
        }
    }
    panic!("binding workers timed out: {unfinished:?}");
}
