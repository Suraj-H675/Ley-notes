import { describe, expect, it, beforeEach } from 'vitest';
import { db } from './db';
import { makePage, resetDb } from '@/test/helpers';

describe('db schema', () => {
  beforeEach(async () => {
    await resetDb();
  });

  it('opens all tables', async () => {
    expect(db).toBeDefined();
    expect(db.pages).toBeDefined();
    expect(db.blocks).toBeDefined();
    expect(db.links).toBeDefined();
    expect(db.tags).toBeDefined();
    expect(db.assets).toBeDefined();
    expect(db.revisions).toBeDefined();
    expect(db.settings).toBeDefined();
    // Legacy browser-app tables remain readable until the SQLite migration
    // defines explicit import/export handling for old local browser data.
    expect(db.browserLocalPages).toBeDefined();
    expect(db.browserLocalAssets).toBeDefined();
    expect(db.browserLocalRevisions).toBeDefined();
  });

  it('inserts and reads a page', async () => {
    const p = makePage({ title: 'Hello' });
    await db.pages.add(p);
    const got = await db.pages.get(p.id);
    expect(got?.title).toBe('Hello');
    expect(got?.lcTitle).toBe('hello');
  });

  it('indexes pages by lcTitle', async () => {
    await db.pages.bulkAdd([
      makePage({ title: 'Foo' }),
      makePage({ title: 'Bar' }),
      makePage({ title: 'Baz' }),
    ]);
    const result = await db.pages.where('lcTitle').equals('foo').toArray();
    expect(result).toHaveLength(1);
    expect(result[0].title).toBe('Foo');
  });
});
