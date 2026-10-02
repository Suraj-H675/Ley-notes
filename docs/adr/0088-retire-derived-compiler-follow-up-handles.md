# ADR 0088: Retire derived compiler follow-up handles

Status: accepted

## Context

C3 established that Ley should keep a deterministic compiler/admission layer in front of raw retrieval. It did
not establish that every diagnostic field in `CompiledContextPack` still earned its own product surface.

One remaining field, `followUps`, was introduced when the product exposed a much broader set of granular
inspection tools. The current focused initialized-project MCP surface is only `ley_brief`, `ley_search`,
`ley_evidence`, and `ley_checkpoint`.

The follow-up generator contributed no unique durable state:

- `read-evidence` follow-ups copied `item.citation.artifactPath`, while canonical `ley_evidence` requires the
  richer citation already present on the admitted item;
- `session` follow-ups copied `item.sessionId`, but granular session inspection is not a canonical agent tool;
- `learning` follow-ups copied `item.learningId`, but granular learning inspection is not a canonical agent tool;
- a superseded-learning replacement follow-up copied
  `premiseAdjudication.warnings[].replacementLearningId`, which is already preserved directly on the warning.

The accompanying `reason` values were fixed boilerplate strings. Repository search found no current app/package
consumer of `followUps`; remaining uses were compiler/MCP/evaluator assertions and the retained non-canonical
Context Pack Inspector compatibility manifest.

## Decision

Remove the derived follow-up layer from the current compiler contract:

1. `ContextFollowUpKind`, `ContextFollowUp`, and `CompiledContextPack.followUps` are removed.
2. `ContextCompileCoverage.returnedFollowUps` / `omittedFollowUps` are removed.
3. The compiler no longer generates, token-estimates, budgets, or fits follow-up entries.
4. The retained Context Pack Inspector no longer copies follow-ups and advances its manifest schema from 4 to 5.
5. Premise warnings keep `replacementLearningId`; admitted items keep their full citation, `sessionId`, and
   `learningId` provenance fields. Tests assert those canonical fields directly.

This is a structural redundancy contraction, not a claim from a model-performance ablation. A separate model
study is unnecessary to establish whether duplicated identifiers and static boilerplate contain unique
information; they do not.

## Context pack identity

`contextPackId` hashes the finalized logical compiled pack. Removing a logical response field can therefore
change the ID produced by a newer binary for otherwise unchanged source state. Ley does not emulate the old
follow-up list solely to preserve those hashes.

Historical bindings/observations keep the IDs they originally recorded. The retained Inspector already defines
the safe behavior for a supplied historical ID that no longer matches the current recompilation: report the
mismatch and describe only the current pack; never pretend to reconstruct an older pack that was not persisted.

## Consequences

- Brief has a smaller response and diagnostic budget model.
- Agents use the canonical citation object for `ley_evidence` instead of a weaker path-only pseudo-handle.
- Replacement learning identity remains visible without implying a retired direct-learning inspection action.
- Session/learning IDs remain provenance where present, not promises that a granular canonical agent tool exists.
- Historical ADRs/tests that described follow-up handles remain evidence for the older architecture but are
  superseded for the current focused product by this ADR.
