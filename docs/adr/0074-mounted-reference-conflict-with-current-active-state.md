# ADR 0074: Mounted reference conflict with current active-project state

- Status: Accepted
- Date: 2026-09-22

## Context

ADR 0034 established task-conditioned Context Mount admission with structural precedence:

1. current approved Specifications;
2. admitted active-project context;
3. mounted reference context.

It already withheld mounted historical guidance that explicitly contradicted admitted human-intent
Specifications while keeping mounted direct evidence visible. However, `referencePrecedence:
active-project-over-mounted-reference` was otherwise only a structural/budget rule. A mounted
historical Decision or Learning could still appear beside a reviewed current active-project Learning
even when the two expressed explicit opposite-polarity claims about the same state.

LEY.md's adversarial corpus explicitly requires reference mounts whose knowledge conflicts with the
active project's current state. Treating recency, semantic similarity, or an arbitrary source as truth
would be unsafe, but ignoring an exact reviewed-current-vs-historical contradiction would weaken the
declared precedence contract.

## Decision

After ordinary mounted relevance/trust admission and after the existing human-intent conflict check,
the Context Compiler performs one additional conservative cross-scope conflict check.

The higher-precedence side is limited to admitted active-project Learning items that are all of:

- `authority: trusted-reviewed-knowledge`;
- `trustedForReuse: true`;
- `state: verified`;
- `trustState: trusted`; and
- `freshness: current`.

The lower-precedence mounted side is limited to historical `Decision` or `Learning` candidates.
Mounted artifact, symbol, dependency, revision, and other direct evidence are not eligible for this
suppression rule.

Conflict uses Ley's existing deterministic explicit-negation test: the two texts must contain opposite
polarity clauses with high normalized term overlap. Semantic similarity, recency, title similarity,
embedding rank, or project order alone cannot create a contradiction.

When a conflict is found:

- the mounted historical candidate is withheld as admission reason `conflicting-memory`;
- `mountedReferenceCoverage.activeProjectConflicts` increments;
- the mounted exclusion carries sorted/deduplicated
  `conflictingActiveProjectEntityIds`; and
- the admitted active-project Learning remains available normally.

The exclusion does not claim that the mounted project's raw evidence is false. Contradictory mounted
direct source evidence remains inspectable with its existing
`untrusted-mounted-project-memory` boundary.

## Authority boundary

This rule does not create a generic newest-state resolver.

- A session Decision in the active project is not automatically current merely because it is newer.
- An unreviewed, stale, contested, rejected, superseded, or source-changed Learning cannot suppress
  mounted history.
- Mounted history cannot create or change active-project trust.
- Direct captured evidence is not hidden because it disagrees with reviewed memory.
- Similarity alone never decides which statement is true.

Specifications remain the highest-precedence human intent and are checked before this rule. When a
mounted candidate conflicts with a Specification, the existing
`specificationIds` diagnostic remains the controlling exclusion path.

## Consequences

- `referencePrecedence: active-project-over-mounted-reference` now has a narrow semantic conflict
  behavior for reviewed/current active knowledge instead of being only a budget/ordering label.
- Agents can distinguish a mounted historical contradiction from an unavailable mount or ordinary
  lower-precedence reference.
- Raw contradictory reference evidence remains available for debugging, migration, and historical
  comparison.
- The rule remains deterministic and inspectable; no embedding-based truth inference is introduced.

## Evaluation

Core regression coverage constructs:

- an active project with a user-confirmed current Constraint:
  `Do not use Redis cache for startup state.`;
- a mounted project with both raw captured source:
  `Use Redis cache for startup state.`; and
- a historical mounted Decision carrying the same old guidance.

Compilation must keep the active reviewed Learning, keep the mounted raw artifact, withhold only the
mounted historical Decision, report `conflicting-memory`, and cite the exact active Learning ID.

The existing real-binary `explicit-project-context-mount` journey is strengthened with the same
conflict while retaining its existing mount authorization, identity-change, source/vault-unavailable,
privacy, and unmount lifecycle checks. P0 Context Mount coverage therefore cannot pass without both
reference availability/isolation and current-state conflict handling.
