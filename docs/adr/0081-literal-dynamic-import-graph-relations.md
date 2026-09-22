# ADR 0081: Capture literal JavaScript/TypeScript dynamic import relations

Status: Accepted

## Context

Ley already captures static JavaScript/TypeScript imports, source-bearing re-exports, and explicit
Python relative imports as deterministic `imports` graph relations. Modern JavaScript/TypeScript
also uses the language-level dynamic `import()` syntax for lazy loading and code splitting. A quoted
literal such as `import('./renderer')` names a concrete module source, but Ley previously ignored the
relationship entirely.

Generic call-like module loading must not be inferred casually. In particular, CommonJS `require`
is an ordinary identifier and can be shadowed, while computed/template dynamic imports may depend on
runtime values. Those cases do not satisfy this slice's deterministic evidence standard.

## Decision

Treat a JavaScript/TypeScript `call_expression` as a module relation only when all of these conditions
hold:

- Tree-sitter exposes its function node as the dedicated `import` syntax;
- the argument container is the normal argument list;
- it contains one or two named arguments;
- the first argument is a single- or double-quoted string literal.

The optional second argument is import options/attributes metadata and does not change the module
specifier identity. Ley does not interpret or persist those option fields as graph semantics.

The extracted literal then follows the existing JS/TS module-target rules:

- an unambiguous `./` or `../` target may resolve against the approved captured path set;
- bare/package targets remain `ExternalModule` nodes;
- ambiguous, escaping, query/hash/backslash-bearing targets remain unresolved/external;
- the edge remains `GraphEdgeKind::Imports`, deterministic confidence `1.0`, with a citation to the
  captured dynamic-import expression.

Computed identifier imports, template-string imports, calls with more than two named arguments,
malformed forms, and generic `require(...)` calls produce no module relation in this slice.

## Evaluation

`graph-dynamic-import-test-impact` compares direct context search with the captured graph. A unique
implementation marker must not make lexical search discover the dependent test, while graph
neighbors/path must recover the test through a literal `import('../src/renderer')` relation and
exclude an unrelated test.

`graph-dynamic-import-options-impact` repeats the contract for
`import('../data/config.json', { with: { type: 'json' } })`: the second argument may affect import
attributes but must not hide the exact first-argument module relation.

Core regressions additionally require computed and template-string dynamic imports to remain absent
from module relations, a literal package dynamic import to stay external rather than being promoted
to a captured project file, and an options-bearing literal import to resolve its exact captured
target.

## Derived-state evolution

This changes graph extraction semantics without changing the serialized graph schema. Re-ingestion
rebuilds the derived graph from the approved captured artifact set and may create a new immutable
graph snapshot; historical snapshots remain unchanged.
