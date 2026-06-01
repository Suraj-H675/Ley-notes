import { nanoid } from 'nanoid';
import type { CreateRevisionInput, Revision } from '@/types';
import { db } from './index';

export async function createRevision(
  input: CreateRevisionInput
): Promise<Revision> {
  const revision: Revision = {
    id: nanoid(),
    nodeId: input.nodeId,
    content: input.content,
    plainText: input.plainText,
    createdAt: Date.now(),
  };

  await db.revisions.add(revision);
  return revision;
}

export async function getRevision(
  id: string
): Promise<Revision | undefined> {
  return db.revisions.get(id);
}

export async function getRevisionsByNode(
  nodeId: string,
  limit?: number
): Promise<Revision[]> {
  let query = db.revisions
    .where('nodeId')
    .equals(nodeId)
    .reverse()
    .sortBy('createdAt');

  if (limit) {
    return query.then((revs) => revs.slice(0, limit));
  }
  return query;
}

export async function deleteRevision(id: string): Promise<boolean> {
  const revision = await db.revisions.get(id);
  if (!revision) return false;

  await db.revisions.delete(id);
  return true;
}

export async function deleteRevisionsByNode(nodeId: string): Promise<number> {
  const revisions = await db.revisions.where('nodeId').equals(nodeId).toArray();
  const ids = revisions.map((r) => r.id);
  await db.revisions.bulkDelete(ids);
  return ids.length;
}

export async function pruneRevisions(
  nodeId: string,
  keepCount = 50
): Promise<number> {
  const revisions = await getRevisionsByNode(nodeId);
  if (revisions.length <= keepCount) return 0;

  const toDelete = revisions.slice(keepCount);
  const ids = toDelete.map((r) => r.id);
  await db.revisions.bulkDelete(ids);
  return ids.length;
}
