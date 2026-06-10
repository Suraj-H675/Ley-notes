import { useEffect, useMemo, useRef, useState } from 'react';
import Graph from 'graphology';
import { createSimulation, type SimulationHandle } from '@/lib/graph/simulation';
import type { PhysicsConfig } from '@/types/graph-settings.types';

export function useGraphSimulation(
  graph: Graph,
  physics: PhysicsConfig
): {
  positions: Map<string, { x: number; y: number }>;
  tick: (iterations?: number) => void;
  setNodePosition: (nodeId: string, x: number, y: number) => void;
  pause: () => void;
  resume: () => void;
} {
  // Recreate the simulation only when the graph topology changes.
  const graphKey = useMemo(
    () => `${graph.order}:${graph.size}`,
    [graph.order, graph.size]
  );

  const handleRef = useRef<SimulationHandle | null>(null);
  const [positions, setPositions] = useState<Map<string, { x: number; y: number }>>(
    () => new Map()
  );

  useEffect(() => {
    handleRef.current = createSimulation(graph, physics);
    setPositions(handleRef.current.positions());
    return () => {
      handleRef.current?.stop();
      handleRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graphKey]);

  useEffect(() => {
    handleRef.current?.reconfigure(physics);
  }, [physics]);

  const tick = (iterations: number = 1) => {
    handleRef.current?.tick(iterations);
    if (handleRef.current) {
      setPositions(new Map(handleRef.current.positions()));
    }
  };

  const setNodePosition = (nodeId: string, x: number, y: number) => {
    handleRef.current?.setNodePosition(nodeId, x, y);
    if (handleRef.current) {
      setPositions(new Map(handleRef.current.positions()));
    }
  };

  const pause = () => handleRef.current?.stop();
  const resume = () => handleRef.current?.start();

  return { positions, tick, setNodePosition, pause, resume };
}
