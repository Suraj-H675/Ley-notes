/**
 * Build a graphology graph from the live pages and links in Dexie.
 *
 * Produces a graph with rich per-node attributes:
 *   - label (page title)
 *   - degree (number of edges, used for sizing)
 *   - community (Louvain cluster id)
 *   - folder (path prefix)
 *   - tags (frontmatter + inline)
 *
 * Edges have:
 *   - kind: wiki, embed, or portable Markdown link
 *
 * Auto-aggregation: when node count exceeds `nodeLimit`, we collapse to a
 * community-level meta-graph (per the Graphify pattern at 5000 nodes).
 */

import Graph from 'graphology';
import louvain from 'graphology-communities-louvain';
import type { Link, LinkKind, Page } from '@/infrastructure/database/schema';
import type { Tag } from '@/infrastructure/database/schema';

export interface GraphNodeAttrs {
  label: string;
  /** Number of incident edges (in + out). */
  degree: number;
  /** Louvain cluster id. Stable across rebuilds (deterministic RNG). */
  community: number;
  /** Folder path (path minus filename). Empty for root-level pages. */
  folder: string;
  /** All tags this page has. */
  tags: string[];
}

export interface GraphBuildResult {
  graph: Graph<GraphNodeAttrs, { kind: LinkKind }>;
  /** Community id per node. Same as graph.getNodeAttribute(id, 'community'). */
  communities: Map<string, number>;
  /** True if collapsed to community-level meta-graph. */
  aggregated: boolean;
  /** Number of communities in the graph. */
  communityCount: number;
}

export function buildGraph(
  pages: Page[],
  links: Link[],
  tags: Tag[],
  nodeLimit = 5000,
): GraphBuildResult {
  const live = pages.filter((p) => p.deletedAt === null);
  const tagsByPage = new Map<string, string[]>();
  for (const t of tags) {
    if (t.pageId) {
      const cur = tagsByPage.get(t.pageId) ?? [];
      cur.push(t.tag);
      tagsByPage.set(t.pageId, cur);
    }
  }

  const byId = new Map(live.map((p) => [p.id, p]));
  const g = new Graph<GraphNodeAttrs, { kind: LinkKind }>({
    type: 'directed',
    multi: true,
    allowSelfLoops: false,
  });

  for (const p of live) {
    const folder = p.path.includes('/')
      ? p.path.split('/').slice(0, -1).join('/')
      : '';
    g.addNode(p.id, {
      label: p.title,
      degree: 0,
      community: 0,
      folder,
      tags: tagsByPage.get(p.id) ?? [],
    });
  }

  for (const l of links) {
    if (l.sourcePageId === l.targetPageId) continue;
    if (!l.targetPageId) continue; // ghost link
    if (!byId.has(l.sourcePageId) || !byId.has(l.targetPageId)) continue;
    g.addDirectedEdge(l.sourcePageId, l.targetPageId, { kind: l.kind });
  }

  // Compute degree per node.
  for (const id of g.nodes()) {
    g.setNodeAttribute(id, 'degree', g.degree(id));
  }

  // Community detection.
  let communities = new Map<string, number>();
  if (g.order > 0) {
    try {
      const assignment = louvain(g, {
        getEdgeWeight: null,
        rng: mulberry32(42),
      });
      for (const [nodeId, cId] of Object.entries(assignment)) {
        const cid = Number(cId);
        communities.set(nodeId, cid);
        g.setNodeAttribute(nodeId, 'community', cid);
      }
    } catch (e) {
      console.warn('[graph] community detection failed:', e);
      for (const n of g.nodes()) {
        communities.set(n, 0);
        g.setNodeAttribute(n, 'community', 0);
      }
    }
  }
  const communityCount = new Set(communities.values()).size;

  // Auto-aggregate above node limit.
  if (g.order > nodeLimit) {
    return {
      graph: aggregateToCommunities(g),
      communities,
      aggregated: true,
      communityCount,
    };
  }
  return { graph: g, communities, aggregated: false, communityCount };
}

interface MetaNodeAttrs {
  label: string;
  degree: number;
  community: number;
  folder: string;
  tags: string[];
  size: number;
}

function aggregateToCommunities(
  g: Graph<GraphNodeAttrs, { kind: LinkKind }>,
): Graph<MetaNodeAttrs, { weight: number; kind: LinkKind }> {
  const meta = new Graph<MetaNodeAttrs, { weight: number; kind: LinkKind }>({
    type: 'directed',
    allowSelfLoops: false,
  });
  const communityNodes = [...new Set(g.nodes().map((n) => g.getNodeAttribute(n, 'community')))];
  const membersByC = new Map<number, string[]>();
  for (const n of g.nodes()) {
    const c = g.getNodeAttribute(n, 'community');
    const cur = membersByC.get(c) ?? [];
    cur.push(n);
    membersByC.set(c, cur);
  }
  for (const cId of communityNodes) {
    const members = membersByC.get(cId) ?? [];
    meta.addNode(`c${cId}`, {
      label: `Cluster ${cId + 1} (${members.length})`,
      degree: members.length,
      community: cId,
      folder: '',
      tags: [],
      size: members.length,
    });
  }
  for (const e of g.edges()) {
    const s = g.getNodeAttribute(g.source(e), 'community');
    const t = g.getNodeAttribute(g.target(e), 'community');
    if (s === t) continue;
    const key = `c${s}->c${t}`;
    const kind = g.getEdgeAttribute(e, 'kind');
    if (meta.hasEdge(key)) {
      const w = meta.getEdgeAttribute(key, 'weight');
      meta.setEdgeAttribute(key, 'weight', w + 1);
    } else {
      meta.addDirectedEdgeWithKey(key, `c${s}`, `c${t}`, { weight: 1, kind });
    }
  }
  return meta;
}

/**
 * Build an N-hop BFS subgraph centered on `centerId`. Both in- and out-edges
 * are followed so backlinks appear in the local view.
 */
export function localGraph(
  g: Graph<GraphNodeAttrs, { kind: LinkKind }>,
  centerId: string,
  hops = 2,
): Graph<GraphNodeAttrs, { kind: LinkKind }> {
  if (!g.hasNode(centerId)) {
    return new Graph<GraphNodeAttrs, { kind: LinkKind }>({
      type: 'directed',
      allowSelfLoops: false,
    });
  }
  const visited = new Set<string>([centerId]);
  let frontier = new Set<string>([centerId]);
  for (let i = 0; i < hops; i++) {
    const next = new Set<string>();
    for (const n of frontier) {
      for (const nb of g.outNeighbors(n)) {
        if (!visited.has(nb)) {
          visited.add(nb);
          next.add(nb);
        }
      }
      for (const nb of g.inNeighbors(n)) {
        if (!visited.has(nb)) {
          visited.add(nb);
          next.add(nb);
        }
      }
    }
    frontier = next;
  }
  const sub = new Graph<GraphNodeAttrs, { kind: LinkKind }>({
    type: 'directed',
    allowSelfLoops: false,
  });
  for (const n of visited) {
    if (g.hasNode(n)) sub.addNode(n, g.getNodeAttributes(n));
  }
  for (const e of g.edges()) {
    if (sub.hasNode(g.source(e)) && sub.hasNode(g.target(e))) {
      try {
        sub.addDirectedEdgeWithKey(e, g.source(e), g.target(e), g.getEdgeAttributes(e));
      } catch {
        // duplicate multi-edge — skip
      }
    }
  }
  return sub;
}

/** Tiny seeded RNG for deterministic Louvain assignments. */
function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
