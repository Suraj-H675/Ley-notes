import { useEffect, useState } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';
import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import type { Page } from '@/infrastructure/database/schema';
import { noteBookmarksDataKey } from '@/core/vault/note-bookmarks';

export function useBookmarkedPageIds(): string[] {
  const [kind, setKind] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void activeDataKind().then((value) => {
      if (active) setKind(value);
    });
    return () => { active = false; };
  }, []);

  return useLiveQuery(async () => {
    const value = (await db.settings.get(noteBookmarksDataKey(await activeDataKind())))?.value;
    return Array.isArray(value) ? value.filter((id): id is string => typeof id === 'string') : [];
  }, [kind]) ?? [];
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
    const byId = new Map(pages.filter((page) => page.deletedAt === null && !page.missingFromDisk).map((page) => [page.id, page]));
    return ids.flatMap((id) => {
      const page = byId.get(id);
      return page ? [page] : [];
    });
  }, [ids.join('\0')]) ?? [];
}
