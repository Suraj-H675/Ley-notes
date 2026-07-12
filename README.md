# Ley

Ley is a local-first knowledge workspace for durable Markdown notes, wiki links, backlinks, search, and visual graph exploration. It runs as a native desktop application and as an installable web app.

The desktop app and supported browsers open a real folder as a vault. Markdown files remain the source of truth; the local database is a rebuildable index for search, links, tags, graph data, and recovery snapshots.

## Product capabilities

- Real filesystem vaults on desktop, with atomic saves and a `.trash` folder
- Browser folder vaults through the File System Access API, plus an explicit browser-local mode
- Markdown editing and reading views with `[[wiki links]]`, aliases, heading links, and properties
- Automatic backlinks, outgoing links, unlinked mentions, tags, and ghost-link resolution
- Full-text search, quick switcher, command palette, daily notes, and keyboard navigation
- Global and contextual knowledge graphs with deterministic layouts and community coloring
- Sparse local revision snapshots with a user-facing recovery panel
- Offline-capable PWA and a separate public website

## Repository map

```text
.
├── src/
│   ├── app/              # Application composition and workspace shell
│   ├── core/             # Framework-free Markdown, graph, indexing, and vault domain logic
│   ├── features/         # User-facing vertical slices (editor, search, graph, vault, ...)
│   ├── infrastructure/   # IndexedDB and native/browser filesystem adapters
│   ├── shared/           # Reusable UI, hooks, state, styles, and small utilities
│   ├── test/             # Shared test setup and fixtures
│   └── website/          # Public marketing site
├── src-tauri/            # Rust desktop shell and confined filesystem commands
├── docs/                 # Architecture and product research
└── ref/                  # Local reference projects; intentionally not committed
```

See [docs/architecture.md](docs/architecture.md) for boundaries and persistence guarantees.

## Development

Requirements: Node.js 22+, Rust stable, and the Tauri 2 platform prerequisites for your operating system.

```bash
npm install
npm run dev              # website at / and browser app at /app
npm run desktop          # native desktop development shell
npm run lint
npm run test
npm run build            # web/PWA production build
npm run desktop:build    # native application bundle
```

No account, backend, or environment file is required.

## Data model

Filesystem-backed vaults are portable folders containing ordinary `.md` files. YAML frontmatter stores note properties. Dexie/IndexedDB holds derived indexes and recovery metadata; deleting it never invalidates the underlying filesystem vault.

Browser-local mode is a compatibility option for browsers without folder access. Its notes live in IndexedDB and can be exported as an Obsidian-compatible ZIP.

## Status

Ley is under active development. Core note creation, editing, linking, search, graph, folder vaults, and recovery are functional; advanced Obsidian-style canvases, embeds, and plugin extensibility remain future work.
