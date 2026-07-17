# ADR 0005: Deterministic cited project graph

- Status: Accepted
- Date: 2026-07-18

## Context

The artifact store preserves approved, redacted project evidence, but agents need a compact structural view before retrieval can answer questions such as “where is this function defined?”, “what calls it?”, or “which package supplies this import?” Ley must extract that structure without sending source code to a model, pretending syntactic name matching proves semantic resolution, or retaining source text in Minimal capture mode.

The graph must remain subordinate to evidence. Every source-derived fact needs a stable project-relative citation and source-snapshot identity. Malformed code, unusual manifests, Git repositories, concurrent ingestion, and interrupted writes must not corrupt the current projection or expand the approved capture boundary.

## Decision

Every `ley ingest` run now derives a versioned project graph from the post-redaction text already held inside the ingestion boundary. The current graph and immutable history live at:

```text
<vault>/.ley/agent-memory/projects/<project-id>/graph/
├── graph-v1.json
└── snapshots/<graph-snapshot-id>.json
```

No absolute project or vault path is stored. `ley graph [project] [--vault <temporary-vault>] [--json]` resolves the same explicit binding, verifies the current graph against its immutable snapshot, and returns it without rescanning the repository.

The v1 graph contains project, file, symbol, dependency, external-symbol, and external-module nodes. It contains `contains`, `defines`, `imports`, `calls`, `inherits`, `implements`, `references`, and `depends-on` edges. Direct syntax and manifest facts are `deterministic` with confidence `1.0`; the project name is `user-authored`. The schema reserves `agent-authored` and `inferred`, but v1 does not emit inferred cross-file symbol resolution. A call to `recall` therefore targets a stable external-symbol node named `recall` rather than falsely claiming which same-named definition executes.

Rust, JavaScript, TypeScript/TSX, and Python definitions, calls, and class/type references use the language grammars' official Tree-sitter tag queries. Ley adds syntax-tree-bounded extraction for imports and explicit inheritance/implementation relationships. Syntax-error files produce bounded diagnostics while valid regions remain indexed. Symbol identity uses project ID, relative path, symbol kind, name, and same-name occurrence ordinal, so unrelated line movement does not churn node identity.

`Cargo.toml`, `package.json`, `pyproject.toml`, and `requirements.txt` produce direct dependency facts with manager, declared requirement, and manifest citation. Unsupported or malformed manifests remain artifacts and produce diagnostics rather than guessed dependency facts.

Local Git state is captured with `git status --porcelain=v2 --branch -z --untracked-files=no`. Optional locks, filesystem monitoring, and the untracked cache are disabled for the command. Output is capped at 8 MiB. Only changes whose current or original path is in the already-approved artifact set survive into the graph; untracked, ignored, excluded, and outside-root paths are not introduced through Git metadata.

Every citation records the project-relative artifact path, one-based source range, post-redaction content hash, and artifact snapshot ID. The graph snapshot ID is a SHA-256-derived identity over the complete graph, diagnostics, Git state, and source snapshot, excluding only generation time. Identical source and Git state is a true graph no-op. A Git-only change can create a new graph snapshot while the artifact snapshot remains unchanged.

Graph writes share the artifact ingestion lock. Ley writes the immutable artifact snapshot before a graph that cites it, then writes the immutable graph snapshot and atomically replaces the current graph. The current artifact manifest is replaced last. On restart, each current file is verified against its immutable snapshot; a graph temporarily ahead of the current artifact pointer after an interrupted write is recoverable because its cited artifact snapshot is already immutable.

The graph format is capped at 64 MiB, rejects unknown fields, checks project/snapshot identities, node and edge uniqueness, edge endpoints, paths, citations, confidences, diagnostics, Git metadata, and immutable-snapshot equality. Files use mode `600` and directories mode `700` on Unix through the same capability-scoped store as artifacts.

## Evidence

- Tree-sitter defines the `@definition.*`, `@reference.*`, and `@name` tagging convention for code navigation: <https://tree-sitter.github.io/tree-sitter/4-code-navigation.html>.
- Tree-sitter queries identify syntax structures without requiring a compiler or model: <https://tree-sitter.github.io/tree-sitter/using-parsers/queries/index.html>.
- Git porcelain v2 is the stable machine-readable status format and `-z` preserves path boundaries without quoting ambiguity: <https://git-scm.com/docs/git-status#_porcelain_format_version_2>.
- `Cargo.toml` is Cargo's authoritative package manifest: <https://doc.rust-lang.org/cargo/reference/manifest.html>.
- npm documents dependency declarations in `package.json`: <https://docs.npmjs.com/cli/configuring-npm/package-json#dependencies>.
- Python's `pyproject.toml` project dependency field uses PEP 508 strings: <https://packaging.python.org/en/latest/specifications/pyproject-toml/#dependencies-optional-dependencies>.

## Consequences

- Read-only MCP retrieval can traverse a small verified projection and return exact citations before opening bounded source evidence.
- Minimal capture can retain useful structure without retaining source bodies; citations still identify the current project file and hash, but old source text is intentionally unavailable in that mode.
- Syntax facts are not compiler-grade semantic facts. Cross-file resolution, dynamic dispatch, macro expansion, generated code, and runtime imports require later resolvers and must be marked `inferred` unless independently proven.
- Git state intentionally omits untracked files and excluded paths. Artifact ingestion remains the privacy authority.
- Tree-sitter grammars increase native build size and compile time, but keep extraction local, deterministic, cross-platform, and independent of language toolchains.
