# Ley architecture during the Project Brain remake

This document separates **the current implementation foundation** from **the target Project Brain
architecture**. It must not imply that a target milestone already ships.

The product/domain contract lives in [`../LEY.md`](../LEY.md). The M0 repository audit and migration map is
[`research/project-brain-m0-reconciliation-2026-10-06.md`](research/project-brain-m0-reconciliation-2026-10-06.md).

## Target shape

Ley is moving toward one private logical **Project Brain** per software project:

```text
Project
  ├─ Sources ── SourceVersions ── evidence
  ├─ Repository/working-copy attachments
  ├─ Sessions ── observable Episodes ── evidence
  ├─ Human actions / reviewed claim versions
  └─ derived Entities / Assertions / relationships / summaries / indexes
                               ↓
                   bounded context retrieval
                               ↓
                       coding agents
```

The Project identity is stable and private. A path or `.ley/project.json` can associate a working copy with
the Brain but is not the Brain and does not grant private-state/capture/egress permission.

Canonical retained history and human decisions must survive rebuilding derived knowledge.

## Current implementation foundation

The implementation at the M0 branch point still reflects the preceding focused-continuity product.

### Product surfaces

The current runtime has:

1. **Ley Desktop** — native project/capture/continuity/integration/privacy UI;
2. **public website** — static product/marketing surface without project-memory runtime;
3. **legacy browser recovery page** — migration-only inspect/export/erase utility for retired browser data.

The future sparse Desktop IA is not implemented yet.

### Repository layout

- `crates/ley-core` — local domain/persistence/capture/retrieval/host logic;
- `crates/ley-cli` — current CLI and maintenance/transition flows;
- `crates/ley-mcp` — local stdio MCP server;
- `src-tauri` — native shell/runtime and Desktop host integration;
- `src/features/agent-memory` — current focused-continuity Desktop UI;
- `integrations/` — current Codex and Claude packages/hooks/skills;
- `eval/` — deterministic and real-agent evaluation harnesses.

### Current durable state

Current canonical machine-managed continuity uses one owner-private SQLite database (`continuity.sqlite3`,
schema v11 at M0), partitioned by stable project ID. Important structures include:

- projects;
- append-only/versioned events;
- event links;
- machine-local project observations;
- approved-source state;
- artifact snapshots/files/current snapshot state;
- migration/authority-cutover state.

Large retained captured evidence can live in Ley-managed private content-addressed storage.

The store already provides valuable production properties including transactions, request idempotency,
foreign-key/project qualification, ordering guards, private filesystem permissions, locking, bounded payloads,
WAL, and full synchronous writes in the current implementation.

These are foundations for M1, not a requirement to keep the existing schema shape.

### Current repo-local state

An initialized filesystem project currently stores small `.ley/` metadata:

- stable `project.json` identity;
- capture configuration;
- project-owned ignore rules.

Project Brain changes the meaning: a future repo-local ID associates a working copy with a private logical
Project. It must not contain the Brain database, conversations, secrets, embeddings, machine-private paths, or
permission grants.

### Current artifact capture

Current native ingestion already supports bounded/redacted capture, ignore rules, symlink/no-follow safety,
hashes, optional retained text/media evidence, and captured Git metadata. It does not build a new native code
graph by default.

Artifact snapshots are a useful precursor to SourceVersions but do not yet satisfy the new model:

- Source identity and path/locator identity are not separate;
- capture occurrence is not a first-class domain concept;
- retention currently favors current/cited snapshots rather than all intentionally retained Brain Sources;
- redacted retained content must be distinguished from exact original bytes.

M2 owns that implementation change after M1 establishes identity/persistence.

### Current Sessions and Learnings

Current Session events can retain structured goals, decisions, attempts, problems/resolutions, verification,
unresolved work, visible turn evidence, tool observations, and citations. Current Learning records add review,
correction, supersession, provenance, trust, and freshness behavior.

Those records remain valuable migration/evidence inputs. They do not define the final Chronicle or Assertion
ontology. In particular:

- structured checkpoints are not complete observable history;
- an old `trusted` state must not mechanically become new human acceptance;
- tool/agent statements remain reported evidence unless Ley observed the underlying result;
- derived knowledge must not overwrite exact human-reviewed claim versions.

### Current retrieval and MCP

The implemented canonical-native MCP surface is currently:

- `ley_brief`;
- `ley_search`;
- `ley_evidence`;
- optional `ley_checkpoint` when writes are explicitly enabled.

Those routes contain useful boundedness, citation, project-binding, egress, and write-gating behavior. Their
names are not frozen Project Brain vocabulary.

The target agent experience is the native Ley skill/invocation (`$ley` in Codex; appropriate native equivalent
elsewhere), while the engine continues to enforce capability boundaries independent of prompt instructions.

### Current host integrations

Codex and Claude integration already has strong reusable work:

- app-owned helper packaging;
- project-scoped explicit Connect/Disconnect;
- config ownership/conflict checks;
- lifecycle hooks;
- bounded visible turn/tool capture;
- stable Ley session association;
- separation of host detection, configuration, review/restart, and observed runtime activity.

The future Chronicle may capture more supported observations, but it must remain adapter/version aware and expose
gaps rather than promise complete transcripts or hidden reasoning.

## Target domain/persistence boundaries

M1 must implement the smallest coherent model that can represent these without another identity reset:

### Project

Stable private logical Brain, able to exist with no live repository.

### Working-copy/repository attachment

The initial product allows zero or one logical code-repository attachment per Project. That repository can have
multiple explicitly authorized machine-local working-copy/worktree locators, each with independent locator/revision
identity and authorization state. Multiple-copy/worktree conflicts must fail closed or require explicit resolution;
copied markers are not grants. Multi-repository Projects are deferred.

### Source

Stable project-owned input identity independent of locator or current bytes.

### SourceVersion

Immutable retained representation of a Source, carrying content identity plus transformation/retention metadata.

### Capture/import occurrence

Canonical observation that a specific SourceVersion was observed/imported, including relevant actor/session/scope
and omissions.

### Session and Episode

Session identifies a host work session. Episode is one supported observable occurrence. Inferred semantic grouping
is derived rather than fabricated as canonical chronology.

### Evidence reference

Exact reference to retained SourceVersion or Session/Episode evidence. Integrity does not prove entailment.

### Reviewed claim target / human action

When a human accepts/rejects/corrects/supersedes machine interpretation, the exact reviewed version and evidence
dependencies become canonical durable history along with the human action. A caller-provided `actor=user` label is
not authority; the action must originate through a human-control boundary not exposed as ordinary agent write
authority.

### Derived knowledge

Entities, unreviewed Assertions, inferred relationships, summaries, retrieval indexes, embeddings, and graph
layout remain rebuildable unless an exact version became the target of a durable human decision.

## Applicability model

Knowledge should not use one overloaded trust/freshness value.

Keep separately:

- evidence basis (observed / reported / inferred);
- producer/origin;
- adoption (proposed / accepted / rejected / superseded);
- applicability scope (project/source/version/revision/branch/worktree/etc.);
- applicability assessment for a context/time (current / stale / contradicted / unknown).

Git ancestry is useful revision evidence, not semantic authority.

## Import boundary

Target import has two distinct phases:

1. **deterministic local import** — creates/updates the Brain and retained approved Sources without any model;
2. **optional agent analysis** — explicit egress to a selected coding-agent/provider scope, producing candidate
   knowledge only.

The Project must remain useful if phase 2 never runs.

## Derived state and retrieval

Start with exact filters + lexical retrieval + typed relationship traversal where earned + time/source/revision
applicability + bounded task-conditioned assembly.

Semantic embeddings/model reranking are optional rebuildable indexes. They return only if controlled downstream
evaluation materially beats the simpler baseline under the same evidence/privacy/budget constraints.

## Compatibility architecture

Historical stores/registries/readers survive only while a concrete migration, cleanup, replay, erasure, or
restrictive privacy-ancestry obligation exists. Relevant categories are recorded in the M0 reconciliation.

Do not add new product features to legacy binding, Context Mount, Knowledge Scope, Policy Bundle, connector,
old graph, or shape-specific recovery architecture. Do not delete their remaining safety/migration paths until
the supported historical state has an explicit verified destination or removal contract.

## Architecture invariants that survive the reset

- local core operation without mandatory cloud inference;
- explicit project/source selection and isolation;
- owner-private machine state;
- bounded/redacted/no-follow local capture;
- provenance survives derivation;
- historical text is evidence, not executable instruction;
- human authority cannot be self-granted by an agent;
- stale/contradictory/unknown applicability stays visible;
- egress and erasure fail closed;
- retries/concurrent writers cannot silently duplicate or overwrite state;
- omission/truncation is disclosed when it matters;
- user-owned source files remain outside Brain erasure unless explicitly selected under a different contract;
- derived indexes/views never become a second mutable source of truth.

## Verification direction

Current deterministic tests/evals remain transition confidence. New milestones must additionally prove the two
Project Brain workflows:

1. cold fresh-agent continuation recovers requirement, decision, failed approach/root cause, solution,
   verification, unresolved work, applicability, and evidence—and avoids repeating the historical failure;
2. a sources-only Brain later guides a new/empty code repository from explicitly accepted sources without treating
   every imported reference as authoritative.

Passing legacy tests cannot veto a deliberate contract change, but behavior still intended to survive must retain
meaningful regression coverage.
