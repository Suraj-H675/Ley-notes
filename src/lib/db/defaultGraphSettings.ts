import {
  DEFAULT_DISPLAY,
  DEFAULT_FILTERS,
  DEFAULT_PANEL_SECTIONS_OPEN,
  DEFAULT_PHYSICS,
  type GraphScope,
  type GraphSettings,
} from '@/types/graph-settings.types';

export function defaultGraphSettings(scope: GraphScope): GraphSettings {
  return {
    scope,
    colorScheme: 'untyped',
    physics: { ...DEFAULT_PHYSICS },
    display: { ...DEFAULT_DISPLAY },
    filters: {
      ...DEFAULT_FILTERS,
      selectedTags: [],
      selectedCollections: [],
    },
    panelSectionsOpen: { ...DEFAULT_PANEL_SECTIONS_OPEN },
    panelVisible: true,
    localDepth: 1,
    updatedAt: Date.now(),
  };
}
