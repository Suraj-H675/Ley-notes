import { useCallback } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import type { KnowledgeNode, CreateNodeInput, UpdateNodeInput } from '@/types';
import { db, createNode, updateNode, deleteNode, archiveNode, unarchiveNode } from '@/lib/db';

export function useNodes(includeArchived = false) {
  const nodes = useLiveQuery(
    () => includeArchived
      ? db.nodes.toArray()
      : db.nodes.where('isArchived').equals(0).toArray(),
    [includeArchived],
    []
  );

  const handleCreateNode = useCallback(async (input: CreateNodeInput): Promise<KnowledgeNode> => {
    return createNode(input);
  }, []);

  const handleUpdateNode = useCallback(async (id: string, input: UpdateNodeInput): Promise<KnowledgeNode | undefined> => {
    return updateNode(id, input);
  }, []);

  const handleDeleteNode = useCallback(async (id: string): Promise<boolean> => {
    return deleteNode(id);
  }, []);

  const handleArchiveNode = useCallback(async (id: string): Promise<KnowledgeNode | undefined> => {
    return archiveNode(id);
  }, []);

  const handleUnarchiveNode = useCallback(async (id: string): Promise<KnowledgeNode | undefined> => {
    return unarchiveNode(id);
  }, []);

  return {
    nodes: nodes || [],
    createNode: handleCreateNode,
    updateNode: handleUpdateNode,
    deleteNode: handleDeleteNode,
    archiveNode: handleArchiveNode,
    unarchiveNode: handleUnarchiveNode,
  };
}

export function useNode(id: string | null) {
  const node = useLiveQuery(
    () => id ? db.nodes.get(id) : undefined,
    [id],
    undefined
  );

  const handleUpdateNode = useCallback(async (input: UpdateNodeInput): Promise<KnowledgeNode | undefined> => {
    if (!id) return undefined;
    return updateNode(id, input);
  }, [id]);

  return {
    node,
    updateNode: handleUpdateNode,
  };
}
