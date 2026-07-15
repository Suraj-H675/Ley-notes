import { useLiveQuery } from 'dexie-react-hooks';
import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';
import { bookmarksDataKey, parseBookmarksSetting, type DestinationBookmark } from '@/core/vault/bookmarks';
import type { Page } from '@/infrastructure/database/schema';
import { findMarkdownDestinationLine } from '@/core/parser/destinations';

export interface ResolvedDestinationBookmark {
  bookmark: DestinationBookmark;
  page: Page | null;
  destinationAvailable: boolean;
}

export function useDestinationBookmarks(): ResolvedDestinationBookmark[] {
  return useLiveQuery(async () => {
    const key = bookmarksDataKey(await activeDataKind());
    const bookmarks = parseBookmarksSetting((await db.settings.get(key))?.value);
    if (bookmarks.length === 0) return [];
    const pages = await db.pages.filter((page) => page.deletedAt === null).toArray();
    const byId = new Map(pages.map((page) => [page.id, page]));
    const byPath = new Map(pages.map((page) => [page.path.toLowerCase(), page]));
    return bookmarks.map((bookmark) => {
      const page = byId.get(bookmark.target.pageId) ?? byPath.get(bookmark.target.path.toLowerCase()) ?? null;
      return {
        bookmark,
        page,
        destinationAvailable: Boolean(page && findMarkdownDestinationLine(
          page.content,
          bookmark.target.kind === 'heading' ? bookmark.target.anchor : null,
          bookmark.target.kind === 'block' ? bookmark.target.anchor : null,
        )),
      };
    });
  }, []) ?? [];
}
