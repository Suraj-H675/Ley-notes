import { describe, expect, it } from 'vitest';
import { extractMarkdownBlockReferences, extractMarkdownDestination, extractMarkdownHeadings, findMarkdownDestinationLine } from './destinations';

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
});
