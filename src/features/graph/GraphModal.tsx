/**
 * GraphModal — full-screen graph view, the Obsidian-style "second brain" map.
 * Opens via Cmd+G (or titlebar button). Has the full controls sidebar, the
 * big interactive canvas, and a community legend.
 */

import { useEffect, useMemo, useState } from 'react';
import { SlidersHorizontal, X, Maximize2 } from 'lucide-react';
import { Kbd } from '@/shared/components/Kbd';
import * as Dialog from '@radix-ui/react-dialog';
import {
  applyLocalFilter,
  applyNodeFilter,
  useGraphData,
  DEFAULT_PHYSICS,
  type PhysicsSettings,
} from './useGraphData';
import { GraphCanvas, type ColorMode } from './GraphCanvas';
import { GraphControls, type GraphControlsState } from './GraphControls';
import { GraphLegend } from './GraphLegend';
import { layoutGraph } from '@/core/graph/layout';
import { useNavStore } from '@/shared/state/nav';
import { useTagFilter } from '@/shared/state/tag-filter';

const SIDEBAR_WIDTH = 280;

export function GraphModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const activePageId = useNavStore((s) => s.activeTab);
  const data = useGraphData();
  const sidebarTag = useTagFilter((state) => state.activeTag);

  const fullGraph = data?.fullGraph ?? null;

  // Filter / display / physics state.
  const [query, setQuery] = useState('');
  const [tagFilter, setTagFilter] = useState<string | null>(sidebarTag);
  const [orphansOnly, setOrphansOnly] = useState(false);
  const [colorMode, setColorMode] = useState<ColorMode>('community');
  const [linkThickness, setLinkThickness] = useState(1);
  const [showArrows, setShowArrows] = useState(true);
  const [physics, setPhysics] = useState<PhysicsSettings>(DEFAULT_PHYSICS);
  const [localEnabled, setLocalEnabled] = useState(false);
  const [localDepth, setLocalDepth] = useState(2);
  const [hiddenCommunities, setHiddenCommunities] = useState<Set<number>>(new Set());
  const [controlsOpen, setControlsOpen] = useState(() => window.matchMedia('(min-width: 768px)').matches);

  // Esc to close.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  // Derive the graph we'll render: filter → local.
  const filteredGraph = useMemo(() => {
    if (!fullGraph) return null;
    const filtered = applyNodeFilter(fullGraph, {
      query,
      tagFilter,
      orphansOnly,
      hiddenCommunities,
    });
    return applyLocalFilter(filtered, activePageId, localEnabled, localDepth);
  }, [fullGraph, query, tagFilter, orphansOnly, hiddenCommunities, activePageId, localEnabled, localDepth]);

  // Recompute layout when the graph, physics, or canvas size changes.
  const positions = useMemo(() => {
    if (!filteredGraph) return new Map<string, { x: number; y: number }>();
    return layoutGraph(filteredGraph, {
      width: 1000,
      height: 700,
      physics,
    });
  }, [filteredGraph, physics]);

  if (!open) return null;

  const controlsState: GraphControlsState = {
    query,
    setQuery,
    tagFilter,
    setTagFilter,
    orphansOnly,
    setOrphansOnly,
    colorMode,
    setColorMode,
    linkThickness,
    setLinkThickness,
    showArrows,
    setShowArrows,
    physics,
    setPhysics,
    localEnabled,
    setLocalEnabled,
    localDepth,
    setLocalDepth,
  };

  const stats = data
    ? {
        pageCount: data.pageCount,
        edgeCount: data.edgeCount,
        communityCount: data.communityCount,
      }
    : { pageCount: 0, edgeCount: 0, communityCount: 0 };

  return (
    <Dialog.Root open onOpenChange={(next) => { if (!next) onClose(); }}>
      <Dialog.Portal>
        <Dialog.Content aria-describedby={undefined} className="fixed inset-0 z-50 flex flex-col bg-background outline-none">
      <Dialog.Title className="sr-only">Graph view</Dialog.Title>
      {/* Header */}
      <header className="app-chrome flex h-11 shrink-0 items-center justify-between px-3">
        <div className="flex items-center gap-2">
          <Maximize2 size={14} className="text-muted-foreground" />
          <span className="text-body font-semibold">Graph view</span>
          {stats.pageCount > 0 && (
            <span className="hidden text-meta text-muted-foreground sm:inline">
              — {stats.pageCount} pages · {stats.edgeCount} edges · {stats.communityCount} clusters
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
        <button type="button" onClick={() => setControlsOpen((value) => !value)} className="flex items-center gap-1 rounded-md border border-border bg-surface-2 px-2 py-1 text-meta text-muted-foreground md:hidden"><SlidersHorizontal size={12} />Controls</button>
        <button
          type="button"
          onClick={onClose}
          className="flex items-center gap-1.5 rounded-md border border-border bg-surface-2 px-2 py-0.5 text-meta text-muted-foreground hover:bg-surface-3"
        >
          <X size={12} />
          Close
          <Kbd>esc</Kbd>
        </button>
        </div>
      </header>

      {/* Body: sidebar + canvas */}
      <div className="relative flex flex-1 overflow-hidden">
        {controlsOpen && <button type="button" onClick={() => setControlsOpen(false)} className="absolute inset-0 z-10 bg-background/55 backdrop-blur-sm md:hidden" aria-label="Close graph controls" />}
        <aside
          className={`absolute inset-y-0 left-0 z-20 shrink-0 overflow-hidden border-r border-border bg-surface-1 transition-transform md:static md:translate-x-0 ${controlsOpen ? 'translate-x-0' : '-translate-x-full'}`}
          style={{ width: SIDEBAR_WIDTH }}
        >
          <GraphControls state={controlsState} stats={stats} />
          {data && (
            <GraphLegend
              communitySizes={data.communitySizes}
              hiddenCommunities={hiddenCommunities}
              onToggle={(id) => {
                setHiddenCommunities((prev) => {
                  const next = new Set(prev);
                  if (next.has(id)) next.delete(id);
                  else next.add(id);
                  return next;
                });
              }}
              onToggleAll={(hide) => {
                if (hide) {
                  setHiddenCommunities(new Set(data.communitySizes.keys()));
                } else {
                  setHiddenCommunities(new Set());
                }
              }}
            />
          )}
        </aside>

        <main className="relative min-w-0 flex-1 bg-background">
          {filteredGraph ? (
            <GraphCanvas
              graph={filteredGraph}
              positions={positions}
              activePageId={activePageId}
              colorMode={colorMode}
              showArrows={showArrows}
              linkThickness={linkThickness}
              enableHoverHighlight
              showHalos
            />
          ) : (
            <div className="flex h-full items-center justify-center text-meta text-muted-foreground">
              {data ? 'No pages match the current filters.' : 'Loading…'}
            </div>
          )}
        </main>
      </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
