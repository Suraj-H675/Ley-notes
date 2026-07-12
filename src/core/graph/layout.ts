/**
 * Synchronous force-directed layout using ForceAtlas1 (graphology-layout-force).
 *
 * IMPORTANT — units:
 *   The FA1 algorithm's force constants are TINY. The library's defaults are
 *   `attraction=0.0005, repulsion=0.1, gravity=0.0001`. Per-iteration movement
 *   is `force / distance`, so values much larger than the defaults will
 *   explode the graph (everything flies apart) or collapse it (everything
 *   converges to origin).
 *
 * To preserve Obsidian-style UI sliders (center force 0-1, repel 0-1000, link
 * 0-5), we accept those in `PhysicsSettings` and rescale internally:
 *
 *   FA1 gravity       = userCenter * 0.0006   (so userCenter=0.5 → 0.0003)
 *   FA1 repulsion     = userRepel  * 0.001    (so userRepel=200 → 0.2)
 *   FA1 attraction    = userLink   * 0.0008   (so userLink=1 → 0.0008)
 *   FA1 maxMove       = 30  (hard cap to prevent explosion regardless of params)
 *
 * With these mappings and maxMove=30, the layout converges in ~500 iterations
 * for graphs of 25-2000 nodes without exploding or collapsing.
 */

import forceLayout from 'graphology-layout-force';
import type Graph from 'graphology';

export interface PhysicsSettings {
  /** Obsidian "Center force" — 0..1. Higher = more compact. */
  centerForce: number;
  /** Obsidian "Repel force" — 0..1000. Higher = more spread out. */
  repelForce: number;
  /** Obsidian "Link force" — 0..5. Higher = edges pull nodes together. */
  linkForce: number;
  /** Max iterations to run. */
  iterations: number;
}

/**
 * Obsidian-friendly defaults. Rescaled internally to FA1's tiny units.
 */
export const DEFAULT_PHYSICS: PhysicsSettings = {
  centerForce: 0.1,
  repelForce: 250,
  linkForce: 1,
  iterations: 500,
};

/** Maximum per-iteration node movement — prevents explosion regardless of params. */
const MAX_MOVE = 30;

export interface LayoutOptions {
  width?: number;
  height?: number;
  physics?: Partial<PhysicsSettings>;
}

export function layoutGraph(
  graph: Graph,
  opts: LayoutOptions = {},
): Map<string, { x: number; y: number }> {
  const width = opts.width ?? 1200;
  const height = opts.height ?? 800;
  const physics = { ...DEFAULT_PHYSICS, ...opts.physics };

  // CRITICAL: graphology-layout-force reads each node's starting position from
  // its `x`/`y` attributes. If uninitialized, all positions start at (0,0)
  // and the repulsion/attraction forces are zero — the graph never spreads.
  // Seed random initial positions in a small circle so FA1 has something to
  // push apart. (FA2 initializes randomly internally; FA1 doesn't.)
  seedInitialPositions(graph);

  // Rescale Obsidian-friendly slider values to FA1's tiny units.
  const fa1Settings = {
    gravity: physics.centerForce * 0.0006,
    repulsion: physics.repelForce * 0.001,
    attraction: physics.linkForce * 0.0008,
    inertia: 0.6,
    maxMove: MAX_MOVE,
  };

  const positions = forceLayout(graph, {
    maxIterations: physics.iterations,
    settings: fa1Settings,
  });

  // Scale positions to fit the viewport.
  let minX = Infinity,
    maxX = -Infinity,
    minY = Infinity,
    maxY = -Infinity;
  for (const id of graph.nodes()) {
    const p = positions[id];
    if (!p) continue;
    if (p.x < minX) minX = p.x;
    if (p.x > maxX) maxX = p.x;
    if (p.y < minY) minY = p.y;
    if (p.y > maxY) maxY = p.y;
  }

  // Empty graph: return empty map so the UI shows the "no nodes" message.
  if (!Number.isFinite(minX)) {
    return new Map();
  }

  const dx = Math.max(maxX - minX, 1);
  const dy = Math.max(maxY - minY, 1);
  const scale = Math.min(width / dx, height / dy) * 0.85;
  const offsetX = (width - dx * scale) / 2 - minX * scale;
  const offsetY = (height - dy * scale) / 2 - minY * scale;

  const out = new Map<string, { x: number; y: number }>();
  for (const id of graph.nodes()) {
    const p = positions[id];
    if (!p) continue;
    out.set(id, { x: p.x * scale + offsetX, y: p.y * scale + offsetY });
  }
  return out;
}

/**
 * Seed nodes deterministically on a sunflower spiral. Force layouts are
 * sensitive to their initial state; deterministic seeds prevent the graph
 * from jumping between opens and keep tests reproducible.
 */
function seedInitialPositions(graph: Graph, r = 50): void {
  const nodes = graph.nodes().sort();
  const goldenAngle = Math.PI * (3 - Math.sqrt(5));
  for (const [index, id] of nodes.entries()) {
    const angle = index * goldenAngle;
    const radius = Math.sqrt((index + 1) / Math.max(1, nodes.length)) * r;
    const x = radius * Math.cos(angle);
    const y = radius * Math.sin(angle);
    graph.setNodeAttribute(id, 'x', x);
    graph.setNodeAttribute(id, 'y', y);
  }
}
