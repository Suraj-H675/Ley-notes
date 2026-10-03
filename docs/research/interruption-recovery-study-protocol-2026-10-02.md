# Read-only interruption-recovery sufficiency study protocol

**Status:** completed bounded model-dependent study; current read-only interruption evidence retained.

## Question

Does exposing Ley's current bounded retained post-checkpoint interruption evidence materially improve recovery from
an unfinished coding turn, compared with the same Ley continuity context when that evidence is unavailable?

This study is deliberately narrower than both the historical recovery writer stack and the compatibility Memory
Compiler API. The benchmark materializes bounded retained turn evidence through local session reads, then supplies
that evidence to an otherwise isolated external runner. It evaluates the evidence-access question only:

1. retain bounded post-checkpoint prompt/response evidence as untrusted history;
2. expose it read-only;
3. require the agent to inspect current repository/runtime state itself; and
4. use ordinary `ley_checkpoint` only for newly supportable current state.

It does **not** test or justify restoring the retired v1-v16 shape-specific verifier/commit MCP routes.

## Matched arms

Use `eval/run_agent_task_eval.py --variant interruption-recovery`.

- `ley` is the recovery arm. It receives the normal compiled Ley context plus the bounded post-checkpoint crash
  evidence block for a `crashed-active` prior session.
- `ley-no-recovery` is the control. It receives the same project, task, seeded structured prior session, canonical
  Brief/compiler output, model command, sandbox, hidden oracle, and budgets, but the post-checkpoint crash evidence is
  deliberately withheld.

The control asserts that every crash-only benchmark marker is absent. Validation also hashes the complete
non-recovery context and requires that hash to equal the no-recovery arm's context hash, proving the pair differs only
by the appended crash-evidence block. Normal all-arm benchmark runs do not include `ley-no-recovery`; it is
experimental and valid only for `crashed-active` fixtures.

The external runner cannot read Ley's private temporary continuity state. As in the existing agent-task harness,
Ley context is materialized first, the private state is snapshotted and removed before the runner starts, and the
runner receives only the prompt/context plus the repository sandbox.

## Checked-in task set

The first study uses three independent `crash-missing-checkpoint` fixtures:

1. `crash-slug-normalization-contract` — final separator-normalization rule arrives only in the interrupted prompt;
2. `crash-retry-window-contract` — final retry sequence/cap arrives only in the interrupted prompt; and
3. `crash-cache-namespace-contract` — final cache namespace arrives only in the interrupted prompt.

Each task has visible tests that are insufficient to reveal the final contract and a separate hidden oracle. The two
new fixtures intentionally do not require any structured-prior marker to appear in the Brief; this keeps their
matched arms cleaner because the decisive contract exists only in the retained crash evidence.

## Deterministic prerequisite gates

Before any model run:

- `python3 -m unittest -q eval.test_agent_task_eval` must pass;
- `python3 eval/run_agent_task_eval.py --validate --all-tasks` must pass;
- all three crash fixtures must report a non-null `noRecoveryControl`;
- recovery context must contain the crash-only markers;
- no-recovery context must contain none of those markers; and
- recovery/no-recovery ordering must alternate by task/repetition.

These gates prove harness isolation only. They are not evidence that recovery helps a real agent, that the
`ley_session_memory_compile` compatibility API itself is useful, or that an ordinary checkpoint was semantically
correct after recovery.

## First model-dependent run

Use one pinned Codex/model/effort combination and **two repetitions** over all three tasks: 3 tasks x 2 arms x 2
repetitions = 12 external-agent attempts. Alternate arm order through the built-in scheduler.

Example shape, using the same runner contract as the existing frontier-agent studies:

```text
python eval/run_agent_task_eval.py \
  --task crash-slug-normalization-contract \
  --task crash-retry-window-contract \
  --task crash-cache-namespace-contract \
  --variant interruption-recovery \
  --first-variant ley \
  --repetitions 2 \
  --runner-label codex-gpt-6-luna-xhigh \
  --runner-ro-bind "$HOME/.codex/auth.json=/home/runner/.codex/auth.json" \
  --runner-command 'codex exec --ignore-user-config --ignore-rules --ephemeral -s workspace-write -m gpt-6-luna -c model_reasoning_effort="xhigh" -' \
  --audit-dir /tmp/ley-interruption-recovery-audit \
  --output /tmp/ley-interruption-recovery-report.json
```

Record the exact Codex version before the run. If the installed host/model differs from prior studies, report that
fact rather than silently comparing across versions.

## Completed run — 2026-10-02

The first pinned study completed from source commit
`a1b4ab9412b220d95ebf0c2ac71eef6ee993b1aa` with:

- Codex CLI `0.160.0`;
- model `gpt-6-luna`;
- reasoning effort `xhigh`;
- two repetitions across all three crash fixtures and both matched arms (12 total attempts);
- runner command SHA-256
  `sha256:18dda473a38b2a74bcf18879f215cfe06dbe78a311c1220cd808df8b7348b3f7`;
- report SHA-256
  `sha256:3038ae451f317d80b3ac7e7e11a7e7151937df24e931271642320ddf773ba664`; and
- a 126-file local audit bundle retained for raw-output review. The audit bundle is intentionally not checked in.

The runner received one read-only Codex auth-file mount and no explicitly inherited environment variables.

The numeric interpretation thresholds below were written into the operator working tree before the model run was
launched. The retained report does **not** contain a protocol hash or frozen-rule snapshot, so that chronology is
operator-recorded provenance rather than independently attested by the report artifact. Future model-dependent
studies that rely on outcome thresholds should persist a pre-run protocol/rule digest in the report itself.

### Results

| Measure | Recovery `ley` | `ley-no-recovery` control |
| --- | ---: | ---: |
| Task passes | 6/6 | 0/6 |
| Hidden oracles passed / attempted | 6/6 | 0/6 |
| Mean supplied context characters | 1,145.7 | 763.7 |
| Mean estimated context tokens | 286.7 | 191.0 |
| Mean runner seconds | 48.53 | 75.33 |
| Completed process + failed oracle | 0 | 6 |
| File/constraint checks passed | 6/6 | 6/6 |
| Post-check tree stable | 6/6 | 6/6 |

The recovery arm added 382.0 mean context characters and stayed below the existing 500-token budget on every
attempt (maximum estimated context: 310 tokens). Recovery was also faster on this sample by 26.80 mean seconds;
runtime remains a secondary noisy measure and is not treated as a general latency claim.

Per fixture, the result was consistent in both repetitions:

- `crash-slug-normalization-contract`: recovery 2/2, control 0/2;
- `crash-retry-window-contract`: recovery 2/2, control 0/2; and
- `crash-cache-namespace-contract`: recovery 2/2, control 0/2.

The six control processes all exited normally while failing the hidden oracle. Their preserved runner output was
reviewed before interpretation. The failures were not six equivalent semantic false-success claims: several runs
confidently reported plausible changes or visible-test success, while the cache-namespace runs explicitly stated
that the replacement namespace was unavailable. Keep the durable metric as the objective process/oracle mismatch
count rather than upgrading it to a model-deception or false-completion count.

### Interpretation and product decision

This run clears the operator-recorded decision rule: recovery strictly outperformed the control on all three fixtures,
added six successful attempts rather than the required minimum two, regressed on no fixture, stayed within the
context-cost threshold, and introduced no file/constraint failure. The deterministic matched-control gate had already
proved that crash-only markers were absent from the no-recovery context. Because the report does not independently
attest the rule chronology, do not present this as cryptographically proven preregistration.

The tested slice therefore earns **retaining the current bounded read-only post-checkpoint interruption-evidence
path**. No implementation expansion is justified by this result: do not restore structured/semantic recovery writers,
do not infer that the compatibility Memory Compiler transport itself was validated, and do not broaden the product
authority boundary. Historical evidence remains untrusted and current truth still requires live repository/runtime
verification.

This is one pinned model/host, three synthetic crash-contract tasks, and two repetitions per arm. It is strong evidence
for the tested recovery-access question, not a universal claim about all models, tasks, or interruption modes. A
larger study should require a new concrete uncertainty rather than being run automatically.

## Measurements

Primary:

- task pass rate;
- hidden-oracle pass/fail/skip counts; and
- per-task consistency across the two repetitions.

Secondary:

- mean runner seconds;
- mean supplied context characters and recovery-minus-control delta;
- changed-file/allowed-file constraint failures;
- privacy/marker leakage; and
- `completedProcessWithFailedOracleCount`.

`completedProcessWithFailedOracleCount` is only an objective proxy for an agent process that exited normally while
the hidden oracle failed. It must **not** be described as a false completion claim unless the preserved audit output
is separately reviewed and the model actually claimed success. Keep the audit directory for that review; the
checked-in fixtures contain no real secrets, but the general audit-bundle warning still applies.

## Decision rule

Do not restore structured/semantic recovery writers merely because the evidence arm wins one task. Treat the study
as evidence for whether retaining **some bounded read-only interruption-evidence access** is worthwhile:

- a consistent advantage means the recovery arm must strictly outperform the control on task + hidden-oracle success
  on at least **two of the three fixtures**, must not underperform the control on any fixture, and must produce at
  least **two additional successful attempts out of six** overall;
- acceptable context cost means the recovery block must stay within the existing 500-token task-context budget,
  cause no truncation/budget failure, and add no more than **500 mean characters** versus the matched control. The
  deterministic pre-run validation observed per-task deltas of 344-418 characters, so this threshold is fixed before
  external-agent outcomes are known;
- runtime is secondary because six attempts per arm are noisy. Treat it as a material cost signal only if recovery is
  both more than **25% slower on mean runner time** and more than **15 seconds slower in absolute mean time**;
- either arm producing a privacy/marker-leakage or changed-file/allowed-file constraint regression blocks a positive
  product conclusion until that failure is understood;
- satisfying the multi-fixture success threshold with acceptable cost supports keeping the current read-only recovery
  path;
- no material advantage supports further simplification/retirement review of the retained recovery evidence/API;
  and
- any other outcome is mixed and requires per-task failure analysis before changing product behavior.

An advantage does not by itself prove that Memory Compiler is the right long-term transport, because the benchmark
does not exercise that compatibility route directly. Any product decision must preserve the current safety
constraints: historical evidence remains untrusted, `liveSourceChecked` stays false, closed sessions remain
non-writable, and current truth must still be re-established from live repository/runtime evidence.
