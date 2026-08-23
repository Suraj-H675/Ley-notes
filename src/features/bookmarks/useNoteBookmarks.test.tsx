import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import {
  markActiveDataKind,
} from '@/infrastructure/database/browser-local-vault';
import { resetDb } from '@/test/helpers';
import {
  togglePageBookmark,
} from '@/core/vault/note-bookmarks';
import { useBookmarkedPageIds } from './useNoteBookmarks';

describe('note bookmark vault identity', () => {
  beforeEach(async () => {
    await resetDb();
    await markActiveDataKind('browser-local');
    await act(async () => togglePageBookmark('local-note'));
  });

  it('re-reads bookmarks when the active vault changes', async () => {
    const first = renderHook(() => useBookmarkedPageIds());
    await waitFor(() => expect(first.result.current).toEqual(['local-note']));

    await act(async () => markActiveDataKind('filesystem:/vault/next'));
    const second = renderHook(() => useBookmarkedPageIds());
    await waitFor(() => expect(second.result.current).toEqual([]));
  });
});
