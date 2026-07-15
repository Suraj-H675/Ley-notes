import { beforeEach, describe, expect, it } from 'vitest';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { resetDb } from '@/test/helpers';
import {
  addDestinationBookmark,
  deleteDestinationBookmark,
  listDestinationBookmarks,
  parseBookmarksSetting,
  renameDestinationBookmark,
  toggleDestinationBookmark,
} from './bookmarks';

const heading = { kind: 'heading' as const, pageId: 'page-1', path: 'Notes/First.md', anchor: 'Design' };

describe('destination bookmarks', () => {
  beforeEach(async () => {
    await resetDb();
    await markActiveDataKind('browser-local');
  });

  it('adds one resilient destination and deduplicates the same target', async () => {
    const first = await addDestinationBookmark(heading);
    const duplicate = await addDestinationBookmark({ ...heading, anchor: 'design' }, 'Duplicate');
    expect(duplicate.id).toBe(first.id);
    expect(await listDestinationBookmarks()).toEqual([first]);
  });

  it('renames and deletes a bookmark without changing its target', async () => {
    const bookmark = await addDestinationBookmark(heading);
    await renameDestinationBookmark(bookmark.id, 'Architecture decision');
    expect(await listDestinationBookmarks()).toEqual([expect.objectContaining({ title: 'Architecture decision', target: heading })]);
    await deleteDestinationBookmark(bookmark.id);
    expect(await listDestinationBookmarks()).toEqual([]);
  });

  it('toggles a destination without creating duplicate records', async () => {
    expect(await toggleDestinationBookmark(heading)).toBe(true);
    expect(await toggleDestinationBookmark(heading)).toBe(false);
    expect(await listDestinationBookmarks()).toEqual([]);
  });

  it('isolates bookmarks by vault identity', async () => {
    await markActiveDataKind('filesystem:/vault/a');
    await addDestinationBookmark(heading);
    await markActiveDataKind('filesystem:/vault/b');
    expect(await listDestinationBookmarks()).toEqual([]);
  });

  it('ignores malformed persisted records and validates user input', async () => {
    const bookmark = await addDestinationBookmark(heading);
    expect(parseBookmarksSetting([bookmark, { id: 'broken' }, null])).toEqual([bookmark]);
    await expect(addDestinationBookmark({ ...heading, anchor: ' ' })).rejects.toThrow('need a note');
    await expect(renameDestinationBookmark(bookmark.id, 'x'.repeat(81))).rejects.toThrow('80 characters');
  });
});
