import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { resetDb } from '@/test/helpers';
import { renameSavedSearch, saveSearch } from '@/core/vault/saved-searches';
import { useSavedSearches } from './useSavedSearches';

describe('useSavedSearches', () => {
  beforeEach(async () => {
    await resetDb();
    await markActiveDataKind('browser-local');
  });

  it('reacts to saved-query creation and rename without a reload', async () => {
    const { result } = renderHook(() => useSavedSearches());
    await waitFor(() => expect(result.current).toEqual([]));
    let id = '';
    await act(async () => {
      id = (await saveSearch('Active work', 'property:status=active')).id;
    });
    await waitFor(() => expect(result.current.map((search) => search.name)).toEqual(['Active work']));
    await act(async () => renameSavedSearch(id, 'Current work'));
    await waitFor(() => expect(result.current.map((search) => search.name)).toEqual(['Current work']));
  });

  it('re-reads the active vault identity before loading searches', async () => {
    const first = renderHook(() => useSavedSearches());
    await waitFor(() => expect(first.result.current).toEqual([]));
    await act(async () => saveSearch('Local query', 'tag:local'));
    await waitFor(() =>
      expect(first.result.current.map((search) => search.name)).toEqual(['Local query']),
    );

    await act(async () => markActiveDataKind('filesystem:/vault/next'));
    const second = renderHook(() => useSavedSearches());
    await waitFor(() => expect(second.result.current).toEqual([]));
    await act(async () => saveSearch('Folder query', 'tag:folder'));
    await waitFor(() =>
      expect(second.result.current.map((search) => search.name)).toEqual(['Folder query']),
    );
  });
});
