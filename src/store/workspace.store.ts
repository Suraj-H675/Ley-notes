import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

interface WorkspaceState {
  expandedCollections: string[];
  sidebarWidth: number;
  sidebarCollapsed: boolean;
  theme: 'light' | 'dark' | 'system';
  lastOpenedNodeId: string | null;
  recentNodeIds: string[];

  toggleCollection: (id: string) => void;
  setSidebarWidth: (width: number) => void;
  toggleSidebar: () => void;
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
  setLastOpenedNode: (nodeId: string) => void;
  addToRecentNodes: (nodeId: string) => void;
  removeFromRecentNodes: (nodeId: string) => void;
}

export const useWorkspaceStore = create<WorkspaceState>()(
  persist(
    immer((set) => ({
      expandedCollections: [],
      sidebarWidth: 280,
      sidebarCollapsed: false,
      theme: 'dark',
      lastOpenedNodeId: null,
      recentNodeIds: [],

      toggleCollection: (id) =>
        set((state) => {
          if (state.expandedCollections.includes(id)) {
            state.expandedCollections = state.expandedCollections.filter((c) => c !== id);
          } else {
            state.expandedCollections.push(id);
          }
        }),

      setSidebarWidth: (width) =>
        set((state) => {
          state.sidebarWidth = Math.max(200, Math.min(500, width));
        }),

      toggleSidebar: () =>
        set((state) => {
          state.sidebarCollapsed = !state.sidebarCollapsed;
        }),

      setTheme: (theme) =>
        set((state) => {
          state.theme = theme;
        }),

      setLastOpenedNode: (nodeId) =>
        set((state) => {
          state.lastOpenedNodeId = nodeId;
        }),

      addToRecentNodes: (nodeId) =>
        set((state) => {
          const filtered = state.recentNodeIds.filter((id) => id !== nodeId);
          state.recentNodeIds = [nodeId, ...filtered].slice(0, 10);
        }),

      removeFromRecentNodes: (nodeId) =>
        set((state) => {
          state.recentNodeIds = state.recentNodeIds.filter((id) => id !== nodeId);
        }),
    })),
    {
      name: 'knowledge-universe-workspace',
      partialize: (state) => ({
        expandedCollections: state.expandedCollections,
        sidebarWidth: state.sidebarWidth,
        sidebarCollapsed: state.sidebarCollapsed,
        theme: state.theme,
        lastOpenedNodeId: state.lastOpenedNodeId,
        recentNodeIds: state.recentNodeIds,
      }),
    }
  )
);
