import { describe, it, expect } from 'vitest';
import { extractPlainText } from './extract-plaintext';

describe('extractPlainText', () => {
  it('returns empty string for empty input', () => {
    expect(extractPlainText('')).toBe('');
  });

  it('strips bold and italic markers', () => {
    expect(extractPlainText('**bold** and *italic*')).toBe('bold and italic');
  });

  it('strips inline code backticks', () => {
    expect(extractPlainText('use `npm install` to start')).toBe(
      'use npm install to start'
    );
  });

  it('strips link syntax but keeps the link text', () => {
    expect(extractPlainText('see [the docs](https://example.com)')).toBe(
      'see the docs'
    );
  });

  it('strips wikilink brackets', () => {
    expect(extractPlainText('related: [[Other Note]] and more')).toBe(
      'related: Other Note and more'
    );
  });

  it('converts headings to plain text', () => {
    expect(extractPlainText('# Title\n\nbody text.')).toBe('Title\n\nbody text.');
  });

  it('converts list items to plain text', () => {
    const md = '- one\n- two\n- three\n';
    expect(extractPlainText(md)).toBe('one\ntwo\nthree');
  });

  it('removes code block fences but keeps code content', () => {
    const md = '```js\nconst x = 1;\n```\n';
    expect(extractPlainText(md)).toBe('const x = 1;');
  });

  it('separates top-level blocks with a blank line', () => {
    const md = 'a\n\nb\n';
    expect(extractPlainText(md)).toBe('a\n\nb');
  });

  it('trims leading and trailing whitespace', () => {
    expect(extractPlainText('   \n\n  hello  \n\n   ')).toBe('hello');
  });

  it('keeps callout text as-is (callout-marker stripping is added in 8e)', () => {
    // Plain text extraction is structural; callout marker stripping lives
    // in the callout feature (Phase 8e). For now, the paragraph text passes through.
    const md = '> [!note] Title\n>\n> Body line\n';
    const out = extractPlainText(md);
    expect(out).toContain('Title');
    expect(out).toContain('Body line');
  });
});
