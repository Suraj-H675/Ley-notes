# ADR 0016: Trusted learning promotion into Markdown

- Status: Accepted
- Date: 2026-07-18

## Context

Agent Memory intentionally keeps agent-authored claims separate from ordinary notes until a person reviews them. Ley could confirm and correct a lesson, but it could not complete the second-brain loop by turning a reviewed lesson into user-owned Markdown.

Promotion must not silently overwrite a note, duplicate one lesson after a rename, copy a clipped claim, or make the append-only learning ledger depend on later note edits.

## Decision

Ley Desktop offers **Promote to note** only for a learning whose inspected version is current, verified, trusted, and fully visible. Promotion copies that exact guidance into an ordinary Markdown note under `Agent Memory/Lessons`. The user may choose the note title before creation.

The note receives portable YAML provenance: source kind, project and learning IDs, learning kind/state, trust and freshness at promotion, confidence, version validity, promotion time, and the `ley/lesson` tag. Its body includes the reviewed guidance, a human-readable provenance callout, and bounded artifact/session/record identifiers. Evidence notes are not copied automatically because they remain untrusted supporting material.

The authoritative learning ledger is not modified. The promoted note becomes normal user-owned Markdown and can be edited, moved, linked, or deleted independently. Ley does not silently synchronize later learning corrections into that note.

Promotion is idempotent by `ley-learning-id`. If the user already promoted the lesson and later renamed or moved its note, Ley opens that existing note. If an unrelated note already owns the requested title, creation fails and asks for another title instead of overwriting or silently reusing it.

## Consequences

- Reviewed procedural memory can join the normal notes, backlinks, search, tags, graph, and revision workflows.
- The Agent Memory/human-note trust boundary remains explicit.
- Repeated promotion does not create duplicate notes.
- A promoted note is a historical, attributed snapshot rather than a live mirror.
- Updating a promoted note from a later learning version remains an explicit future workflow.
