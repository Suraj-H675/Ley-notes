# ADR 0022: Immutable project revision citations on session checkpoints

- Status: Accepted
- Date: 2026-07-18

## Context

A session checkpoint can accurately describe work only relative to the project evidence the agent actually received. Reading the live Git repository while recording a checkpoint would create a race: the working tree or HEAD may have moved after Ley's last approved ingestion, and a later retry could silently acquire a different revision. A Git commit alone is also insufficient for non-Git projects and cannot identify the exact redacted artifact and graph projections used for retrieval.

## Decision

Ley derives one optional `projectRevision` for every new checkpoint from the already loaded, internally verified project memory pair. It records the immutable graph snapshot ID, its matching artifact snapshot ID, capture timestamp, captured Git HEAD and branch when present, and the count of captured tracked changes. The checkpoint input cannot supply or override these fields.

The revision describes the most recent approved Ley capture, not live source. Checkpoint recording never invokes Git. A project without Git still receives the graph and artifact snapshot citation with no invented commit or branch.

Revision metadata is excluded from the stable request fingerprint, as are the other derived artifact details. Repeating the same request ID and checkpoint input therefore replays the original event and original revision after a later ingestion; changed caller content still conflicts.

Stored revisions are validated as part of immutable event replay. The desktop inspector exposes an internal link to the exact retained graph capture, and Markdown projections and user-authorized session-note exports state the same provenance. Ley does not construct a remote repository URL or disclose one.

## Consequences

- Agents and users can distinguish “what Ley knew at this checkpoint” from today's filesystem state.
- Git commits, dirty tracked state, graph relations, and redacted artifacts share one capture boundary.
- Non-Git projects retain useful provenance without pretending that a commit exists.
- Old schema-version-1 events remain readable because the field is optional; all newly recorded checkpoints include it.
- A retained revision is an evidence pointer, not proof that the source is current, pushed, reviewed, or trustworthy.
