# ADR 0057: Automatic bounded host task context

Status: accepted

## Context

Ley already has two complementary agent-context paths:

- lifecycle `SessionStart` provides a compact continuity/resume brief before the user's concrete task is known;
- `ley_compile_context` provides the authority-aware, egress-aware, task-conditioned Context Compiler once an agent explicitly calls MCP.

Codex and Claude Code both expose the submitted user prompt to a synchronous `UserPromptSubmit` hook before the model handles the turn. The existing Ley adapter used that event only for bounded prompt capture, then relied on the bundled Skill to remind the model to call `ley_compile_context`. This made the highest-value Context Plane operation dependent on model/tool choreography even though the lifecycle adapter already had the exact current task and the required local authority registries.

The North Star requires a small bounded context pack for the current task/session while preserving explicit scope, egress, provenance, uncertainty, and live-source honesty. Automatic task context must not become a second compiler, a hidden persistence tier, an ambient project search, or a new write/authority path.

The host output also needs its own bound. Current Codex hook behavior uses a finite `additionalContext` threshold and may spill oversized output to disk; Claude Code similarly caps each `additionalContext` string before replacing it with a path/preview. Ley should remain comfortably below those host spill paths rather than serialize the complete MCP payload into every prompt turn.

## Decision

Host adapter schema version 4 extends `UserPromptSubmit` for installed Codex and Claude Code integrations.

### Preserve startup continuity

`SessionStart` remains the broad continuity layer. It still establishes/reuses the stable Ley session, returns bounded resume state when allowed, exposes only a recovery count/state signal for post-checkpoint turn evidence, and applies the existing conservative historical-memory egress ceiling.

Prompt-time compilation complements rather than replaces that startup orientation.

### Compile the exact bounded task after turn capture

For an allowed `UserPromptSubmit`:

1. Ley performs the existing idempotent bounded prompt-evidence append first.
2. The prompt is whitespace-normalized only for the compiler query.
3. Ley does not truncate, summarize, or semantically rewrite the task to fit retrieval.
4. If that exact normalized task exceeds the existing project-memory query bound or contains unsupported control content, automatic compilation is skipped and the hook returns a generic explicit-compile fallback notice.
5. Otherwise Ley invokes `compile_project_context_for_agent_with_registries` with the normal default `8` results / `1,500` estimated-token budget.

The automatic path therefore uses the same current active-project Specification authority, Policy Bundles, Context Mounts, Knowledge Scopes, project/source egress policies, premise adjudication, revision semantics, conflict handling, abstention, and retrieval logic as `ley_compile_context`. No weaker lifecycle-only retrieval algorithm is introduced.

### Use a compact non-persistent host renderer

The hook does not serialize the entire `CompiledContextPack`.

It renders a compact model-visible projection containing, as space allows:

- the logical `contextPackId`, project/snapshot identity, evidence state, premise state, and compiler budget;
- live-source / Git freshness boundaries;
- bounded egress-withholding counts;
- premise warnings;
- current approved active-project Specification excerpts with stable IDs and human-intent authority;
- admitted Policy Bundle excerpts with bundle/spec/source provenance;
- admitted active-project memory with stable IDs, authority/trust/revision labels, title, and bounded excerpt;
- admitted explicit mounted/shared references with source/scope provenance and bounded excerpt;
- conflicts, gaps, and stable follow-up handles;
- explicit counts for content omitted by the host renderer and by compiler budgeting.

Copied bodies are individually clipped and the automatic task-context block has a strict 3,500-byte Ley limit. The renderer reserves space for its final omission disclosure instead of silently cutting the string. Codex's packaged `UserPromptSubmit` hook sets `additionalContextLimit: 5000`; Claude Code uses its own larger fixed string ceiling. Ley does not rely on either host's oversized-output spill file as the normal delivery mechanism.

The raw user prompt is intentionally not repeated inside the automatic task-context block. The host already carries that prompt as the user turn, and duplicating it would consume context and increase spill/privacy risk. This does not change the existing bounded local prompt-evidence retention policy.

### Failure and retry semantics

- A project-level egress denial remains a host-valid no-op **before** Ley creates or appends a host session/turn.
- Fine-grained restrictions are handled by the normal Context Compiler and are disclosed through bounded withholding diagnostics; blocked source text is not reconstructed.
- If automatic compilation otherwise fails, the already-allowed prompt turn remains captured and the user prompt proceeds. The hook exposes only a generic content-free fallback directing the agent to an explicit concise `ley_compile_context` query when useful; raw local errors and machine paths are not injected.
- Exact host prompt retries retain the existing idempotent turn-evidence append. Ley recompiles the current task projection on retry rather than persisting/caching a compiled pack. With unchanged authority/memory state the logical pack remains stable; if current permitted context changed, the retried hook may truthfully return the newer pack.
- Automatic injection creates no `ley_context_utility_bind`. Utility measurement remains an explicit workflow requiring an explicit compiler call immediately followed by the bind.

### Authority and persistence boundaries

Automatic task context:

- does not initialize, bind, ingest, mount, attach scopes/bundles, approve Specifications, or mutate egress policy;
- does not checkpoint, propose/confirm learning, finish sessions, or otherwise create semantic memory;
- does not persist another context-pack body/manifest inside Ley;
- does not claim the host itself will not retain injected context according to the host's own conversation/session behavior;
- never changes source authority merely because text was automatically supplied;
- keeps `liveSourceChecked: false`; bounded Git metadata is not a live file-content check.

The normal explicit MCP compiler remains available for refined/materially changed tasks, deeper results/budgets, fallback cases, and deliberate context-utility measurement.

## Consequences

- Codex and Claude Code can receive task-specific Ley context without depending on the model to remember an initial compiler tool call.
- Startup remains small and continuity-oriented rather than becoming an all-memory dump.
- Hook latency now includes one bounded local compiler operation on eligible prompt submissions.
- Automatic task context can be smaller than the complete MCP response; host-rendering omissions are explicit and deeper retrieval remains progressive.
- Very long prompts do not receive silently truncated or reinterpreted automatic retrieval. The model can formulate a concise explicit retrieval query when needed.
- Exact retries may recompile rather than replay a persisted host-context blob, avoiding a new sensitive cache while preserving turn-capture idempotency.

## Verification requirements

The slice must prove at least:

- Codex and Claude Code `UserPromptSubmit` receive the compact automatic block;
- exact retry does not duplicate prompt evidence;
- a raw prompt-only marker is not echoed by the host renderer;
- approved task-relevant Specification content can be supplied through normal authority;
- fine-grained cloud egress withholds restricted Specification text while disclosing the omission;
- project `never-send` remains a no-op before session mutation;
- oversized exact tasks fall back without truncation;
- automatic output remains under the local host-rendering bound and reports omissions;
- the real `ley hook` CLI path exercises automatic task context for both Codex and Claude Code;
- existing SessionStart/recovery/Stop semantics remain green.

## Deferred

This ADR does not add:

- automatic context-utility scoring/binding;
- a persisted per-turn context-pack history;
- model-generated task summarization for oversized prompts;
- background/asynchronous context compilation;
- automatic live-source reads;
- automatic setup, capture expansion, mounts/scopes/bundles, or egress changes;
- broader host integrations beyond the existing Codex/Claude Code lifecycle contracts.

## Primary host references

- Codex hooks: <https://developers.openai.com/docs/hooks>
- Claude Code hooks: <https://code.claude.com/docs/en/hooks>
