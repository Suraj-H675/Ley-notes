import { describe, expect, it, beforeEach } from 'vitest';
import { db } from '@/infrastructure/database/db';
import {
  createPage,
  deletePage,
  duplicatePage,
  getPageByTitle,
  listDeletedPages,
  listPages,
  movePage,
  permanentlyDeletePage,
  renamePage,
  restorePage,
  updatePageContent,
  updatePageFrontmatter,
  updatePageProperty,
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

  it('keeps the newest content and derived indexes during rapid saves', async () => {
    const page = await createPage({ title: 'Rapid capture' });
    await Promise.all([
      updatePageContent(page.id, '#project/ley'),
      updatePageContent(page.id, '#project/ley #project/research #status/active'),
    ]);
    expect((await db.pages.get(page.id))?.content).toBe(
      '#project/ley #project/research #status/active',
    );
    expect(
      (await db.tags.where('pageId').equals(page.id).toArray())
        .map((row) => row.tag)
        .sort(),
    ).toEqual(['project/ley', 'project/research', 'status/active']);
  });

  it('serializes content and property edits without losing either revision', async () => {
    const page = await createPage({ title: 'Concurrent edits' });
    await Promise.all([
      updatePageContent(page.id, 'New body #active'),
      updatePageFrontmatter(page.id, { status: 'active', priority: 2 }),
    ]);
    expect(await db.pages.get(page.id)).toMatchObject({
      content: 'New body #active',
      frontmatter: { status: 'active', priority: 2 },
    });
    expect(await db.tags.get([page.id, 'active'])).toBeTruthy();
  });

  it('merges rapid cell-level property edits against the newest frontmatter', async () => {
    const page = await createPage({ title: 'Property table' });
    await Promise.all([
      updatePageProperty(page.id, 'status', 'active'),
      updatePageProperty(page.id, 'priority', 2),
    ]);
    expect((await db.pages.get(page.id))?.frontmatter).toEqual({ status: 'active', priority: 2 });
  });

  it('indexes relative Markdown links by resolved vault path', async () => {
    const target = await createPage({ title: 'Design', folder: 'docs' });
    const source = await createPage({ title: 'Source', folder: 'projects', content: '[Design](../docs/design.md#API)' });
    const link = await db.links.where('sourcePageId').equals(source.id).first();
    expect(link).toMatchObject({ targetPageId: target.id, targetTitle: 'Design', kind: 'markdown' });
  });

  it('resolves a missing relative Markdown link when its file is later created at that path', async () => {
    const source = await createPage({ title: 'Source', folder: 'projects', content: '[Future](../docs/future.md)' });
    expect((await db.links.where('sourcePageId').equals(source.id).first())?.targetPageId).toBeNull();
    const target = await createPage({ title: 'Future', folder: 'docs' });
    expect((await db.links.where('sourcePageId').equals(source.id).first())?.targetPageId).toBe(target.id);
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

  it('moves a page between folders without changing its identity', async () => {
    const page = await createPage({ title: 'Roadmap', folder: 'projects/ley' });
    const moved = await movePage(page.id, 'archive/2026');
    expect(moved.id).toBe(page.id);
    expect(moved.title).toBe('Roadmap');
    expect(moved.path).toBe('archive/2026/roadmap.md');
    expect((await db.pages.get(page.id))?.path).toBe('archive/2026/roadmap.md');
  });

  it('retargets incoming Markdown links when their target moves or is renamed', async () => {
    const target = await createPage({ title: 'Design', folder: 'docs' });
    const source = await createPage({ title: 'Source', folder: 'projects', content: '[Design](../docs/design.md#API)' });
    await movePage(target.id, 'archive');
    expect((await db.pages.get(source.id))?.content).toBe('[Design](../archive/design.md#API)');
    await renamePage(target.id, 'Design System');
    expect((await db.pages.get(source.id))?.content).toBe('[Design](../archive/design-system.md#API)');
  });

  it('rebases outgoing Markdown links when their source moves', async () => {
    await createPage({ title: 'Design', folder: 'docs' });
    const source = await createPage({ title: 'Source', folder: 'projects', content: '[Design](../docs/design.md)' });
    await movePage(source.id, 'projects/active');
    expect((await db.pages.get(source.id))?.content).toBe('[Design](../../docs/design.md)');
    expect((await db.links.where('sourcePageId').equals(source.id).first())?.targetTitle).toBe('Design');
  });

  it('moves a nested page back to the vault root', async () => {
    const page = await createPage({ title: 'Inbox', folder: 'capture' });
    expect((await movePage(page.id, '')).path).toBe('inbox.md');
  });

  it('rejects unsafe move destinations and path collisions', async () => {
    await createPage({ title: 'One', folder: 'target' });
    const another = await createPage({ title: 'One!', folder: 'source' });
    await expect(movePage(another.id, '../outside')).rejects.toThrow(/safe folder/);
    await expect(movePage(another.id, 'target')).rejects.toThrow(/already exists/);
  });

  it('duplicates content and properties but removes ambiguous aliases', async () => {
    const page = await createPage({
      title: 'Project brief',
      folder: 'projects',
      content: 'Links to [[Welcome]].',
      frontmatter: { status: 'active', aliases: ['Brief'] },
    });
    const copy = await duplicatePage(page.id);
    expect(copy.title).toBe('Project brief copy');
    expect(copy.path).toBe('projects/project-brief-copy.md');
    expect(copy.content).toBe(page.content);
    expect(copy.frontmatter).toEqual({ status: 'active' });
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

  it('lists and restores browser-local deleted notes with rebuilt indexes', async () => {
    const target = await createPage({ title: 'Target' });
    const page = await createPage({ title: 'Recover me', content: 'See [[Target]] and #recovered.' });
    await deletePage(page.id);
    expect((await listDeletedPages()).map((candidate) => candidate.id)).toEqual([page.id]);
    const restored = await restorePage(page.id);
    expect(restored.deletedAt).toBeNull();
    expect((await db.links.where('sourcePageId').equals(page.id).first())?.targetPageId).toBe(target.id);
    expect(await db.tags.get([page.id, 'recovered'])).toBeTruthy();
  });

  it('allows a title to be recreated but prevents restoring over it', async () => {
    const deleted = await createPage({ title: 'Reusable title' });
    await deletePage(deleted.id);
    const replacement = await createPage({ title: 'Reusable title' });
    expect(replacement.id).not.toBe(deleted.id);
    await expect(restorePage(deleted.id)).rejects.toThrow(/current note/);
  });

  it('permanently deletes only recycled notes and their private data', async () => {
    const page = await createPage({ title: 'Disposable' });
    const source = await createPage({ title: 'Source', content: 'Still references [[Disposable]].' });
    await db.revisions.add({ id: 'revision', pageId: page.id, content: 'old', createdAt: Date.now() });
    await deletePage(page.id);
    await permanentlyDeletePage(page.id);
    expect(await db.pages.get(page.id)).toBeUndefined();
    expect(await db.revisions.where('pageId').equals(page.id).count()).toBe(0);
    expect((await db.links.where('sourcePageId').equals(source.id).first())?.targetPageId).toBeNull();
  });

  it('getPageByTitle is case-insensitive', async () => {
    await createPage({ title: 'CamelCase' });
    const got = await getPageByTitle('camelcase');
    expect(got).not.toBeNull();
    expect(got?.title).toBe('CamelCase');
  });
});
