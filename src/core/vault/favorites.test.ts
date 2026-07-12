import { beforeEach, describe, expect, it } from 'vitest';
import { resetDb } from '@/test/helpers';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { isFavoritePage, listFavoritePageIds, setFavoritePage, toggleFavoritePage } from './favorites';

describe('vault favorites', () => {
  beforeEach(() => resetDb());

  it('adds, removes, and deduplicates favorite page ids', async () => {
    await markActiveDataKind('browser-local');
    await setFavoritePage('one', true);
    await setFavoritePage('one', true);
    expect(await listFavoritePageIds()).toEqual(['one']);
    expect(await toggleFavoritePage('one')).toBe(false);
    expect(await isFavoritePage('one')).toBe(false);
  });

  it('isolates favorites by active vault identity', async () => {
    await markActiveDataKind('filesystem:/vault/a');
    await setFavoritePage('page-a', true);
    await markActiveDataKind('filesystem:/vault/b');
    expect(await listFavoritePageIds()).toEqual([]);
    await setFavoritePage('page-b', true);
    await markActiveDataKind('filesystem:/vault/a');
    expect(await listFavoritePageIds()).toEqual(['page-a']);
  });
});
