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
- Normal click inside a wiki link positions the cursor; Ctrl/Cmd-click opens its target without creating an ID-named note.
- Read-mode task clicks persist to Markdown, including tasks after an embed and tasks inside a partial-note embed.
- Read-mode and Ctrl/Cmd-click wiki links honor both `#Heading` and `#^block-id` destinations and focus the exact source line.
- Heading-scoped embeds stop at the next sibling heading; block embeds render only the referenced block.
- The browser console remains free of CodeMirror plugin errors throughout completion, acceptance, and navigation.
- An external content update replaces a clean editor automatically. With unsaved local text, autosave pauses and both conflict actions are verified: reload preserves disk, while keep-mine explicitly persists the editor version.

## Release evidence

- Run lint, the complete test suite, the web/PWA production build, Rust formatting/tests, and the native release bundle.
- Verify generated Debian and RPM artifacts on Linux.
- Record any platform-specific packaging limitation honestly rather than substituting an unrelated artifact.
