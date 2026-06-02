import { describe, it, expect } from 'vitest';
import { tagColor, collectionColor } from './tagColor';

describe('tagColor', () => {
  it('is deterministic for the same input', () => {
    expect(tagColor('react')).toBe(tagColor('react'));
    expect(tagColor('javascript')).toBe(tagColor('javascript'));
  });

  it('returns a valid HSL string', () => {
    const c = tagColor('typescript');
    expect(c).toMatch(/^hsl\(\s*\d+(\.\d+)?,\s*\d+%,\s*\d+%\s*\)$/);
  });

  it('produces different colors for different inputs (most of the time)', () => {
    const colors = new Set([
      tagColor('react'),
      tagColor('typescript'),
      tagColor('rust'),
      tagColor('python'),
      tagColor('design'),
      tagColor('product'),
    ]);
    expect(colors.size).toBeGreaterThan(4);
  });

  it('uses saturation 50% and lightness 65%', () => {
    expect(tagColor('any-tag')).toContain('50%');
    expect(tagColor('any-tag')).toContain('65%');
  });
});

describe('collectionColor', () => {
  it('rotates 180° from tagColor', () => {
    const tag = tagColor('react');
    const col = collectionColor('react');
    const tagHue = parseFloat(tag.match(/hsl\((\d+)/)![1]);
    const colHue = parseFloat(col.match(/hsl\((\d+)/)![1]);
    const diff = Math.abs(tagHue - colHue);
    expect(diff === 180 || diff === 180 - 360 || diff === 360 - 180).toBe(true);
  });
});
