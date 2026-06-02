import { useMemo } from 'react';
import Graph from 'graphology';
import { colorForNode } from '@/lib/graph/colors';
import type { ColorScheme } from '@/types/graph-settings.types';
import type { KnowledgeNode, KnowledgeEdge } from '@/types';
import type { CommunityResult } from '@/lib/graph/louvain';

export function colorMapForGraph(
  nodes: KnowledgeNode[],
  _edges: KnowledgeEdge[],
  graph: Graph,
  scheme: ColorScheme,
  communities?: CommunityResult | null
): Map<string, string> {
  let maxDegree = 0;
  graph.forEachNode((id) => {
    const d = graph.degree(id);
    if (d > maxDegree) maxDegree = d;
  });

  const partition = communities?.communities ?? new Map<string, number>();
  const colorByCommunity = (id: string) => partition.get(id) ?? 0;

  const map = new Map<string, string>();
  for (const n of nodes) {
    const color = colorForNode(n, scheme, {
      degree: graph.hasNode(n.id) ? graph.degree(n.id) : 0,
      maxDegree,
      community: colorByCommunity(n.id),
    });
    map.set(n.id, color);
  }
  return map;
}

export function useColoredGraph(
  nodes: KnowledgeNode[],
  edges: KnowledgeEdge[],
  graph: Graph,
  scheme: ColorScheme,
  communities?: CommunityResult | null
): Map<string, string> {
  return useMemo(
    () => colorMapForGraph(nodes, edges, graph, scheme, communities),
    [nodes, edges, graph, scheme, communities]
  );
}
