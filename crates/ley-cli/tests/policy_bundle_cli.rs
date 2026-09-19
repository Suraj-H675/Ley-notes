use ley_core::{
    generate_specification_id, SpecificationRegistry, APP_IDENTIFIER, SPECIFICATION_REGISTRY_FILE,
};
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

fn init_and_bind(config: &Path, project: &Path, vault: &Path, name: &str) {
    fs::create_dir_all(project).unwrap();
    fs::create_dir_all(vault).unwrap();
    fs::write(project.join("README.md"), format!("# {name}\n")).unwrap();
    ley(
        config,
        &["init", project.to_str().unwrap(), "--name", name, "--json"],
    );
    ley(
        config,
        &[
            "bind",
            project.to_str().unwrap(),
            "--vault",
            vault.to_str().unwrap(),
            "--json",
        ],
    );
}

#[test]
fn cli_policy_bundle_lifecycle_is_explicit_exact_revision_and_path_safe() {
    let base = tempdir().unwrap();
    let config = base.path().join("config");
    let active = base.path().join("active");
    let active_vault = base.path().join("active-vault");
    let source = base.path().join("source");
    let source_vault = base.path().join("source-vault");
    let unrelated = base.path().join("unrelated");
    let unrelated_vault = base.path().join("unrelated-vault");
    init_and_bind(&config, &active, &active_vault, "Active project");
    init_and_bind(&config, &source, &source_vault, "Policy source");
    init_and_bind(&config, &unrelated, &unrelated_vault, "Unrelated project");

    let private_policy_marker = "cli_private_policy_marker_71ca";
    fs::write(
        source_vault.join("TeamPolicy.md"),
        format!("# Team policy\n\n{private_policy_marker} use signed releases.\n"),
    )
    .unwrap();
    let specification_id = generate_specification_id();
    let specification_registry = SpecificationRegistry::at(
        config
            .join(APP_IDENTIFIER)
            .join(SPECIFICATION_REGISTRY_FILE),
    );
    specification_registry
        .approve(&source, &source_vault, &specification_id, "TeamPolicy.md")
        .unwrap();

    let created_scope = json_stdout(ley(
        &config,
        &[
            "scope",
            "create",
            "team",
            "Platform team",
            source.to_str().unwrap(),
            "--json",
        ],
    ));
    let scope_id = created_scope["scope"]["scopeId"]
        .as_str()
        .unwrap()
        .to_owned();

    let created = json_stdout(ley(
        &config,
        &[
            "policy-bundle",
            "create",
            &scope_id,
            "Platform policy",
            "--source",
            source.to_str().unwrap(),
            &specification_id,
            "--json",
        ],
    ));
    assert_eq!(created["created"], true);
    assert_eq!(created["bundle"]["scopeId"], scope_id);
    assert_eq!(created["bundle"]["scopeKind"], "team");
    assert_eq!(created["bundle"]["name"], "Platform policy");
    assert_eq!(
        created["bundle"]["sources"][0]["specificationId"],
        specification_id
    );
    assert_eq!(created["bundle"]["sources"][0]["status"], "ready");
    let bundle_id = created["bundle"]["bundleId"].as_str().unwrap().to_owned();
    assert!(bundle_id.starts_with("pbd_"));

    let duplicate = json_stdout(ley(
        &config,
        &[
            "policy-bundle",
            "create",
            &scope_id,
            "Platform policy",
            "--source",
            source.to_str().unwrap(),
            &specification_id,
            "--json",
        ],
    ));
    assert_eq!(duplicate["created"], false);
    assert_eq!(duplicate["bundle"]["bundleId"], bundle_id);

    let listed = json_stdout(ley(&config, &["policy-bundle", "list", "--json"]));
    assert_eq!(listed["bundles"].as_array().unwrap().len(), 1);
    assert_eq!(listed["bundles"][0]["bundleId"], bundle_id);
    let listed_json = serde_json::to_string(&listed).unwrap();
    for private in [
        active.to_str().unwrap(),
        source.to_str().unwrap(),
        unrelated.to_str().unwrap(),
        active_vault.to_str().unwrap(),
        source_vault.to_str().unwrap(),
        unrelated_vault.to_str().unwrap(),
        private_policy_marker,
    ] {
        assert!(!listed_json.contains(private));
    }

    let premature_attach = run_ley(
        &config,
        &[
            "policy-bundle",
            "attach",
            &bundle_id,
            active.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!premature_attach.status.success());
    assert!(String::from_utf8_lossy(&premature_attach.stderr)
        .contains("must be attached before policy bundle"));

    ley(
        &config,
        &[
            "scope",
            "attach",
            &scope_id,
            active.to_str().unwrap(),
            "--json",
        ],
    );
    let attached = json_stdout(ley(
        &config,
        &[
            "policy-bundle",
            "attach",
            &bundle_id,
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(attached["created"], true);
    assert_eq!(attached["attachment"]["bundleId"], bundle_id);
    assert_eq!(attached["attachment"]["scopeId"], scope_id);
    assert_eq!(attached["attachment"]["state"], "active");

    let duplicate_attach = json_stdout(ley(
        &config,
        &[
            "policy-bundle",
            "attach",
            &bundle_id,
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(duplicate_attach["created"], false);

    let attached_list = json_stdout(ley(
        &config,
        &[
            "policy-bundle",
            "attached",
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(attached_list["attachments"].as_array().unwrap().len(), 1);
    assert_eq!(attached_list["attachments"][0]["bundleId"], bundle_id);
    assert_eq!(attached_list["attachments"][0]["state"], "active");

    let status = json_stdout(ley(
        &config,
        &[
            "policy-bundle",
            "status",
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(status, attached_list);

    let unrelated_status = json_stdout(ley(
        &config,
        &[
            "policy-bundle",
            "attached",
            unrelated.to_str().unwrap(),
            "--json",
        ],
    ));
    assert!(unrelated_status["attachments"]
        .as_array()
        .unwrap()
        .is_empty());

    let attachment_json = serde_json::to_string(&attached_list).unwrap();
    assert!(!attachment_json.contains(source.to_str().unwrap()));
    assert!(!attachment_json.contains(source_vault.to_str().unwrap()));
    assert!(!attachment_json.contains(private_policy_marker));

    let detached = json_stdout(ley(
        &config,
        &[
            "policy-bundle",
            "detach",
            &bundle_id,
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(detached["bundleId"], bundle_id);
    let empty = json_stdout(ley(
        &config,
        &[
            "policy-bundle",
            "attached",
            active.to_str().unwrap(),
            "--json",
        ],
    ));
    assert!(empty["attachments"].as_array().unwrap().is_empty());
}
