# Compiler vs retrieval-only ablation — 2026-10-01

Status: completed bounded model-dependent study. This is evidence for the tested slice, not causal proof or a
claim that every current compiler heuristic is individually necessary.

## Question

Does canonical `ley_brief` still earn its compiler/admission/premise layer over a simpler active-project
`ley_search` context when the downstream coding agent, task fixtures, host startup/session state, and explicit
selected-source Search are otherwise held constant?

The benchmark-only Search arm preserves Search's native trust, revision-applicability, conflict, freshness,
provenance, instruction, and privacy metadata. It does not add compiler premise adjudication, admission
exclusions, or follow-ups. It is not a shipped Ley workflow.

## Runner and sample

- Date: 2026-10-01.
- Codex CLI: `0.159.3`.
- Model: `gpt-6-luna`.
- Reasoning effort: `xhigh`.
- Repetitions: 2.
- Tasks: `divergent-feature-flag-contract`, `verified-timeout-contract`,
  `explicit-reference-cache-contract`.
- Arms: `ley-brief`, benchmark-only `ley-search`.
- Total attempts: 12.
- Runner command hash: `sha256:18dda473a38b2a74bcf18879f215cfe06dbe78a311c1220cd808df8b7348b3f7`.
- The isolated runner received only the explicitly mounted Codex auth file from the host; the benchmark report
  records one read-only mount and no explicitly inherited environment variables.

Crash recovery was deliberately excluded from this comparison because canonical Brief and Search both omit
unconsolidated recovery bodies unless a separate recovery surface is supplied; that would measure a shared
capability limitation rather than the compiler layer cleanly.

## Results

| Measure | `ley_brief` | retrieval-only `ley_search` |
| --- | ---: | ---: |
| Task pass rate | 6/6 (100%) | 4/6 (66.7%) |
| Hidden oracles passed / attempted | 6/6 | 4/4 |
| Hidden oracles skipped | 0 | 2 |
| Mean supplied context characters | 2,257.7 | 3,418.0 |
| Mean runner seconds | 44.68 | 52.25 |
| Mean required-marker coverage | 100% | 100% |
| Forbidden-marker leakage | 0 | 8 total |

Per task:

- `divergent-feature-flag-contract`: Brief 2/2; Search 2/2. Search nevertheless exposed four forbidden
  divergent-history markers in each repetition, while Brief exposed none.
- `verified-timeout-contract`: Brief 2/2; Search 2/2.
- `explicit-reference-cache-contract`: Brief 2/2; Search 0/2. Both Search attempts failed an evaluator
  file-change constraint before the hidden oracle, despite full required-marker coverage and zero forbidden
  selected-source leakage.

Search supplied about 1.514× the context of Brief (~51.4% more) on this slice. The study therefore does not
support replacing Brief with raw Search merely to simplify the implementation.

## Interpretation

The strongest conclusion supported here is **keep a compiler/admission layer in the normal deliberate task
context path**. Raw bounded retrieval retained all fixture-required markers but was less selective, exposed
divergent historical material, used more context, and produced worse downstream task outcomes on the explicit
selected-source fixture.

The divergent Search arm still completed its task in both repetitions, so forbidden-marker presence alone is
not proof that the model followed stale history. Conversely, passing the other Search tasks does not prove the
extra history was harmless. The benchmark reports exposure and downstream outcome separately.

This experiment does **not** establish that every current compiler feature is required. Conflict handling,
premise warnings, individual admission rules, follow-ups, ranking signals, and formatting can still be tested
with narrower ablations. Any deletion should preserve the capabilities that earned evidence: revision-aware
withholding, structured verification/evidence, explicit selected-source context, egress, provenance, and
bounded/honest context.

## Product decision

Do not replace canonical `ley_brief` with retrieval-only `ley_search`. Keep Search as the explicit deeper-history
surface and Brief as the normal task-conditioned context surface. Future compiler simplification must use a
more targeted ablation rather than deleting the whole admission/premise layer.
