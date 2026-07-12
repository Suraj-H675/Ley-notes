import type { EditorView } from '@codemirror/view';

export type EditorFormat = 'bold' | 'italic' | 'code' | 'wiki-link' | 'task';

export function applyEditorFormat(view: EditorView, format: EditorFormat): boolean {
  if (format === 'task') return toggleTaskLines(view);
  if (format === 'bold') return toggleWrap(view, '**', '**', 'bold text');
  if (format === 'italic') return toggleWrap(view, '*', '*', 'italic text');
  if (format === 'wiki-link') return toggleWrap(view, '[[', ']]', 'note title');

  const selected = view.state.sliceDoc(view.state.selection.main.from, view.state.selection.main.to);
  return selected.includes('\n')
    ? toggleWrap(view, '```\n', '\n```', 'code')
    : toggleWrap(view, '`', '`', 'code');
}

function toggleWrap(view: EditorView, before: string, after: string, placeholder: string): boolean {
  const selection = view.state.selection.main;
  const selected = view.state.sliceDoc(selection.from, selection.to);
  const outsideBefore = view.state.sliceDoc(Math.max(0, selection.from - before.length), selection.from);
  const outsideAfter = view.state.sliceDoc(selection.to, Math.min(view.state.doc.length, selection.to + after.length));

  if (outsideBefore === before && outsideAfter === after) {
    view.dispatch({
      changes: [
        { from: selection.to, to: selection.to + after.length, insert: '' },
        { from: selection.from - before.length, to: selection.from, insert: '' },
      ],
      selection: { anchor: selection.from - before.length, head: selection.to - before.length },
    });
    view.focus();
    return true;
  }

  if (selected.startsWith(before) && selected.endsWith(after) && selected.length >= before.length + after.length) {
    const inner = selected.slice(before.length, selected.length - after.length);
    view.dispatch({
      changes: { from: selection.from, to: selection.to, insert: inner },
      selection: { anchor: selection.from, head: selection.from + inner.length },
    });
    view.focus();
    return true;
  }

  const content = selected || placeholder;
  const insert = `${before}${content}${after}`;
  const from = selection.from + before.length;
  view.dispatch({
    changes: { from: selection.from, to: selection.to, insert },
    selection: selected
      ? { anchor: from, head: from + selected.length }
      : { anchor: from, head: from + placeholder.length },
  });
  view.focus();
  return true;
}

function toggleTaskLines(view: EditorView): boolean {
  const selection = view.state.selection.main;
  const startLine = view.state.doc.lineAt(selection.from);
  const endLine = view.state.doc.lineAt(selection.to);
  const changes: Array<{ from: number; to: number; insert: string }> = [];
  for (let number = startLine.number; number <= endLine.number; number += 1) {
    const line = view.state.doc.line(number);
    const replacement = nextTaskLine(line.text);
    if (replacement !== line.text) changes.push({ from: line.from, to: line.to, insert: replacement });
  }
  view.dispatch({ changes });
  view.focus();
  return true;
}

export function nextTaskLine(line: string): string {
  const checked = /^(\s*)- \[[xX]\] (.*)$/.exec(line);
  if (checked) return `${checked[1]}${checked[2]}`;
  const unchecked = /^(\s*)- \[ \] (.*)$/.exec(line);
  if (unchecked) return `${unchecked[1]}- [x] ${unchecked[2]}`;
  const bullet = /^(\s*)[-*+] (.*)$/.exec(line);
  if (bullet) return `${bullet[1]}- [ ] ${bullet[2]}`;
  const indent = /^(\s*)(.*)$/.exec(line);
  return `${indent?.[1] ?? ''}- [ ] ${indent?.[2] ?? line}`;
}

export function editorFormattingKeymap() {
  return [
    { key: 'Mod-b', run: (view: EditorView) => applyEditorFormat(view, 'bold') },
    { key: 'Mod-i', run: (view: EditorView) => applyEditorFormat(view, 'italic') },
    { key: 'Mod-k', run: (view: EditorView) => applyEditorFormat(view, 'wiki-link') },
    { key: 'Mod-Shift-`', run: (view: EditorView) => applyEditorFormat(view, 'code') },
  ];
}
