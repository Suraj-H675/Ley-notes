export type ColorScheme =
  | 'untyped'
  | 'tag'
  | 'collection'
  | 'link-count'
  | 'community';

export type GraphScope = 'global' | 'local';

export interface PhysicsConfig {
  centerForce: number;
  chargeForce: number;
  linkForce: number;
  linkDistance: number;
}

export interface DisplayConfig {
  nodeSize: number;
  edgeThickness: number;
  textFade: number;
  showLabels: boolean;
}

export interface FilterConfig {
  searchQuery: string;
  selectedTags: string[];
  selectedCollections: string[];
  showOrphans: boolean;
}

export interface PanelSectionsOpen {
  groups: boolean;
  filters: boolean;
  display: boolean;
  physics: boolean;
}

export interface GraphSettings {
  scope: GraphScope;
  colorScheme: ColorScheme;
  physics: PhysicsConfig;
  display: DisplayConfig;
  filters: FilterConfig;
  panelSectionsOpen: PanelSectionsOpen;
  panelVisible: boolean;
  localDepth: 1 | 2;
  updatedAt: number;
}

export const DEFAULT_PHYSICS: PhysicsConfig = {
  centerForce: 1,
  chargeForce: -60,
  linkForce: 1,
  linkDistance: 80,
};

export const DEFAULT_DISPLAY: DisplayConfig = {
  nodeSize: 1,
  edgeThickness: 1,
  textFade: 0.25,
  showLabels: true,
};

export const DEFAULT_FILTERS: FilterConfig = {
  searchQuery: '',
  selectedTags: [],
  selectedCollections: [],
  showOrphans: true,
};

export const DEFAULT_PANEL_SECTIONS_OPEN: PanelSectionsOpen = {
  groups: true,
  filters: false,
  display: false,
  physics: false,
};
