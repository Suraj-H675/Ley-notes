import { describe, it, expect } from 'vitest';
import {
  colorForNode,
  COMMUNITY_PALETTE,
  UNCOLORED,
  linkCountColor,
} from './colors';
import type { KnowledgeNode } from '@/types';

const baseNode: KnowledgeNode = {
  id: '1',
  type: 'document',
  title: 'Test',
  content: null,
  plainText: '',
  collections: [],
  tags: [],
  properties: {},
  isArchived: 0,
  createdAt: 0,
  updatedAt: 0,
};

describe('colorForNode', () => {
  it('returns UNCOLORED when scheme is untyped', () => {
    expect(
      colorForNode(baseNode, 'untyped', { degree: 0, maxDegree: 10, community: 0 })
    ).toBe(UNCOLORED);
  });

  it('uses tag color when scheme is tag and node has tags', () => {
    const node = { ...baseNode, tags: ['react'] };
    const c = colorForNode(node, 'tag', { degree: 0, maxDegree: 0, community: 0 });
    expect(c).toMatch(/^hsl\(/);
  });

  it('falls back to UNCOLORED when scheme is tag but node has no tags', () => {
    const c = colorForNode(baseNode, 'tag', { degree: 0, maxDegree: 0, community: 0 });
    expect(c).toBe(UNCOLORED);
  });

  it('uses collection color when scheme is collection and node has collections', () => {
    const node = { ...baseNode, collections: ['work'] };
    const c = colorForNode(node, 'collection', { degree: 0, maxDegree: 0, community: 0 });
    expect(c).toMatch(/^hsl\(/);
  });

  it('uses COMMUNITY_PALETTE for community scheme', () => {
    const c = colorForNode(baseNode, 'community', {
      degree: 0,
      maxDegree: 0,
      community: 2,
    });
    expect(c).toBe(COMMUNITY_PALETTE[2 % COMMUNITY_PALETTE.length]);
  });
});

describe('linkCountColor', () => {
  it('returns base color when degree is 0', () => {
    const c = linkCountColor(0, 10);
    expect(c).toMatch(/^hsl\(/);
  });

  it('returns deeper color at max degree', () => {
    const c = linkCountColor(10, 10);
    expect(c).toMatch(/^hsl\(/);
  });

  it('handles maxDegree=0', () => {
    const c = linkCountColor(5, 0);
    expect(c).toMatch(/^hsl\(/);
  });
});
