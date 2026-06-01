import { useCallback, useMemo, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  ReactFlow,
  MiniMap,
  Controls,
  Background,
  BackgroundVariant,
  type Node,
  type Edge,
  MarkerType,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useNodes, useEdges } from '@/hooks';
import { useUniverseStore } from '@/store';
import { useGraph } from '@/hooks/useGraph';
import { applyForceLayout, circularLayout } from '@/lib/graph/layout';
import { Button } from '@/components/ui';
import { ArrowLeft } from 'lucide-react';
import { UniverseToolbar } from '@/components/universe/UniverseToolbar';

const NODE_COLORS: Record<string, string> = {
  document: '#3b82f6',
  task: '#22c55e',
  project: '#f59e0b',
  concept: '#a855f7',
};

const EDGE_COLORS: Record<string, string> = {
  'wiki-link': '#8b5cf6',
  'explicit': '#06b6d4',
  'task-dependency': '#22c55e',
  'project-member': '#f59e0b',
  'depends-on': '#ef4444',
  'part-of': '#3b82f6',
  'related-to': '#6b7280',
  'contradicts': '#f97316',
  'extends': '#ec4899',
  'uses': '#14b8a6',
  'created-by': '#a855f7',
};

export function UniversePage() {
  const navigate = useNavigate();
  const { nodes: dbNodes } = useNodes();
  const { edges: dbEdges } = useEdges();
  const { graph, metrics } = useGraph();

  const {
    showMiniMap,
    showLabels,
    layoutMode,
    filterType,
    setSelectedNodes,
  } = useUniverseStore();

  const [nodes, setNodes] = useState<any[]>([]);
  const [edges, setEdges] = useState<any[]>([]);
  const [rfInstance, setRfInstance] = useState<any>(null);

  const degreeMap = useMemo(() => {
    const map = new Map<string, number>();
    if (metrics) {
      graph.forEachNode((nodeId) => {
        const degree = graph.degree(nodeId);
        map.set(nodeId, degree);
      });
    }
    return map;
  }, [graph, metrics]);

  useEffect(() => {
    const filteredNodes = filterType
      ? dbNodes.filter((n) => n.type === filterType)
      : dbNodes;

    const nodeIds = new Set(filteredNodes.map((n) => n.id));

    let positions: Map<string, { x: number; y: number }>;

    if (layoutMode === 'force') {
      positions = applyForceLayout(graph, { iterations: 50 });
    } else if (layoutMode === 'circular') {
      positions = circularLayout(graph);
    } else {
      positions = new Map();
      const cols = Math.ceil(Math.sqrt(filteredNodes.length));
      filteredNodes.forEach((node, i) => {
        const row = Math.floor(i / cols);
        const col = i % cols;
        positions.set(node.id, { x: col * 150, y: row * 150 });
      });
    }

    const flowNodes: Node[] = filteredNodes.map((node) => {
      const rawPos = positions.get(node.id) || { x: 0, y: 0 };
      const pos = {
        x: typeof rawPos.x === 'number' && !isNaN(rawPos.x) ? rawPos.x : 0,
        y: typeof rawPos.y === 'number' && !isNaN(rawPos.y) ? rawPos.y : 0,
      };
      const degree = degreeMap.get(node.id) || 0;
      const size = Math.min(40 + degree * 4, 80);
      const color = NODE_COLORS[node.type] || '#6b7280';

      return {
        id: node.id,
        type: 'default',
        position: pos,
        data: {
          label: showLabels ? (node.title || 'Untitled') : '',
        },
        style: {
          width: size,
          height: size,
          backgroundColor: color,
          borderRadius: '9999px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: '20px',
          cursor: 'pointer',
        },
      };
    });

    const flowEdges: Edge[] = dbEdges
      .filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target))
      .map((edge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
        type: 'smoothstep',
        data: {
          edgeType: edge.type,
          label: showLabels ? edge.type : undefined,
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          color: EDGE_COLORS[edge.type] || '#6b7280',
        },
        style: {
          stroke: EDGE_COLORS[edge.type] || '#6b7280',
          strokeWidth: 1.5,
          opacity: 0.7,
        },
      }));

    setNodes(flowNodes);
    setEdges(flowEdges);
  }, [
    dbNodes,
    dbEdges,
    filterType,
    layoutMode,
    showLabels,
    degreeMap,
    graph,
    setNodes,
    setEdges,
  ]);

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      setSelectedNodes([node.id]);
      navigate(`/page/${node.id}`);
    },
    [navigate, setSelectedNodes]
  );

  const onPaneClick = useCallback(() => {
    setSelectedNodes([]);
  }, [setSelectedNodes]);

  const fitView = useCallback(() => {
    rfInstance?.fitView({ padding: 0.2 });
  }, [rfInstance]);

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center gap-4 border-b p-4">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => navigate(-1)}
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <h1 className="text-xl font-semibold">Universe</h1>
        <div className="flex-1" />
        <UniverseToolbar onFitView={fitView} />
      </header>

      <main className="flex-1 relative">
        {dbNodes.length === 0 ? (
          <div className="h-full flex items-center justify-center text-muted-foreground">
            <div className="text-center space-y-4">
              <p className="text-4xl">🌌</p>
              <p className="text-lg font-medium">Your knowledge graph</p>
              <p>Create some pages to see your knowledge universe</p>
            </div>
          </div>
        ) : (
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodeClick={onNodeClick}
            onPaneClick={onPaneClick}
            onInit={setRfInstance}
            colorMode="dark"
            fitView
            minZoom={0.1}
            maxZoom={2}
          >
            {showMiniMap && (
              <MiniMap
                nodeColor={(node) => {
                  return (node.style?.backgroundColor as string) || '#6b7280';
                }}
                pannable
                zoomable
              />
            )}
            <Controls />
            <Background variant={BackgroundVariant.Dots} gap={16} size={1} />
          </ReactFlow>
        )}
      </main>
    </div>
  );
}
