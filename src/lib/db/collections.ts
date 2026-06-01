import { nanoid } from 'nanoid';
import type { Collection, CreateCollectionInput, UpdateCollectionInput } from '@/types';
import { db } from './index';

export async function createCollection(
  input: CreateCollectionInput
): Promise<Collection> {
  const now = Date.now();
  const collection: Collection = {
    id: nanoid(),
    name: input.name,
    emoji: input.emoji,
    parentId: input.parentId,
    createdAt: now,
    updatedAt: now,
  };

  await db.collections.add(collection);
  return collection;
}

export async function getCollection(
  id: string
): Promise<Collection | undefined> {
  return db.collections.get(id);
}

export async function updateCollection(
  id: string,
  input: UpdateCollectionInput
): Promise<Collection | undefined> {
  const collection = await db.collections.get(id);
  if (!collection) return undefined;

  const updated: Collection = {
    ...collection,
    ...input,
    updatedAt: Date.now(),
  };

  await db.collections.put(updated);
  return updated;
}

export async function deleteCollection(id: string): Promise<boolean> {
  const collection = await db.collections.get(id);
  if (!collection) return false;

  await db.transaction('rw', [db.collections, db.nodes], async () => {
    const childCollections = await db.collections
      .where('parentId')
      .equals(id)
      .toArray();

    for (const child of childCollections) {
      await updateCollection(child.id, { parentId: collection.parentId });
    }

    const nodesInCollection = await db.nodes
      .where('collections')
      .equals(id)
      .toArray();

    for (const node of nodesInCollection) {
      await db.nodes.update(node.id, {
        collections: node.collections.filter((c) => c !== id),
        updatedAt: Date.now(),
      });
    }

    await db.collections.delete(id);
  });

  return true;
}

export async function getAllCollections(): Promise<Collection[]> {
  return db.collections.toArray();
}

export async function getRootCollections(): Promise<Collection[]> {
  return db.collections.filter((c) => !c.parentId).toArray();
}

export async function getChildCollections(
  parentId: string
): Promise<Collection[]> {
  return db.collections.where('parentId').equals(parentId).toArray();
}

export async function getCollectionTree(): Promise<
  (Collection & { children: Collection[] })[]
> {
  const all = await getAllCollections();
  const rootCollections = all.filter((c) => !c.parentId);

  function buildTree(parentId: string): Collection[] {
    return all
      .filter((c) => c.parentId === parentId)
      .map((c) => ({ ...c, children: buildTree(c.id) }));
  }

  return rootCollections.map((c) => ({ ...c, children: buildTree(c.id) }));
}
