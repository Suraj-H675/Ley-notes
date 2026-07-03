import { describe, it, expect } from 'vitest';
import {
  parseWikiLinks,
  extractWikiLinkTitles,
  replaceWikiLinks,
  removeWikiLinks,
  hasWikiLinks,
  createWikiLink,
} from './parseWikiLinks';

describe('parseWikiLinks', () => {
  it('parses a single wiki link', () => {
    const links = parseWikiLinks('See [[Note A]] for context');
    expect(links).toHaveLength(1);
    expect(links[0]).toEqual({
      raw: '[[Note A]]',
      title: 'Note A',
      startIndex: 4,
      endIndex: 14,
    });
  });

  it('parses multiple wiki links in one text', () => {
    const text = 'See [[Note A]] and [[Note B]] and [[Note C]]';
    const links = parseWikiLinks(text);
    expect(links.map((l) => l.title)).toEqual(['Note A', 'Note B', 'Note C']);
    expect(links[0].startIndex).toBe(4);
    expect(links[1].startIndex).toBe(19);
    expect(links[2].startIndex).toBe(34);
  });

  it('trims whitespace from title', () => {
    const links = parseWikiLinks('See [[  Padded Title  ]] please');
    expect(links).toHaveLength(1);
    expect(links[0].title).toBe('Padded Title');
  });

  it('returns empty array when no wiki links present', () => {
    expect(parseWikiLinks('No links here.')).toEqual([]);
    expect(parseWikiLinks('')).toEqual([]);
  });

  it('does not match unclosed wiki links', () => {
    // `[[Note` is unclosed — should not match.
    expect(parseWikiLinks('text [[Note more text')).toEqual([]);
  });

  it('does not match malformed nested brackets', () => {
    // `[[A]B]]` is structurally invalid (the `]` after A breaks the
    // `[^\]]+` capture). The regex correctly returns no match rather
    // than guessing — that's the safe behaviour for a parser.
    const links = parseWikiLinks('[[A]B]]');
    expect(links).toEqual([]);
  });

  it('matches the first complete wiki link when there are extra brackets', () => {
    // `[[A]]extra]]` — the regex matches `[[A]]` (first complete match).
    const links = parseWikiLinks('[[A]]extra]]');
    expect(links).toHaveLength(1);
    expect(links[0].title).toBe('A');
  });

  it('handles titles with special characters', () => {
    const links = parseWikiLinks('[[note-with-dashes]] and [[note.with.dots]]');
    expect(links.map((l) => l.title)).toEqual([
      'note-with-dashes',
      'note.with.dots',
    ]);
  });

  it('handles unicode titles', () => {
    const links = parseWikiLinks('[[Résumé]] and [[日本語]]');
    expect(links.map((l) => l.title)).toEqual(['Résumé', '日本語']);
  });

  it('finds consecutive wiki links without space between them', () => {
    const links = parseWikiLinks('[[A]][[B]][[C]]');
    expect(links.map((l) => l.title)).toEqual(['A', 'B', 'C']);
  });

  it('returns stable offsets', () => {
    const text = 'before [[first]] middle [[second]] after';
    const links = parseWikiLinks(text);
    expect(text.substring(links[0].startIndex, links[0].endIndex)).toBe('[[first]]');
    expect(text.substring(links[1].startIndex, links[1].endIndex)).toBe('[[second]]');
  });

  it('handles repeated regex.exec state correctly (no sticky match bleed)', () => {
    // This test guards against a known regex pitfall: when a global regex
    // is reused after another invocation, lastIndex may carry over.
    parseWikiLinks('[[once]]');
    const links = parseWikiLinks('[[twice]]');
    expect(links).toHaveLength(1);
    expect(links[0].title).toBe('twice');
  });
});

describe('extractWikiLinkTitles', () => {
  it('returns just the titles', () => {
    expect(extractWikiLinkTitles('See [[A]] and [[B]]')).toEqual(['A', 'B']);
  });

  it('returns empty array when no links', () => {
    expect(extractWikiLinkTitles('plain text')).toEqual([]);
  });
});

describe('replaceWikiLinks', () => {
  it('replaces each wiki link via the callback', () => {
    const result = replaceWikiLinks(
      'See [[A]] and [[B]]',
      (title) => `[${title.toUpperCase()}]`,
    );
    expect(result).toBe('See [A] and [B]');
  });

  it('passes trimmed titles to callback', () => {
    const titles: string[] = [];
    replaceWikiLinks('[[  spaced  ]]', (title) => {
      titles.push(title);
      return title;
    });
    expect(titles).toEqual(['spaced']);
  });

  it('returns text unchanged when no wiki links present', () => {
    const result = replaceWikiLinks('plain text', () => 'NEVER');
    expect(result).toBe('plain text');
  });
});

describe('removeWikiLinks', () => {
  it('removes the brackets but keeps the title text', () => {
    expect(removeWikiLinks('See [[A]] and [[B]]')).toBe('See A and B');
  });

  it('handles a single wiki link', () => {
    expect(removeWikiLinks('[[only]]')).toBe('only');
  });

  it('returns text unchanged when no wiki links present', () => {
    expect(removeWikiLinks('plain text')).toBe('plain text');
  });
});

describe('hasWikiLinks', () => {
  it('returns true when wiki links exist', () => {
    expect(hasWikiLinks('See [[A]]')).toBe(true);
  });

  it('returns false when no wiki links exist', () => {
    expect(hasWikiLinks('plain text')).toBe(false);
    expect(hasWikiLinks('')).toBe(false);
  });

  it('does not consume the regex state for subsequent calls', () => {
    expect(hasWikiLinks('[[a]]')).toBe(true);
    expect(hasWikiLinks('plain text')).toBe(false);
    expect(hasWikiLinks('[[b]]')).toBe(true);
  });
});

describe('createWikiLink', () => {
  it('wraps the title in double brackets', () => {
    expect(createWikiLink('Note Title')).toBe('[[Note Title]]');
  });

  it('does not trim the title (caller should provide clean titles)', () => {
    expect(createWikiLink('  spaced  ')).toBe('[[  spaced  ]]');
  });
});