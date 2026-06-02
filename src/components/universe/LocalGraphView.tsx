import { useMemo } from 'react';
import { useGraph } from '@/hooks/useGraph';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { nHopSubgraph } from '@/lib/graph/localGraph';
import { UniverseView } from './UniverseView';
import type { KnowledgeNode, KnowledgeEdge } from '@/types';

export interface LocalGraphViewProps {
  nodeId: string;
  onNodeClick?: (id: string) => void;
}

export function LocalGraphView({ nodeId, onNodeClick }: LocalGraphViewProps) {
  const { graph, nodeMap, edgeMap, communities } = useGraph();
  const { settings, update } = useGraphSettings('local');

  const depth = settings?.localDepth ?? 1;

  // Build the N-hop subgraph. Inherit x/y from the global graph so positions
  // are consistent with the global view.
  const subgraph = useMemo(() => nHopSubgraph(graph, nodeId, depth), [
    graph,
    nodeId,
    depth,
  ]);

  // Materialize the subgraph's node/edge records for the filter+color pipeline.
  const subNodes = useMemo<KnowledgeNode[]>(() => {
    return subgraph.nodes()
      .map((id) => nodeMap.get(id))
      .filter((n): n is KnowledgeNode => Boolean(n));
  }, [subgraph, nodeMap]);

  const subEdges = useMemo<KnowledgeEdge[]>(() => {
    return subgraph.edges()
      .map((id) => edgeMap.get(id))
      .filter((e): e is KnowledgeEdge => Boolean(e));
  }, [subgraph, edgeMap]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-foreground/[0.06] px-3 py-1.5 text-[11.5px]">
        <span className="text-foreground/85">Depth</span>
        <button
          type="button"
          onClick={() =>
            settings && update({ ...settings, localDepth: 1 })
          }
          className={
            'rounded px-1.5 py-0.5 transition-colors ' +
            (depth === 1
              ? 'bg-foreground/[0.08] text-foreground'
              : 'text-foreground/75 hover:bg-foreground/[0.04]')
          }
        >
          1
        </button>
        <button
          type="button"
          onClick={() =>
            settings && update({ ...settings, localDepth: 2 })
          }
          className={
            'rounded px-1.5 py-0.5 transition-colors ' +
            (depth === 2
              ? 'bg-foreground/[0.08] text-foreground'
              : 'text-foreground/75 hover:bg-foreground/[0.04]')
          }
        >
          2
        </button>
        <span className="ml-auto text-muted-foreground/70">
          {subgraph.order} {subgraph.order === 1 ? 'node' : 'nodes'}
        </span>
      </div>
      <div className="flex-1">
        <UniverseView
          scope="local"
          onNodeClick={onNodeClick}
          graphOverride={subgraph}
          communitiesOverride={communities}
          nodesOverride={subNodes}
          edgesOverride={subEdges}
        />
      </div>
    </div>
  );
}
