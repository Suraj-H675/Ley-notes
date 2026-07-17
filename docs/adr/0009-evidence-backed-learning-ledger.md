# ADR 0009: Evidence-backed append-only learning ledger

- Status: Accepted
- Date: 2026-07-18

## Context

An agent can extract a useful project lesson from a successful session, but one generated statement is not automatically true. Mutable “memory” documents hide who made a claim, erase corrections, and make poisoning or stale advice difficult to detect. Source files can also change after a lesson was learned.

Ley needs project-level procedural and semantic memory that remains inspectable, correctable, and safe to retrieve in later sessions.

## Decision

Ley stores immutable version 1 learning events in the bound filesystem vault:

```text
<vault>/.ley/agent-memory/projects/<project-id>/learnings/
├── events/<event-id>.json
├── learnings-v1.json
└── review.md
```

The event files are authoritative. `learnings-v1.json` and `review.md` are atomic projections rebuilt by validating and replaying the complete ledger.

A proposal records:

- an explicit user or agent actor;
- `user-authored`, `agent-authored`, or `inferred` provenance consistent with that actor;
- a procedure, constraint, pitfall, convention, or fact;
- bounded title, guidance, and confidence;
- one to twenty references to existing structured-session records.

Ley resolves every reference before writing. If the cited session checkpoint touched project artifacts, the learning pins those artifact snapshot IDs, post-redaction hashes, and line ranges. A missing session or record is rejected.

All proposals start as `tentative` and `review-required`, including user-authored proposals. An explicit user confirmation is the only version 1 path to `verified` and `trusted`. Agents may contest or mark a lesson stale, but cannot confirm, reject, or supersede it. Corrections append a new event and return the lesson to review instead of overwriting trusted history. Rejection and supersession are terminal. Supersession must name an existing learning, and replay rejects missing replacements or cycles.

Artifact-backed lessons are `current` while every cited path and hash still matches the latest approved ingestion. They become `source-changed` after a modification or deletion and re-enter the review inbox even if previously trusted. Session evidence without an artifact citation is visibly `uncited`; user confirmation may still trust it, but it never gains a source-freshness claim.

Every mutation requires a stable `req_` request ID. Event and learning IDs are deterministic. Exact retries replay the original event even if a session or current source snapshot changed later; changed reuse fails. A private project-level lock serializes concurrent writers. User interfaces may additionally supply the event count they inspected; a mismatch under the lock rejects a stale correction or trust decision before mutation. Event files are created once, projections are replaced atomically, and all reads validate identity, fingerprints, sequences, text, citations, authority, replacement integrity, and file permissions.

Recognized credential patterns are redacted before event creation, including titles, guidance, evidence notes, correction notes, and review notes. The ledger stores redaction metadata. Redaction remains a safety layer rather than a guarantee.

The CLI requires actor and provenance to be explicit rather than inferring who invoked it. Its review inbox excludes terminally rejected or superseded records. [ADR 0010](0010-explicit-mcp-learning-proposal-consent.md) separately exposes trusted-first reads and an opt-in agent-only proposal capability without adding ambient review authority.

## Consequences

- Future agents can distinguish generated suggestions from user-confirmed project knowledge.
- Corrections, disputes, rejection, staleness, and supersession remain auditable.
- A successful historical citation is preserved while current-source drift remains visible.
- Concurrent agents cannot overwrite or silently merge one another’s proposals.
- A desktop trust action cannot approve a concurrent learning version the user did not inspect.
- The Markdown review file is human-readable but not authoritative.
- Version 1 does not automatically trust a claim based only on corroboration count.
- Deletion and promotion into an ordinary note require later explicit workflows.
