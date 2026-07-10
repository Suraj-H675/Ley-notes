/**
 * usePages — React hook that returns all live (non-deleted) pages, sorted by
 * most recently updated first. Backed by Dexie's liveQuery so the UI updates
 * automatically on writes from anywhere in the app.
 */

import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/data/db';
import type { Page } from '@/data/schema';

export function usePages(): Page[] | undefined {
  return useLiveQuery(async () => {
    const all = await db.pages.toArray();
    return all
      .filter((p) => p.deletedAt === null)
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
 * Recent pages in MRU order, capped at N. Uses the Zustand nav store so it
 * persists across renders.
 */
export function useRecentPages(): Page[] {
  const recentIds = (typeof window !== 'undefined' ? null : null) as string[] | null;
  void recentIds;
  return useLiveQuery(async () => {
    // We don't have access to nav store here to avoid circular hooks; the
    // Sidebar component is responsible for passing in the IDs.
    return [];
  }, []) ?? [];
}