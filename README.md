# Ley

Ley is a local-first knowledge workspace for durable Markdown notes, wiki links, backlinks, search, and visual graph exploration. It runs as a native desktop application and as an installable web app.

The desktop app and supported browsers open a real folder as a vault. Markdown files remain the source of truth; the local database is a rebuildable index for search, links, tags, graph data, and recovery snapshots.

## Product capabilities

- Real filesystem vaults on desktop, with atomic saves and a `.trash` folder
- Native recursive change watching for external Markdown/Canvas edits, with explicit conflict resolution for unsaved notes
- Browser folder vaults through the File System Access API, plus an explicit browser-local mode
- Folder-aware file explorer with safe move, drag/drop, duplicate, rename, trash, and browser-local restore workflows
- Safe vault switching and rescanning without mixing browser-local authority with filesystem cache data
- Markdown editing and reading views with interactive tasks, `[[wiki links]]`, portable relative `[Markdown](note.md)` links, precise heading/block navigation, and properties
- Keyboard and touch-accessible Markdown formatting with vault-aware completion for tags, notes, headings, and block references
- First-class in-note find/replace with match navigation, case, regular-expression, and whole-word controls
- Full and heading/block-scoped note embeds, pasted or dropped attachments, and safe vault-relative media rendering
- Automatic backlinks and graph edges for wiki and relative Markdown links, plus outgoing links, unlinked mentions, tags, and ghost-link resolution
- Full-text search with composable tag, path, title, YAML-property, quoted, and exclusion filters; quick switcher, command palette, daily notes, and keyboard navigation
- Unified vault-scoped bookmarks for notes, headings, stable Markdown blocks, and saved searches, with rename/deletion and responsive sidebar access
- Live query-backed property tables with typed sorting, configurable columns, inline YAML editing, split-note opening, and saved per-query layouts
- Resizable side-by-side note panes with pane-local linking, responsive focus, and vault-scoped restoration of tabs, panes, focus, and recents
- Named, vault-scoped workspace layouts that restore tabs, split panes, focus, sidebars, dock context, and divider width
- Global and contextual knowledge graphs with deterministic layouts and community coloring
- Interoperable JSON Canvas files with text, note, link, and group cards; resizing, colors, labeled directional connections, and trash recovery
- Vault-native templates for new notes and daily notes
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

Ley is under active development. Core note creation, editing, linking, embeds, attachments, templates, search, graph, JSON Canvas, folder vaults, and recovery are functional. Sync, a stable plugin API, and dedicated mobile clients remain future work.
