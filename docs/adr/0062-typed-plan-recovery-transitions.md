# ADR 0062: Typed Plan recovery transitions

- Status: Accepted
- Date: 2026-09-20
- Extends: ADR 0029, ADR 0061

## Context

ADR 0061 added a separate typed-v2 verifier and schema-v9 writer for Task because the generic
`{kind, subject, statement, evidence}` candidate shape discarded required Task status. Plan has the
same concrete failure in a smaller durable shape:

```text
PlanItem:
  text
  status = pending | in-progress | completed | blocked
```

The generic verifier currently projects an existing Plan as `subject = text` and
`statement = text`. Status is therefore absent from generic candidate fingerprints and overlap
comparison. A `pending → completed` Plan transition can look like an exact duplicate.

Plan is the smallest remaining rich episodic record. Command and Verification introduce execution,
exit-status, or snapshot-evidence semantics. Attempt and Resolution are nested under Problem and
require parent linkage. Multi-claim atomic recovery does not itself solve missing typed fields.

## Decision

Extend the existing read-only typed-v2 verifier with exactly one additional candidate variant:

```text
Plan candidate:
  text
  status
  evidenceRecordIds
```

The existing generic verifier and all generic-v1 candidate fingerprints remain unchanged. The
existing Task variant and Task typed-v2 fingerprints also remain byte-identical.

Plan overlap semantics are:

- same normalized text + same exact status → exact duplicate;
- same normalized text + changed status → same-subject changed content;
- different normalized text → no Plan overlap.

The typed candidate fingerprint keeps the existing `ley-memory-transition-v2-typed` domain and
binds:

- session ID;
- inspected event count;
- candidate kind (`plan`);
- normalized Plan text;
- exact Plan status;
- sorted evidence IDs; and
- sorted deferred evidence IDs.

The typed verifier continues to reuse the proven generic recovery-window accounting for stale,
invalid, metadata-only, duplicate, uncovered, and deferred evidence. Any internal bridge used for
that accounting must not silently impose the generic 256-character subject limit on Plan's public
4,000-character text field.

## Bound write

The bound write route accepts exactly one verifier-approved Plan candidate. Ley re-runs typed
verification, requires `review-required`, requires the exact typed
candidate fingerprint and complete recovery evidence set, and derives exactly one Plan item:

```text
summary = plan.text
plan = [{ text, status }]
all other checkpoint record collections = []
```

No status, text, rationale, Task, command, verification, problem relation, or completion state is
inferred.

Plan recovery uses durable schema version 10 and `session-v10.json`. Schema-v9 remains Task-only and
its replay contract is not broadened after release.

## MCP authority

The existing read-only `ley_session_memory_verify_typed` route gains the Plan variant while preserving
the existing Task encoding. `ley_session_memory_commit_plan` is a separate explicit write route exposed
only under `--allow-session-writes`; the verifier itself remains read-only.

## Consequences

- Plan lifecycle state becomes review-visible and transition-safe.
- `pending → completed` is a revision, not an exact duplicate.
- Full-length Plan text can be verified without invented truncation or hidden bridge overhead.
- Existing generic-v1 and Task typed-v2 fingerprints remain compatible.
- Candidate generation remains host/user supplied and untrusted.
- `review-required` remains structural accounting, not semantic proof, live-source proof, or trust.

## Deliberately deferred

- multi-claim atomic recovery; ADR 0063 later implements it for the already-supported recovery shapes
  through a separate batch fingerprint and schema-v11 writer, without broadening typed-v2;
- Command recovery;
- Verification recovery;
- nested Attempt/Resolution recovery;
- automatic/model-generated candidates inside Ley;
- background/scheduled consolidation;
- Inbox mutation or processed/acknowledged state; and
- automatic authority/trust promotion.

## Verification

The slice must prove:

- typed Plan candidates reach `review-required` with complete current evidence;
- exact status changes alter the Plan typed-v2 fingerprint;
- same Plan text/status is an exact duplicate;
- same Plan text with changed status requires revision;
- the full advertised 4,000-character Plan text boundary verifies without hidden generic-subject
  truncation;
- stale, invalid, metadata-only, uncovered, and deferred evidence retain fail-closed semantics;
- the generic-v1 fingerprint golden remains unchanged;
- the existing Task typed-v2 fingerprint golden remains unchanged;
- the schema-v10 writer preserves exact Plan text/status, exact retry is idempotent, replay/tamper
  checks preserve schema-v3/v8/v9 history, and MCP write authority remains explicit; and
- the real recovery eval proves the Plan path with zero privacy leakage before landing the write.
