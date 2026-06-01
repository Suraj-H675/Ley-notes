import Graph from 'graphology';
import { bidirectional, dijkstra } from 'graphology-shortest-path';

export interface PathResult {
  path: string[] | null;
  length: number;
}

export function findShortestPath(
  graph: Graph,
  source: string,
  target: string
): PathResult {
  const path = bidirectional(graph, source, target);

  if (!path) {
    return { path: null, length: -1 };
  }

  let length = 0;
  for (let i = 0; i < path.length - 1; i++) {
    const edge = graph.edge(path[i], path[i + 1]);
    const weight = edge ? (graph.getEdgeAttribute(edge, 'weight') as number) || 1 : 1;
    length += weight;
  }

  return { path, length };
}

export function findPathWithDijkstra(
  graph: Graph,
  source: string,
  target: string
): PathResult {
  const path = dijkstra.bidirectional(graph, source, target);

  if (!path) {
    return { path: null, length: -1 };
  }

  let length = 0;
  for (let i = 0; i < path.length - 1; i++) {
    const edge = graph.edge(path[i], path[i + 1]);
    const weight = edge ? (graph.getEdgeAttribute(edge, 'weight') as number) || 1 : 1;
    length += weight;
  }

  return { path, length };
}

export function findAllPaths(
  graph: Graph,
  source: string,
  target: string,
  maxDepth = 5
): string[][] {
  const paths: string[][] = [];

  function dfs(current: string, visited: Set<string>, path: string[]): void {
    if (path.length > maxDepth) return;

    if (current === target) {
      paths.push([...path]);
      return;
    }

    graph.forEachOutNeighbor(current, (neighbor) => {
      if (!visited.has(neighbor)) {
        visited.add(neighbor);
        path.push(neighbor);
        dfs(neighbor, visited, path);
        path.pop();
        visited.delete(neighbor);
      }
    });
  }

  const visited = new Set<string>([source]);
  dfs(source, visited, [source]);

  return paths;
}

export function getDistance(
  graph: Graph,
  source: string,
  target: string
): number {
  const { length } = findShortestPath(graph, source, target);
  return length;
}

export function getConnectedComponents(
  graph: Graph
): string[][] {
  const nodes = graph.nodes();
  const visited = new Set<string>();
  const components: string[][] = [];

  function bfs(start: string): string[] {
    const component: string[] = [];
    const queue = [start];

    while (queue.length > 0) {
      const node = queue.shift()!;
      if (visited.has(node)) continue;

      visited.add(node);
      component.push(node);

      graph.forEachOutNeighbor(node, (neighbor) => {
        if (!visited.has(neighbor)) {
          queue.push(neighbor);
        }
      });
    }

    return component;
  }

  for (const node of nodes) {
    if (!visited.has(node)) {
      components.push(bfs(node));
    }
  }

  return components;
}
