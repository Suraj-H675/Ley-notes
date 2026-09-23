# ADR 0067: Deterministic observed Command candidates

Later extensions: ADR 0069 adds exact source-bound verification; ADR 0085 adds an explicit
schema-v16 writer only for an isolated complete current tool-evidence window. The persistence
boundary below records this ADR's original read-only slice.

- Status: Accepted
- Date: 2026-09-21
- Extends: ADR 0029, ADR 0055, ADR 0066
- Supersedes: ADR 0066 only for its deferral of an automatic **read-only** Command candidate projection inside the Memory Compiler

## Context

ADR 0066 added immutable supported Bash observations as a separate Tier-1 evidence stream. It deliberately did not create a checkpoint Command, Verification, or automatic candidate because Ley did not yet have a safe contract for interpreting a host tool return.

The remaining P0 Memory Compiler gap is automatic candidate formation. Most structured memory shapes require semantic interpretation that Ley cannot safely derive from prompt/response prose with deterministic rules. A supported Bash observation is narrower: when its retained command text is complete, Ley mechanically knows that the host reported an invocation of that exact redacted command. The durable `CommandRecord` shape already permits an unknown exit code.

Ley can therefore form a useful first automatic candidate without inferring task state, command success, test success, verification, or semantic meaning.

## Decision

`ley_session_memory_compile` may expose a separate `automaticCommandCandidates` derived projection for eligible post-checkpoint Bash observations.

A source observation is eligible only when:

- it is already inside the compiler's post-checkpoint supporting-tool window;
- `toolName == Bash`;
- retention is `captured`;
- retained command text exists and is non-empty;
- the command was not truncated at capture; and
- the compiler returned the complete command text without truncating it for the requested character budget.

The candidate does **not** duplicate command text. It references the exact `toe_` row through `sourceRecordId` and declares `commandField: supportingToolEvidence.command`. The referenced row therefore remains the single bounded copy of the exact retained/redacted command.

Each candidate reports:

- source `toe_` observation ID;
- host observation kind (`returned` or `explicit-failure`);
- command field reference;
- explicit `exitCode: null`;
- an outcome-neutral summary;
- `persisted: false`;
- `candidateBindingAllowed: false`;
- `automaticWriteAllowed: false`;
- `verificationClaimed: false`; and
- `outcomeProven: false`.

`returned` still means only that the host emitted its ordinary post-tool event. `explicit-failure` means only that the host emitted its explicit tool-failure event. Neither state provides an exit code, test result, or durable Verification claim.

Result/error retention is not part of Command eligibility. A complete retained command may yield a Command candidate even when result text is absent or truncated, because the candidate makes no claim derived from the result.

## Ineligibility and coverage

Every returned `supportingToolEvidence` row reports one deterministic `automaticCommandCandidateEligibility` value:

- `eligible`;
- `omitted-minimal`;
- `omitted-capacity`;
- `missing-command`;
- `capture-truncated`; or
- `compilation-truncated`.

The pack separately reports:

- total eligible candidate sources in the complete post-checkpoint tool window;
- returned candidates;
- eligible candidate sources omitted by `maxResults`;
- selected eligible sources suppressed because the compiler character budget could not return the complete command; and
- observations ineligible before result-limit selection.

This distinction prevents an older eligible observation omitted by result bounds from being mislabeled as semantically ineligible.

## Authority and persistence boundary

This ADR does not create a durable memory event, checkpoint, Command record, candidate fingerprint, verifier anchor, or writer.

Specifically, in this ADR's original read-only slice:

- `toe_` IDs remain invalid evidence IDs for the generic/typed/batch/rich/composite Memory Transition verifiers/writers;
- `totalUnconsolidatedEvidence`, recovery state, prompt/response `tev_` coverage, and all existing fingerprints are unchanged;
- no session schema version changes; schema v14 remains the newest durable tool-observation schema;
- no checkpoint count or session event is added by compilation;
- no trust, review, egress, filesystem, network, or tool permission is granted; and
- generic checkpoint writes are not an automatic-candidate commit path.

A future durable candidate-bound Command writer requires a separate ADR and must define exact provenance, stale-write semantics, overlap/duplication behavior, and whether any stronger execution evidence can supply an exit code. ADR 0085 later provides only the narrow isolated-source writer: an active session, one exact current `toe_`, no current `tev_` turns, no sibling tool observations, `exitCode: null`, and no outcome claim. Mixed/general Command and Verification recovery remain deferred.

## Privacy and resource behavior

The candidate references the already bounded/redacted command in `supportingToolEvidence`; it does not create a second user-controlled text copy. Raw host tool-call IDs remain opaque Ley `tol_` references and are never exposed.

Minimal and automatic-capacity omissions produce no candidate. Capture- or compiler-truncated commands produce no candidate. Result truncation does not broaden the candidate claim.

## Consequences

- Ley gains its first deterministic automatic Memory Compiler candidate without invoking a model.
- The candidate is mechanically faithful to one narrow fact: an exact retained Bash command was observed by the supported host hook.
- The feature provides a simple baseline against which later model-assisted candidate formation can be evaluated.
- Existing recovery writers and schema meanings remain immutable.
- Command outcome and Verification semantics remain intentionally unsolved.

## Verification

The slice must prove:

- complete retained Bash observations emit one candidate referencing the exact `toe_` row;
- JSON contains explicit `exitCode: null`;
- `returned` and `explicit-failure` do not produce success/failure Verification claims;
- Minimal, capacity-omitted, missing, capture-truncated, and compiler-truncated commands emit no candidate and disclose the reason;
- result truncation does not suppress an otherwise complete command-only candidate;
- result limits distinguish omitted eligible sources from ineligible observations;
- pre-checkpoint observations remain excluded;
- compilation is deterministic and non-mutating;
- raw host tool-call IDs and secret canaries remain absent;
- `toe_` remains rejected by current verifiers/writers;
- prompt/response recovery state, counts, fingerprints, and stale-write behavior are unchanged; and
- the existing P0 crash/recovery scenario exercises the derived candidate without adding a weaker standalone metric.
