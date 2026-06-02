import { describe, it, expect } from 'vitest';
import { parseInlineRanges } from './inline-ranges';

describe('parseInlineRanges', () => {
  it('returns an empty array for plain text', () => {
    expect(parseInlineRanges('just text')).toEqual([]);
  });

  it('detects bold (**text**)', () => {
    const md = 'this is **bold** text';
    const ranges = parseInlineRanges(md);
    expect(ranges).toHaveLength(1);
    expect(ranges[0]).toMatchObject({
      from: md.indexOf('**bold**'),
      to: md.indexOf('**bold**') + '**bold**'.length,
      kind: 'strong',
      inner: { from: 10, to: 14, text: 'bold' },
    });
  });

  it('detects italic (*text*)', () => {
    const md = 'an *italic* word';
    const ranges = parseInlineRanges(md);
    expect(ranges).toHaveLength(1);
    expect(ranges[0].kind).toBe('em');
    expect(ranges[0].inner.from).toBe(4);
    expect(ranges[0].inner.to).toBe(10);
  });

  it('detects inline code (`text`)', () => {
    const md = 'use `npm install` here';
    const ranges = parseInlineRanges(md);
    expect(ranges).toHaveLength(1);
    expect(ranges[0].kind).toBe('code');
  });

  it('detects strikethrough (~~text~~)', () => {
    const md = 'old ~~deleted~~ new';
    const ranges = parseInlineRanges(md);
    expect(ranges).toHaveLength(1);
    expect(ranges[0].kind).toBe('strike');
  });

  it('detects a link [text](url)', () => {
    const md = 'see [docs](https://example.com)';
    const ranges = parseInlineRanges(md);
    expect(ranges).toHaveLength(1);
    expect(ranges[0].kind).toBe('link');
    expect(ranges[0].href).toBe('https://example.com');
    expect(ranges[0].inner.text).toBe('docs');
  });

  it('detects a wikilink [[Note Title]]', () => {
    const md = 'see [[Other Note]] for context';
    const ranges = parseInlineRanges(md);
    expect(ranges).toHaveLength(1);
    expect(ranges[0].kind).toBe('wikilink');
    expect(ranges[0].href).toBe('Other Note');
    expect(ranges[0].inner.text).toBe('Other Note');
  });

  it('detects multiple inline formats in one string', () => {
    const md = '**bold** and *italic* and `code`';
    const ranges = parseInlineRanges(md);
    const kinds = ranges.map((r) => r.kind);
    expect(kinds).toContain('strong');
    expect(kinds).toContain('em');
    expect(kinds).toContain('code');
  });

  it('handles nested bold inside italic', () => {
    const md = '*italic with **bold** inside*';
    const ranges = parseInlineRanges(md);
    const kinds = ranges.map((r) => r.kind).sort();
    expect(kinds).toContain('strong');
    expect(kinds).toContain('em');
  });

  it('does not match unclosed formatting', () => {
    const md = 'this **is not bold';
    const ranges = parseInlineRanges(md);
    expect(ranges.find((r) => r.kind === 'strong')).toBeUndefined();
  });

  it('skips ranges inside code blocks', () => {
    const md = '`code` and **bold**';
    const ranges = parseInlineRanges(md);
    const kinds = ranges.map((r) => r.kind);
    expect(kinds).toContain('code');
    expect(kinds).toContain('strong');
  });
});
