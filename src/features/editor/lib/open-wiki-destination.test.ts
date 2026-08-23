import { beforeEach, describe, expect, it, vi } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { makePage, resetDb } from '@/test/helpers';
import { createPage } from '@/core/vault/pages';
import { openMarkdownDestination, openWikiDestination } from './open-wiki-destination';

const nav = {
  openPage: vi.fn(),
  pushRecent: vi.fn(),
};

vi.mock('@/shared/state/nav', () => ({
  useNavStore: {
    getState: () => nav,
  },
}));

vi.mock('@/infrastructure/vault/filesystem-vault', async (importOriginal) => ({
  ...(await importOriginal()),
  getActiveVaultKind: vi.fn(),
  writeActiveVaultFile: vi.fn(async () => undefined),
  hashVaultSource: vi.fn(async () => 'sha256:test'),
}));

import { getActiveVaultKind } from '@/infrastructure/vault/filesystem-vault';

describe('wiki destination continuity', () => {
  beforeEach(async () => {
    await resetDb();
    for (const mock of [nav.openPage, nav.pushRecent]) mock.mockClear();
    vi.mocked(getActiveVaultKind).mockClear();
  });

  it('does not recreate an externally missing note from a wiki link', async () => {
    await db.pages.put({
      ...makePage({ id: 'missing-id', title: 'Ghost target' }),
      missingFromDisk: true,
    });
    vi.mocked(getActiveVaultKind).mockReturnValue('desktop');

    await expect(openWikiDestination({ target: 'Ghost target' })).rejects.toThrow(
      /deleted outside Ley/,
    );
    await expect(createPage({ title: 'Ghost target' })).rejects.toThrow(
      /deleted outside Ley/,
    );
    expect(nav.openPage).not.toHaveBeenCalled();
  });

  it('requires an active vault before creating a truly absent note', async () => {
    vi.mocked(getActiveVaultKind).mockReturnValue(null);

    await expect(openWikiDestination({ target: 'Brand new' })).rejects.toThrow(
      /Open a vault/,
    );
    expect(nav.openPage).not.toHaveBeenCalled();
  });

  it('ignores externally deleted projections when resolving Markdown paths', async () => {
    const source = makePage({ id: 'source-id', title: 'Source', content: '[Missing](Missing.md)' });
    await db.pages.put({
      ...makePage({ id: 'missing-path-id', title: 'Missing path note' }),
      path: 'Missing.md',
      missingFromDisk: true,
    });
    await db.pages.put(source);

    const opened = await openMarkdownDestination(source.path, 'Missing.md');
    expect(opened).toBe(false);
    expect(nav.openPage).not.toHaveBeenCalled();
  });
});
