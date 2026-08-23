import { useEffect, useState } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { parseSavedSearchesSetting, savedSearchesDataKey, type SavedSearch } from '@/core/vault/saved-searches';
import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';

export function useSavedSearches(): SavedSearch[] {
  const [kind, setKind] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void activeDataKind().then((value) => {
      if (active) setKind(value);
    });
    return () => { active = false; };
  }, []);

  return useLiveQuery(async () => {
    const key = savedSearchesDataKey(await activeDataKind());
    return parseSavedSearchesSetting((await db.settings.get(key))?.value);
  }, [kind]) ?? [];
}
