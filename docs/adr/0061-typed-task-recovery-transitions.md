# ADR 0061: Typed Task recovery transitions

- Status: Accepted
- Date: 2026-09-20
- Extends: ADR 0029, ADR 0060

## Context

ADR 0060 proved a candidate-bound recovery path for records that are lossless from the generic
`{kind, subject, statement, evidence}` claim shape. That safely covers one Decision or Problem, but
the remaining rich episodic kinds carry required semantics that the generic shape cannot represent.

Task is the smallest useful next step. A durable Task requires:

- title;
- status (`pending`, `in-progress`, `completed`, `blocked`, or `cancelled`); and
- optional details.

The existing generic verifier compares Task title/details but does **not** include status in its
candidate fingerprint or overlap comparison. Extending the generic schema in place would therefore
either break existing fingerprint/replay contracts or make a status transition look like an exact
duplicate.

## Decision

Ley adds a separate typed-v2 recovery verifier and one bound Task writer. The existing generic
`verify_memory_transition`, `ley_session_memory_verify`, and v1 candidate fingerprints remain
unchanged.

The typed verifier accepts exactly one candidate in this first slice:

```text
Task candidate:
  title
  status
  details
  evidenceRecordIds
```

It reuses the existing current-window evidence accounting, stale-event detection, metadata-only
rejection, deferred-evidence semantics, and coverage requirements. Task-specific overlap logic then
compares normalized title plus exact status plus normalized details.

- same title + same status + same details → exact duplicate;
- same title + changed status and/or details → same-subject changed content;
- no overlap + complete current-window accounting → `review-required`.

The typed candidate fingerprint uses a new domain and binds:

- session ID;
- inspected event count;
- candidate kind (`task`);
- normalized title;
- exact task status;
- normalized details;
- sorted evidence IDs; and
- sorted deferred evidence IDs.

This preserves all existing v1 fingerprints byte-for-byte while making Task status transition-safe.

## Bound write

The write route accepts exactly one verifier-approved Task candidate. Ley re-runs typed verification,
requires `review-required`, requires the exact typed candidate fingerprint and complete recovery
evidence set, and derives the checkpoint itself:

```text
summary = task.title
tasks = [{ title, status, details }]
all other checkpoint record collections = []
```

No status, detail, attempt, resolution, command, verification result, or parent relation is inferred.

Typed Task recovery uses a new durable session schema version rather than changing schema-v8
Decision/Problem semantics. Existing schema-v3 unresolved and schema-v8 Decision/Problem recovery
events remain unchanged and readable.

## MCP authority

Ley adds:

- a read-only typed Task transition verifier; and
- a Task recovery commit tool exposed only under the existing explicit `--allow-session-writes`
  capability.

The generic verifier remains available for generic review-only candidates and the existing bound
unresolved/Decision/Problem flows.

## Consolidation Inbox relationship

ADR 0055 remains unchanged. The Inbox is an evidence-selection/review surface only. It does not
generate Task candidates, return turn bodies, invoke a model, mark evidence processed, or write the
Task recovery event. A host/user may deliberately inspect bounded evidence and submit a typed Task
candidate through the separate verifier/write flow.

## Consequences

- Task status becomes part of recovery identity instead of being silently discarded.
- A `pending → completed` change is review-visible revision evidence, not an exact duplicate.
- Empty Task details remain valid; Ley does not invent explanatory text merely to satisfy a generic
  statement field.
- Existing generic candidate/recovery fingerprints and stored recovery events remain compatible.
- Candidate generation remains host/user supplied and untrusted.
- Structural verification still does not prove semantic faithfulness or live-source correctness.

## Deliberately deferred

- multi-claim atomic recovery; ADR 0063 later implements it for the already-supported recovery shapes;
- Plan recovery; ADR 0062 later implements it through the typed-v2 verifier plus schema v10;
- nested Problem Attempt/Resolution recovery; ADR 0064 later implements one new rich Problem episode
  with ordered Attempts and an optional Resolution, and ADR 0065 later preserves that rich episode
  atomically with minimal siblings through a separate schema-v13 route. Updates to an existing
  historical Problem remain deferred;
- Command or Verification recovery;
- automatic/model-generated candidates inside Ley;
- background/scheduled consolidation;
- Inbox mutation or processed/acknowledged state; and
- automatic authority/trust promotion.

## Verification

The slice must prove:

- typed Task candidates can reach `review-required` with complete current evidence;
- Task status changes alter the typed candidate fingerprint;
- exact same Task state is detected as a duplicate;
- same title with changed status/details requires revision;
- empty details are valid without invented text;
- the full advertised 4,000-character Task details boundary verifies and commits without hidden
  bridge overhead;
- stale, invalid, metadata-only, uncovered, and deferred evidence retain existing fail-closed
  semantics;
- the bound writer rejects fingerprint substitution, stale windows, normalization/redaction drift,
  and incomplete evidence;
- exact retries are idempotent;
- generic v1 candidate fingerprints remain byte-stable;
- the new durable schema replays/tamper-checks without changing schema-v3/v8 behavior, including a
  mixed schema-v3 → schema-v8 → schema-v9 recovery ledger;
- MCP read/write capability boundaries remain exact; and
- a real recovery eval exercises compile → typed Task verify → bound commit → session projection →
  origin lineage with exact durable Task details and zero privacy leakage, including a Task-specific
  secret canary absent from recovery output and durable schema-v9 storage.
