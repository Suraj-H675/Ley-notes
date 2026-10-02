# ADR 0103: Retire the legacy session checkpoint alias

**Status:** Accepted — 2026-10-02

## Context

Ley has one canonical model-facing structured checkpoint write: `ley_checkpoint`. The older
`ley_session_checkpoint` route had the same request schema, idempotency/event-count semantics, cited-artifact
behavior, optimistic `expectedEventCount` guard, and underlying transition-aware writer.

The other legacy session lifecycle routes are not equivalent duplicates:

- `ley_session_start` remains useful to a hostless legacy MCP client that has no Codex/Claude lifecycle hook to
  establish the first session ID. Supported packaged hosts do not need it because every relevant hook calls
  `ensure_host_session`.
- `ley_session_finish` remains the compatibility model-facing terminalization route. Host Stop hooks capture the
  assistant response but intentionally do not mark the session completed/paused/abandoned; local CLI/Desktop can
  also finish sessions.

Removing all three routes together would therefore conflate a pure alias with two distinct compatibility jobs.

## Decision

Retire only `ley_session_checkpoint` from MCP:

- canonical `ley_checkpoint` directly owns the checkpoint implementation;
- remove the duplicate route and schema/inventory expectations;
- migrate active evaluator use to canonical `ley_checkpoint`; and
- retain `ley_session_start` and `ley_session_finish` behind explicit legacy session-write capability.

Supported host packages continue to teach only the four-tool canonical surface and must not teach any granular
legacy lifecycle route.

## Consequences

- Legacy clients must rename checkpoint calls to `ley_checkpoint`; no session event or data migration is required.
- Hostless compatibility clients can still create a session with `ley_session_start` before calling
  `ley_checkpoint`.
- Compatibility clients can still terminalize a session with `ley_session_finish`.
- CLI/hook/Desktop lifecycle behavior and all core session writers remain unchanged.

## Non-goals

This ADR does not remove session creation, terminalization, local CLI lifecycle commands, host lifecycle hooks,
session recovery readers, Memory Compiler, or canonical `ley_checkpoint`.
