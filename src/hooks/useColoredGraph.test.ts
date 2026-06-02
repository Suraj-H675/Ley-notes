import { describe, it, expect } from 'vitest';
import { colorMapForGraph } from './useColoredGraph';
import type { KnowledgeNode } from '@/types';
import Graph from 'graphology';
import { detectCommunities } from '@/lib/graph/louvain';

const mkNode = (id: string, overrides: Partial<KnowledgeNode> = {}): KnowledgeNode => ({
  id,
  type: 'document',
  title: id,
  content: null,
  plainText: '',
  collections: [],
  tags: [],
  properties: {},
  isArchived: 0,
  createdAt: 0,
  updatedAt: 0,
  ...overrides,
});

describe('colorMapForGraph', () => {
  it('returns a color for each node in untyped scheme', () => {
    const nodes: KnowledgeNode[] = [mkNode('a'), mkNode('b')];
    const g = new Graph({ type: 'undirected', multi: false });
    g.addNode('a');
    g.addNode('b');
    const colors = colorMapForGraph(nodes, [], g, 'untyped');
    expect(colors.size).toBe(2);
    expect(colors.get('a')).toMatch(/^hsl\(/);
  });

  it('uses tag color for tag scheme', () => {
    const nodes: KnowledgeNode[] = [mkNode('a', { tags: ['react'] })];
    const g = new Graph({ type: 'undirected', multi: false });
    g.addNode('a');
    const colors = colorMapForGraph(nodes, [], g, 'tag');
    expect(colors.get('a')).toMatch(/^hsl\(/);
  });

  it('uses community palette for community scheme when communities provided', () => {
    const nodes: KnowledgeNode[] = [mkNode('a'), mkNode('b'), mkNode('c')];
    const g = new Graph({ type: 'undirected', multi: false });
    g.addNode('a');
    g.addNode('b');
    g.addNode('c');
    g.addEdge('a', 'b');
    g.addEdge('b', 'c');
    const communities = detectCommunities(g);
    const colors = colorMapForGraph(
      nodes,
      [],
      g,
      'community',
      communities ?? undefined
    );
    expect(colors.get('a')).toMatch(/^hsl\(/);
  });
});
