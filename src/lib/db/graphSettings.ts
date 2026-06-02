import { db } from './index';
import { defaultGraphSettings } from './defaultGraphSettings';
import type { GraphScope, GraphSettings } from '@/types/graph-settings.types';

export async function getGraphSettings(
  scope: GraphScope
): Promise<GraphSettings | null> {
  const row = await db.graphSettings.get(scope);
  return row ? (row as GraphSettings) : null;
}

export async function upsertGraphSettings(
  settings: GraphSettings
): Promise<void> {
  await db.graphSettings.put({ ...settings, updatedAt: Date.now() });
}

export async function ensureDefaultGraphSettings(): Promise<void> {
  const existing = await db.graphSettings.count();
  if (existing === 0) {
    await db.graphSettings.bulkPut([
      defaultGraphSettings('global'),
      defaultGraphSettings('local'),
    ]);
  }
}
