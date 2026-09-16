# ADR 0030: Candidate-bound unresolved recovery writes

- Status: Accepted
- Date: 2026-09-16

## Context

ADR 0028 exposed post-checkpoint turn evidence after an interruption, and ADR 0029 added a read-only structural verifier. That verifier deliberately does not grant write authority or semantic trust.

A generic checkpoint call after verification is still too weak for recovery: the host could verify one candidate and then submit different checkpoint content. Closing the checkpoint boundary would also hide the recovered evidence window from later compilation, so the write must preserve the exact transition that was reviewed.

Ley cannot safely auto-convert every candidate kind. Tasks, attempts, resolutions, verification records, and similar structures require typed fields that cannot be inferred without inventing status, outcome, root cause, or verification state.

## Decision
Ley adds one narrow write route for a verifier-approved `unresolved` claim. The caller supplies the stable session/request IDs, exact inspected event count, exact verifier `candidateFingerprint`, unresolved subject and statement, and the complete cited `tev_` set.

The commit path first checks whether the deterministic recovery event already exists. An exact retry replays that event idempotently; changed reuse fails. If no event exists, Ley re-runs transition verification and requires `review-required` with no deferred evidence and an identical candidate fingerprint.

Ley derives the checkpoint payload itself: the subject becomes the checkpoint summary and the statement becomes its single unresolved item. The caller cannot attach decisions, tasks, problems, commands, verification records, or touched artifacts to this route.

The immutable recovery event uses schema version 3 and retains the inspected event count, verifier candidate fingerprint, exact sorted evidence IDs, and a separate binding fingerprint over the session, version, candidate fingerprint, evidence set, summary, and unresolved statement. The ordinary public `SessionCheckpoint` projection remains unchanged.

Before append, the writer lock re-checks that the cited evidence IDs equal the complete post-checkpoint recovery window. Ley also recomputes the candidate fingerprint from the normalized checkpoint text; if redaction or sanitization changes the reviewed subject/statement, the write fails before append and the caller must re-formulate and reverify the sanitized candidate. On replay, Ley repeats that chronological equivalence check, recomputes the verifier candidate fingerprint from the stored unresolved subject/statement/evidence set, and validates the outer binding fingerprint. Tampered, omitted, or rebound provenance therefore makes the session store invalid instead of silently accepting a different transition.

The MCP route `ley_session_memory_commit_unresolved` is absent by default and appears only under the existing `--allow-session-writes` startup capability.

## Consequences
- The supported recovery write is structurally bound to the candidate that passed verification and to the full current recovery window.
- Semantic faithfulness and live-source correctness are still not proven by this route; it prevents payload substitution, not false interpretation.
- Generic checkpoints remain useful for ordinary deliberate capture but are not the candidate-bound recovery mechanism.
- Richer candidate kinds remain review-only until Ley has lossless typed bound writers for their required fields.
- Schema-v1 and schema-v2 immutable events remain unchanged and readable; reading old sessions does not rewrite them.
- Exact retries remain idempotent even after the recovery checkpoint has closed the window.
