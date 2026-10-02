# ADR 0094: Defer the bundled semantic model from the focused product

**Status:** Accepted — 2026-10-02

## Context

Ley's reset requires optional retrieval complexity to earn itself against the simpler lexical baseline.
The current evidence register has no controlled native-state downstream ablation showing that the bundled
Model2Vec model materially improves task success enough to justify its download, maintenance, ranking
variability, and product surface.

The implementation had drifted past that evidence boundary:

- canonical Search always collected a bounded cross-kind candidate set and, when the pinned model happened
  to be installed, semantically reranked that set;
- Desktop Search offered an **Enable semantic search** download/repair flow;
- the CLI exposed `ley semantic status` / `ley semantic install`;
- a dedicated `ley-semantic-installer` crate downloaded roughly 125 MiB of pinned model data from Hugging Face.

This was explicit/local rather than a privacy leak, but it still meant two users with the same durable Ley state
could receive different canonical Search ordering depending on optional cache state that is not part of the
focused release evidence.

ADR 0093 separately retired an old model-enabled evaluator because it measured the legacy artifact semantic
index. That discovery also clarified an important distinction: native artifact retrieval is lexical-only, while
the bounded cross-kind reranker was still model-assisted. The latter therefore needs its own evidence decision
rather than inheriting evidence from the retired artifact-index experiment.

## Decision

Defer the bundled model from the **canonical focused product** until a native-state downstream ablation earns it.

1. Canonical native Search and the transition path's bounded **cross-kind** ranking use the deterministic lexical
   baseline. They do not inspect the model cache and do not emit a missing-model fallback merely because semantic
   reranking is intentionally absent. A pre-cutover transition read may still fall back to the retained legacy
   artifact-search implementation, including its old semantic-index behavior; that is compatibility, not the
   canonical native retrieval contract.
2. Remove the Desktop semantic setup/install card and its Tauri commands/API/types.
3. Remove `ley semantic status` / `ley semantic install` from the CLI.
4. Remove the networked `ley-semantic-installer` crate and its CLI/Desktop dependencies.
5. Keep the existing core semantic/index implementation on the explicit legacy core path for compatibility and
   historical research for now. This ADR does not claim that code has current product value, and it does not
   authorize a broader compatibility purge.

Existing model-cache files are derived public-model data, not Ley project authority. This change neither reads nor
deletes them. Silently deleting an existing cache is unnecessary for correctness and would turn a product-surface
contraction into unrelated filesystem cleanup.

## Why not keep the installer merely because the model is local?

Local inference is a useful privacy property, but it is not evidence of downstream value. Ley's product standard is
that advanced retrieval must improve realistic coding-agent outcomes enough to justify additional concepts,
dependencies, maintenance, and variable ranking behavior. The deterministic lexical baseline already backs the
focused release matrices and remains honest about its limits.

## Compatibility

The intentional user-visible breaks are:

- `ley semantic ...` is no longer a CLI command;
- the Desktop no longer offers model status/download/repair controls;
- installing or retaining the old model cache no longer changes canonical native Search or transition cross-kind
  ordering. Pre-cutover legacy artifact fallback remains compatibility behavior until separately retired.

Durable project/session/learning/evidence state is unchanged. No migration or erasure operation is required.
Legacy core search helpers may still use the model when explicitly exercised by compatibility/research callers.

## Re-entry condition

Reintroduce model-assisted canonical retrieval only after a bounded native-state experiment compares it with the
current lexical baseline under matched authority, revision, egress, context-budget, and task conditions. Measure at
least downstream correctness, stale-memory errors, recall of genuinely relevant paraphrases, context size,
latency/resource cost, and privacy/failure behavior. A synthetic similarity win by itself is insufficient.

## Non-goals

This ADR does not remove `model2vec-rs`, legacy semantic-index parsing/validation, historical semantic tests, or
other compatibility machinery that still has explicit core consumers. Those can be reconsidered only when their
remaining callers and migration value are separately audited.
