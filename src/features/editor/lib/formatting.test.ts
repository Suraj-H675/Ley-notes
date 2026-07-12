import { describe, expect, it } from 'vitest';
import { nextTaskLine } from './formatting';

describe('editor Markdown formatting', () => {
  it('cycles plain lines through unchecked, checked, and plain task states', () => {
    expect(nextTaskLine('Ship the editor')).toBe('- [ ] Ship the editor');
    expect(nextTaskLine('- [ ] Ship the editor')).toBe('- [x] Ship the editor');
    expect(nextTaskLine('- [x] Ship the editor')).toBe('Ship the editor');
  });

  it('preserves indentation and upgrades ordinary bullets', () => {
    expect(nextTaskLine('  Nested')).toBe('  - [ ] Nested');
    expect(nextTaskLine('  * Existing bullet')).toBe('  - [ ] Existing bullet');
  });
});
