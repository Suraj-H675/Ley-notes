/**
 * useGraphData — shared hook that loads pages + links + tags and builds the
 * graphology graph with community detection. Used by both the small side-panel
 * GraphView and the full-screen GraphModal so they share the same data layer.
 */

import { useMemo } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import Graph from 'graphology';
import { db } from '@/infrastructure/database/db';
import { buildGraph, localGraph, type GraphNodeAttrs } from '@/core/graph/builder';
import { DEFAULT_PHYSICS, type PhysicsSettings } from '@/core/graph/layout';
import type { LinkKind } from '@/infrastructure/database/schema';

export interface GraphData {
  /** The base graph (full vault). null until first load. */
  fullGraph: Graph<GraphNodeAttrs, { kind: LinkKind }> | null;
  /** The graph we're actually rendering (local or full). */
  graph: Graph<GraphNodeAttrs, { kind: LinkKind }> | null;
  /** Map: node id → community id. Stable. */
  communities: Map<string, number>;
  /** Number of distinct communities. */
  communityCount: number;
  /** Map: community id → count of nodes in that community. */
  communitySizes: Map<number, number>;
  /** True if collapsed to meta-graph (above node limit). */
  aggregated: boolean;
  /** Total page count (for stats). */
  pageCount: number;
  /** Total edge count (for stats). */
  edgeCount: number;
}

export function useGraphData(): GraphData | null {
  const raw = useLiveQuery(async () => {
    const [pages, links, tags] = await Promise.all([
      db.pages.toArray(),
      db.links.toArray(),
      db.tags.toArray(),
    ]);
    return { pages, links, tags };
  }, []);

  return useMemo(() => {
    if (!raw) return null;
    const built = buildGraph(raw.pages, raw.links, raw.tags, 5000);
    return {
      fullGraph: built.graph,
      graph: built.graph,
      communities: built.communities,
      communityCount: built.communityCount,
      communitySizes: countByCommunity(built.communities),
      aggregated: built.aggregated,
      pageCount: raw.pages.filter((p) => p.deletedAt === null && !p.missingFromDisk).length,
      edgeCount: raw.links.filter(
        (l) =>
          l.targetPageId !== null &&
          l.sourcePageId !== l.targetPageId &&
          raw.pages.some((p) => p.id === l.targetPageId && p.deletedAt === null && !p.missingFromDisk),
      ).length,
    };
  }, [raw]);
}

function countByCommunity(communities: Map<string, number>): Map<number, number> {
  const m = new Map<number, number>();
  for (const cId of communities.values()) {
    m.set(cId, (m.get(cId) ?? 0) + 1);
  }
  return m;
}

/**
 * Apply local-graph filtering to the base graph. Returns a derived graph.
 */
export function applyLocalFilter(
  base: Graph<GraphNodeAttrs, { kind: LinkKind }>,
  activePageId: string | null,
  enabled: boolean,
  depth: number,
): Graph<GraphNodeAttrs, { kind: LinkKind }> {
  if (!enabled || !activePageId || !base.hasNode(activePageId)) return base;
  return localGraph(base, activePageId, depth);
}

/**
 * Apply a node filter (search + tag include + orphans + community visibility).
 * Returns a new graph with only the matching nodes + their incident edges.
 */
export function applyNodeFilter(
  base: Graph<GraphNodeAttrs, { kind: LinkKind }>,
  opts: {
    query: string;
    tagFilter: string | null;
    orphansOnly: boolean;
    hiddenCommunities: Set<number>;
  },
): Graph<GraphNodeAttrs, { kind: LinkKind }> {
  const q = opts.query.trim().toLowerCase();
  const keep = new Set<string>();
  for (const id of base.nodes()) {
    if (id === null) continue;
    const attrs = base.getNodeAttributes(id);
    if (opts.hiddenCommunities.has(attrs.community)) continue;
    if (opts.tagFilter && !attrs.tags.includes(opts.tagFilter)) continue;
    if (q && !attrs.label.toLowerCase().includes(q)) continue;
    if (opts.orphansOnly && attrs.degree > 0) continue;
    keep.add(id);
  }
  const sub = new Graph<GraphNodeAttrs, { kind: LinkKind }>({
    type: 'directed',
    allowSelfLoops: false,
  });
  for (const id of keep) sub.addNode(id, base.getNodeAttributes(id));
  for (const e of base.edges()) {
    const s = base.source(e);
    const t = base.target(e);
    if (sub.hasNode(s) && sub.hasNode(t)) {
      try {
        sub.addDirectedEdgeWithKey(e, s, t, base.getEdgeAttributes(e));
      } catch {
        // duplicate
      }
    }
  }
  return sub;
}

export { DEFAULT_PHYSICS };
export type { PhysicsSettings };
