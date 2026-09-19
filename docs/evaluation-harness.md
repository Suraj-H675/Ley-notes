# Ley end-to-end evaluation harness

Ley's executable acceptance corpus lives in `eval/fixtures/scenarios.jsonl` and is driven by
`eval/run_eval.py`. The harness exercises the real `ley` CLI, stdio MCP server, and lifecycle-hook
surfaces against isolated temporary projects/vaults. It is intentionally deterministic: unit tests
prove local invariants, while these scenarios prove multi-surface product behavior.

## What the harness measures

The corpus contains both write-time and read/use-time checks. Current metric families include:

- retrieval recall/precision and strict token-budget enforcement;
- selective abstention when no useful memory exists;
- crash recovery, transition binding, idempotency, and origin-lineage preservation;
- human-intent Specification admission, Context Mount isolation, premise resistance, and revision applicability;
- explicit external GitHub connector scope/egress, read-only MCP exposure, stable remove/re-add identity, and non-laundering;
- reusable team/organization Knowledge Scope authority, bounded multi-source retrieval, source-egress inheritance, detach ancestry, and non-laundering;
- parallel-session separation and cross-host durability;
- secret/cross-project/model-egress privacy violation rate;
- deletion fidelity and forgetting-residue rate across Ley-managed raw/derived retrieval surfaces;
- harmless behavior in uninitialized workspaces;
- a deterministic downstream task-evidence contract and a bounded recent-resume baseline comparison;
- Topic Dossier source binding, bounded topic-state coverage, privacy, and post-erasure rebuild behavior.

`privacy_violation_rate` and `forgetting_residue_rate` are lower-is-better metrics. A zero result means
the fixture's canaries were not extractable from the probed Ley-managed surfaces; it is not a claim
that arbitrary undiscovered channels or user-owned external copies do not exist.

## Downstream task contract and baseline

The budgeted-quality scenario compares a 500-token Context Compiler result with a bounded recent-resume
baseline. The compiler passes only when its returned context bodies contain required task evidence,
exclude declared distractors, and remain inside the requested budget. The resume baseline uses a
character budget chosen as an approximate four-characters-per-token equivalent.

This is a deterministic evidence-sufficiency proxy, **not** a score for model reasoning and not a claim
that Ley has beaten an external agent benchmark. Model-dependent downstream benchmarks can be layered
on later, but they must remain reproducible and separately reported.

## P0 capability coverage

A full-corpus run validates a matrix for each P0 capability:

- Context Compiler
- Reliable Memory Compiler
- user-authored Specifications
- Context Mounts
- origin-preserving derivation lineage
- premise/state adjudication
- revision/branch-aware retrieval
- agent-context egress policy

Every capability must retain measured adversarial, downstream, privacy, and regression evidence. The
matrix references concrete scenario/metric pairs. Missing scenarios, misspelled metrics, unsupported
expectations, or failing metric values make a full-corpus run fail.

Focused subset runs validate the matrix schema but intentionally skip full result-value coverage because
not every representative scenario was executed.

## P2 capability coverage

The implemented P2 matrix covers public GitHub issue/PR connectors, commit-pinned text-document connectors, the first bounded multimodal Agent Memory evidence slice, and reusable team/organization Knowledge Scopes. `python eval/run_eval.py --p2-coverage` runs deterministic, network-free scenarios through the real CLI/MCP surfaces. Connector scenarios require authority creation without an implicit provider request, target-specific egress/non-laundering behavior, immutable document provenance, and retained restriction across remove/re-add. The multimodal scenario uses a real binary PNG under Full Evidence, writes a verification citation, mutates the live file afterward, and calls the real `ley_read_media_evidence` MCP route; it must return the exact old captured bytes through a native image block, preserve schema-v6 `mediaType` + non-text `0/0` citation semantics, report no generated description/live-source check, and leak no local path or live mutation canary. The Knowledge Scope scenario creates three real initialized/bound projects, explicitly attaches a two-source team scope, proves unrelated-project isolation and path-free Inspector attribution, applies a source-project `local-model-only` ceiling before cloud retrieval, verifies local-target access, detaches the scope, then requires retained ancestry to withhold derived historical memory and broad historical reads for cloud. Every P2 capability defines adversarial, downstream, privacy, and regression dimensions just like P0/P1. Reusable policy bundles remain a later sub-slice and are not counted as implemented coverage.

The provider network adapter is tested separately so the normal evaluation corpus does not depend on public internet availability. Rust tests verify canonical-target revalidation, issue-vs-PR parsing, pinned-document UTF-8 mapping, response bounds, v1 registry compatibility, and structured-source tamper rejection. Multimodal core/MCP/desktop tests separately cover Full-Evidence-only retention, signature validation, exact snapshot/hash reads, original-vs-derived labeling, serialized output bounds, and historical UI inspection. Knowledge Scope core/CLI tests separately cover owner-private persistence, symlink/corruption failure, bounded membership/attachments/history, immutable/idempotent definitions, unavailable-source states, lock serialization, compiler precedence, host startup withholding, runbook export withholding, and broad MCP historical gating. Real disposable CLI workflows are also used during landing verification to exercise an issue refresh plus this repository's `README.md` at an already-pushed immutable commit, without touching user project/config state. This separation keeps the deterministic corpus reproducible while still testing actual fixed-origin network paths and local binary-evidence behavior before release.

## P1 capability coverage

Implemented P1 capabilities get a separate matrix rather than weakening the completed P0 contract. The
current entries are **Topic Dossiers**, **Current Project State**, **Context Pack Inspector**, **Memory Health / Hygiene**, **Agent Legibility Map**, **Reviewed Runbooks / Skill Export**, **Verification Evidence Links**, **Branch / Worktree Controls**, **Richer Graph Relations**, and **Context / Memory Utility Feedback**. Their matrices require:

- adversarial deletion/forgetting evidence from the session-erasure scenario;
- downstream/regression success from the real authentication dossier journey;
- zero privacy-canary leakage from the returned dossier.
- Current Project State adversarial deletion/forgetting coverage;
- a real active-session project-state journey proving working/open-state, historical-decision, verification, fingerprint, and privacy semantics;
- zero privacy-canary leakage from the returned project-state projection.
- Context Pack Inspector exact-ID reproduction, hidden-body omission, forged-ID mismatch honesty, and zero path leakage through the real MCP path.
- Memory Health advisory/non-destructive semantics, review/uncited/duplicate/session/unresolved/recovery signals, explicit unsupported-metric disclosure, private recovery-body omission, deletion fidelity, and zero path leakage through the real MCP path.
- Agent Legibility coverage for captured architecture, important directory, declared command, observed command, policy, schema/migration, API candidate, observability reference, current plan, and approved Specification metadata; deterministic source binding; explicit no-score semantics; private Specification-body omission; deletion fidelity; and zero local-path leakage through the real MCP path.
- Reviewed Runbook coverage for explicit user-confirmed current procedure/pitfall/convention selection, deterministic runbook/source identity, no authority increase, unrelated-session omission, exact-ID stale-export rejection, explicit host/egress selection, cloud blocking under local-only policy, non-installing Skill output, and zero local-path leakage through the real CLI path.
- Verification Evidence coverage for a real structured test outcome with an immutable captured-artifact citation; propagation through `ley_session_get` and Current Project State; deliberate live-file drift after the checkpoint without hash drift in Ley; explicit `liveSourceChecked: false`; and zero live-canary or local-path leakage. Unknown/uncaptured evidence paths are rejected by focused core coverage.
- Branch / Worktree Controls reuse the real divergent-branch journey. Before merge, exact `divergent` Memory Search must return only divergent-applicable history, `current-lineage` must exclude the experimental evidence, and `ley_session_get` must show the checkpoint and capture freshness as divergent. After a real `--no-ff` merge, the same retained search/session evidence must recompute to `merged` without re-ingestion. Every surface keeps `liveSourceChecked: false`, and local-path privacy remains zero.
- Richer Graph Relations use a real captured implementation/importing-test fixture. Direct context search for an implementation-only marker is the simpler baseline and must not surface the test; `ley_graph_neighbors` must discover exactly the importing test through an incoming deterministic `imports` edge while excluding an unrelated test, and `ley_graph_path` must prove the one-edge test→implementation path with captured citation/provenance. Focused core coverage additionally proves ambiguous dual file matches, package imports, and project-escape paths are not promoted into local deterministic file relations. Graph results keep `liveSourceChecked: false` and leak no machine paths.
- Context / Memory Utility Feedback uses a real write-enabled MCP journey: start a session, compile a task pack, bind the exact logical pack before work, replay that bind idempotently, record typed checkpoint and terminal outcomes, reject a pre-binding session event as downstream evidence, observe the valid outcome pair, replay the observation idempotently, and inspect the joined bounded session projection. The scenario requires typed completed/resolved/helped/passed outcome counts, `contextUsageProven: false`, `causalUtilityProven: false`, no trust/ranking mutation, omission of a canary present in the compiled context body, `liveSourceChecked: false`, and zero absolute-path/privacy-canary leakage.

As later P1 capabilities land, each should add its own adversarial, downstream, privacy, and regression
representatives before the slice is considered complete.

## Running safely

List available scenarios without executing them:

```text
python eval/run_eval.py --list
```

Run one or more exact scenarios:

```text
python eval/run_eval.py --scenario no-useful-memory-honesty
python eval/run_eval.py --scenario crash-before-session-end-resume --scenario session-erasure-derived-residue
```

Run a category:

```text
python eval/run_eval.py --category egress-policy
```

Run only the representative scenarios required by the P0 capability matrix, while still enforcing
every adversarial/downstream/privacy/regression cell:

```text
python eval/run_eval.py --p0-coverage
```

Run only the representative scenarios for currently implemented P1 capabilities while enforcing their
coverage matrix:

```text
python eval/run_eval.py --p1-coverage
```

Run the complete corpus only when the machine can tolerate it:

```text
python eval/run_eval.py
```

The harness is sequential rather than concurrent, but individual scenarios can still compile/start real
Ley processes. For constrained development machines, prefer focused runs while iterating and reserve the
full corpus for a final gate.

## Interpretation rules

- Captured snapshots are not live source; tests must not convert `liveSourceChecked: false` into a freshness claim.
- Similarity/retrieval success is not authority. Scenarios separately assert Specification, trust, premise, revision, and egress gates.
- Privacy canaries should be unique and must not appear in the query/task itself when the assertion inspects returned context.
- Erasure checks probe both public retrieval surfaces and Ley-managed vault files, while honestly excluding independent user-owned copies.
- A green scenario is evidence for the exact fixture contract, not proof of arbitrary real-world correctness.
- General benchmarks are comparative signals only; do not claim benchmark superiority without reproducible runs.
