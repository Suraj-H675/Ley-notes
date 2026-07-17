# Ley architecture

## Product boundary

Ley ships one product through two runtimes:

- Tauri desktop is the primary, technically complete runtime. It owns unrestricted user-selected filesystem vaults through a small Rust command surface.
- The web application is an installable PWA. Chromium-family browsers can grant a directory handle for a real folder vault; other browsers receive an explicit IndexedDB compatibility mode.

The public website and workspace share the same Vite build but have separate entry routes. `/` is the website and `/app` is the workspace; Tauri always mounts the workspace.

All surfaces follow the [local storage and data boundaries](privacy-and-storage.md). Application assets may be delivered from the web, while knowledge data remains on-device. Context is shown to a cloud agent only when the user intentionally asks that agent to retrieve it.

## Source boundaries

| Directory            | Responsibility                                                         | May depend on                               |
| -------------------- | ---------------------------------------------------------------------- | ------------------------------------------- |
| `src/core`           | Markdown parsing, links, graph construction, indexes, vault operations | data types and small shared utilities       |
| `src/features`       | Complete user-facing capabilities                                      | core, infrastructure gateways, shared       |
| `src/infrastructure` | Dexie persistence and native/browser filesystem integration            | core parsers/index builders, shared         |
| `src/app`            | Runtime initialization and feature composition                         | all application modules                     |
| `src/shared`         | Product-agnostic components, hooks, state, and utilities               | third-party primitives only                 |
| `src/website`        | Public site                                                            | shared presentation primitives where useful |
| `src-tauri`          | Confined native filesystem operations and desktop packaging            | Rust/Tauri only                             |
| `crates/ley-core`    | Identity, binding, ingestion, graph, retrieval, and session engine      | capability I/O and deterministic parsers    |
| `crates/ley-mcp`     | Fixed-project stdio retrieval and opt-in session capture                | `ley-core`, official Rust MCP SDK            |
| `crates/ley-cli`     | Human/agent command surface and MCP process entry point                 | `ley-core`, `ley-mcp`                        |

Feature-specific hooks are colocated with their feature. Only genuinely reusable hooks belong in `shared/hooks`. Tests for pure domain behavior remain beside the module they verify.

Linux development builds emit the native executable plus `.deb` and `.rpm` packages. AppImage packaging is reserved for an Ubuntu-based release runner because the upstream linuxdeploy GTK plugin assumes a legacy gdk-pixbuf module layout that current Arch Linux no longer ships.

## Agent-memory projection

An initialized repository keeps only portable identity, capture consent, and ignore rules in `.ley/`. A private OS-local registry binds that stable project ID to a filesystem vault. Approved repository evidence is stored as redacted content-addressed artifacts and immutable manifests beneath `<vault>/.ley/agent-memory/projects/<project-id>/`; no repository path is written into the vault.

Ingestion derives a separate immutable project graph from the same post-redaction source boundary. Files, language symbols, calls, imports, inheritance, implementations, manifest dependencies, and bounded local Git state carry artifact snapshot citations. Direct syntax stays deterministic; v1 deliberately leaves names unresolved across files instead of promoting heuristic matches into facts. Each graph and artifact current pointer is atomically replaced under one shared cross-process lock, and both are verified against immutable snapshots before use.

Read-only retrieval opens that store without creating or changing it, holds a shared project lock while verifying the current artifact and graph against their immutable snapshots, and refuses inconsistent pointers. The MCP server is scoped at process startup to one initialized project and one resolved binding. Tool arguments contain no project or vault selector, so an agent cannot use a running server to enumerate or cross into another project. Search, evidence, graph, and session results are bounded; every serialized result also has a 256 KB hard limit. Project evidence carries relative citations and snapshot times, while session resume packs report omitted checkpoints and truncation. Both boundaries label stored text as untrusted and distinguish it from current agent policy.

Structured session capture appends one immutable JSON event per start, checkpoint, or finish request beneath the same project namespace. A separate project lock serializes session writers. Stable request IDs make retries idempotent, while reused IDs with changed content fail. Checkpoints replace touched project paths with citations to the current approved artifact snapshot. Ley derives `session-v1.json` and reviewable `session.md` projections by replaying the event sequence, so a missing or interrupted projection does not erase history. Text is bounded and credential-redacted before persistence; raw transcripts are not captured.

MCP session writes are absent by default. `--allow-session-writes` enables only start, checkpoint, and finish routes at process construction, so an existing read-only configuration cannot gain authority from a tool argument or stored instruction. Write tools return compact receipts and delegate to the same session engine as the CLI.

## Persistence contract

For filesystem vaults, Markdown is authoritative. A vault scan projects files into Dexie, then rebuilds links and tags after every page exists so forward references resolve correctly. First scans derive IDs from vault identity and relative path; subsequent scans preserve the current ID for each matching path so in-session creations, renames, tabs, and revisions remain coherent.

The active `pages`, `assets`, and `revisions` tables represent exactly one vault at a time. Before a browser-local vault yields the active projection to a filesystem folder, Ley snapshots its authoritative pages, binary assets, and revision history into dedicated browser-local tables. Returning restores that snapshot and rebuilds disposable links/tags. Filesystem rescans preserve existing IDs by path for the same vault, while changing vaults clears cache-only assets and history so state cannot leak across vault boundaries.

Writes follow this order:

1. Serialize YAML properties and Markdown body.
2. Persist the file through the active vault adapter. Desktop writes stage and atomically rename a temporary file.
3. Update the local page projection.
4. Rebuild backlinks, tags, search, and graph-derived state.

Editor content writes are serialized per note. A rapid later save cannot be overtaken by an earlier filesystem/index rebuild, and leaving an editor flushes its pending debounced value before teardown. This keeps authoritative Markdown and every derived row on the same newest revision.

Cell-level property mutations also join the per-note mutation queue and merge against the newest projected frontmatter inside that queue. Two collection cells, the properties panel, and an editor autosave therefore cannot overwrite one another with stale object snapshots.

Renames move the file and retarget incoming wiki links. Ordinary relative Markdown links are resolved by vault path, indexed as backlinks and graph edges, and rewritten when either their source or target moves so their filesystem meaning remains stable. Folder moves preserve note identity and titles, so links and note bookmarks remain valid; duplicates receive an independent title/path and intentionally drop aliases to avoid ambiguous resolution. Deletes move filesystem notes into `.trash`; browser-local notes use a recoverable soft-delete marker and expose restore/permanent-delete controls. Binary attachments live under `attachments/` and are constrained by extension, size, and safe relative paths. Interoperable JSON Canvas documents live under `canvases/` and use the same atomic-write and trash lifecycle. Ley normalizes all JSON Canvas 1.0 text, file, link, and group nodes plus file subpaths, group background metadata, colors, edge sides, endpoint styles, and labels; malformed geometry and dangling edges are excluded before rendering or saving. Spatial edge paths are derived from document coordinates so resize, pan, zoom, reload, and cross-app imports do not depend on transient renderer measurements. All native relative paths reject absolute paths and traversal segments.

Workspace metadata is namespaced by the active vault identity. Desktop vaults use their full path, browser-local uses a dedicated identity, and browser directory handles receive a persisted random ID rather than relying on a potentially colliding folder name. Note bookmarks, destination bookmarks, saved searches, named workspace layouts, and navigation sessions therefore never cross vault boundaries. Saved searches are validated setting records rather than hidden note files, because they describe Ley workspace retrieval rather than user-authored knowledge. Their optional collection configuration contains presentation-only column IDs and sort direction; every displayed value is still read from Markdown/YAML. Heading and block bookmarks resolve a note by stable page ID and then path; heading text and explicit `^block-id` anchors provide the durable in-note destination. Creating a block bookmark writes that portable identity into authoritative Markdown instead of storing a fragile line offset. Missing-note bookmarks remain visible and recover automatically if the note returns. Navigation sessions store both stable page IDs and paths, restore tab order, primary and secondary panes, focused pane, and recents, and discard references to deleted or unavailable notes. Named layouts reuse that resilient note-reference contract and add presentation-only sidebar visibility, right-dock context, and split width; loading drops missing notes and refuses to replace the live arrangement when every reference is stale. Editor insert/jump events carry their page identity, and link opens carry their source pane, so two mounted editors cannot receive one another's commands.

The desktop runtime owns one recursive native watcher for the active vault. It emits only visible Markdown and JSON Canvas paths, ignores hidden/internal files, and suppresses events caused by Ley's own atomic writes. The web layer debounces event bursts before rescanning and reconciles tabs against the new projection. If an external edit arrives while the current CodeMirror document is dirty, autosave pauses and the user explicitly chooses the disk version or their local version; neither side wins silently.

## Derived state

Dexie tables for pages, blocks, links, tags, and settings support reactive UI queries. FlexSearch and the synchronous title resolver subscribe to Dexie and are disposable. CodeMirror's shared completion lifecycle reads the derived tag table on demand, so tag authoring reuses the current vault taxonomy without making the index authoritative. The search projection indexes title, aliases, content, tags, paths, and flattened YAML properties; structured operators are parsed separately and applied as composable post-filters so filter-only queries and exclusions remain exact. Collection tables reuse the same parsed filters against a live page/tag snapshot, discover columns from frontmatter coverage, and never persist row data outside the notes. Revisions are sparse checkpoints rather than keystroke logs and are restored through the same normal save path.

## Design system

Ley uses self-hosted Geist Sans and Geist Mono with semantic CSS tokens. Radix primitives provide accessible interaction foundations where native controls are insufficient. Feature components consume shared buttons, inputs, keyboard hints, and tokens rather than hard-coded brand values.

## Non-goals

- A proprietary document format
- Mandatory cloud accounts or sync
- Treating the graph as the primary editor
- Claiming browser-local storage is equivalent to a filesystem vault
- A plugin API before the core file format and lifecycle are stable
