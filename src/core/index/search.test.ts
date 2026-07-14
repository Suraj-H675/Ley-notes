import { describe, expect, it } from 'vitest';
import { matchesSearchFilters, parseSearchQuery } from './search';

describe('structured vault search', () => {
  it('parses quoted, repeated, negative, and property filters', () => {
    expect(parseSearchQuery('design system tag:#work tag:active -tag:archive path:"Projects Alpha" title:roadmap property:status=In-Progress [owner:Suraj] -[draft]')).toEqual({
      terms: 'design system',
      tags: [
        { value: 'work', exclude: false },
        { value: 'active', exclude: false },
        { value: 'archive', exclude: true },
      ],
      paths: [{ value: 'projects alpha', exclude: false }],
      titles: [{ value: 'roadmap', exclude: false }],
      properties: [
        { key: 'status', value: 'in-progress', exclude: false },
        { key: 'owner', value: 'suraj', exclude: false },
        { key: 'draft', value: undefined, exclude: true },
      ],
    });
  });

  it('applies nested tags, substring paths, titles, properties, and exclusions together', () => {
    const doc = { id: 'page', title: 'Product Roadmap', path: 'Projects Alpha/roadmap.md', tags: 'work/research active' };
    const properties = new Map([
      ['status', ['in-progress']],
      ['owner', ['suraj', 'team']],
    ]);
    expect(matchesSearchFilters(doc, parseSearchQuery('tag:work -tag:archive path:alpha title:road property:status=progress [owner:suraj] -[draft]'), properties)).toBe(true);
    expect(matchesSearchFilters(doc, parseSearchQuery('tag:work tag:archive'), properties)).toBe(false);
    expect(matchesSearchFilters(doc, parseSearchQuery('-property:status=progress'), properties)).toBe(false);
  });

  it('keeps unknown operators as searchable text instead of silently discarding them', () => {
    expect(parseSearchQuery('meeting before:2026-01-01').terms).toBe('meeting before:2026-01-01');
  });
});
