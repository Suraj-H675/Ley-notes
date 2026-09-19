use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn json(path: impl AsRef<Path>) -> Value {
    let path = path.as_ref();
    serde_json::from_str(&fs::read_to_string(path).expect("package JSON"))
        .unwrap_or_else(|error| panic!("{} is invalid JSON: {error}", path.display()))
}

fn assert_portable(path: impl AsRef<Path>) {
    let path = path.as_ref();
    let text = fs::read_to_string(path).expect("portable package file");
    for forbidden in [
        "/home/",
        "/Users/",
        "C:\\Users\\",
        "Suraj-H675/Ley-notes/integrations",
    ] {
        assert!(
            !text.contains(forbidden),
            "{} contains a machine-specific path: {forbidden}",
            path.display()
        );
    }
}

#[test]
fn claude_plugin_is_portable_discoverable_and_turn_aware() {
    let root = repository_root();
    let plugin = root.join("integrations/claude-code/ley-memory");
    let marketplace = json(root.join(".claude-plugin/marketplace.json"));
    assert_eq!(marketplace["name"], "ley");
    assert_eq!(
        marketplace["plugins"][0]["source"],
        "./integrations/claude-code/ley-memory"
    );
    assert!(plugin.join(".claude-plugin/plugin.json").is_file());

    let mcp = json(plugin.join(".mcp.json"));
    assert_eq!(mcp["mcpServers"]["ley"]["command"], "ley");
    assert_eq!(
        mcp["mcpServers"]["ley"]["args"],
        serde_json::json!([
            "mcp",
            "${CLAUDE_PROJECT_DIR}",
            "--allow-session-writes",
            "--allow-learning-proposals"
        ])
    );

    let hooks = json(plugin.join("hooks/hooks.json"));
    for event in ["SessionStart", "UserPromptSubmit", "Stop"] {
        let handler = &hooks["hooks"][event][0]["hooks"][0];
        assert_eq!(handler["command"], "ley", "{event}");
        assert_eq!(
            handler["args"],
            serde_json::json!(["hook", "--host", "claude", "${CLAUDE_PROJECT_DIR}"]),
            "{event}"
        );
    }

    let skill = fs::read_to_string(plugin.join("skills/ley-memory/SKILL.md")).unwrap();
    assert!(skill.contains("ley_session_checkpoint"));
    assert!(skill.contains("raw transcripts"));
    assert_portable(root.join(".claude-plugin/marketplace.json"));
    for path in [
        plugin.join(".claude-plugin/plugin.json"),
        plugin.join(".mcp.json"),
        plugin.join("hooks/hooks.json"),
        plugin.join("skills/ley-memory/SKILL.md"),
    ] {
        assert_portable(path);
    }
}
#[test]
fn packaged_skills_prefer_compiled_task_context_without_weak_memory_padding() {
    let root = repository_root();
    for path in [
        root.join("integrations/claude-code/ley-memory/skills/ley-memory/SKILL.md"),
        root.join("integrations/codex/plugins/ley-memory/skills/ley/SKILL.md"),
    ] {
        let skill = fs::read_to_string(&path).expect("packaged Ley skill");
        let normalized_skill = skill.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            skill.contains("ley_project_specifications"),
            "{}",
            path.display()
        );
        assert!(skill.contains("human-intent"), "{}", path.display());
        assert!(skill.contains("shared budget"), "{}", path.display());
        assert!(skill.contains("authorityPrecedence"), "{}", path.display());
        assert!(skill.contains("referencePrecedence"), "{}", path.display());
        assert!(
            skill.contains("sharedKnowledgePrecedence"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("mountedReferenceScopes"),
            "{}",
            path.display()
        );
        assert!(skill.contains("mountedReferences"), "{}", path.display());
        assert!(
            skill.contains("sharedKnowledgeScopes"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("sharedKnowledgeReferences"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("sharedKnowledgeExclusions"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("mountedReferenceExclusions"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("lower-precedence read-only"),
            "{}",
            path.display()
        );
        assert!(skill.contains("mountId"), "{}", path.display());
        assert!(
            skill.contains("untrusted-shared-project-memory"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("create/remove Context Mounts"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("create/list/attach/detach Knowledge Scopes"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("cannot approve")
                || skill.contains("cannot approve or revoke")
                || skill.contains("never approve"),
            "{}",
            path.display()
        );
        assert!(skill.contains("ley_compile_context"), "{}", path.display());
        assert!(
            skill.contains("ley_context_pack_inspect"),
            "{}",
            path.display()
        );
        assert!(skill.contains("contextPackId"), "{}", path.display());
        assert!(
            skill.contains("matchesExpectedContextPack"),
            "{}",
            path.display()
        );
        assert!(skill.contains("ley_project_state"), "{}", path.display());
        assert!(skill.contains("ley_memory_health"), "{}", path.display());
        assert!(skill.contains("unsupportedSignals"), "{}", path.display());
        assert!(
            skill.contains("destructiveActionsTaken"),
            "{}",
            path.display()
        );
        assert!(skill.contains("ley_agent_legibility"), "{}", path.display());
        assert!(
            skill.contains("tableOfContentsNotScore"),
            "{}",
            path.display()
        );
        assert!(skill.contains("selectionBasis"), "{}", path.display());
        assert!(skill.contains("declaredCommands"), "{}", path.display());
        assert!(skill.contains("observedCommands"), "{}", path.display());
        assert!(skill.contains("ley runbook compile"), "{}", path.display());
        assert!(
            skill.contains("ley runbook export-skill"),
            "{}",
            path.display()
        );
        assert!(skill.contains("--expected-runbook"), "{}", path.display());
        assert!(skill.contains("installed: false"), "{}", path.display());
        assert!(skill.contains("outside MCP"), "{}", path.display());
        assert!(
            skill.contains("evidenceArtifactPaths"),
            "{}",
            path.display()
        );
        assert!(skill.contains("evidenceArtifacts"), "{}", path.display());
        assert!(
            skill.contains("ley_read_media_evidence"),
            "{}",
            path.display()
        );
        assert!(skill.contains("mediaType"), "{}", path.display());
        assert!(
            normalized_skill.contains("original untrusted media evidence"),
            "{}",
            path.display()
        );
        assert!(skill.contains("OCR"), "{}", path.display());
        assert!(skill.contains("Full Evidence"), "{}", path.display());
        assert!(skill.contains("ley_topic_dossier"), "{}", path.display());
        assert!(skill.contains("currentStateProven"), "{}", path.display());
        assert!(skill.contains("premiseAdjudication"), "{}", path.display());
        assert!(skill.contains("obsolete-assumption"), "{}", path.display());
        assert!(skill.contains("revisionFreshness"), "{}", path.display());
        assert!(
            skill.contains("revisionApplicability"),
            "{}",
            path.display()
        );
        assert!(skill.contains("divergent"), "{}", path.display());
        assert!(skill.contains("merged"), "{}", path.display());
        assert!(skill.contains("liveGitChecked"), "{}", path.display());
        assert!(skill.contains("ley_graph_neighbors"), "{}", path.display());
        assert!(skill.contains("ley_graph_path"), "{}", path.display());
        assert!(
            normalized_skill.contains("relative JavaScript/TypeScript import"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("revisionCompatibility"),
            "{}",
            path.display()
        );
        assert!(skill.contains("current-lineage"), "{}", path.display());
        assert!(
            skill.contains("revisionFilteredCandidates"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("Filtering is inspection scope"),
            "{}",
            path.display()
        );
        assert!(skill.contains("egressTarget"), "{}", path.display());
        assert!(skill.contains("egressCoverage"), "{}", path.display());
        assert!(skill.contains("egressExclusions"), "{}", path.display());
        assert!(
            skill.contains("withheld by egress policy"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("historicalMemoryWithheld"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("withheldDerivedResults"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("policyBundlePrecedence"),
            "{}",
            path.display()
        );
        assert!(skill.contains("policyBundlePolicies"), "{}", path.display());
        assert!(
            skill.contains("policyBundleExclusions"),
            "{}",
            path.display()
        );
        assert!(skill.contains("ley policy-bundle"), "{}", path.display());
        assert!(skill.contains("local-model-only"), "{}", path.display());
        assert!(skill.contains("confirm-per-use"), "{}", path.display());
        assert!(skill.contains("never-send"), "{}", path.display());
        assert!(
            skill.contains("cannot change egress policy"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("ley_external_connectors_list"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("ley_external_connector_get"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("untrusted-external-reference"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("Neither tool contacts GitHub"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill
                .contains("Never add, refresh, remove, or change egress for a connector"),
            "{}",
            path.display()
        );
        assert!(skill.contains("40-hex commit SHA"), "{}", path.display());
        assert!(
            normalized_skill.contains("branch/tag document URLs are deliberately unsupported"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill
                .contains("does not prove the current branch still points to that commit"),
            "{}",
            path.display()
        );
        assert!(skill.contains("conflicting-state"), "{}", path.display());
        assert!(skill.contains("uncertain-state"), "{}", path.display());
        assert!(
            skill.contains("replacementLearningId"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill.contains("not proof") && normalized_skill.contains("live source"),
            "{}",
            path.display()
        );
        assert!(skill.contains("no-useful-evidence"), "{}", path.display());
        assert!(
            skill.contains("ley_session_memory_compile"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("ley_session_memory_verify"),
            "{}",
            path.display()
        );
        assert!(skill.contains("review-required"), "{}", path.display());
        assert!(
            skill.contains("semantic faithfulness"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("ley_session_memory_commit_unresolved"),
            "{}",
            path.display()
        );
        assert!(skill.contains("generic checkpoint"), "{}", path.display());
        assert!(skill.contains("re-verif"), "{}", path.display());
        assert!(skill.contains("do not advance"), "{}", path.display());
        assert!(skill.contains("expectedEventCount"), "{}", path.display());
        assert!(
            skill.contains("Do not infer completion"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill.contains("mechanically known origin lineage"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill.contains("complete causal ancestry"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill
                .contains("Automatic derivation cannot grant authority above `review-required`"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill.contains("does not erase origin history"),
            "{}",
            path.display()
        );
        assert!(skill.contains("live source"), "{}", path.display());
        assert_portable(&path);
    }
}
