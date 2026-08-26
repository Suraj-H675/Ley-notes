/**
 * Navigation state — open tabs, focused pane, split view, and recent pages.
 */

import { create } from 'zustand';

export type EditorPane = 'primary' | 'secondary';

export interface NavState {
  openTabs: string[];
  activeTab: string | null;
  primaryTab: string | null;
  secondaryTab: string | null;
  activePane: EditorPane;
  recentPages: string[];

  openPage: (pageId: string, pane?: EditorPane) => void;
  openInSplit: (pageId: string) => void;
  focusPane: (pane: EditorPane) => void;
  closeSplit: () => void;
  closeTab: (pageId: string) => void;
  setActiveTab: (pageId: string) => void;
  pushRecent: (pageId: string) => void;
  reconcile: (pageIds: Set<string>) => void;
  hydrate: (state: Pick<NavState, 'openTabs' | 'activeTab' | 'recentPages'> & Partial<Pick<NavState, 'primaryTab' | 'secondaryTab' | 'activePane'>>) => void;
  reset: () => void;
}

const MAX_RECENT = 20;

export const useNavStore = create<NavState>((set) => ({
  openTabs: [],
  activeTab: null,
  primaryTab: null,
  secondaryTab: null,
  activePane: 'primary',
  recentPages: [],

  openPage: (pageId, requestedPane) => set((state) => {
    const pane = requestedPane ?? state.activePane;
    if (pane === 'secondary' && pageId === state.primaryTab) return { activePane: 'primary', activeTab: pageId };
    if (pane === 'primary' && pageId === state.secondaryTab) return { activePane: 'secondary', activeTab: pageId };
    return {
      openTabs: state.openTabs.includes(pageId) ? state.openTabs : [...state.openTabs, pageId],
      activeTab: pageId,
      activePane: pane,
      primaryTab: pane === 'primary' ? pageId : state.primaryTab,
      secondaryTab: pane === 'secondary' ? pageId : state.secondaryTab,
    };
  }),

  openInSplit: (pageId) => set((state) => pageId === state.primaryTab ? state : ({
    openTabs: state.openTabs.includes(pageId) ? state.openTabs : [...state.openTabs, pageId],
    activeTab: pageId,
    activePane: 'secondary',
    primaryTab: state.primaryTab ?? state.activeTab ?? pageId,
    secondaryTab: pageId,
    recentPages: [pageId, ...state.recentPages.filter((id) => id !== pageId)].slice(0, MAX_RECENT),
  })),

  focusPane: (pane) => set((state) => {
    const activeTab = pane === 'primary' ? state.primaryTab : state.secondaryTab;
    if (!activeTab) return state;
    return { activePane: pane, activeTab };
  }),

  closeSplit: () => set((state) => ({
    secondaryTab: null,
    activePane: 'primary',
    activeTab: state.primaryTab,
  })),

  closeTab: (pageId) =>
    set((s) => {
      const next = s.openTabs.filter((id) => id !== pageId);
      let primaryTab = s.primaryTab;
      let secondaryTab = s.secondaryTab;
      let activePane = s.activePane;
      if (primaryTab === pageId && secondaryTab) {
        primaryTab = secondaryTab;
        secondaryTab = null;
        activePane = 'primary';
      } else if (primaryTab === pageId) {
        primaryTab = next.at(-1) ?? null;
      }
      if (secondaryTab === pageId) {
        secondaryTab = null;
        activePane = 'primary';
      }
      const activeTab = activePane === 'secondary' ? secondaryTab : primaryTab;
      return { openTabs: next, activeTab, primaryTab, secondaryTab, activePane };
    }),

  setActiveTab: (pageId) => set((state) => {
    const recentPages = [pageId, ...state.recentPages.filter((id) => id !== pageId)].slice(0, MAX_RECENT);
    if (state.activePane === 'primary' && pageId === state.secondaryTab) return { activePane: 'secondary', activeTab: pageId, recentPages };
    if (state.activePane === 'secondary' && pageId === state.primaryTab) return { activePane: 'primary', activeTab: pageId, recentPages };
    return {
      activeTab: pageId,
      primaryTab: state.activePane === 'primary' ? pageId : state.primaryTab,
      secondaryTab: state.activePane === 'secondary' ? pageId : state.secondaryTab,
      recentPages,
    };
  }),

  pushRecent: (pageId) =>
    set((s) => {
      const filtered = s.recentPages.filter((id) => id !== pageId);
      return { recentPages: [pageId, ...filtered].slice(0, MAX_RECENT) };
    }),
  reconcile: (pageIds) => set((state) => {
    const openTabs = state.openTabs.filter((id) => pageIds.has(id));
    let primaryTab = state.primaryTab && pageIds.has(state.primaryTab) ? state.primaryTab : null;
    let secondaryTab = state.secondaryTab && pageIds.has(state.secondaryTab) ? state.secondaryTab : null;
    let activePane = state.activePane;
    if (!primaryTab && secondaryTab) {
      primaryTab = secondaryTab;
      secondaryTab = null;
      activePane = 'primary';
    }
    primaryTab ??= openTabs.at(-1) ?? null;
    if (!secondaryTab) activePane = 'primary';
    return {
      openTabs,
      activeTab: activePane === 'secondary' ? secondaryTab : primaryTab,
      primaryTab,
      secondaryTab,
      activePane,
      recentPages: state.recentPages.filter((id) => pageIds.has(id)),
    };
  }),
  hydrate: ({ openTabs, activeTab, primaryTab, secondaryTab, activePane, recentPages }) => set({
    openTabs,
    primaryTab: primaryTab ?? activeTab,
    secondaryTab: secondaryTab ?? null,
    activePane: secondaryTab ? (activePane ?? 'primary') : 'primary',
    activeTab: secondaryTab && activePane === 'secondary' ? secondaryTab : (primaryTab ?? activeTab),
    recentPages,
  }),
  reset: () => set({ openTabs: [], activeTab: null, primaryTab: null, secondaryTab: null, activePane: 'primary', recentPages: [] }),
}));
