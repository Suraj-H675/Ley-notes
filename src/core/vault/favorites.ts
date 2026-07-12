import { db } from '@/infrastructure/database/db';
import { activeDataKind } from '@/infrastructure/database/browser-local-vault';

const FAVORITES_PREFIX = 'favorites:';

export async function listFavoritePageIds(): Promise<string[]> {
  const key = await activeFavoritesKey();
  const value = (await db.settings.get(key))?.value;
  if (!Array.isArray(value)) return [];
  return value.filter((id): id is string => typeof id === 'string');
}

export async function isFavoritePage(pageId: string): Promise<boolean> {
  return (await listFavoritePageIds()).includes(pageId);
}

export async function setFavoritePage(pageId: string, favorite: boolean): Promise<void> {
  const key = await activeFavoritesKey();
  const current = await listFavoritePageIds();
  const next = favorite
    ? current.includes(pageId) ? current : [...current, pageId]
    : current.filter((id) => id !== pageId);
  await db.settings.put({ key, value: next });
}

export async function toggleFavoritePage(pageId: string): Promise<boolean> {
  const next = !await isFavoritePage(pageId);
  await setFavoritePage(pageId, next);
  return next;
}

async function activeFavoritesKey(): Promise<string> {
  return `${FAVORITES_PREFIX}${await activeDataKind() ?? 'unselected'}`;
}
