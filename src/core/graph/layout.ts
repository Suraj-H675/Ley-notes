/**
 * Synchronous force-directed layout using ForceAtlas1 (graphology-layout-force).
 *
 * FA1 exposes the exact parameters Obsidian's graph sliders use:
 *   - gravity       — center force (pulls everything toward origin)
 *   - repulsion     — how strongly nodes push each other apart
 *   - attraction    — link force (rubber-band tightness)
 *   - maxMove       — distance cap per iteration (stability)
 *
 * For graphs > ~2000 nodes, layout runs on the main thread but typically
 * completes in <1s on modern hardware. Web-worker offload is a v2 concern.
 */

import forceLayout from 'graphology-layout-force';
import type Graph from 'graphology';

export interface PhysicsSettings {
  /** Obsidian "Center force" — pulls nodes toward origin. Higher = more compact. */
  centerForce: number;
  /** Obsidian "Repel force" — node-node repulsion strength. */
  repelForce: number;
  /** Obsidian "Link force" — edge attraction strength (rubber-band tightness). */
  linkForce: number;
  /** Max iterations to run. */
  iterations: number;
}

export const DEFAULT_PHYSICS: PhysicsSettings = {
  centerForce: 0.05,
  repelForce: 200,
  linkForce: 1,
  iterations: 300,
};

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

  const positions = forceLayout(graph, {
    maxIterations: physics.iterations,
    settings: {
      gravity: physics.centerForce,
      repulsion: physics.repelForce,
      attraction: physics.linkForce,
      // maxMove not directly exposed; rely on iterations + damping-by-default.
    },
  });

  // Scale positions to fit the viewport with padding.
  let minX = Infinity,
    maxX = -Infinity,
    minY = Infinity,
    maxY = -Infinity;
  for (const [, p] of Object.entries(positions)) {
    if (p.x < minX) minX = p.x;
    if (p.x > maxX) maxX = p.x;
    if (p.y < minY) minY = p.y;
    if (p.y > maxY) maxY = p.y;
  }
  const dx = maxX - minX || 1;
  const dy = maxY - minY || 1;
  const scale = Math.min(width / dx, height / dy) * 0.85;
  const offsetX = (width - dx * scale) / 2 - minX * scale;
  const offsetY = (height - dy * scale) / 2 - minY * scale;

  const out = new Map<string, { x: number; y: number }>();
  for (const [id, p] of Object.entries(positions)) {
    out.set(id, { x: p.x * scale + offsetX, y: p.y * scale + offsetY });
  }
  return out;
}