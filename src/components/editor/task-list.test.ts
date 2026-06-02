import { describe, it, expect } from 'vitest';
import { findTaskLine } from './task-list';

describe('findTaskLine', () => {
  it('returns null for non-task lines', () => {
    expect(findTaskLine('regular paragraph', 0)).toBeNull();
    expect(findTaskLine('a list item without checkbox', 0)).toBeNull();
    expect(findTaskLine('not - [ ] a task', 0)).toBeNull();
  });

  it('finds unchecked task at start of line', () => {
    const md = '- [ ] write tests';
    const result = findTaskLine(md, 0);
    expect(result).not.toBeNull();
    expect(result!.checkboxFrom).toBe(2);
    expect(result!.checkboxTo).toBe(5);
    expect(result!.checked).toBe(false);
  });

  it('finds checked task at start of line', () => {
    const md = '- [x] ship feature';
    const result = findTaskLine(md, 0);
    expect(result).not.toBeNull();
    expect(result!.checked).toBe(true);
  });

  it('finds indented task (any leading whitespace)', () => {
    const md = '  - [ ] indented task';
    const result = findTaskLine(md, 0);
    expect(result).not.toBeNull();
    expect(result!.checked).toBe(false);
  });

  it('finds task on a non-zero starting line', () => {
    const md = 'paragraph\n- [ ] next task';
    // Position at start of "next task" line
    const lineStart = md.indexOf('- [ ]');
    const result = findTaskLine(md, lineStart);
    expect(result).not.toBeNull();
    expect(result!.checked).toBe(false);
  });

  it('returns null if cursor is in the middle of a paragraph (not on task line)', () => {
    const md = 'a regular paragraph';
    const result = findTaskLine(md, 5);
    expect(result).toBeNull();
  });
});
