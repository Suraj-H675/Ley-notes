import { memo } from 'react';
import { BaseEdge, getBezierPath, type EdgeProps } from '@xyflow/react';

export interface UniverseEdgeData extends Record<string, unknown> {
  stroke?: string;
  thickness?: number;
  dimmed?: boolean;
  isHighlighted?: boolean;
}

export const UniverseEdge = memo(function UniverseEdge(props: EdgeProps) {
  const data = props.data as UniverseEdgeData;
  const stroke = data.stroke ?? 'hsl(220 8% 55%)';
  const thickness = (data.thickness ?? 1.5) * (data.isHighlighted ? 1.5 : 1);
  const opacity = data.dimmed ? 0.1 : data.isHighlighted ? 1 : 0.6;

  const [edgePath] = getBezierPath({
    sourceX: props.sourceX,
    sourceY: props.sourceY,
    targetX: props.targetX,
    targetY: props.targetY,
    sourcePosition: props.sourcePosition,
    targetPosition: props.targetPosition,
    curvature: 0.25,
  });

  return (
    <BaseEdge
      id={props.id}
      path={edgePath}
      style={{
        stroke,
        strokeWidth: thickness,
        opacity,
      }}
      markerEnd={props.markerEnd}
    />
  );
});
