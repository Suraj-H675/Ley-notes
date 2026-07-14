import { beforeEach, describe, expect, it } from 'vitest';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { resetDb } from '@/test/helpers';
import { deleteSavedSearch, listSavedSearches, renameSavedSearch, saveSearch, updateSavedSearchTable } from './saved-searches';

describe('saved searches', () => {
  beforeEach(async () => {
    await resetDb();
    await markActiveDataKind('browser-local');
  });

  it('creates, updates duplicate queries, renames, and deletes searches', async () => {
    const saved = await saveSearch('Active research', 'tag:research property:status=active');
    const updated = await saveSearch('Current research', 'tag:research property:status=active');
    expect(updated.id).toBe(saved.id);
    expect(await listSavedSearches()).toHaveLength(1);
    await renameSavedSearch(saved.id, 'Research now');
    expect((await listSavedSearches())[0]?.name).toBe('Research now');
    await deleteSavedSearch(saved.id);
    expect(await listSavedSearches()).toEqual([]);
  });

  it('isolates searches by active vault identity', async () => {
    await markActiveDataKind('filesystem:/vault/a');
    await saveSearch('A', 'path:a');
    await markActiveDataKind('filesystem:/vault/b');
    expect(await listSavedSearches()).toEqual([]);
    await saveSearch('B', 'path:b');
    await markActiveDataKind('filesystem:/vault/a');
    expect((await listSavedSearches()).map((item) => item.name)).toEqual(['A']);
  });

  it('rejects empty and excessively long user input', async () => {
    await expect(saveSearch('', 'tag:work')).rejects.toThrow('name');
    await expect(saveSearch('Work', '   ')).rejects.toThrow('query');
    await expect(saveSearch('x'.repeat(81), 'tag:work')).rejects.toThrow('80');
  });

  it('persists a sanitized table presentation with the saved query', async () => {
    const saved = await saveSearch('Projects', 'tag:project');
    await updateSavedSearchTable(saved.id, {
      columns: ['tags', 'property:status', 'property:status', 'modified'],
      sort: { column: 'property:status', direction: 'asc' },
    });
    expect((await listSavedSearches())[0]?.table).toEqual({
      columns: ['tags', 'property:status', 'modified'],
      sort: { column: 'property:status', direction: 'asc' },
    });
  });

  it('keeps the newest presentation during rapid layout changes', async () => {
    const saved = await saveSearch('Projects', 'tag:project');
    await Promise.all([
      updateSavedSearchTable(saved.id, { columns: ['tags'], sort: { column: 'tags', direction: 'asc' } }),
      updateSavedSearchTable(saved.id, { columns: ['property:status'], sort: { column: 'title', direction: 'desc' } }),
    ]);
    expect((await listSavedSearches())[0]?.table).toEqual({
      columns: ['property:status'],
      sort: { column: 'title', direction: 'desc' },
    });
  });

  it('never redirects a saved layout to another vault identity', async () => {
    await markActiveDataKind('filesystem:/vault/a');
    const saved = await saveSearch('A projects', 'tag:project');
    await updateSavedSearchTable(saved.id, { columns: ['property:status'], sort: { column: 'title', direction: 'asc' } });
    await markActiveDataKind('filesystem:/vault/b');
    expect(await listSavedSearches()).toEqual([]);
    await markActiveDataKind('filesystem:/vault/a');
    expect((await listSavedSearches())[0]?.table?.columns).toEqual(['property:status']);
  });
});
