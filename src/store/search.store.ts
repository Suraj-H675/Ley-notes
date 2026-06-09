import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

interface SearchState {
  isOpen: boolean;
  query: string;
  recentSearches: string[];
  selectedResultIndex: number;
  quickSwitcherOpen: boolean;

  openSearch: () => void;
  closeSearch: () => void;
  toggleSearch: () => void;
  setQuery: (query: string) => void;
  addRecentSearch: (query: string) => void;
  clearRecentSearches: () => void;
  setSelectedResultIndex: (index: number) => void;
  moveSelectionUp: () => void;
  moveSelectionDown: (maxIndex: number) => void;
  resetSelection: () => void;
  openQuickSwitcher: () => void;
  closeQuickSwitcher: () => void;
}

export const useSearchStore = create<SearchState>()(
  immer((set) => ({
    isOpen: false,
    query: '',
    recentSearches: [],
    selectedResultIndex: -1,
    quickSwitcherOpen: false,

    openSearch: () =>
      set((state) => {
        state.isOpen = true;
        state.selectedResultIndex = -1;
      }),

    closeSearch: () =>
      set((state) => {
        state.isOpen = false;
        state.query = '';
        state.selectedResultIndex = -1;
      }),

    toggleSearch: () =>
      set((state) => {
        state.isOpen = !state.isOpen;
        if (!state.isOpen) {
          state.query = '';
          state.selectedResultIndex = -1;
        } else {
          state.selectedResultIndex = -1;
        }
      }),

    setQuery: (query) =>
      set((state) => {
        state.query = query;
        state.selectedResultIndex = -1;
      }),

    addRecentSearch: (query) =>
      set((state) => {
        if (!query.trim()) return;
        const filtered = state.recentSearches.filter((s) => s !== query);
        state.recentSearches = [query, ...filtered].slice(0, 10);
      }),

    clearRecentSearches: () =>
      set((state) => {
        state.recentSearches = [];
      }),

    setSelectedResultIndex: (index) =>
      set((state) => {
        state.selectedResultIndex = index;
      }),

    moveSelectionUp: () =>
      set((state) => {
        if (state.selectedResultIndex > 0) {
          state.selectedResultIndex -= 1;
        }
      }),

    moveSelectionDown: (maxIndex) =>
      set((state) => {
        if (state.selectedResultIndex < maxIndex - 1) {
          state.selectedResultIndex += 1;
        }
      }),

    resetSelection: () =>
      set((state) => {
        state.selectedResultIndex = -1;
      }),

    openQuickSwitcher: () =>
      set((state) => {
        state.quickSwitcherOpen = true;
        state.selectedResultIndex = -1;
      }),

    closeQuickSwitcher: () =>
      set((state) => {
        state.quickSwitcherOpen = false;
        state.selectedResultIndex = -1;
      }),
  }))
);
