import { beforeEach, describe, expect, it } from 'vitest';
import { useNavStore } from './nav';

describe('navigation reconciliation', () => {
  beforeEach(() => useNavStore.getState().reset());

  it('removes tabs and recents for files that disappeared during a rescan', () => {
    const nav = useNavStore.getState();
    nav.openPage('a');
    nav.openPage('b');
    nav.pushRecent('a');
    nav.pushRecent('b');
    nav.reconcile(new Set(['b', 'c']));
    expect(useNavStore.getState()).toMatchObject({ openTabs: ['b'], activeTab: 'b', recentPages: ['b'] });

    useNavStore.getState().reconcile(new Set(['c']));
    expect(useNavStore.getState()).toMatchObject({ openTabs: [], activeTab: null, recentPages: [] });
  });

  it('opens, focuses, and closes an independent secondary pane', () => {
    const nav = useNavStore.getState();
    nav.openPage('source');
    nav.openInSplit('reference');
    expect(useNavStore.getState()).toMatchObject({ primaryTab: 'source', secondaryTab: 'reference', activePane: 'secondary', activeTab: 'reference' });
    useNavStore.getState().focusPane('primary');
    useNavStore.getState().openPage('draft');
    expect(useNavStore.getState()).toMatchObject({ primaryTab: 'draft', secondaryTab: 'reference', activeTab: 'draft' });
    useNavStore.getState().closeSplit();
    expect(useNavStore.getState()).toMatchObject({ primaryTab: 'draft', secondaryTab: null, activePane: 'primary', activeTab: 'draft' });
  });

  it('promotes the remaining pane when its primary tab closes', () => {
    const nav = useNavStore.getState();
    nav.openPage('source');
    nav.openInSplit('reference');
    useNavStore.getState().closeTab('source');
    expect(useNavStore.getState()).toMatchObject({ openTabs: ['reference'], primaryTab: 'reference', secondaryTab: null, activePane: 'primary', activeTab: 'reference' });
  });

  it('focuses a note already visible in the other pane instead of duplicating its editor', () => {
    const nav = useNavStore.getState();
    nav.openPage('source');
    nav.openInSplit('reference');
    useNavStore.getState().openPage('source', 'secondary');
    expect(useNavStore.getState()).toMatchObject({ primaryTab: 'source', secondaryTab: 'reference', activePane: 'primary', activeTab: 'source' });
    useNavStore.getState().setActiveTab('reference');
    expect(useNavStore.getState()).toMatchObject({ primaryTab: 'source', secondaryTab: 'reference', activePane: 'secondary', activeTab: 'reference' });
  });
});
