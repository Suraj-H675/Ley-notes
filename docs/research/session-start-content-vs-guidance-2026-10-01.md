# SessionStart content vs guidance-only study — 2026-10-01

Status: completed model-dependent study. Product decision recorded in ADR 0087.

## Question

Should initialized host `SessionStart` automatically inject bounded prior-session / reviewed-learning bodies, or
provide only current-session identity, retrieval/checkpoint guidance, and the existing body-free recovery signal?

## Controlled setup

- Source commit: `43bcdcfe47c5f2ebd749875c54e5f99d9d1a6cbd`.
- C4 harness SHA-256: `c84e5dd1ad7ec3cab320bf9e0e16191dbc0879ca0ee0e95fa5c4a549dcbb082e`.
- Fixture manifest SHA-256: `de952555d120e83ddd57dc7eaa93643e2431413e6c7c625fab107ce4ef0144b7`.
- Codex CLI: `0.159.3`.
- Model: `gpt-6-luna`.
- Reasoning effort: `xhigh`.
- Repetitions: 2.
- Total attempts: 24.
- Operator read-only mount: Codex `auth.json` only.
- Worktree was clean when the study started.

Both arms used the same repository task, hidden oracle, HostHook session-ID algorithm, current prompt capture,
runner/sandbox, and live canonical Ley MCP (`ley_brief`, `ley_search`, `ley_evidence`, `ley_checkpoint`). Ley's
private continuity state remained outside the writable model workspace behind a benchmark-local relay that logged
MCP method/tool names only, never tool arguments. The guidance-only arm was not forced to call Ley.

The six risk classes were:

- ambiguous continuation;
- known failed approach;
- stale same-lineage history;
- divergent history;
- reviewed/trusted learning; and
- interrupted-current-session body-free recovery.

## Results

| Measure | Contentful startup | Guidance-only startup |
| --- | ---: | ---: |
| Task pass rate | 12/12 (100%) | 12/12 (100%) |
| Hidden oracle pass rate | 12/12 (100%) | 12/12 (100%) |
| MCP server failures | 0 | 0 |
| Mean startup context characters | 1,983.8 | 1,188.7 |
| Mean runner seconds | 86.13 | 85.70 |
| Mean retrieval calls | 0.75 | 0.83 |
| Retrieval used rate | 58.3% | 50.0% |
| Mean checkpoint calls | 2.0 | 2.0 |
| Forbidden stale-marker exposure | 4 | 0 |

Guidance-only used ~40.1% less startup context. Its mean runner time was ~0.43 seconds lower (~0.5%), which is
not treated as a meaningful latency advantage in this small study. It averaged only 0.08 more retrieval calls per
attempt. Both arms checkpointed equally.

Per risk class, both arms passed 2/2 tasks and 2/2 hidden oracles. Retrieval behavior varied naturally:

- ambiguous continuation: both arms called Brief in both repetitions;
- known failed approach: contentful called Brief in both repetitions, guidance-only called no retrieval and still
  passed both;
- stale same-lineage history: neither arm retrieved; contentful exposed two stale goal/summary strings per
  repetition (four total) while guidance-only exposed none; both still passed;
- divergent history: neither arm retrieved and both remained free of divergent forbidden markers;
- reviewed learning: both arms retrieved in both repetitions; guidance-only made slightly more Evidence calls;
- body-free recovery: guidance-only retrieved in both repetitions, contentful in one of two; both passed.

## Interpretation

The tested automatic startup bodies produced no measured correctness advantage. Guidance-only preserved every task
and oracle outcome, substantially reduced unconditional context, eliminated the measured stale same-lineage
exposure, and did not create a meaningful aggregate retrieval or latency penalty.

This does not prove that no future workload could benefit from automatic startup history. It does establish that
the current pre-task body injection did not earn its disclosure/steering surface on a deliberately continuity-heavy
slice where it should have had a fair opportunity to help.

## Decision

Adopt guidance-only initialized `SessionStart` (ADR 0087). Keep current-session identity, current prompt capture,
checkpointing, live canonical Ley retrieval tools, and body-free interrupted-session recovery. Retire automatic
prior-session, handoff, and reviewed-learning bodies from initialized host startup.
