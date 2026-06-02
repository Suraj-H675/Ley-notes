import { describe, it, expect } from 'vitest';
import { parseCalloutBlocks, CALLOUT_TYPES } from './callout';

describe('parseCalloutBlocks', () => {
  it('returns an empty array when there are no callouts', () => {
    expect(parseCalloutBlocks('regular text\n\nmore text')).toEqual([]);
  });

  it('returns an empty array for plain blockquotes', () => {
    expect(parseCalloutBlocks('> not a callout\n> just a quote')).toEqual([]);
  });

  it('detects a single-line note callout', () => {
    const md = '> [!note]\n> This is a note.';
    const blocks = parseCalloutBlocks(md);
    expect(blocks).toHaveLength(1);
    expect(blocks[0].type).toBe('note');
    expect(blocks[0].title).toBe('');
    expect(blocks[0].body).toEqual(['This is a note.']);
    expect(blocks[0].startLine).toBe(1);
    expect(blocks[0].endLine).toBe(2);
  });

  it('detects a callout with a title', () => {
    const md = '> [!warning] Be careful\n> Hot stove ahead.';
    const blocks = parseCalloutBlocks(md);
    expect(blocks).toHaveLength(1);
    expect(blocks[0].type).toBe('warning');
    expect(blocks[0].title).toBe('Be careful');
    expect(blocks[0].body).toEqual(['Hot stove ahead.']);
  });

  it('detects a callout with multi-line body', () => {
    const md = [
      '> [!tip]',
      '> First line',
      '> Second line',
      '> Third line',
    ].join('\n');
    const blocks = parseCalloutBlocks(md);
    expect(blocks).toHaveLength(1);
    expect(blocks[0].type).toBe('tip');
    expect(blocks[0].body).toEqual([
      'First line',
      'Second line',
      'Third line',
    ]);
  });

  it('handles all 12 callout types', () => {
    for (const type of CALLOUT_TYPES) {
      const md = `> [!${type}]\n> body`;
      const blocks = parseCalloutBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe(type);
    }
  });

  it('normalizes the type to lowercase', () => {
    const blocks = parseCalloutBlocks('> [!NOTE]\n> body');
    expect(blocks[0].type).toBe('note');
  });

  it('falls back to "note" for unknown callout types', () => {
    const blocks = parseCalloutBlocks('> [!bogus]\n> body');
    expect(blocks[0].type).toBe('note');
  });

  it('detects two callouts separated by a non-callout line', () => {
    const md = [
      '> [!note]',
      '> first',
      '',
      'paragraph',
      '',
      '> [!warning] Title',
      '> second',
    ].join('\n');
    const blocks = parseCalloutBlocks(md);
    expect(blocks).toHaveLength(2);
    expect(blocks[0].type).toBe('note');
    expect(blocks[0].body).toEqual(['first']);
    expect(blocks[1].type).toBe('warning');
    expect(blocks[1].title).toBe('Title');
    expect(blocks[1].body).toEqual(['second']);
  });

  it('records the correct line numbers for a callout in the middle of a doc', () => {
    const md = ['line1', 'line2', '> [!note]', '> body', 'line5'].join('\n');
    const blocks = parseCalloutBlocks(md);
    expect(blocks[0].startLine).toBe(3);
    expect(blocks[0].endLine).toBe(4);
  });

  it('handles a callout with no body (just the header)', () => {
    const blocks = parseCalloutBlocks('> [!note]');
    expect(blocks).toHaveLength(1);
    expect(blocks[0].type).toBe('note');
    expect(blocks[0].title).toBe('');
    expect(blocks[0].body).toEqual([]);
  });
});
