import { db } from '@/infrastructure/database/db';
import { activeDataKind } from '@/infrastructure/database/browser-local-vault';

// Preserve the historical key so upgrading from the former “Favorites” UI
// never strands a user's starred notes.
const NOTE_BOOKMARKS_PREFIX = 'favorites:';

export function noteBookmarksDataKey(kind: string | null): string {
  return `${NOTE_BOOKMARKS_PREFIX}${kind ?? 'unselected'}`;
}

export async function listBookmarkedPageIds(): Promise<string[]> {
  return listBookmarkedPageIdsAtKey(await activeNoteBookmarksKey());
}

export async function isPageBookmarked(pageId: string): Promise<boolean> {
  return (await listBookmarkedPageIds()).includes(pageId);
}

export async function setPageBookmarked(pageId: string, bookmarked: boolean): Promise<void> {
  const key = await activeNoteBookmarksKey();
  const current = await listBookmarkedPageIdsAtKey(key);
  const next = bookmarked
    ? current.includes(pageId) ? current : [...current, pageId]
    : current.filter((id) => id !== pageId);
  await db.settings.put({ key, value: next });
}

export async function togglePageBookmark(pageId: string): Promise<boolean> {
  const next = !await isPageBookmarked(pageId);
  await setPageBookmarked(pageId, next);
  return next;
}

export function removePageBookmarkReference(pageId: string): Promise<void> {
  return setPageBookmarked(pageId, false);
}

async function listBookmarkedPageIdsAtKey(key: string): Promise<string[]> {
  const value = (await db.settings.get(key))?.value;
  if (!Array.isArray(value)) return [];
  return value.filter((id): id is string => typeof id === 'string');
}

async function activeNoteBookmarksKey(): Promise<string> {
  return noteBookmarksDataKey(await activeDataKind());
}
