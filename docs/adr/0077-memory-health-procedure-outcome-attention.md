# ADR 0077: Memory Health Procedure outcome attention

## Status

Accepted.

## Context

ADR 0042 introduced Memory Health as a bounded, on-demand, non-persistent advisory projection. It
deliberately left Procedure reverification unsupported because Ley had no sound evidence that a
Procedure was faithfully followed under comparable conditions.

ADR 0073 later added stronger but still carefully bounded evidence: a caller may explicitly claim that
an exact reviewed active-project Procedure version from one immutable Context Compiler binding was
applied, then bind that claim to typed downstream checkpoint/session outcomes. Ley preserves exact
learning event count and pass/fail/skipped/unknown verification counts while keeping
`procedureFollowedProven`, `conditionApplicabilityProven`, `contextUsageProven`, and
`causalUtilityProven` false.

Those observations are useful maintenance evidence even though they are insufficient to say that a
Procedure "worked", "failed", or was reverified. In particular, a failed typed verification after an
exact-current caller-declared application is a legitimate reason for a human to inspect the historical
run before reusing the Procedure again.

## Decision

Memory Health schema v2 adds one signal kind:

`procedure-application-outcome-attention`

The projection emits one signal per qualifying retained application observation only when:

- the current learning is kind `procedure`, state `verified`, trust `trusted`, and freshness `current`;
- the observation explicitly includes that learning ID in `claimedAppliedLearningIds`;
- the observation resolves to an immutable context-utility binding whose included active-project
  learning record is that same Procedure ID and exact current `learningEventCount`; and
- the observation's typed downstream outcomes contain at least one failed verification.

The signal has `review` severity. It carries exactly the Procedure learning ID, the session ID, and the
application observation ID as related stable IDs. Its bounded detail may report aggregate
passed/failed/skipped/unknown verification counts for that observation.

Pass-only observations produce no Procedure outcome-attention signal. If the Procedure later gains a
new learning event version, an older application observation no longer qualifies as current-version
attention even if the learning remains verified/trusted/current.

The signal remains subject to the normal `maxSessions`, `maxSignals`, and text budgets. It is therefore
an advisory view over the bounded inspected history, not a claim that Ley exhaustively reviewed every
historical application.

## Non-causality and authority boundary

This signal deliberately does **not** mean any of the following:

- the Procedure was faithfully followed;
- the Procedure was applicable under equivalent conditions;
- the supplied context was actually read or used;
- the Procedure caused the downstream result;
- a pass successfully reverified the Procedure;
- a fail proves the Procedure is wrong;
- trust, freshness, review state, or retrieval ranking should change automatically.

Memory Health therefore keeps `old-procedure-never-successfully-reverified` in `unsupportedSignals`.
The new signal is narrower: "this exact current Procedure version has a caller-declared historical
application with a failed typed verification outcome; inspect it."

No learning/session mutation, automatic contest/stale/reject action, utility score, ranking weight, or
new write authority is introduced.

## Privacy, persistence, and erasure

The signal is rebuilt from the same bounded session ledgers Memory Health already reads. It adds no
database, analytics store, background job, or telemetry channel.

It copies no Procedure guidance, task excerpt, context body, prompt/response body, project path, or
vault path. Only stable learning/session/observation IDs and typed outcome counts are included. The
existing historical-memory egress gate applies before the health report is built.

Because the report remains non-persistent, normal session/project erasure removes the source
application observation and a subsequent health rebuild cannot retain a separate copy.

## Identity and schema

`MEMORY_HEALTH_SCHEMA_VERSION` advances from 1 to 2 because the exported signal-kind contract gains a
new serialized value. The existing deterministic `mhs_` identity function includes signal kind and the
related learning/session/record IDs, so each application observation gets a stable distinct signal.
The health fingerprint naturally changes when the returned signal set changes.

## Evaluation

Focused core tests use the real Context Compiler → utility binding → checkpoint → utility observation
path and require:

- one failed exact-current Procedure application to create exactly one review signal;
- a pass-only application to create none;
- a later learning event version to prevent the old observation from being classified as
  current-version attention;
- the report to remain deterministic and non-mutating; and
- `old-procedure-never-successfully-reverified` to remain unsupported.

The existing real-binary `procedure-application-outcome-history` pass → fail → pass scenario additionally
checks the MCP health projection. Exactly the failed run must produce the new signal with exact
learning/session/observation IDs, typed outcome counts, non-causality wording, schema v2, and zero
path/body leakage.

## Rejected alternatives

### Call the signal Procedure failure or failed reverification

Rejected. A correlated downstream failure does not establish faithful execution, condition equivalence,
context use, or causation.

### Aggregate all applications into one Procedure score

Rejected. Different runs may have materially different conditions, and aggregation would make the
evidence less inspectable while encouraging an unsupported quality score.

### Automatically stale or contest the Procedure after a failed outcome

Rejected. That would turn historical correlation into authority and create a self-reinforcing memory
feedback loop. Explicit user review remains the authority boundary.
