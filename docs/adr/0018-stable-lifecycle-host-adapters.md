# ADR 0018: Stable lifecycle host adapters

Status: accepted; turn-capture semantics superseded by [ADR 0025](0025-bounded-session-turn-evidence.md); initialized prompt-time context injection from ADR 0057 retired by [ADR 0086](0086-explicit-task-retrieval-after-turn-capture.md); contentful initialized SessionStart retired by [ADR 0087](0087-guidance-only-session-start.md)

## Context

MCP gives agents deliberate, typed access to Ley, but MCP does not guarantee that an agent remembers to retrieve context or checkpoint a turn. Codex and Claude Code expose lifecycle hooks with JSON on standard input and stable fields for session identity, event name, working directory, and final assistant text. Each also exposes a transcript path, but both explicitly treat transcript storage as a host-owned implementation detail.

A global integration must also be harmless in repositories where the user has not initialized Ley. A hook must not infer a project from a payload-controlled path, scan neighboring directories, emit protocol noise, or turn an agent-authored statement into a trusted lesson.

## Decision

`ley hook --host codex|claude [project]` is the versioned lifecycle
adapter entry point. Adapter schema version 7 makes initialized SessionStart guidance-only per ADR 0087 while
preserving stable session identity and body-free recovery signaling. Version 6 kept bounded prompt capture but
retired initialized-project automatic task-history injection per ADR 0086. Version 5 added deterministic host tool evidence; version 4
added the now-retired initialized prompt-time Context Compiler projection from ADR 0057. Version 3 introduced
the bounded turn-evidence semantics in ADR 0025; version 2 used prompt-free turn preparation and fallback
checkpoints.

- The CLI resolves only the explicit command path (the host process working directory by default), then requires an existing `.ley` identity, private project-to-vault binding, and captured project snapshot.
- Uninitialized, unbound, and moved-vault projects return the host-valid empty JSON object and do not create, scan, bind, or ingest anything.
- The packaged MCP command stays protocol-valid outside Ley projects by serving an inactive, zero-capability connection. It exposes no tools or resources and performs no discovery or writes; its server instructions explain the explicit setup required.
- A host plus its stable external session ID deterministically maps to one Ley session inside one project. Replayed starts and turn deliveries use deterministic request IDs, so process crashes and hook retries cannot duplicate records.
- `SessionStart` creates or reopens that session and returns only the stable Ley session ID plus concise retrieval/checkpoint guidance. Prior session bodies, handoffs, learnings, Specifications, mounts/scopes/bundles, and other historical project bodies are not auto-injected. A same-session interrupted window may still produce the bounded body-free recovery count/state signal from ADR 0028.
- Codex and Claude `UserPromptSubmit` return the exact current Ley session and append bounded turn evidence according to capture policy. Structured and Full Evidence retain pattern-redacted prompt bodies; Minimal retains only disclosure events. Initialized projects do not compile or inject task-specific history at prompt time; agents call `ley_brief` deliberately when continuity is materially useful. Bootstrap Specification context remains a separate uninitialized-workspace exception.
- Codex and Claude `Stop` append the paired bounded response as turn evidence, not as a fabricated checkpoint. Tool traffic, hidden reasoning, and transcripts are not automatically retained.
- Rich decisions, tasks, problem attempts/outcomes, resolutions, citations, commands, verification, unresolved work, and handoffs remain typed MCP/CLI writes guided by the bundled agent skill.
- Automatic checkpoints do not confirm or promote learnings. MCP can only propose review-required learnings when the integration was started with the independent proposal capability.
- Every host response is one compact JSON value on stdout. Diagnostics use stderr. Stop events return `{}` so they never accidentally continue or block the host.

Codex and Claude Code receive separate installable packages under `integrations/`, but all call the same Rust engine and store the same session semantics. Adapter parity is semantic, not syntactic: host event names and packaging remain native to each host.

## Consequences

- A new host session does not receive historical project bodies automatically. Prior checkpoints, outcomes, handoffs, learnings, and deeper evidence require deliberate bounded retrieval.
- A final assistant message is useful turn evidence, not a complete account of work. The skill and MCP checkpoint tools remain necessary for high-quality structured memory.
- Host threads remain active until an agent or user explicitly finishes them. This avoids falsely terminating a thread on a per-turn Stop event and permits host-native resume after a crash.
- Changing an integration's stable-field mapping requires an adapter schema/version change and a real multi-turn compatibility exercise.
- Full Evidence permission does not silently enable transcript parsing. A future transcript-capable adapter requires a separately versioned, explicitly acknowledged design.

## Primary sources

- [Codex lifecycle hooks](https://learn.chatgpt.com/docs/hooks)
- [Codex plugin packaging](https://learn.chatgpt.com/docs/build-plugins#bundled-mcp-servers-and-lifecycle-hooks)
- [Claude Code hooks](https://code.claude.com/docs/en/hooks)
- [Claude Code plugins](https://code.claude.com/docs/en/plugins-reference)
