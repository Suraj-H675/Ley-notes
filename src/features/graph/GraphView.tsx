/**
 * Side-panel graph preview — a small version of the graph shown in the right
 * dock. For the full Obsidian-style interactive view, use Cmd+G to open the
 * GraphModal.
 */

import { useMemo, useState } from 'react';
import { Maximize2 } from 'lucide-react';
import { useGraphData, applyLocalFilter } from './useGraphData';
import { layoutGraph } from '@/core/graph/layout';
import { GraphCanvas } from './GraphCanvas';

export function GraphView({ activePageId, onOpenFullGraph }: { activePageId: string | null; onOpenFullGraph: () => void }) {
  const [localMode, setLocalMode] = useState(false);
  const data = useGraphData();

  const fullGraph = data?.fullGraph ?? null;

  const filteredGraph = useMemo(() => {
    if (!fullGraph) return null;
    return applyLocalFilter(fullGraph, activePageId, localMode, 2);
  }, [fullGraph, activePageId, localMode]);

  const positions = useMemo(() => {
    if (!filteredGraph || filteredGraph.order === 0) {
      return new Map<string, { x: number; y: number }>();
    }
    return layoutGraph(filteredGraph, {
      width: 320,
      height: 460,
      physics: { iterations: 200 },
    });
  }, [filteredGraph]);

  return (
    <div className="relative h-full w-full">
      <div className="absolute right-2 top-2 z-10 flex gap-1">
        <button
          type="button"
          onClick={onOpenFullGraph}
          className="flex items-center gap-1 rounded-sm border border-border bg-surface-2 px-1.5 py-0.5 text-micro text-muted-foreground hover:bg-surface-3"
          title="Open full graph (⌘G)"
        >
          <Maximize2 size={10} />
        </button>
        <button
          type="button"
          onClick={() => setLocalMode((v) => !v)}
          className={
            localMode
              ? 'rounded-sm bg-primary px-2 py-0.5 text-micro font-medium text-primary-foreground'
              : 'rounded-sm border border-border bg-surface-2 px-2 py-0.5 text-micro text-muted-foreground hover:bg-surface-3'
          }
          title="Toggle local graph (centered on active page)"
        >
          {localMode ? 'Local' : 'Full'}
        </button>
      </div>

      {filteredGraph && filteredGraph.order > 0 ? (
        <GraphCanvas
          graph={filteredGraph}
          positions={positions}
          activePageId={activePageId}
          enableHoverHighlight
        />
      ) : (
        <div className="flex h-full items-center justify-center px-4 text-center text-meta text-muted-foreground">
          {data ? 'Open a page to see connections.' : 'Loading…'}
        </div>
      )}
    </div>
  );
}
