# Ley architecture

## Product boundary

Ley ships one product through two runtimes:

- Tauri desktop is the primary, technically complete runtime. It owns unrestricted user-selected filesystem vaults through a small Rust command surface.
- The web application is an installable PWA. Chromium-family browsers can grant a directory handle for a real folder vault; other browsers receive an explicit IndexedDB compatibility mode.

The public website and workspace share the same Vite build but have separate entry routes. `/` is the website and `/app` is the workspace; Tauri always mounts the workspace.

## Source boundaries

| Directory | Responsibility | May depend on |
| --- | --- | --- |
| `src/core` | Markdown parsing, links, graph construction, indexes, vault operations | data types and small shared utilities |
| `src/features` | Complete user-facing capabilities | core, infrastructure gateways, shared |
| `src/infrastructure` | Dexie persistence and native/browser filesystem integration | core parsers/index builders, shared |
| `src/app` | Runtime initialization and feature composition | all application modules |
| `src/shared` | Product-agnostic components, hooks, state, and utilities | third-party primitives only |
| `src/website` | Public site | shared presentation primitives where useful |
| `src-tauri` | Confined native filesystem operations and desktop packaging | Rust/Tauri only |

Feature-specific hooks are colocated with their feature. Only genuinely reusable hooks belong in `shared/hooks`. Tests for pure domain behavior remain beside the module they verify.

Linux development builds emit the native executable plus `.deb` and `.rpm` packages. AppImage packaging is reserved for an Ubuntu-based release runner because the upstream linuxdeploy GTK plugin assumes a legacy gdk-pixbuf module layout that current Arch Linux no longer ships.

## Persistence contract

For filesystem vaults, Markdown is authoritative. A vault scan projects files into Dexie, then rebuilds links and tags after every page exists so forward references resolve correctly. First scans derive IDs from vault identity and relative path; subsequent scans preserve the current ID for each matching path so in-session creations, renames, tabs, and revisions remain coherent.

The active `pages`, `assets`, and `revisions` tables represent exactly one vault at a time. Before a browser-local vault yields the active projection to a filesystem folder, Ley snapshots its authoritative pages, binary assets, and revision history into dedicated browser-local tables. Returning restores that snapshot and rebuilds disposable links/tags. Filesystem rescans preserve existing IDs by path for the same vault, while changing vaults clears cache-only assets and history so state cannot leak across vault boundaries.

Writes follow this order:

1. Serialize YAML properties and Markdown body.
2. Persist the file through the active vault adapter. Desktop writes stage and atomically rename a temporary file.
3. Update the local page projection.
4. Rebuild backlinks, tags, search, and graph-derived state.

Renames move the file and retarget incoming wiki links. Folder moves preserve note identity and titles, so links remain valid; duplicates receive an independent title/path and intentionally drop aliases to avoid ambiguous resolution. Deletes move filesystem notes into `.trash`; browser-local notes use a recoverable soft-delete marker and expose restore/permanent-delete controls. Binary attachments live under `attachments/` and are constrained by extension, size, and safe relative paths. Interoperable JSON Canvas documents live under `canvases/` and use the same atomic-write and trash lifecycle. All native relative paths reject absolute paths and traversal segments.

## Derived state

Dexie tables for pages, blocks, links, tags, and settings support reactive UI queries. FlexSearch and the synchronous title resolver subscribe to Dexie and are disposable. Revisions are sparse checkpoints rather than keystroke logs and are restored through the same normal save path.

## Design system

Ley uses self-hosted Geist Sans and Geist Mono with semantic CSS tokens. Radix primitives provide accessible interaction foundations where native controls are insufficient. Feature components consume shared buttons, inputs, keyboard hints, and tokens rather than hard-coded brand values.

## Non-goals

- A proprietary document format
- Mandatory cloud accounts or sync
- Treating the graph as the primary editor
- Claiming browser-local storage is equivalent to a filesystem vault
- A plugin API before the core file format and lifecycle are stable
