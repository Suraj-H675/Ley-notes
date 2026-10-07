use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
fn run(config: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .env("XDG_DATA_HOME", config.join("data"))
        .env("XDG_CACHE_HOME", config.join("cache"))
        .args(args)
        .output()
        .unwrap()
}
fn json(config: &Path, args: &[&str]) -> Value {
    let output = run(config, args);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn explicit_brain_workflow_reads_exact_retained_bytes_and_safe_move() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config");
    let root = temp.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("README.md"), "# first\n").unwrap();
    let brain = json(
        &config,
        &[
            "brain",
            "create",
            "--name",
            "CLI Brain",
            "--request",
            "req_create",
            "--json",
        ],
    );
    let project = brain["identity"]["projectId"].as_str().unwrap();
    let attach = json(
        &config,
        &[
            "brain",
            "attach",
            "--project",
            project,
            "--root",
            root.to_str().unwrap(),
            "--request",
            "req_attach",
        ],
    );
    let locator = attach["locator"]["locatorId"].as_str().unwrap();
    let import = json(
        &config,
        &[
            "brain",
            "import",
            "--project",
            project,
            "--locator",
            locator,
            "--root",
            root.to_str().unwrap(),
            "--request",
            "req_import",
        ],
    );
    assert_eq!(import["retainedCount"], 1);
    let contents = json(
        &config,
        &[
            "brain",
            "contents",
            "--project",
            project,
            "--locator",
            locator,
        ],
    );
    let entry = &contents["entries"][0];
    let source = entry["sourceId"].as_str().unwrap();
    let version = entry["sourceVersionId"].as_str().unwrap();
    fs::write(root.join("README.md"), "live changed\n").unwrap();
    let raw = run(
        &config,
        &[
            "brain",
            "version",
            "--project",
            project,
            "--source",
            source,
            "--version",
            version,
            "--raw",
        ],
    );
    assert!(raw.status.success());
    assert_eq!(raw.stdout, b"# first\n");
    let versions = json(
        &config,
        &[
            "brain",
            "versions",
            "--project",
            project,
            "--source",
            source,
        ],
    );
    assert_eq!(versions.as_array().unwrap().len(), 1);
    assert!(versions[0].get("bytes").is_none());
    assert_eq!(
        versions[0]["occurrences"][0]["import"]["scanId"],
        import["scanId"]
    );
    let changed = json(
        &config,
        &[
            "brain",
            "import",
            "--project",
            project,
            "--locator",
            locator,
            "--root",
            root.to_str().unwrap(),
            "--request",
            "req_changed",
        ],
    );
    assert_eq!(changed["modified"][0], "README.md");
    let moved = temp.path().join("moved");
    fs::rename(&root, &moved).unwrap();
    let movement = json(
        &config,
        &[
            "brain",
            "move-locator",
            "--project",
            project,
            "--locator",
            locator,
            "--root",
            moved.to_str().unwrap(),
            "--request",
            "req_move",
        ],
    );
    assert!(movement["generation"].as_u64().unwrap() > brain["generation"].as_u64().unwrap());
    json(
        &config,
        &[
            "brain",
            "import",
            "--project",
            project,
            "--locator",
            locator,
            "--root",
            moved.to_str().unwrap(),
            "--request",
            "req_after_move",
        ],
    );
    let moved_contents = json(
        &config,
        &[
            "brain",
            "contents",
            "--project",
            project,
            "--locator",
            locator,
        ],
    );
    assert_eq!(moved_contents["entries"][0]["sourceId"], source);
    assert_eq!(
        json(&config, &["brain", "list"])[0]["identity"]["projectId"],
        project
    );
}
#[test]
fn copied_marker_and_wrong_root_never_authorize_import() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config");
    let root = temp.path().join("root");
    let clone = temp.path().join("clone");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&clone).unwrap();
    let brain = json(
        &config,
        &[
            "brain",
            "create",
            "--name",
            "CLI Brain",
            "--request",
            "req_create",
        ],
    );
    let project = brain["identity"]["projectId"].as_str().unwrap();
    fs::create_dir(clone.join(".ley")).unwrap();
    fs::write(
        clone.join(".ley/project.json"),
        serde_json::to_vec(&brain["identity"]).unwrap(),
    )
    .unwrap();
    let attach = json(
        &config,
        &[
            "brain",
            "attach",
            "--project",
            project,
            "--root",
            root.to_str().unwrap(),
            "--request",
            "req_attach",
        ],
    );
    let locator = attach["locator"]["locatorId"].as_str().unwrap();
    assert!(!run(
        &config,
        &[
            "brain",
            "import",
            "--project",
            project,
            "--locator",
            locator,
            "--root",
            clone.to_str().unwrap(),
            "--request",
            "req_bad"
        ]
    )
    .status
    .success());
    assert!(!run(
        &config,
        &[
            "brain",
            "move-locator",
            "--project",
            project,
            "--locator",
            locator,
            "--root",
            clone.to_str().unwrap(),
            "--request",
            "req_bad_move"
        ]
    )
    .status
    .success());
    assert!(!run(
        &config,
        &[
            "brain",
            "import",
            "--project",
            project,
            "--root",
            root.to_str().unwrap(),
            "--request",
            "req_no_locator"
        ]
    )
    .status
    .success());
}
