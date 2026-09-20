# Connect Ley to coding agents

Ley uses three layers together:

1. lifecycle hooks load a bounded continuity brief, capture bounded turn evidence, and inject compact task-specific Context Compiler output on supported prompt events;
2. local stdio MCP provides the task-conditioned `ley_compile_context` entry point, explicit `ley_session_memory_compile` recovery plus `ley_session_memory_verify` transition checks for missed checkpoints, deeper cited retrieval, and typed session writes;
3. a portable agent skill tells the host to prefer compiled task context, inspect live source when needed, and preserve meaningful structure.

All three run on the user's machine. The host may send deliberately retrieved context to its model provider. Lifecycle hooks and MCP retrieval make no external-connector network request; the separate local `ley connector refresh` command may contact the fixed public GitHub API only after the user has explicitly configured a supported connector. Semantic-model installation remains another separate explicit download path.

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

The packaged integrations enable session writes and tentative learning proposals in their local MCP process. Host permission controls still apply. Remove `--allow-learning-proposals` from the package's MCP arguments if proposals are not wanted.

For structural impact questions, hosts may use `ley_graph_neighbors` / `ley_graph_path` progressively rather than preloading the whole graph. A captured unambiguous relative JavaScript/TypeScript import can connect the importing test/module to the captured implementation file; ambiguous or package imports remain external. These are deterministic captured relations with citations, not a live-source check or permission to skip inspecting the current workspace.

## Codex

Install directly from GitHub with a sparse checkout of only the marketplace and plugin bundle:

```bash
codex plugin marketplace add Suraj-H675/Ley-notes --ref main \
  --sparse .agents/plugins \
  --sparse integrations/codex/plugins/ley-memory
codex plugin add ley-memory@ley
```

For local plugin development, the repository also contains a standalone marketplace at `integrations/codex`:

```bash
codex plugin marketplace add /absolute/path/to/Ley-notes/integrations/codex
codex plugin add ley-memory@ley
```

Restart Codex, open `/hooks`, and review the exact Ley hook commands before trusting them. Codex intentionally does not trust newly installed command hooks automatically.

Start a new Codex chat in an initialized project and invoke **@Ley**, or ask Codex to use Ley. The packaged MCP and lifecycle-hook commands intentionally use the default `cloud` egress target. Local-only knowledge therefore stays unavailable unless the user deliberately configures both relevant commands with `--egress-target local` for an approved local-model/runtime setup; Ley does not auto-detect or attest that runtime. When startup egress is allowed, `SessionStart` loads the bounded resume pack automatically. When a fine-grained restriction blocks historical context, SessionStart instead injects only the stable Ley session plus a content-free egress-withheld notice—no resume, learning, or recovery content. A project-wide denial is a no-op. If an allowed host session has prompt/response observations after its latest structured checkpoint, startup adds only a recovery count/state signal; it does not inject those bodies. The agent explicitly inspects `ley_session_memory_compile`, formulates only evidence-supported candidate structure, and runs `ley_session_memory_verify` with exact recovery record IDs before any reconstruction. `review-required` is still not semantic proof or write authorization. On each eligible `UserPromptSubmit`, the adapter records the bounded turn and runs the existing Context Compiler with the exact whitespace-normalized task when it fits Ley's existing project-memory query bound. It injects a compact `# Ley task context (automatic)` projection rather than the full MCP payload: allowed human intent/evidence, essential egress/premise/revision state, follow-up handles, logical `contextPackId`, and explicit host-rendering omissions. The block never repeats the raw prompt, stays below Ley's strict local hook-output bound, performs no utility binding or write-authority change, and falls back without truncating the task if exact automatic compilation is unavailable. `ley_compile_context` is therefore the refinement/deeper-inspection path rather than a mandatory duplicate call every turn. If context quality itself needs debugging, `ley_context_pack_inspect` must use the same task/result/token limits plus that exact ID; a mismatch means Ley is inspecting a newer recompilation, not reconstructing the older pack. For deliberate branch/worktree-history inspection, `ley_search_memory` may use exact `revisionCompatibility` values `current-lineage`, `ancestor`, `merged`, `divergent`, or `unknown`; omitting the filter preserves all captured history. The filter is inspection scope only: returned `revisionApplicability`/`revisionFreshness` still govern interpretation, divergent/unknown history is not current state, and `liveSourceChecked` remains false. `ley_session_get` likewise recomputes checkpoint applicability at read time, so an experimental checkpoint may become `merged` after Git proves it landed without re-ingestion. For an explicit project-status question, `ley_project_state` surfaces active/paused latest-checkpoint state, verification, reviewed-current knowledge, and attention-needed knowledge while keeping recent decisions historical with `currentStateProven: false`. A checkpoint verification may include `evidenceArtifactPaths` only for directly supporting artifacts already present in Ley's approved capture; returned `evidenceArtifacts` are immutable captured provenance, not live-source proof or increased authority. For deliberate maintenance, `ley_memory_health` surfaces advisory hygiene signals and unsupported measurement gaps but never authorizes or performs automatic deletion/rewrite/suppression. For compact project orientation, `ley_agent_legibility` exposes a source-bound table of contents over captured architecture/docs, directories, declared commands, separately labeled observed commands, policies, schema/API/observability references, current plans, and Specification metadata. Agents must treat `tableOfContentsNotScore` and `selectionBasis` as hard semantic boundaries: the map is navigation, not authority or a live-source check, and observed commands remain historical evidence rather than canonical instructions. For a repeatedly revisited area, `ley_topic_dossier` provides a bounded source-fingerprinted map for progressive disclosure, but none of these derived views replaces task-specific compilation or live-source inspection. Agents must respect `egressTarget`, `egressCoverage`, and `egressExclusions`; blocked Specifications are not source-read/scored and blocked mounts are not searched. If `historicalMemoryWithheld` is true, old session/decision/problem/learning text is intentionally absent because Ley cannot prove it is independent of a blocked source; agents must not reconstruct it from nearby memory, Inspector output, state, health diagnostics, legibility maps, or dossiers. Direct captured project evidence may still be available under the active-project policy. `confirm-per-use` remains fail-closed until Ley has a local confirmation flow. Lifecycle adapters and MCP cannot change egress policy, approve/revoke Specification authority, or create/remove Context Mounts.

Codex also supports the separate **Bootstrap Specification** mode from ADR 0058. The local user must first attach an already-approved source Specification with `ley bootstrap-spec attach`; Codex cannot create that authority. In the still-uninitialized target, `SessionStart` and `Stop` remain no-ops, `UserPromptSubmit` may inject `# Ley bootstrap task context (automatic)`, and no Ley session or prompt/response evidence is created. Bootstrap automatic context contains only whole exact approved Specifications. ADR 0059 additionally permits explicit `ley bootstrap-ref attach` captured reference projects, but those are **MCP-only** in this slice: reference-only authority does not make `UserPromptSubmit` inject memory. The bootstrap MCP server's sole read-only `ley_compile_context` tool may combine exact Specifications with admitted captured reference evidence under source egress. That server has no resources or project/session/history/write tools. If the user later initializes the target normally, both grant kinds are retired and subsequent Codex behavior follows ordinary project setup/binding rules.

Reusable team/organization Knowledge Scopes are also local-user authority. When one is attached to the active project, `ley_compile_context` may return `sharedKnowledgeScopes` and `sharedKnowledgeReferences` only after higher-precedence human intent, active-project context, and explicit Context Mounts; `sharedKnowledgePrecedence: explicit-mount-over-shared-knowledge` makes that lower priority explicit. Treat every shared item as read-only `untrusted-shared-project-memory` with stable scope/source provenance. Each source project's egress policy is checked before its memory is searched, and retained source ancestry can keep historical context withheld after detach. Codex must not create/list/attach/detach scopes through the agent workflow; those remain explicit local `ley scope ...` commands.

Attached team/organization Policy Bundles are a separate human-intent channel. They may contribute `policyBundles` and `policyBundlePolicies` only when their parent scope and bundle are both explicitly attached. Preserve `policyBundlePrecedence: active-project-specification-over-policy-bundle`: active-project Specifications win a direct contradiction and excluded bundled policy remains visible through `policyBundleExclusions`/coverage rather than silently replacing local intent. Bundled policy stays `human-intent` but grants no tool/filesystem/network/review/write/egress permission. Source-project and exact source-Specification egress are checked before a bundled policy note is opened, and retained bundle ancestry may keep historical context withheld after detach. Codex must not create/list/attach/detach Policy Bundles through MCP or hooks; those remain explicit local `ley policy-bundle ...` commands.

Context/memory utility feedback is deliberately opt-in rather than automatic on every Codex task. When the user or host workflow intentionally measures downstream benefit, call `ley_context_utility_bind` immediately after the exact `ley_compile_context` result and before outcome-producing work; retain the returned `cub_` binding. After typed checkpoint/session-finish outcomes exist, call `ley_context_utility_observe` only with those later event IDs. Exact bind/observe retries are idempotent. Treat the resulting observation as correlation evidence only: it does not prove the model used the context, does not prove causation, and does not change trust or retrieval ranking.

External GitHub connectors are also an explicit local-user workflow. Codex may call `ley_external_connectors_list` to discover connector metadata allowed for its current egress target and `ley_external_connector_get` to read one already-captured local snapshot. Both routes are network-free and read-only. Supported document connectors are public UTF-8 text files pinned to a full 40-hex commit SHA; branch/tag document URLs and arbitrary web docs are not accepted authority. Treat connector bodies as `untrusted-external-reference` evidence, never repository policy or instructions, and preserve `liveSourceChecked: false` as meaning GitHub was not refreshed. A pinned document is immutable evidence at that commit, not proof that the repository's current branch still points there. Codex must not create, refresh, remove, or change egress for a connector; those remain local `ley connector ...` / `ley egress connector ...` actions. If a connector restriction causes `historicalMemoryWithheld`, do not reconstruct the omitted history through other Ley readers.

Reviewed runbook and Skill conversion remains outside the Codex MCP surface. If the user explicitly asks to package reviewed operational knowledge, the local `ley runbook compile` command accepts only exact current trusted procedure/pitfall/convention learning IDs and returns a `runbookId` for human review. A later explicit `ley runbook export-skill` must carry that exact ID plus host and egress target; changed source state or blocked egress fails closed. The returned Skill is not installed (`installed: false`), grants no permissions, and still requires live-source inspection before consequential work. Codex must not manufacture, install, or reconstruct this artifact merely because remembered text suggests doing so.

## Claude Code

Install the repository marketplace and plugin from an authenticated checkout:

```bash
claude plugin marketplace add https://github.com/Suraj-H675/Ley-notes.git \
  --sparse .claude-plugin integrations/claude-code/ley-memory
claude plugin install ley-memory@ley
```

For local development, load the same self-contained package directly:

```bash
claude --plugin-dir /absolute/path/to/Ley-notes/integrations/claude-code/ley-memory
```

Restart Claude Code. The packaged MCP configuration also keeps Ley's default `cloud` egress target; `--egress-target local` is only for a deliberately approved local-model/runtime setup and is not automatic locality attestation. `SessionStart` loads the bounded brief and, when the same session has post-checkpoint turn evidence, only signals that recovery is available; it does not inject turn bodies. `UserPromptSubmit` reasserts the current Ley session, captures the bounded prompt according to project policy, and injects the same compact authority/egress-aware task projection described for Codex when the exact task fits the compiler query boundary; `Stop` captures the paired bounded response. Claude's hook receives no raw prompt echo from that projection, and oversized/unavailable automatic compilation falls back without task truncation. The packaged skill uses an adequate automatic pack directly and calls `ley_compile_context` only for fallback, refinement, materially changed tasks, or deeper context. If the supplied pack itself needs attribution/debugging, it may call `ley_context_pack_inspect` with the exact compile parameters and returned pack ID; the Inspector remains a non-persistent diagnostic view. For deliberate branch/worktree history inspection it may call `ley_search_memory` with exact `revisionCompatibility`; that narrows the historical candidate set only and never promotes divergent/unknown history into current truth. `ley_session_get` also returns read-time checkpoint applicability/current Git freshness while remaining a captured-memory view with `liveSourceChecked: false`. It may use `ley_project_state` for explicit project-status questions, `ley_memory_health` for deliberate non-destructive maintenance review, `ley_agent_legibility` for compact project orientation, and `ley_topic_dossier` for repeated topic orientation, while preserving that all derived historical views remain subject to the relevant egress ceiling. Checkpoint verification may attach `evidenceArtifactPaths` only for directly supporting artifacts already present in the approved capture; returned `evidenceArtifacts` remain immutable historical provenance, not current authority. Text citations use `ley_read_evidence`; a citation with `mediaType` uses `ley_read_media_evidence` with its exact path/snapshot/hash and returns original untrusted image evidence with no supplied OCR/vision description and `liveSourceChecked: false`. Agent Legibility is navigation only: keep declared commands distinct from historical observed commands, preserve `tableOfContentsNotScore`/`selectionBasis` semantics, and inspect live source before consequential edits. MCP cannot mutate egress policy. The package uses Claude Code's documented `${CLAUDE_PROJECT_DIR}` placeholder rather than assuming its process working directory.

Claude Code has the same explicit Bootstrap Specification exception as Codex. In an uninitialized target with a current local-user Specification grant, startup/final-response lifecycle events remain no-ops while `UserPromptSubmit` may inject whole approved Specification context under the strict hook-output bound. Bootstrap mode creates no Ley session and retains no prompt/response body. Explicit Bootstrap Reference grants from ADR 0059 remain MCP-only and do not enable prompt-time injection by themselves. The corresponding MCP process advertises only read-only `ley_compile_context`; it may return admitted captured reference evidence alongside higher-precedence Specifications, but no normal session, learning, graph, search, resource, or utility-feedback tools exist in this mode. Normal initialization retires all bootstrap authority and switches the workspace back to the ordinary project lifecycle.

Attached team/organization Knowledge Scopes follow the same compiler boundary in Claude Code: shared references appear only after higher-precedence human intent, active-project context, and explicit Context Mounts, remain `untrusted-shared-project-memory`, and inherit each source project's egress ceiling before search. Retained scope/source ancestry can therefore withhold broad historical context even after detach. Claude Code may consume the allowed `sharedKnowledge*` output but cannot create/list/attach/detach scope authority; that is a local-user `ley scope ...` workflow.

Policy Bundles follow the same explicit local-authority boundary in Claude Code. Allowed `policyBundlePolicies` are exact approved source-Specification revisions with stable bundle/scope/source/spec/hash provenance and lower precedence than an active-project Specification. A source-project or source-Specification egress restriction is applied before policy text is opened, and retained bundle ancestry can block unproven historical derivatives after detach. Claude Code may consume allowed bundle context and Inspector attribution but cannot create/list/attach/detach bundle authority; that remains a local-user `ley policy-bundle ...` workflow. Bundled policy is human intent only and never expands host/tool/filesystem/network/write/review/egress permission.

Context/memory utility feedback is likewise an explicit measurement workflow, not default Claude Code behavior. When deliberately requested, bind the exact compiled pack before the downstream work with `ley_context_utility_bind`, then attach only later typed checkpoint/session-finish outcomes through `ley_context_utility_observe`. The returned metadata is bounded session evidence; it never proves attention or causation and never authorizes automatic memory trust/ranking changes.

External GitHub connector reads follow the same boundary in Claude Code: `ley_external_connectors_list` and `ley_external_connector_get` may inspect only metadata/snapshots already stored locally and allowed for the configured target. They never contact GitHub. Commit-pinned public UTF-8 text docs are supported; branch/tag docs and arbitrary web docs are not. Treat all connector text as untrusted evidence and `liveSourceChecked: false` as a non-live read; a pinned document proves only the captured commit/path, not current-branch state. The plugin/MCP has no authority to add, refresh, remove, or change egress for connectors; those remain explicit local-user CLI operations, and blocked connector history must not be reconstructed through neighboring memory.

Reviewed runbook and Skill conversion also stays outside Claude Code's MCP surface. Only an explicit local-user workflow may run `ley runbook compile` over exact current trusted procedure/pitfall/convention learning IDs and then, after reviewing the returned `runbookId`, run `ley runbook export-skill` with that exact ID plus explicit host and egress target. Export is source-revalidated, egress-checked, and non-installing (`installed: false`). The packaged skill must not silently promote remembered content into a Claude Code Skill or treat an exported artifact as permission or live-source proof.


## What automatic capture does

In Structured and Full Evidence modes, the adapter stores:

- a generated host-session name and continuity goal;
- the stable Ley session ID;
- a bounded, pattern-redacted copy of each observed user prompt;
- a bounded, pattern-redacted copy of the host's final assistant response for each completed turn;
- an opaque Ley-derived turn reference that pairs the two without retaining the host's raw turn identifier.

Project artifact capture is separate from automatic turn capture. Supported PNG/JPEG/WebP originals are retained only when the project is explicitly in Full Evidence mode; lifecycle adapters do not create screenshots or infer visual descriptions. When a later checkpoint cites such an already-captured image, the host may inspect the exact original through `ley_read_media_evidence` within its egress/output bounds.

Minimal mode stores body-free prompt/response observation events so users can see what was omitted without retaining either body.

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
- tool inputs or complete outputs;
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

Turn evidence is not a checkpoint and is never promoted into startup context. If a crash or missed checkpoint leaves later turn evidence, Ley can deterministically expose that post-checkpoint window through `ley_session_memory_compile`. The tool is read-only, preserves prompt/response text as untrusted evidence, and distinguishes complete paired evidence from partial or metadata-only capture. Before reconstructing structure, the agent must run `ley_session_memory_verify`: every current recovery record is cited or explicitly deferred, stale/invalid/duplicate/revision cases fail closed, and even `review-required` does not prove semantic faithfulness or live-source correctness. `review-required` is emitted only when no current recovery evidence remains deferred; if any record is intentionally deferred, do not advance the checkpoint boundary. For one unresolved candidate, reconstruction uses `ley_session_memory_commit_unresolved`, which re-verifies the same event count and exact evidence set under the writer lock and stores binding provenance. Concurrent newer evidence forces a recompile and reverify; generic checkpointing is not a substitute for this bound recovery path. For substantive work the bundled skill asks the agent to write typed decisions, tasks, problems, attempts, outcomes, solutions, verification, touched artifacts, unresolved items, and handoff through MCP.

Ley deliberately does not add a global tool logger. Tool calls can contain
credentials, large outputs, or irrelevant details, and a hook cannot reliably
infer their durable meaning. Each bundled skill instead records bounded
commands, touched artifacts, and observed outcomes inside meaningful structured
checkpoints.

## Failure and retry behavior

An ordinary uninitialized workspace with no Bootstrap Specification authority, or an initialized-but-unbound project, returns `{}` and remains untouched. An uninitialized workspace with only Bootstrap Reference authority also remains a hook no-op. The narrow prompt-time exception requires current Bootstrap Specification authority: non-prompt events still return `{}`, but `UserPromptSubmit` may return read-only whole-Specification context without a Ley session or turn capture. In normal project mode, a stable host session maps to the same Ley session after process restart. Codex pairs retries with its documented stable `turn_id`; Claude Code uses the append-only Ley session state because their pre/post events do not share a stable turn identifier. Exact prompt/response retries replay the existing bounded turn evidence instead of duplicating it. Prompt-time automatic context is recomputed from the current permitted Ley state rather than persisted as a retry cache, so an unchanged retry returns the same logical pack while a real intervening authority/memory change may truthfully produce a newer pack. The same prompt submitted after a completed response remains a new turn. A new normal-project host session receives the bounded resume pack, including earlier checkpoint evidence and only user-trusted, artifact-current learnings—not captured prompt/response bodies.

The bundled MCP process also starts cleanly in an ordinary workspace, but advertises zero capabilities and no tools or resources. It never initializes or scans that directory. If and only if the local user has explicitly attached Bootstrap Specification **or Bootstrap Reference** authority, that same uninitialized workspace instead receives the one-tool read-only bootstrap MCP server. This keeps a globally installed integration quiet and harmless until the user either sets up a normal Ley project or explicitly grants a narrow bootstrap source.

If a captured snapshot is missing or inconsistent, the hook fails rather than inventing context. Run `ley doctor`, restore the binding if needed, then `ley ingest` deliberately.
