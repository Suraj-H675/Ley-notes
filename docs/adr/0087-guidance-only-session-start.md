# ADR 0087: Guidance-only initialized SessionStart

Status: accepted

## Context

Initialized Codex and Claude Code `SessionStart` previously rendered a bounded `ProjectResumePack` before the
agent had a concrete task. That pack could include recent session goals, latest checkpoint summaries, handoffs,
and reviewed/trusted learnings. C1 added revision-aware withholding after a real divergent-branch fixture showed
that stale branch history could otherwise enter startup context.

The stronger remaining question was whether any contentful automatic startup history still earned its disclosure,
context, and steering surface now that canonical `ley_brief`, `ley_search`, and `ley_evidence` are available on
demand. A dedicated C4 evaluator therefore compared the shipped revision-safe contentful startup with a
benchmark-only guidance-only startup while holding the current HostHook session identity, current prompt capture,
repository state, hidden oracle, model, sandbox, and live canonical four-tool Ley MCP access constant. The
guidance-only arm was not forced to retrieve; whether the agent called Ley was part of the measured behavior.

The final 2026-10-01 study ran 24 isolated Codex `0.159.3` / `gpt-6-luna` / `xhigh` attempts: six risk classes ×
two startup arms × two repetitions. Both arms passed 12/12 tasks and 12/12 hidden oracles with zero MCP server
failures. Guidance-only startup averaged 1,188.7 context characters versus 1,983.8 for contentful startup
(~40.1% less), mean runner time was effectively unchanged (85.70s vs 86.13s), and mean retrieval calls were
0.83 vs 0.75. Guidance-only exposed zero forbidden stale markers; contentful startup exposed four stale
same-lineage markers across the two stale-history attempts.

## Decision

Adapter schema version 7 makes initialized `SessionStart` **guidance-only**.

For an allowed initialized project:

1. Ley creates or reuses the stable HostHook session identity.
2. Ley returns the current Ley session ID plus concise retrieval/checkpoint guidance.
3. Ley does **not** automatically read or emit prior session bodies, checkpoint summaries, handoffs, reviewed
   learnings, Specifications, mounts, scopes, policy bundles, or other historical project bodies.
4. If the same current Ley session has unconsolidated post-checkpoint turn evidence, Ley may still emit the
   existing body-free recovery count/state signal. Retained prompt/response bodies remain uninjected.
5. `ley_brief` is the normal deliberate task-conditioned continuity surface. `ley_search` and `ley_evidence`
   provide deeper cited recall, and `ley_checkpoint` records supported current-session state.
6. Project-level egress still gates the lifecycle hook before session mutation. Fine-grained historical-source
   egress is no longer evaluated by `SessionStart` because the hook no longer emits historical source content;
   explicit retrieval surfaces keep their source-level egress checks.

The explicit `ProjectResumePack` API remains available for deliberate local inspection and keeps its bounded
revision-aware safety rules. This ADR removes only automatic host startup contribution.

Bootstrap Specifications are unchanged. Their uninitialized-workspace `UserPromptSubmit` path remains the
separate explicit Bootstrap-authority exception from ADR 0058.

## Consequences

- Startup disclosure and pre-task steering shrink structurally instead of depending on increasingly complex
  admission rules for context the agent may not need.
- Agents may make an explicit retrieval call when continuity matters; the controlled study found no task/oracle
  regression and no meaningful aggregate latency penalty on the tested slice.
- Automatic reviewed-learning and handoff delivery is retired from initialized host startup. Those records remain
  available through deliberate Brief/Search/Evidence and explicit Resume inspection.
- The `ContextWithheld` host disposition and fine-grained historical startup-withholding branch are removed because
  schema-7 SessionStart has no historical body to withhold.
- The C1 divergent-session Resume filter remains useful for explicit Resume consumers even though host startup no
  longer consumes that pack.
- Reintroducing contentful initialized startup requires new controlled evidence showing a material downstream
  advantage that justifies automatic disclosure and steering.
