import { newGroupCanvasNode, type CanvasNode } from './canvas';

export const CANVAS_MIN_WIDTH = 180;
export const CANVAS_MIN_HEIGHT = 96;
export const GROUP_MIN_WIDTH = 320;
export const GROUP_MIN_HEIGHT = 220;

export function resizeCanvasNode<T extends CanvasNode>(
  node: T,
  width: number,
  height: number,
): T {
  const minWidth = node.type === 'group' ? GROUP_MIN_WIDTH : CANVAS_MIN_WIDTH;
  const minHeight = node.type === 'group' ? GROUP_MIN_HEIGHT : CANVAS_MIN_HEIGHT;
  return {
    ...node,
    width: Number.isFinite(width) ? Math.max(minWidth, width) : node.width,
    height: Number.isFinite(height) ? Math.max(minHeight, height) : node.height,
  };
}

export function groupAroundContent(nodes: CanvasNode[]): CanvasNode {
  const content = nodes.filter((node) => node.type !== 'group');
  if (content.length === 0) return newGroupCanvasNode({ x: 40, y: 40 });
  const left = Math.min(...content.map((node) => node.x));
  const top = Math.min(...content.map((node) => node.y));
  const right = Math.max(...content.map((node) => node.x + node.width));
  const bottom = Math.max(...content.map((node) => node.y + node.height));
  return {
    ...newGroupCanvasNode({ x: left - 40, y: top - 56 }),
    width: Math.max(GROUP_MIN_WIDTH, right - left + 80),
    height: Math.max(GROUP_MIN_HEIGHT, bottom - top + 96),
  };
}
