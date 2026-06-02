import { describe, it, expect, beforeEach } from 'vitest';
import 'fake-indexeddb/auto';
import { db } from './index';
import {
  getGraphSettings,
  upsertGraphSettings,
  ensureDefaultGraphSettings,
} from './graphSettings';

describe('graphSettings CRUD', () => {
  beforeEach(async () => {
    await db.graphSettings.clear();
  });

  it('ensureDefaultGraphSettings inserts both rows on first call', async () => {
    await ensureDefaultGraphSettings();
    const rows = await db.graphSettings.toArray();
    expect(rows).toHaveLength(2);
    const scopes = rows.map((r) => r.scope).sort();
    expect(scopes).toEqual(['global', 'local']);
  });

  it('ensureDefaultGraphSettings is idempotent', async () => {
    await ensureDefaultGraphSettings();
    await ensureDefaultGraphSettings();
    const rows = await db.graphSettings.toArray();
    expect(rows).toHaveLength(2);
  });

  it('getGraphSettings returns null when row is missing', async () => {
    const s = await getGraphSettings('global');
    expect(s).toBeNull();
  });

  it('getGraphSettings returns the row when present', async () => {
    await upsertGraphSettings({
      scope: 'global',
      colorScheme: 'tag',
      physics: {
        centerForce: 1,
        chargeForce: -60,
        linkForce: 1,
        linkDistance: 80,
      },
      display: {
        nodeSize: 1,
        edgeThickness: 1,
        textFade: 0.25,
        showLabels: true,
      },
      filters: {
        searchQuery: '',
        selectedTags: [],
        selectedCollections: [],
        showOrphans: true,
      },
      panelSectionsOpen: {
        groups: true,
        filters: false,
        display: false,
        physics: false,
      },
      panelVisible: true,
      localDepth: 1,
      updatedAt: Date.now(),
    });
    const s = await getGraphSettings('global');
    expect(s).not.toBeNull();
    expect(s?.colorScheme).toBe('tag');
  });

  it('upsertGraphSettings overwrites existing row by primary key', async () => {
    await ensureDefaultGraphSettings();
    const original = await getGraphSettings('global');
    expect(original?.colorScheme).toBe('untyped');
    await upsertGraphSettings({
      ...(original as any),
      colorScheme: 'collection',
    });
    const updated = await getGraphSettings('global');
    expect(updated?.colorScheme).toBe('collection');
  });
});
