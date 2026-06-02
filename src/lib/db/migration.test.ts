import { describe, it, expect } from 'vitest';
import { migrateV2NodeToV3 } from './index';
import type { JSONContent } from '@tiptap/react';

describe('migrateV2NodeToV3', () => {
  it('converts JSONContent content to a Markdown string', () => {
    const json: JSONContent = {
      type: 'doc',
      content: [
        { type: 'heading', attrs: { level: 1 }, content: [{ type: 'text', text: 'Hi' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'world' }] },
      ],
    };
    const migrated = migrateV2NodeToV3({
      id: 'n1',
      content: json,
      plainText: 'stale text',
    } as any);
    expect(typeof migrated.content).toBe('string');
    expect(migrated.content).toContain('# Hi');
    expect(migrated.content).toContain('world');
  });

  it('recomputes plainText from the new Markdown', () => {
    const migrated = migrateV2NodeToV3({
      id: 'n1',
      content: {
        type: 'doc',
        content: [
          { type: 'paragraph', content: [{ type: 'text', text: 'fresh text' }] },
        ],
      },
      plainText: 'stale',
    } as any);
    expect(migrated.plainText).toContain('fresh text');
  });

  it('handles null content by setting it to empty string', () => {
    const migrated = migrateV2NodeToV3({
      id: 'n1',
      content: null,
      plainText: '',
    } as any);
    expect(migrated.content).toBe('');
  });

  it('passes through string content without re-converting', () => {
    const migrated = migrateV2NodeToV3({
      id: 'n1',
      content: '# Already markdown',
      plainText: 'stale',
    } as any);
    expect(migrated.content).toBe('# Already markdown');
    expect(migrated.plainText).toBe('Already markdown');
  });

  it('preserves all other fields', () => {
    const migrated = migrateV2NodeToV3({
      id: 'n1',
      title: 'My title',
      type: 'document',
      content: { type: 'doc', content: [] },
      plainText: '',
      collections: ['a'],
      tags: ['b'],
    } as any);
    expect(migrated.id).toBe('n1');
    expect(migrated.title).toBe('My title');
    expect(migrated.type).toBe('document');
    expect(migrated.collections).toEqual(['a']);
    expect(migrated.tags).toEqual(['b']);
  });
});
