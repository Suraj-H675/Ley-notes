# ADR 0079: Capture source-bearing JavaScript/TypeScript re-export relations

Status: Accepted

## Context

ADR 0047 resolves unambiguous captured JavaScript/TypeScript relative imports to captured project
files, and ADR 0078 extends the same captured-path-only rule to explicit Python relative imports.
That improves direct and transitive structural-impact retrieval, but a common JavaScript/TypeScript
module shape remains invisible: barrel modules that re-export another module with statements such as
`export { value } from './value'` or `export * from './value'`.

Ignoring that statement breaks an otherwise deterministic implementation -> barrel -> consumer ->
test chain. Treating every `export` statement as a dependency would be unsafe, though: ordinary
exports can contain string literals that are values rather than module specifiers.

## Decision

Keep the existing graph schema and `GraphEdgeKind::Imports`. For captured JavaScript/TypeScript
syntax, both `import_statement` and `export_statement` may produce a module relation, but only when
Tree-sitter exposes the statement's explicit `source` field.

- `export { value } from './value'`, `export * from './value'`, and namespace re-exports with an
  explicit source are eligible.
- `export const label = './value'`, `export default './value'`, local `export { value }`, and other
  exports without a module `source` produce no module edge.
- Relative re-export sources use the exact same captured-path-only resolver as relative imports.
  Exactly one approved captured candidate is required; zero/multiple candidates remain external.
- Bare/package re-export sources remain `ExternalModule` nodes.
- Resolution never probes the live filesystem, runs a package manager/compiler/language server, or
  infers package aliases.
- The relation remains deterministic confidence `1.0` and cites the captured import/re-export
  statement that produced it.

`Imports` continues to mean a captured module-dependency relation for graph traversal; this slice
does not claim that a re-export is semantically identical to a runtime import, that the exported
symbol exists now, or that live source still matches the captured snapshot.

## Evaluation

`graph-barrel-reexport-ripple` uses the existing real-binary ripple contract. Direct context search
for an implementation-only marker must not discover the downstream test. The captured graph must
recover the exact test -> consumer -> barrel -> implementation path through three deterministic
`imports` edges while excluding an unrelated test and leaking no project/vault machine path.

Core regressions additionally prove that an ordinary exported string literal does not become a
false dependency, a bare package re-export stays external, and an ambiguous relative re-export is
not promoted to a captured file.

## Derived-state evolution

This changes extraction semantics without changing the serialized graph shape, so the project graph
schema version remains unchanged. Re-ingestion rebuilds the disposable graph projection from the
approved captured artifact set and can create a new immutable graph snapshot; historical graph
snapshots are not rewritten.
