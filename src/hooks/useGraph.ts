import { useMemo } from 'react';
import Graph from 'graphology';
import { useNodes } from './useNodes';
import { useEdges } from './useEdges';
import { calculateGraphMetrics } from '@/lib/graph';
import { detectCommunities, type CommunityResult } from '@/lib/graph/louvain';
import type { KnowledgeNode, KnowledgeEdge } from '@/types';

export function useGraph() {
  const { nodes } = useNodes();
  const { edges } = useEdges();

  const graph = useMemo(() => {
    const g = new Graph();

    nodes.forEach((node) => {
      g.addNode(node.id, {
        type: node.type,
        title: node.title,
        emoji: node.emoji,
        taskStatus: node.taskStatus,
        isArchived: node.isArchived,
      });
    });

    edges.forEach((edge) => {
      if (g.hasNode(edge.source) && g.hasNode(edge.target)) {
        g.addEdge(edge.source, edge.target, {
          type: edge.type,
          label: edge.label,
          strength: edge.strength,
        });
      }
    });

    return g;
  }, [nodes, edges]);

  const metrics = useMemo(() => {
    if (graph.order === 0) {
      return null;
    }
    return calculateGraphMetrics(graph);
  }, [graph]);

  const communities = useMemo((): CommunityResult | null => {
    if (graph.order === 0) {
      return null;
    }
    return detectCommunities(graph);
  }, [graph]);

  const nodeMap = useMemo(() => {
    const map = new Map<string, KnowledgeNode>();
    nodes.forEach((node) => map.set(node.id, node));
    return map;
  }, [nodes]);

  const edgeMap = useMemo(() => {
    const map = new Map<string, KnowledgeEdge>();
    edges.forEach((edge) => map.set(edge.id, edge));
    return map;
  }, [edges]);

  return {
    graph,
    metrics,
    communities,
    nodeMap,
    edgeMap,
    nodeCount: graph.order,
    edgeCount: graph.size,
  };
}
