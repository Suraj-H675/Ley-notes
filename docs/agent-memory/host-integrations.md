# Connect Ley to coding agents

Ley uses three layers together:

1. lifecycle hooks load a bounded continuity brief and capture bounded turn evidence;
2. local stdio MCP provides the task-conditioned `ley_compile_context` entry point, explicit `ley_session_memory_compile` recovery plus `ley_session_memory_verify` transition checks for missed checkpoints, deeper cited retrieval, and typed session writes;
3. a portable agent skill tells the host to prefer compiled task context, inspect live source when needed, and preserve meaningful structure.

All three run on the user's machine. The host may send deliberately retrieved context to its model provider. Ley itself makes no network request.

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

Start a new Codex chat in an initialized project and invoke **@Ley**, or ask Codex to use Ley. The packaged MCP and lifecycle-hook commands intentionally use the default `cloud` egress target. Local-only knowledge therefore stays unavailable unless the user deliberately configures both relevant commands with `--egress-target local` for an approved local-model/runtime setup; Ley does not auto-detect or attest that runtime. When startup egress is allowed, `SessionStart` loads the bounded resume pack automatically. When a fine-grained restriction blocks historical context, SessionStart instead injects only the stable Ley session plus a content-free egress-withheld notice—no resume, learning, or recovery content. A project-wide denial is a no-op. If an allowed host session has prompt/response observations after its latest structured checkpoint, startup adds only a recovery count/state signal; it does not inject those bodies. The agent explicitly inspects `ley_session_memory_compile`, formulates only evidence-supported candidate structure, and runs `ley_session_memory_verify` with exact recovery record IDs before any reconstruction. `review-required` is still not semantic proof or write authorization. For a concrete task, `ley_compile_context` exposes egress, premise, revision diagnostics, and a logical `contextPackId` before admitted context. If context quality itself needs debugging, `ley_context_pack_inspect` must use the same task/result/token limits plus that exact ID; a mismatch means Ley is inspecting a newer recompilation, not reconstructing the older pack. For deliberate branch/worktree-history inspection, `ley_search_memory` may use exact `revisionCompatibility` values `current-lineage`, `ancestor`, `merged`, `divergent`, or `unknown`; omitting the filter preserves all captured history. The filter is inspection scope only: returned `revisionApplicability`/`revisionFreshness` still govern interpretation, divergent/unknown history is not current state, and `liveSourceChecked` remains false. `ley_session_get` likewise recomputes checkpoint applicability at read time, so an experimental checkpoint may become `merged` after Git proves it landed without re-ingestion. For an explicit project-status question, `ley_project_state` surfaces active/paused latest-checkpoint state, verification, reviewed-current knowledge, and attention-needed knowledge while keeping recent decisions historical with `currentStateProven: false`. A checkpoint verification may include `evidenceArtifactPaths` only for directly supporting artifacts already present in Ley's approved capture; returned `evidenceArtifacts` are immutable captured provenance, not live-source proof or increased authority. For deliberate maintenance, `ley_memory_health` surfaces advisory hygiene signals and unsupported measurement gaps but never authorizes or performs automatic deletion/rewrite/suppression. For compact project orientation, `ley_agent_legibility` exposes a source-bound table of contents over captured architecture/docs, directories, declared commands, separately labeled observed commands, policies, schema/API/observability references, current plans, and Specification metadata. Agents must treat `tableOfContentsNotScore` and `selectionBasis` as hard semantic boundaries: the map is navigation, not authority or a live-source check, and observed commands remain historical evidence rather than canonical instructions. For a repeatedly revisited area, `ley_topic_dossier` provides a bounded source-fingerprinted map for progressive disclosure, but none of these derived views replaces task-specific compilation or live-source inspection. Agents must respect `egressTarget`, `egressCoverage`, and `egressExclusions`; blocked Specifications are not source-read/scored and blocked mounts are not searched. If `historicalMemoryWithheld` is true, old session/decision/problem/learning text is intentionally absent because Ley cannot prove it is independent of a blocked source; agents must not reconstruct it from nearby memory, Inspector output, state, health diagnostics, legibility maps, or dossiers. Direct captured project evidence may still be available under the active-project policy. `confirm-per-use` remains fail-closed until Ley has a local confirmation flow. Lifecycle adapters and MCP cannot change egress policy, approve/revoke Specification authority, or create/remove Context Mounts.

Context/memory utility feedback is deliberately opt-in rather than automatic on every Codex task. When the user or host workflow intentionally measures downstream benefit, call `ley_context_utility_bind` immediately after the exact `ley_compile_context` result and before outcome-producing work; retain the returned `cub_` binding. After typed checkpoint/session-finish outcomes exist, call `ley_context_utility_observe` only with those later event IDs. Exact bind/observe retries are idempotent. Treat the resulting observation as correlation evidence only: it does not prove the model used the context, does not prove causation, and does not change trust or retrieval ranking.

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

Restart Claude Code. The packaged MCP configuration also keeps Ley's default `cloud` egress target; `--egress-target local` is only for a deliberately approved local-model/runtime setup and is not automatic locality attestation. `SessionStart` loads the bounded brief and, when the same session has post-checkpoint turn evidence, only signals that recovery is available; it does not inject turn bodies. `UserPromptSubmit` reasserts the current Ley session and captures the bounded prompt according to project policy; `Stop` captures the paired bounded response. For concrete tasks, the packaged skill requires `ley_compile_context` and inspection of egress, premise, and revision diagnostics before historical state is acted on. If the supplied pack itself needs attribution/debugging, it may call `ley_context_pack_inspect` with the exact compile parameters and returned pack ID; the Inspector remains a non-persistent diagnostic view. For deliberate branch/worktree history inspection it may call `ley_search_memory` with exact `revisionCompatibility`; that narrows the historical candidate set only and never promotes divergent/unknown history into current truth. `ley_session_get` also returns read-time checkpoint applicability/current Git freshness while remaining a captured-memory view with `liveSourceChecked: false`. It may use `ley_project_state` for explicit project-status questions, `ley_memory_health` for deliberate non-destructive maintenance review, `ley_agent_legibility` for compact project orientation, and `ley_topic_dossier` for repeated topic orientation, while preserving that all derived historical views remain subject to the relevant egress ceiling. Checkpoint verification may attach `evidenceArtifactPaths` only for directly supporting artifacts already present in the approved capture; returned `evidenceArtifacts` remain immutable historical provenance, not current authority. Agent Legibility is navigation only: keep declared commands distinct from historical observed commands, preserve `tableOfContentsNotScore`/`selectionBasis` semantics, and inspect live source before consequential edits. MCP cannot mutate egress policy. The package uses Claude Code's documented `${CLAUDE_PROJECT_DIR}` placeholder rather than assuming its process working directory.

Context/memory utility feedback is likewise an explicit measurement workflow, not default Claude Code behavior. When deliberately requested, bind the exact compiled pack before the downstream work with `ley_context_utility_bind`, then attach only later typed checkpoint/session-finish outcomes through `ley_context_utility_observe`. The returned metadata is bounded session evidence; it never proves attention or causation and never authorizes automatic memory trust/ranking changes.

Reviewed runbook and Skill conversion also stays outside Claude Code's MCP surface. Only an explicit local-user workflow may run `ley runbook compile` over exact current trusted procedure/pitfall/convention learning IDs and then, after reviewing the returned `runbookId`, run `ley runbook export-skill` with that exact ID plus explicit host and egress target. Export is source-revalidated, egress-checked, and non-installing (`installed: false`). The packaged skill must not silently promote remembered content into a Claude Code Skill or treat an exported artifact as permission or live-source proof.


## What automatic capture does

In Structured and Full Evidence modes, the adapter stores:

- a generated host-session name and continuity goal;
- the stable Ley session ID;
- a bounded, pattern-redacted copy of each observed user prompt;
- a bounded, pattern-redacted copy of the host's final assistant response for each completed turn;
- an opaque Ley-derived turn reference that pairs the two without retaining the host's raw turn identifier.

Minimal mode stores body-free prompt/response observation events so users can see what was omitted without retaining either body.

Each adapter also injects the stable current Ley session ID at session and turn
start so MCP writes continue the same session instead of creating duplicates.
The native turn event is Codex and Claude `UserPromptSubmit`.

It does not automatically store:

- transcript files or transcript paths;
- hidden reasoning;
- tool inputs or complete outputs;
- environment variables;
- arbitrary files outside the approved project capture boundary.

Turn evidence is not a checkpoint and is never promoted into startup context. If a crash or missed checkpoint leaves later turn evidence, Ley can deterministically expose that post-checkpoint window through `ley_session_memory_compile`. The tool is read-only, preserves prompt/response text as untrusted evidence, and distinguishes complete paired evidence from partial or metadata-only capture. Before reconstructing structure, the agent must run `ley_session_memory_verify`: every current recovery record is cited or explicitly deferred, stale/invalid/duplicate/revision cases fail closed, and even `review-required` does not prove semantic faithfulness or live-source correctness. `review-required` is emitted only when no current recovery evidence remains deferred; if any record is intentionally deferred, do not advance the checkpoint boundary. For one unresolved candidate, reconstruction uses `ley_session_memory_commit_unresolved`, which re-verifies the same event count and exact evidence set under the writer lock and stores binding provenance. Concurrent newer evidence forces a recompile and reverify; generic checkpointing is not a substitute for this bound recovery path. For substantive work the bundled skill asks the agent to write typed decisions, tasks, problems, attempts, outcomes, solutions, verification, touched artifacts, unresolved items, and handoff through MCP.

Ley deliberately does not add a global tool logger. Tool calls can contain
credentials, large outputs, or irrelevant details, and a hook cannot reliably
infer their durable meaning. Each bundled skill instead records bounded
commands, touched artifacts, and observed outcomes inside meaningful structured
checkpoints.

## Failure and retry behavior

An uninitialized or unbound project returns `{}` and remains untouched. A stable host session maps to the same Ley session after process restart. Codex pairs retries with its documented stable `turn_id`; Claude Code uses the append-only Ley session state because their pre/post events do not share a stable turn identifier. Exact hook retries replay instead of duplicating, while the same prompt submitted after a completed response becomes a new turn. A new host session receives the normal bounded resume pack, including earlier checkpoint evidence and only user-trusted, artifact-current learnings—not captured prompt/response bodies.

The bundled MCP process also starts cleanly in an ordinary workspace, but advertises zero capabilities and no tools or resources. It never initializes or scans that directory. This keeps a globally installed integration quiet and harmless until the user explicitly sets up Ley for that project.

If a captured snapshot is missing or inconsistent, the hook fails rather than inventing context. Run `ley doctor`, restore the binding if needed, then `ley ingest` deliberately.
