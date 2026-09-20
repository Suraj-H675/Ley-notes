# ADR 0064: Typed rich Problem recovery

- Status: Accepted
- Date: 2026-09-20
- Extends: ADR 0029, ADR 0060, ADR 0063

## Context

Ley can already store a rich structured debugging episode as one Problem containing expected
behavior, ordered Attempts with outcomes/evidence, and an optional Resolution. Recovery is weaker:
the generic schema-v8 Problem route and schema-v11 batch Problem candidate preserve only
`{title, symptom}`. If a crash happens after several failed attempts and a verified fix, reconstructing
only the minimal Problem loses exactly the experience that the Memory Compiler is intended to retain.

Standalone recovered Attempt or Resolution records are not a safe first answer. Both are nested under
Problem and need an unambiguous parent/update contract. Attaching a recovered Attempt to an older
Problem with the same title could silently rewrite history, while persisting a detached Attempt or
Resolution would destroy the durable relation Ley already models.

The smallest lossless extension is therefore one **new rich Problem episode**. It does not mutate an
existing Problem. Same-title existing memory remains overlap evidence requiring revision/review rather
than an automatic parent update.

## Decision

Add a separate typed rich-Problem recovery family. It does not broaden generic-v1, typed-v2, batch-v3,
schema-v8, or schema-v11 contracts.

The read-only verifier accepts exactly one candidate:

```text
problem:
  title
  symptom
  expected
  evidenceRecordIds
  attempts[]:
    action
    outcome = helped | no-effect | worsened | unknown
    evidence
    evidenceRecordIds
  resolution?:
    rootCause
    change
    verification
    evidenceRecordIds
```

Every durable component must cite at least one current `tev_` record. Evidence may be shared between
components, but the union of Problem, Attempt, Resolution, and explicitly deferred evidence must still
account for the complete current recovery window. Deferred evidence keeps the transition
non-committable.

The verifier reuses the existing deterministic recovery-window checks for event-count staleness,
invalid/metadata-only evidence, used-plus-deferred conflicts, uncovered evidence, and bounded output.
It then performs rich Problem overlap comparison against existing structured session memory.

Rich Problem overlap semantics are:

- same normalized title plus exact normalized symptom/expected, identical ordered Attempts
  (`action`, `outcome`, `evidence`), and identical optional Resolution
  (`rootCause`, `change`, `verification`) → exact duplicate;
- same normalized title with any changed rich field, Attempt order/count/outcome, or Resolution
  presence/content → same-subject changed content;
- different normalized title → no rich Problem overlap.

No existing Problem is automatically updated or replaced.

## Candidate fingerprint

Rich Problem verification uses a new fingerprint domain:

```text
ley-memory-transition-v4-rich-problem
```

It binds:

- session ID;
- inspected event count;
- normalized title, symptom, and expected text;
- sorted Problem evidence IDs;
- each Attempt in durable order, including normalized action, exact outcome, normalized evidence, and
  sorted Attempt evidence IDs;
- optional Resolution presence plus normalized root cause/change/verification and sorted Resolution
  evidence IDs; and
- sorted deferred evidence IDs.

Attempt order is transition-significant because durable debugging history is ordered. Evidence-ID
order inside one component is not significant.

Existing generic-v1, typed-v2, and batch-v3 fingerprint bytes remain unchanged.

## Bound write

`ley_session_memory_commit_problem` accepts exactly one verifier-approved rich Problem candidate and
appends one new Problem episode. It does not target or mutate an existing Problem.

The writer must:

- replay an exact already-committed retry before requiring the recovery window to remain open;
- re-run the complete rich Problem verifier against the session snapshot while the session writer
  lock is held;
- require `review-required` with no deferred evidence;
- require the exact candidate fingerprint and exact current recovery window;
- normalize/redact the supplied checkpoint payload before append and reject any candidate identity
  drift caused by that normalization;
- preserve exact Attempt order/outcomes/evidence and optional Resolution fields without inference;
- append exactly one recovery event; and
- close the recovery window once.

The durable checkpoint shape is:

```text
summary = problem.title
problems = [the exact supplied rich Problem]
all other checkpoint collections = []
```

Rich Problem recovery uses schema version 12 and `session-v12.json`. Earlier schemas remain readable
and byte-compatible.

## Provenance and lineage

Schema v12 persists both the complete recovery evidence union and an exact binding for each durable
child:

- Problem record ID → Problem evidence subset;
- each Attempt record ID → that Attempt's evidence subset; and
- optional Resolution record ID → Resolution evidence subset.

The binding has its own `ley-recovery-checkpoint-binding-v6-rich-problem` fingerprint domain. Replay
recomputes both candidate identity and binding integrity. Rebinding evidence between children must fail
even if an attacker recomputes the surrounding candidate/request fingerprints.

A learning citing the checkpoint itself may resolve to the complete recovery union. A learning citing
the recovered Problem, one Attempt, or its Resolution resolves only that child's persisted evidence
subset while retaining the overall rich-Problem candidate fingerprint. Provenance remains evidence of
derivation, not semantic proof.

## MCP authority

`ley_session_memory_verify_problem` is read-only and available in the default MCP surface.
`ley_session_memory_commit_problem` is a separate route available only when
`--allow-session-writes` was granted at process startup.

Neither route grants filesystem, network, tool, review, trust, or egress authority. Stored recovery
text cannot enable the write route.

## Consequences

Interrupted debugging episodes can retain failed attempts, outcomes, expected behavior, and the final
resolution instead of collapsing to a minimal symptom record. Future learning proposals can cite the
specific failed Attempt or Resolution evidence without laundering unrelated turns. Old recovery
schemas and fingerprint domains remain stable, and ambiguous mutation of existing Problems is still
avoided.

## Deliberately deferred

- attaching a recovered Attempt or Resolution to an existing historical Problem;
- rich Problem candidates inside schema-v11 atomic multi-claim batches; ADR 0065 later preserves the
  same atomic use case through a separate schema-v13 composite family instead of broadening v11 or
  this schema-v12 contract;
- standalone Command recovery and execution-occurrence identity;
- standalone Verification recovery and observed-vs-recovered result semantics;
- standalone Summary recovery;
- automatic/model-generated recovery candidates inside Ley;
- background/scheduled consolidation or Inbox mutation; and
- automatic authority/trust promotion.

## Verification

The slice must prove complete component-level evidence accounting, Attempt-order-sensitive candidate
identity, exact duplicate vs same-title revision behavior, fail-closed missing/deferred/stale evidence,
schema-v12 append/replay/exact retry, lock-held re-verification, normalization/redaction rejection,
nested-field substitution rejection, record-binding tamper resistance, component-specific learning
lineage, v3/v8/v9/v10/v11 replay compatibility, explicit MCP write gating, public MCP protocol
discovery, packaged-host guidance, and a real interrupted-debugging eval with zero privacy leakage.
