import { beforeEach, describe, expect, it } from 'vitest';
import { resetDb } from '@/test/helpers';
import { createCanvas, deleteCanvas, listCanvases, newTextCanvasNode, saveCanvas } from './canvas';

describe('JSON Canvas persistence', () => {
  beforeEach(async () => resetDb());

  it('creates and updates an interoperable browser-local canvas document', async () => {
    const canvas = await createCanvas('Research map');
    expect(canvas.path).toBe('canvases/research-map.canvas');

    const node = newTextCanvasNode({ x: 40, y: 80 });
    node.text = 'Question → evidence → conclusion';
    await saveCanvas(canvas.path, { nodes: [node], edges: [] });

    const [restored] = await listCanvases();
    expect(restored.document.nodes).toEqual([node]);
    expect(restored.document.edges).toEqual([]);

    await deleteCanvas(canvas.path);
    expect(await listCanvases()).toEqual([]);
  });
});
