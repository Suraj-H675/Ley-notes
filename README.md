# Ley

Ley is being rebuilt as a **local, trustworthy continuity and context layer for coding agents**.

A repository tells an agent what exists now. Ley is for the information that usually disappears between sessions: what was decided, what was attempted, what failed, what was actually verified, what remains unresolved, and where that evidence came from.

The 2026-09-25 first-principles reset is documented in [`docs/research/first-principles-audit-2026-09-25.md`](docs/research/first-principles-audit-2026-09-25.md). Existing ADRs and feature documents remain useful implementation history, but they are no longer automatically current requirements.

## Product surfaces

Ley now has exactly two intended surfaces:

- **Ley Desktop** — the actual local application. It owns project access, local agent integrations, privacy controls, review, search/recall, evidence inspection, and the continuity workflow.
- **Ley website** — a normal public marketing/showcase/documentation site. It does not run a reduced copy of Ley in the browser.

The previous `/app` browser workspace, PWA, browser-folder mode, and browser-local notebook mode have been retired. A web page cannot provide the same local project/MCP boundary as the native product, and maintaining a parallel notebook implementation added substantial complexity without strengthening Ley's differentiated value.

## Direction

The target agent-facing contract is intentionally small:

- **Brief** — the smallest cited continuity pack useful for the current task.
- **Search** — explicit bounded historical recall.
- **Evidence** — exact provenance/source drill-down.
- **Checkpoint** — one structured durable write path for meaningful session state.

The target human-facing desktop is a focused control center for project setup/status, integrations, brief preview, recall, session/handoff history, review/correction, evidence, privacy, export, and erasure.

General-purpose note editing, backlinks, Canvas, daily notes, browser-local storage, and other notebook features still present in parts of the migration tree are being retired rather than expanded.

## Important principles

- current user intent and live project evidence outrank historical memory;
- historical agent text is context/evidence, not instruction or truth;
- durable memory preserves provenance;
- project and cross-project scope must be explicit;
- context is small and task-conditioned by default;
- privacy, egress, erasure, and export remain inspectable;
- Git revision/applicability stays lightweight and visible;
- derived indexes/views are rebuildable;
- advanced features must beat a simpler baseline in realistic agent-task evaluation before they earn their complexity.

See [`LEY.md`](LEY.md) in a local development checkout for the current execution direction and [`docs/README.md`](docs/README.md) for documentation status.

## Repository map

```text
.
├── crates/               # Rust core, CLI, MCP, and supporting adapters
├── desktop/              # Dedicated native frontend HTML entry
├── docs/                 # Current docs plus historical ADR/research evidence
├── eval/                 # Deterministic and real-agent evaluation runners
├── integrations/         # Codex and Claude Code packages
├── schemas/              # Stable public/import-export payload contracts
├── src/
│   ├── app/              # Native desktop React composition
│   ├── core/             # Legacy note domain + migration-era shared logic
│   ├── features/         # Desktop feature slices
│   ├── infrastructure/   # Desktop projection/cache adapters
│   ├── shared/           # Reusable UI/state/utilities
│   └── website/          # Public marketing site
└── src-tauri/            # Native shell and local filesystem/project commands
```

The website and desktop share reusable source code where useful, but they have **separate entrypoints and build artifacts**. A website deployment cannot expose the desktop application runtime.

## Development

Requirements today: Node.js 22+, Rust stable, and the Tauri 2 platform prerequisites for your operating system. Explicit checked-in toolchain pins are planned as part of the reset.

```bash
npm install

npm run dev:website       # public marketing/showcase site
npm run build:website

npm run desktop           # Tauri development app
npm run build:desktop-ui  # desktop webview frontend only
npm run desktop:build     # native application bundle

npm run typecheck
npm run lint
npm test
cargo check --locked --workspace
```

`npm run build` currently aliases the website production build.

## Migration status

This is an incremental reset rather than a blind rewrite. The current tree still contains substantial legacy notebook and Agent Memory implementation so the new design can be benchmarked and migrated without throwing away user data or hard-won safety work.

Work being retained or adapted includes:

- bounded credential redaction and evidence handling;
- stable project identity and idempotency patterns;
- structured sessions/handoffs;
- exact provenance/citation IDs;
- Git revision relation logic;
- privacy/egress/erasure tests;
- six-lane Linux/macOS/Windows x64/ARM64 portability evidence;
- host compatibility probes.

Machine-managed state is planned to move from many custom JSON registries toward transactional SQLite, while large immutable evidence may remain content-addressed files. The old browser IndexedDB stores are currently left inert rather than destructively dropped; their final import/export handling belongs to that migration.

## Evaluation before feature growth

New major product concepts are frozen until Ley has a stronger realistic downstream benchmark. At minimum the comparison should include:

1. host-native agent + repository tools/instructions only;
2. a concise human-authored `HANDOFF.md`;
3. minimal redesigned Ley;
4. the current full Ley implementation while it still exists.

The goal is not to maximize the number of memory features. The goal is to show that Ley helps real agents resume work more correctly, with less stale-context harm and less wasted context.
