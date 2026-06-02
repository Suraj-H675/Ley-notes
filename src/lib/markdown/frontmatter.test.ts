import { describe, it, expect } from 'vitest';
import {
  parseFrontmatter,
  stringifyFrontmatter,
  splitFrontmatter,
} from './frontmatter';

describe('splitFrontmatter', () => {
  it('returns body unchanged when no frontmatter present', () => {
    const md = 'just a body\nwith text\n';
    expect(splitFrontmatter(md)).toEqual({ frontmatter: null, body: md });
  });

  it('extracts frontmatter from the top of the document', () => {
    const md = '---\ntitle: Hello\n---\nbody\n';
    const { frontmatter, body } = splitFrontmatter(md);
    expect(frontmatter).toBe('title: Hello\n');
    expect(body).toBe('body\n');
  });

  it('handles Windows line endings', () => {
    const md = '---\r\ntitle: Hello\r\n---\r\nbody\r\n';
    const { frontmatter, body } = splitFrontmatter(md);
    expect(frontmatter).toBe('title: Hello\r\n');
    expect(body).toBe('body\r\n');
  });

  it('does not treat --- in the middle of the body as frontmatter', () => {
    const md = 'body\n---\nstill body\n';
    expect(splitFrontmatter(md)).toEqual({ frontmatter: null, body: md });
  });

  it('handles an empty frontmatter block', () => {
    const md = '---\n\n---\nbody\n';
    const { frontmatter, body } = splitFrontmatter(md);
    expect(frontmatter).toBe('\n');
    expect(body).toBe('body\n');
  });
});

describe('parseFrontmatter', () => {
  it('parses simple key-value pairs', () => {
    const result = parseFrontmatter('title: Hello\ncount: 3\n');
    expect(result).toEqual({ title: 'Hello', count: 3 });
  });

  it('parses dates into Date objects', () => {
    const result = parseFrontmatter('due: 2026-06-15\n');
    expect(result.due).toBeInstanceOf(Date);
    expect((result.due as Date).getUTCFullYear()).toBe(2026);
  });

  it('parses booleans', () => {
    const result = parseFrontmatter('draft: true\npublished: false\n');
    expect(result).toEqual({ draft: true, published: false });
  });

  it('parses inline lists', () => {
    const result = parseFrontmatter('tags: [react, design, ux]\n');
    expect(result.tags).toEqual(['react', 'design', 'ux']);
  });

  it('parses block lists', () => {
    const result = parseFrontmatter('tags:\n  - react\n  - design\n');
    expect(result.tags).toEqual(['react', 'design']);
  });

  it('preserves string values that look like numbers', () => {
    const result = parseFrontmatter('zip: "01234"\n');
    expect(result.zip).toBe('01234');
  });

  it('returns an empty object for empty frontmatter', () => {
    expect(parseFrontmatter('')).toEqual({});
  });
});

describe('stringifyFrontmatter', () => {
  it('produces a YAML block with --- delimiters', () => {
    const out = stringifyFrontmatter({ title: 'Hello' }, 'body\n');
    expect(out).toBe('---\ntitle: Hello\n---\nbody\n');
  });

  it('round-trips through parseFrontmatter', () => {
    const original = {
      title: 'Note',
      tags: ['a', 'b'],
      count: 5,
      draft: true,
    };
    const out = stringifyFrontmatter(original, 'paragraph\n');
    const parsed = parseFrontmatter(splitFrontmatter(out).frontmatter!);
    expect(parsed).toEqual(original);
  });

  it('handles empty body', () => {
    const out = stringifyFrontmatter({ title: 'X' }, '');
    expect(out).toBe('---\ntitle: X\n---\n');
  });

  it('handles empty properties', () => {
    const out = stringifyFrontmatter({}, 'body\n');
    expect(out).toBe('body\n');
  });
});
