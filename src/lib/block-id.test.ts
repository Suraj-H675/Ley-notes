import { describe, it, expect } from 'vitest';
import {
  generateBlockId,
  parseBlockId,
  setBlockId,
  stripBlockId,
  isValidBlockId,
} from './block-id';

describe('generateBlockId', () => {
  it('produces a string matching the block ID pattern', () => {
    const id = generateBlockId();
    expect(id).toMatch(/^\d{14}-[a-z0-9]{7}$/);
  });

  it('uses the passed Date argument', () => {
    const date = new Date(Date.UTC(2026, 6, 3, 4, 12, 55));
    const id = generateBlockId(date);
    expect(id).toMatch(/^20260703041255-/);
  });

  it('zero-pads single-digit months and days', () => {
    const date = new Date(Date.UTC(2026, 0, 5, 4, 12, 55));
    const id = generateBlockId(date);
    expect(id).toMatch(/^20260105041255-/);
  });

  it('produces different IDs on 1000 consecutive calls (no immediate collisions)', () => {
    const ids = new Set<string>();
    for (let i = 0; i < 1000; i++) {
      ids.add(generateBlockId());
    }
    expect(ids.size).toBe(1000);
  });

  it('uses UTC time, not local time', () => {
    // Build a date whose UTC and local representations differ.
    // We test that the generated ID's timestamp matches the UTC components.
    const date = new Date(Date.UTC(2030, 11, 31, 23, 59, 59));
    const id = generateBlockId(date);
    expect(id.startsWith('20301231235959-')).toBe(true);
  });
});

describe('isValidBlockId', () => {
  it('returns true for valid IDs', () => {
    expect(isValidBlockId('20200812220555-lj3enxa')).toBe(true);
    expect(isValidBlockId('20260703041255-0000000')).toBe(true);
    expect(isValidBlockId('20260703041255-zzzzzzz')).toBe(true);
  });

  it('returns false for IDs with wrong digit count', () => {
    expect(isValidBlockId('2020081222055-lj3enxa')).toBe(false); // 13 digits
    expect(isValidBlockId('202008122205550-lj3enxa')).toBe(false); // 15 digits
  });

  it('returns false for IDs with wrong hash length', () => {
    expect(isValidBlockId('20200812220555-lj3enx')).toBe(false); // 6 chars
    expect(isValidBlockId('20200812220555-lj3enxaa')).toBe(false); // 8 chars
  });

  it('returns false for IDs with uppercase or invalid characters in hash', () => {
    expect(isValidBlockId('20200812220555-LJ3ENXA')).toBe(false); // uppercase
    expect(isValidBlockId('20200812220555-lj3_nxa')).toBe(false); // underscore
    expect(isValidBlockId('20200812220555-lj3enx!')).toBe(false); // special char
  });

  it('returns false for empty string or non-strings', () => {
    expect(isValidBlockId('')).toBe(false);
  });
});

describe('parseBlockId', () => {
  it('extracts ID from standard marker at end of markdown', () => {
    expect(parseBlockId('Hello world\n<!-- bid: 20260703041255-lj3enxa -->'))
      .toBe('20260703041255-lj3enxa');
  });

  it('returns null when no marker present', () => {
    expect(parseBlockId('Hello world')).toBeNull();
    expect(parseBlockId('')).toBeNull();
    expect(parseBlockId('Some text with <!-- a regular comment --> in it')).toBeNull();
  });

  it('handles extra whitespace inside marker', () => {
    expect(parseBlockId('text\n<!--bid:20200812220555-lj3enxa-->'))
      .toBe('20200812220555-lj3enxa');
    expect(parseBlockId('text\n<!--  bid:  20200812220555-lj3enxa  -->'))
      .toBe('20200812220555-lj3enxa');
  });

  it('returns the LAST marker if multiple are present', () => {
    const md =
      '<!-- bid: 20200812220555-aaaaaaa -->\ntext\n<!-- bid: 20260703041255-lj3enxa -->';
    expect(parseBlockId(md)).toBe('20260703041255-lj3enxa');
  });

  it('does not match a bid: comment that has invalid format', () => {
    expect(parseBlockId('text\n<!-- bid: not-a-real-id -->')).toBeNull();
    expect(parseBlockId('text\n<!-- bid: 20200812220555-LJ3ENXA -->')).toBeNull();
  });
});

describe('setBlockId', () => {
  it('appends marker to markdown without one', () => {
    expect(setBlockId('Hello world', '20260703041255-lj3enxa')).toBe(
      'Hello world\n<!-- bid: 20260703041255-lj3enxa -->',
    );
  });

  it('replaces an existing marker', () => {
    expect(
      setBlockId('Hello\n<!-- bid: 20200812220555-aaaaaaa -->', '20260703041255-lj3enxa'),
    ).toBe('Hello\n<!-- bid: 20260703041255-lj3enxa -->');
  });

  it('is idempotent — calling twice with the same ID produces the same result', () => {
    const md = 'Hello world';
    const once = setBlockId(md, '20260703041255-lj3enxa');
    const twice = setBlockId(once, '20260703041255-lj3enxa');
    expect(twice).toBe(once);
  });

  it('appends marker to multi-line markdown', () => {
    expect(setBlockId('line one\nline two', '20260703041255-lj3enxa')).toBe(
      'line one\nline two\n<!-- bid: 20260703041255-lj3enxa -->',
    );
  });

  it('collapses extra trailing newlines before marker', () => {
    expect(setBlockId('Hello world\n\n\n', '20260703041255-lj3enxa')).toBe(
      'Hello world\n<!-- bid: 20260703041255-lj3enxa -->',
    );
  });
});

describe('stripBlockId', () => {
  it('removes the marker', () => {
    expect(stripBlockId('Hello\n<!-- bid: 20260703041255-lj3enxa -->')).toBe(
      'Hello',
    );
  });

  it('returns unchanged markdown when no marker present', () => {
    expect(stripBlockId('Hello world')).toBe('Hello world');
    expect(stripBlockId('')).toBe('');
  });

  it('removes only the last marker line, not earlier ones', () => {
    expect(
      stripBlockId(
        '<!-- bid: 20200812220555-aaaaaaa -->\ntext\n<!-- bid: 20260703041255-lj3enxa -->',
      ),
    ).toBe('<!-- bid: 20200812220555-aaaaaaa -->\ntext');
  });

  it('preserves trailing newline at end of file', () => {
    expect(stripBlockId('Hello\n<!-- bid: 20260703041255-lj3enxa -->\n')).toBe(
      'Hello\n',
    );
  });
});