import Graph from 'graphology';
import {
  forceCenter,
  forceLink,
  forceManyBody,
  forceSimulation,
  type Simulation,
  type SimulationNodeDatum,
} from 'd3-force';
import type { PhysicsConfig } from '@/types/graph-settings.types';

interface SimNode extends SimulationNodeDatum {
  id: string;
}

interface SimLink {
  source: string | SimNode;
  target: string | SimNode;
}

export interface SimulationHandle {
  start(): void;
  stop(): void;
  tick(iterations: number): void;
  positions(): Map<string, { x: number; y: number }>;
  reconfigure(physics: PhysicsConfig): void;
}

export function createSimulation(
  graph: Graph,
  physics: PhysicsConfig
): SimulationHandle {
  graph.forEachNode((node) => {
    const x = graph.getNodeAttribute(node, 'x');
    const y = graph.getNodeAttribute(node, 'y');
    if (typeof x !== 'number' || typeof y !== 'number' || isNaN(x) || isNaN(y)) {
      graph.setNodeAttribute(node, 'x', Math.random() * 400 - 200);
      graph.setNodeAttribute(node, 'y', Math.random() * 400 - 200);
    }
  });

  const nodes: SimNode[] = graph.nodes().map((id) => ({
    id,
    x: graph.getNodeAttribute(id, 'x') as number,
    y: graph.getNodeAttribute(id, 'y') as number,
  }));

  const idToNode = new Map(nodes.map((n) => [n.id, n]));

  const links: SimLink[] = graph.edges().map((e) => ({
    source: idToNode.get(graph.source(e))!,
    target: idToNode.get(graph.target(e))!,
  }));

  const sim: Simulation<SimNode, SimLink> = forceSimulation<SimNode>(nodes)
    .force('charge', forceManyBody<SimNode>().strength(physics.chargeForce))
    .force(
      'link',
      forceLink<SimNode, SimLink>(links)
        .id((d) => (d as SimNode).id)
        .distance(physics.linkDistance)
        .strength(physics.linkForce)
    )
    .force('center', forceCenter(0, 0).strength(physics.centerForce))
    .alphaDecay(0.05)
    .stop();

  function applyToGraph() {
    for (const n of nodes) {
      if (
        typeof n.x === 'number' &&
        typeof n.y === 'number' &&
        !isNaN(n.x) &&
        !isNaN(n.y)
      ) {
        graph.setNodeAttribute(n.id, 'x', n.x);
        graph.setNodeAttribute(n.id, 'y', n.y);
      }
    }
  }

  return {
    start() {
      sim.restart();
    },
    stop() {
      sim.stop();
    },
    tick(iterations: number) {
      for (let i = 0; i < iterations; i++) sim.tick();
      applyToGraph();
    },
    positions() {
      const map = new Map<string, { x: number; y: number }>();
      for (const n of nodes) {
        if (typeof n.x === 'number' && typeof n.y === 'number') {
          map.set(n.id, { x: n.x, y: n.y });
        }
      }
      return map;
    },
    reconfigure(next: PhysicsConfig) {
      sim
        .force('charge', forceManyBody<SimNode>().strength(next.chargeForce))
        .force(
          'link',
          forceLink<SimNode, SimLink>(links)
            .id((d) => (d as SimNode).id)
            .distance(next.linkDistance)
            .strength(next.linkForce)
        )
        .force('center', forceCenter(0, 0).strength(next.centerForce))
        .alpha(0.5)
        .restart();
    },
  };
}
