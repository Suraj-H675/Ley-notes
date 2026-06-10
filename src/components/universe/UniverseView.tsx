import { useEffect, useMemo, useRef, useState, useCallback } from 'react';
import {
  ReactFlow,
  Background,
  MarkerType,
  type Node,
  type Edge,
  type NodeChange,
} from '@xyflow/react';
import Graph from 'graphology';
import { useGraph } from '@/hooks/useGraph';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { useGraphSimulation } from '@/hooks/useGraphSimulation';
import { useFilteredGraph } from '@/hooks/useFilteredGraph';
import { useColoredGraph } from '@/hooks/useColoredGraph';
import { ColorLegend } from './ColorLegend';
import { nodeTypes, edgeTypes } from '.';
import { db } from '@/lib/db';
import type { GraphScope } from '@/types/graph-settings.types';
import type { KnowledgeNode, KnowledgeEdge } from '@/types';
import type { CommunityResult } from '@/lib/graph/louvain';

export interface UniverseViewProps {
  scope: GraphScope;
  onNodeClick?: (nodeId: string) => void;
  /** Override the graph (e.g., for local graph). Defaults to the global graph. */
  graphOverride?: Graph;
  communitiesOverride?: CommunityResult | null;
  nodesOverride?: KnowledgeNode[];
  edgesOverride?: KnowledgeEdge[];
}

export function UniverseView({
  scope,
  onNodeClick,
  graphOverride,
  communitiesOverride,
  nodesOverride,
  edgesOverride,
}: UniverseViewProps) {
  const global = useGraph();
  const { settings } = useGraphSettings(scope);

  const graph = graphOverride ?? global.graph;
  const communities = communitiesOverride ?? global.communities;
  const rawNodes = useMemo(
    () => nodesOverride ?? Array.from(global.nodeMap.values()),
    [nodesOverride, global.nodeMap]
  );
  const rawEdges = useMemo(
    () => edgesOverride ?? Array.from(global.edgeMap.values()),
    [edgesOverride, global.edgeMap]
  );

  const filters = settings?.filters;
  const display = settings?.display;
  const physics = settings?.physics ?? {
    centerForce: 1,
    chargeForce: -60,
    linkForce: 1,
    linkDistance: 80,
  };
  const colorScheme = settings?.colorScheme ?? 'untyped';

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

  const { positions, tick, setNodePosition, pause, resume } = useGraphSimulation(graph, physics);

  const [isInteracting, setIsInteracting] = useState(false);

  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const neighborSet = useMemo(() => {
    if (!hoveredId) return new Set<string>();
    const s = new Set<string>([hoveredId]);
    if (graph.hasNode(hoveredId)) {
      graph.forEachNeighbor(hoveredId, (n) => s.add(n));
    }
    return s;
  }, [hoveredId, graph]);

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
        width: size,
        height: size,
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

  const lastFlushRef = useRef(0);
  const [, force] = useState(0);
  useEffect(() => {
    let raf = 0;
    // Throttle React Flow flushes to ~30fps. The simulation continues at 60fps
    // internally, but DOM updates happen at half rate to keep pan responsive.
    // Skip simulation ticks while the user is dragging/panning the graph.
    const loop = (t: number) => {
      if (!isInteracting) tick(1);
      if (t - lastFlushRef.current > 33) {
        lastFlushRef.current = t;
        force((n) => n + 1);
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [tick, isInteracting]);

  // Restore persisted positions on graph change.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const positions = await db.graphPositions.toArray();
      if (cancelled) return;
      for (const p of positions) {
        if (graph.hasNode(p.nodeId)) {
          graph.setNodeAttribute(p.nodeId, 'x', p.x);
          graph.setNodeAttribute(p.nodeId, 'y', p.y);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [graph]);

  // Warn at scale.
  useEffect(() => {
    if (graph.order > 2000) {
      console.warn(
        `Graph has ${graph.order} nodes. Performance may degrade beyond 2k nodes.`
      );
    }
  }, [graph.order]);

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, n: Node) => onNodeClick?.(n.id),
    [onNodeClick]
  );

  // Pause simulation while user is dragging a node so it doesn't fight back.
  const handleNodeDragStart = useCallback(() => {
    pause();
    setIsInteracting(true);
  }, [pause]);

  // Resume after drag ends and persist the pinned position.
  const handleNodeDragStop = useCallback(
    (_: React.MouseEvent, n: Node) => {
      setNodePosition(n.id, n.position.x, n.position.y);
      void db.graphPositions.put({
        nodeId: n.id,
        x: n.position.x,
        y: n.position.y,
        updatedAt: Date.now(),
      });
      setIsInteracting(false);
      resume();
    },
    [setNodePosition, resume]
  );

  // Handle node position changes from React Flow (dragging, etc.)
  const onNodesChange: (changes: NodeChange[]) => void = useCallback(
    (_changes: NodeChange[]) => {
      // React Flow needs this to properly track node positions during drag
      // We don't need to do anything with the changes since positions are
      // managed by our simulation and synced via handleNodeDragStop
    },
    []
  );

  if (graph.order === 0) {
    return (
      <div className="flex h-full w-full items-center justify-center bg-[hsl(220_14%_9%)] text-muted-foreground">
        <div className="max-w-sm space-y-1.5 text-center">
          <p className="text-[14px] text-foreground/85">No pages yet</p>
          <p className="text-[12px] text-muted-foreground/70">
            Create some pages and link them. The graph will appear here.
          </p>
        </div>
      </div>
    );
  }

  if (filtered.nodes.length === 0) {
    return (
      <div className="flex h-full w-full items-center justify-center bg-[hsl(220_14%_9%)] text-muted-foreground">
        <div className="max-w-sm space-y-1.5 text-center">
          <p className="text-[14px] text-foreground/85">No nodes match the current filters</p>
          <p className="text-[12px] text-muted-foreground/70">
            Try clearing the search or selecting fewer tags.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div
      className="relative h-full w-full bg-[hsl(220_14%_9%)]"
      onMouseUp={() => {
        if (isInteracting) {
          setIsInteracting(false);
          resume();
        }
      }}
    >
      <ReactFlow
        nodes={flowNodes}
        edges={flowEdges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onNodeClick={handleNodeClick}
        onNodesChange={onNodesChange}
        onNodeDragStart={handleNodeDragStart}
        onNodeDragStop={handleNodeDragStop}
        onPaneClick={() => {
          setIsInteracting(true);
          pause();
        }}
        colorMode="dark"
        fitView
        fitViewOptions={{ padding: 0.25 }}
        minZoom={0.1}
        maxZoom={2}
        panOnDrag
        zoomOnScroll
        zoomOnDoubleClick={false}
        panOnScroll={false}
        selectionOnDrag={false}
        proOptions={{ hideAttribution: true }}
        nodesDraggable
        nodesConnectable={false}
        elementsSelectable
        defaultEdgeOptions={{
          type: 'universe',
          markerEnd: { type: MarkerType.ArrowClosed },
        }}
      >
        <Background
          color="hsl(220 10% 22%)"
          gap={28}
          size={1.5}
          style={{ opacity: 0.6 }}
        />
      </ReactFlow>
      <ColorLegend scope={scope} />
    </div>
  );
}
