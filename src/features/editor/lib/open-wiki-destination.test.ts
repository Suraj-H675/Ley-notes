import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Mock } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { makePage, resetDb } from '@/test/helpers';
import { createPage } from '@/core/vault/pages';
import type { EditorPane, NavState } from '@/shared/state/nav';
import { openMarkdownDestination, openWikiDestination } from './open-wiki-destination';

function assignPane(pageId: string, requestedPane: EditorPane) {
  if (requestedPane === 'secondary' && pageId === nav.primaryTab) {
    Object.assign(nav, { activePane: 'primary', activeTab: pageId });
    return;
  }
  if (requestedPane === 'primary' && pageId === nav.secondaryTab) {
    Object.assign(nav, { activePane: 'secondary', activeTab: pageId });
    return;
  }
  Object.assign(nav, {
    openTabs: nav.openTabs?.includes(pageId) ? nav.openTabs : [...(nav.openTabs ?? []), pageId],
    activeTab: pageId,
    activePane: requestedPane,
    primaryTab: requestedPane === 'primary' ? pageId : nav.primaryTab,
    secondaryTab: requestedPane === 'secondary' ? pageId : nav.secondaryTab,
  });
}

const nav = {
  openPage: vi.fn((pageId: string, pane?: EditorPane) => assignPane(pageId, pane ?? 'primary')),
  pushRecent: vi.fn(),
} as unknown as Omit<NavState, 'openPage' | 'pushRecent' | 'setActiveTab'> & {
  openPage: Mock<(pageId: string, pane?: EditorPane) => void>;
  pushRecent: Mock<(pageId: string) => void>;
};

function openInSplit(pageId: string) {
  if (pageId === nav.primaryTab) return;
  Object.assign(nav, {
    openTabs: nav.openTabs?.includes(pageId) ? nav.openTabs : [...(nav.openTabs ?? []), pageId],
    activeTab: pageId,
    activePane: 'secondary',
    primaryTab: nav.primaryTab ?? nav.activeTab ?? pageId,
    secondaryTab: pageId,
  });
}

function focusPane(pane: EditorPane) {
  const activeTab = pane === 'primary' ? nav.primaryTab : nav.secondaryTab;
  if (!activeTab) return;
  Object.assign(nav, { activePane: pane, activeTab });
}

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

describe('wiki destination pane routing', () => {
  beforeEach(() => {
    Object.assign(nav, {
      openTabs: [],
      activeTab: null,
      primaryTab: null,
      secondaryTab: null,
      activePane: 'primary',
      recentPages: [],
    });
    for (const mock of [nav.openPage, nav.pushRecent]) mock.mockClear();
  });

  it('focuses an opposite-pane destination instead of duplicating it', async () => {
    await db.pages.put(makePage({ id: 'source-id', title: 'Source' }));
    const reference = makePage({ id: 'reference-id', title: 'Reference' });
    await db.pages.put(reference);
    nav.openPage?.('source-id');
    openInSplit('reference-id');
    focusPane('primary');
    nav.openPage.mockClear();

    await openWikiDestination({ target: 'Reference' }, 'primary');

    expect(nav).toMatchObject({
      primaryTab: 'source-id',
      secondaryTab: 'reference-id',
      activePane: 'secondary',
      activeTab: 'reference-id',
      openTabs: ['source-id', 'reference-id'],
    });
  });

  it('focuses the opposite pane when its visible note is opened again', async () => {
    await db.pages.put(makePage({ id: 'source-id', title: 'Source' }));
    await db.pages.put(makePage({ id: 'reference-id', title: 'Reference' }));
    nav.openPage?.('source-id');
    openInSplit('reference-id');
    focusPane('secondary');

    await openWikiDestination({ target: 'Source' }, 'secondary');

    expect(nav).toMatchObject({
      primaryTab: 'source-id',
      secondaryTab: 'reference-id',
      activePane: 'primary',
      activeTab: 'source-id',
    });
  });

  it('routes relative Markdown links through their originating pane', async () => {
    const source = makePage({ id: 'source-id', title: 'Source' });
    await db.pages.put(source);
    await db.pages.put({ ...makePage({ id: 'target-id', title: 'Target note' }), path: 'Target.md' });
    await db.pages.put({ ...makePage({ id: 'reference-id', title: 'Reference' }), path: 'Reference.md' });
    nav.openPage?.('source-id');
    openInSplit('reference-id');
    focusPane('secondary');

    const opened = await openMarkdownDestination(source.path, 'Target.md', undefined, undefined, 'secondary');

    expect(opened).toBe(true);
    expect(nav).toMatchObject({
      primaryTab: 'source-id',
      secondaryTab: 'target-id',
      activePane: 'secondary',
      activeTab: 'target-id',
    });
  });
});
