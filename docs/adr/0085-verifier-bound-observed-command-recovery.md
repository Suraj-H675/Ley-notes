# ADR 0085: Verifier-bound isolated observed Command recovery

## Status

Accepted.

Extends ADR 0069. The read-only observed-Command verifier remains non-authoritative; this ADR adds a
separate explicit writer only while the session is active and the recovery window contains one
complete current tool observation with no current turn evidence or sibling tool observations.

## Context

ADR 0066 retained bounded supported host-tool observations as `toe_` evidence. ADR 0067 derived a
read-only automatic Command candidate from one complete retained Bash observation, and ADR 0069 gave
that candidate a deterministic fingerprint plus an exact-source verifier.

ADR 0069 deliberately stopped before persistence because every recovery checkpoint advances the
session recovery boundary. Checkpointing one tool observation while prompt/response `tev_` evidence
from the same post-checkpoint interval remained unconsolidated would strand that turn evidence. The
same problem exists for multiple current tool observations if only one were checkpointed.

The safe first durable slice therefore does **not** solve arbitrary mixed turn/tool consolidation. It
handles only the case where the exact observed Command source is already the complete current tool
window and the current turn-evidence window is empty.

## Decision

Ley adds `commit_observed_command_memory_transition` in core and the write-gated MCP tool
`ley_session_memory_commit_observed_command`.

The existing verifier now reports:

- `currentTurnEvidenceCount`;
- `currentToolEvidenceCount`;
- `otherCurrentToolEvidenceCount`; and
- `candidateBindingAllowed`.

`candidateBindingAllowed` is true only when all of the following hold:

1. the exact `toe_` source still verifies as `review-required`;
2. the expected session event count is current;
3. the session status is still `active`;
4. the source is strictly after the latest checkpoint;
5. its retained Bash command is complete;
6. there are zero current post-checkpoint `tev_` prompt/response records; and
7. there are zero other current post-checkpoint tool observations.

This flag is **not** automatic write permission. `automaticWriteAllowed` remains false. A caller must
make a separate explicit write-enabled commit call carrying the exact verifier fingerprint,
`sourceRecordId`, expected event count, and a fresh idempotency request ID.

The writer derives the Command from the retained tool observation; callers do not resupply command
text, exit code, summary, source event ID, or observation kind.

## Session schema v16

An accepted write appends one recovery checkpoint event using session schema v16 and projects to
`session-v16.json`.

The checkpoint contains exactly one Command and no other record kinds:

- exact retained/redacted Bash command text;
- `exitCode: null`;
- fixed summary `Observed Bash invocation; execution outcome unproven`.

Its recovery provenance contains:

- the exact verifier candidate fingerprint;
- exactly one `toe_` evidence ID;
- one record-specific Command -> `toe_` binding;
- the immutable source tool event ID; and
- the original tool observation kind (`returned` or `explicit-failure`).

The binding fingerprint covers the session ID, expected event count, candidate fingerprint, source
event ID, observation kind, exact normalized Command record, and record binding. Session replay
reconstructs turn and tool recovery windows separately. Schema v16 requires an empty current turn
window and an exact one-record tool window. Older recovery schemas continue to require their complete
sorted `tev_` window and cannot carry source-tool provenance.

The writer re-runs the same isolation verifier under the session writer lock before append. Any new
turn or sibling tool observation after verification therefore makes the commit fail closed. Exact
already-committed retries replay idempotently without creating another checkpoint.

## Authority and outcome semantics

Persistence means only that Ley durably recorded one reviewed historical Bash invocation. It does not
prove:

- process execution success or failure;
- an exit status;
- test success/failure;
- Verification status;
- semantic importance;
- command canonicality; or
- live-source correctness.

`returned` and `explicit-failure` remain host lifecycle observations. Both persist with
`exitCode: null`; neither is converted into a Verification record.

Existing durable Commands with identical text remain advisory overlap handles only. Repeating the
same command in two distinct legitimate tool invocations represents distinct provenance and does not
silently suppress the later observation.

Generic, typed, batch, rich-Problem, and composite recovery routes continue to reject `toe_` evidence.
Mixed turn+tool atomic recovery remains deferred.

## Learning origin preservation

A later reviewed Learning may cite the recovered Command. That transformation must preserve the
Command's exact tool origin rather than laundering it into turn evidence.

Learning schema v3 therefore adds `LearningOriginSource::ToolEvidence { sessionId, recordId }`, where
`recordId` must be `toe_...`. The compact origin summary adds `toolEvidence`.

Schema-v1/v2 learning events remain readable. `ToolEvidence` is valid only for schema v3. Learning
replay rebuilds current projections as v3 while preserving older event history and projected
freshness. A v16 Command-derived Learning carries both the recovery candidate fingerprint and exact
`toe_` origin; it must not fabricate `TurnEvidence` for that source.

## Consequences

- a useful deterministic Command candidate can now become durable when doing so cannot strand other
  current evidence;
- the existing prompt/response recovery semantics and old recovery schema meanings remain unchanged;
- the write remains explicit, verifier-bound, idempotent, and stale-safe;
- origin-preserving Learning derivation now distinguishes tool evidence from turn evidence;
- no generic Command writer, automatic tool-to-memory conversion, mixed turn/tool recovery, or
  outcome inference is introduced.

## Verification

Focused core/MCP/evaluation coverage must prove:

- one isolated complete Bash observation in an active session verifies with `candidateBindingAllowed: true` while
  `automaticWriteAllowed` stays false;
- paused/completed/abandoned sessions report `session-not-active`, never advertise bindability, and
  reject the write attempt;
- a current `tev_` record makes binding unavailable and commit fail closed;
- a sibling current tool observation makes binding unavailable and commit fail closed;
- commit produces exactly one schema-v16 Command with `exitCode: null` and closes the current tool
  candidate window;
- exact retry replays without another event;
- restart/replay accepts the v16 projection and retains the exact Command;
- recovery derivation provenance resolves the Command to the exact source `toe_`;
- a raw schema-v14 `toe_` observation remains invalid as direct Learning proposal evidence;
- a later Learning persists schema-v3 `ToolEvidence` plus the recovery candidate fingerprint and no
  fake `TurnEvidence` for the tool source;
- erasing the source Ley session physically removes a Learning derived through that recovered Command,
  so exact `ToolEvidence` lineage does not create deletion residue;
- old schema-v2 Learning lineage remains readable;
- the real crash/recovery evaluator exercises schema-v14 capture -> candidate -> verifier ->
  schema-v16 commit/replay with zero privacy-canary leakage.
