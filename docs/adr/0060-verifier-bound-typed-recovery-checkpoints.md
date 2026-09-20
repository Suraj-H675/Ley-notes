# ADR 0060: Verifier-bound typed recovery checkpoints

- Status: Accepted
- Date: 2026-09-20
- Extends: ADR 0029, ADR 0030

## Context

Ley already captures bounded post-checkpoint prompt/response evidence, exposes that evidence through
the Memory Compiler, verifies proposed structured transitions against the complete current recovery
window, and can commit one verifier-bound `unresolved` claim without trusting an agent-authored
generic checkpoint payload.

That leaves a reliability gap for two common durable records that can be represented losslessly by
the verifier's existing claim shape:

- a **Decision** needs a title plus decision text; rationale and rejected alternatives are optional;
- a **Problem** needs a title plus observed symptom; expected behavior, attempts, and resolution are
  optional.

Other structured kinds are not lossless from `{kind, subject, statement, evidence}` alone. Plans and
tasks require status, attempts require outcome, verification requires status, and resolutions require
root-cause/change/verification fields. Inventing those values would violate Ley's evidence-before-
assertion and useful-uncertainty principles.

## Decision

Ley adds one separate bound recovery writer for exactly one verifier-approved `decision` or
`problem` claim.

Candidate interpretation remains **host supplied**. Ley does not run a background model, local
model, classifier, or automatic semantic extractor in this slice. The host reads the bounded
Memory Compiler evidence, proposes a claim, and calls the existing read-only transition verifier.

The write is permitted only when all of the following remain true under the session writer lock:

- the candidate contains exactly one Decision or Problem claim;
- the verifier state is `review-required`;
- no current recovery evidence is deferred or uncovered;
- the inspected `sessionEventCount` is still current;
- the exact verifier `candidateFingerprint` matches;
- the exact complete recovery `tev_` evidence set still matches; and
- deterministic overlap checks found no duplicate or same-subject changed record requiring explicit
  revision semantics.

The writer derives the durable checkpoint itself:

```text
Decision:
  summary = subject
  title = subject
  decision = statement
  rationale = ""
  alternatives = []

Problem:
  summary = subject
  title = subject
  symptom = statement
  expected = ""
  attempts = []
  resolution = null
```

No omitted typed field is inferred.

### Durable schema boundary

Existing schema-v3 unresolved recovery events remain unchanged. Typed bound recovery uses session
event/projection schema **v8** and `session-v8.json`.

The existing `RecoveryCheckpointRecorded` event envelope and provenance fields are reused, but v8
validation requires exactly one minimal Decision **or** exactly one minimal Problem and no other
typed records. Its recovery binding fingerprint advances to v2 and includes the candidate kind,
subject, statement, expected event count, verifier fingerprint, and complete sorted evidence IDs.
This prevents a stored Decision from being reinterpreted as a Problem (or vice versa) while
preserving the same mechanically resolvable recovery-origin lineage used by derived learnings.

### MCP authority

The new `ley_session_memory_commit_structured` tool is absent by default and appears only under the
existing explicit `--allow-session-writes` capability. Its input schema admits only `decision` and
`problem`; other verifier candidate kinds cannot be requested through this route.

The existing `ley_session_memory_commit_unresolved` tool remains separate and unchanged.

## Consequences

- A missed host checkpoint can now recover common Decision/Problem structure without routing through
  an unbound generic checkpoint write.
- Ley still does not claim semantic faithfulness merely because structural verification succeeded.
- Source turn evidence remains immutable and available after the derived checkpoint closes the
  recovery window.
- Recovery-derived learning lineage continues to expose the candidate fingerprint and exact turn
  evidence rather than laundering the checkpoint into independent authority.
- Plan, Task, Attempt, Resolution, Command, Verification, and Summary candidates remain review-only
  in this bound-write path until Ley has lossless typed inputs for their required semantics. ADR 0061
  later adds a separate typed-v2 verifier and schema-v9 bound writer for exactly one Task; this ADR's
  generic verifier and schema-v8 Decision/Problem contract remain unchanged.
- This slice does not perform cross-session consolidation, background scheduling, automatic learning
  promotion, or trust changes.

## Verification

The implementation must prove:

- exact Decision and Problem reconstruction with no invented optional fields;
- read-only MCP omission and write-enabled tool discovery with a two-value kind schema;
- complete-window, stale-count, fingerprint, and overlap enforcement through the existing verifier;
- exact retry idempotency;
- schema-v3 unresolved recovery compatibility;
- schema-v8 replay/projection and tamper rejection;
- recovery-origin lineage preservation; and
- deterministic end-to-end evaluation through real MCP compile → verify → typed commit → session
  read, with the recovery window closed only after a valid bound write.

## Deliberately deferred

- automatic/model-generated Memory Compiler candidates inside Ley;
- multi-claim atomic recovery commits;
- bound Plan/Task/Attempt/Resolution/Command/Verification/Summary writers. ADR 0061 later implements
  Task through a separate typed-v2 verifier/schema-v9 route without changing this schema-v8 contract;
- inference of status, expected behavior, rationale, outcomes, verification, or root cause;
- cross-session/background consolidation; and
- automatic trust, learning promotion, or retrieval-ranking changes from a recovery commit.
