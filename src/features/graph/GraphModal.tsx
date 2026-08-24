/**
 * GraphModal — full-screen graph view, the Obsidian-style "second brain" map.
 * Opens via Cmd+G (or titlebar button). Has the full controls sidebar, the
 * big interactive canvas, and a community legend.
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
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
import { useMediaQuery } from '@/shared/hooks/useMediaQuery';

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
  const [controlsOpen, setControlsOpen] = useState(false);
  const isNarrowViewport = useMediaQuery('(max-width: 767px)');
  const controlsToggleRef = useRef<HTMLButtonElement>(null);
  const controlsPanelRef = useRef<HTMLElement>(null);
  const controlsVisible = !isNarrowViewport || controlsOpen;

  const closeControls = useCallback(() => {
    setControlsOpen(false);
    queueMicrotask(() => controlsToggleRef.current?.focus());
  }, []);

  useEffect(() => {
    if (!isNarrowViewport || !controlsOpen) return;
    const frame = window.requestAnimationFrame(() => {
      controlsPanelRef.current
        ?.querySelector<HTMLElement>('input, select, button, [tabindex]:not([tabindex="-1"])')
        ?.focus();
    });
    return () => window.cancelAnimationFrame(frame);
  }, [controlsOpen, isNarrowViewport]);

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
        <Dialog.Content
          aria-describedby={undefined}
          onEscapeKeyDown={(event) => {
            if (!isNarrowViewport || !controlsOpen) return;
            event.preventDefault();
            closeControls();
          }}
          className="fixed inset-0 z-50 flex flex-col bg-background outline-none"
        >
      <Dialog.Title className="sr-only">Graph view</Dialog.Title>
      {/* Header */}
      <header className="app-chrome flex h-11 shrink-0 items-center justify-between px-3">
        <div className="flex min-w-0 items-center gap-2">
          <Maximize2 size={14} className="shrink-0 text-muted-foreground" />
          <span className="truncate text-body font-semibold">Graph view</span>
          {stats.pageCount > 0 && (
            <span className="hidden text-meta text-muted-foreground sm:inline">
              — {stats.pageCount} pages · {stats.edgeCount} edges · {stats.communityCount} clusters
            </span>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2">
        <button ref={controlsToggleRef} type="button" onClick={() => setControlsOpen((value) => !value)} aria-expanded={controlsOpen} aria-controls="graph-controls-panel" className="flex items-center gap-1 rounded-md border border-border bg-surface-2 px-2 py-1 text-meta text-muted-foreground outline-none transition-colors focus-visible:ring-2 focus-visible:ring-primary md:hidden"><SlidersHorizontal size={12} />Controls</button>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close graph view"
          className="flex items-center gap-1.5 rounded-md border border-border bg-surface-2 px-2 py-0.5 text-meta text-muted-foreground outline-none transition-colors hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-primary"
        >
          <X size={12} />
          <span className="hidden sm:inline">Close</span>
          <span className="hidden sm:inline-flex"><Kbd>esc</Kbd></span>
        </button>
        </div>
      </header>

      {/* Body: sidebar + canvas */}
      <div className="relative flex flex-1 overflow-hidden">
        {controlsOpen && <button type="button" onClick={closeControls} className="absolute inset-0 z-10 bg-background/55 outline-none backdrop-blur-sm focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary md:hidden" aria-label="Close graph controls" />}
        <aside
          ref={controlsPanelRef}
          id="graph-controls-panel"
          aria-hidden={isNarrowViewport && !controlsOpen}
          inert={isNarrowViewport && !controlsOpen}
          className={`absolute inset-y-0 left-0 z-20 shrink-0 overflow-y-auto border-r border-border bg-surface-1 transition-transform md:static md:translate-x-0 ${controlsVisible ? 'translate-x-0' : '-translate-x-full'}`}
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
