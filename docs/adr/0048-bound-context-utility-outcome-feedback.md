# ADR 0048: Bound context utility outcome feedback

## Status

Accepted.

Later extension: ADR 0073 adds optional exact-version Procedure application claims on top of this
correlation protocol. Ordinary utility events remain schema v5; claim-bearing observations use schema
v15 and keep the original non-causation/non-authority boundary.

Later extension: ADR 0082 adds read-time coverage for bindings that still have no observation.
Terminal non-empty bindings may appear as Memory Health v3 measurement-gap attention, while
`ley_session_get` returns only bounded body-free metadata for recent unobserved bindings. This does
not change durable binding/observation schemas or infer utility.

## Context

The final P1 roadmap item asks for context/memory utility feedback based on downstream outcomes. `LEY.md` is explicit that Ley should measure final task benefit rather than retrieval theater, but the current product has no durable way to say which exact compiled context pack preceded a later structured checkpoint or session result.

That absence creates two bad options:

1. infer utility later from whichever memories happen to look similar to a successful session; or
2. recompile the task after the work and pretend the resulting pack is the one the agent originally received.

Both are unsound. Similarity is not provenance, and ADR 0041 already establishes that an older pack cannot be reconstructed honestly once source, policy, session state, or retrieval inputs change. A session checkpoint itself can change future context compilation, so “recompile after work” can fail even when the caller is retrying the exact same workflow.

Ley also must not turn this feature into hidden analytics, reward-model training, automatic memory promotion, or a mutable retrieval score. A successful task after a context pack is correlation evidence. It does not prove that the model read a particular record, that the record caused the success, or that the same record will be useful under different conditions.

## Decision

Ley records context utility through a two-phase, explicit, session-scoped protocol:

1. **Bind before downstream work.** Immediately after `ley_compile_context`, a write-enabled host may call `ley_context_utility_bind` with the exact `contextPackId`, task, `maxResults`, `maxTokens`, current Ley session, and optimistic `expectedEventCount`.
2. **Observe after typed outcomes exist.** Later, the host may call `ley_context_utility_observe` with the returned immutable `cub_` binding ID and one or more checkpoint/session-finish event IDs that occurred after the binding.

The bind tool recompiles once through the same agent-aware Context Compiler path and requires the logical `contextPackId` to match before persisting anything. It then stores bounded metadata in the existing session ledger:

- binding/event identity and session ordering;
- logical `contextPackId`;
- a redacted/bounded task excerpt;
- artifact and graph snapshot IDs;
- agent egress target when applicable;
- compile result/token limits and estimated token count;
- bounded stable handles for included Specifications, active-project memory, and mounted-reference records;
- explicit omission counts when the included-handle set is truncated.

It does **not** store the compiled context bodies, Specification source bodies, memory excerpts, mounted-reference excerpts, or a reconstructed pack timestamp. The session binding event's own `recordedAtUnixMs` is the truthful time Ley registered the logical pack for later utility measurement. ADR 0041 remains authoritative that a previous compile invocation timestamp is not reconstructable from a later compilation.

The observe tool does not run retrieval. Under the session lock it resolves the supplied `cub_` binding and verifies that every cited event is from the same immutable session history and has a sequence strictly after the binding. Only checkpoint and session-finish events are accepted. Checkpoints must contain at least one typed outcome signal.

Ley deterministically derives bounded outcome counts from those source events, including:

- completed/blocked/cancelled tasks;
- resolved problems;
- helped/no-effect/worsened/unknown attempts;
- passed/failed/skipped/unknown verification results;
- terminal session status;
- unresolved-item count.

The stored observation includes the binding ID, logical pack ID, cited downstream event IDs, and those derived outcome counts. Replay validation recomputes the outcome evidence from the cited immutable events and rejects mismatches.

## Authority and causation boundary

Every binding/observation explicitly preserves the following semantics:

- `contextPackRevalidated: true` means the logical pack ID matched during the pre-work bind.
- `contextUsageProven: false` means Ley did not prove that the downstream model actually attended to or used the supplied context.
- `causalUtilityProven: false` means Ley did not prove that the supplied context caused the downstream outcome.
- `trustChangesApplied: false` means the observation does not promote, confirm, reject, stale, supersede, or otherwise alter memory trust.
- `rankingChangesApplied: false` means the observation does not silently change retrieval ranking.

This first slice is therefore evidence collection, not an automatic reinforcement loop. Future ranking/admission changes must be separately evaluated against simpler baselines, condition applicability, privacy, poisoning, branch/revision compatibility, and negative outcomes before they are allowed to consume this evidence.

## Ordering and idempotency

The binding and observation use session event schema v5. Schema v5 is reserved for `context-utility-bound` and `context-utility-observed` events; older v1-v4 session events remain readable without rewrite.

The binding must occur while the session is active. The observation may be appended after a terminal finish because final task status is itself useful typed outcome evidence. This is a narrow terminal exception; other arbitrary writes remain rejected after finish.

Both tools use deterministic event IDs and caller-stable request IDs. Exact observation retries replay the immutable event normally. Exact binding retries are handled specially: Ley checks for the deterministic binding event first and validates the stored pack ID, bounded/redacted task, expected prior event count, result limit, and token limit. If it already exists, Ley returns the original `cub_` binding without recompiling. This matters because the first binding event itself changes session history and could legitimately change a subsequent Context Compiler result.

A reused request ID with different stable binding parameters fails with the existing session idempotency conflict. New binding requests still re-run the compiler and must match the supplied logical pack ID.

## Privacy, storage, and erasure

This capability creates no telemetry service and no independent analytics database. Utility bindings and observations live in the existing private per-project session ledger and inherit its permissions, integrity checks, event-size limits, lifecycle lock, and explicit erasure behavior.

The session projection exposes only the five most recent observations. Each projected observation exposes at most 24 included-record handles and discloses omitted counts. The durable binding itself caps included handles at 64, while one observation can cite at most 20 downstream events.

`ley_session_get` joins each returned observation back to its immutable binding and returns metadata/outcome counts without copying the context bodies. Absolute project/vault paths are not returned. Explicit session erasure removes the binding and observation with the rest of that session history; there is no second utility-feedback store to purge.

Historical-memory egress policy still applies to `ley_session_get`. The bind path itself uses the same egress-aware compiler before persisting the metadata and a fresh project egress check for the write. A blocked source therefore cannot be smuggled into utility metadata through a pack the target was not allowed to receive.

## Rejected alternatives

### Recompile only after the task

Rejected. The session/checkpoint produced by the task can itself change compilation. A later matching failure says nothing reliable about which pack was originally supplied, and a later match still cannot recover the original invocation timestamp. This contradicts ADR 0041's mismatch honesty.

### Persist every full compiled pack

Rejected for this slice. It would create a new sensitive context-history tier with duplicated source/spec/memory text, new retention/erasure semantics, and broader extraction risk. Stable bounded metadata is sufficient to bind the logical pack.

### Let hosts submit arbitrary “helpful/not helpful” ratings

Rejected as primary evidence. Subjective host/model ratings are easy to game and do not provide the typed downstream task evidence `LEY.md` requires. Future explicit user feedback may be additive, but it must remain distinct from deterministic outcome evidence.

### Automatically boost successful memories

Rejected. Correlation is not causation or applicability. Automatic reinforcement would create a poisoning and feedback-loop surface and could amplify stale or coincidental memories.

## Consequences

Benefits:

- Ley can now attribute later structured outcomes to one exact pre-work logical context pack.
- Feedback is local-first, auditable, replayable, and erased with the session.
- Exact retries remain safe even after session state changes.
- Typed negative and ambiguous outcomes are preserved rather than collapsed into a success score.
- The feature creates evidence for future context-quality evaluation without silently changing authority or ranking.

Tradeoffs:

- hosts must deliberately perform one pre-work bind and one later observation;
- the first slice proves temporal/provenance correlation, not model attention or causation;
- only Ley-structured checkpoint/session-finish outcomes can be linked;
- no aggregate “memory utility score” or automatic optimizer is shipped yet;
- unobserved bindings may remain in session history as evidence that a pack was registered but no eligible downstream outcome was later attached.

That final tradeoff is now explicitly inspectable through ADR 0082. The measurement gap is advisory;
it is not converted into a negative utility label.

## Evaluation

The deterministic `context-memory-utility-feedback` scenario exercises the real MCP path. It requires:

- exact compile → bind → checkpoint → finish → observe ordering;
- exact bind and observe retry safety;
- rejection of a pre-binding session event as downstream utility evidence;
- stable binding/pack IDs in `ley_session_get`;
- typed checkpoint and terminal outcome counts;
- `contextUsageProven: false`, `causalUtilityProven: false`, `trustChangesApplied: false`, and `rankingChangesApplied: false`;
- omission of a canary that was present in the compiled context body from the durable utility projection;
- zero absolute project/vault path leakage.

The P1 capability matrix uses that scenario for adversarial, downstream, privacy, and regression coverage.
