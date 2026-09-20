# ADR 0065: Atomic rich-Problem composite recovery

- Status: Accepted
- Date: 2026-09-20
- Extends: ADR 0029, ADR 0063, ADR 0064

## Context

ADR 0063 made a recovery window atomic for 2–50 minimal unresolved/Decision/Problem/Task/Plan
candidates. ADR 0064 then made one complete rich Problem episode recoverable without losing expected
behavior, ordered Attempts/outcomes/evidence, or optional Resolution.

Those two contracts expose one remaining data-loss seam. One interrupted debugging window can support
all of the following at once:

- a complete Problem → Attempts → Resolution episode;
- a Decision caused by that investigation;
- a Task or Plan state change caused by the fix; and
- unresolved follow-up.

Committing the rich Problem through schema v12 closes the whole recovery window and strands the
sibling claims. Committing the siblings through schema v11 closes the same window while collapsing the
Problem to `{title, symptom}`. Sequential writes cannot solve this because the first checkpoint moves
the entire window behind the recovery boundary.

This is the same atomicity failure ADR 0063 solved for minimal candidates, now at the rich-Problem
composition seam.

The existing contracts must remain immutable:

- schema-v11 / `ley-memory-transition-v3-batch` continues to accept exactly its current minimal
  candidate family;
- schema-v12 / `ley-memory-transition-v4-rich-problem` continues to accept exactly one rich Problem
  and no sibling collections.

Broadening either released contract in place would make replay/fingerprint meaning conditional on new
payloads and would weaken compatibility guarantees.

## Decision

Add a **separate composite recovery family** with one explicit rich Problem and one or more existing
minimal sibling candidates.

The read-only verifier accepts:

```text
checkpointSummary
expectedEventCount
richProblem
siblings[1..49]
deferredEvidenceRecordIds
```

`richProblem` is exactly the ADR-0064 candidate shape:

```text
problem:
  title
  symptom
  expected
  evidenceRecordIds
  attempts[]:
    action
    outcome
    evidence
    evidenceRecordIds
  resolution?:
    rootCause
    change
    verification
    evidenceRecordIds
```

Each sibling is exactly one existing ADR-0063 minimal batch candidate:

```text
unresolved { text, evidenceRecordIds }
decision   { title, decision, evidenceRecordIds }
problem    { title, symptom, evidenceRecordIds }
task       { title, status, details, evidenceRecordIds }
plan       { text, status, evidenceRecordIds }
```

The first slice allows exactly one rich Problem. Multiple rich Problems remain deferred so the durable
Problem ordering and nested provenance contract stay simple and reviewable.

The top-level candidate count is therefore 2–50. In addition, the total number of verifier components
must remain at most 50:

```text
1 rich Problem parent
+ number of rich Attempts
+ optional rich Resolution
+ number of sibling candidates
<= 50
```

This preserves the existing bounded claim-check surface. A very large rich Problem can still use the
single schema-v12 route when it cannot fit a composite recovery.

## Evidence accounting

Every durable component must cite current `tev_` evidence. Evidence may be shared by several
components. The union of:

- rich Problem evidence;
- every rich Attempt/Resolution evidence subset;
- every sibling evidence subset; and
- explicitly deferred evidence

must cover the complete current recovery window. Invalid, metadata-only, duplicate, uncovered,
used-plus-deferred, or stale evidence remains fail-closed. Any deferred evidence keeps the transition
non-committable.

`review-required` remains only structural accounting. It does not prove semantic faithfulness,
live-source correctness, execution, verification, or trust.

## Overlap and intra-composite semantics

Existing semantics are reused without changing their old fingerprint domains:

- the rich Problem uses ADR-0064 full-episode exact/revision comparison;
- Task and Plan siblings retain typed status-aware overlap semantics;
- minimal unresolved/Decision/Problem siblings retain ADR-0063 semantics;
- duplicate/conflicting minimal siblings remain rejected before write.

A minimal Problem sibling with the same normalized title as the rich Problem is also rejected:

- same normalized symptom → duplicate candidate;
- changed normalized symptom → conflicting candidate.

The verifier never stores both as separate same-subject Problem records.

## Composite fingerprint

Use a new domain:

```text
ley-memory-transition-v5-composite
```

It binds:

- session ID;
- inspected event count;
- normalized explicit checkpoint summary;
- the complete rich Problem payload, exact Attempt order, and component evidence bindings;
- all minimal sibling payloads and evidence bindings; and
- sorted deferred evidence IDs.

Fingerprint ordering follows durable checkpoint collection semantics. Cross-kind sibling interleaving
does not affect identity. Same-kind sibling relative order does. For the Problem collection, the one
rich Problem is canonicalized first, followed by minimal Problem siblings in their relative input
order. Rich Attempt order remains transition-significant.

The existing generic-v1, typed-v2, batch-v3, and rich-Problem-v4 fingerprint bytes must remain
unchanged.

## Bound write and durable schema

Add one explicit writer that appends exactly one composite checkpoint. It must:

- replay an exact committed retry before requiring the now-closed recovery window to remain open;
- re-run the complete composite verifier while the session writer lock is held;
- require `review-required` with no deferred evidence;
- require the exact composite fingerprint and complete current recovery window;
- normalize/redact the candidate-derived checkpoint before append and reject any identity drift;
- preserve exact rich Attempt order and all existing typed sibling fields;
- append one event and close the recovery window once.

Composite recovery uses schema version 13 and `session-v13.json`.

The durable checkpoint contains the explicit summary plus all supplied supported record collections.
The rich Problem is the first Problem record; minimal Problem siblings follow it. No Command,
Verification, touched-artifact, or unsupported derived state is invented.

## Provenance and lineage

Use a new binding domain:

```text
ley-recovery-checkpoint-binding-v7-composite
```

The event preserves:

1. the complete sorted recovery-window evidence union; and
2. an exact evidence binding for every durable child:
   - rich Problem parent;
   - every rich Attempt;
   - optional rich Resolution;
   - every Decision/Task/Plan/minimal-Problem sibling; and
   - every deterministic `unr_...` unresolved child.

Replay reconstructs the composite candidate from the checkpoint and record bindings, re-verifies the
candidate fingerprint, and recomputes binding integrity. Rebinding evidence between any two children
must fail even if outer request/candidate fingerprints are recomputed.

A learning citing the checkpoint receives the complete composite evidence union. A learning citing any
child receives only that child's persisted evidence subset while retaining the overall composite
candidate fingerprint. This prevents a failed Attempt, Resolution, Decision, or sibling claim from
laundering unrelated turns from the same atomic checkpoint.

## MCP authority

Expose a separate read-only `ley_session_memory_verify_composite` route. It is available without
session-write capability.

Expose `ley_session_memory_commit_composite` only under the existing `--allow-session-writes`
process capability.

Existing `ley_session_memory_verify_batch` / `commit_batch` and
`ley_session_memory_verify_problem` / `commit_problem` behavior remains unchanged.

Stored candidate text, checkpoint summary, and recovery evidence grant no filesystem, network, tool,
review, trust, or egress authority.

## Consequences

- A rich debugging episode can survive atomically with the decisions/open work it caused.
- Schema-v11 and schema-v12 contracts stay simple and immutable.
- No historical Problem mutation semantics are introduced.
- Future automatic candidate generation gets a lossless composite target instead of choosing which
  supported fact to discard.
- The MCP recovery surface gains one additional verifier/writer pair; that interface cost is accepted
  to keep old contracts deep, local, and compatibility-safe.

## Deliberately deferred

- multiple rich Problems in one composite checkpoint;
- attaching recovered Attempt/Resolution children to an existing historical Problem;
- standalone Command recovery until authoritative host execution evidence exists;
- standalone Verification recovery until observed-vs-recovered provenance is explicit;
- standalone Summary recovery;
- automatic/model-generated recovery candidates inside Ley;
- background/scheduled consolidation or Inbox mutation; and
- automatic authority/trust promotion.

## Verification

The slice must prove:

- one rich Problem plus at least one minimal sibling can account for one complete window;
- the 50-component bound is strict;
- shared evidence, deferred evidence, stale counts, invalid IDs, metadata-only evidence, and uncovered
  evidence remain fail-closed;
- rich Problem overlap and typed Task/Plan sibling overlap semantics remain exact;
- rich-vs-minimal same-title Problem duplicate/conflict cases fail deterministically;
- cross-kind sibling reordering leaves identity stable while same-kind ordering and rich Attempt order
  change identity;
- old v1/v2/v3/v4 candidate fingerprint goldens remain byte-identical;
- schema-v13 append, exact retry, stale-window rejection, normalization/redaction rejection,
  substitution rejection, record-binding rebinding resistance, and v3/v8/v9/v10/v11/v12 replay
  compatibility;
- checkpoint-level and child-level learning lineage remain exact;
- default MCP exposes the verifier but not the writer, and explicit session-write mode adds the writer;
- packaged Codex/Claude guidance selects composite recovery instead of sequential writes; and
- a real interrupted-debugging eval proves the rich Problem and siblings all survive one atomic
  checkpoint with zero privacy leakage.
