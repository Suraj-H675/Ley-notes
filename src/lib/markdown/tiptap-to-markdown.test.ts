import { describe, it, expect } from 'vitest';
import { tiptapJsonToMarkdown } from './tiptap-to-markdown';

describe('tiptapJsonToMarkdown', () => {
  it('returns empty string for empty doc', () => {
    const json = { type: 'doc', content: [] };
    expect(tiptapJsonToMarkdown(json)).toBe('');
  });

  it('converts a paragraph', () => {
    const json = {
      type: 'doc',
      content: [
        { type: 'paragraph', content: [{ type: 'text', text: 'Hello' }] },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('Hello');
  });

  it('converts a heading', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'heading',
          attrs: { level: 2 },
          content: [{ type: 'text', text: 'Title' }],
        },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('## Title');
  });

  it('converts bold and italic marks', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            {
              type: 'text',
              text: 'bold italic',
              marks: [
                { type: 'bold' },
                { type: 'italic' },
              ],
            },
          ],
        },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('***bold italic***');
  });

  it('converts a wikilink', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'see ' },
            {
              type: 'text',
              text: 'other',
              marks: [
                { type: 'wikiLink', attrs: { id: 'x', title: 'Other Note' } },
              ],
            },
            { type: 'text', text: ' for context' },
          ],
        },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('see [[Other Note]] for context');
  });

  it('converts a bullet list', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'bulletList',
          content: [
            {
              type: 'listItem',
              content: [
                {
                  type: 'paragraph',
                  content: [{ type: 'text', text: 'one' }],
                },
              ],
            },
            {
              type: 'listItem',
              content: [
                {
                  type: 'paragraph',
                  content: [{ type: 'text', text: 'two' }],
                },
              ],
            },
          ],
        },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('- one\n- two');
  });

  it('converts a task list with checked/unchecked items', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'taskList',
          content: [
            {
              type: 'taskItem',
              attrs: { checked: false },
              content: [
                {
                  type: 'paragraph',
                  content: [{ type: 'text', text: 'todo' }],
                },
              ],
            },
            {
              type: 'taskItem',
              attrs: { checked: true },
              content: [
                {
                  type: 'paragraph',
                  content: [{ type: 'text', text: 'done' }],
                },
              ],
            },
          ],
        },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('- [ ] todo\n- [x] done');
  });

  it('converts a code block with language', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'codeBlock',
          attrs: { language: 'js' },
          content: [{ type: 'text', text: 'const x = 1;' }],
        },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('```js\nconst x = 1;\n```');
  });

  it('converts a link', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            {
              type: 'text',
              text: 'docs',
              marks: [
                { type: 'link', attrs: { href: 'https://example.com' } },
              ],
            },
          ],
        },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('[docs](https://example.com)');
  });

  it('converts a hard break as two trailing spaces + newline', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            { type: 'text', text: 'a' },
            { type: 'hardBreak' },
            { type: 'text', text: 'b' },
          ],
        },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('a  \nb');
  });

  it('converts a blockquote', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'blockquote',
          content: [
            {
              type: 'paragraph',
              content: [{ type: 'text', text: 'quoted' }],
            },
          ],
        },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('> quoted');
  });

  it('converts a horizontal rule', () => {
    const json = {
      type: 'doc',
      content: [{ type: 'horizontalRule' }],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('---');
  });

  it('handles multiple block types separated by blank lines', () => {
    const json = {
      type: 'doc',
      content: [
        { type: 'heading', attrs: { level: 1 }, content: [{ type: 'text', text: 'T' }] },
        { type: 'paragraph', content: [{ type: 'text', text: 'p' }] },
        { type: 'horizontalRule' },
        { type: 'paragraph', content: [{ type: 'text', text: 'q' }] },
      ],
    };
    expect(tiptapJsonToMarkdown(json)).toBe('# T\n\np\n\n---\n\nq');
  });

  it('handles a wikiLink mark with missing attrs (falls back to text)', () => {
    const json = {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [
            {
              type: 'text',
              text: 'fallback',
              marks: [{ type: 'wikiLink' }],
            },
          ],
        },
      ],
    };
    // When label is missing, fall back to the text content inside [[ ]].
    expect(tiptapJsonToMarkdown(json)).toBe('[[fallback]]');
  });

  it('returns empty string for null/undefined input', () => {
    expect(tiptapJsonToMarkdown(null)).toBe('');
    expect(tiptapJsonToMarkdown(undefined)).toBe('');
  });
});
