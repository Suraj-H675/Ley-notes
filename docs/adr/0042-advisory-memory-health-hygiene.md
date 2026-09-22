# ADR 0042: Advisory Memory Health / Hygiene projection

## Status

Accepted.

Later extension: ADR 0073 adds version-bound caller-declared Procedure application observations with
typed downstream outcomes. The stronger health claim discussed here—successful/failed reverification
under faithfully executed and comparable conditions—remains unsupported because those properties are
still not proven.

Later extension: ADR 0077 upgrades Memory Health to schema v2 with a narrower
`procedure-application-outcome-attention` review signal. It reports a failed typed verification outcome
only when the caller-declared application is bound to the exact current trusted Procedure version. It
does **not** classify the Procedure as failed or successfully/unsuccessfully reverified.

Later extension: ADR 0082 upgrades Memory Health to schema v3 with
`unobserved-context-utility-binding`. A terminal session whose non-empty bound context pack has no
utility observation is surfaced as an incomplete measurement record, not as evidence that the context
was helpful or unhelpful. Recent body-free unobserved binding metadata is inspectable through
`ley_session_get`.

ADR 0082 later extends that same feature to Memory Health schema v4 by adding exact utility-binding
coverage counts for the bounded sessions the report inspected. The v3 signal semantics remain
unchanged.

## Context

Ley's fourth P1 roadmap item is Memory Health / Hygiene. The North Star calls for maintenance signals such as stale or redundant knowledge, uncited claims, conflicting memories, incomplete sessions, unconsolidated evidence, and other signs that retained memory may need review. It also requires hygiene to remain non-destructive: Ley should surface problems and support deliberate maintenance rather than silently deleting or rewriting history.

Several useful signals already exist as typed state in P0/P1 infrastructure: learning trust/freshness,
session lifecycle status, latest-checkpoint open work, Memory Compiler recovery backlog, revision
applicability, and exact-version caller-declared Procedure application observations with typed downstream
outcomes. Other desired signals do **not** yet have enough recorded evidence to support a truthful
classification. In particular, Ley still does not retain typed consolidation-attempt outcomes or
downstream retrieval-helpfulness feedback, and Procedure application observations do not prove faithful
execution, comparable conditions, context use, or causation.

## Decision

Ley implements Memory Health first as an **on-demand, non-persistent, advisory projection** exposed by `memory_health_report` and the read-only MCP tool `ley_memory_health`.

The first slice reports only signals derivable from existing typed state:

- review-required learnings;
- contested learnings;
- stale learnings;
- trusted learnings whose cited source changed;
- uncited learnings;
- exact duplicate active learnings with the same normalized subject and materially equivalent bounded guidance;
- active same-subject learnings with different guidance;
- multiple trusted-current same-subject claims with different guidance, reported as a conflict without choosing a winner;
- active/paused sessions that have not reached a terminal state;
- unresolved latest-checkpoint working state for active/paused sessions;
- post-checkpoint unconsolidated prompt/response evidence reported by the existing Memory Compiler;
- latest-checkpoint session memory whose captured Git revision is proven divergent from the current line.

Schema v2 additionally reports `procedure-application-outcome-attention` once per qualifying retained
observation when all of the following are true: the current learning is a verified/trusted/current
Procedure; the immutable context binding contains that exact Procedure event version; the caller
explicitly claimed that Procedure was applied; and typed downstream evidence contains at least one
failed verification. The signal remains `review` severity and cites only the learning/session/observation
IDs plus bounded outcome counts. Pass-only observations do not create this signal, and an observation
bound to an older Procedure event version stops qualifying once the current learning version changes.

Schema v3 additionally reports `unobserved-context-utility-binding` only for terminal sessions when a
non-empty retained context binding has no observation. The signal cites only the stable session/binding
IDs and bounded included-record counts, copies no task/context body, and disappears after a later valid
observation. Active sessions are not flagged because downstream work may still be in progress.

Schema v4 adds coverage counts for total, uniquely observed, and unobserved context-utility bindings
across the selected sessions. Observation rows are not used as a proxy for observed bindings because
one binding may have multiple observations. `sessionsOmitted` continues to disclose bounded-history
coverage outside that inspected set.

The projection explicitly lists unsupported health ideas rather than inferring them from weak proxies:

- `old-procedure-never-successfully-reverified` remains unsupported because a caller-declared application
  plus a typed verification outcome does not prove the Procedure was faithfully followed, applicable
  under comparable conditions, used from the supplied context, or causally responsible for the result;
- `failed-consolidations` is unsupported because Ley does not yet persist consolidation-attempt outcome history;
- `chronically-retrieved-but-unhelpful-memory` is unsupported because Ley does not yet record downstream retrieval utility/helpfulness feedback with sufficient provenance.

## Advisory semantics

Memory Health does **not** compute a single health score. It returns typed signals with stable IDs and `info`, `review`, or `high` severity, plus counts and bounded details.

Severity is triage metadata, not authority. A health signal does not grant permission to delete, suppress, rewrite, confirm, contest, stale, or supersede memory. The report always exposes `destructiveActionsTaken: false`, and the current implementation has no mutation path.

Conflicting same-subject records never use a newest-wins rule. If multiple trusted-current learnings disagree, the report surfaces `conflicting-trusted-claims` and recommends explicit review; it does not select a winner.

`procedure-application-outcome-attention` is likewise triage, not judgment. A failed downstream
verification is a historical outcome associated with one exact caller-declared Procedure application.
Ley does not infer that the Procedure itself failed, that a pass reverified it, or that a fail should
contest/stale/reject it. `procedureFollowedProven`, condition applicability, context usage, and causal
utility remain unproven under ADR 0073, and Memory Health applies no trust or ranking change.

`unobserved-context-utility-binding` is measurement hygiene only. It does not imply the context was
used, useful, harmful, causally relevant, or that a missing observation should change trust/ranking.

## Privacy, egress, and persistence

The report is rebuilt on demand from the fixed project's learning index, structured sessions, Memory Compiler metadata, captured snapshot identity, and bounded local Git freshness metadata. It persists no health cache and keeps `liveSourceChecked: false`.

The MCP reader uses the same historical-memory egress gate as project resume, Topic Dossiers, Current Project State, and broad session/learning readers. Fine-grained historical restrictions therefore cannot be bypassed through health diagnostics.

Recovery backlog signals expose counts/state/checkpoint identity, not post-checkpoint prompt/response
bodies. Procedure outcome attention copies no task excerpt, Procedure guidance, context body, project
path, or vault path; it uses stable IDs and typed outcome counts already retained in the selected
session ledger. Real evaluation fixtures require those bodies/paths to remain absent from
`ley_memory_health` output.

Terminal unobserved-binding attention likewise copies no task excerpt or included context. The
separate session reader exposes only bounded body-free binding metadata and count/omission coverage.

Because the report is non-persistent, session/project erasure requires no separate health-cache purge transaction. Deletion-fidelity evaluation rebuilds Memory Health after erasure and requires erased canaries to remain absent.

## Bounds and identity

Callers can bound returned signal count, inspected sessions, and aggregate copied signal text. Coverage discloses total sessions/learnings, inspected/omitted sessions, candidate/returned/omitted signals, text usage, and truncation.

Each signal gets a deterministic `mhs_` ID derived from its typed kind and related stable learning/session/record IDs. The report gets a SHA-256 `healthFingerprint` over the returned typed state, coverage, revision freshness, and unsupported-signal disclosure; generation time is excluded from that logical fingerprint.

## Consequences

Benefits:

- makes existing trust/freshness/recovery problems visible in one bounded maintenance view;
- avoids destructive automatic cleanup;
- reuses P0/P1 truth, provenance, recovery, revision, and egress semantics rather than duplicating them;
- makes measurement gaps explicit instead of inventing metrics;
- gives future maintenance UX stable signal IDs and typed recommended actions.

Tradeoffs:

- duplicate detection is intentionally conservative: exact normalized subject plus materially equivalent bounded guidance, not embedding similarity;
- same-subject/different-guidance is a review signal, not proof the claims truly contradict;
- only the bounded inspected session set contributes session/recovery health signals;
- Procedure outcome attention is similarly limited to that bounded inspected session set and therefore
  does not claim a whole-history quality judgment;
- no automatic consolidation, deletion, archival, or suppression is performed;
- later feedback/evidence systems are required before Ley can truthfully diagnose failed consolidations, chronically unhelpful retrievals, or procedure reverification history.
