# Connect Ley to coding agents

Ley uses three layers together:

1. lifecycle hooks establish/reuse the Ley session, emit guidance-only continuity/recovery context, and capture bounded turn evidence plus supported Bash post-tool observations. Initialized projects do **not** auto-inject task-specific project history on prompt submission; explicit uninitialized-workspace Bootstrap Specification authority is the narrow exception;
2. local stdio MCP exposes the focused `ley_brief` / `ley_search` / `ley_evidence` / `ley_checkpoint` surface for canonical projects; older compatibility mode may expose legacy session inspection surfaces, while retained recovery events remain replay-only;
3. a portable agent skill tells the host to prefer compiled task context, inspect live source when needed, and preserve meaningful structure.

For ordinary agent work, prefer the four focused MCP tools: call `ley_brief` for the current task, `ley_search` when the brief needs deeper captured history, carry an exact returned citation into `ley_evidence`, and use `ley_checkpoint` only at a meaningful structured session boundary when writes are enabled. `ley_evidence` verifies the immutable snapshot/path/hash citation instead of accepting an arbitrary path. `ley_compile_context` is bootstrap-only. Remaining lower-level evidence/session lifecycle routes are compatibility/specialized surfaces during migration and should not define the normal host workflow.

All three run on the user's machine. The host may send deliberately retrieved context to its model provider. Lifecycle hooks and MCP retrieval make no external-connector network request. The legacy connector provider lifecycle is retired as well: current CLI does not create or refresh connectors and therefore performs no GitHub connector fetch; retained local connector authority/snapshots remain inspectable/removable only for compatibility. The former semantic-model/index subsystem is also removed; all current and retained legacy Search paths are lexical-only.

## Before connecting a host

For normal use, install Ley Desktop and finish the project's first-run flow there. The packaged app contains the
native Ley engine and materializes it into Ley's private per-user application data before configuring a coding host.
No separate CLI installation or `PATH` edit is part of ordinary setup.

In **Project settings → Coding agents** (also offered at the first-project success state), Ley distinguishes these
facts instead of collapsing them into one "connected" badge:

- **Detected** — the host executable and version were found on this device;
- **Configured** — Ley Desktop's own integration package/config is present for the selected project;
- **Restart required / Review hooks** — the host still needs its documented restart/trust step;
- **Smoke check passed** — Ley has actually retained a host-hook session from that exact host for this project.

Connecting is always a user action. Ley does not silently mutate Codex or Claude configuration at application
startup. Generated integration files invoke the absolute app-owned helper path, so they do not depend on a shell's
`PATH` and Desktop does not need to remain open.

Fresh projects need no filesystem-vault binding. `ley bind PROJECT --vault EXISTING_LEGACY_VAULT` is a
reconnect-only compatibility command for pre-cutover projects whose selected vault already validates as historical
memory for that exact project.

The CLI remains a developer/maintenance surface. Contributors who deliberately need it from a source checkout may
still run:

```bash
cargo run --locked -p ley-cli -- --version
```

That developer command is not a prerequisite for Desktop integration. Generated packages contain no developer home
directory, vault path, token, or source-checkout path; project-specific Codex configuration is created only for the
project the user explicitly connected.

Ley Desktop does inspect documented host CLI state for detection/configuration, but it still does **not** infer
runtime health from installation. **Recorded agent activity** is a separate historical evidence surface built from
retained Ley session provenance:
recognized Codex/Claude Code host-hook sessions, generic host-hook provenance
when the retained host label is missing or unrecognized, and MCP-origin
sessions. Rows show retained-session counts plus the most recent qualifying
session start time; later session updates are not treated as host-observation
events. Manual CLI sessions and explicit
historical imports are not presented as integration activity. A blank activity surface
means only that Ley has no retained matching session for that project. The explicit Desktop smoke check uses this
same retained host-hook evidence; it never manufactures a session merely to make the status green.

The packaged integrations enable only the canonical session-checkpoint write capability in their local MCP process. Tentative learning proposals are not enabled by default; a deliberate compatibility workflow may still opt into `--allow-learning-proposals` while that legacy route remains available. Host permission controls still apply.

Dedicated Ley graph query and Context Utility mutation tools are retired. For structural impact questions, hosts should use bounded Ley continuity context and then inspect the live workspace with the coding host's normal repository tools. Retained deterministic graph history and old Context Utility observation records are historical compatibility/provenance data only; they do not re-enable those model-facing workflows or prove that a model used or ignored context.

## Codex

Use **Connect Codex** in Ley Desktop for normal setup. Ley registers a generated local marketplace/package and writes
only the selected project's `.codex/config.toml` entries needed to enable the Ley package and bind its `ley` MCP
server to that exact canonical project path. Existing unrelated project settings are preserved; an existing foreign
`mcp_servers.ley` is treated as a conflict and is not overwritten.

Ley deliberately keeps the generated Codex plugin's MCP declaration out of the plugin package itself: current
portable plugin MCP execution is rooted at the plugin, while Ley requires exact active-project isolation. The
project-level Codex MCP entry launches the stable app-owned helper with the selected project path explicitly. Hooks
use the same helper and rely on Codex's session working directory only for hook-time project context.

After connecting, restart Codex if Ley requests it, trust the project configuration, then open `/hooks` and review
the exact Ley hook commands before trusting them. The Desktop status remains "review required" until that is a
human/host action; Ley does not fake host trust.

For local plugin development only, contributors can still register the repository marketplace manually:

```bash
codex plugin marketplace add /absolute/path/to/Ley-notes
codex plugin add ley-memory@ley
```

The packaged MCP process uses the default `cloud` egress target and enables session writes only; tentative learning proposals are not enabled by default.

For an initialized project, the package's normal MCP contract is the same four-tool surface taught by its Skill: `ley_brief`, `ley_search`, `ley_evidence`, and `ley_checkpoint`. `ley_checkpoint` appears only because the package starts MCP with `--allow-session-writes`. `ley_compile_context` is not a normal-project alias; it exists only in the explicit Bootstrap Context mode described below.

Lifecycle hooks carry routine session continuity. `SessionStart` establishes/reuses the stable Ley session and returns only session/retrieval/checkpoint guidance; it does not auto-inject prior session, handoff, learning, Specification, or other historical project bodies. A same-session interrupted window may still add a bounded body-free recovery count/state signal. `UserPromptSubmit` records the bounded prompt and returns only session/capture/checkpoint guidance, `PostToolUse` records bounded/redacted Bash supporting evidence, and `Stop` captures the paired bounded response. A Codex `PostToolUse` event is recorded as `returned`; it does **not** prove the command, test, or requested work succeeded. Initialized-project `UserPromptSubmit` does not compile or inject task-specific project history.

Call `ley_brief` when prior project continuity would materially help the current task; use `ley_search` for deeper bounded history; carry exact citations into `ley_evidence`; and use `ley_checkpoint` only for meaningful supported state in the current hook-provided session. Do not call Brief reflexively on every turn. In canonical native mode, granular session/recovery/context-utility/learning/connector tools are intentionally not advertised. Older projects may expose legacy session inspection surfaces, but the shape-specific recovery verifiers and writers are retired; persisted recovery events remain available through replay and provenance reads.

Project-level egress still gates lifecycle hooks before session mutation. Fine-grained historical-source egress is enforced by deliberate retrieval surfaces (`ley_brief`, Search, Evidence), not by initialized `SessionStart`, because schema-7 startup no longer emits historical source bodies. `--egress-target local` is only for a deliberately approved local-model/runtime configuration; Ley does not auto-detect or attest model locality. `confirm-per-use` remains fail-closed until a trustworthy local confirmation flow exists.

Codex also supports the separate **Bootstrap Specification** mode. A local user must first attach already-approved bootstrap authority. The still-uninitialized target creates no Ley project/session history, while its MCP server advertises exactly one read-only tool: `ley_compile_context`. Normal initialization retires the bootstrap authority and returns the workspace to the ordinary four-tool project lifecycle.

Retained Context Mount, Knowledge Scope, Policy Bundle, external-connector, and fine-grained egress records remain compatibility/privacy state. They can still constrain what `ley_brief` may admit, even after the agent-facing surface is small. Legacy Bootstrap Reference grants are narrower cleanup-only state: they do not activate or contribute bootstrap context and survive only so local users can list/detach them before initialization cleanup. Codex cannot mutate these authorities through normal MCP or hooks.

## Claude Code

Use **Connect Claude Code** in Ley Desktop for normal setup. Ley generates a local Claude marketplace/package whose
MCP and hook commands point at the app-owned helper while retaining Claude's documented project-directory boundary,
then asks Claude Code to install that package at project scope.

Restart Claude Code when prompted and review the plugin/hooks before relying on capture. An installed/enabled plugin
is still configuration state, not proof that a hook ran; use Ley's smoke check after starting a fresh Claude Code
session in the selected project.

For local development only:

```bash
claude --plugin-dir /absolute/path/to/Ley-notes/integrations/claude-code/ley-memory
```

Restart Claude Code. The packaged MCP configuration uses the default `cloud` egress target and enables session writes only. Its normal initialized-project MCP contract is `ley_brief`, `ley_search`, `ley_evidence`, and `ley_checkpoint`; learning proposals and granular compatibility tools are not part of the canonical native surface.

Claude registers `SessionStart`, `UserPromptSubmit`, Bash-only `PostToolUse` and `PostToolUseFailure`, and `Stop` hooks. `SessionStart` is guidance-only just like Codex: stable Ley session identity, deliberate retrieval/checkpoint guidance, and body-free recovery signaling when applicable. Normal Bash returns are retained only as `returned`; Claude's explicit failure hook is retained as `explicit-failure`. Neither observation becomes a checkpoint Command/Verification outcome or grants success authority. Initialized-project prompt submission captures bounded turn evidence and returns session/checkpoint guidance only.

Call `ley_brief` when prior continuity would materially help, `ley_search` for deeper bounded recall, `ley_evidence` for exact cited text or supported original image evidence, and `ley_checkpoint` only for meaningful supported state in the current hook-provided session. `ley_compile_context` remains bootstrap-only. Inspect live project source with Claude's normal workspace tools before consequential edits whenever correctness depends on current state.

Claude Code has the same fail-closed Bootstrap Specification exception as Codex: an uninitialized target with explicit bootstrap authority advertises exactly one read-only `ley_compile_context` tool and creates no normal Ley project/session history. Initializing the target retires bootstrap authority.

Retained mounts/references/scopes/bundles/connectors and finer-grained egress ancestry remain compatibility/privacy state that may constrain compiled context. Claude cannot create/attach/mutate those authorities through the canonical MCP surface; local CLI cleanup routes remain only while real persisted records still need inspection/removal and their egress ancestry still protects historical derivatives.

## What automatic capture does

In Structured and Full Evidence modes, the adapter stores:

- a generated host-session name and continuity goal;
- the stable Ley session ID;
- a bounded, pattern-redacted copy of each observed user prompt;
- a bounded, pattern-redacted copy of the host's final assistant response for each completed turn;
- an opaque Ley-derived turn reference that pairs the two without retaining the host's raw turn identifier;
- for Bash only, bounded pattern-redacted post-tool command/result-or-error evidence with opaque Ley-owned tool-call identity. Raw command/error input and structured-result normalization are hard-bounded by text size, visited-node/fan-out counts, and nesting depth before temporary copies/traversal can grow without limit; accepted retained text then follows the ordinary redaction/truncation rules.

Project artifact capture is separate from automatic turn capture. Supported PNG/JPEG/WebP originals are retained only when the project is explicitly in Full Evidence mode; lifecycle adapters do not create screenshots or infer visual descriptions. When a later checkpoint cites such an already-captured image, the host may carry that exact citation into canonical `ley_evidence`, which returns the original image within its egress/output bounds.

Minimal mode stores body-free prompt/response and supported Bash observation events so users can see what was omitted without retaining those bodies. Structured and Full Evidence share one aggregate automatic-evidence capacity across turn and tool bodies.

Each adapter injects the stable current Ley session ID at session and turn
start so MCP writes continue the same session instead of creating duplicates. Initialized SessionStart does
not inject prior project-history bodies; agents retrieve continuity deliberately when the task needs it.
The native turn event is Codex and Claude `UserPromptSubmit`.

On that event initialized projects do **not** compile or inject task-specific project history. The hook keeps
capture/session/checkpoint guidance small and directs the agent to `ley_brief` when prior continuity would
materially help. The uninitialized Bootstrap Specification exception remains separate and may still return
its bounded read-only `# Ley bootstrap task context (automatic)` block.

It does not automatically store:

- transcript files or transcript paths;
- hidden reasoning;
- arbitrary non-Bash tool inputs or complete outputs;
- environment variables;
- arbitrary files outside the approved project capture boundary.

Historical import is intentionally **not** another lifecycle hook. When the user explicitly wants
to bring older Codex CLI message history into Ley, the local CLI supports:

```bash
ley session import codex-history PROJECT --source FILE --host-session SESSION_UUID
```

This reads only Codex's documented global message-history JSONL shape and only the selected
session's user messages. It does not auto-discover Codex storage, parse the richer rollout
transcript/session files, reconstruct assistant/tool history, or run because a hook supplied a
`transcript_path`. The imported Ley session is a completed `Import` snapshot with opaque
`hsi_` provenance and original source timestamps; it is excluded from automatic Resume.
There is no equivalent MCP mutation tool and no Claude historical importer in this first slice.
Agents should not initiate this import unless the user explicitly asks for that local history
operation.

Turn evidence is not a checkpoint and is never promoted into startup context. If a crash or missed checkpoint leaves later turn/tool evidence, the lifecycle layer may report a bounded recovery signal, but that signal does not prove completion, correctness, or a recoverable semantic structure.

In fully canonical native mode, granular Memory Compiler/recovery routes are intentionally not advertised. Retained interrupted evidence is therefore not reconstructed through `ley_checkpoint`; the agent should re-establish current truth with normal live workspace tools and use `ley_checkpoint` only for new, currently supportable state. The old evidence remains historical rather than being silently converted into a decision, task, problem, command, or verification.

Older projects may still contain candidate-bound recovery events created by earlier Ley versions. Their shape-specific core verifiers and writers are retired, while persisted schemas v3, v8-v13, and v16 remain replayable with provenance validation. Hosts may inspect bounded retained interruption/session evidence, must re-establish consequential current truth with normal live workspace tools, and may use an ordinary checkpoint only for new supportable state in an active session. Closed historical sessions remain read-only. Stored recovery evidence never grants tool, filesystem, network, review, trust, or egress authority.

Ley deliberately does not add a global tool logger. Tool calls can contain
credentials, large outputs, or irrelevant details, and a hook cannot reliably
infer their durable meaning. Each bundled skill instead records bounded
commands, touched artifacts, and observed outcomes inside meaningful structured
checkpoints.

## Failure and retry behavior

An ordinary uninitialized workspace with no Bootstrap Specification authority, or an initialized-but-unbound project, returns `{}` and remains untouched. An uninitialized workspace with only Bootstrap Reference authority also remains a hook no-op. The narrow prompt-time exception requires current Bootstrap Specification authority: non-prompt events still return `{}`, but `UserPromptSubmit` may return read-only whole-Specification context without a Ley session or turn capture. In normal project mode, a stable host session maps to the same Ley session after process restart. Codex pairs retries with its documented stable `turn_id`; Claude Code uses the append-only Ley session state because their pre/post events do not share a stable turn identifier. Exact prompt/response retries replay the existing bounded turn evidence instead of duplicating it. The same prompt submitted after a completed response remains a new turn. A new normal-project host session receives guidance-only startup context; historical checkpoints, handoffs, learnings, and other project bodies require deliberate retrieval. Captured prompt/response bodies are never auto-injected; an interrupted current session may expose only the bounded body-free recovery signal.

The bundled MCP process also starts cleanly in an ordinary workspace, but advertises zero capabilities and no tools or resources. It never initializes or scans that directory. If and only if the local user has explicitly attached Bootstrap Specification authority, that same uninitialized workspace instead receives the one-tool read-only bootstrap MCP server. A legacy Reference-only workspace remains inactive. This keeps a globally installed integration quiet and harmless until the user either sets up a normal Ley project or explicitly grants the narrow Specification bootstrap source.

If a captured snapshot is missing or inconsistent, the hook fails rather than inventing context. Run `ley doctor`, restore the binding if needed, then `ley ingest` deliberately.
