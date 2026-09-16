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
        assert!(skill.contains("ley_compile_context"), "{}", path.display());
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
        assert!(skill.contains("does not bind"), "{}", path.display());
        assert!(skill.contains("do not advance"), "{}", path.display());
        assert!(skill.contains("expectedEventCount"), "{}", path.display());
        assert!(
            skill.contains("Do not infer completion"),
            "{}",
            path.display()
        );
        assert!(skill.contains("live source"), "{}", path.display());
        assert_portable(&path);
    }
}
