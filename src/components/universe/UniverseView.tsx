import { useEffect, useMemo, useRef, useState, useCallback } from 'react';
import {
  ReactFlow,
  Background,
  type Node,
  type Edge,
} from '@xyflow/react';
import { useGraph } from '@/hooks/useGraph';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { useGraphSimulation } from '@/hooks/useGraphSimulation';
import { useFilteredGraph } from '@/hooks/useFilteredGraph';
import { useColoredGraph } from '@/hooks/useColoredGraph';
import { ColorLegend } from './ColorLegend';
import { nodeTypes, edgeTypes } from '.';
import type { GraphScope } from '@/types/graph-settings.types';

export interface UniverseViewProps {
  scope: GraphScope;
  onNodeClick?: (nodeId: string) => void;
}

export function UniverseView({ scope, onNodeClick }: UniverseViewProps) {
  const { graph, communities, nodeMap, edgeMap } = useGraph();
  const { settings } = useGraphSettings(scope);

  const filters = settings?.filters;
  const display = settings?.display;
  const physics = settings?.physics ?? {
    centerForce: 1,
    chargeForce: -60,
    linkForce: 1,
    linkDistance: 80,
  };
  const colorScheme = settings?.colorScheme ?? 'untyped';

  // Materialize the graph into node/edge arrays for filtering.
  const rawNodes = useMemo(() => Array.from(nodeMap.values()), [nodeMap]);
  const rawEdges = useMemo(() => Array.from(edgeMap.values()), [edgeMap]);

  const filtered = useFilteredGraph(
    rawNodes,
    rawEdges,
    filters ?? {
      searchQuery: '',
      selectedTags: [],
      selectedCollections: [],
      showOrphans: true,
    }
  );

  const colorMap = useColoredGraph(
    filtered.nodes,
    filtered.edges,
    graph,
    colorScheme,
    communities
  );

  // Run the simulation against the FULL graph (positions persist across filters).
  const { positions, tick } = useGraphSimulation(graph, physics);

  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const neighborSet = useMemo(() => {
    if (!hoveredId) return new Set<string>();
    const s = new Set<string>([hoveredId]);
    if (graph.hasNode(hoveredId)) {
      graph.forEachNeighbor(hoveredId, (n) => s.add(n));
    }
    return s;
  }, [hoveredId, graph]);

  // Build React Flow nodes/edges from filtered graph + live positions.
  const flowNodes = useMemo<Node[]>(() => {
    return filtered.nodes.map((n) => {
      const pos = positions.get(n.id) ?? { x: 0, y: 0 };
      const color = colorMap.get(n.id) ?? 'hsl(220 8% 55%)';
      const degree = graph.hasNode(n.id) ? graph.degree(n.id) : 0;
      const size = Math.min(32, 6 + Math.log(1 + degree) * 8) * (display?.nodeSize ?? 1);
      const isHovered = hoveredId === n.id;
      const isNeighbor = neighborSet.has(n.id) && !isHovered;
      const dimmed = hoveredId !== null && !isHovered && !isNeighbor;
      return {
        id: n.id,
        type: 'universe',
        position: pos,
        data: {
          label: n.title,
          color,
          size,
          isHovered,
          isNeighbor,
          dimmed,
          showLabel: display?.showLabels ?? true,
          textFade: display?.textFade ?? 0.25,
          onHover: (id: string, on: boolean) => setHoveredId(on ? id : null),
        },
      };
    });
  }, [filtered.nodes, positions, colorMap, graph, hoveredId, neighborSet, display]);

  const flowEdges = useMemo<Edge[]>(() => {
    return filtered.edges.map((e) => {
      const color = colorMap.get(e.source) ?? 'hsl(220 8% 55%)';
      const dimmed =
        hoveredId !== null &&
        !(hoveredId === e.source || hoveredId === e.target) &&
        !(neighborSet.has(e.source) || neighborSet.has(e.target));
      const isHighlighted = !dimmed && hoveredId !== null;
      return {
        id: e.id,
        source: e.source,
        target: e.target,
        type: 'universe',
        data: {
          stroke: color,
          thickness: display?.edgeThickness ?? 1,
          dimmed,
          isHighlighted,
        },
      };
    });
  }, [filtered.edges, colorMap, hoveredId, neighborSet, display]);

  // Drive the simulation with a RAF loop, throttled to ~30fps for React Flow flushes.
  const lastFlushRef = useRef(0);
  const [, force] = useState(0);
  useEffect(() => {
    let raf = 0;
    const loop = (t: number) => {
      tick(1);
      if (t - lastFlushRef.current > 33) {
        lastFlushRef.current = t;
        force((n) => n + 1);
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [tick]);

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, n: Node) => onNodeClick?.(n.id),
    [onNodeClick]
  );

  return (
    <div className="relative h-full w-full bg-[hsl(220_14%_9%)]">
      <ReactFlow
        nodes={flowNodes}
        edges={flowEdges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodeClick={handleNodeClick}
        colorMode="dark"
        fitView
        minZoom={0.05}
        maxZoom={4}
        proOptions={{ hideAttribution: true }}
        nodesDraggable
        nodesConnectable={false}
        elementsSelectable
        zoomOnDoubleClick={false}
      >
        <Background color="transparent" gap={20} size={0} />
      </ReactFlow>
      <ColorLegend scope={scope} />
    </div>
  );
}
