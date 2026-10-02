# ADR 0099: Retire the Consolidation Inbox MCP wrapper

**Status:** Accepted — 2026-10-02

## Context

The Consolidation Inbox is still useful as an explicit local review workflow over meaningful terminal-session
evidence. Its product contract is intentionally conservative: it is rebuilt on demand, returns stable IDs/counts
rather than prompt/response bodies, does not invoke a model, does not start background work, and never grants
learning-write or trust authority.

That workflow no longer needs a model-facing MCP route. Fresh/canonical Ley MCP deliberately exposes only Brief,
Search, Evidence, and optional Checkpoint, while the current P2 consolidation evaluation already exercises the local
`ley consolidation inbox` command. Shipped host packages forbid `ley_consolidation_inbox`, and no current release
evaluation or product workflow depends on that route.

Keeping the wrapper therefore preserves compatibility breadth without preserving a unique capability.

## Decision

Remove only the `ley_consolidation_inbox` MCP wrapper:

- delete its MCP parameter schema and tool handler;
- remove it from legacy MCP tool inventories and instructions;
- keep a protocol regression proving the route is absent; and
- keep the consolidation engine, CLI command, schema, privacy bounds, direct-turn learning evidence support, and P2
  evaluation unchanged.

This is an API-surface contraction, not a consolidation-feature retirement.

## Consequences

- Agents cannot ask Ley MCP to enumerate a consolidation queue.
- Humans/local workflows can still run `ley consolidation inbox PROJECT --json` deliberately.
- Any future background consolidation scheduler must still beat the same on-demand/manual baseline; removing the MCP
  wrapper does not strengthen the case for automatic work.
- Historical data and session/learning migration behavior are unchanged.

## Non-goals

This ADR does not remove `compile_session_memory`, direct retained-turn learning evidence, the CLI inbox, learning
proposal/review flows, or the degraded read-only Memory Compiler used for interruption recovery compatibility.
