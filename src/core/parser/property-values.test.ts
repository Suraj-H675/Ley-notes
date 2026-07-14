import { describe, expect, it } from 'vitest';
import { comparePropertyValues, formatPropertyValue, parsePropertyValue } from './property-values';

describe('property values', () => {
  it('formats portable scalar, list, and object values', () => {
    expect(formatPropertyValue(['one', 'two'])).toBe('one, two');
    expect(formatPropertyValue({ score: 4 })).toBe('{"score":4}');
    expect(formatPropertyValue(null)).toBe('');
  });

  it('preserves the existing YAML value type when editing', () => {
    expect(parsePropertyValue('12', 3)).toBe(12);
    expect(parsePropertyValue('false', true)).toBe(false);
    expect(parsePropertyValue('one, two', ['old'])).toEqual(['one', 'two']);
    expect(parsePropertyValue('12', undefined)).toBe('12');
  });

  it('sorts numeric values numerically', () => {
    expect(comparePropertyValues(2, 10)).toBeLessThan(0);
  });
});
