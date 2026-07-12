/**
 * useBacklinks — returns the live backlinks for a given page.
 *
 * Backlinks are pre-computed in the `links` table by rebuildPageLinks on save,
 * so this is an indexed lookup. Updates reactively via Dexie's liveQuery.
 */

import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';
import type { Link, Page } from '@/infrastructure/database/schema';
import { extractWikiLinks } from '@/core/parser/wiki-links';

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

export interface OutgoingLinkEntry {
  link: Link;
  target: Page | null;
}

export function useOutgoingLinks(pageId: string | null): OutgoingLinkEntry[] | undefined {
  return useLiveQuery(async () => {
    if (!pageId) return [];
    const links = await db.links.where('sourcePageId').equals(pageId).toArray();
    const ids = [...new Set(links.flatMap((link) => link.targetPageId ? [link.targetPageId] : []))];
    const targets = ids.length > 0 ? await db.pages.where('id').anyOf(ids).toArray() : [];
    const byId = new Map(targets.map((page) => [page.id, page]));
    return links.map((link) => ({ link, target: link.targetPageId ? byId.get(link.targetPageId) ?? null : null }));
  }, [pageId]);
}

export interface UnlinkedMention {
  source: Page;
  position: number;
  excerpt: string;
}

/** Text mentions of the active title that are not already inside a wiki-link. */
export function useUnlinkedMentions(pageId: string | null): UnlinkedMention[] | undefined {
  return useLiveQuery(async () => {
    if (!pageId) return [];
    const target = await db.pages.get(pageId);
    if (!target || target.title.trim().length < 3) return [];
    const needle = target.title.toLowerCase();
    const pages = await db.pages.filter((page) => page.deletedAt === null && page.id !== pageId).toArray();
    const mentions: UnlinkedMention[] = [];

    for (const source of pages) {
      const ranges = extractWikiLinks(source.content).map((link) => [link.position, link.position + link.raw.length] as const);
      const haystack = source.content.toLowerCase();
      let position = haystack.indexOf(needle);
      while (position >= 0) {
        const linked = ranges.some(([from, to]) => position >= from && position < to);
        if (!linked) {
          mentions.push({ source, position, excerpt: excerptAround(source.content, position, target.title.length) });
          break; // One actionable entry per source note keeps the panel useful.
        }
        position = haystack.indexOf(needle, position + needle.length);
      }
    }
    return mentions;
  }, [pageId]);
}

export function excerptAround(content: string, position: number, length = 1): string {
  const start = Math.max(0, position - 55);
  const end = Math.min(content.length, position + length + 90);
  return `${start > 0 ? '…' : ''}${content.slice(start, end).replace(/\s+/g, ' ').trim()}${end < content.length ? '…' : ''}`;
}
