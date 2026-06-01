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
import { PageHeader } from '@/components/layout';
import { UniverseToolbar } from '@/components/universe/UniverseToolbar';

// HSL values match the new design tokens
const NODE_COLORS: Record<string, string> = {
  document: 'hsl(217 70% 62%)',
  task: 'hsl(150 50% 55%)',
  project: 'hsl(265 55% 65%)',
  concept: 'hsl(35 70% 60%)',
};

const EDGE_COLORS: Record<string, string> = {
  'wiki-link': 'hsl(225 55% 60%)',
  explicit: 'hsl(265 50% 62%)',
  'task-dependency': 'hsl(150 50% 55%)',
  'project-member': 'hsl(35 70% 60%)',
  'depends-on': 'hsl(0 55% 58%)',
  'part-of': 'hsl(217 70% 62%)',
  'related-to': 'hsl(220 8% 55%)',
  contradicts: 'hsl(15 65% 58%)',
  extends: 'hsl(295 50% 65%)',
  uses: 'hsl(170 50% 50%)',
  'created-by': 'hsl(265 55% 65%)',
};

export function UniversePage() {
  const navigate = useNavigate();
  const { nodes: dbNodes } = useNodes();
  const { edges: dbEdges } = useEdges();
  const { graph, metrics } = useGraph();

  const { showMiniMap, showLabels, layoutMode, filterType, setSelectedNodes } = useUniverseStore();

  const [nodes, setNodes] = useState<any[]>([]);
  const [edges, setEdges] = useState<any[]>([]);
  const [rfInstance, setRfInstance] = useState<any>(null);

  const degreeMap = useMemo(() => {
    const map = new Map<string, number>();
    if (metrics) {
      graph.forEachNode((nodeId) => {
        map.set(nodeId, graph.degree(nodeId));
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
        positions.set(node.id, {
          x: (i % cols) * 160,
          y: Math.floor(i / cols) * 160,
        });
      });
    }

    const flowNodes: Node[] = filteredNodes.map((node) => {
      const raw = positions.get(node.id) || { x: 0, y: 0 };
      const pos = {
        x: typeof raw.x === 'number' && !isNaN(raw.x) ? raw.x : 0,
        y: typeof raw.y === 'number' && !isNaN(raw.y) ? raw.y : 0,
      };
      const degree = degreeMap.get(node.id) || 0;
      const size = Math.min(28 + degree * 3, 56);
      const color = NODE_COLORS[node.type] || 'hsl(220 8% 55%)';

      return {
        id: node.id,
        type: 'default',
        position: pos,
        data: { label: showLabels ? (node.title || 'Untitled') : '' },
        style: {
          width: size,
          height: size,
          backgroundColor: color,
          borderRadius: '9999px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: '14px',
          fontWeight: 500,
          color: 'hsl(220 14% 8%)',
          cursor: 'pointer',
          border: '2px solid hsl(220 14% 7% / 0.4)',
        },
      };
    });

    const flowEdges: Edge[] = dbEdges
      .filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target))
      .map((edge) => {
        const color = EDGE_COLORS[edge.type] || 'hsl(220 8% 55%)';
        return {
          id: edge.id,
          source: edge.source,
          target: edge.target,
          type: 'smoothstep',
          data: { edgeType: edge.type, label: showLabels ? edge.type : undefined },
          label: showLabels ? edge.type : undefined,
          markerEnd: { type: MarkerType.ArrowClosed, color },
          style: { stroke: color, strokeWidth: 1.25, opacity: 0.6 },
          labelStyle: { fill: 'hsl(220 10% 70%)', fontSize: 10, fontWeight: 500 },
          labelBgStyle: { fill: 'hsl(220 14% 9%)', fillOpacity: 0.8 },
          labelBgPadding: [4, 2] as [number, number],
          labelBgBorderRadius: 4,
        };
      });

    setNodes(flowNodes);
    setEdges(flowEdges);
  }, [dbNodes, dbEdges, filterType, layoutMode, showLabels, degreeMap, graph]);

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      setSelectedNodes([node.id]);
      navigate(`/page/${node.id}`);
    },
    [navigate, setSelectedNodes]
  );

  const onPaneClick = useCallback(() => setSelectedNodes([]), [setSelectedNodes]);

  const fitView = useCallback(() => rfInstance?.fitView({ padding: 0.2 }), [rfInstance]);

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Universe"
        subtitle={`${dbNodes.length} pages, ${dbEdges.length} edges`}
        actions={<UniverseToolbar onFitView={fitView} />}
      />

      <main className="relative flex-1">
        {dbNodes.length === 0 ? (
          <div className="flex h-full items-center justify-center text-muted-foreground">
            <div className="max-w-sm space-y-2 text-center">
              <p className="text-[15px] text-foreground/90">No pages yet</p>
              <p className="text-[13px] text-muted-foreground/70">
                Create some pages and link them. The graph will appear here.
              </p>
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
            proOptions={{ hideAttribution: true }}
          >
            {showMiniMap && (
              <MiniMap
                nodeColor={(node) => (node.style?.backgroundColor as string) || '#6b7280'}
                pannable
                zoomable
                style={{ background: 'hsl(220 14% 9% / 0.6)' }}
                maskColor="hsl(220 14% 7% / 0.6)"
              />
            )}
            <Controls
              style={{
                background: 'hsl(220 14% 11%)',
                border: '1px solid hsl(220 10% 18%)',
                borderRadius: 6,
              }}
            />
            <Background
              variant={BackgroundVariant.Dots}
              gap={20}
              size={1}
              color="hsl(220 10% 25%)"
            />
          </ReactFlow>
        )}
      </main>
    </div>
  );
}
