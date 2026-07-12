/**
 * One-shot vault seeder. Runs on first launch (when pages table is empty).
 * Provides a non-empty starter experience so the app is usable immediately.
 */

import { db } from './db';
import { nanoid } from '@/shared/lib/nanoid';
import { now } from '@/shared/lib/time';

const WELCOME_MARKDOWN = `# Welcome to Ley

Ley is a local-first knowledge graph for your notes. Everything lives in your browser — no servers, no accounts, no telemetry.

## Start here

- Create a new note with the **+** button in the sidebar, or press \`Cmd/Ctrl + N\`.
- Type \`[[\` to link to another note. Links to notes that don't exist yet will appear as ghosts — click them to create the note.
- Open the graph view with \`Cmd/Ctrl + G\` to see how your notes connect.

## Key ideas

**Workspace first.** You should be productive without ever opening the graph. The graph is a view, not the point.

**Markdown stays portable.** This compatibility vault lives in your browser and exports to ordinary Markdown. Open a folder vault or use desktop when files should be authoritative immediately.

**Backlinks are derived.** Every \`[[link]]\` you write is indexed automatically — open the right panel to see what points to this note.

## Daily notes

Press \`Cmd/Ctrl + D\` to open or create today's daily note.

## Tag your notes

Type \`#\` anywhere to add a tag. Tags can be nested: \`#project/ley/architecture\`.

---

*This welcome note is yours to delete. Click anywhere to start editing.*
`;

const DAILY_TEMPLATE = `# {{date}}

## Intentions

-

## Log

-

## Reflections

-
`;

export async function seedIfEmpty(): Promise<void> {
  const pageCount = await db.pages.count();
  if (pageCount > 0) return;

  const ts = now();

  await db.transaction('rw', db.pages, db.settings, async () => {
    await db.pages.add({
      id: nanoid(),
      title: 'Welcome',
      lcTitle: 'welcome',
      path: 'Welcome.md',
      content: WELCOME_MARKDOWN,
      frontmatter: {},
      aliases: [],
      createdAt: ts,
      updatedAt: ts,
      deletedAt: null,
    });

    await db.settings.put({ key: 'daily-note-format', value: 'yyyy-MM-dd' });
    await db.settings.put({ key: 'daily-note-template', value: DAILY_TEMPLATE });
    await db.settings.put({ key: 'theme', value: 'dark' });
    await db.settings.put({ key: 'graph-node-limit', value: 5000 });
    await db.settings.put({ key: 'local-graph-depth', value: 2 });
    await db.settings.put({ key: 'active-data-kind', value: 'browser-local' });
  });

}
