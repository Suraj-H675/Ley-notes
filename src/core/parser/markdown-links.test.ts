import { describe, expect, it } from 'vitest';
import { extractInternalMarkdownLinks, resolveInternalMarkdownPath, retargetInternalMarkdownLinks } from './markdown-links';

describe('internal Markdown links', () => {
  it('extracts relative, root, and same-page links while ignoring external, image, and code links', () => {
    const content = ['[Design](../docs/design.md#API)', '[Block](</notes/My Note.md#^focus>)', '[Here](#Overview)', '[web](https://example.com)', '![image](photo.md)', '`[code](fake.md)`', '```md', '[fenced](fake.md)', '```'].join('\n');
    expect(extractInternalMarkdownLinks(content).map(({ label, path, heading, blockId }) => ({ label, path, heading, blockId }))).toEqual([
      { label: 'Design', path: '../docs/design.md', heading: 'API', blockId: null },
      { label: 'Block', path: '/notes/My Note.md', heading: null, blockId: 'focus' },
      { label: 'Here', path: '', heading: 'Overview', blockId: null },
    ]);
  });

  it('resolves paths inside the vault and rejects traversal beyond its root', () => {
    expect(resolveInternalMarkdownPath('projects/ley/source.md', '../design.md')).toBe('projects/design.md');
    expect(resolveInternalMarkdownPath('projects/ley/source.md', '/reference/api.md')).toBe('reference/api.md');
    expect(resolveInternalMarkdownPath('projects/source.md', '../My\\ Note.md')).toBe('My Note.md');
    expect(resolveInternalMarkdownPath('source.md', '../outside.md')).toBeNull();
  });

  it('retargets a moved target and rebases links when their source moves', () => {
    const changes = new Map([['docs/design.md', 'archive/design-system.md']]);
    expect(retargetInternalMarkdownLinks('[Design](../docs/design.md#API)', 'projects/source.md', 'notes/source.md', changes)).toBe('[Design](../archive/design-system.md#API)');
    expect(retargetInternalMarkdownLinks('[Root](/docs/design.md)', 'source.md', 'source.md', changes)).toBe('[Root](/archive/design-system.md)');
  });
});
