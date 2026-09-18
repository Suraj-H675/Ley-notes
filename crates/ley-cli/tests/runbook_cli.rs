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

fn create_confirmed_learning(
    config: &Path,
    project: &Path,
    evidence: (&str, &str),
    kind: &str,
    title: &str,
    guidance: &str,
    seed: &str,
) -> String {
    let (session_id, evidence_record_id) = evidence;
    let evidence = format!("{session_id}:{evidence_record_id}");
    let proposed = json_stdout(ley(
        config,
        &[
            "learning",
            "propose",
            project.to_str().unwrap(),
            "--actor",
            "agent",
            "--provenance",
            "agent-authored",
            "--kind",
            kind,
            "--title",
            title,
            "--guidance",
            guidance,
            "--confidence",
            "90",
            "--evidence",
            &evidence,
            "--request-id",
            &format!("req_{seed}0000000000000000"),
            "--json",
        ],
    ));
    let learning_id = proposed["learning"]["learningId"]
        .as_str()
        .unwrap()
        .to_owned();
    let reviewed = json_stdout(ley(
        config,
        &[
            "learning",
            "review",
            &learning_id,
            project.to_str().unwrap(),
            "--actor",
            "user",
            "--action",
            "confirm",
            "--note",
            "Reviewed for explicit runbook reuse.",
            "--request-id",
            &format!("req_{seed}1111111111111111"),
            "--json",
        ],
    ));
    assert_eq!(reviewed["learning"]["trustState"], "trusted");
    assert_eq!(reviewed["learning"]["freshness"], "current");
    learning_id
}

#[test]
fn reviewed_runbook_compile_and_skill_export_are_explicit_source_bound_and_non_installing() {
    let base = tempdir().unwrap();
    let project = base.path().join("project");
    let vault = base.path().join("vault");
    let config = base.path().join("config");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&vault).unwrap();
    fs::write(project.join("README.md"), "# Reviewed runbook CLI\n").unwrap();

    ley(
        &config,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Reviewed runbook CLI",
        ],
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
            "Review release operations",
            "--goal",
            "Confirm reusable release knowledge",
            "--json",
        ],
    ));
    let session_id = started["session"]["sessionId"].as_str().unwrap();
    ley(
        &config,
        &[
            "session",
            "checkpoint",
            session_id,
            project.to_str().unwrap(),
            "--summary",
            "Reviewed the release workflow against README.md.",
            "--touched",
            "README.md",
            "--json",
        ],
    );
    let shown = json_stdout(ley(
        &config,
        &[
            "session",
            "show",
            session_id,
            project.to_str().unwrap(),
            "--json",
        ],
    ));
    let checkpoint_id = shown["checkpoints"][0]["checkpointId"].as_str().unwrap();
    let procedure = create_confirmed_learning(
        &config,
        &project,
        (session_id, checkpoint_id),
        "procedure",
        "Run release checks",
        "Run the verified release checks before publishing.",
        "1111111111111111",
    );
    let pitfall = create_confirmed_learning(
        &config,
        &project,
        (session_id, checkpoint_id),
        "pitfall",
        "Avoid stale build output",
        "Never publish from a stale build directory.",
        "2222222222222222",
    );

    let compile_arguments = [
        "runbook",
        "compile",
        project.to_str().unwrap(),
        "--title",
        "Release workflow",
        "--learning",
        &procedure,
        "--learning",
        &pitfall,
        "--json",
    ];
    let compiled = json_stdout(ley(&config, &compile_arguments));
    let rebuilt = json_stdout(ley(&config, &compile_arguments));
    let runbook_id = compiled["runbookId"].as_str().unwrap();
    assert!(runbook_id.starts_with("rbk_"));
    assert_eq!(compiled["runbookId"], rebuilt["runbookId"]);
    assert_eq!(compiled["sourceFingerprint"], rebuilt["sourceFingerprint"]);
    assert_eq!(compiled["projection"], "on-demand-reviewed-runbook");
    assert_eq!(compiled["persisted"], false);
    assert_eq!(compiled["authorityIncreasedThroughProjection"], false);
    assert_eq!(compiled["requiresExplicitSkillExport"], true);
    assert_eq!(compiled["liveSourceChecked"], false);
    assert!(compiled["markdown"]
        .as_str()
        .unwrap()
        .contains("Run the verified release checks"));
    let serialized = compiled.to_string();
    assert!(!serialized.contains(project.to_str().unwrap()));
    assert!(!serialized.contains(vault.to_str().unwrap()));

    let exported = json_stdout(ley(
        &config,
        &[
            "runbook",
            "export-skill",
            project.to_str().unwrap(),
            "--title",
            "Release workflow",
            "--learning",
            &procedure,
            "--learning",
            &pitfall,
            "--expected-runbook",
            runbook_id,
            "--host",
            "codex",
            "--egress-target",
            "cloud",
            "--json",
        ],
    ));
    assert_eq!(exported["host"], "codex");
    assert_eq!(exported["egressTarget"], "cloud");
    assert_eq!(exported["runbookId"], runbook_id);
    assert_eq!(exported["persisted"], false);
    assert_eq!(exported["installed"], false);
    assert_eq!(exported["explicitUserActionRequired"], true);
    assert_eq!(exported["liveSourceChecked"], false);
    let content = exported["content"].as_str().unwrap();
    assert!(content.contains("name: ley-release-workflow"));
    assert!(content.contains(runbook_id));
    assert!(content.contains("Run the verified release checks"));
    assert!(!content.contains(project.to_str().unwrap()));
    assert!(!content.contains(vault.to_str().unwrap()));

    let wrong_id = format!("rbk_{}", "0".repeat(64));
    let stale = run_ley(
        &config,
        &[
            "runbook",
            "export-skill",
            project.to_str().unwrap(),
            "--title",
            "Release workflow",
            "--learning",
            &procedure,
            "--learning",
            &pitfall,
            "--expected-runbook",
            &wrong_id,
            "--host",
            "codex",
            "--egress-target",
            "cloud",
        ],
    );
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("reviewed runbook changed"));

    ley(
        &config,
        &[
            "egress",
            "project",
            "local-model-only",
            project.to_str().unwrap(),
        ],
    );
    let cloud_blocked = run_ley(
        &config,
        &[
            "runbook",
            "export-skill",
            project.to_str().unwrap(),
            "--title",
            "Release workflow",
            "--learning",
            &procedure,
            "--learning",
            &pitfall,
            "--expected-runbook",
            runbook_id,
            "--host",
            "claude-code",
            "--egress-target",
            "cloud",
        ],
    );
    assert!(!cloud_blocked.status.success());
    assert!(String::from_utf8_lossy(&cloud_blocked.stderr)
        .contains("egress policy 'local-model-only' does not allow target 'cloud'"));

    let local = json_stdout(ley(
        &config,
        &[
            "runbook",
            "export-skill",
            project.to_str().unwrap(),
            "--title",
            "Release workflow",
            "--learning",
            &procedure,
            "--learning",
            &pitfall,
            "--expected-runbook",
            runbook_id,
            "--host",
            "claude-code",
            "--egress-target",
            "local",
            "--json",
        ],
    ));
    assert_eq!(local["host"], "claude-code");
    assert_eq!(local["egressTarget"], "local");

    assert!(!contains_named_file(base.path(), "SKILL.md"));
}

fn contains_named_file(directory: &Path, name: &str) -> bool {
    fs::read_dir(directory).unwrap().any(|entry| {
        let path = entry.unwrap().path();
        if path.is_dir() {
            contains_named_file(&path, name)
        } else {
            path.file_name().and_then(|value| value.to_str()) == Some(name)
        }
    })
}
