import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

interface UniverseState {
  selectedNodeIds: string[];
  hoveredNodeId: string | null;
  isPanning: boolean;
  zoomLevel: number;
  showMiniMap: boolean;
  showLabels: boolean;
  layoutMode: 'force' | 'circular' | 'grid';
  filterType: string | null;
  searchQuery: string;

  setSelectedNodes: (nodeIds: string[]) => void;
  addSelectedNode: (nodeId: string) => void;
  removeSelectedNode: (nodeId: string) => void;
  clearSelection: () => void;
  setHoveredNode: (nodeId: string | null) => void;
  setIsPanning: (isPanning: boolean) => void;
  setZoomLevel: (level: number) => void;
  toggleMiniMap: () => void;
  toggleLabels: () => void;
  setLayoutMode: (mode: 'force' | 'circular' | 'grid') => void;
  setFilterType: (type: string | null) => void;
  setSearchQuery: (query: string) => void;
}

export const useUniverseStore = create<UniverseState>()(
  immer((set) => ({
    selectedNodeIds: [],
    hoveredNodeId: null,
    isPanning: false,
    zoomLevel: 1,
    showMiniMap: true,
    showLabels: true,
    layoutMode: 'force',
    filterType: null,
    searchQuery: '',

    setSelectedNodes: (nodeIds) =>
      set((state) => {
        state.selectedNodeIds = nodeIds;
      }),

    addSelectedNode: (nodeId) =>
      set((state) => {
        if (!state.selectedNodeIds.includes(nodeId)) {
          state.selectedNodeIds.push(nodeId);
        }
      }),

    removeSelectedNode: (nodeId) =>
      set((state) => {
        state.selectedNodeIds = state.selectedNodeIds.filter((id) => id !== nodeId);
      }),

    clearSelection: () =>
      set((state) => {
        state.selectedNodeIds = [];
      }),

    setHoveredNode: (nodeId) =>
      set((state) => {
        state.hoveredNodeId = nodeId;
      }),

    setIsPanning: (isPanning) =>
      set((state) => {
        state.isPanning = isPanning;
      }),

    setZoomLevel: (level) =>
      set((state) => {
        state.zoomLevel = Math.max(0.1, Math.min(2, level));
      }),

    toggleMiniMap: () =>
      set((state) => {
        state.showMiniMap = !state.showMiniMap;
      }),

    toggleLabels: () =>
      set((state) => {
        state.showLabels = !state.showLabels;
      }),

    setLayoutMode: (mode) =>
      set((state) => {
        state.layoutMode = mode;
      }),

    setFilterType: (type) =>
      set((state) => {
        state.filterType = type;
      }),

    setSearchQuery: (query) =>
      set((state) => {
        state.searchQuery = query;
      }),
  }))
);
