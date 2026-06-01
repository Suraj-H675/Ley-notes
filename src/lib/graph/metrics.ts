import Graph from 'graphology';

export interface GraphMetrics {
  nodeCount: number;
  edgeCount: number;
  avgDegree: number;
  density: number;
  diameter: number;
  degreeCentrality: Map<string, number>;
  betweennessCentrality: Map<string, number>;
  orphanNodes: string[];
  hubNodes: string[];
  leafNodes: string[];
}

export function calculateGraphMetrics(graph: Graph): GraphMetrics {
  const nodeCount = graph.order;
  const edgeCount = graph.size;

  const avgDegree = nodeCount > 0 ? (2 * edgeCount) / nodeCount : 0;

  const maxPossibleEdges = nodeCount * (nodeCount - 1);
  const density = maxPossibleEdges > 0 ? edgeCount / maxPossibleEdges : 0;

  const degreeCentrality = calculateDegreeCentrality(graph);
  const betweennessCentrality = calculateBetweennessCentrality(graph);

  const orphanNodes: string[] = [];
  const hubNodes: string[] = [];
  const leafNodes: string[] = [];

  graph.forEachNode((node) => {
    const degree = graph.degree(node);
    if (degree === 0) {
      orphanNodes.push(node);
    } else if (degree >= 10) {
      hubNodes.push(node);
    } else if (degree === 1) {
      leafNodes.push(node);
    }
  });

  const diameter = calculateDiameter(graph);

  return {
    nodeCount,
    edgeCount,
    avgDegree,
    density,
    diameter,
    degreeCentrality,
    betweennessCentrality,
    orphanNodes,
    hubNodes,
    leafNodes,
  };
}

function calculateDegreeCentrality(graph: Graph): Map<string, number> {
  const n = graph.order;
  const centrality = new Map<string, number>();

  if (n <= 1) {
    graph.forEachNode((node) => centrality.set(node, 0));
    return centrality;
  }

  graph.forEachNode((node) => {
    const degree = graph.degree(node);
    centrality.set(node, degree / (n - 1));
  });

  return centrality;
}

function calculateBetweennessCentrality(graph: Graph): Map<string, number> {
  const nodes = graph.nodes();
  const centrality = new Map<string, number>();

  nodes.forEach((s) => {
    centrality.set(s, 0);
  });

  nodes.forEach((s) => {
    const stack: string[] = [];
    const pred = new Map<string, string[]>();
    const sigma = new Map<string, number>();
    const dist = new Map<string, number>();

    nodes.forEach((v) => {
      pred.set(v, []);
      sigma.set(v, 0);
      dist.set(v, -1);
    });

    sigma.set(s, 1);
    dist.set(s, 0);

    const queue: string[] = [s];

    while (queue.length > 0) {
      const v = queue.shift()!;
      stack.push(v);

      graph.forEachOutNeighbor(v, (w) => {
        if (dist.get(w)! < 0) {
          queue.push(w);
          dist.set(w, dist.get(v)! + 1);
        }

        if (dist.get(w) === dist.get(v)! + 1) {
          sigma.set(w, sigma.get(w)! + sigma.get(v)!);
          pred.get(w)!.push(v);
        }
      });
    }

    const delta = new Map<string, number>();
    nodes.forEach((v) => delta.set(v, 0));

    while (stack.length > 0) {
      const w = stack.pop()!;
      pred.get(w)!.forEach((v) => {
        delta.set(v, delta.get(v)! + (sigma.get(v)! / sigma.get(w)!) * (1 + delta.get(w)!));
      });

      if (w !== s) {
        centrality.set(w, centrality.get(w)! + delta.get(w)!);
      }
    }
  });

  const n = graph.order;
  if (n > 2) {
    const normalization = 2 / ((n - 1) * (n - 2));
    centrality.forEach((value, node) => {
      centrality.set(node, value * normalization);
    });
  }

  return centrality;
}

function calculateDiameter(graph: Graph): number {
  const nodes = graph.nodes();
  if (nodes.length === 0) return 0;
  if (nodes.length === 1) return 0;

  let maxDist = 0;

  function bfs(start: string): Map<string, number> {
    const distances = new Map<string, number>();
    const queue: string[] = [start];
    distances.set(start, 0);

    while (queue.length > 0) {
      const current = queue.shift()!;
      const currentDist = distances.get(current)!;

      graph.forEachOutNeighbor(current, (neighbor) => {
        if (!distances.has(neighbor)) {
          distances.set(neighbor, currentDist + 1);
          queue.push(neighbor);
        }
      });
    }

    return distances;
  }

  for (const node of nodes) {
    const distances = bfs(node);
    distances.forEach((d) => {
      if (d > maxDist) maxDist = d;
    });
  }

  return maxDist;
}

export function getDegreeDistribution(graph: Graph): Map<number, number> {
  const distribution = new Map<number, number>();

  graph.forEachNode((node) => {
    const degree = graph.degree(node);
    distribution.set(degree, (distribution.get(degree) || 0) + 1);
  });

  return distribution;
}
