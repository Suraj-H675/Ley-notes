import { describe, it, expect, beforeEach } from 'vitest';
import 'fake-indexeddb/auto';
import { db } from '@/lib/db';
import { applyFilters } from './useFilteredGraph';
import type { KnowledgeNode, KnowledgeEdge } from '@/types';

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

const mkEdge = (id: string, source: string, target: string): KnowledgeEdge => ({
  id,
  source,
  target,
  type: 'wiki-link',
  createdAt: 0,
});

beforeEach(async () => {
  await db.nodes.clear();
  await db.edges.clear();
  await db.nodes.bulkPut([
    mkNode('a', { title: 'React patterns', tags: ['react'] }),
    mkNode('b', { title: 'Vue basics', tags: ['vue'] }),
    mkNode('c', { title: 'Standalone note' }),
    mkNode('orphan', { title: 'Unlinked' }),
  ]);
  await db.edges.bulkPut([mkEdge('e1', 'a', 'b')]);
});

describe('applyFilters', () => {
  it('search query filters by title case-insensitive', async () => {
    const nodes = await db.nodes.toArray();
    const edges = await db.edges.toArray();
    const filtered = applyFilters({
      nodes,
      edges,
      filters: {
        searchQuery: 'react',
        selectedTags: [],
        selectedCollections: [],
        showOrphans: true,
      },
    });
    expect(filtered.nodes.map((n) => n.id).sort()).toEqual(['a']);
  });

  it('selectedTags filters to nodes having at least one matching tag', async () => {
    const nodes = await db.nodes.toArray();
    const edges = await db.edges.toArray();
    const filtered = applyFilters({
      nodes,
      edges,
      filters: {
        searchQuery: '',
        selectedTags: ['react'],
        selectedCollections: [],
        showOrphans: true,
      },
    });
    expect(filtered.nodes.map((n) => n.id)).toEqual(['a']);
  });

  it('selectedCollections filters to nodes in one of the collections', async () => {
    const nodes = await db.nodes.toArray();
    const edges = await db.edges.toArray();
    const filtered = applyFilters({
      nodes,
      edges,
      filters: {
        searchQuery: '',
        selectedTags: [],
        selectedCollections: ['work'],
        showOrphans: true,
      },
    });
    expect(filtered.nodes).toEqual([]);
  });

  it('showOrphans=false removes nodes with no edges', async () => {
    const nodes = await db.nodes.toArray();
    const edges = await db.edges.toArray();
    const filtered = applyFilters({
      nodes,
      edges,
      filters: {
        searchQuery: '',
        selectedTags: [],
        selectedCollections: [],
        showOrphans: false,
      },
    });
    const ids = filtered.nodes.map((n) => n.id).sort();
    expect(ids).toEqual(['a', 'b']);
  });

  it('only keeps edges where both endpoints pass the filter', async () => {
    const nodes = await db.nodes.toArray();
    const edges = await db.edges.toArray();
    const filtered = applyFilters({
      nodes,
      edges,
      filters: {
        searchQuery: 'react',
        selectedTags: [],
        selectedCollections: [],
        showOrphans: true,
      },
    });
    expect(filtered.edges).toEqual([]);
  });
});
