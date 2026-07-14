import { describe, expect, it } from 'vitest';
import { findTagCompletion } from './tag-completion';

describe('tag completion context', () => {
  it('matches new and nested inline tags at valid Markdown boundaries', () => {
    expect(findTagCompletion('Discuss #pro', 12)).toEqual({ from: 9, query: 'pro' });
    expect(findTagCompletion('(#project/le', 12)).toEqual({ from: 2, query: 'project/le' });
    expect(findTagCompletion('#root', 5)).toEqual({ from: 1, query: 'root' });
  });

  it('ignores headings, URL fragments, frontmatter, fenced code, and inline code', () => {
    expect(findTagCompletion('##head', 6)).toBeNull();
    expect(findTagCompletion('https://example.com/#tag', 24)).toBeNull();
    expect(findTagCompletion('href=#anchor', 12)).toBeNull();
    expect(findTagCompletion('---\ntags: #pro\n---\nbody', 14)).toBeNull();
    expect(findTagCompletion('```md\n#pro', 11)).toBeNull();
    expect(findTagCompletion('Use `#pro', 9)).toBeNull();
  });

  it('allows completion after a closed inline span and closed fence', () => {
    const inline = 'Use `#fake` then #re';
    expect(findTagCompletion(inline, inline.length)).toEqual({ from: inline.length - 2, query: 're' });
    const fenced = '```md\n#fake\n```\n#real';
    expect(findTagCompletion(fenced, fenced.length)).toEqual({ from: fenced.length - 4, query: 'real' });
  });
});
