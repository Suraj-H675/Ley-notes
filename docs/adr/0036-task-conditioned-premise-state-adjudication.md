# ADR 0036: Task-conditioned premise/state adjudication

Status: accepted

## Context

Ley's fixed-project hybrid search can nominate captured evidence, structured session history, and learnings, while the Context Compiler already separates relevance from authority and withholds stale, contested, rejected, superseded, or materially conflicting memory from normal prompt assembly. That prevents unsafe reuse, but it does not yet tell an agent when the task itself appears to rely on one of those invalidated states.

The North Star requires premise resistance: when a task overlaps an explicitly superseded or rejected state, Ley should surface that mismatch before context assembly rather than quietly returning only whatever else still matches. The same applies to contested/conflicting state and stale state. At the same time, recency, semantic similarity, and title overlap are not sufficient evidence to decide which state is current.

## Decision

The Context Compiler adds a bounded `premiseAdjudication` result computed from the same task-relevant fixed-project candidates used for admission. Lower-level hybrid search remains a nomination/diagnostic layer and does not choose truth.

The first adjudication slice recognizes only states already represented by explicit durable semantics:

- a task-relevant explicitly superseded learning produces `obsolete-assumption` and retains the stable replacement learning ID when one exists;
- a task-relevant explicitly rejected learning produces `obsolete-assumption`;
- a task-relevant contested learning or material durable content disagreement produces `conflicting-state`;
- a task-relevant stale/source-changed learning produces `uncertain-state`;
- otherwise the compiler reports `no-detected-mismatch`.

Premise relevance uses the same admission relevance rule as normal context: lexical relevance qualifies, while semantic-only relevance must meet the compiler's pinned similarity threshold. A weak semantic neighbor therefore cannot manufacture a premise warning.

The compiler does **not** infer supersession from timestamps, normalized titles, last-writer order, or embedding rank. Session decisions currently have no typed supersession field, so this slice does not pretend that a newer decision is automatically authoritative. Explicit learning supersession remains the only replacement relation used here until a later evidence-backed state model introduces additional typed relations.

Warnings preserve stable entity/learning identifiers, a bounded message, and an optional replacement learning ID. A designated replacement receives a normal `Learning` follow-up handle for explicit inspection. The replacement is not automatically promoted merely because it is named by a supersession edge; its own trust/freshness/admission state is evaluated normally.

Premise warnings are fitted before ordinary conflict/gap diagnostics so a tight token budget cannot silently erase the reason an otherwise relevant historical state was withheld. Omission counts remain explicit. The compiler continues to report `liveSourceChecked: false`: an adjudicated replacement is durable project memory, not proof that the current working tree implements it.

Mounted reference projects remain lower-precedence reference evidence. This first premise engine adjudicates the active project's task-relevant state; it does not let a mounted project's historical state redefine the active project's current state.

## Consequences

- Agents can distinguish “no useful memory” from “your task appears to assume an explicitly invalidated state.”
- Explicit user-reviewed supersession becomes useful at task time without deleting the historical claim.
- Similarity, recency, and repeated agent restatement still cannot manufacture current-state authority.
- Replacement follow-ups support progressive disclosure without preloading the replacement body solely because an older claim matched.
- Branch/revision compatibility, live freshness beacons, and additional typed state/supersession relations remain separate roadmap work rather than being guessed by this slice.
