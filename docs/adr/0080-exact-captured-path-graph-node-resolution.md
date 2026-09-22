# ADR 0080: Prefer exact captured file paths in graph node resolution

Status: Accepted

## Context

Ley graph tools accept a node ID or a unique name/path fragment. The fallback resolver previously
treated every path match as substring search after name matching. In a repository containing both
`src/renderer.ts` and `legacy/src/renderer.ts`, querying the exact captured path `src/renderer.ts`
could therefore still return multiple candidates because the longer path contains the same suffix.

That ambiguity hides deterministic graph evidence that already exists. Agents can use opaque node
IDs, but forcing an ID round-trip for a path the caller already knows adds tool friction and weakens
progressive disclosure without adding safety.

Graph symbol nodes also retain their defining artifact path, so an exact path must not simply match
every node carrying that path. A file path denotes the captured `File` node; symbols remain
addressable by stable node ID or unique symbol name/query.

## Decision

Graph node resolution uses this deterministic precedence:

1. exact stable node ID;
2. exact captured project-relative path on a `File` node;
3. exact case-insensitive node name;
4. bounded case-insensitive name/path substring fallback.

The existing ambiguity behavior remains fail-closed at each non-exact stage. This change does not
add fuzzy ranking, path normalization against the live filesystem, or hidden tie-breaking.

## Consequences

- A caller that already knows an exact captured path can use it directly with
  `ley_graph_neighbors` or `ley_graph_path`, even when another path shares that suffix.
- Symbol nodes sharing the same artifact citation path do not make the exact file-path query
  ambiguous.
- Partial path fragments and duplicate names still return candidate ambiguity rather than a guessed
  winner.
- The graph schema and stored graph snapshots do not change; this is read-time retrieval semantics.
- Returned graph evidence remains captured-snapshot evidence with `liveSourceChecked: false`.

## Evaluation

`graph-exact-path-disambiguation` captures both `src/renderer.ts` and
`legacy/src/renderer.ts`, plus a test that imports only the former. The direct lexical baseline must
miss the test. An exact-path graph query must recover the deterministic incoming test relation and
exact reverse path without returning the legacy file or unrelated test and without leaking machine
paths.

A core regression also places a `Symbol` node on the same artifact path and verifies that exact path
resolution still returns only the `File` node.
