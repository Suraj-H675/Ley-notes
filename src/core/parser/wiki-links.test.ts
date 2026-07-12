import { describe, expect, it } from 'vitest';
import {
  completeWikiLink,
  extractWikiLinkTargets,
  extractWikiLinks,
  retargetWikiLinks,
} from './wiki-links';

describe('extractWikiLinks', () => {
  it('extracts a simple [[link]]', () => {
    const r = extractWikiLinks('see [[Foo]] for details');
    expect(r).toHaveLength(1);
    expect(r[0]).toMatchObject({
      target: 'Foo',
      alias: null,
      heading: null,
      blockId: null,
      position: 4,
      isEmbed: false,
    });
  });

  it('extracts [[link|alias]]', () => {
    const r = extractWikiLinks('see [[Foo|as the Foo]]');
    expect(r[0].target).toBe('Foo');
    expect(r[0].alias).toBe('as the Foo');
  });

  it('extracts [[link#heading]]', () => {
    const r = extractWikiLinks('see [[Foo#Section]]');
    expect(r[0].target).toBe('Foo');
    expect(r[0].heading).toBe('Section');
  });

  it('extracts [[link#^block-id]]', () => {
    const r = extractWikiLinks('see [[Foo#^abc123]]');
    expect(r[0].target).toBe('Foo');
    expect(r[0].blockId).toBe('abc123');
  });

  it('extracts Obsidian heading links with aliases', () => {
    const [link] = extractWikiLinks('[[Foo#Deep section|read this]]');
    expect(link).toMatchObject({ target: 'Foo', heading: 'Deep section', alias: 'read this' });
  });

  it('marks embeds via !', () => {
    const r = extractWikiLinks('![[image.png]] and [[Page]]');
    expect(r[0].isEmbed).toBe(true);
    expect(r[0].target).toBe('image.png');
    expect(r[1].isEmbed).toBe(false);
  });

  it('ignores links inside fenced code blocks', () => {
    const src = '```\n[[ignored]]\n```\n[[kept]]';
    const r = extractWikiLinks(src);
    expect(r).toHaveLength(1);
    expect(r[0].target).toBe('kept');
  });

  it('ignores links inside inline code', () => {
    const r = extractWikiLinks('use `[[Foo]]` in code but [[Bar]] is real');
    expect(r).toHaveLength(1);
    expect(r[0].target).toBe('Bar');
  });

  it('preserves positions accurately', () => {
    const src = 'line one\nsee [[Target]] here';
    const r = extractWikiLinks(src);
    const idx = src.indexOf('[[Target]]');
    expect(r[0].position).toBe(idx);
  });

  it('handles multiple links on the same line', () => {
    const r = extractWikiLinks('[[A]] and [[B]] and [[C]]');
    expect(r.map((x) => x.target)).toEqual(['A', 'B', 'C']);
  });

  it('handles titles with spaces', () => {
    const r = extractWikiLinks('[[My Note Title]]');
    expect(r[0].target).toBe('My Note Title');
  });
});

describe('retargetWikiLinks', () => {
  it('updates normal, anchored, aliased, and embedded links while preserving code', () => {
    const source = '[[Old]] [[Old#Part|label]] ![[Old#^block]] `[[Old]]`\n```\n[[Old]]\n```';
    expect(retargetWikiLinks(source, 'Old', 'New title')).toBe(
      '[[New title]] [[New title#Part|label]] ![[New title#^block]] `[[Old]]`\n```\n[[Old]]\n```',
    );
  });
});

describe('extractWikiLinkTargets', () => {
  it('returns lowercase deduped targets', () => {
    const t = extractWikiLinkTargets('[[Foo]] [[foo]] [[Bar]]');
    expect(t.sort()).toEqual(['bar', 'foo']);
  });
});

describe('completeWikiLink', () => {
  it('matches titles and aliases (case-insensitive substring)', () => {
    const out = completeWikiLink('fo', [
      { title: 'Foo', aliases: [] },
      { title: 'Bar', aliases: ['foobar'] },
      { title: 'Baz', aliases: [] },
    ]);
    const titles = out.map((o) => o.title).sort();
    expect(titles).toEqual(['Bar', 'Foo']);
  });
});
