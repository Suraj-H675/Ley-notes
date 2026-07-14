import { beforeEach, describe, expect, it } from 'vitest';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { resetDb } from '@/test/helpers';
import { deleteSavedSearch, listSavedSearches, renameSavedSearch, saveSearch } from './saved-searches';

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
});
