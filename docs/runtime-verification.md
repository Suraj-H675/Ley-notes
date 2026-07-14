# Runtime verification contract

Builds and unit tests do not prove that Ley is usable. Every release-oriented UI pass must verify these runtime invariants with a real browser at representative desktop, mobile, and short-window sizes.

## Public website

- The website owns a constrained `h-full` scrolling root because the shared document is intentionally locked for the workspace.
- At 1440×900, 390×844, and 320×568, `scrollHeight > clientHeight`, `scrollTop` can change, and `scrollWidth === clientWidth`.
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

## Workspace

- The document itself remains fixed to the viewport with no page-level horizontal overflow.
- The sidebar, note body, settings body, graph controls, and canvas list scroll independently where their content exceeds available space.
- At 320×568, the title retains useful width, secondary note actions collapse to labeled icons, and the formatting toolbelt remains visible while the CodeMirror document scrolls.
- Every modal moves focus inside, traps Tab navigation, closes on Escape, and restores focus to its launcher.
- A failure in editor, graph, canvas, or settings is contained by a feature recovery boundary and never invalidates vault data.

## Authoring story

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
- Starring a note updates the workspace button and Favorites sidebar immediately; the state survives reload and is also reachable through the explorer context menu and command palette.
- Trashing a favorite hides it without discarding the preference; restoring the note restores its place in Favorites.
- At 320×568, Favorites remains readable and reachable with no horizontal document overflow.
- `Cmd/Ctrl+F` and the editor Find button open the same focused in-note search panel; next/previous, replace, replace-all, case, regular-expression, and whole-word controls operate on source Markdown.
- Replacements persist after the normal editor debounce and reload. Escape closes the panel without invoking browser page search.
- At 320×568, the search panel remains fully reachable without horizontal overflow and stays beneath the mobile sidebar overlay.
- Quick-switcher filters compose with AND: nested `tag:`, quoted `path:`, `title:`, `property:key=value`/`[key:value]`, and `-` exclusions return the expected live-index results. Filter-only searches work without free text.
- Filter chips insert valid syntax, the syntax guide is keyboard-accessible, Enter opens the selected result in the focused pane, and Shift+Enter opens it in split. At 390px the chip row scrolls without widening the document.
- Save a structured query with a custom name: it appears reactively under Saved searches, reopens the exact query, supports Enter/blur rename and deletion, survives reload, and is absent after switching to a different vault identity. Duplicate query saves update one entry.
- At 390px, saved-search rows and their rename/delete actions remain reachable while the sidebar overlay produces no horizontal document overflow.
- Relative Markdown links between nested folders appear in outgoing links, backlinks, and the graph. Reading-mode clicks and editor Ctrl/Cmd-clicks honor heading and block anchors.
- Renaming or moving a Markdown-link target rewrites incoming destinations; moving the source rebases its outgoing relative paths. Navigation is repeated after both changes.
- Missing `.md` destinations remain visible as missing files and never create a title-based ghost note.
- Open multiple tabs, close one, activate a non-final tab, and reload: exact tab order, active note, and recent order restore for that vault.
- Open a second note from its tab or explorer context menu into a split: both panes edit independently, link navigation stays in its source pane, a destination already visible opposite receives focus, and the keyboard/pointer divider persists its width.
- At 390px, only the focused split pane is visible, selecting the opposite pane's tab reveals it, and the document has no horizontal overflow. Reload restores both pane notes and the focused side.
- Rename an open note and reload; its session survives by stable ID. Trash an open note and reload; the stale tab and recent entry are removed safely.
- Enter the vault chooser and return to the current vault; the complete session restores rather than falling back to the most recently edited page.
- The browser console remains free of CodeMirror plugin errors throughout completion, acceptance, and navigation.
- Rapid consecutive autosaves are serialized per note, navigation flushes the pending edit, and the newest Markdown, backlinks, and tag rows agree after the save settles.
- An external content update replaces a clean editor automatically. With unsaved local text, autosave pauses and both conflict actions are verified: reload preserves disk, while keep-mine explicitly persists the editor version.

## Release evidence

- Run lint, the complete test suite, the web/PWA production build, Rust formatting/tests, and the native release bundle.
- Verify generated Debian and RPM artifacts on Linux.
- Record any platform-specific packaging limitation honestly rather than substituting an unrelated artifact.
