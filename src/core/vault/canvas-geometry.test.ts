import { describe, expect, it } from 'vitest';
import {
  newTextCanvasNode,
  newGroupCanvasNode,
} from './canvas';
import {
  groupAroundContent,
  resizeCanvasNode,
} from './canvas-geometry';

describe('canvas geometry guards', () => {
  it('enforces type-specific minimum dimensions on resize', () => {
    const card = newTextCanvasNode({ x: 0, y: 0 });
    const resizedCard = resizeCanvasNode(card, 40, 20);

    expect(resizedCard.width).toBe(180);
    expect(resizedCard.height).toBe(96);

    const group = newGroupCanvasNode({ x: 0, y: 0 });
    const resizedGroup = resizeCanvasNode(group, 100, 50);

    expect(resizedGroup.width).toBe(320);
    expect(resizedGroup.height).toBe(220);
  });

  it('rejects non-finite resize values without corrupting the document', () => {
    const node = newTextCanvasNode({ x: 10, y: 10 });
    node.width = 240;
    node.height = 140;

    expect(resizeCanvasNode(node, Number.NaN, Number.POSITIVE_INFINITY)).toEqual({
      ...node,
      width: 240,
      height: 140,
    });
    expect(resizeCanvasNode(node, Number.NEGATIVE_INFINITY, Number.NaN)).toEqual({
      ...node,
      width: 240,
      height: 140,
    });
  });

  it('wraps existing content in a group with padding', () => {
    const first = newTextCanvasNode({ x: 100, y: 200 });
    first.width = 200;
    first.height = 100;
    const second = newTextCanvasNode({ x: 400, y: 300 });
    second.width = 120;
    second.height = 80;

    const group = groupAroundContent([first, second]);

    expect(group.type).toBe('group');
    expect(group.x).toBe(60);
    expect(group.y).toBe(144);
    expect(group.width).toBe(500);
    expect(group.height).toBe(276);
  });

  it('uses type minima when wrapping a single small card', () => {
    const card = newTextCanvasNode({ x: 500, y: 600 });
    card.width = 80;
    card.height = 40;

    const group = groupAroundContent([card]);

    expect(group.width).toBe(320);
    expect(group.height).toBe(220);
  });
});
