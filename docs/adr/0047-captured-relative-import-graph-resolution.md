# ADR 0047: Resolve unambiguous captured relative imports to project files

Status: Accepted

Later extension: ADR 0078 applies the same captured-path-only rule to explicit Python relative import
targets while leaving absolute Python imports and ambiguous module/package matches external.

## Context

Ley's project graph already extracts deterministic `imports` edges from captured JavaScript and TypeScript syntax, but every import target is represented as an external-module node. That preserves safety, yet it throws away a relationship Ley can sometimes prove exactly: a relative module specifier such as `../src/renderer` can identify one and only one file already present in the same approved captured artifact set.

The P1 roadmap calls for richer graph relations only when extraction and evaluation prove they improve retrieval. `LEY.md` names a concrete graph question: after changing an implementation, which tests are likely relevant? A test that directly imports that implementation is a high-value deterministic relationship; resolving it does not require an LLM, package manager, compiler, language server, live filesystem walk, or new authority type.

## Decision

Keep the existing graph schema, `File` node kind, and `Imports` edge kind. During graph construction, a captured JavaScript/TypeScript relative import may target the existing captured `File` node instead of an `ExternalModule` node only when Ley can resolve the specifier to exactly one captured project-relative path.

- Only `./` and `../` JavaScript/TypeScript imports participate in this first slice.
- Resolution is lexical over the approved captured path set; it never probes the live filesystem.
- Exact captured paths are accepted. Extensionless TypeScript imports may try `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs` and matching `index.*`; JavaScript imports use `.js`, `.jsx`, `.mjs`, `.cjs` and matching `index.*`.
- If zero or more than one captured path matches, Ley keeps the existing deterministic `ExternalModule` edge rather than guessing.
- Package/bare imports stay external. Backslash-containing specifiers and specifiers containing query/hash suffixes are not locally resolved in this slice.
- Lexical `..` traversal cannot escape the captured project root. An escape attempt stays unresolved/external.
- The edge remains `FactProvenance::Deterministic`, confidence `1.0`, and cites the captured import statement that produced it.
- Cross-file symbol-name resolution remains deliberately out of scope. A call/reference named `renderFrame` is not upgraded to a project symbol merely because another file defines a matching name.

The public graph tools need no new surface. Existing `ley_graph_neighbors` and `ley_graph_path` traversal over `imports` can now move between an importing captured test/module and its captured implementation when the relationship is exact.

## Derived-state evolution

This changes extractor semantics, not the serialized graph shape or endpoint invariants, so `PROJECT_GRAPH_SCHEMA_VERSION` remains unchanged. Existing immutable graph snapshots remain valid historical derived projections and are never rewritten.

`ley ingest` already rebuilds the graph from the approved captured artifact set on every ingestion attempt, even when the artifact snapshot ID is unchanged. Re-ingestion therefore produces/persists a new graph snapshot when the richer relation changes graph identity while preserving older graph history. This is the explicit rebuild path for the changed derived semantics; no hidden migration rewrites old snapshots.

## Evaluation and authority

The real-binary `graph-relative-import-test-impact` scenario compares the new graph path against the simpler direct context-search baseline. A unique marker in `src/renderer.ts` does not make direct context search surface `tests/renderer.test.ts`; graph traversal from the changed implementation must discover that importing test, exclude an unrelated test, and prove a one-edge deterministic `imports` path with a citation to the test's import statement. Local project/vault paths must not leak and every graph result keeps `liveSourceChecked: false`.

This relation is structural captured evidence, not proof that the imported implementation is currently correct, that the test is sufficient, or that live source still matches the capture. Agents must still inspect current source before consequential edits or test selection decisions that require live correctness.
