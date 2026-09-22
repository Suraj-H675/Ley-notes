# ADR 0078: Resolve captured Python relative imports to project files

## Status

Accepted.

## Context

ADR 0047 proved that an unambiguous captured JavaScript/TypeScript relative import can improve
implementation-to-test retrieval without introducing heuristic symbol resolution, live filesystem
probing, or a new graph schema. Ley already extracts Python `import` / `from ... import ...` syntax as
deterministic `imports` edges, but every Python target still terminates at an `ExternalModule` node.

Python has one similarly strong local signal: an explicit relative `from` target with one or more
leading dots. When that module path maps to exactly one file already present in the approved captured
artifact set, Ley can preserve the same import edge while pointing it at the existing captured `File`
node.

## Decision

Extend the existing captured-relative-import resolver to Python without changing the graph schema or
public graph tools.

- Only Python targets beginning with `.` participate. Absolute imports remain external because their
  resolution depends on package roots / `sys.path` and must not be guessed from repository layout.
- One leading dot means the importing file's containing package directory; each additional leading dot
  moves up one captured package-directory level. Traversal beyond the captured project-relative
  directory hierarchy fails closed.
- The remaining dotted module name is converted to project-relative path components and may match
  either exactly one `<module>.py` file or exactly one `<module>/__init__.py` package file already in
  the approved captured path set.
- If zero or more than one captured candidate matches, Ley keeps the existing deterministic
  `ExternalModule` edge rather than selecting a winner.
- A relative target without a module component (for example `from . import sibling`) is not resolved
  in this slice because the imported name may represent either a submodule or a package attribute.
- Slash/backslash/query/hash-bearing targets are rejected by the local resolver.
- Resolution is purely lexical over captured paths. Ley does not read the live filesystem, import or
  execute Python, inspect environments, run a language server, or infer package search paths.

The edge remains `GraphEdgeKind::Imports`, `FactProvenance::Deterministic`, confidence `1.0`, and cites
the captured Python import statement. Existing immutable graph snapshots remain valid historical
derived projections; re-ingestion rebuilds current graph semantics without rewriting prior snapshots.

## Evaluation

Focused core coverage requires `from ..renderer import render_frame` inside
`app/tests/test_renderer.py` to resolve to the captured `app/renderer.py` file. A module/package
ambiguity (`app/ambiguous.py` plus `app/ambiguous/__init__.py`) must remain external, an absolute
`from app.renderer ...` import must remain external, and a root-level relative import must not be
promoted.

The real-binary `graph-python-relative-import-test-impact` scenario mirrors the existing JS/TS
code-to-test baseline: direct context search for an implementation-only marker must miss the Python
test, while one incoming deterministic `imports` edge and the exact reverse path recover the relevant
test, exclude an unrelated test, keep `liveSourceChecked: false`, and leak no project/vault path.

## Authority and limits

This relationship is captured structural evidence only. It does not prove that the Python package is
currently importable, that runtime `sys.path` matches the project layout, that the test is sufficient,
or that live source still matches the capture. Consequential work must still inspect current source.

Cross-file symbol-name resolution, absolute Python import resolution, namespace/package-root discovery,
and `from . import sibling` disambiguation remain out of scope until deterministic evidence and
evaluation justify them.
