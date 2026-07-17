# ADR 0015: Version-guarded learning corrections

- Status: Accepted
- Date: 2026-07-18

## Context

Ley’s learning ledger already preserves corrections as immutable events, but the desktop provenance inspector only exposed confirmation and feedback. A user who found an inaccurate title, claim, or confidence value could mark it contested yet could not replace it from the primary interface.

Trust actions also need an inspected-version boundary. Without one, a user can open a lesson, an agent can append a correction, and the user can then unknowingly confirm text they never reviewed. Cross-process serialization prevents corrupt writes but does not prevent this semantic lost-decision race.

## Decision

The desktop provenance inspector exposes an explicit **Correct** workflow for non-terminal learnings. It allows the user to change the title, guidance, and confidence and requires a correction reason. The native adapter hardcodes `actor: user`, reads the complete authoritative learning rather than the bounded UI projection, preserves every existing session evidence reference, and appends a normal `Corrected` event through `ley-core`.

If the bounded inspector had to truncate the learning’s title or guidance, it exposes no correction or review controls. The user must inspect the complete CLI projection first; Ley does not allow a partial view to overwrite or trust omitted text.

A correction never edits or deletes prior events. Replay makes the new version tentative and review-required, resets its temporal `validFrom`, and keeps the correction reason in history. The user must inspect and confirm that version separately before agents may reuse it.

Desktop correction and review requests include the exact event count shown by the inspector. The shared ledger validates this expected count while holding its project writer lock. If another process appended an event first, the stale request fails before mutation and tells the client to reload. Exact idempotent retries of an event already accepted still replay successfully.

Rejected and superseded learnings remain terminal history. Their inspector does not offer controls that the engine would refuse.

## Consequences

- Users can correct inaccurate memory without erasing provenance.
- A trust click cannot silently approve a concurrent correction the user did not see.
- The UI preserves the complete evidence set even when its context view is bounded.
- Corrected claims require a second, explicit confirmation before reuse.
- Evidence editing, reviewed deletion, supersession selection, and promotion to ordinary notes remain separate workflows.
