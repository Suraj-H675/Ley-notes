# Phase 0 frontier-agent benchmark — 2026-09-26

## Purpose

Phase 0 asked a narrow product question before the Ley reset begins:

> Does selective Ley continuity materially help a strong coding agent beyond no history, a normal human
> handoff, or a tiny historical brief — and, if so, which capabilities actually earn their complexity?

This is **not** a claim that the current Ley architecture should survive. The comparison exists so the
reset can preserve outcomes that earned value while deleting the machinery that did not.

## Runner and comparison contract

- Model: `gpt-6-luna`
- Reasoning effort: `xhigh`
- Codex CLI: `0.157.1`
- Runner command identity: `sha256:33dbfbb0887e52231e1338b321f3884c01d24e2a79a671b707e9e2a5584a8a7f`
- External runner sandbox: fresh disposable project, bounded output, least-privilege read-only auth,
  standalone `codex-code-mode-host`, no Ley/private-memory filesystem available to the model.
- Four arms:
  1. no-history baseline;
  2. evaluator-created human `HANDOFF.md`;
  3. fixture-derived minimal continuity brief;
  4. current/full Ley compiled context.
- Representative task families:
  - divergent-branch stale memory;
  - recorded Verification vs narrative claim;
  - crash/missing-final-checkpoint recovery;
  - explicit second-project reference with an unrelated-project leakage canary.
- Final sample: three valid observations for every task/arm cell, 48 valid attempts total.
- Final valid sample contains zero runner failures, runner timeouts, or output-limit failures.

The tasks remain synthetic and intentionally small. Their purpose is controlled ablation of continuity
capabilities, not a general coding benchmark.

## Evaluator corrections discovered during the run

The first raw comparison exposed two evaluator artifacts. They were fixed before the final sample was
accepted:

1. `__pycache__` directories and `.pyc` files produced when an agent ran Python tests were counted as
   unauthorized project changes. The external runner now sets `PYTHONDONTWRITEBYTECODE=1`, and evaluator
   snapshots ignore `__pycache__` trees as non-semantic runtime artifacts.
2. The handoff arm was penalized when the agent edited or removed evaluator-injected `HANDOFF.md` after
   reading it. Task constraints now ignore changes to that benchmark-scaffolding file **only for the
   handoff arm**. Source, tests, config, symlinks, and other filesystem changes remain fully enforced.

Across all collection/replacement runs, 63 attempts were observed. Fifteen were discarded solely because
their task failure was provably caused by one of those evaluator artifacts: 11 Python bytecode-cache cases
and 4 benchmark-handoff-scaffold cases. Exactly 15 fresh observations replaced them, yielding the balanced
48-attempt dataset below. No discarded attempt was reclassified as a pass.

## Corrected results

| Task | No history | `HANDOFF.md` | Minimal brief | Full Ley |
| --- | ---: | ---: | ---: | ---: |
| Divergent stale memory | 3/3 | 3/3 | 3/3 | 3/3 |
| Verified vs claimed | 0/3 | 0/3 | 0/3 | **3/3** |
| Crash / missing checkpoint | **3/3** | 1/3 | 0/3 | **3/3** |
| Explicit second-project reference | 0/3 | 0/3 | 0/3 | **3/3** |
| **Overall** | **6/12 (50%)** | **4/12 (33%)** | **3/12 (25%)** | **12/12 (100%)** |

Observed runtime/context summaries for the same valid attempts:

| Arm | Mean runner time | Median runner time | Mean supplied context characters |
| --- | ---: | ---: | ---: |
| No history | 64.77 s | 59.27 s | 0 |
| `HANDOFF.md` | 74.72 s | 66.73 s | 571.0 |
| Minimal brief | 69.66 s | 69.51 s | 556.2 |
| Full Ley | 38.52 s | 34.31 s | 811.2 |

The runtime difference is observational only. Three repetitions per cell are not enough to claim a causal
latency improvement, and the synthetic tasks are too small for a general performance statement.

## What the evidence earns

### 1. Structured verification evidence is worth preserving

On `verified-timeout-contract`, no-history, handoff, and minimal-brief arms were all 0/3 while full Ley was
3/3. The useful capability is **retrievable structured evidence that distinguishes a recorded verification
result from a narrative claim**. It does not justify treating a stored `passed` status as current truth;
the record remains historical evidence with provenance.

Phase-1 implication: keep a small verification/evidence record in the transactional continuity core and
make provenance visible in briefing/search. Do not preserve the legacy registry/session machinery merely
because it currently supplies this evidence.

### 2. Explicit selected-source context is worth preserving

On the explicit cross-project task, only full Ley passed (3/3). The unrelated registered project remained
outside the selected context. This earns the **user/agent-selected source boundary**, not persistent
Context Mount / Knowledge Scope / Policy Bundle architecture.

The legacy control also needed a 1,000-token context budget where the benchmark normally requested 500
because active-project and mount/provenance packing could starve the selected reference. That overhead is
evidence for replacing persistent mount graphs with a small per-request/per-session selected-source input.

### 3. Crash evidence should survive only in the cheap event model

Full Ley and no-history both passed the crash task 3/3. Human handoff passed 1/3 and the minimal historical
brief 0/3. Therefore this fixture does **not** show that Ley crash context is incrementally necessary over
live-repo reasoning. It does show that incomplete historical summaries can be actively harmful and that
current Ley did not suffer that harm.

Phase-1 implication: keep bounded raw post-checkpoint evidence because an append-only event model gets it
cheaply and it supports provenance/recovery. Do **not** carry forward shape-specific crash/recovery APIs,
v1-v16 session projections, or special recovery state machines without new evidence.

### 4. Divergent-history filtering is a safety guardrail, not a benchmark win here

All four arms passed the corrected divergent task 3/3. The earlier apparent failures in handoff/minimal
were evaluator cache artifacts. This fixture therefore provides no downstream performance advantage for
branch filtering with this pinned model.

Phase-1 implication: retain lightweight Git revision/applicability metadata only if it stays simple and
fail-closed, because stale-branch suppression is a sensible safety boundary and is already deterministically
tested. Do not preserve a large revision-authority subsystem on the strength of this model task.

## Broader reset conclusion

The strongest signal is not “more memory is better.” The no-history baseline outperformed both simple
historical baselines overall, while selective full Ley context performed best. Historical context is useful
when it carries information unavailable from the live repository **and** its provenance/applicability is
clear; otherwise omission can be safer than a plausible stale summary.

The benchmark therefore supports the reset direction already reached independently by the first-principles
audit:

- preserve structured verification/evidence;
- preserve explicit source selection;
- preserve bounded event evidence and cheap revision applicability;
- build task-conditioned, selective briefing that is willing to omit uncertain history;
- replace fragmented JSON machine state with one transactional core;
- remove/defer full repository capture, code graphs, persistent mount/scope/policy hierarchies,
  shape-specific recovery APIs, and other legacy breadth unless later ablations re-earn them.

Current/full Ley going 12/12 on this controlled sample **does not** earn the current architecture. It only
establishes that a smaller replacement must reproduce the useful behaviors above before legacy machinery is
deleted.

## Limits

- One pinned model/runtime (`gpt-6-luna`, `xhigh`, Codex `0.157.1`).
- Three valid repetitions per task/arm cell.
- Four deliberately synthetic high-information tasks, not the full ten-task corpus and not real production
  repositories.
- The runner reports downstream task correctness; it does not prove that a particular context sentence was
  causally read or used.
- No claim is made that full Ley is generally superior to handoffs, no-history coding, or future models.

This evidence is sufficient to end the Phase-0 freeze for the reset decision. Future realistic ablations
should compare the **new minimal implementation** against these same controls rather than use this result as
a permanent exemption from simplification.
