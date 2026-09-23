# ADR 0069: Verifier-bound observed Command candidates

Later extension: ADR 0085 adds a separate explicit schema-v16 writer only when the verified `toe_`
source is the sole current tool observation and there is no current post-checkpoint `tev_` evidence.
The persistence boundary below records this ADR's original read-only slice.

## Context

ADR 0066 added deterministic, bounded host-tool observations as schema-v14 session evidence.
ADR 0067 then derived a read-only automatic Command candidate from one complete retained
post-checkpoint Bash observation. That candidate intentionally had no durable fingerprint, verifier
anchor, or writer because a normal tool return does not prove command success, test success, or
Verification outcome.

The next useful step is narrower than persistence: let an agent re-check that the exact deterministic
candidate it saw is still current after the session may have changed. Reusing the existing generic
Memory Transition verifier would be incorrect because its evidence namespace is deliberately limited
to candidate-bound prompt/response `tev_` records. Broadening that namespace would silently change
the meaning of all existing recovery writers.

## Decision

Ley adds a separate read-only observed-Command verifier:
`verify_observed_command_memory_transition` in core and
`ley_session_memory_verify_observed_command` in MCP.

The input contains only:

- the fixed Ley session ID;
- the exact expected session event count; and
- one exact `toe_` source-record ID returned by Memory Compiler.

The verifier succeeds as `review-required` only when the source still identifies one complete
retained Bash observation strictly after the latest structured/recovery checkpoint. It re-checks the
current session event count, source identity, checkpoint boundary, capture retention, command
presence, and capture truncation. Missing, pre-checkpoint, omitted, or command-truncated sources fail
closed as `needs-revision`; any later session event makes the result `stale`. Unsupported/non-Bash
tool observations are rejected by the schema-v14 capture/replay boundary before they can become a
valid verifier source; the verifier retains the same defensive Bash-only check.

The verified candidate continues to carry:

- the exact retained/redacted command text;
- `exitCode: null`;
- the original observation kind (`returned` or `explicit-failure`);
- no success/failure/test/Verification claim;
- `persisted: false`;
- `candidateBindingAllowed: false`;
- `automaticWriteAllowed: false`;
- `verificationClaimed: false`; and
- `outcomeProven: false`; and
- `semanticFaithfulnessProven: false`.

`explicit-failure` therefore remains a host lifecycle observation, not an inferred process exit
code or failed Verification record.

## Candidate identity

Eligible Memory Compiler rows now expose the same deterministic candidate fingerprint produced by
the verifier. The `ley-memory-transition-v6-observed-command` fingerprint binds:

- session ID;
- expected session event count;
- exact `toe_` source-record ID;
- exact source event ID;
- observation kind;
- exact retained/redacted command bytes; and
- the fixed unknown-outcome Command summary.

The source observation, not normalized command text, is the identity anchor. Repeating the same
command in two legitimate tool invocations therefore produces distinct provenance. Existing durable
Command rows with identical command text are reported only as advisory overlap handles; they do not
make a new observation false or silently suppress it.

## Persistence boundary

This ADR does **not** add a Command recovery writer, session schema version, checkpoint event, or new
recovery evidence namespace. `toe_` remains invalid input to the generic, typed, batch, rich-Problem,
and composite recovery verifiers/writers.

That restriction is load-bearing. Today a recovery checkpoint advances the prompt/response
post-checkpoint boundary. A tool-only writer could therefore checkpoint one `toe_` candidate while
silently stranding unconsolidated `tev_` turn evidence from the same session interval. A future
durable Command writer must first define atomic mixed turn/tool recovery accounting, exact derivation
provenance, stale-write behavior under the session writer lock, and overlap/replay semantics. It
requires a separate ADR.

Verification is therefore review support, not write authorization.

## Consequences

- automatic Command candidates can be safely revalidated after compilation without becoming trusted;
- any intervening session mutation invalidates an earlier candidate view through event-count
  staleness;
- the existing prompt/response recovery contracts and fingerprints remain byte-for-byte separate;
- no command/test outcome is inferred from host hook shape or result text;
- no checkpoint boundary advances and no durable memory is created by verification; and
- the design creates a precise precursor for a later atomic writer without prematurely broadening
  recovery authority.

## Verification

The slice must prove:

- compiler and verifier produce the same fingerprint for the same complete retained Bash source;
- a later session event makes an earlier verification request stale;
- a source at or before the latest checkpoint fails closed;
- Minimal/capacity-omitted, missing, and capture-truncated command sources cannot verify, while
  unsupported/non-Bash tool observations are rejected before they can enter the supported source
  namespace;
- `returned` and `explicit-failure` preserve their observation kind while keeping exit/outcome and
  Verification unknown;
- malformed/non-`toe_` source IDs are rejected;
- the new MCP route is read-only, idempotent, closed-world, and available in both read-only and
  session-write server modes;
- the candidate remains unavailable to every recovery writer; and
- the existing crash/recovery evaluation exercises candidate fingerprint + verifier parity rather
  than adding a weaker standalone scenario.
