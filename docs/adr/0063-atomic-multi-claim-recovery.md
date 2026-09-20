# ADR 0063: Atomic multi-claim recovery

- Status: Accepted
- Date: 2026-09-20
- Extends: ADR 0029, ADR 0030, ADR 0060, ADR 0061, ADR 0062

## Context

Ley can now candidate-bind and durably recover exactly one unresolved claim, Decision/Problem,
Task, or Plan. Those single-claim paths are intentionally narrow and replay-safe, but they expose a
remaining correctness gap: every recovery checkpoint closes the complete post-checkpoint evidence
window.

A realistic interrupted window may support several independent facts at once, for example one
Decision, one completed Task, and one updated Plan. Committing any one of those through the current
single-claim routes makes the remaining evidence part of historical pre-checkpoint state, so the
other supported facts can no longer be reconstructed through Memory Compiler. Sequential writes are
therefore not an atomic multi-claim substitute.

The generic verifier already supports multiple claims and union evidence coverage, but typed Task and
Plan fields cannot be collapsed back into the generic `{kind, subject, statement, evidence}` shape
without losing status. A safe batch path must preserve those typed distinctions and must remain
replayable from durable state.

Command and Verification are deliberately not added by this ADR. Repeated identical command
executions or checks can be distinct historical occurrences, so Task/Plan-style same-subject
duplicate/revision semantics are not yet a safe temporal identity model for them. Attempt and
Resolution remain nested under Problem and require an explicit parent/update contract.

## Decision

Add a separate batch recovery candidate family. It does not modify generic-v1 or typed-v2 candidate
fingerprints and does not broaden schema-v3/v8/v9/v10 events.

The read-only batch verifier accepts:

```text
checkpointSummary
expectedEventCount
candidates[2..50]
deferredEvidenceRecordIds
```

Each candidate is exactly one of:

```text
unresolved { text, evidenceRecordIds }
decision   { title, decision, evidenceRecordIds }
problem    { title, symptom, evidenceRecordIds }
task       { title, status, details, evidenceRecordIds }
plan       { text, status, evidenceRecordIds }
```

The shapes intentionally match the information that the existing single-claim recovery writers can
persist without invention. Decision rationale/alternatives and Problem expected/attempt/resolution
remain absent from this recovery family.

`checkpointSummary` is explicit host/user-supplied candidate text. Ley does not synthesize the first
claim, concatenate candidate text, or invent an editorial summary. It is fingerprint-bound and remains
untrusted/review-required like the candidate interpretations themselves.

The verifier reuses the current recovery-window accounting:

- every candidate must cite at least one current `tev_` record;
- evidence may support more than one candidate;
- the union of cited plus explicitly deferred evidence must cover the complete current window;
- stale, invalid, metadata-only, uncovered, and used-plus-deferred evidence fail closed;
- any deferred evidence keeps the transition non-committable.

Plan and Task use their existing typed status-aware overlap semantics. Unresolved, Decision, and
Problem preserve their existing generic overlap semantics. The batch verifier additionally rejects
duplicate/conflicting candidates inside the same proposed batch before any write:

- unresolved candidates with the same normalized text are duplicates;
- Decisions with the same normalized title are duplicates only when decision text also matches;
- Problems with the same normalized title are duplicates only when symptom text also matches;
- Tasks with the same normalized title are duplicates only when status and details also match;
- Plans with the same normalized text are duplicates only when status also matches;
- same durable subject with different state/content is an intra-batch conflict for Decision,
  Problem, Task, and Plan.

## Batch fingerprint

The batch verifier uses a new `ley-memory-transition-v3-batch` fingerprint domain binding:

- session ID;
- inspected event count;
- normalized explicit checkpoint summary;
- every typed candidate payload;
- each candidate's sorted evidence IDs; and
- sorted deferred evidence IDs.

Fingerprint ordering follows durable checkpoint semantics, not arbitrary mixed-kind input
interleaving. Candidates are canonicalized by checkpoint collection kind; relative order among
candidates of the same kind is preserved because that order survives in the durable collection.
Reordering a Decision around a Task therefore does not change identity, while swapping two Decisions
does.

Existing generic-v1 and typed-v2 fingerprint bytes remain unchanged.

## Bound write

One explicit write route appends exactly one atomic recovery checkpoint containing the verified
candidate set. It must:

- re-run batch verification under the session writer lock;
- require `review-required` with no deferred evidence;
- require the exact batch fingerprint and exact complete recovery window;
- preserve candidate order within each durable record collection;
- derive only the supplied supported record fields;
- append exactly one recovery event and close the recovery window once; and
- replay exact retries before requiring the now-closed window to be open again.

The write uses a new durable schema version rather than broadening v3/v8/v9/v10.

The durable provenance must preserve both:

1. the complete sorted recovery-window evidence set; and
2. the exact evidence subset supporting each derived durable record.

This per-record binding is required so later learning lineage does not attribute unrelated recovery
turns to every child record in the atomic checkpoint. A learning citing the checkpoint itself may
resolve to the full batch union; a learning citing a derived child record must resolve only that
candidate's bound evidence subset while retaining the overall batch candidate fingerprint.
Because durable checkpoints intentionally retain unresolved work as strings, the read projection
derives stable index-aligned `unr_...` child IDs without rewriting the ledger. Those IDs are valid
learning-evidence citation handles and resolve through the same schema-v11 per-record binding.

## MCP authority

Batch verification is read-only and available without write authority. The atomic batch commit is a
separate route exposed only under the existing `--allow-session-writes` process capability. Existing
single-claim verify/commit routes remain unchanged for compatibility and remain the preferred surface
for a single candidate.

Stored candidate text, recovery evidence, and checkpoint summary grant no tool/filesystem/network,
review, trust, or egress authority.

## Consequences

- A single interrupted evidence window can preserve several already-supported structured facts
  without first-write data loss.
- The recovery interface gains composability without broadening the semantic surface to new record
  kinds.
- Old fingerprint domains and recovery schemas remain immutable.
- Batch origin lineage can remain record-specific instead of laundering the full recovery window into
  every derived child record.
- Candidate generation remains host/user supplied and untrusted; `review-required` is structural
  accounting, not semantic proof or live-source proof.

## Deliberately deferred

- Command recovery and execution-occurrence identity;
- Verification recovery and recovered-vs-observed outcome provenance;
- nested Attempt/Resolution parent/update semantics; ADR 0064 later implements one separate single
  rich-Problem recovery episode with ordered Attempts and optional Resolution. ADR 0065 later composes
  that rich episode with minimal siblings through a separate schema-v13 route, without broadening the
  schema-v11 batch contract; mutation of an existing Problem remains deferred;
- standalone Summary recovery;
- automatic/model-generated candidates inside Ley;
- background/scheduled consolidation or Inbox mutation; and
- automatic authority/trust promotion.

## Verification

The read-only phase must prove:

- 2–50 mixed supported candidates can share or split the same recovery evidence;
- union coverage, stale, invalid, metadata-only, uncovered, and deferred semantics remain fail-closed;
- typed Task/Plan status-aware overlap semantics remain exact;
- intra-batch duplicates/conflicts are rejected deterministically;
- mixed-kind interleaving does not change the batch fingerprint;
- same-kind relative reordering does change the fingerprint;
- checkpoint summary changes alter the fingerprint;
- generic-v1 and typed-v2 golden fingerprints remain unchanged.

The writer implementation must additionally prove a new durable schema with atomic append/replay,
exact retry, stale-window rejection, normalization/redaction rejection, tamper/rebinding resistance,
v3/v8/v9/v10 compatibility, record-specific recovery lineage, explicit MCP write gating, and a real
interrupted-window eval containing several claims with zero privacy leakage.
