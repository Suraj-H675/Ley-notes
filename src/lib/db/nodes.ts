import { nanoid } from 'nanoid';
import type { CreateNodeInput, KnowledgeNode, UpdateNodeInput } from '@/types';
import { db } from './index';

export async function createNode(input: CreateNodeInput): Promise<KnowledgeNode> {
  const now = Date.now();
  const node: KnowledgeNode = {
    id: nanoid(),
    type: input.type,
    title: input.title,
    emoji: input.emoji,
    content: input.content ?? null,
    plainText: '',
    collections: input.collections ?? [],
    tags: input.tags ?? [],
    properties: input.properties ?? {},
    template: input.template,
    taskStatus: input.taskStatus,
    taskDueDate: input.taskDueDate,
    isArchived: 0,
    createdAt: now,
    updatedAt: now,
    parentId: input.parentId,
  };

  await db.nodes.add(node);
  return node;
}

export async function getNode(id: string): Promise<KnowledgeNode | undefined> {
  return db.nodes.get(id);
}

export async function updateNode(
  id: string,
  input: UpdateNodeInput
): Promise<KnowledgeNode | undefined> {
  const node = await db.nodes.get(id);
  if (!node) return undefined;

  const updated: KnowledgeNode = {
    ...node,
    ...input,
    updatedAt: Date.now(),
  };

  await db.nodes.put(updated);
  return updated;
}

export async function deleteNode(id: string): Promise<boolean> {
  const node = await db.nodes.get(id);
  if (!node) return false;

  await db.transaction('rw', [db.nodes, db.edges, db.revisions], async () => {
    await db.edges.where('source').equals(id).delete();
    await db.edges.where('target').equals(id).delete();
    await db.revisions.where('nodeId').equals(id).delete();
    await db.nodes.delete(id);
  });

  return true;
}

export async function archiveNode(id: string): Promise<KnowledgeNode | undefined> {
  return updateNode(id, { isArchived: 1 });
}

export async function unarchiveNode(id: string): Promise<KnowledgeNode | undefined> {
  return updateNode(id, { isArchived: 0 });
}

export async function getAllNodes(includeArchived = false): Promise<KnowledgeNode[]> {
  if (includeArchived) {
    return db.nodes.toArray();
  }
  return db.nodes.where('isArchived').equals(0).toArray();
}

export async function getNodesByType(
  type: KnowledgeNode['type'],
  includeArchived = false
): Promise<KnowledgeNode[]> {
  const query = includeArchived
    ? db.nodes.where('type').equals(type)
    : db.nodes.where('isArchived').equals(0).and((n) => n.type === type);
  return query.toArray();
}

export async function getNodesByCollection(
  collectionId: string,
  includeArchived = false
): Promise<KnowledgeNode[]> {
  const allNodes = includeArchived
    ? await db.nodes.toArray()
    : await db.nodes.where('isArchived').equals(0).toArray();

  return allNodes.filter((n) => n.collections.includes(collectionId));
}

export async function getNodesByTag(
  tag: string,
  includeArchived = false
): Promise<KnowledgeNode[]> {
  const allNodes = includeArchived
    ? await db.nodes.toArray()
    : await db.nodes.where('isArchived').equals(0).toArray();

  return allNodes.filter((n) => n.tags.includes(tag));
}

export async function searchNodes(
  query: string,
  limit = 20
): Promise<KnowledgeNode[]> {
  const lowerQuery = query.toLowerCase();
  const nodes = await db.nodes.where('isArchived').equals(0).toArray();

  return nodes
    .filter(
      (n) =>
        n.title.toLowerCase().includes(lowerQuery) ||
        n.plainText.toLowerCase().includes(lowerQuery) ||
        n.tags.some((t) => t.toLowerCase().includes(lowerQuery))
    )
    .slice(0, limit);
}

export async function getRecentNodes(limit = 10): Promise<KnowledgeNode[]> {
  return db.nodes
    .where('isArchived')
    .equals(0)
    .reverse()
    .sortBy('updatedAt')
    .then((nodes) => nodes.slice(0, limit));
}
