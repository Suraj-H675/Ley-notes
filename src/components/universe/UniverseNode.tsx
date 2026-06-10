import { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';

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
  const size = data.size ?? 8;
  const color = data.color ?? 'hsl(220 8% 55%)';
  const opacity = data.dimmed ? 0.2 : 1;

  const isHovered = data.isHovered;
  const isNeighbor = data.isNeighbor && !isHovered;

  return (
    <div
      className="relative flex items-center justify-center"
      style={{ width: size, height: size, opacity }}
      onMouseEnter={() => data.onHover?.(props.id, true)}
      onMouseLeave={() => data.onHover?.(props.id, false)}
    >
      {/* Invisible handles — React Flow uses these for edge endpoint positions */}
      <Handle
        id="top"
        type="target"
        position={Position.Top}
        style={{ visibility: 'hidden', width: 1, height: 1 }}
      />
      <Handle
        id="bottom"
        type="source"
        position={Position.Bottom}
        style={{ visibility: 'hidden', width: 1, height: 1 }}
      />

      {/* Outer glow on hover - Obsidian style */}
      {isHovered && (
        <div
          className="absolute rounded-full"
          style={{
            width: size + 12,
            height: size + 12,
            backgroundColor: color,
            opacity: 0.15,
            filter: 'blur(6px)',
          }}
        />
      )}
      {/* Ring on neighbor - subtle indicator */}
      {isNeighbor && (
        <div
          className="absolute rounded-full"
          style={{
            width: size + 8,
            height: size + 8,
            border: `1px solid ${color}`,
            opacity: 0.35,
          }}
        />
      )}
      {/* Core node - solid circle like Obsidian */}
      <div
        className="relative rounded-full"
        style={{
          width: size,
          height: size,
          backgroundColor: color,
          boxShadow: isHovered
            ? `0 0 0 1.5px ${color}40, 0 0 12px 3px ${color}60`
            : isNeighbor
              ? `0 0 0 1px ${color}50`
              : `0 0 3px 0.5px ${color}25`,
        }}
      />
      {/* Label - fades in based on textFade threshold, positioned below node */}
      {data.label && (isHovered || isNeighbor) && (
        <div
          className="pointer-events-none absolute left-1/2 -bottom-5 -translate-x-1/2 whitespace-nowrap rounded-sm bg-black/80 px-1.5 py-0.5 text-[9px] font-medium leading-tight text-white/90"
          style={{
            opacity: isNeighbor ? (data.textFade ?? 0.25) : 0.95,
          }}
        >
          {data.label}
        </div>
      )}
    </div>
  );
});