import type { KnowledgeNode } from './node.types';

export type SearchOperator =
  | 'type'
  | 'tag'
  | 'collection'
  | 'related'
  | 'depends'
  | 'uses'
  | 'created'
  | 'modified';

export interface SearchResult {
  id: string;
  node: KnowledgeNode;
  score: number;
  matchedField: 'title' | 'plainText' | 'tags' | 'properties';
  highlights: string[];
}

export interface SearchOptions {
  limit?: number;
  offset?: number;
  type?: string;
  tags?: string[];
  collections?: string[];
  includeArchived?: boolean;
}

export interface CommandAction {
  id: string;
  label: string;
  icon?: string;
  category: 'navigation' | 'create' | 'action';
  keywords: string[];
  execute: () => void | Promise<void>;
}

export interface ParsedQuery {
  raw: string;
  operator?: SearchOperator;
  operatorValue?: string;
  text: string;
}
