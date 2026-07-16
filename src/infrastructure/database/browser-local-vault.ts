import { rebuildPageLinks } from '@/core/index/backlink';
import { rebuildPageTags } from '@/core/index/tag-index';
import { db } from './db';
import type { Asset, Page, Revision } from './schema';

const ACTIVE_DATA_KIND = 'active-data-kind';
export const BROWSER_LOCAL_KIND = 'browser-local';
export type BrowserStoragePersistence = 'persistent' | 'best-effort' | 'unavailable';

export async function browserStoragePersistenceStatus(): Promise<BrowserStoragePersistence> {
  if (typeof navigator === 'undefined' || !navigator.storage?.persisted) return 'unavailable';
  try {
    return await navigator.storage.persisted() ? 'persistent' : 'best-effort';
  } catch {
    return 'unavailable';
  }
}

export async function requestBrowserStoragePersistence(): Promise<BrowserStoragePersistence> {
  if (typeof navigator === 'undefined' || !navigator.storage?.persist) return 'unavailable';
  try {
    return await navigator.storage.persist() ? 'persistent' : 'best-effort';
  } catch {
    return 'unavailable';
  }
}

export async function activeDataKind(): Promise<string | null> {
  const value = (await db.settings.get(ACTIVE_DATA_KIND))?.value;
  return typeof value === 'string' ? value : null;
}

export async function markActiveDataKind(kind: string): Promise<void> {
  await db.settings.put({ key: ACTIVE_DATA_KIND, value: kind });
}

export function filesystemDataKind(vaultKey: string): string {
  return `filesystem:${vaultKey}`;
}

/** Preserve the authoritative browser-local vault before projecting a folder. */
export async function stashBrowserLocalVault(): Promise<number> {
  const kind = await activeDataKind();
  if (kind && kind !== BROWSER_LOCAL_KIND) return 0;
  const [pages, assets, revisions] = await Promise.all([
    db.pages.toArray(),
    db.assets.toArray(),
    db.revisions.toArray(),
  ]);
  await db.transaction('rw', db.browserLocalPages, db.browserLocalAssets, db.browserLocalRevisions, async () => {
    await Promise.all([
      db.browserLocalPages.clear(),
      db.browserLocalAssets.clear(),
      db.browserLocalRevisions.clear(),
    ]);
    if (pages.length > 0) await db.browserLocalPages.bulkPut(pages);
    if (assets.length > 0) await db.browserLocalAssets.bulkPut(assets);
    if (revisions.length > 0) await db.browserLocalRevisions.bulkPut(revisions);
  });
  return pages.length;
}

/** Restore the browser-local vault and rebuild its disposable indexes. */
export async function restoreBrowserLocalVault(): Promise<boolean> {
  const [pages, assets, revisions] = await Promise.all([
    db.browserLocalPages.toArray(),
    db.browserLocalAssets.toArray(),
    db.browserLocalRevisions.toArray(),
  ]);
  if (pages.length === 0) return false;
  await replaceActiveData(pages, assets, revisions);
  await markActiveDataKind(BROWSER_LOCAL_KIND);
  return true;
}

export async function clearActiveVaultData(): Promise<void> {
  await replaceActiveData([], [], []);
}

async function replaceActiveData(
  pages: Page[],
  assets: Asset[],
  revisions: Revision[],
) {
  await db.transaction('rw', [db.pages, db.blocks, db.links, db.tags, db.assets, db.revisions], async () => {
    await Promise.all([
      db.pages.clear(),
      db.blocks.clear(),
      db.links.clear(),
      db.tags.clear(),
      db.assets.clear(),
      db.revisions.clear(),
    ]);
    if (pages.length > 0) await db.pages.bulkPut(pages);
    if (assets.length > 0) await db.assets.bulkPut(assets);
    if (revisions.length > 0) await db.revisions.bulkPut(revisions);
  });
  for (const page of pages) {
    if (page.deletedAt !== null) continue;
    await rebuildPageLinks(page.id, page.content);
    await rebuildPageTags(page.id, page.content, page.frontmatter);
  }
}
