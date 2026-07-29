# ADR 0018: Stable lifecycle host adapters

Status: accepted

## Context

MCP gives agents deliberate, typed access to Ley, but MCP does not guarantee that an agent remembers to retrieve context or checkpoint a turn. Codex, Claude Code, and Gemini CLI now expose lifecycle hooks with JSON on standard input and stable fields for session identity, event name, working directory, and final assistant text. Each also exposes a transcript path, but all three explicitly treat transcript storage as a host-owned implementation detail.

A global integration must also be harmless in repositories where the user has not initialized Ley. A hook must not infer a project from a payload-controlled path, scan neighboring directories, emit protocol noise, or turn an agent-authored statement into a trusted lesson.

## Decision

`ley hook --host codex|claude|gemini [project]` is the versioned lifecycle
adapter entry point. Adapter schema version 2 adds prompt-free per-turn
continuity for Claude Code and Gemini CLI while preserving the version 1
session and fallback-checkpoint semantics.

- The CLI resolves only the explicit command path (the host process working directory by default), then requires an existing `.ley` identity, private project-to-vault binding, and captured project snapshot.
- Uninitialized, unbound, and moved-vault projects return the host-valid empty JSON object and do not create, scan, bind, or ingest anything.
- The packaged MCP command stays protocol-valid outside Ley projects by serving an inactive, zero-capability connection. It exposes no tools or resources and performs no discovery or writes; its server instructions explain the explicit setup required.
- A host plus its stable external session ID deterministically maps to one Ley session inside one project. Replayed starts and turn deliveries use deterministic request IDs, so process crashes and hook retries cannot duplicate records.
- `SessionStart` creates or reopens that session and returns the existing bounded project-resume projection as additional context. Stored text is labeled untrusted historical evidence, the captured snapshot is identified, and `liveSourceChecked` remains false.
- Codex and Claude `UserPromptSubmit`, and Gemini `BeforeAgent`, return the exact
  current Ley session and structured-capture contract for each turn without
  storing the raw prompt. The event names and optional correlation fields remain
  host-native.
- Codex and Claude `Stop`, and Gemini `AfterAgent`, append the stable final assistant text as a bounded, redacted fallback checkpoint. User prompts, tool traffic, hidden reasoning, and transcripts are not automatically retained.
- Rich decisions, tasks, problem attempts/outcomes, resolutions, citations, commands, verification, unresolved work, and handoffs remain typed MCP/CLI writes guided by the bundled agent skill.
- Automatic checkpoints do not confirm or promote learnings. MCP can only propose review-required learnings when the integration was started with the independent proposal capability.
- Every host response is one compact JSON value on stdout. Diagnostics use stderr. Stop events return `{}` so they never accidentally continue or block the host.

Codex, Claude Code, and Gemini CLI receive separate installable packages under `integrations/`, but all call the same Rust engine and store the same session semantics. Adapter parity is semantic, not syntactic: host event names and packaging remain native to each host.

## Consequences

- Session two can receive session one's bounded handoff without Ley scraping a transcript or running a cloud service.
- A final assistant message is useful fallback evidence, not a complete account of work. The skill and MCP checkpoint tools remain necessary for high-quality structured memory.
- Host threads remain active until an agent or user explicitly finishes them. This avoids falsely terminating a thread on a per-turn Stop event and permits host-native resume after a crash.
- Changing an integration's stable-field mapping requires an adapter schema/version change and a real multi-turn compatibility exercise.
- Full Evidence permission does not silently enable transcript parsing. A future transcript-capable adapter requires a separately versioned, explicitly acknowledged design.

## Primary sources

- [Codex lifecycle hooks](https://learn.chatgpt.com/docs/hooks)
- [Codex plugin packaging](https://learn.chatgpt.com/docs/build-plugins#bundled-mcp-servers-and-lifecycle-hooks)
- [Claude Code hooks](https://code.claude.com/docs/en/hooks)
- [Claude Code plugins](https://code.claude.com/docs/en/plugins-reference)
- [Gemini CLI hook reference](https://geminicli.com/docs/hooks/reference/)
- [Gemini CLI extension reference](https://geminicli.com/docs/extensions/reference/)
