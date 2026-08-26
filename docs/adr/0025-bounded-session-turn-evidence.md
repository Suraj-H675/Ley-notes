# ADR 0025: Bounded session turn evidence

- Status: Accepted
- Date: 2026-08-09

## Context

Ley's structured checkpoints preserve durable meaning, but lifecycle adapters previously discarded user prompts and represented final assistant messages as fabricated generic checkpoints. That loses the request/response evidence needed to understand a later session and conflates observed text with agent-authored structure. Reading complete host transcripts would capture excessive private data and bind Ley to unstable host formats.

## Decision

Ley adds immutable schema-v2 `user-prompt-observed` and `assistant-response-observed` events. Existing v1 events and schemas remain unchanged and readable. A v1-only session remains v1 on read; its first deliberate turn event creates `session-v2.json` while retaining `session-v1.json`. Immutable events remain authoritative.

Structured and Full Evidence modes retain pattern-redacted text bounded to 4,000 prompt characters and 8,000 response characters, with a 1 MiB aggregate retained-turn budget per session. Minimal mode appends body-free disclosure events. Capacity exhaustion also appends a body-free disclosure. Ley stores no raw host turn ID, transcript path, original length, or body-derived fingerprint for omitted content.

Codex pairs prompt and response events with its documented stable `turn_id`. Claude Code does not expose one stable identifier shared by its current pre/post events, so Ley derives an ordinal under the serialized append-only session state. Exact pending-turn retries reuse an ordinal; after a paired response, an identical prompt starts a new turn.

Normal startup, resume, activity search, cross-project search, learning evidence, and note export exclude turn bodies. They may expose counts. Bodies require the explicit bounded `ley_session_turns_get`, `ley session turns`, or desktop **Captured turns** action and are labeled `untrusted-user-prompt` or `untrusted-agent-output`.

Hooks never read `transcript_path`. Full Evidence remains permission for a future separately invoked evidence adapter, not authorization for automatic transcript capture.

## Consequences

- Later sessions can inspect what was asked and answered without treating that text as a checkpoint or startup instruction.
- Minimal/capacity retries cannot distinguish a changed omitted body without storing forbidden body-derived metadata; Ley preserves the original omission result and still validates request identity, origin, host, and turn reference.
- Pattern redaction reduces accidental secret capture but is not a guarantee; users who do not want turn bodies retained must select Minimal.
- Host correlation behavior is versioned and requires real multi-turn compatibility exercises when host payload contracts change.

## Primary sources

- [Codex lifecycle hooks](https://learn.chatgpt.com/docs/hooks)
- [Claude Code hooks](https://code.claude.com/docs/en/hooks)
