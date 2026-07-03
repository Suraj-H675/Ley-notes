import Dexie, { type Table } from 'dexie';
import { tiptapJsonToMarkdown, extractPlainText } from '@/lib/markdown';
import type {
  RefLinkType,
  BlockType,
} from '@/types';

type LegacyNodeRecord = {
  id: string;
  content: unknown;
  plainText: string;
  [key: string]: unknown;
};

/**
 * Convert a v2-style node (content is TipTap JSONContent) to v3-style
 * (content is Markdown string). Exported for unit testing.
 */
export function migrateV2NodeToV3(node: LegacyNodeRecord): LegacyNodeRecord {
  const out = { ...node };
  if (out.content && typeof out.content === 'object') {
    const md = tiptapJsonToMarkdown(out.content as any);
    out.content = md || '';
    out.plainText = extractPlainText(out.content as string);
  } else if (typeof out.content === 'string') {
    out.plainText = extractPlainText(out.content);
  } else if (out.content == null) {
    out.content = '';
  }
  return out;
}

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
export type {
  KnowledgeBlock,
  BlockType,
  RefRecord,
  RefLinkType,
  BlockAttribute,
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
  content: string | null;
  plainText: string;
  collections: string[];
  tags: string[];
  properties: Record<string, string>;
  template?: 'blank' | 'book-note' | 'research-paper' | 'meeting-note' | 'person' | 'concept';
  taskStatus?: 'pending' | 'in-progress' | 'completed';
  taskDueDate?: number;
  isArchived: 0 | 1;
  isPinned?: 0 | 1;
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
  content: string;
  plainText: string;
  createdAt: number;
}

/**
 * Dexie record shape for the `blocks` table. Mirrors `KnowledgeBlock` but
 * uses literal unions where Dexie/IndexedDB needs them.
 */
interface KnowledgeBlockRecord {
  id: string;
  nodeId: string;
  order: number;
  type: BlockType;
  markdown: string;
  textContent: string;
  createdAt: number;
  updatedAt: number;
}

/**
 * Dexie record shape for the `refs` (denormalized backlink index) table.
 * See `KnowledgeBlock`/`RefRecord` in `@/types/block.types` for semantics.
 */
interface RefRecordRecord {
  id: string;
  sourceBlockId: string;
  targetNodeId: string | null;
  targetNodeTitle: string | null;
  targetBlockId: string | null;
  linkType: RefLinkType;
  context: string;
  createdAt: number;
}

/** Dexie record shape for the `blockAttributes` table. */
interface BlockAttributeRecord {
  id: string;
  blockId: string;
  name: string;
  value: string;
  createdAt: number;
}

interface GraphSettingsRecord {
  scope: 'global' | 'local';
  colorScheme: 'untyped' | 'tag' | 'collection' | 'link-count' | 'community';
  physics: {
    centerForce: number;
    chargeForce: number;
    linkForce: number;
    linkDistance: number;
  };
  display: {
    nodeSize: number;
    edgeThickness: number;
    textFade: number;
    showLabels: boolean;
  };
  filters: {
    searchQuery: string;
    selectedTags: string[];
    selectedCollections: string[];
    showOrphans: boolean;
  };
  panelSectionsOpen: {
    groups: boolean;
    filters: boolean;
    display: boolean;
    physics: boolean;
  };
  panelVisible: boolean;
  localDepth: 1 | 2;
  updatedAt: number;
}

class KnowledgeUniverseDB extends Dexie {
  nodes!: Table<KnowledgeNodeRecord>;
  edges!: Table<KnowledgeEdgeRecord>;
  collections!: Table<CollectionRecord>;
  revisions!: Table<RevisionRecord>;
  graphPositions!: Table<GraphPosition>;
  graphSettings!: Table<GraphSettingsRecord>;
  blocks!: Table<KnowledgeBlockRecord>;
  refs!: Table<RefRecordRecord>;
  blockAttributes!: Table<BlockAttributeRecord>;

  constructor() {
    super('knowledge-universe');

    this.version(1).stores({
      nodes: 'id, type, title, *collections, *tags, isArchived, createdAt, updatedAt, parentId',
      edges: 'id, source, target, type, createdAt',
      collections: 'id, name, parentId, createdAt',
      revisions: 'id, nodeId, createdAt',
      graphPositions: 'nodeId, updatedAt',
    });

    this.version(2).stores({
      nodes: 'id, type, title, *collections, *tags, isArchived, createdAt, updatedAt, parentId',
      edges: 'id, source, target, type, createdAt',
      collections: 'id, name, parentId, createdAt',
      revisions: 'id, nodeId, createdAt',
      graphPositions: 'nodeId, updatedAt',
      graphSettings: 'scope, updatedAt',
    });

    // v3: content field changed from TipTap JSONContent to Markdown string.
    this.version(3)
      .stores({
        nodes: 'id, type, title, *collections, *tags, isArchived, createdAt, updatedAt, parentId',
        edges: 'id, source, target, type, createdAt',
        collections: 'id, name, parentId, createdAt',
        revisions: 'id, nodeId, createdAt',
        graphPositions: 'nodeId, updatedAt',
        graphSettings: 'scope, updatedAt',
      })
      .upgrade(async (tx) => {
        await tx
          .table('nodes')
          .toCollection()
          .modify((node: any) => {
            const migrated = migrateV2NodeToV3(node);
            Object.assign(node, migrated);
          });
        await tx
          .table('revisions')
          .toCollection()
          .modify((rev: any) => {
            const migrated = migrateV2NodeToV3(rev);
            Object.assign(rev, migrated);
          });
      });

    // v4: add isPinned field with index
    this.version(4)
      .stores({
        nodes: 'id, type, title, *collections, *tags, isArchived, isPinned, createdAt, updatedAt, parentId',
        edges: 'id, source, target, type, createdAt',
        collections: 'id, name, parentId, createdAt',
        revisions: 'id, nodeId, createdAt',
        graphPositions: 'nodeId, updatedAt',
        graphSettings: 'scope, updatedAt',
      })
      .upgrade(async (tx) => {
        await tx.table('nodes').toCollection().modify((node: any) => {
          if (node.isPinned === undefined) node.isPinned = 0;
        });
      });

    // v5: add block-level tables for the v2 refactor.
    //
    // Tables:
    //   blocks          — one row per top-level block (id, nodeId, order, type,
    //                     markdown, textContent, createdAt, updatedAt)
    //   refs            — denormalized backlink index (parsed from `[[Title]]`
    //                     and `((block-id))` references in block markdown)
    //   blockAttributes — per-block typed key-value (parsed from `key:: value`
    //                     inline syntax in Phase 9)
    //
    // No data migration in v5 — blocks are populated lazily by Phase 4's
    // `lazyMigrateNode`. Until that runs, the tables are empty.
    this.version(5).stores({
      // Existing tables — unchanged.
      nodes: 'id, type, title, *collections, *tags, isArchived, isPinned, createdAt, updatedAt, parentId',
      edges: 'id, source, target, type, createdAt',
      collections: 'id, name, parentId, createdAt',
      revisions: 'id, nodeId, createdAt',
      graphPositions: 'nodeId, updatedAt',
      graphSettings: 'scope, updatedAt',

      // New tables — see KnowledgeBlockRecord / RefRecordRecord / BlockAttributeRecord.
      blocks: 'id, nodeId, [nodeId+order], type, updatedAt',
      refs:
        'id, sourceBlockId, targetNodeId, targetBlockId, [sourceBlockId+targetNodeId], [targetNodeId+sourceBlockId]',
      blockAttributes: 'id, blockId, name, value, [blockId+name], [name+value]',
    });
  }
}

export const db = new KnowledgeUniverseDB();

export * from './nodes';
export * from './edges';
export * from './collections';
export * from './revisions';
export * from './templates';
