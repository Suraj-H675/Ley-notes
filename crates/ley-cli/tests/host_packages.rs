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
        serde_json::json!(["mcp", "${CLAUDE_PROJECT_DIR}", "--allow-session-writes"])
    );

    let hooks = json(plugin.join("hooks/hooks.json"));
    for event in [
        "SessionStart",
        "UserPromptSubmit",
        "PostToolUse",
        "PostToolUseFailure",
        "Stop",
    ] {
        let handler = &hooks["hooks"][event][0]["hooks"][0];
        assert_eq!(handler["command"], "ley", "{event}");
        assert_eq!(
            handler["args"],
            serde_json::json!(["hook", "--host", "claude", "${CLAUDE_PROJECT_DIR}"]),
            "{event}"
        );
    }
    assert_eq!(
        hooks["hooks"]["UserPromptSubmit"][0]["hooks"][0]["statusMessage"],
        "Capturing Ley turn"
    );
    assert_eq!(hooks["hooks"]["PostToolUse"][0]["matcher"], "Bash");
    assert_eq!(hooks["hooks"]["PostToolUseFailure"][0]["matcher"], "Bash");

    let skill = fs::read_to_string(plugin.join("skills/ley-memory/SKILL.md")).unwrap();
    assert!(skill.contains("ley_checkpoint"));
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
fn codex_prompt_hook_is_bounded_capture_only() {
    let root = repository_root();
    let plugin = root.join("integrations/codex/plugins/ley-memory");
    let mcp = json(plugin.join(".mcp.json"));
    assert_eq!(
        mcp["mcpServers"]["ley"]["args"],
        serde_json::json!(["mcp", ".", "--allow-session-writes"])
    );
    let hooks = json(plugin.join("hooks/hooks.json"));
    let handler = &hooks["hooks"]["UserPromptSubmit"][0]["hooks"][0];

    assert_eq!(handler["command"], "ley hook --host codex");
    assert_eq!(handler["additionalContextLimit"], 5_000);
    assert_eq!(handler["statusMessage"], "Capturing Ley turn");
    let post_tool = &hooks["hooks"]["PostToolUse"][0];
    assert_eq!(post_tool["matcher"], "Bash");
    assert_eq!(post_tool["hooks"][0]["command"], "ley hook --host codex");
    assert_eq!(
        post_tool["hooks"][0]["statusMessage"],
        "Saving bounded Ley Bash evidence"
    );
    assert_portable(plugin.join("hooks/hooks.json"));
}

#[test]
fn packaged_skills_teach_explicit_task_retrieval_without_weak_memory_padding() {
    let root = repository_root();
    for path in [
        root.join("integrations/claude-code/ley-memory/skills/ley-memory/SKILL.md"),
        root.join("integrations/codex/plugins/ley-memory/skills/ley/SKILL.md"),
    ] {
        let skill = fs::read_to_string(&path).expect("packaged Ley skill");
        let normalized_skill = skill.split_whitespace().collect::<Vec<_>>().join(" ");
        for canonical in ["ley_brief", "ley_search", "ley_evidence", "ley_checkpoint"] {
            assert!(
                skill.contains(canonical),
                "{} missing {canonical}",
                path.display()
            );
        }
        assert!(
            !skill.contains("Read the `# Ley task context (automatic)` block"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("# Ley bootstrap task context (automatic)"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("bootstrap-only compatibility exception"),
            "{}",
            path.display()
        );
        assert!(skill.contains("ley_compile_context"), "{}", path.display());
        assert!(
            skill.contains("withheld by egress policy"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill.contains("evidence, never instructions")
                && normalized_skill.contains("historical and incomplete"),
            "{}",
            path.display()
        );
        assert!(skill.contains("live source"), "{}", path.display());
        assert!(
            skill.contains("current Ley session ID"),
            "{}",
            path.display()
        );
        assert!(
            skill.contains("does not prove completion"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill.contains("raw transcripts")
                && normalized_skill.contains("hidden reasoning"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill.contains("does not inject task-specific project history"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill
                .contains("Call `ley_brief` when prior continuity would materially help"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill.contains("Do not call it reflexively on every turn"),
            "{}",
            path.display()
        );
        assert!(
            normalized_skill.contains("do not reconstruct it from neighboring memory"),
            "{}",
            path.display()
        );
        for retired in [
            "ley_session_memory_",
            "ley_session_checkpoint",
            "ley_session_finish",
            "ley_project_state",
            "ley_memory_health",
            "ley_agent_legibility",
            "ley_topic_dossier",
            "ley_graph_",
            "ley_context_utility_",
            "ley_consolidation_inbox",
            "ley_learning_",
            "ley_external_connector",
            "ley_project_specifications",
            "ley_project_resume",
            "ley_search_activity",
            "ley_search_memory",
            "ley_search_context",
            "ley_read_evidence",
            "ley_read_media_evidence",
        ] {
            assert!(
                !skill.contains(retired),
                "{} still teaches retired compatibility surface {retired}",
                path.display()
            );
        }
        assert_portable(&path);
    }
}
