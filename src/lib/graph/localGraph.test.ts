import { describe, it, expect } from 'vitest';
import Graph from 'graphology';
import { nHopSubgraph } from './localGraph';

function buildGraph(): Graph {
  const g = new Graph({ type: 'undirected', multi: false });
  //  a - b - c
  //  |
  //  d - e
  g.addNode('a');
  g.addNode('b');
  g.addNode('c');
  g.addNode('d');
  g.addNode('e');
  g.addNode('x');
  g.addEdge('a', 'b');
  g.addEdge('b', 'c');
  g.addEdge('a', 'd');
  g.addEdge('d', 'e');
  return g;
}

describe('nHopSubgraph', () => {
  it('depth=1 returns the node and direct neighbors', () => {
    const sub = nHopSubgraph(buildGraph(), 'a', 1);
    expect(sub.order).toBe(3);
    expect(sub.hasNode('a')).toBe(true);
    expect(sub.hasNode('b')).toBe(true);
    expect(sub.hasNode('d')).toBe(true);
    expect(sub.hasNode('c')).toBe(false);
    expect(sub.hasNode('e')).toBe(false);
  });

  it('depth=2 returns up to 2-hop neighborhood', () => {
    const sub = nHopSubgraph(buildGraph(), 'a', 2);
    expect(sub.order).toBe(5);
    expect(sub.hasNode('c')).toBe(true);
    expect(sub.hasNode('e')).toBe(true);
  });

  it('excludes disconnected nodes', () => {
    const sub = nHopSubgraph(buildGraph(), 'a', 2);
    expect(sub.hasNode('x')).toBe(false);
  });

  it('only includes edges where both endpoints are in the subgraph', () => {
    const sub = nHopSubgraph(buildGraph(), 'a', 1);
    expect(sub.hasEdge('a', 'b')).toBe(true);
    expect(sub.hasEdge('a', 'd')).toBe(true);
    expect(sub.hasEdge('b', 'c')).toBe(false);
    expect(sub.hasEdge('d', 'e')).toBe(false);
  });

  it('handles a node with no neighbors by returning just that node', () => {
    const g = new Graph({ type: 'undirected', multi: false });
    g.addNode('solo');
    const sub = nHopSubgraph(g, 'solo', 1);
    expect(sub.order).toBe(1);
    expect(sub.hasNode('solo')).toBe(true);
  });

  it('returns empty graph for missing center node', () => {
    const sub = nHopSubgraph(buildGraph(), 'nope', 2);
    expect(sub.order).toBe(0);
  });
});
