import { describe, it, expect } from 'vitest';
import { extractHeadings } from './extract-headings';

describe('extractHeadings', () => {
  it('returns an empty array for plain text', () => {
    const md = 'just some text\nwith two lines\n';
    expect(extractHeadings(md)).toEqual([]);
  });

  it('extracts a single h1', () => {
    const md = '# Top heading\n\nBody text.\n';
    const headings = extractHeadings(md);
    expect(headings).toHaveLength(1);
    expect(headings[0]).toMatchObject({ level: 1, text: 'Top heading' });
  });

  it('extracts headings at all 6 levels', () => {
    const md = [
      '# H1',
      '## H2',
      '### H3',
      '#### H4',
      '##### H5',
      '###### H6',
    ].join('\n');
    const headings = extractHeadings(md);
    expect(headings.map((h) => h.level)).toEqual([1, 2, 3, 4, 5, 6]);
    expect(headings.map((h) => h.text)).toEqual([
      'H1',
      'H2',
      'H3',
      'H4',
      'H5',
      'H6',
    ]);
  });

  it('strips inline markdown from heading text', () => {
    const md = '# A **bold** heading with `code`\n';
    const headings = extractHeadings(md);
    expect(headings[0].text).toBe('A bold heading with code');
  });

  it('strips wikilink brackets from heading text', () => {
    const md = '# See [[Other Note]] here\n';
    const headings = extractHeadings(md);
    expect(headings[0].text).toBe('See Other Note here');
  });

  it('reports byte offset of each heading in the original string', () => {
    const md = 'intro\n\n# First\n\nbody\n\n## Second\n';
    const headings = extractHeadings(md);
    expect(headings).toHaveLength(2);
    expect(md.substring(headings[0].offset, headings[0].offset + 7)).toBe(
      '# First'
    );
    expect(md.substring(headings[1].offset, headings[1].offset + 9)).toBe(
      '## Second'
    );
  });

  it('ignores ATX headings inside code blocks', () => {
    const md = '```\n# Not a heading\n```\n\n# Real heading\n';
    const headings = extractHeadings(md);
    expect(headings).toHaveLength(1);
    expect(headings[0].text).toBe('Real heading');
  });

  it('extracts Setext-style headings (=== and ---)', () => {
    const md = 'H1 title\n=========\n\nH2 title\n---------\n';
    const headings = extractHeadings(md);
    expect(headings).toHaveLength(2);
    expect(headings[0]).toMatchObject({ level: 1, text: 'H1 title' });
    expect(headings[1]).toMatchObject({ level: 2, text: 'H2 title' });
  });
});
