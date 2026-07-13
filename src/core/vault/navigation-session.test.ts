import { beforeEach, describe, expect, it } from 'vitest';
import { resetDb } from '@/test/helpers';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { createPage, deletePage, renamePage } from './pages';
import { restoreNavigationSession, saveNavigationSession, startNavigationSession, stopNavigationSession } from './navigation-session';
import { useNavStore } from '@/shared/state/nav';

describe('navigation sessions', () => {
  beforeEach(async () => {
    await stopNavigationSession();
    await resetDb();
    useNavStore.getState().reset();
  });

  it('flushes the current session before a vault reset', async () => {
    await markActiveDataKind('browser-local');
    const first = await createPage({ title: 'First' });
    const second = await createPage({ title: 'Second' });
    await startNavigationSession();
    useNavStore.getState().hydrate({ openTabs: [first.id, second.id], activeTab: second.id, recentPages: [second.id, first.id] });
    await stopNavigationSession();
    useNavStore.getState().reset();
    await restoreNavigationSession();
    expect(useNavStore.getState()).toMatchObject({ openTabs: [first.id, second.id], activeTab: second.id, recentPages: [second.id, first.id] });
  });

  it('restores tab order, active tab, and recents after a reload', async () => {
    await markActiveDataKind('browser-local');
    const first = await createPage({ title: 'First' });
    const second = await createPage({ title: 'Second' });
    useNavStore.getState().hydrate({ openTabs: [first.id, second.id], activeTab: first.id, recentPages: [second.id, first.id] });
    await saveNavigationSession();
    useNavStore.getState().reset();
    expect(await restoreNavigationSession()).toBe(true);
    expect(useNavStore.getState()).toMatchObject({ openTabs: [first.id, second.id], activeTab: first.id, recentPages: [second.id, first.id] });
  });

  it('restores both sides of a split workspace and its focused pane', async () => {
    await markActiveDataKind('browser-local');
    const source = await createPage({ title: 'Source' });
    const reference = await createPage({ title: 'Reference' });
    useNavStore.getState().hydrate({
      openTabs: [source.id, reference.id],
      activeTab: reference.id,
      primaryTab: source.id,
      secondaryTab: reference.id,
      activePane: 'secondary',
      recentPages: [reference.id, source.id],
    });
    await saveNavigationSession();
    useNavStore.getState().reset();
    await restoreNavigationSession();
    expect(useNavStore.getState()).toMatchObject({
      primaryTab: source.id,
      secondaryTab: reference.id,
      activePane: 'secondary',
      activeTab: reference.id,
    });
  });

  it('treats activating an existing tab as a recent visit', async () => {
    const first = await createPage({ title: 'First' });
    const second = await createPage({ title: 'Second' });
    useNavStore.getState().hydrate({ openTabs: [first.id, second.id], activeTab: second.id, recentPages: [second.id] });
    useNavStore.getState().setActiveTab(first.id);
    expect(useNavStore.getState()).toMatchObject({ activeTab: first.id, recentPages: [first.id, second.id] });
  });

  it('survives internal renames by stable page id and drops deleted tabs', async () => {
    await markActiveDataKind('browser-local');
    const keep = await createPage({ title: 'Keep' });
    const remove = await createPage({ title: 'Remove' });
    useNavStore.getState().hydrate({ openTabs: [keep.id, remove.id], activeTab: keep.id, recentPages: [remove.id, keep.id] });
    await saveNavigationSession();
    await renamePage(keep.id, 'Kept and renamed');
    await deletePage(remove.id);
    useNavStore.getState().reset();
    await restoreNavigationSession();
    expect(useNavStore.getState()).toMatchObject({ openTabs: [keep.id], activeTab: keep.id, recentPages: [keep.id] });
  });

  it('isolates sessions by vault identity', async () => {
    const first = await createPage({ title: 'First' });
    const second = await createPage({ title: 'Second' });
    await markActiveDataKind('filesystem:/vault/a');
    useNavStore.getState().hydrate({ openTabs: [first.id, second.id], activeTab: first.id, recentPages: [first.id] });
    await saveNavigationSession();
    await markActiveDataKind('filesystem:/vault/b');
    useNavStore.getState().reset();
    expect(await restoreNavigationSession()).toBe(false);
    expect(useNavStore.getState().openTabs).toHaveLength(1);
    expect(useNavStore.getState().openTabs).not.toEqual([first.id, second.id]);
  });
});
