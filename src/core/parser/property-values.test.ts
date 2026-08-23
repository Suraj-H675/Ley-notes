import { describe, expect, it } from 'vitest';
import {
  comparePropertyValues,
  formatPropertyValue,
  parsePropertyValue,
  propertyValueError,
} from './property-values';

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

  it('reports invalid edits before they rewrite portable YAML types', () => {
    expect(propertyValueError('one,, three', ['two'])).toMatch(/Separate list/);
    expect(propertyValueError('maybe', true)).toMatch(/true or false/);
    expect(propertyValueError('', 12)).toMatch(/finite number/);
    expect(propertyValueError('{bad json}', { score: 4 })).toMatch(/valid JSON/);
    expect(propertyValueError('planned', 'active')).toBeNull();
    expect(propertyValueError('true', false)).toBeNull();
    expect(propertyValueError('18', 3)).toBeNull();
  });
});
