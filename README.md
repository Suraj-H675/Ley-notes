# Ley

Ley is a local-first knowledge workspace for durable Markdown notes, wiki links, backlinks, search, and visual graph exploration. It runs as a native desktop application and as an installable web app.

Knowledge data stays on the user's device. Ley has no note backend, mandatory account, analytics, or telemetry. Context is shared with a cloud agent only when the user intentionally asks that agent to retrieve it. See [Local storage and data boundaries](docs/privacy-and-storage.md).

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
- Full-text search with composable tag, path, title, YAML-property, Markdown task-state, quoted, and exclusion filters; quick switcher, command palette, daily notes, and keyboard navigation
- Unified vault-scoped bookmarks for notes, headings, stable Markdown blocks, and saved searches, with rename/deletion and responsive sidebar access
- Live query-backed property tables with typed sorting, configurable columns, inline YAML editing, split-note opening, and saved per-query layouts
- Resizable side-by-side note panes with pane-local linking, responsive focus, and vault-scoped restoration of tabs, panes, focus, and recents
- Named, vault-scoped workspace layouts that restore tabs, split panes, focus, sidebars, dock context, and divider width
- Global and contextual knowledge graphs with deterministic layouts and community coloring
- Interoperable JSON Canvas files with text, note, link, and group cards; resizing, colors, labeled directional connections, and trash recovery
- Vault-native templates for new notes and daily notes
- Sparse local revision snapshots with a user-facing recovery panel
- Desktop Agent Memory workspace with an explicit multi-project catalog, local cross-project search, capture/privacy controls, deterministic refresh, complete session history, bounded continuity briefs, checkpoint/decision/problem/attempt/outcome inspection, cited artifacts, trusted lessons, temporal provenance inspection, version-guarded corrections, promotion into ordinary Markdown, and append-only human review
- An honest browser Agent Memory boundary: the PWA keeps notes usable but does not pretend a web page can serve external local agents or read arbitrary coding projects
- Offline-capable PWA and a separate public website

## Repository map

```text
.
├── crates/               # Shared Rust core, `ley` CLI, and local stdio MCP server
├── docs/                 # Architecture, decisions, security, and product research
├── schemas/              # Versioned open agent-memory contracts
├── src/
│   ├── app/              # Application composition and workspace shell
│   ├── core/             # Framework-free Markdown, graph, indexing, and vault domain logic
│   ├── features/         # User-facing vertical slices (editor, search, graph, vault, ...)
│   ├── infrastructure/   # IndexedDB and native/browser filesystem adapters
│   ├── shared/           # Reusable UI, hooks, state, styles, and small utilities
│   ├── test/             # Shared test setup and fixtures
│   └── website/          # Public marketing site
├── src-tauri/            # Rust desktop shell and confined filesystem commands
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
cargo test --workspace   # local core, CLI, and desktop Rust tests
```

No account, backend, or environment file is required.

### Use Agent Memory

In Ley Desktop, open **Agent Memory** from the title bar or command palette, choose a project folder, then explicitly initialize or connect it to the active filesystem vault. Ley uses Structured capture by default, shows only engine-backed sessions and lessons, and keeps review actions under user authority. The project-level **Capture & privacy** surface previews the approved filesystem boundary and applies Minimal, Structured, or explicitly acknowledged Full Evidence retention through a real local re-capture. See [Capture and privacy](docs/agent-memory/capture-and-privacy.md). The browser app explains why this local-agent integration requires desktop instead of displaying fabricated or incomplete memory.

The same foundation is available without the GUI through the local CLI:

```bash
cargo run -p ley-cli -- init /path/to/project --capture structured
cargo run -p ley-cli -- bind /path/to/project --vault /path/to/ley-vault
cargo run -p ley-cli -- binding /path/to/project
cargo run -p ley-cli -- ingest /path/to/project
cargo run -p ley-cli -- graph /path/to/project
cargo run -p ley-cli -- session start /path/to/project \
  --name "First session" --goal "Ship cited memory"
cargo run -p ley-cli -- learning list /path/to/project --review
cargo run -p ley-cli -- resume /path/to/project
cargo run -p ley-cli -- mcp /path/to/project
cargo run -p ley-cli -- doctor /path/to/project
cargo run -p ley-cli -- preview /path/to/project
```

Initialization creates a minimal `.ley/` project identity, capture policy, and additional ignore rules. Structured capture is the default and does not enable raw transcripts. Repeating `init` reads the existing identity without changing its name or capture consent. `bind` stores only the stable project-ID-to-canonical-vault-path association in Ley's private OS application configuration; no machine path is written into the repository. A temporary `binding --vault /other/vault` override is validated but not persisted, and `unbind` removes the private association without touching project or vault data. `preview` deterministically lists the regular files that fit the approved roots and byte limits without reading their contents; ignored files and symlink targets are not captured. `ingest` performs a real incremental capture into the bound vault: Structured/Full Evidence modes retain redacted UTF-8 evidence, Minimal retains metadata only, identical runs are no-ops, and additions/changes/renames/deletions create immutable cited snapshots. It also projects cited repository structure, Tree-sitter symbols/calls/imports/inheritance, declared dependencies, and bounded local Git state. `graph` integrity-checks and reads that durable projection without rescanning source.

`session start`, `session checkpoint`, and `session finish` append structured, credential-redacted events to the bound vault. Checkpoints can retain plans, decisions, tasks, problems, attempts, outcomes, resolutions, cited touched artifacts, commands, verification, and unresolved work. Repeated request IDs are idempotent, concurrent writers are serialized, and session reads replay immutable events instead of trusting a mutable summary. Use `session list` and `session show` to inspect the result. See [Capture structured agent sessions](docs/agent-memory/sessions.md), [ADR 0007](docs/adr/0007-append-only-structured-sessions.md), and the versioned [checkpoint input schema](schemas/agent-memory/checkpoint-input.schema.json).

`learning propose` distills one or more existing session records into a project lesson. Every proposal remains tentative until an explicit user confirmation; agents can contest or mark stale but cannot grant trust, reject, or supersede memory. Corrections and feedback append immutable events, while source-hash drift returns trusted lessons to `learning list --review`. See [Review project learnings](docs/agent-memory/learnings.md), [ADR 0009](docs/adr/0009-evidence-backed-learning-ledger.md), and the versioned [learning event](schemas/agent-memory/learning-event.schema.json) and [projection](schemas/agent-memory/learning.schema.json) schemas.

`resume` produces one token-bounded startup pack: active and paused work, recent handoffs and unresolved items, plus only user-trusted lessons whose artifact citations still match the latest ingestion. It never checks live source or includes tentative/uncited advice. See [Resume a project](docs/agent-memory/resume.md) and [ADR 0011](docs/adr/0011-bounded-project-resume-context.md).

`mcp` starts a stdout-clean, read-only Model Context Protocol (MCP) server over standard input/output (stdio). The process is fixed to that one project and its explicit binding. It exposes a project overview, bounded lexical context search, cited evidence reads, graph traversal, recent session listing, compact session resume packs, and trusted-first project lessons; it cannot enumerate or switch to other projects. Retrieved project, session, and learning text is labeled as untrusted evidence, not live source or agent policy. Every serialized tool result has a 256 KB hard limit. Point an MCP host at the executable and project:

```json
{
  "mcpServers": {
    "ley": {
      "command": "/absolute/path/to/ley",
      "args": ["mcp", "/absolute/path/to/project"]
    }
  }
}
```

Add `--allow-session-writes` to the MCP arguments only when that host should append structured start, checkpoint, and finish events. Existing configurations remain read-only. The write tools require stable request IDs and return compact idempotent receipts; they use the same redaction, citation, locking, and event engine as the CLI.

Add the independent `--allow-learning-proposals` flag only when that host should suggest cited, review-required lessons. It adds no confirmation, correction, rejection, supersession, deletion, or promotion authority. See [ADR 0010](docs/adr/0010-explicit-mcp-learning-proposal-consent.md).

The host launches this local process when it needs context. If that host uses a cloud model, the context it deliberately retrieves can be sent to that provider. See [Using Ley with an agent](docs/agent-memory/mcp.md), [ADR 0006](docs/adr/0006-read-only-project-mcp.md), [ADR 0008](docs/adr/0008-explicit-mcp-session-write-consent.md), and the [agent-memory threat model](docs/security/agent-memory-threat-model.md).

## Data model

Filesystem-backed vaults are portable folders containing ordinary `.md` files. YAML frontmatter stores note properties. Dexie/IndexedDB holds derived indexes and recovery metadata; deleting it never invalidates the underlying filesystem vault.

Browser-local mode is a compatibility option for browsers without folder access. Its notes live in IndexedDB and can be exported as an Obsidian-compatible ZIP.

## Status

Ley is under active development. Core note creation, editing, linking, embeds, attachments, templates, search, graph, JSON Canvas, folder vaults, and recovery are functional. Sync, a stable plugin API, and dedicated mobile clients remain future work.
