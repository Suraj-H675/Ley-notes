/**
 * Tag filter — single tag selection shared by the sidebar pane and the graph.
 * Clicking a tag in the pane highlights matching nodes in the graph; clicking
 * the active tag again clears it.
 */

import { create } from 'zustand';

interface TagFilterState {
  activeTag: string | null;
  setActiveTag: (tag: string | null) => void;
}

export const useTagFilter = create<TagFilterState>((set) => ({
  activeTag: null,
  setActiveTag: (tag) => set({ activeTag: tag }),
}));