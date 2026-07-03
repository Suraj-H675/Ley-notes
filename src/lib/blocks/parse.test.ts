import { describe, it, expect } from 'vitest';
import { parseMarkdownToBlocks, blocksToMarkdown } from './parse';
import { generateBlockId } from '../block-id';

describe('parseMarkdownToBlocks', () => {
  describe('empty / whitespace input', () => {
    it('returns an empty array for empty markdown', () => {
      expect(parseMarkdownToBlocks('')).toEqual([]);
    });

    it('returns an empty array for whitespace-only markdown', () => {
      expect(parseMarkdownToBlocks('   \n\n  \n')).toEqual([]);
    });
  });

  describe('paragraphs', () => {
    it('parses a single paragraph', () => {
      const blocks = parseMarkdownToBlocks('Hello world');
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('paragraph');
      expect(blocks[0].blockId).toBeNull();
      expect(blocks[0].textContent).toBe('Hello world');
    });

    it('parses multiple paragraphs separated by blank lines', () => {
      const md = 'First paragraph.\n\nSecond paragraph.\n\nThird paragraph.';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(3);
      expect(blocks.map((b) => b.type)).toEqual([
        'paragraph',
        'paragraph',
        'paragraph',
      ]);
      expect(blocks.map((b) => b.textContent)).toEqual([
        'First paragraph.',
        'Second paragraph.',
        'Third paragraph.',
      ]);
    });

    it('does NOT split on single newlines (soft breaks)', () => {
      const blocks = parseMarkdownToBlocks('Line one.\nLine two.\nLine three.');
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('paragraph');
    });
  });

  describe('headings', () => {
    it('classifies ATX headings', () => {
      const blocks = parseMarkdownToBlocks('# Title');
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('heading');
      expect(blocks[0].textContent).toBe('Title');
    });

    it('classifies deeper-level headings', () => {
      const blocks = parseMarkdownToBlocks('### Sub section');
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('heading');
      expect(blocks[0].textContent).toBe('Sub section');
    });

    it('parses heading + paragraph as 2 blocks', () => {
      const md = '# Title\n\nBody paragraph.';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(2);
      expect(blocks[0].type).toBe('heading');
      expect(blocks[1].type).toBe('paragraph');
    });
  });

  describe('code blocks', () => {
    it('classifies fenced code blocks', () => {
      const md = '```js\nconst x = 1;\n```';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('code');
    });

    it('does NOT extract bid marker from inside a code fence', () => {
      const bid = '20200812220555-lj3enxa';
      const md = '```\n<!-- bid: ' + bid + ' -->\n```';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('code');
      expect(blocks[0].blockId).toBeNull();
    });
  });

  describe('lists', () => {
    it('classifies bullet lists', () => {
      const md = '- one\n- two\n- three';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('list');
    });

    it('classifies ordered lists', () => {
      const md = '1. one\n2. two\n3. three';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('list');
    });

    it('classifies task lists as task-list (not list)', () => {
      const md = '- [ ] todo\n- [x] done';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('task-list');
    });
  });

  describe('quotes and callouts', () => {
    it('classifies plain blockquotes as quote', () => {
      const md = '> A quoted line.';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('quote');
    });

    it('classifies callout syntax (`> [!note]`) as callout', () => {
      const md = '> [!note]\n> An important note.';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('callout');
    });

    it('classifies callout with other types (warning, tip, etc.) as callout', () => {
      const md = '> [!warning]\n> Be careful.';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks[0].type).toBe('callout');
    });
  });

  describe('other block types', () => {
    it('classifies horizontal rules as divider', () => {
      const md = '---';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('divider');
    });

    it('classifies images as image', () => {
      const md = '![alt text](https://example.com/x.png)';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('image');
    });

    it('classifies GFM tables as table', () => {
      const md = '| a | b |\n|---|---|\n| 1 | 2 |';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('table');
    });

    it('classifies raw HTML blocks as html', () => {
      const md = '<div>\ncustom html\n</div>';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].type).toBe('html');
    });
  });

  describe('bid marker extraction', () => {
    it('extracts bid marker from end of paragraph', () => {
      const bid = '20200812220555-lj3enxa';
      const md = 'Hello world\n<!-- bid: ' + bid + ' -->';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].blockId).toBe(bid);
      expect(blocks[0].markdown).toBe(md);
    });

    it('extracts bid marker from end of heading', () => {
      const bid = '20260703041255-m0z9a4k';
      const md = '# Title\n<!-- bid: ' + bid + ' -->';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].blockId).toBe(bid);
    });

    it('uses the LAST bid marker when multiple are present', () => {
      const bid1 = '20200812220555-aaaaaaa';
      const bid2 = '20260703041255-lj3enxa';
      const md = `<!-- bid: ${bid1} -->\nBody\n<!-- bid: ${bid2} -->`;
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].blockId).toBe(bid2);
    });

    it('returns null blockId when no marker present', () => {
      const blocks = parseMarkdownToBlocks('Hello world');
      expect(blocks[0].blockId).toBeNull();
    });

    it('handles bid marker with flexible whitespace', () => {
      const bid = '20200812220555-lj3enxa';
      const md = 'Body\n<!--bid:' + bid + '-->';
      const blocks = parseMarkdownToBlocks(md);
      expect(blocks).toHaveLength(1);
      expect(blocks[0].blockId).toBe(bid);
    });
  });

  describe('mixed content', () => {
    it('classifies a realistic page correctly', () => {
      const md = `# Project Plan

This is the intro.

## Goals

- First goal
- Second goal

## Tasks

- [ ] Do the thing
- [x] Did the other thing

\`\`\`ts
const x: number = 42;
\`\`\`

> [!note]
> Important context here.

---

See [[Other Note]] for related info.`;
      const blocks = parseMarkdownToBlocks(md);
      const types = blocks.map((b) => b.type);
      expect(types).toEqual([
        'heading', // # Project Plan
        'paragraph', // intro
        'heading', // ## Goals
        'list', // bullet list
        'heading', // ## Tasks
        'task-list', // task list
        'code', // ts code
        'callout', // > [!note]
        'divider', // ---
        'paragraph', // see [[other note]]
      ]);
    });
  });

  describe('text content extraction', () => {
    it('strips heading markers', () => {
      const blocks = parseMarkdownToBlocks('# Title');
      expect(blocks[0].textContent).toBe('Title');
    });

    it('strips emphasis', () => {
      const blocks = parseMarkdownToBlocks('This is **bold** and *italic*.');
      expect(blocks[0].textContent).toBe('This is bold and italic.');
    });

    it('strips inline code backticks', () => {
      const blocks = parseMarkdownToBlocks('Use `npm install` to install.');
      expect(blocks[0].textContent).toMatch(/Use npm install to install/);
    });

    it('strips wiki link brackets but keeps the title', () => {
      const blocks = parseMarkdownToBlocks('See [[Other Note]] for details.');
      expect(blocks[0].textContent).toBe('See Other Note for details.');
    });
  });

  describe('frontmatter', () => {
    it('strips YAML frontmatter before parsing', () => {
      const md = `---
title: My Page
tags: [research]
---

# Real Title

Body paragraph.`;
      const blocks = parseMarkdownToBlocks(md);
      // No block should be produced for the frontmatter itself.
      expect(blocks.map((b) => b.type)).toEqual([
        'heading',
        'paragraph',
      ]);
    });
  });
});

describe('blocksToMarkdown', () => {
  it('returns empty string for empty array', () => {
    expect(blocksToMarkdown([])).toBe('');
  });

  it('returns single block markdown with trailing newline', () => {
    const blocks = [
      {
        type: 'paragraph' as const,
        markdown: 'Hello',
        blockId: null,
        textContent: 'Hello',
      },
    ];
    expect(blocksToMarkdown(blocks)).toBe('Hello\n');
  });

  it('joins multiple blocks with blank line separator and trailing newline', () => {
    const blocks = [
      { type: 'heading' as const, markdown: '# Title', blockId: null, textContent: 'Title' },
      { type: 'paragraph' as const, markdown: 'Body', blockId: null, textContent: 'Body' },
    ];
    expect(blocksToMarkdown(blocks)).toBe('# Title\n\nBody\n');
  });

  it('preserves bid markers in serialization', () => {
    const bid = '20200812220555-lj3enxa';
    const blocks = [
      {
        type: 'paragraph' as const,
        markdown: `Hello\n<!-- bid: ${bid} -->`,
        blockId: bid,
        textContent: 'Hello',
      },
    ];
    const md = blocksToMarkdown(blocks);
    expect(md).toBe(`Hello\n<!-- bid: ${bid} -->\n`);
  });
});

describe('round-trip parse → serialize → parse', () => {
  it('preserves block structure for simple content', () => {
    const original = `# Title

A paragraph with **bold** and *italic*.

Another paragraph.

- List item one
- List item two
`;

    const blocks1 = parseMarkdownToBlocks(original);
    const md2 = blocksToMarkdown(blocks1);
    const blocks2 = parseMarkdownToBlocks(md2);

    expect(blocks2.map((b) => b.type)).toEqual(blocks1.map((b) => b.type));
    expect(blocks2.map((b) => b.textContent)).toEqual(
      blocks1.map((b) => b.textContent),
    );
  });

  it('preserves bid markers through round-trip', () => {
    const bid1 = '20200812220555-aaaaaaa';
    const bid2 = '20260703041255-lj3enxa';
    const original = `# Title

First paragraph.
<!-- bid: ${bid1} -->

Second paragraph.
<!-- bid: ${bid2} -->
`;
    const blocks1 = parseMarkdownToBlocks(original);
    const md2 = blocksToMarkdown(blocks1);
    expect(md2).toBe(original);
    const blocks2 = parseMarkdownToBlocks(md2);
    expect(blocks2[0].blockId).toBeNull(); // heading has no bid
    expect(blocks2[1].blockId).toBe(bid1);
    expect(blocks2[2].blockId).toBe(bid2);
  });

  it('re-generated IDs survive round-trip', () => {
    const id1 = generateBlockId();
    const id2 = generateBlockId();
    const md = `# Title

Para one
<!-- bid: ${id1} -->

Para two
<!-- bid: ${id2} -->
`;
    const blocks1 = parseMarkdownToBlocks(md);
    const md2 = blocksToMarkdown(blocks1);
    expect(md2).toBe(md); // exact same markdown
    const blocks2 = parseMarkdownToBlocks(md2);
    expect(blocks2[0].blockId).toBeNull(); // heading has no bid
    expect(blocks2[1].blockId).toBe(id1);
    expect(blocks2[2].blockId).toBe(id2);
  });
});