# ADR 0040: On-demand Current Project State projection

## Status

Accepted.

Later extension: ADR 0076 upgrades the projection to schema v2 with compact exact-revision
active-project Specification authority handles plus changed/missing Specification attention. It does
not copy Specification bodies or reinterpret human intent into a second state store.

## Context

Ley's second P1 roadmap item is an explicit Current Project State projection. The existing project resume pack is intentionally a bounded continuity view: it helps an agent resume recent sessions and trusted learnings, but it does not claim to adjudicate a single canonical current state for every historical decision, task, or problem.

The North Star requires current state to expose active work, open tasks, recent decisions, verification outcomes, current constraints/knowledge, stale or conflicting knowledge, and revision freshness while preserving the distinction between historical evidence and current truth. In particular, untyped session decisions do not become current merely because they are recent.

## Decision

Ley implements Current Project State first as an **on-demand, non-persistent derived projection** exposed by `current_project_state` and the read-only MCP tool `ley_project_state`.

The projection uses existing typed authority/state rather than inventing new state inference:

- **working sessions** are only sessions explicitly in `active` or `paused` state;
- **working-state details** come only from the latest checkpoint of those active/paused sessions;
- **open work** includes pending/in-progress/blocked tasks, unresolved problems, and explicit unresolved checkpoint items from that latest working checkpoint;
- **recent decisions** may be returned from the latest checkpoint of inspected recent sessions, but every decision is labeled `historical-project-memory` with `currentStateProven: false`;
- **recent verification** preserves typed verification status and revision applicability without converting a past verification into a live-source claim;
- **trusted knowledge** comes only from existing `current-trusted` reviewed learning semantics;
- **attention-needed knowledge** reuses Ley's central learning review inbox, including review-required, contested, stale, and trusted-but-source-changed learnings;
- **revision freshness** reuses the P0 revision-freshness resolver and remains distinct from `liveSourceChecked`;
- the response carries a deterministic SHA-256 state fingerprint over captured snapshot/revision identity and the returned structured state;
- the projection is character-budgeted for returned text fields and reports omission/truncation coverage;
- no Current Project State document/cache is persisted in this slice.

The MCP reader uses the same historical-memory egress gate as project resume, Topic Dossiers, session/turn inspection, and learning readers. A project-level denial or fine-grained historical egress ceiling therefore cannot be bypassed through the state projection.

## Authority model

Current Project State is a **derived navigation/state view**, not a new authority type.

The projection deliberately separates:

1. explicit working state from active/paused latest checkpoints;
2. reviewed trusted-current knowledge;
3. historical decisions and verification;
4. knowledge requiring review or freshness attention.

A historical decision can be useful context while still having `currentStateProven: false`. The projection must not infer a newer-decision-wins rule from timestamps, names, or lexical similarity. Premise/state adjudication remains the Context Compiler's job for a concrete task, and live source remains authoritative for current implementation facts.

## Budget and coverage

`maxCharacters` is a strict aggregate budget over text copied into the projection, not a claim that the complete serialized JSON response is that many characters. IDs, enums, counters, booleans, revision metadata, and structural JSON add response overhead outside that text budget.

Coverage reports total/inspected sessions, working-session omissions, selected/returned/omitted decision, open-work, and verification records, trusted knowledge, attention-needed knowledge, plus a `truncated` summary flag.

## Deletion and rebuildability

Because Current Project State is rebuilt on demand and is not persisted, session/project erasure does not require a new cache-purge transaction. The end-to-end deletion-fidelity scenario explicitly calls `ley_project_state` after erasure and requires erased canaries to remain absent.

Any future persisted/background state document must add dependency-aware invalidation, erasure, egress inheritance, and concurrency semantics before it can replace this rebuildable projection.

## Consequences

Benefits:

- gives agents an explicit state view without flattening history into truth;
- reuses existing session status, learning trust/freshness, revision applicability, and egress semantics;
- surfaces open work and verification alongside reviewed knowledge;
- preserves truthful uncertainty about historical decisions;
- remains disposable and deletion-friendly.

Tradeoffs:

- the latest checkpoint is treated as the bounded continuation state of an active/paused session; Ley does not globally reconcile task identity across every older checkpoint;
- recent historical decisions are useful but intentionally not adjudicated as current;
- the first slice does not auto-merge multiple active agents into one canonical working state;
- the projection is generated on demand rather than continuously maintained in the background.
