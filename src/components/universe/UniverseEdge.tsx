import { memo } from 'react';
import { BaseEdge, getStraightPath, type EdgeProps } from '@xyflow/react';

export interface UniverseEdgeData extends Record<string, unknown> {
  stroke?: string;
  thickness?: number;
  dimmed?: boolean;
  isHighlighted?: boolean;
}

export const UniverseEdge = memo(function UniverseEdge(props: EdgeProps) {
  // Obsidian uses straight lines - clean, minimal connections
  const [path] = getStraightPath({
    sourceX: props.sourceX,
    sourceY: props.sourceY,
    targetX: props.targetX,
    targetY: props.targetY,
  });

  const data = props.data as UniverseEdgeData;
  const isHighlighted = data.isHighlighted && !data.dimmed;
  const thickness = (data.thickness as number) ?? 1.5;
  const opacity = data.dimmed ? 0.08 : isHighlighted ? 1 : 0.5;

  return (
    <BaseEdge
      id={props.id}
      path={path}
      style={{
        strokeWidth: thickness,
        opacity,
        strokeDasharray: isHighlighted ? '6 3' : undefined,
      }}
    />
  );
});