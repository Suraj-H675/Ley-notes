# ADR 0084: Memory Health Procedure application coverage disclosure

Status: Accepted

## Context

ADR 0077 added review-only attention for an exact-current Procedure application observation whose
typed downstream verification contains a failure. That signal intentionally does not classify a
Procedure as successfully or unsuccessfully reverified.

Memory Health is bounded by `maxSessions`. Before this change, its coverage reported the number of
sessions and context-utility bindings inspected, but not the number of caller-declared Procedure
application claims examined by the Procedure outcome logic. A report with no Procedure attention could
therefore be mistaken for exhaustive Procedure history even when older sessions were outside the
inspection bound or when retained claims belonged to older Procedure versions.

## Decision

Memory Health schema v5 adds two coverage fields:

- `procedureApplicationClaimsInspected`: every retained `claimedAppliedLearningIds` entry encountered
  in the bounded inspected sessions;
- `exactCurrentProcedureApplicationClaimsInspected`: the subset whose immutable binding contains the
  same active-project Procedure ID and exact current learning `eventCount`.

The exact-current predicate is shared with `procedure-application-outcome-attention`; coverage and
signal generation therefore cannot silently drift to different definitions.

These are inspection counts only. They do not mean a Procedure was followed, applicable, helpful,
harmful, successfully reverified, or causally responsible for any outcome. A pass-only claim still
contributes to both coverage counts while producing no outcome-attention signal. If the learning later
advances to another event version, the historical claim remains counted as inspected but no longer
counts as exact-current.

The existing `sessionsInspected` / `sessionsOmitted` fields remain the whole-history boundary. Coverage
counts describe only the sessions actually inspected by this report.

## Consequences

- a clean Procedure-health report is easier to interpret without pretending it examined omitted
  sessions;
- version drift is visible numerically without turning old observations into negative health labels;
- `old-procedure-never-successfully-reverified` remains unsupported because typed outcomes still do
  not prove faithful execution or comparable conditions;
- no new persistence, telemetry, score, trust mutation, or automatic retrieval suppression is added.

## Evaluation

Core regressions require failed and pass-only exact-current claims to report coverage `1 / 1`, while a
later learning-version change keeps total inspected claims at `1` and drops exact-current coverage to
`0`. The real pass → fail → pass Procedure scenario requires Memory Health schema v5 and coverage
`3 / 3`, with the same single failed-run review signal and zero privacy leakage.
