import { describe, expect, it, beforeEach } from 'vitest';
import { db } from './db';
import { makePage, resetDb } from '@/test/helpers';
import { seedIfEmpty } from './seed';

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

describe('seedIfEmpty', () => {
  beforeEach(async () => {
    await resetDb();
  });

  it('seeds a Welcome page on first run', async () => {
    await seedIfEmpty();
    const welcome = await db.pages.where('lcTitle').equals('welcome').first();
    expect(welcome).toBeDefined();
    expect(welcome?.content).toContain('Welcome to Ley');
  });

  it('seeds default settings', async () => {
    await seedIfEmpty();
    const theme = await db.settings.get('theme');
    const format = await db.settings.get('daily-note-format');
    expect(theme?.value).toBe('dark');
    expect(format?.value).toBe('yyyy-MM-dd');
  });

  it('is idempotent — does not re-seed if pages exist', async () => {
    await seedIfEmpty();
    const before = await db.pages.count();
    await db.pages.add(makePage({ title: 'Custom' }));
    await seedIfEmpty();
    const after = await db.pages.count();
    expect(after).toBe(before + 1); // only the custom page was added
  });
});
