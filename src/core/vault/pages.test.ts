import { describe, expect, it, beforeEach } from 'vitest';
import { db } from '@/infrastructure/database/db';
import {
  createPage,
  deletePage,
  getPageByTitle,
  listPages,
  renamePage,
  updatePageContent,
} from './pages';
import { resetDb } from '@/test/helpers';

describe('pages CRUD', () => {
  beforeEach(async () => {
    await resetDb();
  });

  it('creates a page with derived path', async () => {
    const p = await createPage({ title: 'Hello World' });
    expect(p.path).toBe('hello-world.md');
    expect(p.lcTitle).toBe('hello world');
  });

  it('slugifies special characters and falls back to "untitled"', async () => {
    const p = await createPage({ title: '???' });
    expect(p.path).toBe('untitled.md');
  });

  it('returns the existing page on duplicate title (idempotent)', async () => {
    const a = await createPage({ title: 'Foo' });
    const b = await createPage({ title: 'Foo' });
    expect(a.id).toBe(b.id);
  });

  it('makes path unique when titles differ but slugify the same', async () => {
    await createPage({ title: 'foo-bar' });
    const p2 = await createPage({ title: 'Foo Bar' });
    expect(p2.path).toBe('foo-bar-2.md');
  });

  it('places in folder when specified', async () => {
    const p = await createPage({ title: 'Inbox', folder: 'daily' });
    expect(p.path).toBe('daily/inbox.md');
  });

  it('is idempotent on title collision', async () => {
    const a = await createPage({ title: 'Foo' });
    const b = await createPage({ title: 'foo' });
    expect(a.id).toBe(b.id);
  });

  it('updates content and rebuilds backlink index', async () => {
    const foo = await createPage({ title: 'Foo' });
    const bar = await createPage({ title: 'Bar', content: 'initial' });
    await updatePageContent(bar.id, 'see [[Foo]] for context');
    const links = await db.links.where('sourcePageId').equals(bar.id).toArray();
    expect(links).toHaveLength(1);
    expect(links[0].targetPageId).toBe(foo.id);
    expect(links[0].targetTitle).toBe('Foo');
  });

  it('extracts frontmatter aliases on save', async () => {
    const p = await createPage({ title: 'Foo' });
    await updatePageContent(p.id, '---\naliases: [FooBar, The F]\n---\nbody');
    const reloaded = await db.pages.get(p.id);
    expect(reloaded?.aliases).toEqual(['FooBar', 'The F']);
  });

  it('renames and updates path', async () => {
    const p = await createPage({ title: 'Old Name' });
    const renamed = await renamePage(p.id, 'New Name');
    expect(renamed.title).toBe('New Name');
    expect(renamed.path).toBe('new-name.md');
  });

  it('rename rejects collision', async () => {
    await createPage({ title: 'Foo' });
    const bar = await createPage({ title: 'Bar' });
    await expect(renamePage(bar.id, 'Foo')).rejects.toThrow(/already exists/);
  });

  it('soft-deletes a page and removes its link/tag rows', async () => {
    const p = await createPage({ title: 'Foo', content: 'see [[Bar]]', folder: 'x' });
    await createPage({ title: 'Bar' });
    await updatePageContent(p.id, 'see [[Bar]] and #tag');
    await deletePage(p.id);
    const reloaded = await db.pages.get(p.id);
    expect(reloaded?.deletedAt).not.toBeNull();
    const links = await db.links.where('sourcePageId').equals(p.id).toArray();
    expect(links).toHaveLength(0);
    const tags = await db.tags.where('pageId').equals(p.id).toArray();
    expect(tags).toHaveLength(0);
  });

  it('listPages excludes soft-deleted', async () => {
    const a = await createPage({ title: 'Keep' });
    const b = await createPage({ title: 'Drop' });
    await deletePage(b.id);
    const live = await listPages();
    expect(live.map((p) => p.id).sort()).toEqual([a.id].sort());
  });

  it('getPageByTitle is case-insensitive', async () => {
    await createPage({ title: 'CamelCase' });
    const got = await getPageByTitle('camelcase');
    expect(got).not.toBeNull();
    expect(got?.title).toBe('CamelCase');
  });
});