import { describe, it, expect, beforeEach } from 'vitest';
import Graph from 'graphology';
import { createSimulation } from './simulation';

describe('createSimulation', () => {
  let g: Graph;
  beforeEach(() => {
    g = new Graph({ type: 'undirected', multi: false });
    g.addNode('a');
    g.addNode('b');
    g.addNode('c');
    g.addEdge('a', 'b');
    g.addEdge('b', 'c');
  });

  it('returns a handle with start/stop/positions', () => {
    const h = createSimulation(g, {
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    expect(h).toHaveProperty('start');
    expect(h).toHaveProperty('stop');
    expect(h).toHaveProperty('positions');
    h.stop();
  });

  it('assigns initial random positions to nodes that lack x/y', () => {
    const h = createSimulation(g, {
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    const positions = h.positions();
    for (const id of ['a', 'b', 'c']) {
      const p = positions.get(id);
      expect(p).toBeDefined();
      expect(typeof p!.x).toBe('number');
      expect(typeof p!.y).toBe('number');
      expect(isNaN(p!.x)).toBe(false);
      expect(isNaN(p!.y)).toBe(false);
    }
    h.stop();
  });

  it('tick advances node positions', () => {
    const h = createSimulation(g, {
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    const before = h.positions();
    h.tick(50);
    const after = h.positions();
    let moved = false;
    for (const id of ['a', 'b', 'c']) {
      if (before.get(id)!.x !== after.get(id)!.x) {
        moved = true;
        break;
      }
    }
    expect(moved).toBe(true);
    h.stop();
  });

  it('reconfigure updates the running simulation', () => {
    const h = createSimulation(g, {
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    expect(() =>
      h.reconfigure({
        centerForce: 2,
        chargeForce: -100,
        linkForce: 1.5,
        linkDistance: 120,
      })
    ).not.toThrow();
    h.stop();
  });
});
