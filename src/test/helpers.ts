/**
 * Test helpers. freshDB() returns a Dexie database hooked up to fake-indexeddb
 * with a unique name per test, so parallel test files don't share state.
 */

import { db as defaultDb } from '@/infrastructure/database/db';

/**
 * Clear all tables. Use between tests to start with a known state.
 */
export async function resetDb(db: typeof defaultDb = defaultDb): Promise<void> {
  await db.transaction(
    'rw',
    [db.pages, db.blocks, db.links, db.tags, db.assets, db.revisions, db.settings],
    async () => {
      await Promise.all([
        db.pages.clear(),
        db.blocks.clear(),
        db.links.clear(),
        db.tags.clear(),
        db.assets.clear(),
        db.revisions.clear(),
        db.settings.clear(),
      ]);
    },
  );
}

export function makePage(overrides: Partial<{
  id: string;
  title: string;
  content: string;
  aliases: string[];
  frontmatter: Record<string, unknown>;
}> = {}) {
  const id = overrides.id ?? `p_${Math.random().toString(36).slice(2, 10)}`;
  const title = overrides.title ?? 'Untitled';
  const ts = Date.now();
  return {
    id,
    title,
    lcTitle: title.toLowerCase(),
    path: `${title}.md`,
    content: overrides.content ?? '',
    frontmatter: overrides.frontmatter ?? {},
    aliases: overrides.aliases ?? [],
    createdAt: ts,
    updatedAt: ts,
    deletedAt: null,
  };
}