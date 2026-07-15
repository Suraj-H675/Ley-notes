import { beforeEach, describe, expect, it } from 'vitest';
import { resetDb } from '@/test/helpers';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { isPageBookmarked, listBookmarkedPageIds, setPageBookmarked, togglePageBookmark } from './note-bookmarks';

describe('vault note bookmarks', () => {
  beforeEach(() => resetDb());

  it('adds, removes, and deduplicates bookmarked page ids', async () => {
    await markActiveDataKind('browser-local');
    await setPageBookmarked('one', true);
    await setPageBookmarked('one', true);
    expect(await listBookmarkedPageIds()).toEqual(['one']);
    expect(await togglePageBookmark('one')).toBe(false);
    expect(await isPageBookmarked('one')).toBe(false);
  });

  it('isolates note bookmarks by active vault identity', async () => {
    await markActiveDataKind('filesystem:/vault/a');
    await setPageBookmarked('page-a', true);
    await markActiveDataKind('filesystem:/vault/b');
    expect(await listBookmarkedPageIds()).toEqual([]);
    await setPageBookmarked('page-b', true);
    await markActiveDataKind('filesystem:/vault/a');
    expect(await listBookmarkedPageIds()).toEqual(['page-a']);
  });
});
