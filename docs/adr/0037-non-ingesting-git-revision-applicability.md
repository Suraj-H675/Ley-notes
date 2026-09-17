# ADR 0037: Non-ingesting Git revision applicability

Status: accepted

## Context

Ley already records captured Git HEAD/branch metadata in immutable project graphs and copies the matching captured revision into structured session checkpoints. That provenance is useful but, by itself, cannot say whether a historical decision still belongs to the user's currently checked-out history. A branch experiment must not silently masquerade as current state merely because its text is relevant or recent.

The freshness check also cannot become a hidden ingestion. Reading arbitrary live source, refreshing artifacts, invoking build tooling, following remotes, or treating timestamps as history would widen Ley's authority and privacy boundary.

## Decision

Read paths may compute one cheap, non-ingesting Git freshness beacon. Ley reuses the bounded local Git-status boundary: no optional Git locks, no filesystem monitor/untracked cache, no untracked files, bounded output, sanitized Git environment, and no network operation. Agent-facing freshness exposes only captured/current HEAD and branch plus a tracked-change count; live status paths are not returned. Git-specific failures degrade to `unknown` and do not make already captured memory unavailable.

Revision compatibility is deterministic and ancestry-based:

- `current-lineage`: captured HEAD exactly equals current HEAD;
- `ancestor`: the captured commit is proven to be an ancestor of current HEAD and the captured/current branch names are equal or unavailable;
- `merged`: the captured commit is proven to be an ancestor of current HEAD and the captured/current branch names differ. This means the captured commit is known to have landed in the current line; it is **not** a claim that Git used a merge commit rather than fast-forward/rebase/history rewriting elsewhere;
- `divergent`: in a non-shallow repository, Git proves the captured commit is not an ancestor of current HEAD;
- `unknown`: HEAD data is missing, an object/command cannot be resolved safely, or a shallow repository cannot prove the negative relationship.

Positive ancestry proof is usable even in a shallow repository; a negative shallow result is not promoted to `divergent` because missing history may explain it. Branch names never substitute for ancestry, and timestamps/semantic similarity never participate.

Fixed-project search attaches checkpoint-specific applicability to revision, decision, and problem candidates. Problems from a divergent branch may remain useful historical experience and therefore remain eligible with the `divergent` label. Divergent revision/decision state is withheld from normal Context Compiler admission and produces an `uncertain-state` premise warning when task-relevant.

Learnings and current captured artifact candidates also receive the latest captured snapshot's applicability as a conservative capture-level ceiling. A learning that is otherwise user-trusted/current is still withheld when the captured snapshot it is being evaluated against is divergent from the checked-out HEAD. This does not claim every origin in a derived learning came from that Git branch; per-origin branch aggregation would require a separate causal/applicability model.

`ley_project_overview`, fixed-project search, and `ley_compile_context` expose `revisionFreshness`. Returned historical items may expose `revisionApplicability`. A changed HEAD/branch or tracked working-tree changes add a `revision-drift` gap. None of these signals reads live file contents, and `liveSourceChecked` therefore remains false.

Mounted project searches use the same source-project revision rules before their lower-precedence results are admitted. No revision result changes mount/write authority.

## Consequences

- Divergent branch decisions cannot become apparent current state merely through retrieval relevance.
- A previously divergent decision can become eligible historical context once Git positively proves that commit landed in the current line; no re-ingestion is required for that ancestry fact.
- Dirty-working-tree and captured-vs-current revision drift becomes visible without pretending Ley inspected changed file contents.
- Shallow/missing/unusual Git states fail soft to `unknown` rather than fabricating divergence or breaking captured-memory reads.
- Branch applicability is independent from trust, Specification authority, semantic similarity, and live-source correctness.
- Remote ancestry, per-origin derived-learning branch aggregation, and branch-specific write scopes are not introduced by this slice.
