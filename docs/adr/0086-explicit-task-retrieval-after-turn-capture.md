# ADR 0086: Explicit task retrieval after turn capture

Status: accepted

## Context

ADR 0057 added initialized-project task-context compilation to Codex and Claude Code
`UserPromptSubmit`. The hook first captured the bounded turn, then synchronously compiled and rendered a
second task-context presentation contract. That guaranteed recall without tool choreography, but also added
prompt-time compiler work, host-specific rendering/omission logic, repeated unsolicited history disclosure,
and a second way to deliver the same active-project Brief semantics.

The post-R3 briefing evaluation on 2026-10-01 ran 12 isolated Codex `0.159.3`, `gpt-6-luna`, `xhigh`
attempts across three representative task families, two repetitions, and the canonical `ley-brief` versus
automatic-first workflows. Both arms passed 5/6 tasks. Automatic context attempted all six hidden oracles and
passed 5/6; explicit Brief passed all five hidden oracles it attempted, while its remaining run was rejected
before oracle because the model modified a disallowed test file. Mean supplied context was 2,217.7 characters
for explicit Brief and 3,607.7 for automatic context. Selected-source Search was held identical between arms
and showed full required-marker coverage with zero forbidden-marker leakage.

This study does not prove causal superiority or production latency/cost savings, and it does not measure the
risk that an agent may fail to call `ley_brief`. It does show no measured task-pass advantage for the extra
initialized automatic-injection machinery in the tested slice.

## Decision

Adapter schema version 6 retires **initialized-project automatic task-history injection**.

For an allowed initialized-project `UserPromptSubmit`:

1. Ley keeps the existing bounded, redacted, retry-idempotent turn capture.
2. Ley returns the current Ley session plus concise capture/checkpoint guidance.
3. Ley does not run the task Context Compiler and does not inject historical task content.
4. The host guidance tells the agent to call `ley_brief` when prior project continuity would materially help,
   with `ley_search` / `ley_evidence` for progressive cited recall.
5. Project-level egress denial still happens before capture and remains a no-op.

`SessionStart` is unchanged by this ADR. Its bounded startup/resume summaries remain a separate automatic
continuity surface because the briefing study supplied them to both arms and therefore did not establish that
removing them is harmless.

Bootstrap Specifications are also unchanged. An uninitialized workspace with explicit Bootstrap
Specification authority may still receive the bounded read-only `# Ley bootstrap task context (automatic)`
block under ADR 0058. That path creates no Ley project/session/capture authority and was not evaluated here.

The normal initialized-project task-context API is therefore `ley_brief`. Request-scoped selected-project
recall remains `ley_search(projectId=...)`, and `ley_evidence` follows project-qualified citations. No session-
persisted selection, combined multi-project Brief, or persistent sharing graph is introduced by this ADR.

## Consequences

- Prompt capture and continuity retrieval become separate responsibilities.
- Default prompt-time disclosure and host-rendering complexity shrink.
- Agents may incur an explicit tool round trip when history is useful; the packaged Skills must teach when to
  retrieve rather than require retrieval on every turn.
- The retired automatic renderer, fallback/query-bound paths, and omission-accounting tests are removed.
- Historical B1 `ley-auto` results remain evidence about the retired behavior and must not be silently
  relabeled as the new capture-only workflow.
- Future automatic injection must re-earn itself with controlled downstream evidence that demonstrates a
  material benefit over explicit Brief retrieval and justifies its extra disclosure/latency/context surface.
