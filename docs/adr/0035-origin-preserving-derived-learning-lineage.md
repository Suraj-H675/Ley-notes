# ADR 0035: Origin-preserving derived learning lineage

- Status: Accepted
- Date: 2026-09-17

## Context

P0 #5 requires automatic derivation to preserve the mechanically known durable inputs used to form derived memory. A model-authored learning must not become more authoritative merely because repository evidence was rewritten through a session checkpoint, recovery candidate, correction, or later summary.

Ley already requires every project learning proposal/correction to cite existing structured-session records. Bound recovery checkpoints additionally retain the exact recovery candidate fingerprint and `tev_` evidence set. This provides enough deterministic structure for a first origin-lineage layer without introducing a new summary/dossier store.

Ley cannot mechanically prove every causal influence on an agent-authored claim. The design must therefore distinguish provenance Ley can resolve from a stronger claim of causal completeness.

## Decision

The learning event schema advances to v2. New proposal and correction events persist a bounded immutable `originLineage` alongside their ordinary evidence. Request fingerprints include that lineage, so alteration is detected during event replay.

Lineage records stable origin identities only:

- cited structured session record ID/type/session;
- captured artifact snapshot/path/content hash carried by that session record;
- exact `tev_` records behind a bound recovery checkpoint;
- the bound recovery candidate fingerprint.

Lineage is sorted, unique, bounded to 256 durable sources, and reports omissions. Corrections union their newly resolved origins with prior lineage rather than replacing history. Legacy v1 events remain readable; Ley reconstructs the origins available from their stored evidence but marks that lineage as not mechanically resolved.
Every lineage carries `automaticAuthorityCeiling: review-required`. Automatic derivation therefore cannot self-promote a proposal. Explicit user review may later establish trust, but it does not remove or rewrite the underlying origin chain.

`mechanicallyResolved` means Ley successfully walked every provenance edge represented by the retained structured evidence without omitting a source from the durable bounded lineage. `causalCompletenessProven` remains false: Ley does not claim that the recorded graph captures every influence on a model-authored statement.

Full lineage is returned only by the bounded learning inspector. That response caps disclosed origins at 32 and makes the returned lineage itself incomplete when clipping occurs. Learning lists, hybrid search, the Context Compiler, and mounted-reference results carry only a compact `LearningOriginSummary`, which is accounted for in their existing budgets.

Session erasure continues to remove a learning if any proposal/correction event cites the erased session. Because every lineage addition is derived from such cited events and corrections preserve their earlier events, this remains conservative for the first lineage model.

## Consequences

- Repository/session/recovery evidence cannot disappear merely because it became a reusable learning.
- Bound crash recovery preserves a trace from learning → checkpoint record → recovery candidate → exact turn evidence.
- User confirmation changes trust state, not historical provenance.
- Legacy learnings remain usable without pretending their reconstructed lineage is complete.
- Search/context consumers can inspect provenance health without preloading the complete origin set.

## Deliberately deferred

This slice does not create automatic summaries or dossiers, prove causal completeness, enforce per-origin egress inheritance, add branch/revision applicability, or implement a general derivation-dependency purge graph for future derived tiers. Those remain separate P0/later slices. Origin lineage is the prerequisite metadata they can enforce against.
