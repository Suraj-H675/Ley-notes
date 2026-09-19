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

## P1 capability coverage

Implemented P1 capabilities get a separate matrix rather than weakening the completed P0 contract. The
current entries are **Topic Dossiers**, **Current Project State**, **Context Pack Inspector**, **Memory Health / Hygiene**, **Agent Legibility Map**, **Reviewed Runbooks / Skill Export**, and **Verification Evidence Links**. Their matrices require:

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
