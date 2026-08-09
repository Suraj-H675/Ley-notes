# ADR 0007: Append-only structured agent sessions

- Status: Accepted
- Date: 2026-07-18

## Context

Ley needs to preserve useful work across agent sessions without recording every conversation or trusting one host's unstable transcript format. A mutable summary can lose earlier evidence, hide corrections, and fail during concurrent writes. A raw transcript captures too much private and irrelevant material.

Session memory also needs a human-readable view. Treating that view as authoritative would make later schema changes and crash recovery unsafe.

## Decision

Ley stores each session as immutable, versioned JSON events under the bound filesystem vault:

```text
<vault>/.ley/agent-memory/projects/<project-id>/sessions/<session-id>/
├── events/<event-id>.json
├── session-v1.json
└── session.md
```

The `events` directory is authoritative. `session-v1.json` and `session.md` are atomic projections rebuilt from those events. Reading a session verifies and replays the event stream instead of trusting either projection.

Version 1 supports four event types:

- `session-started` records the name, goal, host metadata, and artifact snapshot at session start
- `checkpoint-recorded` records plans, decisions, tasks, problems, attempts, outcomes, resolutions, touched artifacts, commands, verification, and unresolved work
- `session-finished` records a completed, paused, or abandoned outcome, final response, handoff, and unresolved work
- `session-renamed` records an append-only user-authored name revision and reason

[ADR 0025](0025-bounded-session-turn-evidence.md) extends the same ledger with v2 prompt/response observation events while leaving every v1 event and schema readable and unchanged.

Every mutation requires a `req_` request ID. Ley derives the event ID from the session, event type, and request ID. Repeating the same request returns the existing event. Reusing a request ID with different content fails. A project-level advisory lock serializes writers, and event sequence numbers must remain contiguous.

Checkpoint artifact paths must match the current approved artifact manifest. Ley replaces each path with a citation containing its artifact snapshot, post-redaction content hash, and line range. Session records store no absolute project or vault path.

Ley trims and bounds every text field, rejects control characters, and runs the local credential redactor before persistence. It stores redaction metadata with the event. This filtering reduces accidental disclosure but cannot identify every possible secret.

The command-line interface (CLI) supports manual start, checkpoint, finish, list, and show operations. A full checkpoint can use the versioned [checkpoint input schema](../../schemas/agent-memory/checkpoint-input.schema.json). Host adapters and future write-capable Model Context Protocol (MCP) tools must call the same core functions.

## Consequences

- A crash can leave a missing projection without destroying the source events
- Concurrent hooks and manual checkpoints cannot overwrite each other
- Sessions preserve structured evidence without automatic raw transcript capture
- Immutable history makes later correction and supersession explicit
- Derived Markdown is reviewable in a filesystem vault but should not be edited as the source of truth
- A filesystem vault is required for external agent capture; browser-local IndexedDB cannot serve a local agent process
- Session capture does not trust its own claims; the separate [ADR 0009](0009-evidence-backed-learning-ledger.md) review ledger creates project learnings, while automatic host parsing remains a later adapter concern
