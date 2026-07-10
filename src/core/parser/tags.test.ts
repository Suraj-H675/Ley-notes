import { describe, expect, it } from 'vitest';
import { extractInlineTags, tagSegments } from './tags';

describe('extractInlineTags', () => {
  it('extracts a simple tag', () => {
    expect(extractInlineTags('hello #foo world')).toEqual(['foo']);
  });

  it('extracts nested tags', () => {
    expect(extractInlineTags('a #project/ley/architecture note')).toEqual([
      'project/ley/architecture',
    ]);
  });

  it('extracts multiple tags on one line', () => {
    expect(extractInlineTags('#a #b #c')).toEqual(['a', 'b', 'c']);
  });

  it('dedupes', () => {
    expect(extractInlineTags('#a #a #b')).toEqual(['a', 'b']);
  });

  it('strips trailing slashes', () => {
    expect(extractInlineTags('a #foo/ #b')).toEqual(['foo', 'b']);
  });

  it('skips tags inside inline code', () => {
    expect(extractInlineTags('use `#not-a-tag` but #real-tag')).toEqual(['real-tag']);
  });

  it('skips tags inside fenced code blocks', () => {
    const src = '```\n#code\n```\n#real';
    expect(extractInlineTags(src)).toEqual(['real']);
  });

  it('does not match headings (## heading)', () => {
    // `## Foo` should not produce `foo` — the boundary requires non-word.
    expect(extractInlineTags('## Heading\nbody')).toEqual([]);
  });
});

describe('tagSegments', () => {
  it('splits nested tag', () => {
    expect(tagSegments('a/b/c')).toEqual(['a', 'b', 'c']);
  });

  it('returns single segment for flat tag', () => {
    expect(tagSegments('foo')).toEqual(['foo']);
  });

  it('filters empty segments', () => {
    expect(tagSegments('a//b/')).toEqual(['a', 'b']);
  });
});