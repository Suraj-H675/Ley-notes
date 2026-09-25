use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

const EVAL_PRIVATE_ROOT_ENV: &str = "LEY_EVAL_PRIVATE_ROOT";

#[cfg(unix)]
fn make_private(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(not(unix))]
fn make_private(_path: &Path) {}

fn create_private_root(path: &Path) {
    fs::create_dir_all(path.join("config")).unwrap();
    fs::create_dir_all(path.join("cache")).unwrap();
    make_private(path);
    make_private(&path.join("config"));
    make_private(&path.join("cache"));
}

fn run_ley(private_root: &Path, decoy_config: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ley"))
        .env(EVAL_PRIVATE_ROOT_ENV, private_root)
        .env("XDG_CONFIG_HOME", decoy_config)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

#[cfg(feature = "eval-private-root")]
#[test]
fn cli_eval_private_root_isolates_binding_and_egress_authority() {
    let base = tempdir().unwrap();
    let private_root = base.path().join("private-state");
    let decoy_config = base.path().join("decoy-config");
    let project = base.path().join("project");
    let vault = base.path().join("vault");
    create_private_root(&private_root);
    fs::create_dir_all(&decoy_config).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&vault).unwrap();
    fs::write(project.join("README.md"), "# Eval private root\n").unwrap();

    for arguments in [
        vec!["init", project.to_str().unwrap(), "--json"],
        vec![
            "bind",
            project.to_str().unwrap(),
            "--vault",
            vault.to_str().unwrap(),
            "--json",
        ],
        vec![
            "egress",
            "project",
            "local-model-only",
            project.to_str().unwrap(),
            "--json",
        ],
    ] {
        let output = run_ley(&private_root, &decoy_config, &arguments);
        assert!(
            output.status.success(),
            "ley {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let application_config = private_root.join("config/app.leynotes.desktop");
    assert!(application_config.join("bindings-v1.json").is_file());
    assert!(application_config.join("agent-egress-v1.json").is_file());
    assert!(!decoy_config.join("app.leynotes.desktop").exists());

    let model = ley_core::supported_semantic_model();
    let model_directory = private_root
        .join("cache/ley/models/minishlab--potion-retrieval-32m")
        .join(&model.revision);
    fs::create_dir_all(&model_directory).unwrap();
    make_private(&private_root.join("cache/ley"));
    make_private(&private_root.join("cache/ley/models"));
    make_private(&private_root.join("cache/ley/models/minishlab--potion-retrieval-32m"));
    make_private(&model_directory);
    let semantic = run_ley(
        &private_root,
        &decoy_config,
        &["semantic", "status", "--json"],
    );
    assert!(semantic.status.success());
    let semantic: serde_json::Value = serde_json::from_slice(&semantic.stdout).unwrap();
    assert_eq!(semantic["state"], "corrupt");
}

#[cfg(not(feature = "eval-private-root"))]
#[test]
fn ordinary_cli_rejects_eval_private_root_instead_of_silently_ignoring_it() {
    let base = tempdir().unwrap();
    let private_root = base.path().join("private-state");
    let decoy_config = base.path().join("decoy-config");
    let project = base.path().join("project");
    let vault = base.path().join("vault");
    create_private_root(&private_root);
    fs::create_dir_all(&decoy_config).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&vault).unwrap();
    ley_core::initialize_project(&project, None, ley_core::CaptureMode::Structured).unwrap();

    let output = run_ley(
        &private_root,
        &decoy_config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            vault.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "LEY_EVAL_PRIVATE_ROOT is accepted only by builds with the eval-private-root feature"
    ));
    assert!(!decoy_config.join("app.leynotes.desktop").exists());
}
