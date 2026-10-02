# ADR 0089: Retire the Context Pack Inspector compatibility route

Status: accepted

## Context

The Context Pack Inspector was introduced as a non-persistent diagnostic manifest over one recompiled context
pack. It could explain included-record metadata, exclusions, premise/conflict diagnostics, revision/retrieval
signals, and budget composition without copying admitted context bodies.

The product surface subsequently contracted to four canonical initialized-project MCP tools:
`ley_brief`, `ley_search`, `ley_evidence`, and optional-write `ley_checkpoint`. The standalone Inspector stopped
being part of current release claims, current host workflows, packaged Skills, CLI guidance, and normal native
MCP routing. It remained reachable only because the broad legacy-vault compatibility server still advertised
`ley_context_pack_inspect`.

The retained implementation had no independent durable state:

- it persisted no manifest;
- it recompiled the current context pack on every call;
- it could not reconstruct an older supplied pack when the logical `contextPackId` no longer matched;
- its core module had no consumer other than the legacy MCP route;
- current Brief/Search already expose the compact authority, coverage, premise, conflict, provenance, and
  citation signals that remain part of the focused product.

ADR 0088 further removed the Inspector's copied follow-up-handle field, leaving even less unique product value.

## Decision

Retire the standalone Context Pack Inspector completely:

1. Remove the `ley_context_pack_inspect` MCP route, including from legacy-vault compatibility servers.
2. Delete `InspectContextPackParams` and the legacy server instruction/schema/tool-list surface for the route.
3. Delete the self-contained `context_pack_inspector` core module and its exported manifest types/schema version.
4. Keep canonical compiler/Brief diagnostics and egress enforcement unchanged.
5. Keep `contextPackId` itself because other retained historical utility/binding records still use exact pack
   identity; retiring the Inspector does not change those storage contracts.

## Compatibility and data safety

This is an intentional legacy MCP compatibility break: an external client that explicitly called
`ley_context_pack_inspect` against an old-vault project will no longer see that tool. No current first-party
workflow or current product documentation depends on it.

There is no data migration or deletion consequence. The Inspector persisted nothing, created no authority, and
owned no durable records. Historical context-pack IDs, bindings, observations, and source evidence remain
unchanged.

## Consequences

- The runtime no longer carries a separate explanation product whose functionality had already been removed from
  current release claims.
- Legacy MCP inventory becomes smaller without weakening Brief/Search/Evidence safety or provenance.
- Future deeper "why was this admitted/withheld?" UX must be earned by actual user/agent need and should fit the
  focused Brief/Evidence workflow rather than silently recreating a standalone Inspector.
- Historical Inspector ADRs and evaluations remain evidence for past design work, not current API commitments.
