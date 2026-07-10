import { describe, expect, it } from 'vitest';
import { splitBlocks } from './blocks';

describe('splitBlocks', () => {
  it('returns empty for empty input', () => {
    expect(splitBlocks('')).toEqual([]);
    expect(splitBlocks('   \n\n   ')).toEqual([]);
  });

  it('splits on blank lines', () => {
    const src = 'para one\n\npara two\n\npara three';
    expect(splitBlocks(src).map((b) => b.content)).toEqual([
      'para one',
      'para two',
      'para three',
    ]);
  });

  it('classifies headings', () => {
    const r = splitBlocks('# Heading 1\n\n## Heading 2');
    expect(r[0].type).toBe('heading');
    expect(r[1].type).toBe('heading');
  });

  it('classifies code blocks', () => {
    const r = splitBlocks('```js\nconst x = 1;\n```');
    expect(r[0].type).toBe('code');
  });

  it('classifies blockquotes', () => {
    const r = splitBlocks('> line one\n> line two');
    expect(r[0].type).toBe('quote');
  });

  it('classifies lists', () => {
    const r = splitBlocks('- one\n- two\n- three');
    expect(r[0].type).toBe('list');
  });

  it('classifies ordered lists', () => {
    const r = splitBlocks('1. one\n2. two');
    expect(r[0].type).toBe('list');
  });

  it('classifies dividers', () => {
    const r = splitBlocks('---');
    expect(r[0].type).toBe('divider');
  });

  it('classifies image-only blocks', () => {
    const r = splitBlocks('![alt](image.png)');
    expect(r[0].type).toBe('image');
  });

  it('computes depth from indent', () => {
    const src = '- top\n  - child\n    - grandchild';
    const r = splitBlocks(src);
    expect(r[0].depth).toBe(0);
  });

  it('produces stable IDs across re-splits (same content → same id)', () => {
    const a = splitBlocks('hello world');
    const b = splitBlocks('hello world');
    expect(a[0].id).toBe(b[0].id);
  });

  it('produces different IDs for different content', () => {
    const a = splitBlocks('hello');
    const b = splitBlocks('world');
    expect(a[0].id).not.toBe(b[0].id);
  });
});