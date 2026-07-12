import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';
import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import type { Page } from '@/infrastructure/database/schema';

export function useFavoritePageIds(): string[] {
  return useLiveQuery(async () => {
    const kind = await activeDataKind() ?? 'unselected';
    const value = (await db.settings.get(`favorites:${kind}`))?.value;
    return Array.isArray(value) ? value.filter((id): id is string => typeof id === 'string') : [];
  }, []) ?? [];
}

export function useIsFavoritePage(pageId: string | null): boolean {
  const ids = useFavoritePageIds();
  return pageId ? ids.includes(pageId) : false;
}

export function useFavoritePages(): Page[] {
  const ids = useFavoritePageIds();
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
