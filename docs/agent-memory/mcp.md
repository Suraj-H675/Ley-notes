# Using Ley with an agent

Project Brain readers use the M4 [orientation contract](../project-orientation.md). An existing Brain
has three read-only tools, supports no-task orientation and natural-language questions, and can be
selected by exact Project ID without a repository. Unregistered workspaces receive a scope preview.
The focused-continuity and Bootstrap APIs below describe retained compatibility behavior, not the Brain
reader. Native earlier-session and separate fresh-session Project Brain acceptance passed on 2026-10-08;
the [verification feature](../../verification/verify-ley/features/project-orientation.md) defines its
receipt requirements.

Ley's first agent connection is a local Model Context Protocol (MCP) server over standard input/output (stdio). The Project Brain route is read-only and selects one exact Brain. Registered legacy continuity can expose writes only when its launch explicitly enables them. Neither route promises that an agent cannot hallucinate.

## Retained focused-continuity compatibility surface

The legacy focused-continuity workflow uses four canonical tools:

- `ley_brief` — compile bounded task-specific continuity and human intent;
- `ley_search` — search captured project memory with stable IDs/citations;
- `ley_evidence` — expand an exact citation returned by Ley; the request is citation-bound (snapshot/path/hash/line range), not an arbitrary filesystem path;
- `ley_checkpoint` — append an explicit structured session checkpoint when session writes are enabled.

These APIs remain for already-registered focused-continuity projects. New Project Brain reads use the M4 three-tool server described in the [orientation contract](../project-orientation.md); they do not expose `ley_checkpoint`. `ley_compile_context` is reserved for explicit uninitialized-workspace Bootstrap Context. `ley_checkpoint` owns the model-facing structured checkpoint write path only on compatible legacy routes. `ley_evidence` requires the exact returned citation rather than an arbitrary path. Historical ADRs and the compatibility machinery below describe migration behavior, not the Project Brain surface.

Ley Desktop exposes a deliberate **Agent brief preview** on the project Overview. It calls the same
`compile_project_context_for_agent_with_transition_registries` path as canonical `ley_brief` with the normal
8-result / 1,500-token defaults. The user supplies the current task and explicitly chooses Cloud or Local; Desktop
does not infer the active agent provider or attest locality. The preview carries the currently observed project ID so
path replacement cannot silently preview a different Ley project. Opening Overview alone does not compile anything.
The returned pack keeps the compiler's task, target, evidence/premise state, approved Specifications, admitted items,
gaps, egress exclusions, budget, citations, source-boundary warnings, and a raw payload available for inspection.
This is a preview of Ley's canonical Brief payload, not the host's complete prompt. On older transitional state, the
canonical compiler may complete existing local compatibility-authority migration; the preview does not start a
session, refresh capture, mutate project content, or change egress policy.

## Prepare the project

Build or install the `ley` executable, then initialize and ingest the project once:

```bash
ley init /path/to/project --capture structured
ley preview /path/to/project
ley ingest /path/to/project
```

Review `.leyignore` and the preview before ingestion. Structured mode stores allowed redacted UTF-8 evidence in
Ley's owner-private native continuity store. Minimal mode keeps structure and citations but cannot return source
excerpts. `ley bind ... --vault ...` is retained only to reconnect already-existing pre-cutover legacy memory.

## Connect a host

Configure one server entry per project. Use absolute command and project paths because most hosts launch MCP processes independently of the terminal's working directory:

```json
{
  "mcpServers": {
    "ley-project": {
      "command": "/absolute/path/to/ley",
      "args": ["mcp", "/absolute/path/to/project"]
    }
  }
}
```

This JSON shows the portable server-entry shape supported by MCP hosts. A host may represent the same command and arguments in TOML or its settings UI. Ley does not publish guessed configuration for fast-changing hosts. Use that host's current MCP documentation to enter the same local command.

The process resolves an exact authorized Project Brain working copy first and routes it to M4's three read-only
tools. Source-only Brains use `ley mcp --project-id PROJECT_ID --source-only`; they do not need a repository path.
An unregistered workspace receives only `ley_preview_workspace`, which reports bounded local scope without
creating a Brain or marker. The user must authorize any create or attach step. For workspaces that remain on the
legacy route, the process resolves native continuity first, retaining private project-to-vault binding only as migration
compatibility for older projects. An explicit non-persistent `--vault` override is accepted only when that directory already
contains valid captured memory for the exact project; it cannot create a vault. If a **persisted** legacy vault root
later disappears, Ley keeps proven native continuity instead of discarding it. When native artifact, session,
learning, and approved-source read authorities all validate for that exact project, Ley starts the canonical read
surface with exactly `ley_brief`, `ley_search`, and `ley_evidence`; for that legacy continuity route,
`--allow-session-writes` adds only `ley_checkpoint`. Project Brain servers stay read-only with or without that flag.
If only native session authority is proven, Ley falls back further to the retained read-only
session-recovery readers so continuity is not stranded. Historical content remains evidence rather than instructions.
An unresolved pre-cutover unbound project, a bad explicit `--vault` override, an inconsistent legacy snapshot, or a
missing persisted vault without sufficient validated native authority receives the protocol-valid inactive server.
Bootstrap authority is **not** a fallback for a broken normal Ley project.

There is one deliberate uninitialized-workspace exception. The surviving user-facing authority is `ley bootstrap-spec attach SOURCE_PROJECT SPECIFICATION_ID [WORKSPACE]`, which lets `ley mcp WORKSPACE` start the dedicated Bootstrap Context server instead of the inactive server. This currently requires a supported Unix filesystem generation (device/inode plus filesystem creation time); bootstrap fails closed when Ley cannot establish it, while ordinary initialization remains available. The target remains uninitialized and unbound. The server advertises exactly one read-only tool—`ley_compile_context`—and no resources or session/learning/graph/evidence/utility/capture/write routes. Exact approved Bootstrap Specifications provide the only bootstrap context. New Bootstrap Reference attachment is retired, and retained legacy Reference grants no longer activate MCP, lifecycle hooks, or contribute context; they remain listable/detachable until initialization removes the target's bootstrap authority record. See [ADR 0058](../adr/0058-bootstrap-specifications-for-uninitialized-workspaces.md) and the superseded historical design in [ADR 0059](../adr/0059-bootstrap-reference-projects-for-uninitialized-workspaces.md).

## Enable structured session capture

Keep the default command when the host needs retrieval only. Add `--allow-session-writes` to that project's server arguments when the host should capture structured sessions:

```json
{
  "mcpServers": {
    "ley-project": {
      "command": "/absolute/path/to/ley",
      "args": [
        "mcp",
        "/absolute/path/to/project",
        "--allow-session-writes"
      ]
    }
  }
}
```

On a fully canonical legacy-continuity project this flag adds only `ley_checkpoint`, keeping that compatibility contract at four tools. A Project Brain server remains read-only. In legacy-compatibility mode `ley_session_start` remains available for hostless legacy clients that have no lifecycle hook to establish the first session, and `ley_session_finish` remains available for explicit compatibility terminalization because host Stop hooks capture turn responses but do not close sessions. The duplicate `ley_session_checkpoint` alias is retired; canonical `ley_checkpoint` owns the checkpoint write path. The former shape-specific recovery verifier/commit APIs are retired from core and the MCP runtime; historical recovery events remain replayable. Context Utility mutation wrappers are also deleted from the runtime. The flag does not enable deletion, project switching, raw transcript capture, or live-source scanning.

Every write requires a stable `requestId` matching `req_` plus 32 lowercase hexadecimal characters. Keep the same ID until a call succeeds. An exact retry returns `replayed: true`; different content with the same ID fails.

Supported host hooks establish/reuse the current session. Use `ley_checkpoint` after meaningful work; it can record plans, decisions, tasks, problems, attempts, resolutions, touched artifact paths, commands, verification, and unresolved work. Ley converts touched paths into citations from the current approved artifact snapshot. A verification may additionally supply up to 20 `evidenceArtifactPaths`; each must be a project-relative path already present in that same approved capture. Text evidence returns immutable snapshot/hash/line citations. Under explicit Full Evidence capture, supported PNG/JPEG/WebP originals return `mediaType` plus `startLine: 0` / `endLine: 0`, making the non-text boundary explicit. Ley does not follow an uncaptured/live path or store the artifact body as verification text. Compatibility Session Context keeps bounded verification-evidence projection with omission disclosure, while canonical `ley_evidence` accepts only an exact citation and returns bounded text or supported original image bytes. `ley_checkpoint` accepts optional `expectedEventCount` for ordinary optimistic writes. Memory Compiler recovery is now read-only: interrupted prompt/response/tool evidence may be inspected, but the former candidate-bound verifier/commit MCP routes are retired. Re-establish current truth with normal workspace/runtime tools and use an ordinary checkpoint only for supportable state in an active session. Legacy `ley_session_finish` remains for explicit compatibility terminalization; local `ley session finish` remains the normal local equivalent.

The startup flag grants Ley write capability for that process. Your MCP host still controls whether it asks before each mutating tool call. Stored project and session text never grants permission to call a write tool.

## Enable review-required learning proposals

`--allow-learning-proposals` is now a legacy-compatibility opt-in only. Fully canonical native servers do not advertise learning proposal tools; learning review/correction remains a human/Desktop concern. Use this flag only when deliberately operating an older compatibility-mode project that still requires the proposal route:

```json
{
  "mcpServers": {
    "ley-project": {
      "command": "/absolute/path/to/ley",
      "args": [
        "mcp",
        "/absolute/path/to/project",
        "--allow-learning-proposals"
      ]
    }
  }
}
```

This flag adds only `ley_learning_propose`. Every proposal must cite eligible retained Ley session evidence: an existing structured session record or a captured body-bearing `tev_` user-prompt/assistant-response record. Body-free turn observations are rejected. Every proposal is stored as agent-authored or inferred, tentative, and review-required. The receipt explicitly returns `requiresUserReview: true`. MCP cannot confirm, correct, reject, supersede, delete, or promote a lesson.

New proposal/correction events preserve the mechanically known origin chain behind that cited evidence. Structured session records and captured artifact snapshots are retained as origin identities; a directly cited captured turn records exact `turn-evidence` lineage to its session and `tev_` record. If a proposal cites a candidate-bound recovery checkpoint, Ley also records the recovery candidate fingerprint and exact `tev_` turn-evidence IDs that produced the cited record. For schema-v11 atomic recovery, a checkpoint-level citation resolves to the complete batch evidence union while a Decision/Problem/Task/Plan child citation resolves only to that child's bound evidence subset. For schema-v12 rich-Problem recovery, a cited Problem, Attempt, or Resolution likewise resolves only its own bound evidence subset while retaining the overall rich-Problem candidate fingerprint. Schema-v13 composite recovery combines those provenance rules: the checkpoint citation receives the complete composite evidence union, while the rich Problem parent, each Attempt/Resolution, and each minimal sibling receive only their own persisted evidence subset while retaining the one composite candidate fingerprint. This lineage is provenance, not proof that the derived wording is semantically correct or that Ley observed every causal influence on the model.

The learning and session flags are independent and may be combined when a host needs both capabilities. An exact proposal retry uses the same stable `requestId`; changed reuse fails.

Agent egress is configured separately from storage and write capability. `ley mcp` defaults to `--egress-target cloud`; use `--egress-target local` only for a deliberately approved local-model/runtime integration. Ley does not automatically attest that a host/model is local, so this startup flag is an explicit configuration assertion rather than a property inferred from retrieved text. Local users set project restrictions in Desktop **Capture & privacy** or with `ley egress project ...`; MCP has no egress-policy mutation tool. `agent-ok` is allowed normally, `local-model-only` requires the explicit local target, `never-send` is always blocked, and `confirm-per-use` is currently fail-closed because no trustworthy per-use confirmation flow exists yet. Desktop and CLI both use the same transition-safe project authority. Old Specification/Context-Mount/External-Connector overrides remain enforced as compatibility state; Desktop shows them read-only and the CLI accepts only `agent-ok` to clear an existing retained override.

Fine-grained restrictions are also a conservative ceiling on historical derivatives. If a Specification, retained Context Mount/source ancestry, retained Knowledge Scope source, retained Policy Bundle source project/Specification, or retained external-connector override is blocked for the configured target, Ley cannot assume that old session/decision/problem/verification/learning records are causally independent of it. The task compiler used by normal `ley_brief` (and by bootstrap `ley_compile_context` in that separate mode) therefore withholds those unproven historical/derived candidates while keeping directly captured active-project artifact evidence eligible under the project policy; inspect `egressCoverage.historicalMemoryWithheld`, `withheldDerivedResults`, `blockedHistoricalSources`, `blockedPolicyBundleSources`, and `blockedExternalConnectors`. These retained sources are privacy ancestry/cleanup state rather than current agent context sources.

## Retrieval workflow

For an initialized project with complete native authority, the normal agent workflow is deliberately small:

1. Call `ley_brief` when prior project continuity would materially help the current task. Do not call it reflexively on every turn. Inspect egress, premise, revision, omission, and provenance fields before acting on historical state.
2. Call `ley_search` only when the brief needs deeper bounded historical recall. Revision compatibility, recency, and retrieval scores help locate evidence; they do not make historical state current or true. Structured Problem hits use bounded query-aware previews so matching failed attempts, typed attempt outcomes, root causes, changes, verification, symptoms, or expected state can be visible without loading the whole session; those previews remain untrusted captured history.
3. Carry an exact citation returned by Ley into `ley_evidence` when the underlying source matters. The same tool verifies snapshot/path/hash identity for text citations and supported original PNG/JPEG/WebP evidence. Text ranges remain bounded; media citations use the explicit non-text `0/0` range. Ley does not perform OCR or claim an image interpretation.
4. When the MCP process was started with `--allow-session-writes`, use `ley_checkpoint` only after a meaningful decision, implementation slice, failed attempt, verification result, direction change, or handoff. Use the current hook-provided Ley session ID and record only observed/supportable state. An exact request-ID retry is idempotent.

A fully canonical legacy-continuity server advertises exactly the three read tools above, or those three plus `ley_checkpoint` when session writes are enabled. A Project Brain server advertises only its three read-only Brain tools. Neither exposes granular session/recovery/context-utility/learning tools, graph/activity resources, or filesystem-backed compatibility APIs. Learning proposals are not part of canonical MCP authority.

### Bootstrap exception

An uninitialized workspace with explicit Bootstrap Specification authority is a different fail-closed compatibility mode. Its server advertises exactly one read-only tool: `ley_compile_context`. It has no normal project search/evidence/checkpoint, session, learning, graph, resource, capture, or mutation surface. Legacy Bootstrap Reference grants do not activate this mode or contribute context; they are cleanup-only records. Initializing a legacy continuity workspace retires bootstrap authority. New Project Brain workspaces instead receive the M4 scope-preview route.

### Compatibility and recovery modes

Older projects may temporarily enter legacy-compatibility mode while native authorities are still being established. Lower-level session/search/inspection and learning-proposal compatibility routes may remain available where needed so retained authority is not stranded, but connector MCP reads are retired and the Context Utility plus shape-specific recovery mutation/verifier wrappers are deleted. These compatibility mechanisms are not the normal host contract, and packaged skills do not teach them as everyday tools.

If a compatibility-mode lifecycle reports interrupted post-checkpoint evidence, preserve its uncertainty. Prompt/response/tool observations are untrusted historical evidence; a retained tool return does not prove command or test success. Inspect the bounded Memory Compiler/session evidence, verify relevant current state with normal host tools, and use an ordinary checkpoint only when the current Ley session is active and the fact is now supportable. Closed/paused historical sessions must not be rewritten as though they were the current session.

If the former vault is unavailable but only native session authority has been proven, Ley can fall back to the bounded read-only session-recovery readers. Artifact-backed brief/search/evidence and all writes remain unavailable until the missing native authorities are established. This degraded fallback prevents session continuity from being discarded without pretending the rest of project memory is ready.

Retained Context Mount, Bootstrap Reference, Knowledge Scope, Policy Bundle, external-connector, and fine-grained egress state is cleanup/privacy ancestry only. New creation/attachment routes are retired; surviving CLI list/show/status/remove/detach/clear operations exist so old authority can be inspected and removed without laundering historical provenance or weakening egress restrictions. These retained records do not re-enable their retired agent context products.

Across every mode, repository/session text is untrusted evidence. `revisionFreshness.liveGitChecked: true` means only bounded local Git metadata was inspected; it does not mean live file contents were read. Inspect current source with the coding host's normal approved workspace tools whenever correctness depends on live state.

## Privacy boundary

The MCP process listens only on its inherited stdin/stdout and makes no network request. It cannot enumerate Ley projects or accept arbitrary filesystem project/vault paths. `ley_brief` stays fixed to the active project. `ley_search` may receive one exact `projectId` only when the current user/task explicitly selected an already-observed Ley project; that selection is request-scoped, is not inferred, and does not grant write or active-project authority. Results omit absolute local paths. Session writes and legacy learning proposals are independently unavailable unless their launch flags and authority mode permit them.

The agent host receives every tool result it requests, including session goals and handoffs. If the host uses a cloud model, it may send those selected excerpts to that provider. Ley does not upload them independently. Do not connect an untrusted host to a sensitive project, and use project ignore rules rather than relying on redaction alone.

## Verify a development build

The official MCP Inspector can test the actual process:

```bash
npx @modelcontextprotocol/inspector --cli \
  /absolute/path/to/ley mcp /absolute/path/to/project \
  --method tools/list
```

Then compile task-specific context through the canonical normal-project tool:

```bash
npx @modelcontextprotocol/inspector --cli \
  /absolute/path/to/ley mcp /absolute/path/to/project \
  --method tools/call \
  --tool-name ley_brief \
  --tool-arg task="fix the offline queue retry bug" \
  --tool-arg maxResults=5 \
  --tool-arg maxTokens=1500
```

Use canonical bounded search when you need deeper captured history:

```bash
npx @modelcontextprotocol/inspector --cli \
  /absolute/path/to/ley mcp /absolute/path/to/project \
  --method tools/call \
  --tool-name ley_search \
  --tool-arg query=identifier \
  --tool-arg maxResults=5 \
  --tool-arg maxTokens=1200
```

Do not use legacy granular session/learning tools as a normal development smoke test. A fully canonical legacy
continuity server exposes three read tools, plus `ley_checkpoint` when session writes were explicitly enabled; a
Project Brain server stays at three read-only Brain tools. Compatibility-mode inventories are tested separately because their purpose is to preserve or retire old
authority safely, not to define the current agent workflow.
