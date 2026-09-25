# Current architecture direction

This document describes the architecture selected by the 2026-09-25 first-principles reset. Older ADRs and runtime reports remain historical evidence; they are not automatically constraints on the design below.

See [`research/first-principles-audit-2026-09-25.md`](research/first-principles-audit-2026-09-25.md) for the full rationale and subsystem-by-subsystem disposition.

## Product boundary

Ley has two intended surfaces:

1. **Native desktop application** — the real product. It may access user-selected local projects, run local integrations/MCP, manage continuity state, and expose privacy/review/evidence controls.
2. **Public website** — static product information, documentation, and project/release links. It holds no project memory and does not mount the desktop workspace.

There is no supported browser Ley application. The former PWA, `/app` route, browser-folder File System Access mode, and browser-local IndexedDB vault were retired because they could not provide the core local-agent boundary and duplicated substantial storage/UI behavior.

## Build boundary

The website and desktop may share components/styles, but they do not share a runtime entrypoint or deployment artifact.

- `index.html` + `src/website-main.tsx` + `vite.config.ts` build the public website into `dist/`.
- `desktop/index.html` + `src/desktop-main.tsx` + `vite.desktop.config.ts` build the Tauri webview into `dist-desktop/`.
- `src-tauri/tauri.conf.json` points only at the desktop build.

This prevents a website deployment from accidentally exposing the desktop runtime and lets each surface evolve without compatibility branches for the other.

## Target continuity architecture

The migration target is a small local continuity engine rather than a general note system.

### Project identity

Keep a tiny repo-local `.ley/` identity/config so a project can retain a stable Ley identity across path moves. It may contain explicit capture/retention choices, but not conversations, generated memory, credentials, embeddings, or machine-specific private paths.

### Machine-managed state

Move machine-managed metadata toward one owner-private SQLite database, initially device-wide and partitioned by project. Candidate durable tables include:

- projects and integration state;
- session/event records;
- compact handoff/memory records;
- evidence/provenance references;
- corrections/supersession/pins;
- selected privileged source approvals;
- privacy/egress/retention settings.

Large immutable evidence blobs may remain content-addressed files under owner-private application data. Search indexes, vectors, summaries, and presentation projections are rebuildable derivatives.

The current JSON registries and filesystem Agent Memory stores remain migration inputs, not architecture that must be preserved.

### Durable memory vocabulary

Prefer a small event vocabulary such as:

- goal / handoff state;
- decision or constraint;
- attempt + outcome;
- verification result;
- unresolved item;
- correction/supersession;
- evidence reference.

Use a generic event envelope with `kind` + `payload_version` rather than versioning an entire session projection every time a new feature shape appears.

### Retrieval

The target historical retrieval path uses independent candidate generators:

- SQLite FTS lexical retrieval;
- optional vector retrieval if ablation earns it;
- metadata/time/revision filters;
- merged/reranked candidates;
- compact task-conditioned final selection.

Explicit revision/supersession relations may exclude or demote historical records. Textual disagreement heuristics should disclose uncertainty rather than claim semantic truth adjudication.

### Agent interface

The desired agent API maps to user goals instead of internal storage operations. The target is approximately:

- Brief
- Search
- Evidence
- Checkpoint

Setup/status may remain a small separate surface if real integrations require it. User-only deletion, privileged-source approval, correction, and privacy mutation should not become ambient agent authority.

### Desktop UI

The focused desktop should converge on:

- projects/setup/integration status;
- preview of the next-agent Brief;
- historical recall/search;
- session/handoff timeline;
- review/correct/pin/delete workflows;
- Evidence / Why inspection;
- privacy/egress/retention/export/erasure.

The legacy Markdown editor, note graph, Canvas, bookmarks, daily notes, workspace layouts, and similar notebook features are migration-era code scheduled for retirement unless realistic evaluation proves a unique continuity need.

## Current transition state

The native app still contains the legacy filesystem-note workspace while the continuity engine is migrated. For that temporary workspace:

- Markdown/Canvas files in the selected desktop folder remain authoritative;
- Dexie is a rebuildable UI projection/cache;
- native file operations go through Tauri commands;
- one recursive watcher tracks external changes;
- browser storage fallbacks are removed;
- old browser-local Dexie tables remain inert until the SQLite migration defines explicit legacy-data handling.

Do not add new capabilities to the legacy note domain merely because it still exists during migration.

## Security boundaries retained through the reset

- user-selected project/file boundaries;
- no-follow/canonical containment for native filesystem access;
- bounded/redacted evidence capture;
- explicit project scope;
- human intent cannot be self-authorized by an agent;
- provenance survives derivation;
- historical memory never silently becomes current source truth;
- privacy/egress/erasure remain inspectable and testable;
- agent/MCP outputs stay bounded.

Where the current implementation violates one of these principles, migration does not excuse the defect; exposed legacy surfaces must be fixed or retired before release.

## Evaluation architecture

Internal invariants are necessary but not sufficient. The primary product evidence should compare realistic external-agent tasks across:

1. host-native baseline;
2. human `HANDOFF.md` baseline;
3. minimal redesigned Ley;
4. current full Ley while available.

Measure task correctness, stale-memory harm, repeated dead ends, evidence correctness, context tokens, latency, setup/review burden, and Ley tool-selection/intervention count.

Optional features return only when their ablation materially improves a relevant outcome.
