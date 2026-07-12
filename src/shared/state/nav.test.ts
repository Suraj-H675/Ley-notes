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
});
