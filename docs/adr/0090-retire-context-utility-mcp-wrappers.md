# ADR 0090: Retire unreachable Context Utility MCP wrappers

Status: accepted

## Context

Ley previously exposed two explicit model-facing Context Utility writes:

- `ley_context_utility_bind`, which bound one exact compiled context pack to a session before downstream work; and
- `ley_context_utility_observe`, which later attached typed checkpoint/session-finish outcomes to that binding.

Those writes were deliberately correlation-only. They never proved context use or causation and never changed
learning trust or retrieval ranking. The durable events remain useful historical provenance for sessions created
while that experiment was active.

The focused product subsequently retired the model-facing workflow. Current agent-evaluation code explicitly
records ordinary checkpoint/finish outcomes and reports `observationOmissionReason: context-utility-retired`
instead of creating Context Utility observations. Current learnings documentation likewise states that the two
model-facing routes are retired.

The remaining MCP implementations were already unreachable: both route names were included in
`RETIRED_MODEL_RECOVERY_TOOLS`, and every server disabled those routes after router construction. Their request
types, method bodies, receipt helper, and direct method tests therefore exercised dead code rather than a callable
compatibility surface.

## Decision

Delete the unreachable MCP wrapper layer while preserving the durable historical event model:

1. Remove `ley_context_utility_bind` and `ley_context_utility_observe` tool methods.
2. Remove their request/receipt types and MCP-only helper/import plumbing.
3. Remove the now-redundant retired-route names and session-write disable calls for routes that no longer exist.
4. Remove MCP tests that directly invoked the unreachable methods.
5. Keep all core Context Utility session event types, schema v5/v15 replay/validation, Session projections,
   learning-provenance reads, utility binding/observation compatibility functions, and erasure semantics.

## Compatibility and data safety

This is not a new MCP behavior break: the routes were already disabled on every server and therefore absent from
the advertised live tool inventory. Removing their dead implementations makes the code match that existing
runtime contract.

Historical sessions containing Context Utility binding/observation events remain readable and verifiable. No
stored event is rewritten or deleted, and no migration is required. Core compatibility functions stay because
they are needed to validate/replay historical session ledgers and expose body-free provenance where applicable.

## Consequences

- The MCP implementation no longer carries unreachable mutation code that current docs and runtime routing had
  already retired.
- Historical Context Utility evidence remains provenance only and continues to grant no trust/ranking authority.
- Future utility measurement should use an explicitly designed current workflow rather than reviving these old
  wrappers by accident.
