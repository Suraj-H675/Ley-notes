export type EdgeType =
  | 'wiki-link'
  | 'explicit'
  | 'task-dependency'
  | 'project-member'
  | 'depends-on'
  | 'part-of'
  | 'related-to'
  | 'contradicts'
  | 'extends'
  | 'uses'
  | 'created-by';

export interface KnowledgeEdge {
  id: string;
  source: string;
  target: string;
  type: EdgeType;
  label?: string;
  strength?: number;
  createdAt: number;
}

export interface CreateEdgeInput {
  source: string;
  target: string;
  type: EdgeType;
  label?: string;
  strength?: number;
}

export const EDGE_TYPE_LABELS: Record<EdgeType, string> = {
  'wiki-link': 'Links to',
  explicit: 'Explicit link',
  'task-dependency': 'Depends on',
  'project-member': 'Member of',
  'depends-on': 'Depends on',
  'part-of': 'Part of',
  'related-to': 'Related to',
  contradicts: 'Contradicts',
  extends: 'Extends',
  uses: 'Uses',
  'created-by': 'Created by',
};

export const EDGE_TYPE_COLORS: Record<EdgeType, string> = {
  'wiki-link': '#6366f1',
  explicit: '#8b5cf6',
  'task-dependency': '#ef4444',
  'project-member': '#f59e0b',
  'depends-on': '#ef4444',
  'part-of': '#6b7280',
  'related-to': '#6b7280',
  contradicts: '#dc2626',
  extends: '#3b82f6',
  uses: '#22c55e',
  'created-by': '#8b5cf6',
};
