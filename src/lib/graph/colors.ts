import { tagColor, collectionColor } from './tagColor';
import type { ColorScheme } from '@/types/graph-settings.types';
import type { KnowledgeNode } from '@/types';

export const UNCOLORED = 'hsl(220 8% 55%)';

export const COMMUNITY_PALETTE: string[] = [
  'hsl(0 65% 62%)',
  'hsl(30 65% 60%)',
  'hsl(60 60% 60%)',
  'hsl(140 55% 58%)',
  'hsl(180 55% 58%)',
  'hsl(220 65% 65%)',
  'hsl(265 55% 65%)',
  'hsl(320 55% 65%)',
];

export interface ColorContext {
  degree: number;
  maxDegree: number;
  community: number;
}

export function colorForNode(
  node: KnowledgeNode,
  scheme: ColorScheme,
  ctx: ColorContext
): string {
  switch (scheme) {
    case 'untyped':
      return UNCOLORED;
    case 'tag':
      if (node.tags.length === 0) return UNCOLORED;
      return tagColor(node.tags[0]);
    case 'collection':
      if (node.collections.length === 0) return UNCOLORED;
      return collectionColor(node.collections[0]);
    case 'link-count':
      return linkCountColor(ctx.degree, ctx.maxDegree);
    case 'community':
      return COMMUNITY_PALETTE[ctx.community % COMMUNITY_PALETTE.length];
  }
}

export function linkCountColor(degree: number, maxDegree: number): string {
  if (maxDegree <= 0) return 'hsl(220 8% 50%)';
  const t = Math.min(1, degree / maxDegree);
  const h = 220 + t * 45;
  const s = 8 + t * 42;
  const l = 50 + t * 20;
  return `hsl(${h} ${s}% ${l}%)`;
}
