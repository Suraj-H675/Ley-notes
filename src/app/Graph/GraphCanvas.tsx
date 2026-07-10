/**
 * GraphCanvas — shared React Flow canvas. Used by both the small GraphView
 * and the full-screen GraphModal. Handles:
 *   - Node/edge styling (size by degree, color by mode)
 *   - Custom GraphNode with labels (so titles are visible without hovering)
 *   - Click to open page
 *   - Hover to highlight connections
 *   - Drag to reposition (React Flow native)
 *   - Pan/zoom (React Flow native)
 *   - Optional arrows toggle
 */

// React Flow positions nodes via CSS transforms. Without its stylesheet
// every node stacks at (0,0) and the canvas appears blank. Import the
// package CSS here so the canvas renders correctly regardless of where
// this component is mounted.
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

import { GraphNode, COMMUNITY_PALETTE } from './GraphNode';
import type { GraphNodeAttrs } from '@/core/graph/builder';
import { useNavStore } from '@/store/nav';
import { cn } from '@/lib/classnames';

export type ColorMode = 'community' | 'tag' | 'folder' | 'degree';

export interface GraphCanvasProps {
  graph: Graph<GraphNodeAttrs, { kind: 'wiki' | 'embed' }>;
  positions: Map<string, { x: number; y: number }>;
  /** Active page id — highlighted as the "you are here" node. */
  activePageId: string | null;
  colorMode?: ColorMode;
  showArrows?: boolean;
  linkThickness?: number;
  /** When true, dim non-hovered nodes when one is hovered. */
  enableHoverHighlight?: boolean;
  /** Class for the outer wrapper. */
  className?: string;
}

const nodeTypes = { graphNode: GraphNode };

const TAG_PALETTE = [
  'hsl(217 70% 62%)',
  'hsl(265 55% 65%)',
  'hsl(150 50% 55%)',
  'hsl(35 70% 60%)',
  'hsl(0 55% 58%)',
  'hsl(195 60% 55%)',
  'hsl(50 65% 55%)',
];

export function GraphCanvas({
  graph,
  positions,
  activePageId,
  colorMode = 'community',
  showArrows = true,
  linkThickness = 1,
  enableHoverHighlight = true,
  className,
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

  // Compute tag-color map once per render.
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

  const nodesAndEdges = useMemo(() => {
    // Compute maxDegree over visible nodes (matches what sizes are scaled against).
    let maxDegree = 1;
    for (const id of graph.nodes()) {
      const d = graph.getNodeAttribute(id, 'degree');
      if (d > maxDegree) maxDegree = d;
    }

    const ns: Node[] = [];
    for (const id of graph.nodes()) {
      const attrs = graph.getNodeAttributes(id);
      const pos = positions.get(id) ?? { x: 0, y: 0 };
      const isHovered = id === hoveredId;
      const faded =
        enableHoverHighlight && hoveredId !== null && !connectedToHovered.has(id);
      const isActive = id === activePageId;
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
          colorOverride: fill,
        },
      });
      void maxDegree;
      void fill;
    }

    const es: Edge[] = [];
    for (const e of graph.edges()) {
      const dimmed =
        enableHoverHighlight &&
        hoveredId !== null &&
        graph.source(e) !== hoveredId &&
        graph.target(e) !== hoveredId;
      const animated =
        hoveredId !== null &&
        (graph.source(e) === hoveredId || graph.target(e) === hoveredId);
      es.push({
        id: e,
        source: graph.source(e),
        target: graph.target(e),
        style: {
          stroke: 'hsl(var(--edge-wiki))',
          strokeWidth: linkThickness,
          opacity: dimmed ? 0.08 : 0.7,
        },
        markerEnd: showArrows
          ? { type: 'arrowclosed', color: 'hsl(var(--edge-wiki))', width: 12, height: 12 }
          : undefined,
        animated,
      });
    }
    return { nodes: ns, edges: es };
  }, [graph, positions, activePageId, colorMode, tagColors, hoveredId, connectedToHovered, enableHoverHighlight, linkThickness, showArrows]);

  const handleNodeClick: NodeMouseHandler = useCallback(
    (_, node) => {
      if (String(node.id).startsWith('c')) return; // community meta-node
      openPage(node.id);
      pushRecent(node.id);
    },
    [openPage, pushRecent],
  );

  const handleNodeMouseEnter: NodeMouseHandler = useCallback((_, node) => {
    setHoveredId(node.id);
  }, []);

  const handleNodeMouseLeave: NodeMouseHandler = useCallback(() => {
    setHoveredId(null);
  }, []);

  return (
    <div className={cn('h-full w-full', className)}>
      <ReactFlowProvider>
        <ReactFlow
          nodes={nodesAndEdges.nodes}
          edges={nodesAndEdges.edges}
          nodeTypes={nodeTypes}
          onNodeClick={handleNodeClick}
          onNodeMouseEnter={handleNodeMouseEnter}
          onNodeMouseLeave={handleNodeMouseLeave}
          fitView
          fitViewOptions={{ padding: 0.18, maxZoom: 1.4, minZoom: 0.1, duration: 400 }}
          minZoom={0.05}
          maxZoom={4}
          proOptions={{ hideAttribution: true }}
          style={{ background: 'hsl(var(--surface-1))' }}
          defaultEdgeOptions={{ type: 'straight' }}
          nodesDraggable
          nodesConnectable={false}
          elementsSelectable
        >
          <Background gap={24} size={1} color="hsl(var(--border))" />
          <Controls showInteractive={false} />
        </ReactFlow>
      </ReactFlowProvider>
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
      if (attrs.tags.length === 0) return 'hsl(220 10% 40%)';
      return tagColors.get(attrs.tags[0]) ?? 'hsl(220 10% 40%)';
    case 'folder': {
      // Hash folder name to a stable hue.
      const hash = attrs.folder.split('').reduce((a, c) => a + c.charCodeAt(0), 0);
      const hue = (hash * 47) % 360;
      return `hsl(${hue} 55% 60%)`;
    }
    case 'degree': {
      // Gradient by degree: low = blue, high = red.
      const d = attrs.degree;
      const t = Math.min(1, d / 10);
      const hue2 = 220 - t * 220;
      return `hsl(${hue2} 60% 60%)`;
    }
  }
}