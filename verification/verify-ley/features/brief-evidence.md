# Agent brief and evidence

A user supplies the current task to preview bounded context and follows an exact citation to retained historical evidence.

## Sub-features

- `brief-explicit` compiles only after a task and explicit action.
- `brief-target` applies the selected cloud/local egress target.
- `evidence-open` displays original captured evidence with provenance.

## How to get to it (user POV)

- Desktop project `Continue`, `Current task`, `Preview agent brief`.
- Desktop project `Evidence` and its captured-file list; search results also lead to retained records.
- Coding-agent MCP tools `ley_brief`, `ley_search`, and `ley_evidence` are separate integration entry points. They require an initialized captured project and the actual MCP transport; the CLI helper does not exercise them.

## Driving it with control-ley and native accessibility

Preconditions: a healthy native run with an explicitly captured disposable README. CLI `drive` does not cover this recipe.

- Choose `Continue`. Capture the empty task state and require that no brief compiled automatically. Fill `Current task` with `Inspect verificationbeacon continuity`, select `Agent brief egress target` as `Cloud (packaged default)`, and choose `Preview agent brief`.
- Capture the action and compiled output, including task, target, budget, citations, gaps, exclusions, and warnings. A valid empty brief is possible; use fixture evidence relevant to the task before expecting admitted content.
- With the disposable project policy set to `local-model-only`, repeat Cloud and require an explicit policy failure. Select `Local (explicit host assertion)` and repeat; require a policy-allowed response. Restore fixture policy afterward.
- Choose `Evidence`, select captured `README.md`, and capture its historical text and snapshot/path/hash provenance. Change the live disposable README without refreshing capture, then reopen the retained evidence. The historical bytes must remain the captured version. Restore the fixture source and retain before/action/after proof.

## Gotchas

- A browser running the desktop frontend lacks the native IPC boundary and cannot prove this recipe.
- Exact citation reads require the selected project/snapshot/hash and egress checks. Do not construct substitute citations or inspect internal stores as the sole proof.
- MCP discovery and transport need their own proof; successful Desktop compilation alone does not verify a coding host.
