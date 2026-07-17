# ADR 0008: Explicit consent for MCP session writes

- Status: Accepted
- Date: 2026-07-18

## Context

Ley agents need to start, checkpoint, and finish durable sessions. Adding write tools to every existing Model Context Protocol (MCP) connection would silently expand the authority of configurations that were created for read-only retrieval. Stored prompt injection could then encourage an agent to poison memory even when the user intended only to search it.

The session engine already provides the required write guarantees: fixed project scope, immutable events, caller-stable request IDs, credential redaction, current-snapshot artifact citations, collection limits, and a cross-process lock. The protocol layer should reuse those guarantees without making capture ambient.

## Decision

`ley mcp [project]` remains read-only by default. It exposes project retrieval, graph traversal, session listing, and bounded session context.

Session writes require an explicit startup flag:

```bash
ley mcp /path/to/project --allow-session-writes
```

The flag enables three additional tools:

| Tool | Append-only effect |
| --- | --- |
| `ley_session_start` | Creates one structured session start event |
| `ley_session_checkpoint` | Appends one checkpoint with structured work and cited artifacts |
| `ley_session_finish` | Appends one completed, paused, or abandoned result |

Each write requires a caller-stable `req_` ID. Exact retries return the original compact receipt. Reusing the ID with different content fails. Receipts contain stable project, session, and event IDs plus status, counts, timestamp, and replay state. They do not return the growing session body or absolute local paths.

Write tools declare `readOnlyHint: false`, `destructiveHint: false`, `idempotentHint: true`, and `openWorldHint: false`. They append immutable local history and cannot delete or rewrite prior events. The seven default tools retain read-only annotations.

The process remains fixed to the project and private binding resolved at startup. Tool arguments contain no project or vault selector. MCP-created sessions record `source.kind: mcp`; optional host and agent labels remain untrusted metadata.

The startup flag is the Ley consent boundary. The host remains responsible for showing and enforcing its tool-call permission policy. Stored project or session content never grants write permission, and Ley does not infer a write request from retrieved text.

## Consequences

- Existing MCP configurations keep their read-only authority
- Users can enable automatic structured capture per project and host configuration
- A malicious retrieved instruction cannot enable tools that were absent at process startup
- Write calls still depend on host permission behavior after the startup opt-in
- MCP and CLI capture produce the same event schema, redaction, citations, and idempotency behavior
- Version 1 captures structured events only; it does not automatically read complete host transcripts
