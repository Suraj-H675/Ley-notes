import type { KnowledgeNode, KnowledgeEdge } from '@/types';
import type { FilterConfig } from '@/types/graph-settings.types';

export interface FilteredGraph {
  nodes: KnowledgeNode[];
  edges: KnowledgeEdge[];
}

export function applyFilters(input: {
  nodes: KnowledgeNode[];
  edges: KnowledgeEdge[];
  filters: FilterConfig;
}): FilteredGraph {
  const { nodes, edges, filters } = input;

  const searchLower = filters.searchQuery.trim().toLowerCase();
  const matched = nodes.filter((n) => {
    if (searchLower && !n.title.toLowerCase().includes(searchLower)) return false;
    if (
      filters.selectedTags.length > 0 &&
      !n.tags.some((t) => filters.selectedTags.includes(t))
    ) {
      return false;
    }
    if (
      filters.selectedCollections.length > 0 &&
      !n.collections.some((c) => filters.selectedCollections.includes(c))
    ) {
      return false;
    }
    return true;
  });

  let visibleNodeIds = new Set(matched.map((n) => n.id));

  if (!filters.showOrphans) {
    const connected = new Set<string>();
    for (const e of edges) {
      if (visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target)) {
        connected.add(e.source);
        connected.add(e.target);
      }
    }
    visibleNodeIds = connected;
  }

  const visibleNodes = matched.filter((n) => visibleNodeIds.has(n.id));
  const visibleEdges = edges.filter(
    (e) => visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target)
  );

  return { nodes: visibleNodes, edges: visibleEdges };
}

export function useFilteredGraph(
  nodes: KnowledgeNode[],
  edges: KnowledgeEdge[],
  filters: FilterConfig
): FilteredGraph {
  return applyFilters({ nodes, edges, filters });
}
