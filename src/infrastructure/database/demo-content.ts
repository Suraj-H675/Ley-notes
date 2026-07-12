/**
 * Demo content — a realistic PKM (personal knowledge management) vault used
 * for first-run demo and "Add demo" in Settings. Idempotent: each page is
 * inserted only if a page with that title doesn't already exist.
 *
 * The vault is structured to produce a visually interesting graph:
 *   - Hub pages with high degree (Index, Welcome, Knowledge Graph)
 *   - 5 distinct communities (productivity, engineering, learning, meta, daily)
 *   - Cross-cluster bridges (e.g. weekly review links engineering + productivity)
 *   - A few orphan/leaf pages
 *   - Recent daily notes chained together
 *
 * To keep the demo dense, each page links to 3-10 others. Total ~25 pages.
 */

import { createPage, getPageByTitle } from '@/core/vault/pages';

interface DemoPage {
  title: string;
  path: string;
  content: string;
  /** Folder is derived from path — keep this field empty unless you want to override. */
  folder?: string;
  tags?: string[];
  aliases?: string[];
}

const DEMO_PAGES: DemoPage[] = [
  // ============ TOP-LEVEL HUBS ============
  {
    title: 'Index',
    path: 'Index.md',
    tags: ['meta'],
    aliases: ['Home', 'MOC'],
    content: `# Index

My map of content. Every note in this vault should be reachable from here.

## Topics

### Productivity
- [[PKM Systems]] — building a personal knowledge base
- [[GTD Workflow]] — getting things done
- [[Zettelkasten Method]] — Niklas Luhmann's slip-box
- [[Daily Notes]] — capture first, organize later
- [[Weekly Review]] — the habit that keeps it all alive

### Engineering
- [[Code Architecture]]
- [[Database Patterns]]
- [[React Patterns]]
- [[Performance Notes]]
- [[TypeScript Tips]]

### Learning
- [[Reading List]]
- [[Spaced Repetition]]
- [[Feynman Technique]]
- [[Note-Taking Methods]]

### Meta
- [[Tools Comparison]]
- [[Obsidian vs Logseq]]
- [[Knowledge Graph]]

See also: [[Welcome]] and the [[2026-07-10]] daily note. #meta
`,
  },

  {
    title: 'Knowledge Graph',
    path: 'notes/Knowledge Graph.md',
    tags: ['meta'],
    content: `# Knowledge Graph

The visualization of connections between notes. In Ley, hit ⌘G to open the full graph.

## Why graphs matter

Most note-taking apps are linear — lists of documents. A **knowledge graph** makes the **relationships** first-class: which note links to which, which clusters of notes form, which notes are hubs (many connections) and which are leaves (few).

This changes how you think about writing. Instead of "where does this note go?", you ask "what does this note connect to?".

## How Ley computes the graph

Every \`[[link]]\` you write is indexed automatically. Edges are derived from the parser and stored in the \`links\` table. The graph view reads from this index — sub-millisecond lookups, regardless of vault size.

## Communities

Ley uses Louvain community detection to cluster related notes. Each cluster gets its own color in the graph view. Use the community legend (bottom of the sidebar) to hide or focus on specific clusters.

For the underlying algorithms, see [[Code Architecture]] and [[Performance Notes]].

#meta #tools/ley

See [[Index]] for everything.`,
  },

  // ============ PRODUCTIVITY CLUSTER ============
  {
    title: 'PKM Systems',
    path: 'notes/productivity/PKM Systems.md',
    tags: ['productivity', 'productivity/pkm'],
    content: `# PKM Systems

Personal Knowledge Management — the discipline of capturing, organizing, and retrieving what you learn.

## The four-stage loop

1. **Capture** — get it out of your head fast. Inbox, daily note, voice memo.
2. **Process** — decide: trash, reference, or develop into a permanent note?
3. **Connect** — find links to existing notes. This is where the magic happens.
4. **Create** — write something new that synthesizes what you've collected.

The first two are mechanical. The third is the actual craft of knowledge work — see [[Note-Taking Methods]] for approaches.

Related: [[Zettelkasten Method]], [[GTD Workflow]], [[Daily Notes]]. #productivity #productivity/pkm`,
  },

  {
    title: 'GTD Workflow',
    path: 'notes/productivity/GTD Workflow.md',
    tags: ['productivity'],
    content: `# GTD Workflow

David Allen's Getting Things Done, distilled.

## Five phases

1. **Capture** — every open loop into a trusted system.
2. **Clarify** — what is it? Is it actionable? If yes, what's the next physical action?
3. **Organize** — list, calendar, someday/maybe, reference, trash.
4. **Reflect** — weekly review. Don't skip this.
5. **Engage** — pick the right thing, given context, time, energy, priority.

The weekly review is what makes GTD work. See [[Weekly Review]].

Complementary to [[PKM Systems]] — GTD is about action, PKM is about knowledge. #productivity`,
  },

  {
    title: 'Zettelkasten Method',
    path: 'notes/productivity/Zettelkasten Method.md',
    tags: ['productivity', 'productivity/pkm'],
    content: `# Zettelkasten Method

Niklas Luhmann's slip-box technique — 90,000 notes, 70+ books, the secret of his productivity.

## The idea

Every atomic idea gets its own note. Notes link to other notes. You don't organize by topic — you let the network of links emerge organically. The slip-box becomes a thinking partner.

## Modern variants

- **Obsidian** — file-based, wiki links, block references.
- **Logseq** — block-based, outliner-first.
- **Roam** — block-based, daily-note-first.

Ley takes the Obsidian approach but adds a proper [[Knowledge Graph]] view.

See also: [[PKM Systems]], [[Note-Taking Methods]]. #productivity #productivity/pkm`,
  },

  {
    title: 'Daily Notes',
    path: 'notes/productivity/Daily Notes.md',
    tags: ['productivity'],
    content: `# Daily Notes

One note per day. Open it every morning, dump everything you think about, link liberally.

## Why they work

Daily notes collapse the "where do I put this?" decision to nothing. Just append. The friction of starting a new note goes away, so you write more.

They also create temporal context — when did you first think about X? Search the date in your vault.

Press ⌘D in Ley to open today's daily note. See also [[Weekly Review]].

Examples in this vault: [[2026-07-10]], [[2026-07-09]], [[2026-07-08]], [[2026-07-07]], [[2026-07-06]]. #productivity`,
  },

  {
    title: 'Weekly Review',
    path: 'notes/productivity/Weekly Review.md',
    tags: ['productivity'],
    content: `# Weekly Review

The single most important habit in any knowledge system.

## Checklist

- [ ] Empty inboxes (real + digital).
- [ ] Process loose notes from the week into permanent ones.
- [ ] Review project lists — any stalled?
- [ ] Review someday/maybe — anything ready to act on?
- [ ] Calendar: next week, anything to prep?
- [ ] Read recent daily notes ([[Daily Notes]]) — anything to follow up?

The review is what keeps the system alive. Without it, the slip-box becomes a graveyard.

Borrowed from [[GTD Workflow]]. See also [[PKM Systems]].

The actual review session: [[2026-07-10]]. #productivity`,
  },

  // ============ ENGINEERING CLUSTER ============
  {
    title: 'Code Architecture',
    path: 'notes/engineering/Code Architecture.md',
    tags: ['engineering'],
    content: `# Code Architecture

Principles I keep returning to.

## Core ideas

- **Boundaries first** — define the contracts before the implementations.
- **Local reasoning** — every module should be understandable in isolation.
- **Boring tech** — pick the obvious tool unless you have a specific reason not to.
- **Reversibility** — favor decisions you can undo.

The hardest part isn't picking the pattern — it's deciding when the pattern fits. See [[Database Patterns]] and [[React Patterns]] for concrete applications. #engineering

Related: [[Performance Notes]], [[TypeScript Tips]].`,
  },

  {
    title: 'Database Patterns',
    path: 'notes/engineering/Database Patterns.md',
    tags: ['engineering'],
    content: `# Database Patterns

What I reach for when designing a schema.

## Reads vs writes

- For read-heavy: denormalize, cache, precompute.
- For write-heavy: normalize, batch, queue.
- For mixed: CQRS is overkill most of the time — start simple.

## Indexes

- Every foreign key needs an index.
- Composite indexes follow the query's WHERE + ORDER BY order.
- Don't index low-cardinality columns (boolean flags) unless used in a partial index.

See [[Performance Notes]] for query-level optimizations. Also [[Code Architecture]] for the broader system thinking. #engineering`,
  },

  {
    title: 'React Patterns',
    path: 'notes/engineering/React Patterns.md',
    tags: ['engineering', 'engineering/react'],
    content: `# React Patterns

Patterns I lean on.

## Component composition

- **Compound components** — parent passes context, children opt-in. Great for tabs, accordions.
- **Render props** — useful when you need access to internal state.
- **Hooks** — the default for new code.

## State

- **Local state first** — only lift when two siblings need it.
- **Server state** — react-query / SWR, not Redux.
- **URL state** — search params, not context.

## Performance

- **Memoize expensive computation**, not JSX.
- **\`useMemo\` and \`useCallback\`** — measure first.
- **\`React.memo\`** — only when re-renders are actually expensive.

See [[Code Architecture]] for the bigger picture. Also [[TypeScript Tips]] for typing patterns. #engineering #engineering/react`,
  },

  {
    title: 'Performance Notes',
    path: 'notes/engineering/Performance Notes.md',
    tags: ['engineering'],
    content: `# Performance Notes

Quick wins and traps.

## Frontend

- **Bundle size** — check it weekly. Tree-shaking is only as good as your imports.
- **Image format** — WebP/AVIF for photos, SVG for icons.
- **Code-split** — dynamic \`import()\` for routes and heavy widgets.

## Backend

- **Cache invalidation** is the hard part. Pick TTLs deliberately.
- **Batch writes** — most DBs are 10x faster on bulk inserts.
- **Connection pooling** — don't open a new DB connection per request.

See [[Database Patterns]] for query-level work. The graph rendering in Ley uses these techniques — see [[Knowledge Graph]]. #engineering`,
  },

  {
    title: 'TypeScript Tips',
    path: 'notes/engineering/TypeScript Tips.md',
    tags: ['engineering'],
    content: `# TypeScript Tips

Stuff I wish I'd known earlier.

## The basics

- \`unknown\` over \`any\` — force yourself to narrow.
- Discriminated unions — \`type Shape = { kind: 'circle'; r: number } | { kind: 'square'; s: number }\`.
- \`satisfies\` — validate a type without widening.

## React

- \`React.ComponentProps<typeof Foo>\` to forward types.
- Generic components: \`function List<T>({ items }: { items: T[] }) { ... }\`.

See [[React Patterns]]. #engineering`,
  },

  // ============ LEARNING CLUSTER ============
  {
    title: 'Reading List',
    path: 'notes/learning/Reading List.md',
    tags: ['learning'],
    content: `# Reading List

Books I want to read or revisit.

## Currently reading

- *Designing Data-Intensive Applications* — Kleppmann. Best systems book ever written. See [[Database Patterns]].

## Next up

- *Code* — Petzold. Classic on the fundamentals.
- *The Pragmatic Programmer* — Hunt & Thomas.

## Reference

- *Site Reliability Engineering* — Google.
- *Structure and Interpretation of Computer Programs* — Abelson & Sussman.

See [[Feynman Technique]] for how to actually retain what you read. Also [[Spaced Repetition]] for the SRS side. #learning`,
  },

  {
    title: 'Spaced Repetition',
    path: 'notes/learning/Spaced Repetition.md',
    tags: ['learning'],
    content: `# Spaced Repetition

The single most evidence-backed learning technique.

## The curve

Forget fast → review → forget slower → review → remember longer. Each successful review pushes the next forgetting curve further out.

## Tools

- **Anki** — the gold standard. Free, open-source, painful UX.
- **RemNote** — combines notes + SRS, integrates with the slip-box.
- **Obsidian Spaced Repetition plugin** — works inside Obsidian/Ley.

## When to use

Almost any declarative knowledge — vocabulary, formulas, definitions, "what is X". Not great for skills (use deliberate practice).

See [[Feynman Technique]] for the other side — understanding, not just recall. Pairs with [[Reading List]]. #learning`,
  },

  {
    title: 'Feynman Technique',
    path: 'notes/learning/Feynman Technique.md',
    tags: ['learning'],
    content: `# Feynman Technique

Richard Feynman's method for learning anything deeply.

## Steps

1. **Pick a concept.** Write the title at the top of a blank page.
2. **Explain it like you're teaching a child.** Use plain language, no jargon.
3. **Identify the gaps.** Where did you stumble? Go back to the source.
4. **Simplify and use analogies.** If your explanation is still complex, you don't understand it yet.

The test: if you can't explain it simply, you don't understand it.

Pairs beautifully with [[Spaced Repetition]] — Feynman for understanding, SRS for retention. See also [[Note-Taking Methods]].

Try it on [[Code Architecture]] or [[Zettelkasten Method]]. #learning`,
  },

  {
    title: 'Note-Taking Methods',
    path: 'notes/learning/Note-Taking Methods.md',
    tags: ['learning'],
    content: `# Note-Taking Methods

A survey of approaches, with my preferences.

## Linear

- **Cornell** — cue column + notes + summary. Good for lectures.
- **Outline** — hierarchical. Good for structured information.

## Networked

- **Zettelkasten** — atomic notes, links, slip-box. See [[Zettelkasten Method]].
- **Evergreen notes** — by Andy Matuschak. Notes that are continuously refined.
- **Maps of content** — index pages like [[Index]] that organize everything.

## Practice

- **Cornell + daily notes + Zettelkasten** — my current stack. Daily for capture, Zettelkasten for permanent, MOCs for navigation.

Related: [[PKM Systems]], [[Spaced Repetition]], [[Feynman Technique]]. #learning`,
  },

  // ============ META CLUSTER ============
  {
    title: 'Tools Comparison',
    path: 'notes/meta/Tools Comparison.md',
    tags: ['meta'],
    content: `# Tools Comparison

Notes apps I've tried, ranked by how long I stuck with them.

| App       | Used for | Liked | Hated |
|-----------|----------|-------|-------|
| Obsidian  | 2 years  | Local files, plugin API | Sync, mobile |
| Logseq    | 6 months | Block model, outliner | Performance |
| Notion    | 1 year   | Collaboration, databases | Speed, lock-in |
| Roam      | 3 months | Block references, graph | Price, sync |
| Apple Notes | ongoing | Speed, free | No links, no graph |
| **Ley**   | current  | Local-first, Obsidian-compat | Brand new |

See [[Obsidian vs Logseq]] for a deeper comparison, and [[Markdown Syntax]] for the underlying format. #meta #tools`,
  },

  {
    title: 'Obsidian vs Logseq',
    path: 'notes/meta/Obsidian vs Logseq.md',
    tags: ['meta'],
    content: `# Obsidian vs Logseq

The two big names in local-first markdown note apps.

## Obsidian

- **File-based** — each note is a \`.md\` file. Easy to back up, sync, version-control.
- **Wiki links** — \`[[Page]]\`, block references, embeds.
- **Plugin API** — massive ecosystem (1000+ community plugins).
- **Graph view** — built-in, decent.

## Logseq

- **Block-based** — each bullet is its own entity. Great for outliners.
- **Bi-directional blocks** — \`((block-id))\` references.
- **Queries** — Datalog-style. Powerful but a learning curve.
- **Graph view** — built-in, similar to Obsidian.

## Which to choose?

- **Want files on disk** → Obsidian.
- **Want block-level outliner** → Logseq.

Ley borrows the best of both: file-based + great graph + local-first.

See [[Tools Comparison]], [[Markdown Syntax]]. #meta #tools`,
  },

  {
    title: 'Markdown Syntax',
    path: 'notes/meta/Markdown Syntax.md',
    tags: ['meta'],
    content: `# Markdown Syntax

The minimum-viable subset I use 95% of the time.

## Text

\`\`\`
# Heading 1
## Heading 2
**bold**, *italic*, ~~strikethrough~~
\`inline code\` and [links](https://example.com)
\`\`\`

## Lists

\`\`\`
- bullet
  - nested
- another

1. ordered
2. list
\`\`\`

## Obsidian-flavored

- \`[[Wiki link]]\` — links to another note. See [[Welcome]].
- \`[[Page|alias]]\` — display alias.
- \`#tag\` — inline tag.
- \`> blockquote\` for callouts.

That's it. Don't overthink formatting — most notes are 90% prose.

#meta #tools`,
  },

  // ============ DAILY NOTES (CHAINED) ============
  {
    title: '2026-07-10',
    path: 'daily/2026-07-10.md',
    tags: ['daily'],
    content: `# 2026-07-10

## Intentions

- Ship the Ley graph view — see [[Knowledge Graph]].
- Read more of *DDIA* — see [[Reading List]].

## Log

- 09:00 — morning pages.
- 11:30 — long walk, no phone. Thinking about [[Zettelkasten Method]] and how the slip-box forces connections.
- 14:00 — Pair-programmed on the React Flow integration. Hit a snag with FA1 layout. Solved by switching from FA2.
- 17:00 — [[Weekly Review]] done. Inbox is empty for the first time in weeks.

## Reflections

The graph visualization changed how I think about writing. Now I write to be linked, not just to be read.

Tomorrow: finish the [[Tools Comparison]] page.`,
  },

  {
    title: '2026-07-09',
    path: 'daily/2026-07-09.md',
    tags: ['daily'],
    content: `# 2026-07-09

## Intentions

- Implement community detection (Louvain). See [[Knowledge Graph]].

## Log

- 08:30 — Read more of *DDIA*. See [[Reading List]].
- 10:00 — Tried \`graphology-communities-louvain\`. Works but deterministic seed matters.
- 13:00 — Lunch.
- 15:00 — Added the legend. Click to toggle cluster visibility.
- 19:00 — Dinner with friends. Talked about [[Spaced Repetition]] and how I learn languages.

## Reflections

Louvain is fast — sub-100ms on 1000 nodes. The hard part is making the cluster IDs stable across re-runs. Solution: deterministic RNG seeded with the graph hash.

Linked: [[2026-07-10]], [[2026-07-08]].`,
  },

  {
    title: '2026-07-08',
    path: 'daily/2026-07-08.md',
    tags: ['daily'],
    content: `# 2026-07-08

## Intentions

- Build the parser. Skip frontmatter complexity for now.

## Log

- All-day coding. Wrote the [[Markdown Syntax]] reference while testing the parser.
- Found bugs around code fences (the parser thought fenced \`[[link]]\` was a real link).
- Fixed by tracking fence state line-by-line.

## Reflections

Parser bugs are embarrassing but inevitable. The fix is to write tests for each edge case before they bite you.

Linked: [[2026-07-09]], [[2026-07-07]].`,
  },

  {
    title: '2026-07-07',
    path: 'daily/2026-07-07.md',
    tags: ['daily'],
    content: `# 2026-07-07

Monday. Kicked off the [[Ley Notes Roadmap]]. Initial scope: vault CRUD, markdown editor, wiki links.

Caught up on the [[Reading List]] — almost done with chapter 3 of *DDIA*. Replication is harder than I thought.

Notes from today feel scattered. Need to refactor tomorrow.

Linked: [[2026-07-08]], [[2026-07-06]].`,
  },

  {
    title: '2026-07-06',
    path: 'daily/2026-07-06.md',
    tags: ['daily'],
    content: `# 2026-07-06

Sunday. Did the [[Weekly Review]]. Realized I've been letting my [[Daily Notes]] slip — going to recommit this week.

Sketched out what the [[Code Architecture]] note should cover. Three principles: boundaries, local reasoning, reversibility.

Cooked a new pasta recipe. Unrelated.`,
  },

  // ============ PROJECT ============
  {
    title: 'Ley Notes Roadmap',
    path: 'projects/Ley Notes Roadmap.md',
    tags: ['project', 'project/ley'],
    content: `# Ley Notes Roadmap

Building a local-first Obsidian alternative.

## Phase 0 — Foundation (DONE)

Wipe + scaffold. Dexie schema, theme tokens, basic shell. See [[Welcome]].

## Phase 1 — Vault + Editor (DONE)

CRUD on pages, markdown editor with CodeMirror 6, \`[[wiki link]]\` parsing + autocomplete. See [[Code Architecture]] for the system-level decisions.

## Phase 2 — Backlinks + Graph + Search (DONE)

The [[Knowledge Graph]] view, full-text search via Flexsearch. Highlights what makes Ley different from a plain markdown editor.

## Phase 3 — Daily Notes + Tags (DONE)

[[Daily Notes]], tag pane, Cmd+D shortcut.

## Phase 4 — Themes + Export (DONE)

Light/dark toggle, ZIP export/import (Obsidian-compatible).

## Future

- Plugin API
- Service worker / offline mode
- Mobile-responsive layout

See [[Open Questions]] for unresolved decisions. #project #project/ley`,
  },

  // ============ ORPHAN / LEAF ============
  {
    title: 'Open Questions',
    path: 'notes/Open Questions.md',
    tags: ['meta'],
    content: `# Open Questions

Things I'm still figuring out.

## Sandboxing

Do plugins need a sandboxed iframe, or is a CSP enough? For now: no sandbox, document the risk.

## Sync

Diff-based or CRDT? Obsidian Sync uses diff-match-patch. We're local-first only — export/import is the answer.

## Mobile

When? Not for v1.

No incoming links — orphan page. Will get picked up by the [[Index]] eventually.`,
  },
];

/**
 * Insert all demo pages into the vault. Idempotent: pages whose title
 * already exists are skipped (createPage returns the existing page).
 *
 * Returns the count of pages newly added.
 */
export async function seedDemoContent(): Promise<number> {
  let added = 0;
  for (const p of DEMO_PAGES) {
    const existing = await getPageByTitle(p.title);
    if (existing && existing.deletedAt === null) continue;
    // Extract folder from path so the file tree groups correctly.
    const folder = p.path.includes('/') ? p.path.split('/').slice(0, -1).join('/') : undefined;
    await createPage({
      title: p.title,
      content: p.content,
      folder,
      aliases: p.aliases,
    });
    added++;
  }
  return added;
}