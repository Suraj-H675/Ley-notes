/**
 * Tag index. Write-through: every time a page is saved, we replace its rows
 * in the `tags` table with the freshly-extracted set.
 *
 * Tags come from two sources:
 *  - frontmatter `tags:` field (handled by getFrontmatterTags in frontmatter.ts)
 *  - inline `#tag` syntax in the body (handled by extractInlineTags in tags.ts)
 *
 * The Tag pane in the sidebar reads from this table grouped by tag path.
 */

import { db } from '@/infrastructure/database/db';
import { extractInlineTags } from '@/core/parser/tags';
import { getFrontmatterTags } from '@/core/parser/frontmatter';

export async function rebuildPageTags(
  pageId: string,
  content: string,
  frontmatter: Record<string, unknown>,
): Promise<void> {
  const inline = extractInlineTags(content).map((t) => ({
    pageId,
    tag: t,
    source: 'inline' as const,
  }));
  const fm = getFrontmatterTags(frontmatter).map((t) => ({
    pageId,
    tag: t,
    source: 'frontmatter' as const,
  }));
  // Dedup across sources, preferring frontmatter for clarity.
  const seen = new Set<string>();
  const rows = [...fm, ...inline].filter((r) => {
    const k = `${r.pageId}::${r.tag}`;
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });

  await db.transaction('rw', db.tags, async () => {
    await db.tags.where('pageId').equals(pageId).delete();
    if (rows.length > 0) await db.tags.bulkAdd(rows);
  });
}

/**
 * Aggregate: returns Map<tag, count> across the whole vault, sorted by count desc.
 * Used by the tag pane.
 */
export async function getTagCounts(): Promise<Array<{ tag: string; count: number }>> {
  const rows = await db.tags.toArray();
  const counts = new Map<string, number>();
  for (const r of rows) {
    counts.set(r.tag, (counts.get(r.tag) ?? 0) + 1);
  }
  return [...counts.entries()]
    .map(([tag, count]) => ({ tag, count }))
    .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag));
}

/**
 * Returns page IDs that have a given tag.
 */
export async function getPagesByTag(tag: string): Promise<string[]> {
  const rows = await db.tags.where('tag').equals(tag).toArray();
  return rows.map((r) => r.pageId);
}