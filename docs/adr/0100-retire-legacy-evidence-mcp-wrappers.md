# ADR 0100: Retire legacy evidence MCP wrappers

**Status:** Accepted — 2026-10-02

## Context

Ley's canonical MCP evidence surface is `ley_evidence`. It accepts an exact citation produced by Ley and revalidates
project, snapshot, path, content hash, line range, egress, and media identity before returning bounded text or
supported original image bytes.

Two older MCP routes remained beside it:

- `ley_read_evidence`, which accepted only a captured artifact path plus line bounds and therefore allowed the model
  to construct a path read without carrying the citation snapshot/hash; and
- `ley_read_media_evidence`, which duplicated the same transition media reader already used by canonical Evidence.

Both were already absent from canonical-native MCP mode and current release evaluations use `ley_evidence`.

## Decision

Retire both legacy MCP wrappers:

- remove `ley_read_evidence` and its parameter schema;
- remove `ley_read_media_evidence` and its parameter schema;
- keep canonical `ley_evidence` as the only model-facing text/media evidence reader;
- preserve active-project legacy-transition fallback and selected-project native citation reads inside canonical
  Evidence; and
- keep core evidence readers and historical storage/migration compatibility intact.

For text, the supported agent flow is now `ley_search` or `ley_brief` → exact citation → `ley_evidence`. This removes
the weaker uncited-path convenience rather than replacing it with another route.

## Consequences

- Existing external legacy MCP clients that call either retired tool name must migrate to `ley_evidence`.
- Historical evidence remains readable; no durable state is deleted or migrated by this ADR.
- Canonical Evidence continues to prove exact original-image bytes, byte bounds, no generated description/OCR,
  egress enforcement, and no local-path disclosure.
- The lower-level core path reader remains available for migration/compatibility tests and non-model internals; it is
  no longer exposed as a model-facing arbitrary captured-path MCP operation.

## Non-goals

This ADR does not remove artifact capture, citation generation, content-addressed evidence, legacy-vault fallback,
selected-project Evidence, or the core evidence readers.
