import { markdown } from '@codemirror/lang-markdown';
import { EditorSelection, EditorState } from '@codemirror/state';
import { GFM } from '@lezer/markdown';
import { describe, expect, it } from 'vitest';
import { collectLivePreviewSyntax } from './live-preview';

function preview(doc: string, cursorLine: number) {
  const initial = EditorState.create({ doc });
  const cursor = initial.doc.line(cursorLine).from;
  const state = EditorState.create({
    doc,
    selection: EditorSelection.cursor(cursor),
    extensions: [markdown({ extensions: GFM })],
  });
  return { state, syntax: collectLivePreviewSyntax(state) };
}

function hiddenText(state: EditorState, syntax: ReturnType<typeof collectLivePreviewSyntax>): string[] {
  return syntax.replacements
    .filter((replacement) => replacement.kind === 'hide')
    .map((replacement) => state.doc.sliceString(replacement.from, replacement.to));
}

describe('live preview syntax', () => {
  it('conceals inline syntax away from the cursor while leaving fenced code literal', () => {
    const doc = '# Heading\n**bold** and *italic* and `code`\n- [ ] Task\n```md\n**raw**\n```';
    const { state, syntax } = preview(doc, 3);
    const hidden = hiddenText(state, syntax);

    expect(hidden).toEqual(expect.arrayContaining(['#', '**', '*', '`']));
    expect(hidden.filter((value) => value === '**')).toHaveLength(2);
    expect(syntax.replacements.some((replacement) => replacement.kind === 'task')).toBe(false);
    expect(syntax.lines).toContainEqual({ from: 0, className: 'cm-live-heading cm-live-heading-1' });
    expect(syntax.replacements.some((replacement) => replacement.from >= doc.indexOf('```md'))).toBe(false);
  });

  it('renders inactive tasks as stateful checkbox replacements', () => {
    const { syntax } = preview('Before\n- [ ] Open\n- [x] Done', 1);
    const tasks = syntax.replacements.filter((replacement) => replacement.kind === 'task');

    expect(tasks).toHaveLength(2);
    expect(tasks).toEqual(expect.arrayContaining([
      expect.objectContaining({ checked: false }),
      expect.objectContaining({ checked: true }),
    ]));
  });

  it('shows readable labels for wiki links and regular Markdown links', () => {
    const doc = 'Cursor\n[[Projects/Roadmap|Roadmap]] and [site](https://example.com)';
    const { state, syntax } = preview(doc, 1);

    expect(hiddenText(state, syntax)).toEqual(expect.arrayContaining([
      '[[Projects/Roadmap|',
      ']]',
      '[',
      '](https://example.com)',
    ]));
  });

  it('reveals all source tokens on the active line', () => {
    const { syntax } = preview('Before\n**bold** [[Target]] [site](https://example.com) - [ ] Task', 2);

    expect(syntax.replacements).toHaveLength(0);
  });
});
