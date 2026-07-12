import { beforeEach, describe, expect, it } from 'vitest';
import { db } from './db';
import { activeDataKind, BROWSER_LOCAL_KIND, restoreBrowserLocalVault, stashBrowserLocalVault } from './browser-local-vault';
import { makePage, resetDb } from '@/test/helpers';
import { projectFilesIntoCache } from '@/infrastructure/vault/filesystem-vault';

describe('active vault data isolation', () => {
  beforeEach(async () => resetDb());

  it('stashes and restores browser-local pages, assets, and history around a folder projection', async () => {
    const local = makePage({ id: 'local-page', title: 'Private local thought', content: 'Only in IndexedDB' });
    await db.pages.add(local);
    await db.assets.add({ id: 'asset', pageId: local.id, filename: 'attachments/local.png', mimeType: 'image/png', blob: new Blob(['image']), createdAt: 1 });
    await db.revisions.add({ id: 'revision', pageId: local.id, content: 'earlier', createdAt: 1 });
    await db.settings.put({ key: 'active-data-kind', value: BROWSER_LOCAL_KIND });

    expect(await stashBrowserLocalVault()).toBe(1);
    await projectFilesIntoCache('browser-folder:Work', [{ path: 'Folder note.md', content: '# Folder', createdAt: 2, updatedAt: 2 }]);
    expect((await db.pages.toArray()).map((page) => page.title)).toEqual(['Folder note']);
    expect(await db.assets.count()).toBe(0);

    expect(await restoreBrowserLocalVault()).toBe(true);
    expect(await activeDataKind()).toBe(BROWSER_LOCAL_KIND);
    expect((await db.pages.get(local.id))?.content).toBe('Only in IndexedDB');
    expect(await db.assets.get('asset')).toBeTruthy();
    expect(await db.revisions.get('revision')).toBeTruthy();
  });

  it('preserves runtime note identity when the same filesystem vault is rescanned', async () => {
    const snapshot = { path: 'projects/roadmap.md', content: '# Roadmap', createdAt: 2, updatedAt: 2 };
    await projectFilesIntoCache('/vault/one', [snapshot]);
    const projected = (await db.pages.toArray())[0];
    await db.pages.delete(projected.id);
    await db.pages.add({ ...projected, id: 'runtime-id', updatedAt: 3 });

    await projectFilesIntoCache('/vault/one', [{ ...snapshot, content: '# Updated', updatedAt: 4 }]);
    expect((await db.pages.toArray())[0].id).toBe('runtime-id');
    expect((await db.pages.toArray())[0].content).toBe('# Updated');
  });

  it('does not leak identities or revisions into a different filesystem vault', async () => {
    const snapshot = { path: 'same.md', content: 'One', createdAt: 1, updatedAt: 1 };
    await projectFilesIntoCache('/vault/one', [snapshot]);
    const firstId = (await db.pages.toArray())[0].id;
    await db.revisions.add({ id: 'old-revision', pageId: firstId, content: 'old', createdAt: 1 });

    await projectFilesIntoCache('/vault/two', [{ ...snapshot, content: 'Two' }]);
    expect((await db.pages.toArray())[0].id).not.toBe(firstId);
    expect(await db.revisions.count()).toBe(0);
  });
});
