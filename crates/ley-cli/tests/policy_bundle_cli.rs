mod common;

use common::seed_legacy_project;
use ley_core::{
    diagnose_project, generate_specification_id, SpecificationRegistry, APP_IDENTIFIER,
    KNOWLEDGE_SCOPE_REGISTRY_FILE, POLICY_BUNDLE_REGISTRY_FILE, SPECIFICATION_REGISTRY_FILE,
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
    seed_legacy_project(project, vault, name);
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

fn seed_legacy_scope(config: &Path, active: &Path, source: &Path, scope_id: &str) {
    let active_id = diagnose_project(active).unwrap().identity.project_id;
    let source_id = diagnose_project(source).unwrap().identity.project_id;
    let path = config
        .join(APP_IDENTIFIER)
        .join(KNOWLEDGE_SCOPE_REGISTRY_FILE);
    let document = serde_json::json!({
        "schemaVersion": 1,
        "scopes": {
            scope_id: {
                "kind": "team",
                "name": "Platform team",
                "sourceProjectIds": [source_id.clone()],
                "createdAtUnixMs": 1_700_000_000_000_u64,
            }
        },
        "attachments": {
            active_id.clone(): {
                scope_id: 1_700_000_000_100_u64,
            }
        },
        "attachmentHistory": {
            active_id: {
                scope_id: [source_id],
            }
        }
    });
    fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

#[test]
fn cli_retires_policy_bundle_growth_but_preserves_legacy_inspection_and_detach() {
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
    let approved = specification_registry
        .approve(&source, &source_vault, &specification_id, "TeamPolicy.md")
        .unwrap();

    let create_scope_rejected = run_ley(
        &config,
        &[
            "scope",
            "create",
            "team",
            "Platform team",
            source.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!create_scope_rejected.status.success());
    assert!(String::from_utf8_lossy(&create_scope_rejected.stderr)
        .contains("unknown scope command 'create'"));

    let scope_id = "ksc_22222222222222222222222222222222";
    seed_legacy_scope(&config, &active, &source, scope_id);

    let create_bundle_rejected = run_ley(
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
    );
    assert!(!create_bundle_rejected.status.success());
    assert!(String::from_utf8_lossy(&create_bundle_rejected.stderr)
        .contains("unknown policy-bundle command 'create'"));

    let bundle_id = "pbd_11111111111111111111111111111111";
    let source_project_id = diagnose_project(&source).unwrap().identity.project_id;
    let active_project_id = diagnose_project(&active).unwrap().identity.project_id;
    let bundle_path = config
        .join(APP_IDENTIFIER)
        .join(POLICY_BUNDLE_REGISTRY_FILE);
    let bundle_document = serde_json::json!({
        "schemaVersion": 1,
        "bundles": {
            bundle_id: {
                "scopeId": scope_id,
                "name": "Platform policy",
                "sources": [{
                    "sourceProjectId": source_project_id,
                    "specificationId": specification_id,
                    "contentHash": approved.content_hash,
                }],
                "createdAtUnixMs": 1_700_000_000_000_u64,
            }
        },
        "attachments": {
            active_project_id.clone(): {
                bundle_id: { "attachedAtUnixMs": 1_700_000_000_100_u64 }
            }
        },
        "attachmentHistory": {
            active_project_id: {
                bundle_id: [{
                    "sourceProjectId": source_project_id.clone(),
                    "specificationId": specification_id.clone(),
                    "contentHash": approved.content_hash.clone(),
                }]
            }
        }
    });
    fs::write(
        &bundle_path,
        serde_json::to_vec_pretty(&bundle_document).unwrap(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bundle_path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    let listed = json_stdout(ley(&config, &["policy-bundle", "list", "--json"]));
    assert_eq!(listed["bundles"].as_array().unwrap().len(), 1);
    assert_eq!(listed["bundles"][0]["bundleId"], bundle_id);
    assert_eq!(listed["bundles"][0]["sources"][0]["status"], "ready");
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

    let attach_rejected = run_ley(
        &config,
        &[
            "policy-bundle",
            "attach",
            &bundle_id,
            active.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!attach_rejected.status.success());
    assert!(String::from_utf8_lossy(&attach_rejected.stderr)
        .contains("unknown policy-bundle command 'attach'"));

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

    fs::remove_file(specification_registry.path()).unwrap();
    fs::remove_dir_all(&source_vault).unwrap();
    let migrated_list = json_stdout(ley(&config, &["policy-bundle", "list", "--json"]));
    assert_eq!(migrated_list["bundles"][0]["sources"][0]["status"], "ready");
    let migrated_json = serde_json::to_string(&migrated_list).unwrap();
    assert!(!migrated_json.contains(source_vault.to_str().unwrap()));
    assert!(!migrated_json.contains(private_policy_marker));

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
