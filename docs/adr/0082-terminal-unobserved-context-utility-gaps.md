# ADR 0082: Surface terminal unobserved context-utility measurement gaps

Status: Accepted

## Context

ADR 0048 deliberately separates pre-work context binding from later typed outcome observation. That
keeps utility evidence provenance-correct, but it also means a binding can remain in a session with no
later `ley_context_utility_observe` event. While a session is still active, that is ordinary workflow
state: the downstream outcome may not exist yet. Once the session is completed or abandoned, however,
an unobserved binding becomes a deterministic measurement gap.

Ley must not turn that gap into a context-quality judgment. Missing observation does not prove the
agent used the pack, ignored it, succeeded because of it, failed because of it, or found any included
record helpful/unhelpful. The existing unsupported Memory Health idea
`chronically-retrieved-but-unhelpful-memory` therefore remains unsupported.

The previous session projection also exposed only total binding/observation counts and recent
observations. A Memory Health signal could therefore cite a binding ID without giving callers a
bounded body-free way to inspect that unobserved binding's provenance metadata.

## Decision

Three coordinated read-time projections are added without changing the durable context-utility event
schemas.

### Session coverage

`ley_session_get` derives `unobservedContextUtilityBindingCount` from unique binding IDs referenced by
the session's observations. It does **not** compute this as `bindingCount - observationCount`, because
one binding may have more than one valid observation.

The projection also returns at most five recent `unobservedContextUtilityBindings`. Each row contains
only body-free provenance/measurement metadata:

- binding/event/context-pack IDs;
- recorded time and captured artifact/graph snapshot IDs;
- egress target when present;
- compile result/token limits and estimated token count;
- retained/omitted included-record counts;
- `contextPackRevalidated` and `contextUsageProven`.

It does not copy the bound task excerpt, Specification text, memory/source excerpts, or included-record
handles. Omitted rows are disclosed through `omittedUnobservedContextUtilityBindings`, and projection
truncation remains explicit.

### Memory Health v3

Memory Health schema v3 adds `unobserved-context-utility-binding` with `review` severity when all of
the following hold:

- the session is terminal (`completed` or `abandoned`);
- a retained context-utility binding has no observation referencing its binding ID; and
- that binding retained at least one included-record handle, or disclosed omitted included records.

The signal cites only the session ID and binding ID plus bounded record-count metadata. It copies no
task/context body. The signal disappears if a later explicit observation validly references that
binding, including the terminal event where permitted by ADR 0048.

The signal means only: **Ley has provenance that a non-empty logical context pack was bound before
work, but its terminal session has no attached typed utility observation.** It does not prove context
usage, helpfulness, harmfulness, causation, or a trust/ranking change.

## Consequences

- utility-measurement coverage is inspectable instead of silently incomplete;
- Memory Health can point to a real follow-up handle rather than a dead-end diagnostic ID;
- active work is not nagged for an observation before an outcome exists;
- terminal measurement gaps remain advisory and reversible by a later valid observation;
- no new analytics database, scoring system, durable schema, or context-body retention tier is added;
- `chronically-retrieved-but-unhelpful-memory` remains unsupported.

## Evaluation

`context-utility-unobserved-binding-health` runs the real MCP flow:

1. compile a pack that contains real admitted context;
2. bind it to an active session;
3. finish the session without an observation;
4. require `ley_session_get` to report one unobserved binding plus one bounded body-free metadata row;
5. require Memory Health v3 to emit exactly one review signal tied to that session/binding;
6. require the context-body canary and absolute project/vault paths to remain absent.

Core tests additionally prove active sessions are not signaled, a later valid observation removes the
signal, and more than five unobserved bindings are bounded with omission/truncation disclosure.
