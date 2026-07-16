import { memo, type KeyboardEvent, type MouseEvent } from "react";
import { ViewportPortal } from "@xyflow/react";
import {
  canvasColorValue,
  type CanvasDocument,
  type CanvasNode,
  type CanvasSide,
} from "@/core/vault/canvas";

interface CanvasEdgeLayerProps {
  document: CanvasDocument;
  selectedEdgeId: string | null;
  onSelect: (id: string) => void;
}

export const CanvasEdgeLayer = memo(function CanvasEdgeLayer({
  document,
  selectedEdgeId,
  onSelect,
}: CanvasEdgeLayerProps) {
  const nodes = new Map(document.nodes.map((node) => [node.id, node]));

  return (
    <ViewportPortal>
      <svg
        className="pointer-events-none absolute left-0 top-0 z-0 overflow-visible"
        width="1"
        height="1"
        aria-hidden="false"
      >
        <defs>
          {document.edges.map((edge) => {
            const color =
              canvasColorValue(edge.color) ?? "hsl(var(--muted-foreground))";
            return (
              <marker
                key={edge.id}
                id={markerId(edge.id)}
                markerWidth="8"
                markerHeight="8"
                refX="7"
                refY="4"
                orient="auto"
                markerUnits="strokeWidth"
              >
                <path d="M 0 0 L 8 4 L 0 8 z" fill={color} />
              </marker>
            );
          })}
        </defs>
        {document.edges.map((edge) => {
          const from = nodes.get(edge.fromNode);
          const to = nodes.get(edge.toNode);
          if (!from || !to) return null;
          const geometry = edgeGeometry(
            from,
            edge.fromSide ?? "right",
            to,
            edge.toSide ?? "left",
          );
          const color =
            canvasColorValue(edge.color) ?? "hsl(var(--muted-foreground))";
          const selected = edge.id === selectedEdgeId;
          const labelWidth = Math.max(42, (edge.label?.length ?? 0) * 7 + 18);
          const select = (event: MouseEvent<SVGPathElement>) => {
            event.stopPropagation();
            onSelect(edge.id);
          };
          const selectWithKeyboard = (event: KeyboardEvent<SVGPathElement>) => {
            if (event.key !== "Enter" && event.key !== " ") return;
            event.preventDefault();
            onSelect(edge.id);
          };
          return (
            <g key={edge.id}>
              <path
                d={geometry.path}
                fill="none"
                stroke={selected ? "hsl(var(--primary))" : color}
                strokeWidth={selected ? 3 : 1.8}
                markerStart={
                  edge.fromEnd === "arrow"
                    ? `url(#${markerId(edge.id)})`
                    : undefined
                }
                markerEnd={
                  edge.toEnd === "none"
                    ? undefined
                    : `url(#${markerId(edge.id)})`
                }
              />
              <path
                d={geometry.path}
                fill="none"
                stroke="transparent"
                strokeWidth="18"
                className="pointer-events-auto cursor-pointer focus:outline-none"
                role="button"
                tabIndex={0}
                aria-label={
                  edge.label ? `Connection: ${edge.label}` : "Canvas connection"
                }
                onClick={select}
                onKeyDown={selectWithKeyboard}
              />
              {edge.label && (
                <g
                  transform={`translate(${geometry.label.x} ${geometry.label.y})`}
                >
                  <rect
                    x={-labelWidth / 2}
                    y="-12"
                    width={labelWidth}
                    height="24"
                    rx="7"
                    fill="hsl(var(--surface-1))"
                    stroke={
                      selected ? "hsl(var(--primary))" : "hsl(var(--border))"
                    }
                  />
                  <text
                    textAnchor="middle"
                    dominantBaseline="central"
                    fill="hsl(var(--foreground))"
                    fontSize="12"
                    fontWeight="600"
                  >
                    {edge.label}
                  </text>
                </g>
              )}
            </g>
          );
        })}
      </svg>
    </ViewportPortal>
  );
});

function edgeGeometry(
  from: CanvasNode,
  fromSide: CanvasSide,
  to: CanvasNode,
  toSide: CanvasSide,
) {
  const start = sidePoint(from, fromSide);
  const end = sidePoint(to, toSide);
  const distance = Math.hypot(end.x - start.x, end.y - start.y);
  const bend = Math.max(48, Math.min(220, distance * 0.45));
  const first = offsetPoint(start, fromSide, bend);
  const second = offsetPoint(end, toSide, bend);
  return {
    path: `M ${start.x} ${start.y} C ${first.x} ${first.y}, ${second.x} ${second.y}, ${end.x} ${end.y}`,
    label: cubicPoint(start, first, second, end, 0.5),
  };
}

function sidePoint(node: CanvasNode, side: CanvasSide) {
  if (side === "top") return { x: node.x + node.width / 2, y: node.y };
  if (side === "right")
    return { x: node.x + node.width, y: node.y + node.height / 2 };
  if (side === "bottom")
    return { x: node.x + node.width / 2, y: node.y + node.height };
  return { x: node.x, y: node.y + node.height / 2 };
}

function offsetPoint(
  point: { x: number; y: number },
  side: CanvasSide,
  distance: number,
) {
  if (side === "top") return { x: point.x, y: point.y - distance };
  if (side === "right") return { x: point.x + distance, y: point.y };
  if (side === "bottom") return { x: point.x, y: point.y + distance };
  return { x: point.x - distance, y: point.y };
}

function cubicPoint(
  start: { x: number; y: number },
  first: { x: number; y: number },
  second: { x: number; y: number },
  end: { x: number; y: number },
  t: number,
) {
  const inverse = 1 - t;
  return {
    x:
      inverse ** 3 * start.x +
      3 * inverse ** 2 * t * first.x +
      3 * inverse * t ** 2 * second.x +
      t ** 3 * end.x,
    y:
      inverse ** 3 * start.y +
      3 * inverse ** 2 * t * first.y +
      3 * inverse * t ** 2 * second.y +
      t ** 3 * end.y,
  };
}

function markerId(id: string): string {
  let hash = 2166136261;
  for (const character of id) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }
  return `canvas-arrow-${(hash >>> 0).toString(36)}`;
}
