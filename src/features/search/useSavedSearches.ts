import { useLiveQuery } from 'dexie-react-hooks';
import { listSavedSearches, type SavedSearch } from '@/core/vault/saved-searches';

export function useSavedSearches(): SavedSearch[] {
  return useLiveQuery(() => listSavedSearches(), []) ?? [];
}
