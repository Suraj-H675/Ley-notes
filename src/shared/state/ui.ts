/**
 * UI state — ephemeral, not persisted across sessions.
 * Per CLAUDE.md: "Use local component state when appropriate. Do not put
 * everything into Zustand." So this store only holds things that span
 * multiple components: sidebar visibility, active dock, theme.
 */

import { create } from 'zustand';

export type Theme = 'light' | 'dark';

interface UIState {
  theme: Theme;
  setTheme: (t: Theme) => void;

  sidebarOpen: boolean;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;

  rightDockOpen: boolean;
  toggleRightDock: () => void;

  /** Context panel currently shown in the right dock. */
  rightDockTab: 'graph' | 'backlinks' | 'outline' | 'history';
  setRightDockTab: (tab: 'graph' | 'backlinks' | 'outline' | 'history') => void;
}

export const useUIStore = create<UIState>((set) => ({
  theme: 'dark',
  setTheme: (theme) => {
    set({ theme });
    document.documentElement.setAttribute('data-theme', theme);
  },
  sidebarOpen: typeof window === 'undefined' || window.matchMedia('(min-width: 768px)').matches,
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  rightDockOpen: typeof window === 'undefined' || window.matchMedia('(min-width: 1024px)').matches,
  toggleRightDock: () => set((s) => ({ rightDockOpen: !s.rightDockOpen })),
  rightDockTab: 'backlinks',
  setRightDockTab: (rightDockTab) => set({ rightDockTab }),
}));
