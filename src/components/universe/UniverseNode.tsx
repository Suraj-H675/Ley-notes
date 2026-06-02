import { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import { cn } from '@/lib/utils';

export interface UniverseNodeData extends Record<string, unknown> {
  label?: string;
  color?: string;
  size?: number;
  dimmed?: boolean;
  isHovered?: boolean;
  isNeighbor?: boolean;
  showLabel?: boolean;
  textFade?: number;
  onHover?: (id: string, on: boolean) => void;
}

export const UniverseNode = memo(function UniverseNode(props: NodeProps) {
  const data = props.data as UniverseNodeData;
  const size = data.size ?? 18;
  const color = data.color ?? 'hsl(220 8% 55%)';
  const opacity = data.dimmed ? 0.15 : 1;
  const outline = data.isHovered
    ? '2px solid hsl(220 15% 88%)'
    : data.isNeighbor
      ? '1.5px solid hsl(220 15% 78%)'
      : 'none';

  return (
    <div
      className="relative flex items-center justify-center"
      style={{ width: size, height: size, opacity }}
      onMouseEnter={() => data.onHover?.(props.id, true)}
      onMouseLeave={() => data.onHover?.(props.id, false)}
    >
      <Handle type="target" position={Position.Top} style={{ visibility: 'hidden' }} />
      <div
        className={cn('rounded-full transition-[outline] duration-150')}
        style={{
          width: size,
          height: size,
          backgroundColor: color,
          outline,
        }}
      />
      {data.showLabel && data.label && (
        <div
          className="pointer-events-none absolute left-1/2 top-full mt-1.5 -translate-x-1/2 whitespace-nowrap rounded bg-foreground/85 px-1.5 py-0.5 text-[10.5px] font-medium text-background"
          style={{ opacity: data.dimmed ? data.textFade ?? 0.25 : 1 }}
        >
          {data.label}
        </div>
      )}
      <Handle type="source" position={Position.Bottom} style={{ visibility: 'hidden' }} />
    </div>
  );
});
