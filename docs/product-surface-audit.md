# Product surface audit

This audit treats screen space, attention, and keyboard shortcuts as costs. A control stays only when it supports a real, distinct workflow; incomplete controls are improved or removed instead of being left as decoration.

## Decision criteria

- **Keep** when the action is frequent, distinct, discoverable, and backed by real persistence.
- **Improve** when the capability is valuable but its placement, state, wording, accessibility, or feedback is weak.
- **Relocate or merge** when two controls perform the same job or a specialist action occupies prime space.
- **Remove** when a control is speculative, inert, misleading, or exposes implementation detail without user value.

## Public website and vault entry

| Surface | Decision | Result |
| --- | --- | --- |
| Website navigation | Keep | Product, principles, and architecture links support evaluation without pretending to be app features. |
| “Open the web app” | Keep prominent | It is the immediate, zero-install path into a functioning workspace. |
| Desktop call to action | Improve | Reworded to an honest local build path; no false download promise before signed release artifacts exist. |
| Open folder | Keep primary | This is the browser's honest equivalent of a filesystem vault. |
| Browser-local vault | Keep secondary | Clearly described as IndexedDB compatibility mode with Markdown ZIP portability, not as a filesystem vault. |
| Demo-content action | Remove | A second brain should begin as the user's space. Fresh local vaults now contain one useful Welcome note, not 25 product-demo pages. |

## Workspace shell

| Surface | Decision | Result |
| --- | --- | --- |
| Sidebar toggle | Keep | Essential for focus and narrow screens; icon has an accessible label and tooltip. |
| Quick switcher | Keep | Optimized for opening a known note; remains distinct from full-text search and commands. |
| New note | Improve | Kept in the sidebar and added to the top toolbar for discovery and mobile reach. |
| Daily note | Keep | A high-frequency capture workflow with configurable filename format and optional template. |
| Canvas | Keep | Opens real `.canvas` documents; no longer a placeholder. |
| Global graph | Keep | Valuable for exploration, not promoted as the primary editor. Duplicate ownership and shortcuts were removed. |
| Settings | Keep | Contains durable product configuration only; transfer controls appear only in browser-local mode. |
| Right dock toggle | Keep | Backlinks and outline are core second-brain context, while the dock remains dismissible. |
| Recent and Tags panels | Improve | Both are collapsible to prevent the sidebar becoming a permanently crowded stack. Tags now truly filter the page tree. |
| Favorites panel | Add | Important notes can be starred from the workspace, explorer, or command palette and remain one click away without changing Markdown or folder structure. Favorites are isolated per vault and survive trash/restore. |
| Saved searches panel | Add | Structured queries can be named from the quick switcher, reopened into the editable query surface, renamed, or deleted. The panel is collapsible, touch-accessible, and scoped to the active vault so recurring retrieval workflows become durable without altering Markdown. |
| Explorer context menu | Improve | Rename, move, duplicate, copy-wiki-link, and trash now operate on real vault state. Moving is also available through drag/drop, with the dialog retained as the keyboard-accessible path. |
| Folder organization | Improve | Existing folders are first-class destinations and nested paths can be created by moving or creating notes. Empty-folder controls remain absent because they add no durable information to browser-local storage. |

## Note workspace

| Surface | Decision | Result |
| --- | --- | --- |
| Tabs and tab close | Keep | Multiple working notes are a desktop knowledge-work expectation. Unsaved content is persisted by the editor lifecycle, while tab order, active note, and recents restore per vault after reload or a vault round trip. |
| Split workspace | Add | Any non-primary tab or explorer note can open beside the current note. Both panes remain fully editable and route links locally, the divider is pointer- and keyboard-resizable, an already-visible destination receives focus instead of a duplicate editor, and narrow screens show the focused pane without overflow. Pane identities, focus, and width survive reload. |
| Editable title | Keep | Renames the filesystem note and retargets incoming wiki links. |
| Edit / Read modes | Keep | Editing and composed reading are materially different tasks; note embeds resolve only in reading mode. |
| Formatting toolbelt | Add | Bold, italic, note-link, code, and task actions remain reachable on touch and short screens; desktop shortcuts use the same underlying Markdown transactions. |
| In-note find and replace | Add | CodeMirror owns scoped search, next/previous match navigation, replace/replace-all, case, regular-expression, and whole-word behavior. `Cmd/Ctrl+F` and the touch toolbelt open the same accessible panel. |
| Wiki-link completion | Improve | Official CodeMirror completion now owns positioning, keyboard precedence, and accessibility. `[[` completes notes, `[[Note#` completes real headings, and `[[Note#^` completes block IDs with source previews; fenced examples are excluded. Selecting a result inserts without navigating; normal click edits and Ctrl/Cmd-click follows. |
| Tag completion | Add | Typing `#` at an authoring-safe Markdown boundary reuses the live vault taxonomy, ranking prefix and nested-segment matches before usage count. Frontmatter, code, headings, URL fragments, and attributes are excluded. The touch toolbelt exposes the same completion lifecycle without inserting a duplicate hash. |
| Relative Markdown note links | Add | Portable `[label](../note.md#Heading)` and block links participate in reading/editor navigation, backlinks, outgoing links, graph edges, ZIP import, and automatic path maintenance across source/target moves. Missing files are reported without creating an unrelated title-based note. |
| Attach | Improve | Now writes real binary files via desktop, browser-folder, or browser-local adapters; paste and drag/drop share the same path. |
| Properties | Keep | YAML frontmatter remains portable and visible rather than hidden in proprietary metadata. |
| Backlinks, outgoing, unlinked mentions | Keep | Each answers a different relationship question and is computed from the vault index. |
| Outline | Keep | Necessary navigation for long notes. |
| History | Keep | Exposes sparse local recovery snapshots and restores through the normal save path. |
| Local graph | Keep | Shows immediate context; the maximize action opens the global graph without mounting a duplicate modal. |

## Search and commands

| Surface | Decision | Result |
| --- | --- | --- |
| Quick switcher | Keep | Title/path-oriented navigation. |
| Full-text search | Improve | Content-oriented retrieval now composes free text with repeated `tag:`, quoted `path:`, `title:`, YAML `property:key=value`/`[key:value]`, and negative filters. Filter chips and an inline syntax guide make the language discoverable; Enter opens normally and Shift+Enter preserves context by opening in split. |
| Saved queries | Add | Non-empty searches can be named without leaving the switcher. Saving the same query updates its existing bookmark instead of creating duplicates, and opening one restores the exact editable query. |
| Command palette | Keep | Action-oriented control surface; note results do not replace commands. |
| Keyboard shortcuts | Improve | One owner per global shortcut; duplicate graph listeners were removed. Buttons retain visible or tooltip discovery paths. |
| Modal focus | Improve | Search, commands, new-note, settings, graph, and canvas surfaces use an accessible dialog lifecycle with focus entry, trapping, Escape dismissal, and restoration. |

## Graph

| Surface | Decision | Result |
| --- | --- | --- |
| Search, tags, unresolved toggle | Keep | Real filters that change the rendered graph. |
| Color mode, arrows, link labels | Keep | Useful semantic display choices with immediate feedback. |
| Physics controls | Keep | Charge, link distance, collision, and centering all affect the simulation and have reset behavior. |
| Node count and zoom controls | Keep | Communicate scope and support navigation. |
| Disabled “Node size” slider | Remove | It did not change graph state. |
| Unused “Text fade threshold” | Remove | It exposed no meaningful behavior. |
| Hard-coded dark canvas | Improve | Graph now follows the semantic light/dark theme. |
| Mobile controls | Improve | Filters and physics remain reachable through a compact overlay instead of disappearing off-screen. |

## Canvas

| Surface | Decision | Result |
| --- | --- | --- |
| Canvas list and create | Keep | Backed by portable JSON Canvas files under `canvases/`. |
| Text and note cards | Keep | The minimal useful spatial-thinking primitives; note cards open their source note. |
| Connections, drag, selection delete | Keep | Core spatial organization behavior using interoperable edge/node fields. |
| Save and status | Keep | Explicit save provides confidence for filesystem work; switching, opening a note, and closing also flush dirty state. |
| Delete | Improve | Two-step destructive confirmation and move-to-trash semantics; browser trash filenames are collision-safe. |
| Decorative/advanced card tools | Defer | Groups, colors, resizing, embeds, and labels should arrive only with complete JSON Canvas-compatible behavior. |

## Settings and internal functions

| Surface | Decision | Result |
| --- | --- | --- |
| Appearance | Keep | Theme is a meaningful cross-runtime preference. |
| Templates folder | Keep | Discovers ordinary Markdown templates; new-note and daily-note creation use them. |
| Daily-note format | Improve | Invalid date patterns are surfaced before they create bad filenames. |
| ZIP import/export | Relocate | Visible only for browser-local storage. Filesystem vault folders are already the portable source of truth. |
| Browser-local recycle bin | Add | Soft-deleted notes can be restored or permanently erased with a two-step destructive action. Filesystem vaults continue to use their ordinary `.trash` directory. |
| Refresh / switch vault | Add | Filesystem users can deliberately rescan external changes or open another folder. Web users can change storage modes without mixing folder projections into authoritative browser-local data, and can return from the chooser without losing context. |
| Live filesystem status | Add | Desktop Settings reports whether native external-change watching is active. Browser-folder mode accurately describes its focus/manual refresh boundary instead of claiming live observation the platform cannot provide. |
| External edit conflict | Add | Clean editors update from disk automatically. Dirty editors pause autosave and require an explicit “Reload disk” or “Keep mine” decision. |
| Legacy demo controls and generator | Remove | Product-demo state was not user knowledge. |
| Unused block parser and result helper | Remove | Neither powered a reachable feature; keeping them implied a capability the product did not have. |
| Empty-folder creation gateway | Remove | No reachable workflow used it; note paths already create required parent folders. |
| Dexie `blocks` table | Retain internally | Preserved solely for schema compatibility while existing databases migrate; it is not presented as a feature. |

## Deferred surfaces

Cloud sync, collaboration, publishing, a public plugin API, and dedicated mobile clients are deliberately absent. They require security, conflict handling, identity, lifecycle, and platform decisions that should not be represented by premature buttons. The current responsive PWA is the mobile bridge while desktop, website, and browser app mature.
