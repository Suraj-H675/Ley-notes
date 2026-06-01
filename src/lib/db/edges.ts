import { nanoid } from 'nanoid';
import type { CreateEdgeInput, KnowledgeEdge } from '@/types';
import { db } from './index';

export async function createEdge(input: CreateEdgeInput): Promise<KnowledgeEdge> {
  const edge: KnowledgeEdge = {
    id: nanoid(),
    source: input.source,
    target: input.target,
    type: input.type,
    label: input.label,
    strength: input.strength,
    createdAt: Date.now(),
  };

  await db.edges.add(edge);
  return edge;
}

export async function getEdge(id: string): Promise<KnowledgeEdge | undefined> {
  return db.edges.get(id);
}

export async function deleteEdge(id: string): Promise<boolean> {
  const edge = await db.edges.get(id);
  if (!edge) return false;

  await db.edges.delete(id);
  return true;
}

export async function deleteEdgesByNode(nodeId: string): Promise<number> {
  const edges = await db.edges
    .where('source')
    .equals(nodeId)
    .or('target')
    .equals(nodeId)
    .toArray();

  const ids = edges.map((e) => e.id);
  await db.edges.bulkDelete(ids);
  return ids.length;
}

export async function getAllEdges(): Promise<KnowledgeEdge[]> {
  return db.edges.toArray();
}

export async function getEdgesByNode(
  nodeId: string
): Promise<{ outgoing: KnowledgeEdge[]; incoming: KnowledgeEdge[] }> {
  const [outgoing, incoming] = await Promise.all([
    db.edges.where('source').equals(nodeId).toArray(),
    db.edges.where('target').equals(nodeId).toArray(),
  ]);

  return { outgoing, incoming };
}

export async function getOutgoingEdges(nodeId: string): Promise<KnowledgeEdge[]> {
  return db.edges.where('source').equals(nodeId).toArray();
}

export async function getIncomingEdges(nodeId: string): Promise<KnowledgeEdge[]> {
  return db.edges.where('target').equals(nodeId).toArray();
}

export async function getEdgesByType(
  type: KnowledgeEdge['type']
): Promise<KnowledgeEdge[]> {
  return db.edges.where('type').equals(type).toArray();
}

export async function edgeExists(
  source: string,
  target: string,
  type?: KnowledgeEdge['type']
): Promise<boolean> {
  const edges = await db.edges
    .where('source')
    .equals(source)
    .and((e) => e.target === target)
    .toArray();

  if (type) {
    return edges.some((e) => e.type === type);
  }
  return edges.length > 0;
}

export async function getRelatedNodes(
  nodeId: string
): Promise<{ nodeId: string; edge: KnowledgeEdge }[]> {
  const { outgoing, incoming } = await getEdgesByNode(nodeId);
  const allEdges = [...outgoing, ...incoming];

  const related: { nodeId: string; edge: KnowledgeEdge }[] = [];
  for (const edge of allEdges) {
    const relatedNodeId = edge.source === nodeId ? edge.target : edge.source;
    related.push({ nodeId: relatedNodeId, edge });
  }

  return related;
}
