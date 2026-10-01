# Connect Ley to coding agents

Ley uses three layers together:

1. lifecycle hooks load a bounded continuity brief, capture bounded turn evidence plus supported Bash post-tool observations, and inject compact task-specific Context Compiler output on supported prompt events;
2. local stdio MCP exposes the focused `ley_brief` / `ley_search` / `ley_evidence` / `ley_checkpoint` surface for canonical projects; older compatibility mode may temporarily expose specialized recovery routes so retained state is not stranded;
3. a portable agent skill tells the host to prefer compiled task context, inspect live source when needed, and preserve meaningful structure.

For ordinary agent work, prefer the four focused MCP tools: call `ley_brief` for the current task, `ley_search` when the brief needs deeper captured history, carry an exact returned citation into `ley_evidence`, and use `ley_checkpoint` only at a meaningful structured session boundary when writes are enabled. `ley_evidence` verifies the immutable snapshot/path/hash citation instead of accepting an arbitrary path. `ley_compile_context` is bootstrap-only. Older `ley_search_memory`, `ley_read_evidence`, and `ley_session_checkpoint` routes remain compatibility/specialized surfaces during migration and should not define the normal host workflow.

All three run on the user's machine. The host may send deliberately retrieved context to its model provider. Lifecycle hooks and MCP retrieval make no external-connector network request. The legacy connector provider lifecycle is now retired as well: current CLI does not create or refresh connectors and therefore performs no GitHub connector fetch; retained local connector authority/snapshots remain inspectable/removable only for compatibility. Semantic-model installation remains a separate explicit download path.

## Before connecting a host

Install the `ley` executable on `PATH`, initialize the project, bind it to the user's chosen filesystem vault, and capture the first snapshot. From a Ley source checkout:

```bash
cargo install --path crates/ley-cli --root "$HOME/.local"
ley init /path/to/project --capture structured
ley bind /path/to/project --vault /path/to/ley-vault
ley ingest /path/to/project
```

Users with access to the repository can install the same CLI directly from
GitHub without machine-specific paths:

```bash
cargo install --git https://github.com/Suraj-H675/Ley-notes.git \
  --locked ley-cli --root "$HOME/.local"
ley --version
```

The repository is currently private, so GitHub-based installation requires an
authenticated collaborator until Suraj deliberately publishes the repository
or separate release artifacts. A local source checkout does not have that
requirement.

Every integration intentionally launches `ley` from `PATH`. On Linux and
macOS, ensure `$HOME/.local/bin` is on the PATH inherited by the agent host; the
command above uses that conventional location. On Windows, install to a
directory already on PATH. Do not continue until `ley --version` succeeds in a
fresh terminal. The packages contain no developer home directory, vault path,
project path, token, or other machine-specific configuration.

The packaged integrations enable only the canonical session-checkpoint write capability in their local MCP process. Tentative learning proposals are not enabled by default; a deliberate compatibility workflow may still opt into `--allow-learning-proposals` while that legacy route remains available. Host permission controls still apply.

Dedicated Ley graph query tools are retired. For structural impact questions, hosts should use bounded Ley continuity context and then inspect the live workspace with the coding host's normal repository tools. Retained deterministic graph history is migration/compatibility state, not live-source authority and not a reason to preserve a separate graph API.

When a host deliberately uses the explicit context-utility workflow, it should observe the bound pack
after eligible typed checkpoint/finish outcomes exist. `ley_session_get` exposes unobserved binding
coverage, unique observed-binding coverage, and at most five recent body-free unobserved binding
metadata rows. For a terminal session, `finish.eventId` and the row's `terminalFinishEventId` expose
the exact retained finish event that can be deliberately passed to the observe tool. A terminal
An unobserved binding row is a measurement-gap record only; it is not evidence that the model used or
ignored the context and must not alter trust/ranking automatically.

## Codex

Install directly from GitHub with a sparse checkout of only the marketplace and plugin bundle:

```bash
codex plugin marketplace add Suraj-H675/Ley-notes --ref main \
  --sparse .agents/plugins \
  --sparse integrations/codex/plugins/ley-memory
codex plugin add ley-memory@ley
```

For local plugin development:

```bash
codex plugin marketplace add /absolute/path/to/Ley-notes/integrations/codex
codex plugin add ley-memory@ley
```

Restart Codex, open `/hooks`, and review the exact Ley hook commands before trusting them. The packaged MCP process uses the default `cloud` egress target and enables session writes only; tentative learning proposals are not enabled by default.

For an initialized project, the package's normal MCP contract is the same four-tool surface taught by its Skill: `ley_brief`, `ley_search`, `ley_evidence`, and `ley_checkpoint`. `ley_checkpoint` appears only because the package starts MCP with `--allow-session-writes`. `ley_compile_context` is not a normal-project alias; it exists only in the explicit Bootstrap Context mode described below.

Lifecycle hooks carry most routine continuity. `SessionStart` establishes/resumes the bounded Ley session context, `UserPromptSubmit` records the bounded prompt and may inject a compact `# Ley task context (automatic)` block, `PostToolUse` records bounded/redacted Bash supporting evidence, and `Stop` captures the paired bounded response. A Codex `PostToolUse` event is recorded as `returned`; it does **not** prove the command, test, or requested work succeeded. The automatic context block never echoes the raw prompt, performs no hidden write-authority change, and remains evidence rather than instructions.

Use an adequate automatic block directly. Call `ley_brief` only when automatic context is absent/fallback/insufficient or the task materially changes; use `ley_search` for deeper bounded history; carry exact citations into `ley_evidence`; and use `ley_checkpoint` only for meaningful supported state in the current hook-provided session. In canonical native mode, granular session/recovery/context-utility/learning/connector tools are intentionally not advertised. If an older project is still in explicit legacy-compatibility mode, specialized recovery routes may exist solely to avoid stranding retained state; they are not the package's normal everyday API.

When egress blocks historical context, hooks inject only bounded/content-free disclosure rather than reconstructing the withheld memory. `--egress-target local` is only for a deliberately approved local-model/runtime configuration; Ley does not auto-detect or attest model locality. `confirm-per-use` remains fail-closed until a trustworthy local confirmation flow exists.

Codex also supports the separate **Bootstrap Specification** mode. A local user must first attach already-approved bootstrap authority. The still-uninitialized target creates no Ley project/session history, while its MCP server advertises exactly one read-only tool: `ley_compile_context`. Normal initialization retires the bootstrap authority and returns the workspace to the ordinary four-tool project lifecycle.

Retained Context Mount, Knowledge Scope, Policy Bundle, external-connector, and fine-grained egress records remain compatibility/privacy state. They can still constrain what `ley_brief` may admit, even after the agent-facing surface is small. Legacy Bootstrap Reference grants are narrower cleanup-only state: they do not activate or contribute bootstrap context and survive only so local users can list/detach them before initialization cleanup. Codex cannot mutate these authorities through normal MCP or hooks.

## Claude Code

Install the repository marketplace and plugin from an authenticated checkout:

```bash
claude plugin marketplace add https://github.com/Suraj-H675/Ley-notes.git \
  --sparse .claude-plugin integrations/claude-code/ley-memory
claude plugin install ley-memory@ley
```

For local development:

```bash
claude --plugin-dir /absolute/path/to/Ley-notes/integrations/claude-code/ley-memory
```

Restart Claude Code. The packaged MCP configuration uses the default `cloud` egress target and enables session writes only. Its normal initialized-project MCP contract is `ley_brief`, `ley_search`, `ley_evidence`, and `ley_checkpoint`; learning proposals and granular compatibility tools are not part of the canonical native surface.

Claude registers `SessionStart`, `UserPromptSubmit`, Bash-only `PostToolUse` and `PostToolUseFailure`, and `Stop` hooks. Normal Bash returns are retained only as `returned`; Claude's explicit failure hook is retained as `explicit-failure`. Neither observation becomes a checkpoint Command/Verification outcome or grants success authority. Prompt-time automatic context follows the same bounded authority/egress rules as Codex and does not echo the raw prompt.

Use the automatic block directly when adequate. `ley_brief` is the normal fallback/refinement call, `ley_search` is deeper bounded recall, `ley_evidence` opens exact cited text or supported original image evidence, and `ley_checkpoint` writes only meaningful supported state into the current hook-provided session. `ley_compile_context` remains bootstrap-only. Inspect live project source with Claude's normal workspace tools before consequential edits whenever correctness depends on current state.

Claude Code has the same fail-closed Bootstrap Specification exception as Codex: an uninitialized target with explicit bootstrap authority advertises exactly one read-only `ley_compile_context` tool and creates no normal Ley project/session history. Initializing the target retires bootstrap authority.

Retained mounts/references/scopes/bundles/connectors and finer-grained egress ancestry remain compatibility/privacy state that may constrain compiled context. Claude cannot create/attach/mutate those authorities through the canonical MCP surface; local CLI cleanup routes remain available until F4 proves the retained state can be safely removed.

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

Each adapter also injects the stable current Ley session ID at session and turn
start so MCP writes continue the same session instead of creating duplicates.
The native turn event is Codex and Claude `UserPromptSubmit`.

On that event the installed adapters also compile a compact task-specific context
projection through the existing Context Compiler after the bounded prompt capture.
This is model-visible context, not another durable Ley memory record. It excludes the
raw prompt string, preserves egress/authority diagnostics, keeps `liveSourceChecked:
false`, reports host-renderer omissions, and creates no context-utility binding.

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

In fully canonical native mode, granular Memory Compiler/verifier/commit routes are intentionally not advertised. Retained interrupted evidence is therefore not reconstructed through `ley_checkpoint`; the agent should re-establish current truth with normal live workspace tools and use `ley_checkpoint` only for new, currently supportable state. The old evidence remains historical rather than being silently converted into a decision, task, problem, command, or verification.

Older projects may still contain candidate-bound recovery events created by earlier Ley versions, but the shape-specific verifier/commit MCP routes are retired even in compatibility mode. Hosts may inspect bounded retained interruption/session evidence, must re-establish consequential current truth with normal live workspace tools, and may use an ordinary checkpoint only for new supportable state in an active session. Closed historical sessions remain read-only. Stored recovery evidence never grants tool, filesystem, network, review, trust, or egress authority.

Ley deliberately does not add a global tool logger. Tool calls can contain
credentials, large outputs, or irrelevant details, and a hook cannot reliably
infer their durable meaning. Each bundled skill instead records bounded
commands, touched artifacts, and observed outcomes inside meaningful structured
checkpoints.

## Failure and retry behavior

An ordinary uninitialized workspace with no Bootstrap Specification authority, or an initialized-but-unbound project, returns `{}` and remains untouched. An uninitialized workspace with only Bootstrap Reference authority also remains a hook no-op. The narrow prompt-time exception requires current Bootstrap Specification authority: non-prompt events still return `{}`, but `UserPromptSubmit` may return read-only whole-Specification context without a Ley session or turn capture. In normal project mode, a stable host session maps to the same Ley session after process restart. Codex pairs retries with its documented stable `turn_id`; Claude Code uses the append-only Ley session state because their pre/post events do not share a stable turn identifier. Exact prompt/response retries replay the existing bounded turn evidence instead of duplicating it. Prompt-time automatic context is recomputed from the current permitted Ley state rather than persisted as a retry cache, so an unchanged retry returns the same logical pack while a real intervening authority/memory change may truthfully produce a newer pack. The same prompt submitted after a completed response remains a new turn. A new normal-project host session receives the bounded resume pack, including earlier checkpoint evidence and only user-trusted, artifact-current learnings—not captured prompt/response bodies.

The bundled MCP process also starts cleanly in an ordinary workspace, but advertises zero capabilities and no tools or resources. It never initializes or scans that directory. If and only if the local user has explicitly attached Bootstrap Specification authority, that same uninitialized workspace instead receives the one-tool read-only bootstrap MCP server. A legacy Reference-only workspace remains inactive. This keeps a globally installed integration quiet and harmless until the user either sets up a normal Ley project or explicitly grants the narrow Specification bootstrap source.

If a captured snapshot is missing or inconsistent, the hook fails rather than inventing context. Run `ley doctor`, restore the binding if needed, then `ley ingest` deliberately.
