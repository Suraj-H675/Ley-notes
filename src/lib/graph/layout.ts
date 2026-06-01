import Graph from 'graphology';
import forceAtlas2, { type ForceAtlas2Settings } from 'graphology-layout-forceatlas2';

export interface LayoutOptions {
  iterations?: number;
  settings?: ForceAtlas2Settings;
}

const DEFAULT_SETTINGS: ForceAtlas2Settings = {
  gravity: 1,
  scalingRatio: 10,
  strongGravityMode: true,
  barnesHutOptimize: true,
  barnesHutTheta: 0.5,
};

export function applyForceLayout(
  graph: Graph,
  options: LayoutOptions = {}
): Map<string, { x: number; y: number }> {
  const { iterations = 100, settings = {} } = options;

  const mergedSettings = {
    ...DEFAULT_SETTINGS,
    ...settings,
  };

  const layout = forceAtlas2(graph, {
    iterations,
    settings: mergedSettings,
  });

  return new Map(Object.entries(layout));
}

export function circularLayout(
  graph: Graph
): Map<string, { x: number; y: number }> {
  const nodes = graph.nodes();
  const positions = new Map<string, { x: number; y: number }>();
  const centerX = 0;
  const centerY = 0;
  const radius = Math.max(100, nodes.length * 20);

  nodes.forEach((node, index) => {
    const angle = (2 * Math.PI * index) / nodes.length;
    positions.set(node, {
      x: centerX + radius * Math.cos(angle),
      y: centerY + radius * Math.sin(angle),
    });
  });

  return positions;
}

export function randomLayout(
  graph: Graph,
  width = 1000,
  height = 1000
): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();

  graph.nodes().forEach((node) => {
    positions.set(node, {
      x: Math.random() * width - width / 2,
      y: Math.random() * height - height / 2,
    });
  });

  return positions;
}

export function assignPositionsToGraph(
  graph: Graph,
  positions: Map<string, { x: number; y: number }>
): void {
  graph.forEachNode((node) => {
    const pos = positions.get(node);
    if (pos) {
      graph.setNodeAttribute(node, 'x', pos.x);
      graph.setNodeAttribute(node, 'y', pos.y);
    }
  });
}
