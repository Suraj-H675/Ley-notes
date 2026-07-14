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
});
