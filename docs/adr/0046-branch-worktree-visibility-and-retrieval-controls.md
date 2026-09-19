# ADR 0046: Branch/worktree visibility and retrieval controls

Status: Accepted

## Context

Ley already has a P0 revision engine that compares captured Git object IDs with the current local repository and classifies historical evidence as `current-lineage`, `ancestor`, `merged`, `divergent`, or `unknown`. The Context Compiler uses those classes to prevent divergent decisions from masquerading as current state, and the existing freshness beacon reports bounded Git metadata without claiming live source inspection.

The next P1 roadmap item is expanded branch/worktree UI and retrieval controls. The missing capability is therefore visibility and deliberate retrieval scope, not another revision engine or a branch-specific memory store. Creating a separate Ley project for every branch/worktree would fragment one project's durable experience and would make branch names behave like authority. Branch names and timestamps are not sufficient to establish applicability; object identity and bounded Git ancestry remain the source of the compatibility classification.

## Decision

Ley keeps one project identity across branches/worktrees and exposes the existing revision model through bounded read-time projections and search controls.

- Session Context recomputes every returned checkpoint's `revisionApplicability` from its stored captured revision against current bounded local Git metadata. The top-level session projection also returns `revisionFreshness`.
- Fixed-project Memory Search accepts an optional exact `revisionCompatibility` filter with the five canonical values: `current-lineage`, `ancestor`, `merged`, `divergent`, and `unknown`. Omitting the filter preserves the prior behavior and searches all captured history.
- The filter is applied after bounded candidates have been assigned real revision applicability and before conflict disclosure, semantic reranking, and result packing. Candidates without a provable applicability do not satisfy an exact filter.
- Session-level search rows use the latest structured checkpoint's applicability. This is an orientation shortcut for the session row only; individual historical checkpoint records retain their own applicability.
- Search coverage keeps `collectedCandidates` as the pre-filter bounded candidate count and reports `revisionFilteredCandidates` separately. Deliberate filtering is scope selection, not response truncation.
- CLI exposes the filter as `ley search --revision ...`; MCP exposes it as `ley_search_memory.revisionCompatibility`; the Tauri/Desktop Memory Search exposes the same exact vocabulary as a revision-scope selector.
- The desktop Session Inspector presents the checkpoint's captured revision beside its current applicability and current Git HEAD/branch/tracked-change metadata. It explicitly states that Git metadata was checked, not live file contents.
- Applicability is recomputed at read time. A checkpoint captured on an experimental branch can move from `divergent` to `merged` after Git proves that commit landed, without re-ingesting or rewriting the historical checkpoint.

No new durable store or session schema is introduced. The filter creates no derivative copy, does not mutate memory, and does not create/switch/delete Git branches or worktrees.

## Authority and privacy

Revision compatibility is applicability metadata, not authority.

- `divergent` and `unknown` history remains historical and must not be presented as current state merely because a caller selected it.
- `ancestor` and `merged` mean Git proved a history relationship; they still do not prove current live file contents or increase the trust level of remembered claims.
- `revisionFreshness.liveGitChecked` is a bounded Git metadata/ancestry check only. `liveSourceChecked` remains false until a separate defined live-source inspection actually occurs.
- Search filtering cannot bypass Specification authority, egress policy, trust/freshness rules, or Context Compiler admission. It only narrows which already-eligible fixed-project search candidates are returned.
- The Git beacon returns bounded HEAD/branch/tracked-change counts and compatibility. It does not expose live status paths, file contents, arbitrary worktree paths, or ambient cross-project discovery.

## Consequences

Users and agents can now deliberately inspect one revision class and can see why a historical checkpoint is currently divergent or merged without fragmenting project memory. The default remains backward-compatible: no filter means all captured history.

This first P1 slice is deliberately not a Git/worktree manager. Ley does not create, switch, reconcile, or delete branches/worktrees, automatically consolidate parallel sessions, or infer project-wide truth from the latest session. Cross-project search is unchanged. Future richer worktree controls should continue to reuse the same compatibility source of truth rather than introducing branch-name heuristics or new authority.

The real evaluation reuses Ley's existing divergent-branch scenario: exact divergent search and session inspection must show divergent applicability on main, a current-lineage filter must exclude that experimental history, and after a real `--no-ff` merge the same retained evidence must become `merged` without re-ingestion. Privacy leakage remains zero and every surface keeps `liveSourceChecked: false`.
