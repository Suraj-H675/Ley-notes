/**
 * usePages — React hook that returns all live (non-deleted) pages, sorted by
 * most recently updated first. Backed by Dexie's liveQuery so the UI updates
 * automatically on writes from anywhere in the app.
 */

import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';
import type { Page } from '@/infrastructure/database/schema';

export function usePages(): Page[] | undefined {
  return useLiveQuery(async () => {
    const all = await db.pages.toArray();
    return all
      // A cache-only recovery projection stays addressable by its open tab,
      // but must not masquerade as a file that still exists in the vault.
      .filter((p) => p.deletedAt === null && !p.missingFromDisk)
      .sort((a, b) => b.updatedAt - a.updatedAt);
  }, []);
}

export function usePageById(id: string | null): Page | undefined {
  return useLiveQuery(
    async () => (id ? (await db.pages.get(id)) ?? undefined : undefined),
    [id],
  );
}

/**
 * Recent pages in MRU order, capped at N. The Zustand nav store has the IDs;
 * callers (e.g. RecentPane) wire those IDs into a Page[] themselves.
 */
export function useRecentPages(/** pageIds from the nav store, in MRU order */ ids: string[]): Page[] {
  return (
    useLiveQuery(async () => {
      if (ids.length === 0) return [];
      const rows = await db.pages.where('id').anyOf(ids).toArray();
      const byId = new Map(rows.map((p) => [p.id, p]));
      // Preserve MRU order.
      const out: Page[] = [];
      for (const id of ids) {
        const p = byId.get(id);
        if (p && p.deletedAt === null && !p.missingFromDisk) out.push(p);
      }
      return out;
    }, [ids]) ?? []
  );
}
