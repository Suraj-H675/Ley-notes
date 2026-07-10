/**
 * GraphHalo — soft background circle around each community's nodes.
 * Renders as a non-interactive React Flow node so it lives in the same
 * coordinate space and zooms/pans with the rest of the graph.
 *
 * Positioning: the caller sets Node.position to (cx - radius, cy - radius).
 * We then render a div starting at (0,0) of that position so the visual
 * center lands on (cx, cy).
 *
 * Sits below real nodes because GraphCanvas prepends halos to the nodes
 * array — React Flow paints nodes in array order.
 */

import { memo } from 'react';
import type { NodeProps } from '@xyflow/react';

export interface HaloNodeData extends Record<string, unknown> {
  radius: number;
  color: string;
  opacity: number;
}

function GraphHaloImpl({ data }: NodeProps) {
  const d = data as unknown as HaloNodeData;
  return (
    <div
      className="pointer-events-none"
      style={{
        width: d.radius * 2,
        height: d.radius * 2,
        background: `radial-gradient(circle, ${d.color}33 0%, ${d.color}14 35%, ${d.color}00 75%)`,
        opacity: d.opacity,
        borderRadius: '50%',
      }}
    />
  );
}

export const GraphHalo = memo(GraphHaloImpl);