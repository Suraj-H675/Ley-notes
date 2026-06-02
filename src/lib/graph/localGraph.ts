import Graph from 'graphology';

export function nHopSubgraph(
  source: Graph,
  centerNode: string,
  depth: number
): Graph {
  const sub = new Graph({ type: source.type, multi: false, allowSelfLoops: false });
  if (!source.hasNode(centerNode)) return sub;

  const visited = new Set<string>([centerNode]);
  let frontier: string[] = [centerNode];

  for (let d = 0; d < depth; d++) {
    const next: string[] = [];
    for (const n of frontier) {
      source.forEachNeighbor(n, (m) => {
        if (!visited.has(m)) {
          visited.add(m);
          next.push(m);
        }
      });
    }
    frontier = next;
    if (frontier.length === 0) break;
  }

  for (const id of visited) {
    const attrs = source.getNodeAttributes(id);
    sub.addNode(id, { ...attrs });
  }

  source.forEachEdge((edge, attrs, s, t) => {
    if (visited.has(s) && visited.has(t)) {
      sub.addEdgeWithKey(edge, s, t, { ...attrs });
    }
  });

  return sub;
}
