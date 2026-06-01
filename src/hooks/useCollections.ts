import { useCallback } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import type { Collection, CreateCollectionInput, UpdateCollectionInput } from '@/types';
import { db, createCollection, updateCollection, deleteCollection, getAllCollections } from '@/lib/db';

export function useCollections() {
  const collections = useLiveQuery(
    () => getAllCollections(),
    [],
    []
  );

  const handleCreateCollection = useCallback(async (input: CreateCollectionInput): Promise<Collection> => {
    return createCollection(input);
  }, []);

  const handleUpdateCollection = useCallback(async (id: string, input: UpdateCollectionInput): Promise<Collection | undefined> => {
    return updateCollection(id, input);
  }, []);

  const handleDeleteCollection = useCallback(async (id: string): Promise<boolean> => {
    return deleteCollection(id);
  }, []);

  return {
    collections: collections || [],
    createCollection: handleCreateCollection,
    updateCollection: handleUpdateCollection,
    deleteCollection: handleDeleteCollection,
  };
}

export function useCollection(id: string | null) {
  const collection = useLiveQuery(
    () => id ? db.collections.get(id) : undefined,
    [id],
    undefined
  );

  const handleUpdateCollection = useCallback(async (input: UpdateCollectionInput): Promise<Collection | undefined> => {
    if (!id) return undefined;
    return updateCollection(id, input);
  }, [id]);

  return {
    collection,
    updateCollection: handleUpdateCollection,
  };
}
