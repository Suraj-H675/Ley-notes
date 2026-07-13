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

import { db } from '@/infrastructure/database/db';
import { extractWikiLinks } from '@/core/parser/wiki-links';
import { extractInternalMarkdownLinks, resolveInternalMarkdownPath } from '@/core/parser/markdown-links';
import { resolveTitle } from '@/core/vault/page-index';
import type { Link, Page } from '@/infrastructure/database/schema';
import { nanoid } from '@/shared/lib/nanoid';

/**
 * Rebuild the link rows for a single source page. Call after every save.
 */
export async function rebuildPageLinks(
  sourcePageId: string,
  sourceContent: string,
): Promise<void> {
  const links = extractWikiLinks(sourceContent);
  const markdownLinks = extractInternalMarkdownLinks(sourceContent);
  const sourcePage = await db.pages.get(sourcePageId);
  const livePages = markdownLinks.length > 0 ? await db.pages.filter((page) => page.deletedAt === null).toArray() : [];
  const byPath = new Map(livePages.map((page) => [page.path.toLowerCase(), page]));
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
  if (sourcePage) {
    for (const link of markdownLinks) {
      const targetPath = resolveInternalMarkdownPath(sourcePage.path, link.path);
      if (!targetPath) continue;
      const target = byPath.get(targetPath.toLowerCase());
      nowRows.push({
        id: nanoid(),
        sourcePageId,
        sourceBlockId: null,
        targetTitle: target?.title ?? targetPath.split('/').at(-1)?.replace(/\.md$/i, '') ?? targetPath,
        targetPageId: target?.id ?? null,
        kind: 'markdown',
        position: link.position,
      });
    }
  }

  await db.transaction('rw', db.links, async () => {
    await db.links.where('sourcePageId').equals(sourcePageId).delete();
    if (nowRows.length > 0) await db.links.bulkAdd(nowRows);
  });
}

/** Resolve dangling links when their target page is eventually created. */
export async function resolveGhostLinksForPage(page: Page): Promise<void> {
  const names = new Set([page.title, ...page.aliases].map((value) => value.toLowerCase()));
  const unresolved = await db.links.filter((link) => link.targetPageId === null).toArray();
  const matching: Link[] = [];
  for (const link of unresolved) {
    if (link.kind !== 'markdown') {
      if (names.has(link.targetTitle.toLowerCase())) matching.push(link);
      continue;
    }
    const source = await db.pages.get(link.sourcePageId);
    if (!source) continue;
    const parsed = extractInternalMarkdownLinks(source.content).find((candidate) => candidate.position === link.position);
    const targetPath = parsed ? resolveInternalMarkdownPath(source.path, parsed.path) : null;
    if (targetPath?.toLowerCase() === page.path.toLowerCase()) matching.push(link);
  }
  if (matching.length === 0) return;
  await db.links.bulkPut(matching.map((link) => ({ ...link, targetPageId: page.id })));
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
