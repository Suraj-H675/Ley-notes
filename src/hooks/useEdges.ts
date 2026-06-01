import { useCallback } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import type { KnowledgeEdge, CreateEdgeInput } from '@/types';
import { db, createEdge, deleteEdge, getEdgesByNode } from '@/lib/db';

export function useEdges() {
  const edges = useLiveQuery(
    () => db.edges.toArray(),
    [],
    []
  );

  const handleCreateEdge = useCallback(async (input: CreateEdgeInput): Promise<KnowledgeEdge> => {
    return createEdge(input);
  }, []);

  const handleDeleteEdge = useCallback(async (id: string): Promise<boolean> => {
    return deleteEdge(id);
  }, []);

  return {
    edges: edges || [],
    createEdge: handleCreateEdge,
    deleteEdge: handleDeleteEdge,
  };
}

export function useNodeEdges(nodeId: string | null) {
  const result = useLiveQuery(
    async () => {
      if (!nodeId) return { outgoing: [], incoming: [] };
      return getEdgesByNode(nodeId);
    },
    [nodeId],
    { outgoing: [], incoming: [] }
  );

  const handleCreateEdge = useCallback(async (input: CreateEdgeInput): Promise<KnowledgeEdge> => {
    return createEdge(input);
  }, []);

  const handleDeleteEdge = useCallback(async (id: string): Promise<boolean> => {
    return deleteEdge(id);
  }, []);

  return {
    outgoingEdges: result?.outgoing || [],
    incomingEdges: result?.incoming || [],
    createEdge: handleCreateEdge,
    deleteEdge: handleDeleteEdge,
  };
}
