import { describe, it, expect } from 'vitest';
import { defaultGraphSettings } from './defaultGraphSettings';

describe('defaultGraphSettings', () => {
  it('returns a global settings row with all defaults', () => {
    const s = defaultGraphSettings('global');
    expect(s.scope).toBe('global');
    expect(s.colorScheme).toBe('untyped');
    expect(s.physics).toEqual({
      centerForce: 1,
      chargeForce: -60,
      linkForce: 1,
      linkDistance: 80,
    });
    expect(s.display).toEqual({
      nodeSize: 1,
      edgeThickness: 1,
      textFade: 0.25,
      showLabels: true,
    });
    expect(s.filters).toEqual({
      searchQuery: '',
      selectedTags: [],
      selectedCollections: [],
      showOrphans: true,
    });
    expect(s.panelVisible).toBe(true);
    expect(s.panelSectionsOpen.groups).toBe(true);
    expect(s.panelSectionsOpen.filters).toBe(false);
    expect(s.localDepth).toBe(1);
  });

  it('returns a local settings row with localDepth=1', () => {
    const s = defaultGraphSettings('local');
    expect(s.scope).toBe('local');
    expect(s.localDepth).toBe(1);
  });

  it('sets updatedAt to a number', () => {
    const s = defaultGraphSettings('global');
    expect(typeof s.updatedAt).toBe('number');
  });

  it('returns a fresh object each call (no shared refs)', () => {
    const a = defaultGraphSettings('global');
    const b = defaultGraphSettings('global');
    expect(a).not.toBe(b);
    expect(a.physics).not.toBe(b.physics);
    expect(a.filters).not.toBe(b.filters);
  });
});
