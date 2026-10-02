# ADR 0104: Defer portable Agent Plugins packaging until project binding is portable

**Status:** Accepted — 2026-10-02

## Context

Ley's Codex package currently uses the supported compatibility layout:

- `.codex-plugin/plugin.json`
- `.mcp.json`
- `skills/`
- `hooks/hooks.json`

The MCP command is intentionally fixed to the active project:

```text
ley mcp . --allow-session-writes
```

Agent Plugins 1.0 adds a portable root `plugin.json` and optional root `mcp.json`. Portable stdio servers may use a
bare executable such as `ley`, but their default working directory is the plugin root. The portable contract only
guarantees `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` placeholders; it does not define an active workspace/project
placeholder. The normative Agent Plugins 1.0 specification and OpenAI's packaging guidance were checked on
2026-10-02:

- <https://agent-plugins.org/specification>
- <https://developers.openai.com/plugins/build/plugins>

OpenAI's guidance also keeps `.codex-plugin/plugin.json` as a supported compatibility fallback. In a portable
package, root `plugin.json` supplies canonical package identity and root `mcp.json` supplies portable MCP
components; the compatibility manifest is not a second portable MCP-component source.

For Ley, changing `.` from the active project to the installed plugin directory is not a cosmetic packaging bug. It
would bind the fixed-project MCP server to the wrong project boundary and could strand or mis-scope continuity.

A recognized portable root manifest also makes portable component discovery authoritative. Adding `plugin.json`
without a correct portable `mcp.json` is therefore not a safe way to keep relying on the compatibility `.mcp.json`.

MCP roots are not adopted as the workaround: they add multi-root selection complexity and are deprecated in the
current protocol line (SEP-2577, checked 2026-10-02:
<https://plan.modelcontextprotocol.io/seps>). Replacing Ley's fixed-project startup boundary with generic
project-path tool parameters would also weaken a deliberate security/product invariant merely to satisfy packaging
symmetry.

The repository-side boundary is equally concrete: the shipped Codex package currently passes
`["mcp", ".", "--allow-session-writes"]`, and `crates/ley-cli/tests/host_packages.rs` locks that argument vector
down. That test is package/configuration coverage; it is not evidence that every current Codex surface installs the
repo marketplace or launches it from the active project directory.

This decision intentionally narrows the older first-principles audit's direction to "move toward" portable Agent
Plugins packaging. The audit remains valid evidence that portability is desirable, but the current portable process
contract does not yet satisfy Ley's fixed-project isolation invariant.

## Decision

Keep the current Codex compatibility package and repo marketplace until a portable client contract can supply one
exact active project root without broadening Ley's authority model.

Do **not** add root `plugin.json` / `mcp.json` yet.

The migration becomes eligible when at least one of these is true and validated end to end:

1. Agent Plugins standardizes a portable active-workspace/project binding suitable for fixed-project stdio MCP;
2. OpenAI exposes a stable documented client extension that passes one exact active project root to bundled MCP
   servers without relying on ambient guessed paths; or
3. Ley develops a new project-binding handshake that preserves the same fail-closed single-project isolation across
   supported hosts and beats the current package in a portability/security evaluation.

Any future migration must keep the `.codex-plugin` fallback until current Codex installs are proven compatible and
must prove that portable and compatibility packages resolve the same project, expose the same canonical tool
surface, and run the same trusted hooks.

## Consequences

- Ley does not claim portable Agent Plugins conformance today.
- The existing Codex package remains supported rather than being rewritten into a subtly wrong portable package.
- Package portability/configuration and cross-package version tests remain useful current coverage; they do not by
  themselves prove marketplace installation or host launch-directory behavior.
- This is a packaging limitation, not a blocker for the focused local-first Ley product.

## Non-goals

This ADR does not redesign MCP project selection, add multi-project tools, adopt deprecated MCP roots, or publish Ley
to the universal plugin directory.
