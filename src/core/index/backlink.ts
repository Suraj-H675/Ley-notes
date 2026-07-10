/**
 * Backlink index. Write-through: every time a page's content is saved, we
 * re-derive its outgoing links from the markdown and replace the rows in
 * the `links` table for that source page.
 *
 * Why write-through (Trilium EntityChange pattern) instead of on-demand?
 *  - Sub-millisecond lookup at any scale (Dexie is indexed by sourcePageId).
 *  - O(1) cost per save (the parser is fast for typical page sizes).
 *  - No need to re-parse on every backlinks panel render.
 *
 * The cost: every save is now an atomic read-derive-write transaction across
 * pages+links tables. That's fine for single-user local-first — no contention.
 */

import { db } from '@/data/db';
import { extractWikiLinks } from '@/core/parser/wiki-links';
import { resolveTitle } from '@/core/vault/page-index';
import type { Link } from '@/data/schema';
import { nanoid } from '@/lib/nanoid';

/**
 * Rebuild the link rows for a single source page. Call after every save.
 */
export async function rebuildPageLinks(
  sourcePageId: string,
  sourceContent: string,
): Promise<void> {
  const links = extractWikiLinks(sourceContent);
  // Resolve targets one by one against Dexie. This is async but fast (indexed
  // lookups). Sequential keeps order deterministic for debuggability.
  const nowRows: Link[] = [];
  for (const l of links) {
    nowRows.push({
      id: nanoid(),
      sourcePageId,
      sourceBlockId: null,
      targetTitle: l.target,
      targetPageId: await resolveTitle(l.target),
      kind: l.isEmbed ? 'embed' : 'wiki',
      position: l.position,
    });
  }

  await db.transaction('rw', db.links, async () => {
    await db.links.where('sourcePageId').equals(sourcePageId).delete();
    if (nowRows.length > 0) await db.links.bulkAdd(nowRows);
  });
}

/**
 * Backlinks for a page = rows where targetPageId == this page.
 * Wrapped in liveQuery via Dexie's reactive helpers for the UI.
 */
export async function getBacklinksForPage(pageId: string): Promise<Link[]> {
  return db.links.where('targetPageId').equals(pageId).toArray();
}

/**
 * Uncreated (ghost) links from a source page = outgoing links whose target
 * resolved to null. Used by the "Uncreated links" section in the backlinks panel.
 */
export async function getGhostLinksFromPage(
  sourcePageId: string,
): Promise<Link[]> {
  const all = await db.links.where('sourcePageId').equals(sourcePageId).toArray();
  return all.filter((l) => l.targetPageId === null);
}