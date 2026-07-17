# ADR 0006: Fixed-project read-only Model Context Protocol retrieval

- Status: Accepted
- Date: 2026-07-18

## Context

Agents need small, trustworthy project context across sessions without loading a whole repository. The Model Context Protocol (MCP) must not become a hidden capture mechanism or let one connection roam across a user's machine. The durable artifact and graph layers already preserve approved post-redaction evidence with immutable citations. The first protocol surface should expose that reviewed boundary, not rescan source or invent semantic memory.

MCP Roots are client-supplied workspace hints, not an authorization boundary, and host support varies. A project path in every tool call is also unsafe and wasteful. It invites accidental cross-project requests and forces the model to handle machine paths. Ley already has a stronger explicit project identity and private binding model.

## Decision

`ley mcp [project] [--vault <temporary-vault>]` starts a local stdio MCP server using protocol version `2025-11-25`. Startup resolves exactly one initialized project and its persisted or explicit temporary vault binding. The process refuses to start without an existing, internally consistent artifact and graph snapshot. It never discovers projects, consumes MCP Roots, or accepts a project/vault selector in a tool call.

The server exposes five tools:

| Tool | Result boundary |
| --- | --- |
| `ley_project_overview` | Identity, capture mode, artifact/graph snapshots, counts, bounded Git state, freshness, and privacy notice |
| `ley_search_context` | Exact local lexical search over approved artifacts/symbols/dependencies, capped at 20 results and an 8,000-token estimate |
| `ley_read_evidence` | Exact current-manifest artifact, at most 200 lines and 16,000 characters |
| `ley_graph_neighbors` | Incoming/outgoing/both traversal, depth 1–3 and at most 100 nodes, with optional edge filters |
| `ley_graph_path` | Bounded shortest path, depth 1–8 and at most 500 inspected nodes, with optional edge filters |

All tools declare the MCP hints `readOnlyHint: true`, `destructiveHint: false`, `idempotentHint: true`, and `openWorldHint: false`. The only resource is `ley://project/<project-id>/overview`; unknown resources fail instead of mapping arbitrary URIs to files. There are no prompts, resource templates, subscriptions, sampling requests, write tools, network listeners, or logging on stdout.

Every successful result is structured JSON and repeats stable project/snapshot identity. Evidence carries a project-relative range, post-redaction content hash, provenance, confidence, trust state, and `untrusted-project-evidence` boundary. Results say `freshness: captured-snapshot` and `liveSourceChecked: false`; Ley does not imply that an old ingestion reflects the current working tree. Tool failures use MCP tool errors with sanitized messages rather than exposing absolute scope paths.

Read operations open existing capability-scoped store directories without creating them, acquire a shared lock, verify mutable pointers against immutable snapshots, and require the graph to cite the current artifact snapshot. Evidence blobs are opened without following symlinks and rechecked against their size and hash before decoding. Search and traversal are local and deterministic; the server invokes no model, compiler, language server, package manager, or remote service.

## Evidence

- [MCP tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools) define discoverable input schemas and optional behavioral annotations.
- [MCP resources](https://modelcontextprotocol.io/specification/2025-11-25/server/resources) are application-controlled URI-addressed context and require explicit listing and reading.
- The [official MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk) supports stdio servers and the `2025-11-25` protocol.
- The [official MCP Inspector](https://github.com/modelcontextprotocol/inspector) exercises discovery and calls against real server processes.

## Consequences

- A host configuration is the user-visible authorization boundary: one command starts one project-scoped process.
- The model never needs a vault path and cannot switch projects through prompt-controlled tool arguments.
- Context is useful but deliberately incomplete. Structured sessions, decisions, problems, learnings, corrections, temporal reranking, and local embeddings remain later slices.
- Lexical token counts are conservative estimates, not tokenizer-specific guarantees; serialized-byte caps and retrieval evaluation remain follow-up hardening.
- A cloud agent may send intentionally retrieved context to its provider even though Ley's own storage and server remain local.
