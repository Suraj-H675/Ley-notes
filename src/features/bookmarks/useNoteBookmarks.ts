import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';
import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import type { Page } from '@/infrastructure/database/schema';
import { noteBookmarksDataKey } from '@/core/vault/note-bookmarks';

export function useBookmarkedPageIds(): string[] {
  return useLiveQuery(async () => {
    const value = (await db.settings.get(noteBookmarksDataKey(await activeDataKind())))?.value;
    return Array.isArray(value) ? value.filter((id): id is string => typeof id === 'string') : [];
  }, []) ?? [];
}

export function useIsPageBookmarked(pageId: string | null): boolean {
  const ids = useBookmarkedPageIds();
  return pageId ? ids.includes(pageId) : false;
}

export function useBookmarkedPages(): Page[] {
  const ids = useBookmarkedPageIds();
  return useLiveQuery(async () => {
    if (ids.length === 0) return [];
    const pages = await db.pages.where('id').anyOf(ids).toArray();
    const byId = new Map(pages.filter((page) => page.deletedAt === null).map((page) => [page.id, page]));
    return ids.flatMap((id) => {
      const page = byId.get(id);
      return page ? [page] : [];
    });
  }, [ids.join('\0')]) ?? [];
}
