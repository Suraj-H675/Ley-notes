# ADR 0055: Local meaningful-boundary consolidation inbox

Status: Accepted

## Context

Ley already retains bounded prompt/response evidence and can recover post-checkpoint evidence through the read-only Memory Compiler. That improves crash recovery, but it does not solve a different case: a native Ley session may reach a meaningful pause/end boundary with useful retained evidence that was never distilled into reusable project knowledge.

The product roadmap permits local/background consolidation only after privacy, resource, and review semantics are proven. Automatically running a model over every turn would violate Ley's stronger constraints:

- consolidation should happen at meaningful boundaries rather than every message;
- retained prompt/response text is untrusted evidence, not durable truth;
- transformation cannot increase authority;
- an eligible lifecycle-boundary session must not be reopened or rewritten merely to create a reusable learning;
- explicit historical-host imports must not be swept into a new automatic processing path;
- background/model work must not be implied before scheduling, resource, and privacy semantics exist.

The next useful slice is therefore an explicit **local consolidation inbox**, not automatic semantic consolidation.

## Decision

Ley adds an on-demand read-only projection:

```text
ley consolidation inbox [PROJECT] [--max-items N] [--max-sessions N] [--json]
```

and the fixed-project MCP read tool:

```text
ley_consolidation_inbox
```

The inbox considers only native sessions at a meaningful lifecycle boundary:

- `paused`;
- `completed`;
- `abandoned`.

Active sessions are excluded because their current work should continue through the ordinary session/Memory Compiler workflow. `SessionSourceKind::Import` sessions are also excluded: historical-host import remains an explicit historical-evidence workflow and is not implicitly promoted into local consolidation.

The inbox calls the existing deterministic `compile_session_memory` projection for each bounded eligible session. It does not interpret turn semantics. It reports only stable session metadata, bounded counts, compilation state, and up to 20 retained `tev_` evidence IDs that an explicit later review may inspect or cite.

Turn bodies, absolute project/vault paths, and imported-host session content are not returned by the inbox.

## Advisory action labels

The inbox maps the existing Memory Compiler evidence state to an advisory action:

- `reviewable-evidence` → `propose-review-required-learning`;
- `partial-evidence` → `review-partial-evidence`;
- `metadata-only` → `inspect-metadata-only`;
- `no-unconsolidated-evidence` → no inbox item.

These labels describe the strongest safe next review step. They are not a semantic judgment that a learning exists or that the retained evidence is correct.

Every response explicitly keeps:

- `persisted: false`;
- `modelInvoked: false`;
- `backgroundWorkStarted: false`;
- `destructiveActionsTaken: false`;
- `liveSourceChecked: false`;
- item-level `automaticWriteAllowed: false`;
- item-level `semanticFaithfulnessProven: false`.

The projection is rebuilt on demand and has a deterministic logical fingerprint over its bounded contents/coverage, excluding generation time.

## Direct retained-turn learning evidence

To make the inbox useful without mutating the cited session, a learning proposal may now cite an already-retained captured prompt or assistant-response `tev_` record directly.

This does **not** make the turn trusted. Direct turn evidence:

- must already exist in the cited Ley session;
- must have retained captured text; body-free Minimal/capacity-omitted evidence cannot support a proposal;
- is recorded as `turn-user-prompt` or `turn-assistant-response` evidence;
- contributes a `turn-evidence` origin-lineage source;
- still produces only a tentative, `review-required` learning;
- keeps `automaticAuthorityCeiling: review-required` and `causalCompletenessProven: false`;
- does not append to, reopen, or otherwise mutate the cited session.

User review remains the only route to trusted learning state. A learning proposal does not mark the underlying turn evidence as consumed or processed; the inbox may continue to surface the same evidence until a future, separately designed consolidation-state model exists.

## Bounds and ordering

The first slice is intentionally small:

- default 20 returned items, maximum 50;
- default 30 inspected boundary sessions, maximum 50;
- at most 20 proposal evidence IDs per item;
- paused sessions are considered before completed sessions, then abandoned sessions;
- sessions of the same status are ordered by newest session update, then stable session ID;
- coverage explicitly reports omitted/truncated sessions, items, and evidence handles.

Schema v2 tightens that coverage contract. The earlier
`sessionsWithUnconsolidatedEvidence` name looked project-wide even though Ley can only know that count
for the bounded `sessionsInspected` subset. The field is replaced with
`inspectedSessionsWithUnconsolidatedEvidence`, and `allEligibleSessionsInspected` states whether
`maxSessions` covered the complete eligible paused/completed/abandoned native-session population.
`sessionsOmitted` remains the exact numeric boundary. Ley does not inspect omitted sessions merely to
manufacture a global unconsolidated-evidence count.

These bounds apply before any future model-assisted interpretation and prevent the inbox from becoming a transcript dump or unbounded work queue.

## Egress and authority

The CLI inbox is a local-user view over the bound project memory. The MCP inbox uses the same broad historical-memory egress gate as other historical readers. A cloud-target agent cannot use the new tool to bypass a blocked historical source.

MCP read access to the inbox does not grant learning-write authority. `ley_learning_propose` still requires the independent `--allow-learning-proposals` startup capability. Even when enabled, the result is review-required and cannot confirm, reject, supersede, correct, delete, promote, or otherwise grant trust.

Stored evidence and inbox action labels never grant filesystem, network, tool, review, egress, or execution permission.

## Imported history remains separate

Explicit Codex message-history imports are completed historical snapshots with their own untrusted source boundary. They are deliberately excluded from this inbox even though the lower-level Memory Compiler can inspect them explicitly.

This avoids turning an explicit one-off historical import into an implicit consolidation queue. If a human or agent deliberately inspects imported evidence and later creates a learning through an allowed explicit workflow, the ordinary evidence/review rules still apply; this ADR does not add an automatic imported-history promotion path.

## Security consequences

The main new risks and controls are:

- **Body leakage through a maintenance list:** inbox output contains IDs/counts/metadata, never turn bodies.
- **Authority laundering:** direct `tev_` citations remain untrusted evidence and can create only review-required proposals.
- **Session-history mutation:** inbox and proposal paths do not append to the cited session.
- **Ambient background work:** the inbox starts no model or background task and persists no queue/cache.
- **Historical-import laundering:** import sessions are excluded from inbox selection.
- **Resource flooding:** inspected sessions, returned items, and proposal handles are strictly bounded.

## Deliberately deferred

This slice does not add:

- background or scheduled consolidation;
- automatic model invocation;
- semantic candidate generation from turn bodies;
- automatic checkpoints for paused/completed/abandoned sessions;
- cross-session merge/deduplication or conflict resolution;
- a durable processed/acknowledged inbox state;
- automatic trust or user-review decisions;
- consolidation-attempt outcome history;
- imported-host consolidation;
- a dedicated desktop consolidation manager.

Those capabilities require separate evidence for scheduling, privacy, resource control, semantic faithfulness, concurrency, and review behavior.

## Evaluation

P2 coverage includes deterministic `local-consolidation-inbox` through the real CLI/MCP surfaces.

Passing requires:

- active native sessions are excluded;
- one completed native boundary session is surfaced without copying retained bodies or local paths;
- the logical inbox fingerprint is stable across unchanged rebuilds;
- no persistence, model invocation, background work, destructive action, automatic write, or live-source claim occurs;
- exact retained turn IDs can support a separately authorized review-required learning proposal;
- origin lineage records those exact turn-evidence IDs with an automatic authority ceiling of `review-required`;
- the cited completed session's event count is unchanged after inbox inspection and learning proposal;
- privacy canaries remain absent from all observable inbox/learning/session outputs used by the scenario.

Focused coverage also requires a two-terminal-session / `maxSessions: 1` case to report one inspected
session, one omitted session, `allEligibleSessionsInspected: false`, and exactly one
`inspectedSessionsWithUnconsolidatedEvidence` rather than presenting that bounded count as project-wide.

Focused core, CLI, and MCP tests additionally cover Minimal/body-free evidence, imported-session exclusion, tool bounds, historical egress, direct-turn validation, and cited-session non-mutation.
