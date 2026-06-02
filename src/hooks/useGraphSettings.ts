import { useCallback, useEffect } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import {
  ensureDefaultGraphSettings,
  upsertGraphSettings,
} from '@/lib/db/graphSettings';
import type { GraphScope, GraphSettings } from '@/types/graph-settings.types';

export function useGraphSettings(scope: GraphScope) {
  // Seed defaults once on first mount.
  useEffect(() => {
    void ensureDefaultGraphSettings();
  }, []);

  const settings = useLiveQuery<GraphSettings | null, null>(
    async () => {
      const row = await db.graphSettings.get(scope);
      return row ? (row as GraphSettings) : null;
    },
    [scope],
    null
  ) as GraphSettings | null;

  const update = useCallback(
    async (next: GraphSettings) => {
      await upsertGraphSettings({ ...next, scope });
    },
    [scope]
  );

  return { settings, update };
}
