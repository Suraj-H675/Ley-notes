# Read-only interruption-recovery sufficiency study protocol

**Status:** deterministic harness ready; no external-agent result recorded yet.

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

- a consistent task/oracle advantage across multiple fixtures with acceptable context/time cost supports keeping the
  current read-only recovery path;
- no material advantage supports further simplification/retirement review of the retained recovery evidence/API;
  and
- mixed results require inspecting per-task failure modes before changing product behavior.

An advantage does not by itself prove that Memory Compiler is the right long-term transport, because the benchmark
does not exercise that compatibility route directly. Any product decision must preserve the current safety
constraints: historical evidence remains untrusted, `liveSourceChecked` stays false, closed sessions remain
non-writable, and current truth must still be re-established from live repository/runtime evidence.
