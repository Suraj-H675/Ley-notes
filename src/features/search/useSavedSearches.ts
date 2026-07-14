import { useLiveQuery } from 'dexie-react-hooks';
import { parseSavedSearchesSetting, savedSearchesDataKey, type SavedSearch } from '@/core/vault/saved-searches';
import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';

export function useSavedSearches(): SavedSearch[] {
  return useLiveQuery(async () => {
    const key = savedSearchesDataKey(await activeDataKind());
    return parseSavedSearchesSetting((await db.settings.get(key))?.value);
  }, []) ?? [];
}
