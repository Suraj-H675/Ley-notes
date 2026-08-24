# Runtime verification contract

Builds and unit tests do not prove that Ley is usable. Every release-oriented UI pass must verify these runtime invariants with a real browser at representative desktop, mobile, and short-window sizes.

## Public website

- The website opts the shared document into native page flow; the workspace route alone owns the fixed viewport lock.
- At 1440×900, 390×844, and 320×568, document `scrollHeight > innerHeight`, `scrollY` can change through ordinary page scrolling, and `scrollWidth === clientWidth`.
- Hero, premise, features, desktop, and footer regions are reachable.
- `#why`, `#features`, and `#desktop` links move the website scroll root to a visible target.
- Both web-app calls to action reach `/app`.
- Browser console contains no application error after a fresh load and full-page traversal.

## Vault onboarding

- Browser and native launchers own `h-full overflow-y-auto` roots; they must not rely on document scrolling.
- At 320×568, the heading, both browser vault choices, errors, and privacy statement are reachable.
- Oversized content starts at a reachable top rather than being vertically centered above the scroll origin.
- Leaving an active web vault exposes a return action; returning restores the same vault and opens a valid note.
- Browser-local → folder → browser-local transitions preserve local pages, attachments, revisions, and deleted notes while folder data is active only as a disposable projection.
- Rescanning the same folder preserves page IDs; changing folders does not preserve IDs or cache-only revision state.
- A native external `.md` create/edit/delete emits a vault-change event; hidden files, unrelated extensions, and Ley's own atomic writes do not produce a user-visible refresh.
- An externally deleted open note remains in its saved tab as a recovery projection after reload; restoring or discarding it is explicit, and a pending debounced save flushes on window unload before the projection changes.
- In desktop and browser-folder vaults, trashing a note preserves its relative folder under `.trash`, and Settings lists Markdown notes there. Restoring one moves it to its original folder (adding ` 2`, ` 3`, … only when that exact path is occupied), refreshes the projection and links, and never exposes files outside the selected folder. If the original record is already active, restore creates an independent recovered note instead of overwriting or merging identities.
- Selecting browser-local compatibility mode requests persistent browser storage without blocking startup when the browser declines or lacks the API. Settings describes persistent versus best-effort storage honestly and keeps ZIP backup controls visible.

## Workspace

- The document itself remains fixed to the viewport with no page-level horizontal overflow.
- The sidebar, note body, settings body, graph controls, and canvas list scroll independently where their content exceeds available space.
- At 320×568, the title retains useful width, secondary note actions collapse to labeled icons, and the formatting toolbelt remains visible while the CodeMirror document scrolls.
- Every modal moves focus inside, traps Tab navigation, closes on Escape, and restores focus to its launcher.
- A failure in editor, graph, canvas, or settings is contained by a feature recovery boundary and never invalidates vault data.

## JSON Canvas

- Create a canvas with a named group, two text cards, a note card, and an HTTP(S) link card. Rename/edit each applicable card and confirm note/link activation reaches the intended destination.
- Connect two distinct handles with the two-click flow and with a drag gesture. The connection uses the chosen sides, renders an arrow, accepts a label and JSON Canvas preset color, remains keyboard-selectable, and deletes from its inspector.
- Resize a card and group. Width and height never fall below the type-specific minimum or become non-finite, and connection endpoints continue to follow the document geometry.
- Save, close, reload, and reopen: card types/content, positions, dimensions, group label, edge label/sides/endpoints/color, and array order remain unchanged. An imported JSON Canvas 1.0 file with `subpath`, group background metadata, and custom colors survives a save round-trip.
- At 390×844, canvas tabs are horizontally reachable, the tool shelf scrolls independently, the graph area retains useful height, Fit View can zoom below 0.5×, every card is reachable, and the document has no horizontal overflow.

## Authoring story

- Live Preview is the default editing style. Inactive heading, emphasis, inline-code, wiki-link, Markdown-link, task, quote, and horizontal-rule syntax renders as readable content; the active line reveals its exact Markdown source.
- Inactive Markdown tasks become accessible checkboxes. Toggling one updates authoritative `[ ]`/`[x]` text immediately; Source exposes the exact change, and both the Markdown and selected editing style survive reload.
- Live Preview, Source, and Reading remain distinct at desktop and 390 px widths. The narrow workspace has no horizontal document overflow and the editor remains vertically scrollable.
- Formatting buttons preserve the editor selection; Ctrl/Cmd+B, Ctrl/Cmd+I, Ctrl/Cmd+K, and Ctrl/Cmd+Shift+backtick invoke the same transactions.
- Typing `[[` exposes recent note suggestions; typing a partial title filters them.
- Enter and Tab accept a completion and keep the current note open.
- `[[Note#` offers that note's real headings with level and line context; `[[Note#^` offers block IDs with source previews, excluding fenced code examples.
- Typing `#` at a Markdown boundary offers the vault's existing flat and nested tags, ranked by match and usage count. Headings, frontmatter, inline/fenced code, URL fragments, and attribute fragments do not trigger tag completion or enter the tag index.
- Accepting a tag replaces only the text after the existing `#`; the touch toolbelt's tag action inserts `#` and opens the same completion list. At 390px the popup and compact action remain reachable with no document overflow.
- Normal click inside a wiki link positions the cursor; Ctrl/Cmd-click opens its target without creating an ID-named note.
- Read-mode task clicks persist to Markdown, including tasks after an embed and tasks inside a partial-note embed.
- Read-mode and Ctrl/Cmd-click wiki links honor both `#Heading` and `#^block-id` destinations and focus the exact source line.
- Heading-scoped embeds stop at the next sibling heading; block embeds render only the referenced block.
- Bookmarking a note updates the workspace button and unified Bookmarks hub immediately; the state survives reload and is also reachable through the explorer context menu and command palette. Trashing it hides the note row without discarding the preference; restoring it restores the row.
- Bookmark a heading from Outline, move the editor elsewhere, and open the bookmark: the exact heading becomes CodeMirror's active line. Rename the bookmark to a custom title, then clear that title to recover the live note/heading label.
- Place the cursor on a prose/list block and bookmark it: Ley appends a valid `^block-id` to authoritative Markdown, shows a text-preview label, and reuses the ID on a second capture. Opening it returns to that exact block. Blank lines, YAML, headings, and fenced code report a useful refusal without creating metadata.
- Save a search and confirm it appears reactively in the same Bookmarks hub with open, property-table, rename, and delete actions rather than creating another sidebar panel.
- Rename or rescan a bookmarked note and verify ID/path fallback keeps anchors usable. Trash it and verify the unavailable row can be deleted but remains non-navigable until restore.
- At 390px, Bookmarks rows and hover-equivalent actions remain reachable, the editor toolbar fits without horizontal scrolling, the sidebar scrolls independently, and the document has no horizontal overflow.
- `Cmd/Ctrl+F` and the editor Find button open the same focused in-note search panel; next/previous, replace, replace-all, case, regular-expression, and whole-word controls operate on source Markdown.
- Replacements persist after the normal editor debounce and reload. Escape closes the panel without invoking browser page search.
- At 320×568, the search panel remains fully reachable without horizontal overflow and stays beneath the mobile sidebar overlay.
- Quick-switcher filters compose with AND: nested `tag:`, quoted `path:`, `title:`, `property:key=value`/`[key:value]`, and `-` exclusions return the expected live-index results. Filter-only searches work without free text.
- `task:`, `task-todo:`, and `task-done:` match text only inside real Markdown task blocks, compose with the other filters and negation, ignore fenced examples, and show the matched task/state as result context when no free-text term is present. The same query filters live collection rows and survives as a saved search.
- Filter chips insert valid syntax, the syntax guide is keyboard-accessible, Enter opens the selected result in the focused pane, and Shift+Enter opens it in split. At 390px the chip row scrolls without widening the document.
- Save a structured query with a custom name: it appears reactively under Saved searches, reopens the exact query, supports Enter/blur rename and deletion, survives reload, and is absent after switching to a different vault identity. Duplicate query saves update one entry.
- At 390px, saved-search rows and their rename/delete actions remain reachable while the sidebar overlay produces no horizontal document overflow.
- Open an empty or structured query as a table: only matching live notes appear, common YAML keys become columns, numeric values sort numerically, missing values remain last in either direction, and title/split actions open the intended note.
- Edit two property cells in quick succession and inspect the underlying note: both values persist in YAML without losing body content or one another. Enter commits; Escape restores the prior value without closing the collection.
- Change visible columns and sort order on a saved query, close it, and reopen from the sidebar table action: the exact layout returns. Ad-hoc query tables do not silently create saved state.
- At 390px the collection dialog and column picker stay within the viewport, the document has no horizontal overflow, and the wide table scrolls internally while the name column remains sticky.
- Relative Markdown links between nested folders appear in outgoing links, backlinks, and the graph. Reading-mode clicks and editor Ctrl/Cmd-clicks honor heading and block anchors.
- Renaming or moving a Markdown-link target rewrites incoming destinations; moving the source rebases its outgoing relative paths. Navigation is repeated after both changes.
- Missing `.md` destinations remain visible as missing files and never create a title-based ghost note.
- Open multiple tabs, close one, activate a non-final tab, and reload: exact tab order, active note, and recent order restore for that vault.
- Open a second note from its tab or explorer context menu into a split: both panes edit independently, link navigation stays in its source pane, a destination already visible opposite receives focus, and the keyboard/pointer divider persists its width.
- At 390px, only the focused split pane is visible, selecting the opposite pane's tab reveals it, and the document has no horizontal overflow. Reload restores both pane notes and the focused side.
- Rename an open note and reload; its session survives by stable ID. Trash an open note and reload; the stale tab and recent entry are removed safely.
- Enter the vault chooser and return to the current vault; the complete session restores rather than falling back to the most recently edited page.
- Save a distraction-free single-pane workspace and a research workspace with two notes, a non-default divider width, visible sidebars, and a selected dock tab. Loading each restores that complete arrangement; saved layouts remain after reload and never appear in another vault.
- Rename a workspace, update it from the current arrangement, and delete it through the two-step destructive action. If some referenced notes have moved or disappeared, valid notes still load by stable ID/path; an entirely stale layout reports an error without replacing the current workspace.
- At 390px the workspace manager stays inside the viewport, all save/load/update/rename/delete controls remain reachable, its list scrolls internally, and neither it nor the underlying document widens.
- The browser console remains free of CodeMirror plugin errors throughout completion, acceptance, and navigation.
- Rapid consecutive autosaves are serialized per note, navigation flushes the pending edit, and the newest Markdown, backlinks, and tag rows agree after the save settles.
- An external content update replaces a clean editor automatically. With unsaved local text, autosave pauses and both conflict actions are verified: reload preserves disk, while keep-mine explicitly persists the editor version.

## Agent Memory session naming

- Finish a real filesystem-backed CLI session, rename it with the inspected event count, and reopen it. The event directory gains exactly one immutable event; `originalName`, the stable session ID, citations, terminal status, reason, and prior work remain unchanged.
- Retry the accepted request ID and confirm it replays without another event. Attempt a stale event count and the current name again under new request IDs; both fail without changing history.
- In Ley Desktop, open a completed session, inspect its original name and prior naming revisions, then append a new name and required reason. The dashboard, resume card, inspector title, event count, and naming history update from the shared engine.
- At desktop, short-window, and 390×844 sizes, the session body and rename fields scroll independently, while Close, Cancel, and Append Rename remain reachable. Unsaved edits require confirmation before dismissal.
- The rendered flow produces no application console errors. MCP tool discovery remains unchanged because session naming is a local user-authority surface, not an agent write capability.

## Agent Memory note links

## Verified browser collections workflow

- On 2026-08-25, a fresh browser-local vault was created with two notes, each given a `status: active` property. The `property:status=active` quick-switcher query opened as a live collection containing both notes.
- Editing one property cell to `archived` committed the YAML value, removed that row immediately, and left the other active row intact.
- The column picker added Path, and clicking its header cycled ascending/descending sort. Saving the query preserved both the query and the four-column/sorted layout after closing and reopening from Saved searches; ad-hoc table changes were not silently saved.

## Verified browser revision workflow

- On 2026-08-25, Project Alpha received two explicit save checkpoints after separate body edits. The History dock exposed both snapshots with relative timestamps.
- Selecting the earlier snapshot restored it through the normal editor path: the body returned to empty while the `status` frontmatter remained intact.
- Closing History, reloading `/app`, and reopening History preserved the same restored note content and revision timeline; no stale in-memory state was required.

## Verified browser workspace layouts

- On 2026-08-25, a single-pane Focus layout and a two-pane Research layout were saved in the same browser-local vault. Research retained Project Alpha as primary and Project Beta as secondary.
- Loading Focus restored one pane; loading Research restored the split with both intended notes.
- Closing the dialog, reloading `/app`, reopening Workspace layouts, and loading each saved layout preserved their distinct arrangements after persistence. The browser reported no page errors during the pass.

## Verified 390px collections behavior

- On 2026-08-25, the same live `property:status=active` collection was reopened at 390×844. The document stayed at 390px with no horizontal overflow, and the dialog content remained within 372px.
- The column picker opened and added Path without leaving the viewport. The four-column table scrolled internally by 676px while the Name column remained sticky.
- The browser console reported no application errors during the pass.

- Open a project whose Agent Memory is bound to the active filesystem vault. Promote a current trusted lesson and export a completed session through **To notes**; each creates and opens ordinary Markdown with the expected portable project/source ID, timestamp, tag, provenance warning, and visible content.
- Rename and move both notes, repeat each action, and verify Ley opens the existing note without creating a duplicate. Create an unrelated title collision and verify the proposed link refuses to overwrite it.
- Open a catalog project bound to another available vault while the first notes vault remains active. Both learning promotion and session export must fail before any note/index mutation, name both vault folders without revealing absolute paths, and succeed only after the bound vault is deliberately opened.
- Export a truncated session projection and verify the note and preview disclose omitted checkpoints/clipped text rather than implying complete history. Stored headings, callouts, links, remote images, raw HTML, control characters, and prompt-injection strings remain escaped quoted evidence and cannot alter the provenance warning or trigger external content.
- At desktop, short-window, and 390×844 sizes, the **Link session to notes** action (visually **To notes** when space permits), title preview, Cancel, and Create remain reachable; unsaved title edits require confirmation before dismissal, and no application console error is produced.

## Agent Memory graph history

- Ingest a real initialized and bound project, change a captured symbol or relationship, and ingest again. The history index gains one graph/artifact pair; an identical third ingestion does not add another entry.
- Open the project graph in Ley Desktop. Switch from Current to the earlier capture and confirm the historical banner, counts, nodes, edges, Git identity, and capture time all come from that immutable snapshot while the working tree remains untouched.
- Combine node-kind, relationship, provenance, and text filters. Backend-reported filtered counts agree with the rendered bounded graph, and reset restores the complete selected capture.
- Select a node and a relationship, navigate between their endpoints, and open each citation. The excerpt has the recorded relative path, hash, line numbers, snapshot ID, and redacted captured text—even after the live source changes.
- Open cited artifacts from a session checkpoint, project decision, and problem. Each control moves to the graph, selects the matching artifact capture when retained, highlights the recorded line range, and reads the immutable redacted excerpt. From a learning evidence row, open its full originating session and its captured artifact record.
- Attempt a citation borrowed from another node or capture and verify that the shared engine rejects it. Open a Minimal capture and verify that the inspector explains why no source text exists instead of reading the live file.
- In **Capture & privacy**, arm erasure and verify the permanent action stays disabled for a missing or case-mismatched project name. With an active lifecycle reader, erasure waits; after it exits, the project memory directory disappears while source files, notes, `.ley` metadata, binding, and catalog observation remain. The UI returns to **Needs capture**, and an explicit recapture produces a fresh store.
- At desktop, short-window, and 390×844 sizes, filters scroll independently, the graph remains usable, the inspector stays reachable without document overflow, focus indicators remain visible, and reduced-motion/reduced-transparency preferences preserve the interaction.

## Release evidence

- Run lint, the complete test suite, the web/PWA production build, Rust formatting/tests, and the native release bundle.
- Verify generated Debian and RPM artifacts on Linux.
- On 2026-08-22, lint, all frontend tests, the production web build, Rust workspace tests, and a current-source Linux desktop bundle completed successfully. The generated `Ley_0.1.0_amd64.deb` and `Ley-0.1.0-1.x86_64.rpm` artifacts were inspected for expected package identity and payload structure; native installation, signed release distribution, and cross-platform launch checks remain user/platform verification work.
- Record any platform-specific packaging limitation honestly rather than substituting an unrelated artifact.
