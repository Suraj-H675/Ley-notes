import { describe, expect, it } from 'vitest';
import {
  getAliases,
  getFrontmatterTags,
  parseFrontmatter,
  serializeFrontmatter,
} from './frontmatter';

describe('parseFrontmatter', () => {
  it('returns empty frontmatter when no fence is present', () => {
    const r = parseFrontmatter('# Hello\n\nbody');
    expect(r.frontmatter).toEqual({});
    expect(r.body).toBe('# Hello\n\nbody');
  });

  it('parses a simple frontmatter block', () => {
    const r = parseFrontmatter('---\ntitle: Foo\n---\n\n# Body');
    expect(r.frontmatter).toEqual({ title: 'Foo' });
    expect(r.body).toBe('\n# Body');
  });

  it('accepts an empty fenced frontmatter map', () => {
    const r = parseFrontmatter('---\n---\nbody');
    expect(r).toEqual({ frontmatter: {}, body: 'body' });
  });

  it('parses inline-list aliases', () => {
    const r = parseFrontmatter('---\naliases: [Foo, Bar]\n---\nbody');
    expect(getAliases(r.frontmatter)).toEqual(['Foo', 'Bar']);
  });

  it('parses block-style aliases', () => {
    const r = parseFrontmatter('---\naliases:\n  - Foo\n  - Bar\n---\nbody');
    expect(getAliases(r.frontmatter)).toEqual(['Foo', 'Bar']);
  });

  it('parses frontmatter tags', () => {
    const r = parseFrontmatter('---\ntags: [a, b/c]\n---\nbody');
    expect(getFrontmatterTags(r.frontmatter)).toEqual(['a', 'b/c']);
  });

  it('returns body unchanged on unclosed fence (does not swallow doc)', () => {
    const raw = '---\ntitle: Oops\nbody without close';
    const r = parseFrontmatter(raw);
    expect(r.frontmatter).toEqual({});
    expect(r.body).toBe(raw);
  });

  it('returns error on invalid YAML but preserves body', () => {
    const r = parseFrontmatter('---\n: invalid: yaml: :\n---\nbody');
    expect(r.error).toBeDefined();
    expect(r.body).toContain('body');
  });

  it('rejects non-object YAML (arrays/scalars) but keeps body', () => {
    const r = parseFrontmatter('---\n- a\n- b\n---\nbody');
    expect(r.error).toMatch(/map/);
    expect(r.body).toContain('body');
  });
});

describe('serializeFrontmatter', () => {
  it('returns body alone when frontmatter is empty', () => {
    expect(serializeFrontmatter({}, 'body')).toBe('body');
  });

  it('round-trips a simple frontmatter + body', () => {
    const out = serializeFrontmatter({ title: 'Foo' }, '# Body');
    expect(out).toMatch(/^---\n/);
    expect(out).toMatch(/\n---\n/);
    const parsed = parseFrontmatter(out);
    expect(parsed.frontmatter).toEqual({ title: 'Foo' });
    expect(parsed.body).toBe('# Body');
  });

  it('drops undefined values', () => {
    const out = serializeFrontmatter({ a: 1, b: undefined }, 'body');
    expect(out).not.toContain('b:');
    expect(out).toContain('a: 1');
  });
});
