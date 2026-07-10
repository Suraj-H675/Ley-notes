/**
 * GraphNode — custom React Flow node for the Ley graph view.
 *
 * Renders a circle sized by degree + a label below (or inside for large nodes).
 * Uses memo so re-renders only happen when the node's own data changes.
 *
 * Visual hierarchy:
 *   - Node size: 8-60px based on √(degree)
 *   - Label visibility: always shown for degree >= 1, only on hover for degree 0
 *   - Active page: 2px primary border + subtle outer glow
 *   - Hovered: stays full-opacity, others dim to 25%
 */

import { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';

export interface GraphNodeData extends Record<string, unknown> {
  label: string;
  degree: number;
  community: number;
  hovered: boolean;
  faded: boolean;
  isActive: boolean;
}

function GraphNodeImpl({ data }: NodeProps) {
  const d = data as unknown as GraphNodeData;
  const size = Math.max(14, Math.min(60, 8 + Math.sqrt(d.degree) * 6));
  const showLabel = d.degree >= 1 || d.hovered;
  // Hub nodes (degree ≥ 4) get the label inside the circle; others below.
  const labelInside = size >= 36;
  const fill = COMMUNITY_PALETTE[d.community % COMMUNITY_PALETTE.length];

  return (
    <div
      className="relative flex items-center justify-center"
      style={{
        width: size,
        height: size,
        opacity: d.faded ? 0.25 : 1,
        transition: 'opacity 120ms ease',
      }}
    >
      {/* Required by React Flow even when we don't use handles — keeps
          edge anchors stable so arrows line up correctly. */}
      <Handle type="target" position={Position.Top} style={{ opacity: 0 }} />
      <Handle type="source" position={Position.Bottom} style={{ opacity: 0 }} />

      <div
        className="absolute inset-0 rounded-full"
        style={{
          background: fill,
          border: d.isActive
            ? '2px solid hsl(var(--primary))'
            : '1.5px solid rgba(255, 255, 255, 0.35)',
          boxShadow: d.isActive
            ? '0 0 0 4px hsl(var(--primary) / 0.15), 0 4px 12px hsl(220 20% 4% / 0.5)'
            : '0 2px 6px hsl(220 20% 4% / 0.4)',
        }}
      />

      {labelInside && (
        <span
          className="relative z-10 select-none px-1 text-center font-medium leading-tight text-white"
          style={{ fontSize: Math.max(9, Math.min(11, size / 5)) }}
        >
          {truncate(d.label, Math.max(3, Math.floor(size / 8)))}
        </span>
      )}

      {!labelInside && showLabel && (
        <span
          className="pointer-events-none absolute left-1/2 top-full mt-1 -translate-x-1/2 select-none whitespace-nowrap rounded-sm px-1 py-0.5 text-[10px] font-medium text-foreground"
          style={{
            background: 'hsl(var(--surface-1) / 0.9)',
            border: '1px solid hsl(var(--border))',
          }}
        >
          {truncate(d.label, 24)}
        </span>
      )}
    </div>
  );
}

function truncate(s: string, max: number): string {
  if (s.length <= max) return s;
  return s.slice(0, max - 1) + '…';
}

export const GraphNode = memo(GraphNodeImpl);

// Shared palette with GraphCanvas/GraphLegend so node fills stay in sync.
export const COMMUNITY_PALETTE = [
  'hsl(217 70% 62%)',
  'hsl(265 55% 65%)',
  'hsl(150 50% 55%)',
  'hsl(35 70% 60%)',
  'hsl(0 55% 58%)',
  'hsl(195 60% 55%)',
  'hsl(50 65% 55%)',
  'hsl(290 50% 65%)',
  'hsl(105 45% 55%)',
  'hsl(330 60% 65%)',
];