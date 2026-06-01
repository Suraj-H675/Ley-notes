import Dexie, { type Table } from 'dexie';
import type { JSONContent } from '@tiptap/react';

export type { NodeType, NodeTemplate, TaskStatus } from '@/types';
export type { EdgeType } from '@/types';
export type { Collection, CreateCollectionInput, UpdateCollectionInput } from '@/types';
export type { Revision, CreateRevisionInput } from '@/types';
export type {
  KnowledgeNode,
  CreateNodeInput,
  UpdateNodeInput,
} from '@/types';
export type {
  KnowledgeEdge,
  CreateEdgeInput,
} from '@/types';

export interface GraphPosition {
  nodeId: string;
  x: number;
  y: number;
  updatedAt: number;
}

interface KnowledgeNodeRecord {
  id: string;
  type: 'document' | 'task' | 'project' | 'concept';
  title: string;
  emoji?: string;
  content: JSONContent | null;
  plainText: string;
  collections: string[];
  tags: string[];
  properties: Record<string, string>;
  template?: 'blank' | 'book-note' | 'research-paper' | 'meeting-note' | 'person' | 'concept';
  taskStatus?: 'pending' | 'in-progress' | 'completed';
  taskDueDate?: number;
  isArchived: 0 | 1;
  createdAt: number;
  updatedAt: number;
  parentId?: string;
}

interface KnowledgeEdgeRecord {
  id: string;
  source: string;
  target: string;
  type: 'wiki-link' | 'explicit' | 'task-dependency' | 'project-member' | 'depends-on' | 'part-of' | 'related-to' | 'contradicts' | 'extends' | 'uses' | 'created-by';
  label?: string;
  strength?: number;
  createdAt: number;
}

interface CollectionRecord {
  id: string;
  name: string;
  emoji?: string;
  parentId?: string;
  createdAt: number;
  updatedAt: number;
}

interface RevisionRecord {
  id: string;
  nodeId: string;
  content: JSONContent;
  plainText: string;
  createdAt: number;
}

class KnowledgeUniverseDB extends Dexie {
  nodes!: Table<KnowledgeNodeRecord>;
  edges!: Table<KnowledgeEdgeRecord>;
  collections!: Table<CollectionRecord>;
  revisions!: Table<RevisionRecord>;
  graphPositions!: Table<GraphPosition>;

  constructor() {
    super('knowledge-universe');

    this.version(1).stores({
      nodes: 'id, type, title, *collections, *tags, isArchived, createdAt, updatedAt, parentId',
      edges: 'id, source, target, type, createdAt',
      collections: 'id, name, parentId, createdAt',
      revisions: 'id, nodeId, createdAt',
      graphPositions: 'nodeId, updatedAt',
    });
  }
}

export const db = new KnowledgeUniverseDB();

export * from './nodes';
export * from './edges';
export * from './collections';
export * from './revisions';
export * from './templates';
