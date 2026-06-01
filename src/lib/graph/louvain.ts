import Graph from 'graphology';
import louvain from 'graphology-communities-louvain';

export interface CommunityResult {
  communities: Map<string, number>;
  modularity: number;
}

export function detectCommunities(graph: Graph): CommunityResult {
  let rawCommunities: Record<string, number> = {};
  try {
    rawCommunities = louvain(graph);
  } catch (err) {
    // eslint-disable-next-line no-console
    console.warn('Louvain community detection failed:', err);
    // Fall back: every node in its own community
    graph.forEachNode((node) => {
      rawCommunities[node] = 0;
    });
  }

  const communities = new Map<string, number>(Object.entries(rawCommunities));

  const modularity = graphologyModularity(graph, communities);

  return { communities, modularity };
}

function graphologyModularity(
  graph: Graph,
  communities: Map<string, number>
): number {
  const nodes = graph.nodes();
  const edges = graph.edges();

  if (nodes.length === 0 || edges.length === 0) return 0;

  const communitySizes = new Map<number, number>();
  communities.forEach((c) => {
    communitySizes.set(c, (communitySizes.get(c) || 0) + 1);
  });

  let totalWeight = 0;
  graph.forEachEdge((_edge, attrs) => {
    totalWeight += (attrs.weight as number) || 1;
  });

  if (totalWeight === 0) totalWeight = 1;

  let sum = 0;
  graph.forEachEdge((_edge, attrs, source, target) => {
    const w = (attrs.weight as number) || 1;
    const c1 = communities.get(source);
    const c2 = communities.get(target);

    if (c1 === c2) {
      sum += w;
    }
  });

  return sum / totalWeight;
}

export function getCommunityColor(communityId: number): string {
  const colors = [
    '#3b82f6',
    '#22c55e',
    '#a855f7',
    '#f59e0b',
    '#ef4444',
    '#06b6d4',
    '#ec4899',
    '#8b5cf6',
    '#14b8a6',
    '#f97316',
  ];

  return colors[communityId % colors.length];
}

export function getNodeCommunity(
  nodeId: string,
  communities: Map<string, number>
): number | undefined {
  return communities.get(nodeId);
}

export function getNodesByCommunity(
  communities: Map<string, number>
): Map<number, string[]> {
  const result = new Map<number, string[]>();

  communities.forEach((community, nodeId) => {
    if (!result.has(community)) {
      result.set(community, []);
    }
    result.get(community)!.push(nodeId);
  });

  return result;
}
