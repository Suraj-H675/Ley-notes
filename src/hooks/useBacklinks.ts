/**
 * useBacklinks — returns the live backlinks for a given page.
 *
 * Backlinks are pre-computed in the `links` table by rebuildPageLinks on save,
 * so this is an indexed lookup. Updates reactively via Dexie's liveQuery.
 */

import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/data/db';
import type { Link, Page } from '@/data/schema';

export interface BacklinkEntry {
  link: Link;
  source: Page;
}

export function useBacklinks(pageId: string | null): BacklinkEntry[] | undefined {
  return useLiveQuery(
    async () => {
      if (!pageId) return [];
      const links = await db.links.where('targetPageId').equals(pageId).toArray();
      if (links.length === 0) return [];
      const sourceIds = [...new Set(links.map((l) => l.sourcePageId))];
      const sources = await db.pages.where('id').anyOf(sourceIds).toArray();
      const byId = new Map(sources.map((p) => [p.id, p]));
      const result: BacklinkEntry[] = [];
      for (const l of links) {
        const s = byId.get(l.sourcePageId);
        if (s && s.deletedAt === null) result.push({ link: l, source: s });
      }
      return result;
    },
    [pageId],
  );
}

/**
 * Ghost (uncreated) outgoing links from the active page.
 */
export function useGhostOutgoingLinks(pageId: string | null): Link[] | undefined {
  return useLiveQuery(
    async () => {
      if (!pageId) return [];
      const links = await db.links.where('sourcePageId').equals(pageId).toArray();
      return links.filter((l) => l.targetPageId === null);
    },
    [pageId],
  );
}