/**
 * GraphNode — custom React Flow node for the Ley graph view.
 *
 * Visual style matches Obsidian / Graphify:
 *   - Tiny dots (4-10px diameter)
 *   - No label by default — label appears on hover OR for hub nodes
 *   - Strong glow on the active page
 *   - Subtle hover ring (no jarring border change)
 *   - Soft scale-up animation when hovered
 *
 * Reads its color from the shared COMMUNITY_PALETTE via the node's `community`
 * attribute. The HoverInfo overlay (rendered separately by GraphCanvas) shows
 * the full title — this node just renders the dot.
 */

import { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import type { LinkKind } from '@/infrastructure/database/schema';

export interface GraphNodeData extends Record<string, unknown> {
  label: string;
  degree: number;
  community: number;
  hovered: boolean;
  faded: boolean;
  isActive: boolean;
  /** Max degree over visible nodes — used to scale node size proportionally. */
  maxDegree: number;
  /** Show the chip label only when true (hover or active or hub). */
  showLabel: boolean;
}

function GraphNodeImpl({ data }: NodeProps) {
  const d = data as unknown as GraphNodeData;
  // Size: 4px (isolated leaf) up to 11px (extreme hub). Sqrt curve so hub
  // pages stand out without dominating the canvas.
  const ratio = d.maxDegree > 0 ? d.degree / d.maxDegree : 0;
  const size = 4 + Math.sqrt(ratio) * 7;

  const fill = COMMUNITY_PALETTE[d.community % COMMUNITY_PALETTE.length];

  return (
    <div
      className="relative flex items-center justify-center"
      style={{
        width: 28,
        height: 28,
        opacity: d.faded ? 0.18 : 1,
        transition: 'opacity 140ms ease, transform 140ms ease',
        transform: d.hovered ? 'scale(1.25)' : 'scale(1)',
      }}
    >
      <Handle type="target" position={Position.Top} style={{ opacity: 0 }} />
      <Handle type="source" position={Position.Bottom} style={{ opacity: 0 }} />

      {/* Glow halo — only visible for active node. */}
      {d.isActive && (
        <div
          className="absolute rounded-full"
          style={{
            width: size * 4,
            height: size * 4,
            background: `radial-gradient(circle, ${fill}66 0%, ${fill}00 70%)`,
            pointerEvents: 'none',
          }}
        />
      )}

      {/* The dot itself. */}
      <div
        className="relative rounded-full"
        style={{
          width: size,
          height: size,
          background: fill,
          border: d.hovered
            ? '1.5px solid rgba(255, 255, 255, 0.95)'
            : '1px solid rgba(255, 255, 255, 0.15)',
          boxShadow: d.hovered ? `0 0 8px ${fill}` : 'none',
        }}
      />

      {/* Label chip — shown on hover, active, or hub. */}
      {d.showLabel && (
        <div
          className="pointer-events-none absolute left-1/2 top-full mt-1.5 -translate-x-1/2 select-none whitespace-nowrap rounded-sm px-1.5 py-0.5 font-mono text-micro font-medium text-foreground"
          style={{
            background: 'hsl(220 14% 9% / 0.92)',
            border: '1px solid hsl(220 10% 22%)',
            color: 'hsl(220 14% 92%)',
          }}
        >
          {truncate(d.label, 30)}
        </div>
      )}
    </div>
  );
}

function truncate(s: string, max: number): string {
  if (s.length <= max) return s;
  return s.slice(0, max - 1) + '…';
}

export const GraphNode = memo(GraphNodeImpl);

/**
 * Shared palette — keep node fills, halo fills, and legend swatches in sync.
 * Chosen to be distinguishable on a dark navy background.
 */
export const COMMUNITY_PALETTE = [
  'hsl(217 75% 65%)', // blue
  'hsl(280 60% 70%)', // purple
  'hsl(150 55% 60%)', // green
  'hsl(35 75% 60%)',  // orange
  'hsl(0 65% 65%)',   // red
  'hsl(190 65% 60%)', // cyan
  'hsl(50 70% 60%)',  // yellow
  'hsl(310 60% 70%)', // magenta
  'hsl(110 50% 60%)', // lime
  'hsl(25 65% 65%)',  // coral
];

/**
 * Edge colors keyed by link kind.
 */
export const EDGE_COLOR: Record<LinkKind, string> = {
  wiki: 'hsl(217 50% 70%)',
  embed: 'hsl(280 50% 75%)',
  markdown: 'hsl(190 55% 68%)',
};
