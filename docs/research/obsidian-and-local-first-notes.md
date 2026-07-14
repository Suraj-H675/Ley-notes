# Product research: Obsidian and local-first knowledge tools

This document records the product conclusions guiding Ley. It is intentionally decision-oriented; reference projects are evidence, not templates to copy wholesale.

## What makes Obsidian feel like a second brain

The durable foundation is not the graph visualization. It is the combination of user-owned Markdown files, extremely fast capture and retrieval, low-friction `[[links]]`, automatic backlinks, composable metadata, and many ways to revisit existing thought. The graph becomes valuable because these quieter systems continuously produce trustworthy structure.

The minimum coherent loop is:

1. Capture a thought without choosing a rigid schema.
2. Link it to an existing idea or leave a ghost link for later.
3. Retrieve it by title, content, tag, recent activity, daily context, or backlink.
4. Refine properties and connections without migrating out of Markdown.
5. Trust that the files remain usable without Ley.

The workspace itself should also feel continuous: reopening a vault must restore the user's working set, including side-by-side source/reference context, not merely choose whichever file was edited most recently.

## Comparable projects

- Logseq demonstrates the power of journal-first capture, block references, and local files. Its outliner model is valuable, but forcing every note into blocks would make Ley less Markdown-native.
- SiYuan demonstrates a strong local application architecture and block-level identity. Ley borrows the principle of stable derived structure, not its proprietary storage model.
- Trilium demonstrates deep trees, cloning, and mature note organization. Its database-centric model is less aligned with filesystem portability.
- Foam shows how far ordinary Markdown, wiki links, and editor-native workflows can go with a small conceptual surface.
- Capacities demonstrates polished object-oriented retrieval and presentation, but a mandatory object schema would weaken Ley’s flexible file-first core.

## Structured views without a second source of truth

Obsidian's current [Bases](https://obsidian.md/help/bases) core plugin confirms that database-style resurfacing belongs in a mature second brain: it filters, sorts, and edits note properties through table, list, card, and map views while keeping the underlying data in local Markdown files. Logseq's query builder and SiYuan's table database point in the same direction from block-first systems. The durable product lesson is the view, not a proprietary object store.

Ley's first collection surface therefore composes the retrieval language it already exposes. A query selects Markdown files, the table discovers their YAML keys, and cell edits use the same filesystem write path as the note properties panel. Saved searches may remember column visibility and sorting as vault-scoped workspace metadata, but that metadata never contains note values. Ley deliberately does not emit an Obsidian `.base` file until it can support that format faithfully rather than create a misleading partial dialect.

Obsidian's [Bookmarks](https://obsidian.md/help/plugins/bookmarks) and [Workspaces](https://obsidian.md/help/plugins/workspaces) clarify two distinct retrieval needs. Ley favorites currently address files and saved searches address queries; heading/block bookmarks remain a future workflow. Named task-specific layouts are now separate from automatic session restoration: a user can deliberately capture writing, research, or review arrangements including open tabs, split context, focus, sidebars, dock context, and divider width. They remain vault-scoped presentation metadata and tolerate renamed or missing notes through stable ID/path references.

## Decisions for Ley

- Desktop-first, real filesystem vaults; browser folder access where the platform supports it.
- Markdown and YAML frontmatter are authoritative. Databases are indexes, caches, and recovery aids.
- Pages are the initial unit of composition. Block IDs may support references later without replacing Markdown as the source.
- Search, quick switching, backlinks, and daily notes have higher priority than decorative graph complexity. Retrieval must continue scaling through composable path, tag, title, and portable YAML-property filters rather than requiring a proprietary object database.
- Recurring retrieval patterns should be saveable as vault-scoped workspace metadata. They should reopen as transparent, editable queries rather than becoming opaque database views.
- Structured collection views should project those same queries over portable YAML properties; inline changes must immediately rewrite Markdown rather than mutate a parallel object database.
- Task-specific workspace layouts should be explicitly named and restorable; automatic last-session recovery solves continuity but not deliberate context switching.
- Unresolved links are first-class and resolve automatically when their target is created.
- Both wiki links and ordinary relative Markdown links are first-class; portable path links must keep their meaning when source or target files move.
- Rename operations update incoming links because broken knowledge structures are worse than surprising automation.
- Recovery is visible to users and uses the normal save pipeline.
- Canvas, embeds, templates, attachments, and extensibility should build on the same vault contract rather than introducing parallel storage systems.

## Product quality bar

A feature is complete only when it performs real persistence, is reachable through the interface, handles an empty vault, and behaves consistently in the desktop and applicable web runtime. A passing unit test alone is not a product feature.
