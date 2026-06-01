import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

interface EditorState {
  activeNodeId: string | null;
  isEditing: boolean;
  isSaving: boolean;
  lastSavedAt: number | null;
  isDirty: boolean;
  wordCount: number;
  characterCount: number;

  setActiveNode: (nodeId: string | null) => void;
  setIsEditing: (isEditing: boolean) => void;
  setIsSaving: (isSaving: boolean) => void;
  setLastSavedAt: (timestamp: number) => void;
  setIsDirty: (isDirty: boolean) => void;
  setWordCount: (count: number) => void;
  setCharacterCount: (count: number) => void;
  resetEditorState: () => void;
}

const initialState = {
  activeNodeId: null,
  isEditing: false,
  isSaving: false,
  lastSavedAt: null,
  isDirty: false,
  wordCount: 0,
  characterCount: 0,
};

export const useEditorStore = create<EditorState>()(
  immer((set) => ({
    ...initialState,

    setActiveNode: (nodeId) =>
      set((state) => {
        state.activeNodeId = nodeId;
      }),

    setIsEditing: (isEditing) =>
      set((state) => {
        state.isEditing = isEditing;
      }),

    setIsSaving: (isSaving) =>
      set((state) => {
        state.isSaving = isSaving;
      }),

    setLastSavedAt: (timestamp) =>
      set((state) => {
        state.lastSavedAt = timestamp;
        state.isDirty = false;
      }),

    setIsDirty: (isDirty) =>
      set((state) => {
        state.isDirty = isDirty;
      }),

    setWordCount: (count) =>
      set((state) => {
        state.wordCount = count;
      }),

    setCharacterCount: (count) =>
      set((state) => {
        state.characterCount = count;
      }),

    resetEditorState: () =>
      set(() => ({ ...initialState })),
  }))
);
