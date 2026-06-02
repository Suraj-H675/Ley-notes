import { describe, it, expect } from 'vitest';
import { wikilinkSource } from './wikilink-source';

describe('wikilinkSource', () => {
  const nodes = [
    { id: '1', title: 'React patterns' },
    { id: '2', title: 'Vue basics' },
    { id: '3', title: 'Standalone note' },
  ];

  function makeContext(textBefore: string, textAfter: string) {
    const fullText = textBefore + textAfter;
    return {
      textBefore,
      textAfter,
      pos: textBefore.length,
      explicit: false,
      state: { doc: { toString: () => fullText, length: fullText.length } },
    } as any;
  }

  it('returns null when there is no [[ trigger', () => {
    const result = wikilinkSource(makeContext('hello world', ''), nodes);
    expect(result).toBeNull();
  });

  it('returns options when trigger is just [[ (user is opening a wikilink)', () => {
    const result = wikilinkSource(makeContext('see [[', ''), nodes);
    expect(result).not.toBeNull();
    expect(result!.options.length).toBe(3);
  });

  it('returns node title options after [[', () => {
    const result = wikilinkSource(makeContext('see [[', ']]'), nodes);
    expect(result).not.toBeNull();
    expect(result!.options.length).toBe(3);
    expect(result!.options.map((o) => o.label)).toEqual([
      'React patterns',
      'Vue basics',
      'Standalone note',
    ]);
  });

  it('filters options by the typed query', () => {
    const result = wikilinkSource(makeContext('see [[Re', ']]'), nodes);
    expect(result).not.toBeNull();
    expect(result!.options.length).toBe(1);
    expect(result!.options[0].label).toBe('React patterns');
  });

  it('is case-insensitive in matching', () => {
    const result = wikilinkSource(makeContext('see [[vue', ']]'), nodes);
    expect(result).not.toBeNull();
    expect(result!.options[0].label).toBe('Vue basics');
  });

  it('marks the completion range covering the [[ prefix', () => {
    const result = wikilinkSource(makeContext('see [[Re', ']]'), nodes);
    expect(result).not.toBeNull();
    // 'from' should be at the start of [[ (position 4)
    expect(result!.from).toBe(4);
    // 'to' should be at the cursor position (after 'Re' = position 8)
    expect(result!.to).toBe(8);
  });

  it('returns an option whose apply value is the title', () => {
    const result = wikilinkSource(makeContext('see [[', ']]'), nodes);
    const opt = result!.options[0];
    expect(opt.apply).toBe('React patterns');
  });
});
