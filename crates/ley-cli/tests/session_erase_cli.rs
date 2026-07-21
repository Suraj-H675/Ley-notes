use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::tempdir;

fn run_ley(config: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ley"))
        .env("XDG_CONFIG_HOME", config)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

fn ley(config: &Path, arguments: &[&str]) -> Output {
    let output = run_ley(config, arguments);
    assert!(
        output.status.success(),
        "ley {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn json_stdout(output: Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn local_cli_erases_one_session_and_its_learning_but_preserves_user_files() {
    let base = tempdir().unwrap();
    let project = base.path().join("project");
    let vault = base.path().join("vault");
    let config = base.path().join("config");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&vault).unwrap();
    fs::write(project.join("README.md"), "# Keep this project\n").unwrap();

    ley(
        &config,
        &["init", project.to_str().unwrap(), "--name", "CLI erasure"],
    );
    ley(
        &config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            vault.to_str().unwrap(),
        ],
    );
    ley(&config, &["ingest", project.to_str().unwrap()]);
    let started = json_stdout(ley(
        &config,
        &[
            "session",
            "start",
            project.to_str().unwrap(),
            "--name",
            "Forget this private session",
            "--goal",
            "Prove physical per-session deletion",
            "--json",
        ],
    ));
    let session_id = started["session"]["sessionId"].as_str().unwrap();
    let evidence = format!("{session_id}:{session_id}");
    let proposed = json_stdout(ley(
        &config,
        &[
            "learning",
            "propose",
            project.to_str().unwrap(),
            "--actor",
            "agent",
            "--provenance",
            "inferred",
            "--kind",
            "procedure",
            "--title",
            "Private derived lesson",
            "--guidance",
            "This text must disappear with its source session.",
            "--confidence",
            "80",
            "--evidence",
            &evidence,
            "--json",
        ],
    ));
    let learning_id = proposed["learning"]["learningId"].as_str().unwrap();

    let note = vault.join("Session handoff.md");
    let canvas = vault.join("Project.canvas");
    fs::write(&note, "# User-owned copy\n\nForget this private session\n").unwrap();
    fs::write(
        &canvas,
        format!(
            r#"{{"nodes":[{{"id":"copy","type":"file","file":"Session handoff.md","x":0,"y":0,"width":320,"height":180}}],"edges":[],"session":"{session_id}"}}"#
        ),
    )
    .unwrap();

    let stale = run_ley(
        &config,
        &[
            "session",
            "erase",
            session_id,
            project.to_str().unwrap(),
            "--confirm-name",
            "forget this private session",
            "--expected-events",
            "1",
            "--json",
        ],
    );
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("type the current name"));

    let erased = json_stdout(ley(
        &config,
        &[
            "session",
            "erase",
            session_id,
            project.to_str().unwrap(),
            "--confirm-name",
            "Forget this private session",
            "--expected-events",
            "1",
            "--json",
        ],
    ));
    assert_eq!(erased["sessionId"], session_id);
    assert_eq!(erased["erasedLearningIds"][0], learning_id);
    assert_eq!(
        json_stdout(ley(
            &config,
            &["session", "list", project.to_str().unwrap(), "--json"],
        )),
        serde_json::json!([])
    );
    assert_eq!(
        json_stdout(ley(
            &config,
            &["learning", "list", project.to_str().unwrap(), "--json"],
        )),
        serde_json::json!([])
    );

    ley(&config, &["graph", project.to_str().unwrap(), "--json"]);
    assert_eq!(
        fs::read_to_string(project.join("README.md")).unwrap(),
        "# Keep this project\n"
    );
    assert!(project.join(".ley/project.json").is_file());
    assert!(note.is_file());
    assert!(canvas.is_file());
    let retained_user_text = format!(
        "{}\n{}",
        fs::read_to_string(note).unwrap(),
        fs::read_to_string(canvas).unwrap()
    );
    assert!(retained_user_text.contains(session_id));

    let private_store_text = walk_text(&vault.join(".ley/agent-memory"));
    assert!(!private_store_text.contains("Forget this private session"));
    assert!(!private_store_text.contains("Private derived lesson"));
    assert!(!private_store_text.contains(session_id));
    assert!(!private_store_text.contains(learning_id));
}

fn walk_text(directory: &Path) -> String {
    let mut text = String::new();
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            text.push_str(&walk_text(&path));
        } else if let Ok(body) = fs::read_to_string(path) {
            text.push_str(&body);
        }
    }
    text
}
