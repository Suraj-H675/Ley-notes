# ADR 0105: Normal-user native distribution and host integration boundary

**Status:** Accepted — 2026-10-05

## Context

Ley's focused continuity product works, but the normal installation path still leaks repository and CLI plumbing:
the Desktop bundle does not carry the `ley` engine, Linux builds omit AppImage, and both bundled host integrations
invoke a bare `ley` executable from `PATH`. That makes a source checkout, Rust toolchain, or manual CLI install part
of ordinary setup.

Current upstream contracts were rechecked before changing that architecture:

- Tauri v2 supports bundled resources, AppImage/DEB/RPM/NSIS/MSI/DMG targets, mandatory signed updater artifacts,
  and per-platform build hooks with `TAURI_ENV_TARGET_TRIPLE`.
- Tauri AppImage resources live inside the AppImage's mounted runtime directory, so a host integration must not
  point directly into that transient mount if Ley Desktop is allowed to be closed.
- current OpenAI Agent Plugins support portable root `plugin.json` / `mcp.json`, while
  `.codex-plugin/plugin.json` remains a compatibility format. Stdio MCP still requires a local executable and the
  portable package contract does not give Ley a documented exact active-project placeholder that replaces the
  current fixed-project boundary.
- current Claude Code plugins support `.claude-plugin/plugin.json`, `.mcp.json`, hooks, skills, plugin-root
  references, and marketplace delivery. Plugin installation/configuration is not proof that hooks are trusted or
  that a host has successfully run Ley.

Primary references checked on 2026-10-05:

- <https://v2.tauri.app/develop/resources/>
- <https://v2.tauri.app/distribute/appimage/>
- <https://v2.tauri.app/plugin/updater/>
- <https://developers.openai.com/plugins/build/plugins>
- <https://code.claude.com/docs/en/plugins>
- <https://code.claude.com/docs/en/plugins-reference>

## Decision

1. **Ship one native Ley engine with Ley Desktop.** The release build compiles the current `ley-cli` binary for the
   Tauri target and embeds it as an application resource. Normal users do not install that CLI separately.
2. **Materialize the engine into one stable, app-owned per-user path before configuring a host.** Desktop copies the
   packaged engine atomically into its private local application-data directory and uses owner-only Unix
   permissions. Host integrations will invoke that absolute path. This deliberately avoids mutating `PATH`.
3. **Desktop is not a daemon.** Codex or Claude Code starts the native Ley engine for hooks/MCP when needed. Closing
   Desktop does not disable a configured integration.
4. **AppImage joins DEB and RPM as the generic Linux artifact.** AppImage is useful for direct download, but its
   transient mount is exactly why integrations use the materialized helper rather than a path inside the bundle.
5. **Keep host state epistemically separated.** Future Desktop integration UI will distinguish detection,
   Ley-owned files/config installed, host trust/review required, restart required, and actually observed Ley host
   activity. Historical session activity remains historical evidence, not a fabricated live health check.
6. **Do not migrate Codex packaging merely for format symmetry.** ADR 0104's project-binding concern still applies:
   portable stdio package discovery does not by itself supply Ley's exact active project. A portable manifest is
   eligible when Ley's generated integration can preserve that boundary end to end; compatibility packaging may
   remain during that transition.
7. **Release/updater trust is separate from helper packaging.** Tauri updater signatures, Windows code signing,
   macOS code signing/notarization, and ordinary release checksums are distinct controls. Repository wiring may be
   completed without pretending unavailable signing credentials exist.
8. **The materialized helper keeps its own platform signature.** Release builds sign the helper binary itself before
   bundling on macOS/Windows, and release validation verifies that nested signature directly before Ley ever copies
   the helper into its private app-data path. A signed outer installer is not treated as sufficient proof for an
   executable that later runs independently.
9. **Windows signing is a trust requirement, not a commitment to one certificate transport.** The initial release
   workflow supports an imported PFX certificate because that is locally testable plumbing, but current Tauri and
   Microsoft guidance no longer make an exportable PFX the universal modern path. Azure Artifact Signing and
   provider-specific/custom signing are valid production models; for an eligible public OSS project, SignPath
   Foundation is especially attractive because it can verify GitHub build origin and deep-sign an MSI together with
   the nested Ley helper. Do not weaken helper-signature verification to fit a provider, and do not add speculative
   provider integration before the external account/project contract exists to test against.

## Consequences

- A packaged Ley install can own the runtime needed by Codex/Claude integrations without Node.js, Rust, a repo
  checkout, or a manually installed `ley` command.
- The same helper path can be reused by multiple supported hosts and updated with the application, avoiding
  per-plugin binary copies and version skew.
- Host setup remains an explicit user-approved mutation. Ley will not silently write host configuration on launch.
- The helper install/update path is a production trust boundary and therefore uses bounded fixed paths and atomic
  replacement rather than shell scripts or `curl | sh`.
- The repository's current PFX-based Windows lane is a fallback implementation detail, not architectural authority.
  A different signing provider may replace that mechanism while preserving the same Authenticode and nested-helper
  trust outcomes.
- Microsoft Store MSIX distribution remains an evaluated alternative rather than an implicit fallback. Store signing
  and Store-managed updates can remove developer-owned Windows certificate infrastructure, but adopting that path
  would introduce a distinct MSIX identity/submission/update channel and therefore requires an explicit product and
  release decision rather than opportunistic packaging code.
- This ADR does not claim integrations are already one-click, releases are signed, or updater credentials exist;
  those are later bounded milestones in the normal-user initiative.
