# Ley first-principles product and architecture audit — 2026-09-25

## Status

This audit supersedes the assumption that existing Ley code, `LEY.md`, ADRs, tests, or product copy are
binding product decisions. They remain evidence about what was built and what was learned, but every
capability must re-earn its place against a simpler alternative and measurable downstream value.

The conclusion is a product reset, not a claim that the current implementation is generally broken.
At audit time the tracked application passes TypeScript typechecking, ESLint, the production web build,
and `cargo check --locked --workspace`; the production npm audit reports no known production-package
vulnerabilities. The problem is primarily scope, conceptual load, duplicated state machinery, and weak
evidence that the accumulated complexity improves real coding-agent outcomes enough to justify itself.

## Product decision

Ley should become a **local, trustworthy continuity and context layer for coding agents**.

Its primary user job is:

> When an agent or human returns to a coding project, recover the smallest useful account of what was
> decided, what was attempted, what failed, what was verified, what remains open, and where the evidence
> came from — without treating historical agent text as current truth or leaking project data by default.

Ley should not primarily compete with Obsidian, Logseq, VS Code, or general note applications. Existing
editors already solve writing, backlinks, Canvas/diagramming, daily notes, search, workspace layout,
bookmarks, and extensibility at far greater ecosystem scale. Ley's differentiated value is trustworthy
agent continuity, scoped retrieval, provenance, stale-memory resistance, and local control.

## Principles that survive the reset

These principles still earn their complexity:

1. **Current evidence outranks remembered evidence.** Historical agent memory is context, not truth.
2. **Human intent must not be self-grantable by an agent.** A user-selected requirement/rule source may
   need an approval/hash boundary if the agent can edit the same file.
3. **Every durable memory should have provenance.** A user or future agent must be able to drill from a
   recalled claim to the originating session/tool/source evidence.
4. **Project scope must be explicit.** Ley must not ambiently search unrelated projects or user data.
5. **Context should be small by default.** Retrieval should optimize downstream task success rather than
   expose Ley's internal ontology.
6. **Privacy and egress are product features.** Local storage, deletion, export, and cloud/local sharing
   boundaries must remain inspectable.
7. **History should be preserved without implying applicability.** Git revision/lineage metadata is a
   useful lightweight signal for stale or divergent memories.
8. **Derived views should be disposable.** Search indexes, summaries, embeddings, and presentation views
   must be rebuildable from durable records.
9. **Complexity must earn itself in task-level evaluation.** Internal invariant coverage is necessary but
   cannot substitute for showing that agents solve representative tasks better.

The old Authority / Memory / Context three-plane model may remain a useful implementation mental model,
but it should not be a user-facing ontology and it should not force separate durable subsystems when a
simpler data model can enforce the same boundaries.

## What the audit covered

The audit covered the tracked repository end to end:

- root package/build/tooling configuration;
- all Rust crates (`ley-core`, CLI, MCP, GitHub connector, semantic installer);
- the Tauri native shell and command boundary;
- the React application, note workspace, browser-local/browser-folder modes, Agent Memory workspace,
  website, and IndexedDB infrastructure;
- Codex and Claude Code plugins, MCP configuration, lifecycle hooks, skills, and marketplaces;
- deterministic and model-dependent evaluation runners and fixtures;
- the portability workflow and platform-security probes;
- public JSON schemas;
- architecture/security/product/research docs and the ADR set;
- current build/dependency health;
- current external alternatives and host guidance.

At the time of audit there were roughly 460 tracked files, about 82k lines in `ley-core`, about 38k
TypeScript/TSX lines, a 15k-line deterministic evaluation runner, an approximately 11k-line MCP server,
85 ADRs, 53 deterministic acceptance scenarios, and only two checked-in model-dependent coding-task
fixtures. Those numbers are not themselves defects; they make the burden of proof for every additional
concept much higher.

## External baseline

Ley should compare itself with current systems and host capabilities rather than with a memoryless model.
Relevant current baselines include:

- OpenAI Agent Plugins: focused skills + MCP + optional hooks; current guidance says to start with the
  smallest plugin shape that supports the use case and to define tools around user outcomes rather than
  mirror an internal API.
  - <https://developers.openai.com/plugins/concepts/plugins>
  - <https://developers.openai.com/plugins/plan/tools>
  - <https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra>
- OpenViking: a context database with one filesystem-like interaction model, scoped retrieval, layered
  summaries, observable retrieval, and a comparatively small memory/tool vocabulary.
  - <https://github.com/volcengine/OpenViking>
- Hindsight: exposes the core memory operations as retain / recall / reflect rather than surfacing its
  internal memory architecture to the agent.
  - <https://github.com/vectorize-io/hindsight>
- Letta: distinguishes small in-context memory blocks from archival/searchable memory without requiring
  users to understand a large authority ontology.
  - <https://docs.letta.com/api/resources/agents>
- Zep/Graphiti: provides temporal memory and changing-fact handling through a temporal graph model.
  - <https://help.getzep.com/v2/understanding-the-graph>
- Mature note editors such as Obsidian already cover the general-purpose Markdown workspace surface that
  Ley independently rebuilt.

These systems are prior art, not templates. Their strongest lesson is interaction compression: powerful
internals do not require dozens of user- or agent-facing concepts.

## Major findings

### 1. Ley currently contains two products

The public website and README primarily present a local-first Markdown notebook / second brain. The code
contains a complete notes application: editor, wiki links, backlinks, tags, search, saved searches,
bookmarks, daily notes, templates, properties/collections, graph, JSON Canvas, workspace layouts,
browser-folder storage, browser-local IndexedDB storage, PWA support, and revision recovery.

Agent Memory is a second large product layered on top. Its differentiated loop is desktop-only because it
needs local project access, CLI/MCP, hooks, and private machine state. The browser app explicitly cannot
provide that loop.

Git history shows the general note product was intentionally rebuilt toward Obsidian-like parity before
Agent Memory was added. Therefore the doctrine that note-taking and agent memory must remain equally
first-class is historical inheritance rather than an evidence-backed requirement.

**Decision:** retire the general note application from Ley's primary product. Keep data export/open-in-editor
interoperability. If a standalone Ley Notes product is ever wanted, it should be justified separately and
should not determine the agent-memory engine's architecture.

### 2. The public product promise is pointed at the less differentiated half

The landing page says “A local-first notebook” and “Notes with a memory.” Tauri metadata calls Ley a
Markdown-native second brain. Meanwhile the strongest technical work is the local agent-continuity engine.

**Decision:** reposition Ley around local project continuity for coding agents. The public website should
explain the continuity problem first, not sell a browser notebook first.

### 3. Setup has too many independent state locations

Today a fully functioning project may involve:

1. repo-local `.ley/` identity/capture policy;
2. OS-private project binding/catalog/authority registries;
3. a separately chosen filesystem “Ley vault” holding Agent Memory plus optionally the user's notes.

This produces moved-vault, initialized-but-unbound, wrong-vault, reconnect, bootstrap, and multi-vault UI
states. The separation is carefully implemented but the selected notes-vault binding primarily supports
the inherited note-application architecture.

**Decision:** keep a tiny repo-local Ley identity/config marker, but remove the requirement for a separate
notes vault. Machine-managed continuity state should live in Ley's OS-private application data. Portable
user-authored files remain where the user already keeps them.

### 4. Machine state is fragmented across too many custom JSON registries

Bindings, project catalog, Specifications, mounts, scopes, policy bundles, egress, external connectors,
bootstrap authority, sessions, learnings, graphs, artifacts, and indexes use multiple independent stores.
Many registries separately implement the same parent-directory validation, lock file, JSON validation,
atomic write, and recovery patterns.

This has produced excellent local defensive code, but at system level it multiplies schemas, locks,
cross-registry coordination, migration logic, tests, and failure modes.

**Decision:** rebuild machine-managed metadata around SQLite transactions rather than a family of JSON
registries.

Proposed storage boundary:

- one OS-private Ley SQLite database for projects, sessions/events, durable handoff/memory records,
  evidence references, selected trusted-source approvals, privacy/egress settings, and integration state;
- content-addressed files under OS-private app data only for large immutable evidence/blob payloads where
  storing bytes in SQLite is undesirable;
- optional vector/index sidecars remain rebuildable caches;
- repo-local `.ley/` contains only portable project identity and user-visible capture/retention settings;
- explicit export produces a portable project-memory bundle when the user wants backup/migration.

A single DB is preferred initially because cross-project catalog/search and authority changes can be
transactional. Project erasure remains a project-partition delete plus referenced-blob garbage collection.
If real scale/corruption isolation evidence later justifies per-project databases, split then.

### 5. Ley captures too much repository state for its core job

The current engine captures source snapshots, builds deterministic code graphs, retains graph history, and
offers source/graph tools even though the coding agent already has the live repository, grep/search, Git,
language tooling, and often GitHub access.

The unique continuity problem is not “help the next agent read the repo”; it is “help the next agent know
what happened before and which historical claims still matter.”

**Decision:** stop making full project capture/code graph a required core path. Keep lightweight Git
revision metadata and evidence references. Store exact snippets/blobs only when needed to preserve
historical evidence that Git/current source cannot safely reconstruct. Reintroduce richer source snapshots
or dependency graphs only if realistic task ablations show incremental value over normal agent repo tools.

### 6. Session/recovery schema evolution is encoding features too literally

Session projections have accumulated v1 through v16 variants for turn evidence, verification, utility,
multimodal evidence, imported history, typed Decision/Problem/Task/Plan recovery, batch recovery, rich
Problem recovery, composite recovery, tool evidence, Procedure application, and observed-command recovery.
`memory_transition.rs` and the MCP tool surface mirror those distinctions with many verify/commit routes.

That machinery is internally rigorous but it makes the agent skill teach Ley's state machine to the model.

**Decision:** replace shape-specific recovery protocols with a small event envelope and a small durable
handoff vocabulary.

Suggested durable record kinds:

- goal / handoff summary;
- decision or constraint;
- attempt + outcome;
- verification result;
- unresolved item;
- correction/supersession marker;
- evidence reference.

Events should use `kind + payload_version` so individual event types can evolve without creating an entire
session-store generation for each feature. A failed/interrupted session should leave bounded raw evidence;
a later local or host-assisted consolidation may propose a tentative handoff rather than require many
special deterministic recovery APIs.

### 7. Trust/authority concepts are more granular than current evidence justifies

The existing design distinguishes many states and product concepts: reviewed learnings, Specifications,
Acceptance criteria, Verification methods, mounts, reusable scopes, policy bundles, bootstrap
Specifications, bootstrap References, egress overrides, current-state projection, dossiers, health,
legibility, inspectors, runbooks, and more.

Several are useful safety experiments. They should not all survive as independent product primitives.

**Decision:** keep only the minimum authority model:

- current user request;
- user-selected current requirement/rule sources;
- current live project/Git evidence;
- historical Ley memory with provenance and applicability metadata.

If the user selects a file as a privileged requirement/rule source and the agent can modify it, store the
approved path/hash and require reapproval after changes. Do not derive separate durable Acceptance-criteria
or Verification-method authority objects unless a later user study proves a need.

Historical memory should expose source, revision/time, correction/supersession, and optional user pinning.
Avoid presenting “trusted” as a synonym for true.

### 8. Cross-project policy machinery should be removed from the first product

Persistent Context Mounts, Knowledge Scopes, Policy Bundles, and separate bootstrap authority flows solve
real theoretical boundaries but create a large authority graph before Ley has proven that users need
persistent cross-project composition.

**Decision:** delete/defer those product concepts. For the first focused product, cross-project context is
an explicit per-request/per-session source selection. If repeated workflows later prove persistent sharing
valuable, add one simple named source group rather than rebuilding the current hierarchy by default.

### 9. Retrieval needs a simpler and better-calibrated foundation

Current structured memory search bounds session/learning candidates to 256 before semantic ranking, with
preselection driven by exact/lexical match and recency. The Context Compiler admits lexical-ranked items or
semantic items above a fixed 0.30 similarity threshold. Conflict detection includes same-title material
difference and token-overlap negation heuristics.

Those are defensible deterministic baselines, but they are not strong enough to carry product language
such as semantic truth/state adjudication, and they can discard semantically relevant paraphrases before
vector ranking sees them.

**Decision:** rebuild retrieval around independent candidate generators:

- SQLite FTS lexical search;
- optional vector search over the full eligible durable-memory set;
- metadata/time/revision filters;
- union + reciprocal-rank/rerank;
- compact task-conditioned final selection.

Explicit structural supersession/revision relations may exclude or demote items. Heuristic textual
disagreement should be disclosed as uncertainty rather than silently treated as adjudicated truth.

The bundled local semantic model is **deferred**, not automatically retained. Keep it only if realistic
ablation shows enough task-success improvement to justify roughly 125 MiB of model data plus maintenance.

### 10. The agent-facing API mirrors internal architecture

The fixed-project MCP implementation defines roughly mid-40s Ley tools before runtime gating. The write
enabled host package exposes a broad recovery/memory workflow. The Codex skill alone is around 28 KiB of
procedural instructions explaining Ley's internal concepts.

Current OpenAI plugin guidance explicitly recommends a focused tool surface around user goals and warns
against unnecessary prompt/skill scaffolding with more capable models.

**Decision:** target a very small agent API, approximately:

1. `ley_brief(task)` — smallest cited current/historical continuity pack for the task;
2. `ley_search(query, filters?)` — explicit broader historical recall;
3. `ley_evidence(ref)` — exact provenance/source drill-down;
4. `ley_checkpoint(...)` — one structured durable write path when automatic lifecycle capture is
   unavailable or when the agent has meaningful state to persist.

Setup/status may be an MCP resource or a small separate tool if evidence shows it is needed. User-only
correction, deletion, privileged-source approval, and privacy controls should stay outside normal agent
mutation authority.

Hooks may capture bounded lifecycle/tool evidence automatically, but the skill should explain only the
four user goals above. Internal schema/version/recovery details must not be model instructions.

### 11. Automatic context injection needs clearer consent and stricter value evidence

Current host plugins inject bounded context on `UserPromptSubmit`. The Full Evidence UI copy currently says
cloud agents may receive only context the user asks them to retrieve, which understates automatic injection
after integration configuration.

**Decision:** fix the copy during migration and make automatic task briefing a separately visible toggle.
The default should be measured against explicit `ley_brief`; automatic injection survives only if it
improves task success without unacceptable irrelevant-context or token cost.

### 12. Derived products should collapse into two inspection surfaces

Topic Dossier, Current Project State, Context Pack Inspector, Memory Health, Agent Legibility, runbook
compilation, and related projections contain useful experiments but create too many product names.

**Decision:** collapse retained value into:

- **Brief** — what Ley would give an agent for a task;
- **Evidence / Why** — why an item was included, provenance, freshness/revision, and excluded alternatives.

Health/setup diagnostics belong in project status. Runbook-to-Skill export should be removed/deferred;
modern hosts already have native Skill packaging and Ley should not turn historical memory into executable
instructions without demonstrated demand.

### 13. Dedicated GitHub connectors are not core

The GitHub connector implementation itself is narrow and careful. The product need is weak because modern
coding hosts already have GitHub tools/plugins and network research capabilities.

**Decision:** defer/remove the dedicated connector from the core product. Preserve a generic explicit
external-evidence model so a host can cite an issue/PR/document when useful. Reintroduce provider-native
connectors only if offline snapshot/reproducibility tasks show unique value.

### 14. The desktop should become a focused control center

Keep a native UI because local project setup, privacy, review, evidence inspection, export/erase, and host
integration are genuinely easier to trust visually.

The primary desktop should contain roughly:

- Projects / setup / integration status;
- “What the next agent gets” Brief preview;
- Search / Recall;
- session timeline / handoffs;
- review/correction/pin queue;
- Evidence / Why inspector;
- Privacy, egress, export, erasure, and retention settings.

Delete from the primary product: general Markdown editing, daily notes, bookmarks, saved-search UI,
collection tables, generic backlinks, note graph, JSON Canvas editor, workspace-layout manager, browser-local
notebook mode, and the PWA note workspace. Keep the marketing/docs website as a normal web surface.

User-authored Markdown should open in the user's preferred editor. A thin Obsidian/VS Code integration may
be added later if it demonstrably improves the continuity workflow.

### 15. Two current desktop defects reinforce the cost of maintaining the note-app surface

The audit confirmed:

1. Graph click routing treats every node ID beginning with `c` as a community aggregate, while normal page
   IDs may also begin with `c`; affected real notes cannot be opened from the graph.
2. The Tauri vault-path helper prevents textual `..` traversal but joins paths without a no-follow
   component walk or canonical containment check. A symlink inside a selected vault can therefore redirect
   some native file operations outside the intended vault root.

If the legacy note app remains runnable during migration, the native symlink boundary is a security fix,
not optional polish. The graph bug should either be fixed or disappear with graph retirement.

### 16. Evaluation is rigorous internally but underpowered where it matters most

Ley has strong deterministic safety/regression coverage. The deterministic “downstream” checks mostly
prove that expected context is included/excluded under bounded budgets. The opt-in real-agent benchmark
currently contains only two small synthetic hidden-contract tasks.

**Decision:** freeze new product concepts until a stronger real-agent benchmark exists.

Required baseline arms:

1. host-native agent + repository tools/instructions only;
2. a concise human-authored `HANDOFF.md` baseline;
3. minimal redesigned Ley;
4. current full Ley while it still exists, for deletion/ablation evidence.

Representative task families should include:

- resume interrupted implementation without rereading everything;
- avoid repeating a known failed attempt;
- respect a changed requirement over stale memory;
- continue across a divergent/merged branch;
- identify what was actually verified vs merely claimed;
- recover from a missing final checkpoint/crash;
- use an explicit second-project reference without ambient leakage.

Measure:

- hidden task correctness;
- stale-memory-caused errors;
- repeated dead-end rate;
- evidence correctness;
- context tokens;
- latency;
- review/setup burden;
- number of Ley interactions/tool-selection failures.

Every optional feature (semantic model, source graph, automatic injection, cross-project sharing, semantic
consolidation, advanced evidence capture) needs an ablation showing material improvement before returning.

### 17. CI should protect the actual baseline continuously

The only checked-in GitHub workflow is a useful manual six-lane portability experiment. It is not a routine
quality gate.

**Decision:** add routine CI for the focused product:

- Rust format/check/test for core paths;
- TypeScript typecheck/lint/test/build for whichever UI remains;
- deterministic minimal continuity/evidence evaluation;
- plugin manifest/package smoke tests;
- schema/migration tests;
- security boundary tests.

Keep the six-lane x64/ARM64 portability workflow manual or scheduled unless its runtime cost becomes small
enough for PR use. Add explicit checked-in Node and Rust toolchain versions so local/CI builds are
reproducible.

### 18. Documentation should stop being an active architecture dependency

The 85 ADRs are valuable history but too many describe product concepts that should no longer be current.
Long skills/docs also cause old concepts to keep re-entering implementation decisions.

**Decision:** during migration:

- keep one concise current architecture document;
- keep one privacy/threat model;
- keep the real-agent evaluation contract;
- archive superseded ADRs/research under an explicitly historical directory rather than deleting evidence;
- mark historical documents non-normative;
- keep `LEY.md` concise and execution-oriented.

## Whole-project decision matrix

| Area | Decision | Reason |
| --- | --- | --- |
| Repo-local project identity | **KEEP / simplify** | Stable project scope across path moves is useful. |
| Current source-capture policy | **REBUILD** | Retention consent is useful; full repo capture should not be the default core. |
| Separate filesystem Ley/note vault binding | **DELETE from core** | Historical note-app coupling and large setup/failure burden. |
| OS-private JSON registries | **REBUILD in SQLite** | Duplicated locks/schemas and poor cross-registry transactions. |
| Content-addressed immutable evidence blobs | **KEEP selectively** | Useful for historical evidence that cannot be reconstructed safely. |
| Full artifact snapshot history | **DEFER/remove default** | Duplicates Git/live agent source access without downstream proof. |
| Deterministic code graph/history | **DEFER** | Interesting but unproven incremental benefit over normal coding tools. |
| Structured sessions | **KEEP / simplify** | Core continuity primitive. |
| Prompt/response/tool evidence | **KEEP bounded / configurable** | Useful provenance and crash recovery; privacy-sensitive. |
| v1–v16 session/recovery protocol family | **REBUILD** | Encodes feature variants into durable/API complexity. |
| Reviewed learnings | **KEEP concept / simplify** | Corrections and reusable lessons matter; current state model is too granular. |
| Memory trust state machine | **REBUILD** | Provenance/applicability/pin/supersession clearer than “trusted = true.” |
| Git revision applicability | **KEEP lightweight** | Strong stale-memory safety value with bounded cost. |
| Specifications approval/hash | **KEEP minimal selected-source approval** | Prevents agent self-authorization. |
| Derived Acceptance criteria / Verification methods | **DELETE/defer** | No evidence they need separate product objects. |
| Context Compiler | **KEEP goal / rewrite simpler** | Task-conditioned small context is core; current admission stack is overgrown. |
| Context Pack Inspector | **MERGE into Evidence/Why** | Explainability useful; separate product object unnecessary. |
| Context Mounts | **DELETE/defer** | Replace with explicit task source selection first. |
| Knowledge Scopes | **DELETE/defer** | Premature persistent sharing ontology. |
| Policy Bundles | **DELETE/defer** | Premature policy-composition ontology. |
| Bootstrap Spec/Reference subsystems | **DELETE/reduce to explicit source selection** | Too many startup authority modes. |
| Project memory search | **KEEP / rebuild retrieval** | Core recall need, but candidate generation/admission needs redesign. |
| Bundled local semantic model | **DEFER pending ablation** | High size/maintenance cost; value not established. |
| Topic Dossier | **DELETE as named feature** | Fold useful output into Brief/Search. |
| Current Project State | **DELETE as named feature** | Fold into Brief. |
| Memory Health | **MERGE into Status** | Diagnostics useful; standalone conceptual layer unnecessary. |
| Agent Legibility | **DELETE/defer** | Agent can inspect repo; no proven incremental value. |
| Runbook compile / Skill export | **DELETE/defer** | Duplicates host-native Skills and risks memory→instruction promotion. |
| Context utility feedback | **DEFER** | Useful research instrumentation, not product primitive until calibrated. |
| Multimodal retained evidence | **DEFER** | No coding-task evidence justifying first-class complexity yet. |
| GitHub connector | **DEFER/remove core** | Host-native GitHub tooling overlaps heavily. |
| Historical host import | **DEFER** | Useful migration niche, not critical first product. |
| MCP server | **KEEP / radically shrink** | Best portable host boundary; tool catalog should map to user goals. |
| Lifecycle hooks | **KEEP / simplify** | Strong automatic continuity value if consent/quality are explicit. |
| Codex/Claude skills | **KEEP / rewrite tiny** | Needed workflow hints, but current instructions are too large. |
| General notes editor | **DELETE from primary Ley** | Mature existing alternatives; little unique agent-memory value. |
| Backlinks/search/bookmarks/daily notes/templates | **DELETE from primary Ley** | General notebook scope. |
| Note graph / Canvas / workspace layouts | **DELETE from primary Ley** | Maintenance-heavy duplicate surface. |
| Browser-folder / browser-local notebook / PWA | **DELETE from primary Ley** | Cannot deliver core agent continuity loop. |
| Native desktop shell | **KEEP focused** | Best user-control surface for local integrations/privacy/review. |
| Public website/docs | **KEEP / reposition** | Needed distribution and explanation. |
| 85 ADRs | **ARCHIVE as history** | Valuable evidence; should no longer steer current design by inertia. |
| Deterministic safety evals | **KEEP smaller core set** | Excellent regression value. |
| Real-agent task eval | **EXPAND before feature growth** | Main missing evidence for product value. |
| Six-lane portability/security workflow | **KEEP** | Strong evidence for supported OS/architecture boundaries. |

## Migration order

The reset should be incremental enough to preserve working evidence and avoid a blind rewrite.

### Phase 0 — freeze and benchmark

1. Stop adding new user-facing Ley concepts.
2. Preserve the current full implementation long enough to benchmark it.
3. Build the realistic baseline-vs-handoff-vs-minimal-Ley-vs-full-Ley task suite.
4. Fix only high-severity current security/data-loss issues needed while the legacy app remains runnable.
5. Pin Node/Rust toolchains and add ordinary CI.

### Phase 1 — new storage and minimal continuity core

1. Add SQLite schema/migrations for projects, sessions/events, handoff items, evidence, corrections,
   selected trusted sources, privacy/egress, and integration state.
2. Add content-addressed evidence blob storage where necessary.
3. Keep the current `.ley` identity but simplify its policy/config fields.
4. Implement import from the current JSON/vault stores; do not force users to lose old memory.
5. Prove transactional erasure, crash recovery, concurrent writers, backup/export, and migration.

### Phase 2 — minimal retrieval and agent API

1. Implement FTS-based historical search over the new store.
2. Implement `brief`, `search`, `evidence`, and one checkpoint/write path.
3. Rebuild Codex/Claude packaging around the small tool surface and a short skill.
4. Compare explicit brief vs automatic hook injection.
5. Add optional vector search only after lexical baseline is measured.

### Phase 3 — focused desktop

1. Build the control-center UI over the new engine.
2. Provide export/open-in-editor rather than a full Markdown editor.
3. Migrate privacy, review/correction, evidence inspection, and erasure into the focused UI.
4. Remove browser/PWA notebook positioning.

### Phase 4 — retire legacy breadth

After migration/export and benchmark equivalence are proven:

1. remove the general note workspace and obsolete Tauri filesystem surface;
2. remove old graph/snapshot/authority subsystems not selected by ablation;
3. shrink CLI/MCP/skills;
4. archive superseded ADRs/tests/docs;
5. delete compatibility code only after migration tests prove user data is preserved.

### Phase 5 — re-earn optional features

Only add back capabilities when a concrete benchmark/user workflow demonstrates value. Candidates include:

- semantic/vector retrieval;
- richer source snapshots;
- dependency graph;
- cross-project persistent sharing;
- external connectors;
- multimodal evidence;
- automatic consolidation;
- utility feedback.

## Explicit non-goals for the reset

Until evidence changes the decision, Ley is not trying to be:

- a general Obsidian replacement;
- a browser-first note app;
- a universal personal knowledge base;
- an issue tracker;
- a Git/worktree manager;
- a generic DLP/policy engine;
- a replacement for GitHub integrations;
- a host Skill authoring platform;
- an autonomous truth engine that semantically decides which text is correct.

## What should be retained from the current implementation while rebuilding

The reset should reuse proven implementation where it matches the new product:

- credential redaction and bounded evidence handling;
- capability/no-follow filesystem techniques from the core ingestion path;
- project identity and deterministic request/idempotency patterns;
- session lifecycle concepts and exact evidence IDs;
- Git revision relation resolver;
- egress/privacy test ideas;
- erasure tests;
- six-lane portability/security infrastructure;
- host compatibility probes;
- strict result-size/budget discipline;
- deterministic provenance/citation structures;
- the best adversarial fixtures, rewritten against the smaller model.

Do not preserve an implementation merely because tests exist for it. Tests for a deleted product concept
should be archived/deleted with that concept after migration evidence is complete.

## Immediate defects / debts to address during the transition

1. Tauri note-vault operations need a no-follow/canonical-containment security boundary if that code ships
   before retirement.
2. Graph node/community ID routing has a real `c*` collision bug.
3. Full Evidence privacy copy must acknowledge automatic hook context injection when enabled.
4. Codex/Claude package versions should come from one release source of truth.
5. Codex packaging should migrate toward the current portable Agent Plugins layout rather than depend on
   the compatibility-only `.codex-plugin` layout forever.
6. Check in explicit Node and Rust toolchain versions.
7. Add ordinary CI; current manual portability CI is not enough.
8. Remove stale local-reference-repository assumptions; the old local clones are no longer part of Ley.

## Final decision

The current project contains valuable engineering, especially around provenance, privacy, revision safety,
erasure, bounded context, and adversarial evaluation. The best decision is **not** to continue the existing
roadmap unchanged and **not** to perform a blind rewrite.

The best decision is to preserve those hard-won invariants while aggressively collapsing the product into
a small local coding-agent continuity engine, migrate machine state to a transactional store, reduce the
agent API to a few goal-level operations, retire the inherited note-app scope, and require realistic
downstream evidence before any advanced feature earns its way back.

That direction is now the baseline against which future Ley work should be judged.

## Coverage appendix — every major tracked subsystem classified

This appendix exists to make the audit's coverage explicit. A module being listed here does not mean its
current API survives; it records how its responsibility maps into the reset.

### `ley-core` modules

| Module | Reset disposition |
| --- | --- |
| `acceptance_verification.rs` | **Delete/defer as a separate subsystem.** Keep only generic evidence/verification records and minimal privileged-source approval. |
| `agent_legibility.rs` | **Delete/defer.** Reintroduce only if agent-task evidence proves a project-legibility projection adds value over normal repo inspection. |
| `binding.rs` | **Replace.** Remove separate notes-vault binding; project location/catalog belongs in transactional app state. |
| `bootstrap_specification.rs` | **Delete/reduce.** Fold useful behavior into explicit privileged-source selection for initialized/uninitialized workspaces. |
| `consolidation_inbox.rs` | **Keep the user need, merge UI/API.** Becomes a simple review/proposed-handoff queue rather than a named architecture layer. |
| `context_compiler.rs` | **Keep the goal, rewrite.** Small task-conditioned Brief remains core; current admission/authority breadth does not. |
| `context_mount.rs` | **Delete/defer.** Replace with explicit per-task/per-session source selection first. |
| `context_pack_inspector.rs` | **Merge.** Its useful explainability becomes Evidence / Why. |
| `cross_project_search.rs` | **Simplify/defer persistent behavior.** Cross-project retrieval must require explicit source selection; no ambient search. |
| `current_project_state.rs` | **Merge into Brief.** No separate named product. |
| `egress_policy.rs` | **Keep principle, simplify state.** Project/source sharing policy moves into the transactional store; avoid a large retained-origin policy graph until needed. |
| `external_connector.rs` | **Defer/remove core.** Generic evidence references survive; provider connector lifecycle does not need to. |
| `graph.rs` | **Defer.** Code graph/history is not required for the first continuity product. |
| `historical_host_import.rs` | **Defer.** Migration/niche utility, not first-order continuity. |
| `host_adapter.rs` | **Keep/rewrite.** Lifecycle capture and optional Brief injection remain important, with a smaller contract. |
| `ingestion.rs` | **Mine for security/evidence primitives, then simplify heavily.** Keep redaction/no-follow/hash/citation ideas; stop requiring full-project snapshot ingestion. |
| `knowledge_scope.rs` | **Delete/defer.** Premature persistent sharing ontology. |
| `knowledge_view.rs` | **Merge into Brief/Search/Evidence.** Do not retain a separate product projection unless a benchmark needs it. |
| `learning.rs` | **Keep concept, rewrite schema/state.** Durable reusable lessons/corrections matter; current review/trust machinery is too granular. |
| `learning_context.rs` | **Merge into Brief.** No separate context product. |
| `lib.rs` | **Refactor after migration.** Public exports should shrink around the continuity core rather than re-export every historical capability. |
| `memory_compiler.rs` | **Simplify/defer automation.** Keep only evidence-backed consolidation needed for handoffs; no background compiler complexity before benchmark proof. |
| `memory_health.rs` | **Merge into project Status.** Diagnostics survive, standalone ontology does not. |
| `memory_transition.rs` | **Rebuild.** Replace shape-specific recovery verifier/commit families with generic event/handoff validation. |
| `policy_bundle.rs` | **Delete/defer.** No initial persistent policy-bundle layer. |
| `private_state.rs` | **Keep responsibility, rewrite around new DB/blob roots.** Platform-private state remains essential. |
| `project_activity.rs` | **Merge into session timeline/Search.** Useful presentation, not independent architecture. |
| `project_catalog.rs` | **Keep concept as DB table.** Project discovery/status is useful; custom registry is not. |
| `project_memory_search.rs` | **Keep/rebuild.** Historical recall is core; candidate generation/ranking changes substantially. |
| `resume_context.rs` | **Replace with Brief.** Same user need, smaller task-conditioned product. |
| `retrieval.rs` | **Keep evidence-read primitives, simplify.** Exact cited evidence retrieval remains valuable. |
| `revision.rs` | **Keep.** Lightweight Git lineage/applicability is one of the strongest evidence-backed safety features. |
| `runbook.rs` | **Delete/defer.** Host-native Skills already exist; historical memory should not become instructions by default. |
| `semantic_retrieval.rs` | **Defer pending ablation.** Architecture should support optional vectors without requiring the bundled model. |
| `session.rs` | **Keep core responsibility, rewrite storage/event schema.** Sessions/handoffs remain fundamental. |
| `session_context.rs` | **Merge into Brief/Evidence.** Avoid another product projection. |
| `specification.rs` | **Keep only minimal privileged-source approval.** Selected path/hash and reapproval after change; discard the wider object hierarchy unless re-earned. |
| `topic_dossier.rs` | **Delete as a named feature.** Search/Brief can produce the useful view on demand. |

### Other Rust crates and native shell

| Area | Reset disposition |
| --- | --- |
| `ley-cli` | **Keep, radically shrink.** Setup/status/brief/search/evidence/checkpoint/export/erase/MCP are enough for the first product; migration/debug commands can be internal. |
| `ley-mcp` | **Keep, radically shrink.** Expose a few goal-level tools rather than the current internal operation catalog. |
| `ley-github-connector` | **Defer/remove from core distribution.** Implementation is careful but overlaps host-native GitHub access. |
| `ley-semantic-installer` | **Defer with semantic feature.** Retain only if vector ablation earns the model. |
| `src-tauri` | **Keep as focused local control shell, reduce command surface.** Retire generic note-vault filesystem commands after migration. |

### Frontend feature families

| Feature directory | Reset disposition |
| --- | --- |
| `agent-memory` | **Keep the useful workflows, redesign into focused continuity UI.** Split the 4k+ line workspace into project/status/brief/search/review/evidence/privacy surfaces. |
| `backlinks` | **Delete from primary Ley.** General note-app feature. |
| `bookmarks` | **Delete from primary Ley.** General note-app feature. |
| `canvas` | **Delete from primary Ley.** General note-app feature. |
| `collections` | **Delete from primary Ley.** General note-app feature. |
| `commands` | **Keep only focused continuity commands.** Remove generic notebook command palette actions with retired features. |
| `editor` | **Delete from primary Ley.** Open user-authored files in the user's editor instead. |
| `favorites` | **Delete from primary Ley.** General note-app feature. |
| `graph` | **Delete note graph.** Any future evidence/provenance visualization should be purpose-built and accessible. |
| `history` | **Delete note revision UI; keep continuity/session history in the new focused UI.** |
| `notes` | **Delete as a primary app domain.** User-owned Markdown remains interoperable external data. |
| `outline` | **Delete from primary Ley.** Editor/notebook concern. |
| `search` | **Replace with continuity Recall/Search.** Do not preserve the note query language by default. |
| `settings` | **Keep, simplify.** Integration/privacy/storage/export settings only. |
| `sidebar` | **Rebuild for focused app.** No note-file explorer requirement. |
| `vault` | **Delete note-vault product model.** Replace with project/app-data model. |
| `workspaces` | **Delete from primary Ley.** General productivity/notebook feature. |

### Frontend core/infrastructure

| Area | Reset disposition |
| --- | --- |
| `src/core/graph` | **Delete/defer with note graph.** |
| `src/core/index` | **Delete/rebuild.** Note index/search becomes engine-backed historical recall/status. |
| `src/core/parser` | **Mostly retire.** Keep only any Markdown/source parsing needed for explicit privileged files/export. |
| `src/core/vault` | **Retire.** Separate notebook vault is no longer the central app domain. |
| `src/infrastructure/database` | **Replace Dexie notebook index with thin UI/cache state where needed.** Durable engine state moves to Rust/SQLite. |
| `src/infrastructure/vault` | **Retire browser/native note-vault adapters after migration.** |
| `src/shared/*` | **Keep reusable UI/hooks/lib/state selectively.** Delete utilities whose only consumers are retired note features. |
| `src/website` | **Keep/rewrite positioning.** Website remains; notebook-first copy does not. |

### Evaluation files

| File | Reset disposition |
| --- | --- |
| `run_agent_task_eval.py` | **Keep/expand; becomes primary product-value evidence lane.** Add realistic tasks and simpler baselines/ablations. |
| `run_eval.py` | **Keep the best adversarial fixtures, then split/shrink.** Do not preserve 15k lines merely for deleted concepts. |
| `run_git_revision_compat_eval.py` | **Keep.** Strong bounded portability evidence for revision safety. |
| `run_mcp_host_compat_eval.py` | **Keep, update for small tool surface and real packaged integration smoke.** |
| `run_private_config_permissions_eval.py` | **Keep intent, rewrite for SQLite/app-data permissions when storage migrates.** |
| `run_semantic_eval.py` | **Keep only while semantic option is under ablation.** |
| `test_agent_task_eval.py` | **Keep/expand with the real-agent runner.** |
| `fixtures/scenarios.jsonl` | **Prune/rewrite around retained invariants.** Archive scenarios for deleted features. |
| `fixtures/agent_tasks.jsonl` | **Expand substantially.** Current two tasks are insufficient. |
| `fixtures/semantic_model_scenario.json` | **Retain only for semantic ablation.** |

### Public schemas

| Schema | Reset disposition |
| --- | --- |
| `checkpoint-input.schema.json` | **Replace with simpler checkpoint/handoff contract.** |
| `learning-event.schema.json` | **Migrate to generic event/memory schema.** |
| `learning.schema.json` | **Migrate to simpler durable-memory/export schema.** |

The new SQLite schema is an internal persistence contract and does not need every internal table exposed as
public JSON. Public schemas should describe stable import/export or agent API payloads only.

### Integrations and packaging

| Area | Reset disposition |
| --- | --- |
| Codex plugin | **Keep/repackage.** Move toward the current portable Agent Plugins layout, shrink Skill and MCP tools, keep only useful hooks. |
| Claude Code plugin | **Keep/repackage around the same small provider-neutral behavior.** Avoid two independently drifting workflow manuals. |
| root/integration marketplace manifests | **Consolidate release/version generation.** One source of version/description truth. |
| hook configurations | **Keep bounded lifecycle capture; make automatic context injection explicit/toggleable.** |
| large provider Skills | **Rewrite tiny.** Teach user goals, not Ley's internal recovery ontology. |

### Root/build/release/docs

| Area | Reset disposition |
| --- | --- |
| Cargo workspace | **Keep, then remove crates that remain unearned after migration.** Add Rust toolchain pin. |
| npm/Vite/Tauri frontend stack | **Keep only for focused desktop + website.** Remove PWA/notebook dependencies as their consumers disappear. Add Node pin. |
| `.github/workflows/portability-security.yml` | **Keep manual six-lane portability/security evidence.** Add ordinary CI separately. |
| README / website product copy | **Reposition after focused engine/UI exists; meanwhile clearly label migration.** |
| ADRs | **Archive as historical evidence after each subsystem migrates.** Do not rewrite history to pretend the old choices never existed. |
| research/runtime docs | **Prune/archive stale product assumptions; keep empirical evidence.** |
| `ref` clone assumptions | **Removed.** External current sources replace stale local clone authority. |

This appendix completes the audit inventory. Any tracked implementation not named individually is either a
test/helper/asset subordinate to one of the classified areas above or generic shared/build material whose
retention follows its remaining consumers.
