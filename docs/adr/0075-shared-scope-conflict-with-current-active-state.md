# ADR 0075: Shared Knowledge Scope conflict with current active-project state

- Status: Accepted
- Date: 2026-09-22

## Context

ADR 0052 introduced reusable team/organization Knowledge Scopes as read-only project-reference
authority below both the active project and explicit Context Mounts. ADR 0074 later gave explicit
Context Mounts a narrow semantic form of that precedence: historical mounted Decision/Learning guidance
that explicitly contradicts reviewed/current active-project knowledge is withheld, while mounted direct
evidence remains visible.

Without the same rule, an even lower-precedence attached shared scope could still inject the exact
historical guidance that an explicit mount would suppress. That would make
`sharedKnowledgePrecedence: explicit-mount-over-shared-knowledge` structurally correct but
semantically inconsistent with `active-project-over-mounted-reference`.

## Decision

Shared Knowledge Scope admission reuses ADR 0074's exact active-state conflict predicate.

After normal shared-source trust/relevance admission and after the existing human-intent Specification
conflict check, a shared candidate is withheld only when:

- the shared candidate is historical `Decision` or `Learning` guidance;
- an admitted active-project Learning is `verified`, `trusted`, `current`,
  `trustedForReuse: true`, and `authority: trusted-reviewed-knowledge`; and
- the two claims satisfy Ley's deterministic explicit opposite-polarity/high-term-overlap test.

Semantic similarity, recency, source ordering, scope ordering, or title similarity alone cannot create
the conflict.

When the condition holds:

- the shared historical candidate is withheld as `conflicting-memory`;
- `sharedKnowledgeCoverage.activeProjectConflicts` increments;
- the exclusion carries sorted/deduplicated `conflictingActiveProjectEntityIds`; and
- the active-project reviewed Learning remains available.

Shared direct artifact/symbol/dependency/revision evidence is not subject to this suppression rule and
remains inspectable beneath `untrusted-shared-project-memory`.

## Precedence and authority boundary

The full relevant reference ordering is:

1. current approved Specifications;
2. reviewed/current active-project knowledge and other admitted active-project context;
3. explicit Context Mount references;
4. attached team/organization Knowledge Scope references.

Specifications still win first through the existing `contradicts-human-intent` path. ADR 0074's
explicit-mount behavior is unchanged. This ADR only prevents the lower shared-reference layer from
reintroducing historical guidance that conflicts with reviewed current active state.

The rule does not:

- infer project truth from arbitrary active sessions;
- let an unreviewed/stale/contested/rejected/superseded Learning suppress shared history;
- suppress contradictory raw source evidence;
- mutate a source project's sessions or learnings;
- make shared content policy/Specification authority; or
- use embeddings as a truth/conflict classifier.

## Diagnostics and budgeting

`SharedKnowledgeExclusion` gains bounded
`conflictingActiveProjectEntityIds`. `SharedKnowledgeCoverage` gains
`activeProjectConflicts`. The extra stable IDs participate in the existing diagnostic token budget,
so strict compiler budgets cannot make the new explanation unbounded.

Context Pack Inspector inherits these shared exclusions/coverage from the finalized compiler pack; no
new tool or persisted conflict store is introduced.

## Evaluation

Focused core coverage constructs:

- an active project with user-confirmed current Constraint
  `Do not use Redis cache for startup state.`;
- one attached team scope source with raw captured evidence
  `Use Redis cache for startup state.`; and
- one historical Decision in that source with the same old guidance.

Compilation must retain the active reviewed Learning, retain the shared raw artifact, withhold only the
historical shared Decision, report `conflicting-memory`, and cite the exact active Learning ID.

The existing real-binary `team-organization-knowledge-scope` P2 scenario is strengthened with the same
conflict while keeping its multi-source retrieval, unrelated-project isolation, Inspector attribution,
source-project egress, local-target access, detach, retained ancestry, historical-reader withholding,
and privacy checks. P2 Knowledge Scope coverage therefore cannot pass while this lower-precedence
historical contradiction is admitted.
