/**
 * GraphCanvas — shared React Flow canvas used by GraphView and GraphModal.
 *
 * Visual style matches Obsidian / Graphify:
 *   - Tiny dots (handled by GraphNode)
 *   - Community halos (handled by GraphHalo, rendered below nodes)
 *   - Edge colors by kind (wiki=blue, embed=purple)
 *   - Dim non-hovered nodes + edges
 *   - Glowing active page
 *   - Hover info chip in the corner showing the focused node's details
 */

import '@xyflow/react/dist/style.css';

import { useCallback, useMemo, useState } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  type Node,
  type Edge,
  type NodeMouseHandler,
  ReactFlowProvider,
} from '@xyflow/react';
import type Graph from 'graphology';

import { GraphNode, EDGE_COLOR, COMMUNITY_PALETTE } from './GraphNode';
import { GraphHalo as GraphHaloComponent } from './GraphHalo';
import type { GraphNodeAttrs } from '@/core/graph/builder';
import { useNavStore } from '@/shared/state/nav';
import { cn } from '@/shared/lib/classnames';

export type ColorMode = 'community' | 'tag' | 'folder' | 'degree';

export interface GraphCanvasProps {
  graph: Graph<GraphNodeAttrs, { kind: 'wiki' | 'embed' }>;
  positions: Map<string, { x: number; y: number }>;
  activePageId: string | null;
  colorMode?: ColorMode;
  showArrows?: boolean;
  linkThickness?: number;
  enableHoverHighlight?: boolean;
  className?: string;
  /** Whether to render community halos behind the nodes. */
  showHalos?: boolean;
}

const nodeTypes = { graphNode: GraphNode, graphHalo: GraphHaloComponent };

const TAG_PALETTE = [
  'hsl(217 75% 65%)',
  'hsl(280 60% 70%)',
  'hsl(150 55% 60%)',
  'hsl(35 75% 60%)',
  'hsl(0 65% 65%)',
  'hsl(190 65% 60%)',
  'hsl(50 70% 60%)',
];

export function GraphCanvas({
  graph,
  positions,
  activePageId,
  colorMode = 'community',
  showArrows = false,
  linkThickness = 1,
  enableHoverHighlight = true,
  className,
  showHalos = true,
}: GraphCanvasProps) {
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const openPage = useNavStore((s) => s.openPage);
  const pushRecent = useNavStore((s) => s.pushRecent);

  const connectedToHovered = useMemo(() => {
    if (!hoveredId) return new Set<string>();
    if (!graph.hasNode(hoveredId)) return new Set<string>([hoveredId]);
    const set = new Set<string>([hoveredId]);
    for (const nb of graph.neighbors(hoveredId)) {
      if (nb !== null) set.add(nb);
    }
    return set;
  }, [hoveredId, graph]);

  // Tag-color map.
  const tagColors = useMemo(() => {
    const m = new Map<string, string>();
    let i = 0;
    for (const id of graph.nodes()) {
      for (const tag of graph.getNodeAttributes(id).tags) {
        if (!m.has(tag)) m.set(tag, TAG_PALETTE[i++ % TAG_PALETTE.length]);
      }
    }
    return m;
  }, [graph]);

  // Max degree over the visible graph — used to scale node sizes.
  const maxDegree = useMemo(() => {
    let m = 1;
    for (const id of graph.nodes()) {
      const d = graph.getNodeAttribute(id, 'degree');
      if (d > m) m = d;
    }
    return m;
  }, [graph]);

  // Community halos — for each community, compute the bounding circle of
  // its members' positions and render a soft glow there.
  const halos = useMemo(() => {
    if (!showHalos) return [];
    const byC = new Map<number, Array<{ x: number; y: number }>>();
    for (const id of graph.nodes()) {
      const c = graph.getNodeAttribute(id, 'community');
      const p = positions.get(id);
      if (!p) continue;
      const cur = byC.get(c) ?? [];
      cur.push(p);
      byC.set(c, cur);
    }
    const out: Array<{
      id: string;
      x: number;
      y: number;
      radius: number;
      color: string;
    }> = [];
    for (const [cId, pts] of byC) {
      if (pts.length < 2) continue;
      let cx = 0,
        cy = 0;
      for (const p of pts) {
        cx += p.x;
        cy += p.y;
      }
      cx /= pts.length;
      cy /= pts.length;
      let maxR = 0;
      for (const p of pts) {
        const dx = p.x - cx;
        const dy = p.y - cy;
        const r = Math.sqrt(dx * dx + dy * dy);
        if (r > maxR) maxR = r;
      }
      const radius = maxR + Math.max(40, pts.length * 2.5);
      out.push({
        id: `halo-${cId}`,
        x: cx - radius,
        y: cy - radius,
        radius,
        color: COMMUNITY_PALETTE[cId % COMMUNITY_PALETTE.length],
      });
    }
    return out;
  }, [graph, positions, showHalos]);

  const nodesAndEdges = useMemo(() => {
    const ns: Node[] = [];
    for (const id of graph.nodes()) {
      const attrs = graph.getNodeAttributes(id);
      const pos = positions.get(id) ?? { x: 0, y: 0 };
      const isHovered = id === hoveredId;
      const faded = enableHoverHighlight && hoveredId !== null && !connectedToHovered.has(id);
      const isActive = id === activePageId;
      // Hub nodes (top ~20% by degree) always show labels; others only on hover.
      const hubThreshold = Math.max(2, Math.floor(maxDegree * 0.3));
      const showLabel = isHovered || isActive || attrs.degree >= hubThreshold;
      const fill = nodeColor(attrs, colorMode, COMMUNITY_PALETTE, tagColors);

      ns.push({
        id,
        position: pos,
        type: 'graphNode',
        data: {
          label: attrs.label,
          degree: attrs.degree,
          community: attrs.community,
          hovered: isHovered,
          faded,
          isActive,
          maxDegree,
          showLabel,
          colorOverride: fill,
        },
      });
      void fill;
    }

    // Add halo nodes FIRST so React Flow renders them below real nodes
    // (React Flow paints in the order nodes appear in the array).
    const haloNodes: Node[] = halos.map((h) => ({
      id: h.id,
      position: { x: h.x, y: h.y },
      type: 'graphHalo',
      data: { radius: h.radius, color: h.color, opacity: 0.55 },
      draggable: false,
      selectable: false,
      connectable: false,
      focusable: false,
    }));
    ns.unshift(...haloNodes);

    const es: Edge[] = [];
    for (const e of graph.edges()) {
      const s = graph.source(e);
      const t = graph.target(e);
      const kind = (graph.getEdgeAttribute(e, 'kind') ?? 'wiki') as 'wiki' | 'embed';
      const dimmed =
        enableHoverHighlight && hoveredId !== null && s !== hoveredId && t !== hoveredId;
      const animated = hoveredId !== null && (s === hoveredId || t === hoveredId);
      es.push({
        id: e,
        source: s,
        target: t,
        style: {
          stroke: EDGE_COLOR[kind],
          strokeWidth: dimmed ? 0.4 * linkThickness : 0.8 * linkThickness,
          opacity: dimmed ? 0.08 : 0.5,
        },
        markerEnd: showArrows
          ? { type: 'arrowclosed', color: EDGE_COLOR[kind], width: 8, height: 8 }
          : undefined,
        animated,
      });
    }
    return { nodes: ns, edges: es };
  }, [graph, positions, activePageId, colorMode, tagColors, hoveredId, connectedToHovered, enableHoverHighlight, showArrows, linkThickness, halos, maxDegree]);

  // Info card content for the hovered node.
  const hoverInfo = useMemo(() => {
    if (!hoveredId || !graph.hasNode(hoveredId)) return null;
    const attrs = graph.getNodeAttributes(hoveredId);
    const neighbors = graph.neighbors(hoveredId);
    const inDeg = graph.inDegree(hoveredId);
    const outDeg = graph.outDegree(hoveredId);
    return {
      title: attrs.label,
      degree: attrs.degree,
      inDeg,
      outDeg,
      neighborCount: neighbors.length,
      tags: attrs.tags,
      folder: attrs.folder,
    };
  }, [hoveredId, graph]);

  const handleNodeClick: NodeMouseHandler = useCallback(
    (_, node) => {
      if (String(node.id).startsWith('c') || String(node.id).startsWith('halo-')) return;
      openPage(node.id);
      pushRecent(node.id);
    },
    [openPage, pushRecent],
  );

  const handleNodeMouseEnter: NodeMouseHandler = useCallback((_, node) => {
    if (String(node.id).startsWith('halo-')) return;
    setHoveredId(node.id);
  }, []);

  const handleNodeMouseLeave: NodeMouseHandler = useCallback(() => {
    setHoveredId(null);
  }, []);

  return (
    <div className={cn('relative h-full w-full', className)}>
      <ReactFlowProvider>
        <ReactFlow
          nodes={nodesAndEdges.nodes}
          edges={nodesAndEdges.edges}
          nodeTypes={nodeTypes}
          onNodeClick={handleNodeClick}
          onNodeMouseEnter={handleNodeMouseEnter}
          onNodeMouseLeave={handleNodeMouseLeave}
          fitView
          fitViewOptions={{ padding: 0.22, maxZoom: 1.2, minZoom: 0.05, duration: 500 }}
          minZoom={0.02}
          maxZoom={3}
          proOptions={{ hideAttribution: true }}
          style={{ background: 'transparent' }}
          defaultEdgeOptions={{ type: 'straight' }}
          nodesDraggable
          nodesConnectable={false}
          elementsSelectable
          panOnScroll
          zoomOnPinch
          zoomOnScroll={false}
        >
          <Background gap={32} size={1} color="hsl(220 10% 18%)" />
          <Controls showInteractive={false} />
        </ReactFlow>
      </ReactFlowProvider>

      {/* Hover info card — bottom-left, frosted glass. */}
      {hoverInfo && (
        <div
          className="pointer-events-none absolute bottom-3 left-3 max-w-[260px] rounded-md border bg-surface-1/90 px-3 py-2 text-meta backdrop-blur"
          style={{
            borderColor: 'hsl(var(--border))',
            backgroundColor: 'hsl(var(--surface-1) / 0.92)',
          }}
        >
          <div className="font-semibold text-foreground">{hoverInfo.title}</div>
          <div className="mt-1 flex gap-3 text-muted-foreground">
            <span>
              <span className="font-mono text-foreground">{hoverInfo.degree}</span> total
            </span>
            <span>
              <span className="font-mono text-foreground">{hoverInfo.inDeg}</span> in
            </span>
            <span>
              <span className="font-mono text-foreground">{hoverInfo.outDeg}</span> out
            </span>
          </div>
          {hoverInfo.tags.length > 0 && (
            <div className="mt-1 flex flex-wrap gap-1">
              {hoverInfo.tags.slice(0, 4).map((t) => (
                <span
                  key={t}
                  className="rounded-sm border border-border bg-surface-2 px-1 font-mono text-[10px] text-muted-foreground-strong"
                >
                  #{t}
                </span>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function nodeColor(
  attrs: GraphNodeAttrs,
  mode: ColorMode,
  communityPalette: string[],
  tagColors: Map<string, string>,
): string {
  switch (mode) {
    case 'community':
      return communityPalette[attrs.community % communityPalette.length];
    case 'tag':
      if (attrs.tags.length === 0) return 'hsl(220 10% 50%)';
      return tagColors.get(attrs.tags[0]) ?? 'hsl(220 10% 50%)';
    case 'folder': {
      const hash = attrs.folder.split('').reduce((a, c) => a + c.charCodeAt(0), 0);
      const hue = (hash * 47) % 360;
      return `hsl(${hue} 60% 65%)`;
    }
    case 'degree': {
      const t = Math.min(1, attrs.degree / 10);
      const hue2 = 220 - t * 220;
      return `hsl(${hue2} 65% 65%)`;
    }
  }
}