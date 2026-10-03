# Ley

Ley is a **local, trustworthy continuity and context layer for coding agents**.

A repository tells an agent what exists now. Ley is for the information that usually disappears between sessions: what was decided, what was attempted, what failed, what was actually verified, what remains unresolved, and where that evidence came from.

The 2026-09-25 first-principles reset is documented in [`docs/research/first-principles-audit-2026-09-25.md`](docs/research/first-principles-audit-2026-09-25.md). Existing ADRs and feature documents remain useful implementation history, but they are no longer automatically current requirements.

## Product surfaces

Ley now has exactly two intended surfaces:

- **Ley Desktop** — the actual local application. It owns project access, local agent integrations, privacy controls, review, search/recall, evidence inspection, and the continuity workflow.
- **Ley website** — a normal public marketing/showcase/documentation site. It does not run a reduced copy of Ley in the browser.

The previous `/app` browser workspace, PWA, browser-folder mode, and browser-local notebook mode have been retired. A web page cannot provide the same local project/MCP boundary as the native product, and maintaining a parallel notebook implementation added substantial complexity without strengthening Ley's differentiated value.

## Direction

The canonical agent-facing contract is intentionally small:

- **Brief** — the smallest cited continuity pack useful for the current task.
- **Search** — explicit bounded historical recall from the active project by default or one exact
  deliberately selected already-observed project; selection is request-scoped, not ambient sharing.
- **Evidence** — exact provenance/source drill-down from the project-qualified citation returned by Ley.
- **Checkpoint** — one structured durable write path for meaningful session state.

The human-facing desktop is a focused control center for project setup/status, brief preview, recall, session/handoff history, review/correction, evidence, privacy, export, and integration-facing continuity controls.

General-purpose note editing, backlinks, Canvas, daily notes, and the retired notebook runtime have been removed from the shipped product tree. Historical browser-local notebook data is handled only by an explicit same-origin recovery page that can inspect, export, or erase the retired IndexedDB after user action; the normal website and Desktop do not open it.

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
│   ├── features/         # Focused continuity desktop feature slices
│   ├── infrastructure/   # Historical local-data compatibility schema
│   ├── shared/           # Reusable UI/state/utilities
│   └── website/          # Public marketing site
└── src-tauri/            # Native shell and local filesystem/project commands
```

The website and desktop share reusable source code where useful, but they have **separate entrypoints and build artifacts**. A website deployment cannot expose the desktop application runtime.

## Development

Requirements today: Node.js **24.21.0** and Rust **1.98.1**, pinned by `.node-version` and
`rust-toolchain.toml`, plus the Tauri 2 platform prerequisites for your operating system. With `rustup`
installed, the repository toolchain file selects/installs the pinned Rust toolchain automatically.

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

Routine GitHub Actions CI runs on pushes to `main` and pull requests. It continuously checks the frontend
typecheck/lint/tests/website+desktop-UI builds/production dependency audit and the pinned Rust workspace
format/check/tests on Ubuntu. The manual six-lane portability/security workflow remains separate because
its Linux/macOS/Windows x64+ARM64 matrix is intentionally more expensive and evidence-oriented.

## Migration status

This is an incremental reset rather than a blind rewrite. The retired notebook implementation has been removed. Remaining compatibility code is kept only where it still protects migration, recovery, privacy/erasure, or historical local data while the focused continuity surface is narrowed.

Work being retained or adapted includes:

- bounded credential redaction and evidence handling;
- stable project identity and idempotency patterns;
- structured sessions/handoffs;
- exact provenance/citation IDs;
- Git revision relation logic;
- privacy/egress/erasure tests;
- six-lane Linux/macOS/Windows x64/ARM64 portability evidence;
- host compatibility probes.

Canonical artifact/session/learning/approved-source continuity now lives in transactional SQLite. A smaller set of legacy JSON registries remains only where it still carries migration, cleanup, or privacy/egress ancestry; those records no longer expand the canonical agent context surface. Large immutable evidence may remain content-addressed files. Old browser IndexedDB bytes remain in the user's browser profile until the user chooses the migration-only same-origin recovery page to export or erase them; they are no longer part of the product runtime.

## Evaluation before feature growth

The Phase-0 comparison is complete for the reset decision. A pinned `gpt-6-luna` / `xhigh` study compared
no history, a human `HANDOFF.md`, a tiny historical brief, and current/full Ley across four controlled
continuity tasks. After fixing two evaluator artifacts and replacing the affected observations, the final
sample contains 48 valid attempts with three observations per task/arm cell and zero runner failures.

The result does **not** justify preserving the current architecture wholesale. It earns a narrower set of
capabilities: structured verification/evidence and explicit selected-source context; bounded event evidence
and lightweight revision applicability remain cheap safety/provenance primitives. Persistent mount/scope/
policy graphs, shape-specific recovery APIs, semantic/vector retrieval, full source graphs/capture, and
other legacy breadth still have to beat simpler replacements before they return.

See [`docs/research/phase0-frontier-agent-benchmark-2026-09-26.md`](docs/research/phase0-frontier-agent-benchmark-2026-09-26.md)
for methodology, corrected results, discarded evaluator artifacts, limits, and migration implications.
