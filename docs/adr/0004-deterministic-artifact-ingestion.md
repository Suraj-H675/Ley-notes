# ADR 0004: Deterministic artifact ingestion

- Status: Accepted
- Date: 2026-07-17

## Context

Capture preview establishes which project files Ley may inspect, but it does not create durable memory. Agent retrieval needs stable, cited evidence that survives source renames and deletions without turning an opaque database into the source of truth. Ingestion also crosses a hostile boundary: repository paths and contents can contain symlink attacks, binary data, malformed text, prompt injection, and credentials.

The first durable layer must work offline and without an LLM. AST relations, lexical indexes, and MCP context packs should build on it rather than each reading the repository independently.

## Decision

Add `ley ingest [project] [--vault <temporary-vault>]`. The command resolves the explicit private binding, reuses the approved deterministic preview, reads eligible files through a capability rooted at the canonical project directory, redacts likely credentials, and writes a versioned artifact store to:

```text
<vault>/.ley/agent-memory/projects/<project-id>/artifacts/
├── manifest-v1.json
├── ingest-v1.lock
├── content/<sha256>.txt
└── snapshots/<snapshot-id>.json
```

All citations use normalized project-relative paths. The store contains no project root. A vault at or below the project root is refused to prevent recursive self-ingestion; a project may live below a larger vault because the agent-memory store remains outside that project subtree.

Every current manifest records the project identity, complete capture policy, a fingerprint over that policy plus `.leyignore`, deterministic artifact classification, language where known, source/stored byte counts, line count, SHA-256 of the post-redaction text, optional content-blob location, redaction kinds/lines, and binary, non-UTF-8, oversized, total-limit, and symlink skips. Unknown fields and malformed IDs, hashes, paths, or blob references fail closed. Generated manifests and snapshots have a separate 64 MiB safety limit rather than inheriting the 1 MiB limit for small repository-controlled policy files.

Structured and Full Evidence capture store post-redaction UTF-8 text as immutable content-addressed blobs. Minimal capture stores the same deterministic metadata and hashes without source blobs. Binary and non-UTF-8 files are reported but not persisted as content. Raw secret values are never intentionally retained in hashes, blobs, or manifests: when a detector matches, the hash is calculated after replacement.

The initial detectors cover private-key blocks, credential-bearing assignments, credentials embedded in URLs, and recognizable provider-token prefixes. This is defense in depth, not a claim that every secret can be detected. Ignore rules remain the first boundary, and future detectors must be evaluated for both leakage and false positives.

A snapshot ID is SHA-256 over canonical serialized project identity, capture policy/fingerprint, current artifact records, and skips. Ley recomputes this identity and verifies the referenced immutable snapshot and every current content blob before updating. An identical run is a true no-op: it does not rewrite the current manifest or create another snapshot. A project-name, consent/policy, ignore-rule, content, or skip change creates an immutable snapshot and atomically replaces the current manifest while holding a project-specific cross-process lock. Unique deleted/added path pairs with the same content hash are reported as renames; ambiguous duplicate-content moves remain explicit additions/deletions.

Repository reads use `cap-std` sandboxed directory capabilities and refuse final-component symlinks. This preserves the approved root even if an untrusted project swaps path components during ingestion. Vault storage directories are also opened as no-follow capabilities. New evidence directories use mode `700` and files use mode `600` on Unix. Immutable blobs/snapshots are verified if already present; mutable manifest replacement is capability-scoped and atomic.

## Evidence

- `cap-std` guarantees relative path resolution cannot escape the directory capability and supports Linux, macOS, FreeBSD, and Windows: <https://docs.rs/cap-std/4.0.2/cap_std/>.
- `cap-fs-ext` exposes no-follow directory and final-component open operations: <https://docs.rs/cap-fs-ext/4.0.2/cap_fs_ext/>.
- `cap-tempfile` supports capability-scoped atomic replacement: <https://docs.rs/cap-tempfile/4.0.2/cap_tempfile/struct.TempFile.html>.
- RustCrypto provides a portable SHA-256 implementation: <https://docs.rs/sha2/0.10.9/sha2/>.
- OWASP recommends repository secret detection because source and history commonly retain exposed credentials: <https://owasp.org/www-project-devsecops-guideline/latest/01a-Secrets-Management>.
- GitHub documents that secret detection combines patterns and, for some types, validation or paired evidence; simple regex coverage is necessarily incomplete: <https://docs.github.com/en/code-security/reference/secret-security/secret-scanning-scope>.
- The checked-in Graphify reference demonstrates deterministic source provenance and incremental graph updates, but its generated graph is prior art rather than Ley's durable user-owned format.

## Consequences

- Subsequent AST extraction, lexical search, graph projection, and MCP citations can consume one immutable source layer.
- Deleted/renamed source remains inspectable through old snapshot and content files.
- Ingestion duplicates allowed source text into the selected vault in Structured/Full Evidence modes. The explicit capture mode and bind/ingest commands are therefore privacy-relevant consent boundaries.
- Secret redaction can produce false positives and false negatives. Ley reports detector kinds and line numbers without storing matched values, and must retain ignored-file preview plus later secret-evaluation fixtures.
- Immutable evidence is not garbage-collected yet. A future reviewed retention/delete command must never silently remove user evidence.
- This ADR does not claim AST symbols, Git relations, semantic facts, or MCP retrieval; those are subsequent projections.
