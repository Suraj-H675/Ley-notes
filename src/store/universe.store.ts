import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

interface UniverseState {
  selectedNodeIds: string[];
  hoveredNodeId: string | null;
  zoomLevel: number;

  setSelectedNodes: (nodeIds: string[]) => void;
  clearSelection: () => void;
  setHoveredNode: (nodeId: string | null) => void;
  setZoomLevel: (level: number) => void;
}

export const useUniverseStore = create<UniverseState>()(
  immer((set) => ({
    selectedNodeIds: [],
    hoveredNodeId: null,
    zoomLevel: 1,

    setSelectedNodes: (nodeIds) =>
      set((state) => {
        state.selectedNodeIds = nodeIds;
      }),
    clearSelection: () =>
      set((state) => {
        state.selectedNodeIds = [];
      }),
    setHoveredNode: (nodeId) =>
      set((state) => {
        state.hoveredNodeId = nodeId;
      }),
    setZoomLevel: (level) =>
      set((state) => {
        state.zoomLevel = Math.max(0.1, Math.min(2, level));
      }),
  }))
);
