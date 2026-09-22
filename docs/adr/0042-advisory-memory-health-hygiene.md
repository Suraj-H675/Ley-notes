# ADR 0042: Advisory Memory Health / Hygiene projection

## Status

Accepted.

Later extension: ADR 0073 adds version-bound caller-declared Procedure application observations with
typed downstream outcomes. The stronger health claim discussed here—successful/failed reverification
under faithfully executed and comparable conditions—remains unsupported because those properties are
still not proven.

## Context

Ley's fourth P1 roadmap item is Memory Health / Hygiene. The North Star calls for maintenance signals such as stale or redundant knowledge, uncited claims, conflicting memories, incomplete sessions, unconsolidated evidence, and other signs that retained memory may need review. It also requires hygiene to remain non-destructive: Ley should surface problems and support deliberate maintenance rather than silently deleting or rewriting history.

Several useful signals already exist as typed state in P0/P1 infrastructure: learning trust/freshness, session lifecycle status, latest-checkpoint open work, Memory Compiler recovery backlog, and revision applicability. Other desired signals do **not** yet have enough recorded evidence to support a truthful classification. In particular, Ley does not currently retain typed consolidation-attempt outcomes, downstream retrieval-helpfulness feedback, or a learning-to-verification linkage that could prove an old procedure was or was not successfully reverified.

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

The projection explicitly lists unsupported health ideas rather than inferring them from weak proxies:

- `old-procedure-never-successfully-reverified` is unsupported because Ley does not yet store a typed procedure-learning → verification-outcome relationship;
- `failed-consolidations` is unsupported because Ley does not yet persist consolidation-attempt outcome history;
- `chronically-retrieved-but-unhelpful-memory` is unsupported because Ley does not yet record downstream retrieval utility/helpfulness feedback with sufficient provenance.

## Advisory semantics

Memory Health does **not** compute a single health score. It returns typed signals with stable IDs and `info`, `review`, or `high` severity, plus counts and bounded details.

Severity is triage metadata, not authority. A health signal does not grant permission to delete, suppress, rewrite, confirm, contest, stale, or supersede memory. The report always exposes `destructiveActionsTaken: false`, and the current implementation has no mutation path.

Conflicting same-subject records never use a newest-wins rule. If multiple trusted-current learnings disagree, the report surfaces `conflicting-trusted-claims` and recommends explicit review; it does not select a winner.

## Privacy, egress, and persistence

The report is rebuilt on demand from the fixed project's learning index, structured sessions, Memory Compiler metadata, captured snapshot identity, and bounded local Git freshness metadata. It persists no health cache and keeps `liveSourceChecked: false`.

The MCP reader uses the same historical-memory egress gate as project resume, Topic Dossiers, Current Project State, and broad session/learning readers. Fine-grained historical restrictions therefore cannot be bypassed through health diagnostics.

Recovery backlog signals expose counts/state/checkpoint identity, not post-checkpoint prompt/response bodies. The real evaluation fixture deliberately inserts a private turn-body canary and requires it to remain absent from `ley_memory_health` output.

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
- no automatic consolidation, deletion, archival, or suppression is performed;
- later feedback/evidence systems are required before Ley can truthfully diagnose failed consolidations, chronically unhelpful retrievals, or procedure reverification history.
