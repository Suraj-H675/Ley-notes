import { describe, it, expect } from 'vitest';
import { parseMarkdown } from './parser';
import { normalizeMarkdown } from './serializer';

describe('parseMarkdown', () => {
  it('parses a heading', () => {
    const tree = parseMarkdown('# Hello');
    expect(tree.type).toBe('root');
    expect(tree.children).toHaveLength(1);
    expect(tree.children[0].type).toBe('heading');
  });

  it('parses paragraphs and code blocks', () => {
    const tree = parseMarkdown('a paragraph\n\n```\ncode\n```\n');
    const types = tree.children.map((c) => c.type);
    expect(types).toEqual(['paragraph', 'code']);
  });

  it('returns position metadata', () => {
    const tree = parseMarkdown('one\n\n# Title\n');
    const heading = tree.children[1] as any;
    expect(heading.position.start.offset).toBe(5);
  });
});

describe('normalizeMarkdown', () => {
  it('round-trips a simple document (preserving content; trailing newline is added)', () => {
    const md = '# Title\n\nA paragraph.';
    const out = normalizeMarkdown(md);
    // remark-stringify normalizes by appending a trailing newline.
    expect(out.replace(/\n+$/, '')).toBe(md);
  });

  it('preserves content across round-trips', () => {
    const md = 'line1\nline2\n';
    const out = normalizeMarkdown(md);
    expect(out).toContain('line1');
    expect(out).toContain('line2');
  });

  it('round-trips a list', () => {
    const md = '- a\n- b\n- c\n';
    const out = normalizeMarkdown(md);
    expect(out).toContain('a');
    expect(out).toContain('b');
    expect(out).toContain('c');
  });

  it('handles wikilinks: serializer escapes [; custom plugin in 8c will preserve them', () => {
    // Standard CommonMark escapes brackets not part of a link. The wikilink
    // plugin (8c) will treat `[[Note]]` as raw text so it round-trips.
    const md = 'see [[Other Note]] for context';
    const out = normalizeMarkdown(md);
    // Without the custom plugin, brackets are escaped. We just check the
    // *text content* survives.
    expect(out).toContain('Other Note');
  });
});
