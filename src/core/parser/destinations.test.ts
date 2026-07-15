import { describe, expect, it } from 'vitest';
import { ensureMarkdownBlockReference, extractMarkdownBlockReferences, extractMarkdownDestination, extractMarkdownHeadings, findMarkdownDestinationLine } from './destinations';

describe('Markdown destinations', () => {
  const content = ['# Overview', '', '## Design & API', 'Paragraph with an identity ^block-42', '```md', '# Not real', 'fake ^ignored', '```'].join('\n');

  it('extracts headings outside fenced code', () => {
    expect(extractMarkdownHeadings(content)).toEqual([
      { level: 1, title: 'Overview', line: 1 },
      { level: 2, title: 'Design & API', line: 3 },
    ]);
  });

  it('finds headings by text or Obsidian-style slug', () => {
    expect(findMarkdownDestinationLine(content, 'design & api')).toBe(3);
    expect(findMarkdownDestinationLine(content, 'design-api')).toBe(3);
  });

  it('finds block identifiers and ignores fenced examples', () => {
    expect(findMarkdownDestinationLine(content, null, 'block-42')).toBe(4);
    expect(findMarkdownDestinationLine(content, null, 'ignored')).toBeNull();
  });

  it('extracts block identifiers with useful previews outside fences', () => {
    expect(extractMarkdownBlockReferences(content)).toEqual([{ id: 'block-42', line: 4, preview: 'Paragraph with an identity' }]);
  });

  it('extracts a heading section or a single referenced block', () => {
    const sections = ['# First', 'One', '## Child', 'Two', '# Second', 'Three ^three'].join('\n');
    expect(extractMarkdownDestination(sections, 'First')).toBe('# First\nOne\n## Child\nTwo');
    expect(extractMarkdownDestination(sections, null, 'three')).toBe('Three ^three');
  });

  it('adds a stable block reference at a source line and reuses an existing one', () => {
    const added = ensureMarkdownBlockReference('First\nSecond', 2, 'focus-42');
    expect(added).toMatchObject({ id: 'focus-42', line: 2, preview: 'Second', content: 'First\nSecond ^focus-42', changed: true });
    expect(ensureMarkdownBlockReference(added.content, 2, 'different')).toMatchObject({ id: 'focus-42', changed: false });
  });

  it('refuses blank, heading, and fenced-code bookmark targets', () => {
    expect(() => ensureMarkdownBlockReference('First\n', 2, 'block')).toThrow('Blank lines');
    expect(() => ensureMarkdownBlockReference('# Heading', 1, 'block')).toThrow('Outline');
    expect(() => ensureMarkdownBlockReference('```md\ninside\n```', 2, 'block')).toThrow('Code fences');
    expect(() => ensureMarkdownBlockReference('---\nstatus: active\n---\nBody', 2, 'block')).toThrow('YAML');
  });
});
