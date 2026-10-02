# ADR 0093: Retire legacy Search MCP wrappers

**Status:** Accepted — 2026-10-02

## Context

Ley's current agent contract is deliberately small: `ley_brief`, `ley_search`, `ley_evidence`, and
`ley_checkpoint` when writes are enabled. Fully canonical native projects already advertise only that
surface, but legacy-vault compatibility mode still exposed three older read routes:

- `ley_search_context` — captured artifact/context search;
- `ley_search_memory` — the predecessor name for canonical Search;
- `ley_search_activity` — a structured Decision/Problem projection.

These routes persisted no route-specific state. Their remaining value therefore had to be judged by unique
user-visible behavior, not by the existence of old tests or compatibility documentation.

The audit found:

1. `ley_search_memory` was a second MCP name for the same implementation and input contract as
   `ley_search`.
2. `ley_search_context` searched captured project evidence already represented by Artifact/Symbol/Dependency
   candidates in canonical Search. For current-state source inspection, coding hosts already have their own
   live workspace tools. Under fine-grained historical egress restrictions Ley intentionally keeps derived
   history fail-closed; canonical Brief remains the authority-aware path that can withhold unproven historical
   derivatives while still admitting eligible direct active-project evidence. Retaining a second raw-context
   MCP route only to bypass Search's historical gate would create two overlapping disclosure contracts.
3. `ley_search_activity` exposed richer cross-session Decision/Problem detail, but canonical Search already
   discovers those records with stable `entityId`/`sessionId` handles. ADR 0092 fixed the concrete downstream
   Problem-preview failure by surfacing matching failed attempts/root causes in bounded Search excerpts. When
   the whole structured episode is actually needed, the existing bounded session reader exposes ordered
   attempts, outcomes, resolution detail, and citations for the returned session/record. The deterministic
   known-failure evaluation can therefore prove Search → session-detail progressive disclosure without a
   separate Activity search product.

Luna xhigh independently classified `ley_search_context` and `ley_search_memory` as immediately redundant.
It initially recommended retaining `ley_search_activity` only until the known-failure evaluation and current
documentation were migrated to canonical Search plus session inspection; that blocker is removed in this
change.

## Decision

Delete the three legacy MCP wrapper routes and their transport-only parameter/schema/test surface:

- `ley_search_context`;
- `ley_search_memory`;
- `ley_search_activity`.

Canonical `ley_search` now owns the MCP search implementation directly instead of delegating through the old
`ley_search_memory` method. Deterministic evaluations call the canonical route. The old opt-in semantic runner
is retired rather than mechanically renamed because canonical native artifact continuity intentionally does
not consult the legacy-vault semantic index; any future native vector lane must be re-earned separately. The
known-failure journey uses canonical Search to recover the stable Problem/session handle and `ley_session_get`
only when it needs the complete structured debugging episode.

This is a model-facing API contraction only. Keep the shared core retrieval/activity implementations wherever
they still have real consumers, including local/Desktop/compatibility internals. Do not migrate or delete
captured artifacts, sessions, learnings, activity records, indexes, or historical event schemas as part of this
decision.

## Egress and trust boundary

This ADR does **not** weaken Search's historical-memory egress gate. If retained fine-grained/source ancestry
makes historical derivatives unsafe to disclose, canonical Search may still fail closed. Canonical Brief is the
task-conditioned path that can suppress those unproven derived candidates while preserving eligible direct
active-project evidence, and the coding host remains responsible for live repository inspection when current
source truth matters.

Search results remain captured historical evidence, not instructions or current truth. Revision/ranking signals
remain retrieval/applicability evidence rather than authority.

## Compatibility

An external legacy MCP client that explicitly calls one of the three retired route names must migrate to:

- `ley_search` for bounded recall/discovery;
- `ley_session_get` after Search when complete structured session detail is genuinely required;
- `ley_evidence` for exact citation-bound evidence;
- the coding host's normal live workspace tools for current source inspection.

No durable data format changes, migrations, erasure changes, or compatibility rewrites are required because the
removed routes were read-only projections over shared state.

Historical ADRs that document the old route names remain historical evidence and are not rewritten to pretend
the APIs never existed.

## Validation

The contraction must preserve:

- canonical Search input constraints, revision filtering, selected-project identity/egress, citations, and
  query-aware Problem previews;
- project-level and fine-grained egress behavior, including Brief's direct-evidence admission while unsafe
  historical derivatives are withheld;
- the deterministic known-failure progressive-disclosure journey through Search → session detail;
- semantic-search evaluation through canonical Search;
- fixed-project isolation and zero path/private-marker leakage;
- packaged host guidance containing only the intended canonical workflow.

Validate the focused MCP crate first, then the affected evaluator lanes and proportionate broad release gates.

## Non-goals

This ADR does not authorize deletion of core Project Activity/search modules, granular session readers, legacy
evidence readers, recovery compatibility, migration state, or unrelated MCP compatibility routes. Those need
their own evidence if they are reconsidered.
