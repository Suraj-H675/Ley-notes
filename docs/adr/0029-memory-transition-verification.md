# ADR 0029: Deterministic memory transition verification

Status: accepted

## Context

ADR 0028 made missed-checkpoint evidence recoverable without automatically turning prompt/response text into durable structure. The remaining risk is the transition itself: an agent can inspect valid recovery evidence and still omit important records, cite evidence outside the current window, duplicate existing memory, revise an existing subject without noticing, or write fluent unsupported claims.

`LEY.md` section 15.2 requires memory transitions to be verifiable and reversible across coverage, preservation, and faithfulness. A deterministic first verifier can prove some structural properties, but it cannot prove that natural-language candidate claims are semantically true or that the live workspace still matches captured history.

## Decision

Ley adds `verify_memory_transition` and read-only MCP tool `ley_session_memory_verify`.

The caller supplies the stable session ID, the exact `sessionEventCount` previously inspected, bounded candidate claims, and any recovery evidence records intentionally deferred. Each claim has a typed candidate kind, subject, statement, and exact `tev_` evidence record IDs.

Ley reconstructs the current post-checkpoint recovery window directly from immutable session events. Every record in that window must be either cited by at least one candidate claim or explicitly deferred. Unknown/pre-checkpoint IDs, duplicate references, records both cited and deferred, or uncovered current evidence force `needs-revision`. If the inspected event count is no longer current, the result is `stale`.
Evidence-anchor quality is reported separately. Metadata-only capture cannot support a claim. Truncated/partially retained evidence is disclosed as partial. Retained bounded bodies may make a claim inspectable, but inspectable evidence still does not prove the interpretation.

The verifier also projects existing structured session memory and reports deterministic overlap. Exact duplicates and same-subject/different-content candidates require revision instead of silent append. Existing memory is never modified by verification, so preservation is guaranteed for this read-only step.

A fully accounted candidate with no deferred recovery evidence reaches only `review-required`. Any structurally valid transition that intentionally defers one or more current recovery records reaches `deferred`, even when it also contains claims, because a checkpoint would otherwise advance the global recovery boundary past evidence that was supposed to remain unconsolidated. `review-required` means structural coverage and overlap checks passed; it explicitly does **not** mean semantic faithfulness, live-source correctness, user approval, trusted knowledge, or a write authorization. The result carries `semanticFaithfulnessProven: false`, `liveSourceChecked: false`, an untrusted source boundary, and a deterministic candidate fingerprint.

Any later recovery checkpoint still uses the inspected `sessionEventCount` as `expectedEventCount`, preserving the ADR 0028 stale-write guard. Verification does not itself create checkpoints or learnings.

## Consequences

- Memory Compiler recovery now has an explicit immutable-evidence → candidate → verification boundary before durable reconstruction.
- Coverage is deterministic: evidence cannot disappear from the transition unless it is deliberately deferred.
- Duplicate/revision pressure is visible before append rather than relying on last-writer behavior.
- Authority does not escalate: candidate text and turn bodies remain untrusted and review-required.
- The verifier remains read-only and advisory. ADR 0030 adds a separate narrow bound writer for exactly one unresolved recovery claim; richer candidate kinds remain review-only rather than being coerced into incomplete checkpoint records. ADR 0060 later adds a separate lossless bound writer for exactly one minimal Decision or Problem. ADR 0061 adds a separate typed-v2 verifier plus schema-v9 bound writer for exactly one Task so Task status participates in identity rather than being invented or discarded. ADR 0062 extends that typed-v2 verifier to exactly one Plan and adds a schema-v10 bound Plan writer. ADR 0063 then adds a separate batch verifier/schema-v11 writer that can preserve 2–50 already-supported unresolved/Decision/Problem/Task/Plan candidates atomically from one complete recovery window without changing the earlier fingerprint domains. ADR 0064 later adds a separate rich-Problem verifier/schema-v12 writer for one new debugging episode with ordered Attempts and optional Resolution, and ADR 0065 adds a separate schema-v13 composite route for that rich episode plus minimal siblings from the same window without broadening v11/v12. Standalone mutation of existing Problems and remaining unsupported kinds stay review-only.
- Semantic faithfulness remains an explicit gap. Later learned/agent verification may assist, but it must preserve provenance, reversibility, and human-review semantics rather than replacing these deterministic checks.
- This slice still does not automatically generate candidate claims, consolidate across sessions, resolve semantic conflicts, adjudicate live revision/branch state, or promote reusable learning.
