# ADR 0017: Append-only, version-guarded session naming

- Status: Accepted
- Date: 2026-07-18

## Context

Agents can suggest a useful session name at capture time, but the user needs final naming authority. A mutable title field would make the derived session disagree with its event history, erase provenance, and allow a stale desktop view to overwrite a newer agent checkpoint.

Session IDs already anchor citations, learnings, resume context, and cross-project search. Renaming must not break those links or imply that the original event was rewritten.

## Decision

A user rename appends a `session-renamed` event to the existing session ledger. Replay retains `originalName`, applies each rename in sequence to derive the current `name`, and exposes a timestamped naming history with the required reason. The stable `ses_` ID and every existing citation remain unchanged. Renaming is allowed after a session is completed, paused, or abandoned because it changes review metadata rather than captured work.

The desktop inspector and manual CLI are the only rename authorities. MCP does not expose a rename tool, including when session writes are enabled. The native adapter hardcodes the local user path, requires the event count shown by the inspector, and validates that count while the session writer lock is held. A concurrent event makes the rename fail stale before mutation; an exact idempotent retry still succeeds.

Names and reasons use the existing credential-pattern redaction and field limits. Agent context returns the current and original names plus up to the ten newest rename entries after budgeting higher-priority goal, outcome, and checkpoint evidence, with total and omitted counts. The immutable event log and derived local review files retain the complete history.

## Consequences

- Users can improve agent-suggested names without erasing history.
- Renames cannot break session citations, learnings, or stable links.
- A stale inspector cannot silently rename a version the user did not review.
- Long naming histories remain bounded and explicitly incomplete in agent context.
- Agent automation cannot rename sessions without a future, separately approved authority design.
