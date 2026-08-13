# ADR 0026: Explicit local hybrid retrieval

Status: accepted

## Context

Ley can already retrieve captured artifacts, symbols, dependencies, sessions, decisions, problems, and learnings with bounded lexical matching. Exact words are not enough for durable agent memory: a later session may describe the same intent with different language. Cloud embeddings would violate Ley's local-first boundary, while an implicit model download would create surprising network traffic, disk use, and supply-chain risk.

Semantic similarity is also not evidence. A useful memory result still needs an immutable citation where one exists, an honest freshness label, a trust state, and disclosure when durable records disagree.

## Decision

Ley supports one retrieval model initially: `minishlab/potion-retrieval-32M`, an MIT-licensed Model2Vec model specialized for retrieval. The supported artifact is pinned to Hugging Face revision `6fc8051fab2a1e0ee76689cf08c853792ac285e7`; Ley never resolves a moving `main` reference at runtime. The model is loaded by the official Rust `model2vec-rs` implementation with remote loading disabled.

The model lifecycle is explicit:

1. status inspection never contacts the network;
2. installation occurs only after a direct user command or equivalent first-party UI action;
3. the installer downloads only the pinned `config.json`, `tokenizer.json`, and `model.safetensors` files into a private staging directory;
4. every file is checked against its pinned size and SHA-256 before the staged directory is atomically promoted into Ley's OS-specific cache directory; and
5. normal ingestion, search, MCP startup, and desktop startup never download a model.

The desktop **Agent Memory → Search memory** surface exposes the same status and installer as the CLI. Before consent it shows the pinned model identity, download size, origin, verification boundary, local-inference guarantee, and lexical fallback. Installation runs outside the UI thread; interrupted or failed downloads leave lexical search usable, and a corrupt cache can be repaired only through the same explicit action.

A missing or invalid model is an available lexical-only mode, not a startup failure. Responses report which retrieval mode actually ran. Ley does not upload source text, queries, vectors, or model telemetry.

Semantic indexes are derived, disposable data. Their manifest binds the index to the Ley project ID, artifact snapshot ID, graph snapshot ID, retrieval model ID and revision, schema version, and embedding dimension. Entries retain the original bounded citation and record identity; vectors do not become evidence. A mismatch invalidates the derived index and causes an explicit rebuild or a lexical-only response. Index writes use private permissions and atomic replacement within the bound vault's project memory namespace.

Hybrid ranking is deterministic. Ley computes bounded lexical and cosine-similarity rankings independently, combines ranks with reciprocal-rank fusion, then applies bounded freshness and trust signals only where those concepts are meaningful. Exact evidence matches remain discoverable even when semantic retrieval is enabled. Stable identifiers break score ties.

Conflict disclosure is a separate output, not a ranking penalty hidden from the caller. Ley reports matching durable records that are contested, rejected, superseded, stale, or materially disagree with another matching current record. It does not ask the embedding score to decide which claim is true.

Every agent-facing hybrid result preserves:

- project and record identity;
- immutable artifact or graph citations where available;
- trust and freshness state;
- the source boundary and prompt-injection warning;
- whether lexical, semantic, or hybrid retrieval actually ran; and
- bounded conflict disclosures.

The fixed-project MCP boundary remains unchanged. Hybrid search cannot enumerate unbound projects, refresh capture, install a model, or read live source outside the current approved snapshot.

## Consequences

- Later agent sessions can recover conceptually related memory without sending private text to a service.
- Installing the initial retrieval model requires an explicit roughly 131 MB download and corresponding local storage.
- First semantic indexing costs local CPU time; repeated search can reuse a snapshot-bound derived index.
- Lexical retrieval remains fully usable on unsupported machines and before model installation.
- Model upgrades require a new reviewed manifest and index version instead of silently changing search behavior.
- Conflict, trust, and freshness semantics remain inspectable and independent from similarity scores.
