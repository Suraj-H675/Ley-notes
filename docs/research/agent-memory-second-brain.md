# Agent memory as a local second brain

Status: approved architecture direction. The four product decisions were confirmed on 2026-07-16.

## Executive decision

Ley should become one local knowledge system with two authors:

1. humans write durable notes, properties, links, canvases, and collections;
2. agents append evidence-backed project and session memory through a local integration.

In simple product language: **Ley remembers what happened across a user's agent sessions and gives future agents the relevant evidence, so they can continue the work instead of starting over or guessing.** Ley should reduce forgetting and unsupported hallucination through durable capture, citations, contradiction checks, freshness, and explicit uncertainty. It must not promise that any LLM can literally never forget or hallucinate; unsupported claims should instead be detectable and visibly unsupported.

The correct integration is not “MCP or a plugin.” It is a layered package:

- a local Ley engine and CLI own filesystem storage, indexing, locking, redaction, and retrieval;
- a local stdio MCP server provides the portable tools and resources used by many agents;
- a small cross-agent skill teaches agents when and how to use those tools;
- versioned host adapters use lifecycle hooks where available to start, checkpoint, end, and resume sessions;
- manual `ley session checkpoint` and `ley session finish` commands cover hosts without usable hooks.

MCP is the stable waist. Hooks are optional automation. Neither is allowed to scan or capture a project until the user explicitly initializes and approves it.

## What “second brain” means here

Obsidian works because the durable substrate is user-owned files, not the graph visualization. Notes remain readable Markdown; `[[links]]`, backlinks, properties, search, Bases, Canvas, and graph views are different projections of the same knowledge. The graph emerges from ordinary work instead of becoming a separate database the user must maintain.

Ley should preserve that model for human knowledge and extend it for agent work:

- **Capture:** fast notes plus structured session events.
- **Connect:** explicit links, deterministic code relations, cited agent relations, and temporal edges.
- **Distill:** reviewed decisions, project briefs, outcomes, and learnings.
- **Express/reuse:** compact context packs supplied to future agents and linked back to source evidence.

This resembles the capture-organize-distill-express loop often used in second-brain practice, but Ley should not impose one folder methodology. Links, typed properties, search, collections, and project/session views should remain composable.

## Findings from Graphify

The checked-in reference at `ref/graphify` is unusually relevant. Its useful ideas are:

- deterministic local tree-sitter extraction for code;
- explicit `EXTRACTED`, `INFERRED`, and `AMBIGUOUS` provenance on graph edges;
- community detection and paths as retrieval aids rather than decorative graph features;
- token-budgeted graph queries instead of stuffing an entire repository into context;
- multi-project MCP queries with an explicit project path;
- persistent outcome records (`useful`, `dead_end`, `corrected`);
- a deterministic, time-decayed reflection layer that distinguishes tentative, preferred, contested, dead-end, and corrected knowledge;
- stale-source detection so old advice is re-verified after code changes;
- host-specific skills/hooks plus an always-on instruction fallback.

Ley should not copy these choices blindly:

- Graphify output is primarily a generated project artifact; Ley needs a long-lived, user-editable knowledge product.
- Its benchmark claims are useful hypotheses, not independent proof. Ley needs its own reproducible evals.
- A pure graph is not sufficient. Dense/lexical retrieval, source documents, temporal facts, and structured session records solve different queries.
- Semantic extraction through a cloud agent conflicts with a strict local data plane unless the user knowingly chooses that provider interaction.
- Absolute project paths in MCP calls are convenient but should resolve through an approved project registry to avoid accidental cross-project access.

## Memory model

Ley should keep four layers separate so that one noisy transcript cannot become “truth.”

### 1. Source record

Immutable or append-only evidence:

- project file snapshots and content hashes;
- deterministic symbols/imports/calls/config/dependencies;
- session lifecycle events;
- user prompts and agent responses only at the selected capture level;
- tool calls, touched files, commands, tests, commits, and outcomes;
- user edits, corrections, approvals, and rejections.

### 2. Episodic memory

What happened in one project session:

- goal and scope;
- plan and status transitions;
- decisions and alternatives;
- artifacts read/changed/created;
- commands and verification evidence;
- unresolved tasks and handoff state;
- problems, attempts, outcomes, corrections, and learnings;
- links to commits, files, symbols, notes, and other sessions.

### 3. Semantic project memory

Slow-changing knowledge distilled across evidence:

- architecture and domain concepts;
- current constraints and conventions;
- component ownership and relationships;
- accepted decisions and superseded decisions;
- active goals and project state;
- facts with valid-from/valid-to and learned-at timestamps.

### 4. Procedural memory

How future work should be performed:

- verified commands and workflows;
- known failure modes and their conditions;
- preferred diagnostic paths;
- security and repository conventions;
- lessons that have enough corroboration or explicit user approval.

Only small, trusted summaries belong in always-on context. The rest must be retrieved on demand.

## Problems, attempts, outcomes, and learnings

A mandatory “Mistakes & Solutions” section is too blunt. Many sessions contain no mistake; agents may mislabel exploration as failure; and a solution can become stale after the code changes.

Use a first-class, evidence-backed model instead:

```text
Problem
  observed symptom
  expected behavior
  environment and scope
  evidence

Attempt
  action or hypothesis
  outcome: helped | no-effect | worsened | unknown
  evidence

Resolution
  root cause
  change
  verification

Learning
  reusable guidance
  confidence
  state: tentative | verified | contested | superseded | stale
```

The session UI can show “Problems & outcomes.” A project-wide “Lessons” view can aggregate verified solutions, corrections, dead ends, and contested advice. A learning should be promoted by explicit user confirmation or corroborating successful outcomes; one agent statement must not silently become trusted memory.

## Identity and storage

### Project-local identity

`ley init` creates a minimal repository-local directory:

```text
.ley/
├── project.json       # schema version, stable UUID, display name
├── capture.json       # approved roots, capture level, limits
└── .leyignore         # additional exclusions beyond .gitignore
```

This directory identifies and scopes a project. It does not contain the memory database, raw transcripts, embeddings, secrets, machine-specific vault paths, or agent credentials. `project.json` can be committed when a team wants stable identity; machine-local binding belongs in Ley's user config.

### Vault-owned durable memory

The selected filesystem vault remains the source of truth:

```text
.ley/
└── agent-memory/
    ├── projects/<project-id>/project.md
    ├── projects/<project-id>/sessions/<session-id>.md
    ├── projects/<project-id>/events/<session-id>.jsonl   # optional raw evidence
    └── projects/<project-id>/artifacts/...               # compact structured sidecars
```

Markdown summaries stay inspectable and linkable from normal notes. Append-only JSONL is appropriate for machine event evidence when enabled. A local SQLite/IndexedDB projection can accelerate graph, lexical, temporal, and optional embedding retrieval, but deleting it must not destroy durable memory.

Browser-local compatibility vaults cannot be directly opened by an external local MCP process. Agent integration therefore requires a filesystem vault or a running desktop companion that owns a filesystem-backed vault. The PWA can still display updates by refreshing the same granted folder.

## Capture policy

“Gather everything” is unsafe and counterproductive. Repository dependencies, build output, credentials, vendored sources, large binaries, and complete tool logs add cost and can leak secrets without improving memory.

Recommended default: **structured capture**.

- scan only the explicitly initialized project root;
- inherit `.gitignore`, then apply `.leyignore` and built-in secret/binary exclusions;
- inventory repository layout, manifests, documentation, tracked source, recent local Git state, and deterministic code relationships;
- record goals, final responses, touched paths, commands, exit status, test results, commits, decisions, unresolved work, and explicit checkpoints;
- store bounded excerpts and hashes rather than unlimited tool output;
- redact likely secrets before durable writes;
- do not store complete raw transcripts by default.

Optional capture levels:

| Level | Stored | Intended use |
| --- | --- | --- |
| Minimal | project graph, checkpoints, final session summary, decisions/learnings | maximum privacy |
| Structured (recommended) | minimal plus prompts, final responses, tool/command metadata, bounded evidence | strong continuity without transcript hoarding |
| Full evidence | versioned raw host transcript plus structured data | opt-in auditing; highest sensitivity and storage |

Every initialization must preview included/excluded categories and offer a dry run. Capture settings are per project and reversible. Deleting a project must show exactly which local files and indexes will be removed.

## Protocol and host integration

### MCP server

Default transport: local stdio. It has no listening port, account, OAuth flow, or remote endpoint. The server receives an approved project ID or path on each call; it must not depend on MCP Roots, which are informational and deprecated in the 2026 MCP evolution.

Initial read tools:

- `ley_projects_list`
- `ley_project_brief`
- `ley_context_search`
- `ley_context_pack`
- `ley_session_get`
- `ley_sessions_list`
- `ley_lessons_list`
- `ley_graph_neighbors`
- `ley_graph_path`

Initial write tools:

- `ley_project_initialize` (must return a preview before confirmation)
- `ley_session_start`
- `ley_session_checkpoint`
- `ley_session_finish`
- `ley_decision_record`
- `ley_problem_record`
- `ley_attempt_record`
- `ley_resolution_record`
- `ley_learning_record`
- `ley_memory_feedback` (`confirm`, `correct`, `reject`, `supersede`, `stale`)

Resources such as `ley://projects`, `ley://project/<id>/brief`, `ley://session/<id>`, and `ley://project/<id>/lessons` are useful for clients that expose resources well. Tools remain primary because client support and invocation policy are more consistent.

Every context response includes stable IDs, project/session scope, citations, timestamps, extraction method, confidence, trust state, freshness, conflicts, and an enforced token/character budget.

### Skill/plugin

The shared workflow should teach the agent to:

1. resolve the current approved project;
2. fetch a small context pack at session start;
3. query on demand instead of loading the entire graph;
4. checkpoint decisions and unresolved state after meaningful changes;
5. record problems/outcomes with evidence;
6. request user confirmation before promoting inferred lessons;
7. finish with a structured handoff.

The distributable package can include MCP configuration, the skill, host hook adapters, schemas, and uninstall/diagnostic commands. MCP configuration should default to user-local scope; project-shared config must never include absolute paths or credentials.

### Host adapters

Host capabilities differ and must remain versioned adapters:

- **Codex:** local/project MCP config is supported. `SessionStart`, `UserPromptSubmit`, `PostToolUse`, `PostCompact`, `SubagentStop`, and `Stop` hooks expose session/turn IDs and a transcript path, but the transcript format is explicitly unstable. `Stop` supplies the latest assistant message. Use stable hook fields first and parse transcripts only in a quarantined adapter.
- **Claude Code:** MCP plus `SessionStart`, per-turn/tool hooks, `PostCompact`, `Stop`, and `SessionEnd`. Hook input includes a transcript path and session ID; `PostCompact` includes a compact summary. Session end can finalize best-effort, while per-turn checkpoints prevent data loss on crashes.
- **Gemini CLI:** MCP plus before/after agent/tool, session start/end, and pre-compress hooks. It exposes a transcript path and stable agent response fields.
- **VS Code/Copilot:** MCP, skills, plugins, and lifecycle hooks exist, but hook support is newer/preview. Ship after the first three adapters prove the event schema.
- **Other agents:** MCP read/write tools work immediately; automatic capture depends on host lifecycle APIs. Provide explicit CLI and prompt workflows instead of pretending support is equivalent.

MCP alone gives the agent memory access. Hooks make capture/resume reliable. Skills make usage habitual.

## Retrieval and context assembly

Use a local hybrid pipeline:

1. exact filters for project, session, type, state, path, symbol, and time;
2. BM25/lexical retrieval over source and summaries;
3. graph expansion over high-confidence, relevant edges;
4. temporal reranking for current vs historical facts;
5. optional local embeddings, disabled until a local model is installed explicitly;
6. diversity and contradiction handling;
7. token-budgeted assembly with citations.

Do not return a giant project summary on every turn. Session start should usually include only project identity, active goal/tasks, recent verified decisions, relevant unresolved work, and a few verified lessons. Agents query deeper when the task demands it.

## Trust and privacy model

Local storage is not the same as end-to-end local inference. Ley itself performs no upload. When a user asks Claude, Codex, Gemini, or another cloud agent to retrieve Ley context, that returned context becomes part of the provider request. The UI and setup flow must say this plainly.

Additional controls:

- retrieval returns stored content as untrusted evidence, never executable instruction;
- trusted policies are stored separately from captured project text;
- source, inferred, agent-authored, and user-confirmed facts have distinct trust labels;
- redaction runs before persistence and again before MCP output;
- `.env`, credentials, private keys, auth stores, browser profiles, VCS internals, and configured patterns are excluded by default;
- all paths are canonicalized and constrained to approved roots;
- project IDs scope every query and transaction;
- context writes are atomic/locked and event ingestion is idempotent;
- corrections and supersession preserve history instead of rewriting it invisibly;
- full-evidence capture warns that OS-level disk access can read plaintext unless the device/vault is encrypted.

## Product experience

Add an **Agent Memory** workspace rather than mixing raw sessions into the normal note list:

```text
Agent Memory
├── Projects
│   └── Project
│       ├── Overview / current brief
│       ├── Sessions
│       ├── Decisions
│       ├── Problems & outcomes
│       ├── Lessons
│       ├── Artifacts
│       └── Project graph
├── Review inbox
├── Cross-project search
└── Capture & privacy
```

Sessions get an agent-suggested name from their goal, a stable ID, host/model metadata, time range, outcome, handoff, and direct links to notes/files/symbols/commits. Users can rename sessions, correct fields, reject memories, promote a learning to a normal note, or link a session into a Canvas. Agent nodes in the graph need a distinct visual language and provenance inspector.

## Implementation roadmap

### Phase 0 — decisions, schemas, and privacy threat model

- confirm the four open product choices below;
- write ADRs for storage, capture levels, trust/provenance, temporal facts, MCP tool semantics, and host adapters;
- define JSON Schema fixtures for project, session, event, fact, edge, problem, attempt, resolution, learning, citation, feedback, and context pack;
- define secret/path/cross-project threat tests before ingestion code.

Exit: a sample project with two sessions can be represented losslessly and reviewed by a human.

### Phase 1 — local engine and professional repo boundary

- extract framework-independent domain logic;
- add a native `ley` CLI and shared Rust core suitable for Tauri and stdio MCP;
- implement vault/project registry, `.ley/` initialization, locking, atomic append/write, schema migration, and diagnostics;
- keep existing note/PWA functionality working during the boundary change.

Exit: `ley init`, `ley doctor`, and project discovery work offline on Linux/macOS/Windows fixtures.

### Phase 2 — deterministic project knowledge

- git-aware scoped walker with ignore/redaction/limits and dry-run preview;
- manifests, docs, configs, Git metadata, and initial language AST extraction;
- incremental content-hash updates and deleted/renamed artifact handling;
- provenance-preserving graph/index projection;
- optional import adapter for Graphify `graph.json`, not a hard dependency.

Exit: a real repository can be indexed, updated, inspected, and deleted without network or cross-project leakage.

### Phase 3 — read-only MCP and context packs

- stdio lifecycle and MCP Inspector coverage;
- project/brief/search/path/neighbor/session/lesson tools and resources;
- budgeted cited context with freshness/conflict/trust metadata;
- installation commands for Codex, Claude, and Gemini.

Exit: each host can ask a project question and receive the same bounded, cited local answer context.

### Phase 4 — structured session capture

- session start/checkpoint/finish and idempotent event writes;
- Codex, Claude Code, and Gemini adapters using stable lifecycle fields;
- manual fallback commands;
- crash recovery and resumed-session identity;
- minimal/structured/full capture controls and transcript adapter versioning.

Exit: session two can accurately resume session one's goal, changes, verification, decisions, and unresolved work.

### Phase 5 — learning and review

- problem/attempt/resolution/learning records;
- confirm/correct/reject/supersede/stale feedback;
- deterministic aggregation with corroboration, recency, source-change staleness, and conflict visibility;
- review inbox and promotion to ordinary Markdown notes.

Exit: a corrected failed approach is not recommended again, while the verified resolution is retrieved with evidence.

### Phase 6 — Agent Memory UI and graph

- project/session navigation, timeline, decisions, problems/outcomes, lessons, artifacts, privacy controls;
- temporal/provenance graph filters and source inspector;
- links between agent memory, notes, Canvas, files, symbols, and commits;
- browser-folder refresh and explicit browser-local limitation states.

Exit: users can understand, edit, delete, and trace every memory without opening raw storage files.

### Phase 7 — broader hosts, optional local semantics, and hardening

- VS Code/Copilot adapter, then other hosts based on stable lifecycle support;
- optional local embedding/reranking package with explicit model installation;
- import/export, backup, migration and encrypted-vault compatibility guidance;
- performance, fuzz, concurrency and security hardening.

## Evaluation plan

Green unit tests are insufficient. Build a deterministic scenario corpus:

- ten-session feature implementation with decisions and handoffs;
- bug diagnosis containing several failed attempts and one verified fix;
- architecture decision later superseded;
- renamed/deleted code invalidating an old learning;
- two projects with similar names but forbidden cross-retrieval;
- secrets placed in ignored and non-ignored locations;
- crash before session end and later resume;
- same session event delivered twice;
- malicious repository text attempting to become an instruction;
- strict 500/1,500/3,000-token retrieval budgets.

Measure recall@k, citation precision, answer key-fact coverage, stale/conflict accuracy, false-memory rate, correction compliance, secret exposure, cross-project leakage, capture completeness, p50/p95 latency, index size, and tokens saved versus raw repository/session loading. Run LOCOMO and LongMemEval only as supplementary conversational-memory checks; coding continuity needs its own ground truth.

## Sources

- [MCP architecture and primitives](https://modelcontextprotocol.io/docs/learn/architecture)
- [MCP tools specification](https://modelcontextprotocol.io/specification/draft/server/tools)
- [MCP Roots deprecation rationale](https://modelcontextprotocol.io/seps/2577-deprecate-roots-sampling-and-logging)
- [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)
- [Codex lifecycle hooks](https://learn.chatgpt.com/docs/hooks)
- [Claude Code lifecycle hooks](https://code.claude.com/docs/en/hooks)
- [Claude Code MCP integration](https://code.claude.com/docs/en/mcp)
- [Gemini CLI hook reference](https://geminicli.com/docs/hooks/reference/)
- [VS Code agent customization](https://code.visualstudio.com/docs/copilot/concepts/customization)
- [Obsidian internal links](https://obsidian.md/help/links)
- [Obsidian backlinks](https://obsidian.md/help/Plugins/Backlinks)
- [Obsidian Bases](https://obsidian.md/help/bases)
- [Obsidian Canvas](https://obsidian.md/help/plugins/canvas)
- [LangGraph memory categories](https://docs.langchain.com/oss/python/concepts/memory)
- [Letta context hierarchy](https://docs.letta.com/guides/core-concepts/memory/context-hierarchy)
- [Zep temporal knowledge graph paper](https://arxiv.org/abs/2501.13956)
- [Mem0 long-term memory paper](https://arxiv.org/abs/2504.19413)
- Local Graphify reference: `ref/graphify/README.md`, `BENCHMARKS.md`, `graphify/serve.py`, and `graphify/reflect.py`

## Confirmed product decisions

1. **Cloud-agent boundary:** Ley never uploads independently. Context is shared with Claude, Codex, or another cloud agent only when the user intentionally invokes that agent. Fully local model inference remains possible later, but is not required for ordinary cloud-agent integration.
2. **Capture default:** Structured capture is the default, with Minimal and opt-in Full Evidence modes. Complete raw transcripts are not stored automatically.
3. **Memory review:** agent-generated memory remains in a separate Agent Memory namespace with review and promotion into ordinary notes.
4. **Project binding:** durable memory lives centrally in the chosen Ley filesystem vault; each initialized repository contains only a minimal `.ley/` identity and capture-policy folder.
