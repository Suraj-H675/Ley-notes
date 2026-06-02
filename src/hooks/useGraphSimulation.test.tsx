import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import Graph from 'graphology';
import { useGraphSimulation } from './useGraphSimulation';

function makeGraph(): Graph {
  const g = new Graph({ type: 'undirected', multi: false });
  g.addNode('a');
  g.addNode('b');
  g.addEdge('a', 'b');
  return g;
}

describe('useGraphSimulation', () => {
  it('returns a Map of positions', () => {
    const graph = makeGraph();
    const { result } = renderHook(() =>
      useGraphSimulation(graph, {
        centerForce: 1,
        chargeForce: -60,
        linkForce: 1,
        linkDistance: 80,
      })
    );
    expect(result.current.positions).toBeInstanceOf(Map);
    expect(result.current.positions.size).toBe(2);
  });

  it('provides a way to trigger a tick', () => {
    const graph = makeGraph();
    const { result } = renderHook(() =>
      useGraphSimulation(graph, {
        centerForce: 1,
        chargeForce: -60,
        linkForce: 1,
        linkDistance: 80,
      })
    );
    expect(() => act(() => result.current.tick(10))).not.toThrow();
  });

  it('reconfigures when physics changes', () => {
    const graph = makeGraph();
    const { rerender } = renderHook(
      ({ physics }: { physics: any }) => useGraphSimulation(graph, physics),
      {
        initialProps: {
          physics: { centerForce: 1, chargeForce: -60, linkForce: 1, linkDistance: 80 },
        },
      }
    );
    expect(() =>
      rerender({
        physics: { centerForce: 2, chargeForce: -100, linkForce: 1.5, linkDistance: 120 },
      })
    ).not.toThrow();
  });

  it('handles empty graphs without crashing', () => {
    const graph = new Graph({ type: 'undirected', multi: false });
    const { result } = renderHook(() =>
      useGraphSimulation(graph, {
        centerForce: 1,
        chargeForce: -60,
        linkForce: 1,
        linkDistance: 80,
      })
    );
    expect(result.current.positions.size).toBe(0);
  });
});
