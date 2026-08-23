/**
 * Verifies the layout produces spread positions (not all overlapping at origin)
 * for the kind of graph the demo vault would create.
 */

import { describe, it, expect } from 'vitest';
import Graph from 'graphology';
import { layoutGraph } from './layout';

function makeSmallGraph(): Graph {
  const g = new Graph({ type: 'directed', multi: true, allowSelfLoops: false });
  // 25 nodes, 50 edges — matches demo-vault scale
  for (let i = 0; i < 25; i++) g.addNode(`n${i}`, { degree: 0 });
  for (let i = 0; i < 50; i++) {
    const s = (i * 7 + 3) % 25;
    const t = (i * 11 + 17) % 25;
    if (s !== t) g.addDirectedEdge(`n${s}`, `n${t}`);
  }
  return g;
}

describe('layoutGraph', () => {
  it('produces spread positions for a 25-node graph', () => {
    const g = makeSmallGraph();
    const positions = layoutGraph(g, { width: 1000, height: 700 });
    expect(positions.size).toBe(25);
    const xs = [...positions.values()].map((p) => p.x);
    const ys = [...positions.values()].map((p) => p.y);
    const xRange = Math.max(...xs) - Math.min(...xs);
    const yRange = Math.max(...ys) - Math.min(...ys);
    // With viewport 1000x700, the layout should fill at least 40% of it.
    // (40% gives slack for unlucky random initial seeds; we still want a
    // meaningful spread, not "everything at origin".)
    expect(xRange).toBeGreaterThan(400);
    expect(yRange).toBeGreaterThan(280);
  });

  it('does not collapse to a single point', () => {
    const g = makeSmallGraph();
    const positions = layoutGraph(g, { width: 1000, height: 700 });
    const uniquePositions = new Set(
      [...positions.values()].map((p) => `${Math.round(p.x)},${Math.round(p.y)}`),
    );
    // Most nodes should land at distinct positions (some clustering is OK).
    expect(uniquePositions.size).toBeGreaterThan(15);
  });

  it('returns an empty map for an empty graph', () => {
    const g = new Graph({ type: 'directed', allowSelfLoops: false });
    const positions = layoutGraph(g, { width: 1000, height: 700 });
    expect(positions.size).toBe(0);
  });
});
