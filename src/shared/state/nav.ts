/**
 * Navigation state — open tabs, active tab, recent pages.
 * Tabs are page IDs in MRU order; the first is the active one.
 */

import { create } from 'zustand';

interface NavState {
  openTabs: string[];
  activeTab: string | null;
  recentPages: string[];

  openPage: (pageId: string) => void;
  closeTab: (pageId: string) => void;
  setActiveTab: (pageId: string) => void;
  pushRecent: (pageId: string) => void;
  reconcile: (pageIds: Set<string>) => void;
  reset: () => void;
}

const MAX_RECENT = 20;

export const useNavStore = create<NavState>((set) => ({
  openTabs: [],
  activeTab: null,
  recentPages: [],

  openPage: (pageId) =>
    set((s) => ({
      openTabs: s.openTabs.includes(pageId) ? s.openTabs : [...s.openTabs, pageId],
      activeTab: pageId,
    })),

  closeTab: (pageId) =>
    set((s) => {
      const next = s.openTabs.filter((id) => id !== pageId);
      const active = s.activeTab === pageId ? (next.at(-1) ?? null) : s.activeTab;
      return { openTabs: next, activeTab: active };
    }),

  setActiveTab: (pageId) => set({ activeTab: pageId }),

  pushRecent: (pageId) =>
    set((s) => {
      const filtered = s.recentPages.filter((id) => id !== pageId);
      return { recentPages: [pageId, ...filtered].slice(0, MAX_RECENT) };
    }),
  reconcile: (pageIds) => set((state) => {
    const openTabs = state.openTabs.filter((id) => pageIds.has(id));
    return {
      openTabs,
      activeTab: state.activeTab && pageIds.has(state.activeTab) ? state.activeTab : (openTabs.at(-1) ?? null),
      recentPages: state.recentPages.filter((id) => pageIds.has(id)),
    };
  }),
  reset: () => set({ openTabs: [], activeTab: null, recentPages: [] }),
}));
