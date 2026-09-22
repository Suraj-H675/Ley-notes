# ADR 0073: Version-bound procedure application observations

## Status

Accepted.

## Context

ADR 0048 records bounded correlation between one exact pre-work Context Compiler pack and later typed
checkpoint/session-finish outcomes. That is enough to measure whether supplied context and downstream
results co-occurred, but it cannot answer a narrower learning question: which exact reviewed Procedure
did the caller say it applied when the downstream outcome was recorded?

Inferring that relationship from retrieval after the fact would be unsound. A learning can be corrected,
re-reviewed, become stale, or leave the compiled pack between the work and later inspection. Pack
inclusion alone also does not prove that the model read or followed a Procedure.

LEY.md's evaluation corpus nevertheless requires the system to preserve experience where a
user-approved procedure is applied successfully, unsuccessfully, and under changed conditions. The
record must remain useful without converting correlation into causal proof or an automatic trust/ranking
loop.

## Decision

Ley extends the existing context-utility protocol with an optional, caller-declared
`claimedAppliedLearningIds` field on `ley_context_utility_observe`.

An application claim is accepted only when every supplied learning ID resolves inside the observation's
immutable `cub_` binding to:

- an active-project memory record;
- record kind `learning`;
- the same exact `lrn_` identifier;
- learning kind `procedure`; and
- a positive `learningEventCount` captured from the exact compiled learning version.

To make that check race-free, current-trusted learning search results carry `learningEventCount` from
the learning ledger through Memory Search and the finalized Context Compiler item. Utility binding
persists only the stable learning handle, kind, and event count; it does not copy the learning body.

Ordinary utility bind/observe events without application claims remain session schema v5. A utility
observation with one or more validated Procedure application claims uses session schema v15 and the
derived `session-v15.json` projection. Replay revalidates the claim against the original immutable
binding, so editing an event file cannot manufacture a Procedure application that was not in that bound
pack.

`ley_session_get` exposes the bounded claim IDs beside the same typed downstream outcomes.
`ley_learning_get` derives a bounded reverse view over retained session ledgers:

- session/observation/binding/context-pack IDs;
- the exact bound `learningEventCount`;
- whether that version still matches the learning's current event count;
- the binding's bounded task excerpt;
- cited downstream event IDs and typed outcome counts; and
- aggregate passed/failed/skipped/unknown verification counts.

The reverse view is rebuilt on demand and persisted nowhere else.

## Authority and applicability boundary

The field is deliberately named `claimedAppliedLearningIds`. Ley does not infer that pack inclusion
means use, and it does not infer that a caller's application claim means the Procedure was faithfully
followed.

Every derived learning application row therefore keeps:

- `procedureFollowedProven: false`;
- `conditionApplicabilityProven: false`;
- `contextUsageProven: false`;
- `causalUtilityProven: false`;
- `trustChangesApplied: false`; and
- `rankingChangesApplied: false`.

A passing verification after a claim is historical outcome evidence, not proof that the Procedure
caused the pass. A failing verification does not automatically reject or stale the Procedure. Distinct
task excerpts may preserve changed operating conditions, but Ley does not infer that two conditions are
equivalent or that one historical result predicts another.

Only explicit user review/correction/supersession continues to change learning authority. No utility
score, reinforcement weight, automatic learning promotion, automatic downgrade, or retrieval boost is
introduced.

## Privacy, retention, and erasure

The new durable metadata remains in the existing private session ledger. It stores IDs, the already
bounded task excerpt on the utility binding, exact learning event count, and typed outcome counts. It
does not add learning guidance text, context bodies, prompt/response bodies, or machine paths.

The learning application history is a read-time join over retained sessions and utility bindings. It has
no second analytics store, and session/project erasure therefore removes the source observation normally.
Agent-facing reads stay behind the existing historical-memory egress boundary.

## Rejected alternatives

### Infer procedure use from pack inclusion

Rejected. Inclusion proves only that Ley supplied the record. ADR 0048 deliberately keeps
`contextUsageProven: false`.

### Record only the learning ID

Rejected. A later correction or review would make the same ID refer to a newer learning version. The
bound event count is required to preserve exact-version provenance.

### Treat a pass/fail as automatic learning feedback

Rejected. Outcome correlation does not establish faithful procedure execution, causation, or comparable
conditions. Automatic reinforcement would create a poisoning and stale-guidance feedback loop.

### Add a separate procedure analytics database

Rejected. The existing immutable session ledger already contains the binding and downstream outcome
provenance and inherits current erasure/privacy semantics.

## Evaluation

Focused core/MCP coverage requires:

- exact reviewed Procedure version metadata to survive compile -> bind -> observe;
- non-Procedure and unbound/fake learning claims to fail without appending an event;
- claim-bearing observations to use schema v15 and replay idempotently;
- `ley_learning_get` to expose exact version/task/outcome history with every proof/authority flag false;
- learning event count/state/trust/freshness to remain unchanged by observations; and
- zero project/vault path leakage.

The real-binary `procedure-application-outcome-history` scenario creates one explicitly user-confirmed
Procedure, then records three separate claimed applications under different task/condition excerpts with
verification outcomes pass -> fail -> pass. The final learning view must preserve all three rows,
reject a fake Procedure claim, keep the learning at the same reviewed event count/trust state, and
report zero privacy leakage.
