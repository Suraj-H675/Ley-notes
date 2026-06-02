export type NodeType = 'document' | 'task' | 'project' | 'concept';

export type NodeTemplate =
  | 'blank'
  | 'book-note'
  | 'research-paper'
  | 'meeting-note'
  | 'person'
  | 'concept';

export type TaskStatus = 'pending' | 'in-progress' | 'completed';

export interface KnowledgeNode {
  id: string;
  type: NodeType;
  title: string;
  emoji?: string;
  content: string | null;
  plainText: string;
  collections: string[];
  tags: string[];
  properties: Record<string, string>;
  template?: NodeTemplate;
  taskStatus?: TaskStatus;
  taskDueDate?: number;
  isArchived: 0 | 1;
  createdAt: number;
  updatedAt: number;
  parentId?: string;
}

export interface CreateNodeInput {
  type: NodeType;
  title: string;
  emoji?: string;
  content?: string | null;
  collections?: string[];
  tags?: string[];
  properties?: Record<string, string>;
  template?: NodeTemplate;
  taskStatus?: TaskStatus;
  taskDueDate?: number;
  parentId?: string;
}

export interface UpdateNodeInput {
  title?: string;
  emoji?: string;
  content?: string | null;
  plainText?: string;
  collections?: string[];
  tags?: string[];
  properties?: Record<string, string>;
  taskStatus?: TaskStatus;
  taskDueDate?: number;
  isArchived?: 0 | 1;
  parentId?: string;
}

export function isNode(node: unknown): node is KnowledgeNode {
  return (
    typeof node === 'object' &&
    node !== null &&
    'id' in node &&
    'type' in node &&
    'title' in node
  );
}
