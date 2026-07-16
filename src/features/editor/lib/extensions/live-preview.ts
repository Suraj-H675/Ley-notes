import { syntaxTree } from '@codemirror/language';
import type { EditorState, Range } from '@codemirror/state';
import { Decoration, type DecorationSet, EditorView, ViewPlugin, WidgetType } from '@codemirror/view';
import { extractWikiLinks } from '@/core/parser/wiki-links';

interface PreviewReplacement {
  from: number;
  to: number;
  kind: 'hide' | 'task';
  checked?: boolean;
}

interface PreviewLine {
  from: number;
  className: string;
}

export interface LivePreviewSyntax {
  replacements: PreviewReplacement[];
  lines: PreviewLine[];
}

const HIDDEN_MARK = Decoration.replace({});

export function livePreviewExtension() {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = buildDecorations(view);
      }

      update(update: import('@codemirror/view').ViewUpdate) {
        if (update.docChanged || update.selectionSet || update.viewportChanged) {
          this.decorations = buildDecorations(update.view);
        }
      }
    },
    {
      decorations: (plugin) => plugin.decorations,
      eventHandlers: {
        mousedown(event, view) {
          const input = taskInput(event.target);
          if (!input) return false;
          event.preventDefault();
          toggleTaskAt(view, Number(input.dataset.taskFrom), !input.checked);
          return true;
        },
        keydown(event, view) {
          if (event.key !== ' ' && event.key !== 'Enter') return false;
          const input = taskInput(event.target);
          if (!input) return false;
          event.preventDefault();
          toggleTaskAt(view, Number(input.dataset.taskFrom), !input.checked);
          return true;
        },
      },
    },
  );
}

export function collectLivePreviewSyntax(state: EditorState): LivePreviewSyntax {
  const activeLines = selectedLines(state);
  const replacements: PreviewReplacement[] = [];
  const lines: PreviewLine[] = [];
  const source = state.doc.toString();

  syntaxTree(state).iterate({
    enter(node) {
      const name = node.name;
      const line = state.doc.lineAt(node.from);
      if (/^ATXHeading[1-6]$/.test(name)) {
        lines.push({ from: line.from, className: `cm-live-heading cm-live-heading-${name.at(-1)}` });
      } else if (name === 'Blockquote') {
        lines.push({ from: line.from, className: 'cm-live-blockquote' });
      } else if (name === 'HorizontalRule' && !activeLines.has(line.number)) {
        lines.push({ from: line.from, className: 'cm-live-horizontal-rule' });
        replacements.push({ from: node.from, to: node.to, kind: 'hide' });
      }

      if (activeLines.has(line.number)) return;
      if (name === 'HeaderMark' || name === 'QuoteMark' || name === 'EmphasisMark' || name === 'StrikethroughMark') {
        replacements.push({ from: node.from, to: node.to, kind: 'hide' });
      } else if (name === 'CodeMark' && node.node.parent?.name === 'InlineCode') {
        replacements.push({ from: node.from, to: node.to, kind: 'hide' });
      } else if (name === 'TaskMarker') {
        replacements.push({
          from: node.from,
          to: node.to,
          kind: 'task',
          checked: source.slice(node.from, node.to).toLowerCase() === '[x]',
        });
      } else if (name === 'Link') {
        const raw = source.slice(node.from, node.to);
        const labelEnd = raw.indexOf('](');
        if (labelEnd >= 0) {
          replacements.push({ from: node.from, to: node.from + 1, kind: 'hide' });
          replacements.push({ from: node.from + labelEnd, to: node.to, kind: 'hide' });
        }
      }
    },
  });

  for (const link of extractWikiLinks(source)) {
    if (link.isEmbed || activeLines.has(state.doc.lineAt(link.position).number)) continue;
    const pipeAt = link.raw.indexOf('|');
    replacements.push({
      from: link.position,
      to: pipeAt >= 0 ? link.position + pipeAt + 1 : link.position + 2,
      kind: 'hide',
    });
    replacements.push({ from: link.position + link.raw.length - 2, to: link.position + link.raw.length, kind: 'hide' });
  }

  return {
    replacements: nonOverlapping(replacements),
    lines: uniqueLines(lines),
  };
}

function buildDecorations(view: EditorView): DecorationSet {
  const syntax = collectLivePreviewSyntax(view.state);
  const visible = (from: number, to = from) => view.visibleRanges.some((range) => to >= range.from && from <= range.to);
  const ranges: Array<Range<Decoration>> = [];
  for (const line of syntax.lines) {
    if (visible(line.from)) ranges.push(Decoration.line({ class: line.className }).range(line.from));
  }
  for (const replacement of syntax.replacements) {
    if (!visible(replacement.from, replacement.to)) continue;
    const decoration = replacement.kind === 'task'
      ? Decoration.replace({ widget: new TaskCheckboxWidget(replacement.from, Boolean(replacement.checked)) })
      : HIDDEN_MARK;
    ranges.push(decoration.range(replacement.from, replacement.to));
  }
  return Decoration.set(ranges, true);
}

class TaskCheckboxWidget extends WidgetType {
  constructor(private readonly from: number, private readonly checked: boolean) {
    super();
  }

  eq(other: TaskCheckboxWidget): boolean {
    return other.from === this.from && other.checked === this.checked;
  }

  toDOM(): HTMLElement {
    const wrapper = document.createElement('span');
    wrapper.className = 'cm-live-task';
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.checked = this.checked;
    input.dataset.taskFrom = String(this.from);
    input.setAttribute('aria-label', this.checked ? 'Mark task incomplete' : 'Mark task complete');
    wrapper.append(input);
    return wrapper;
  }

  ignoreEvent(): boolean {
    return false;
  }
}

function selectedLines(state: EditorState): Set<number> {
  const lines = new Set<number>();
  for (const range of state.selection.ranges) {
    const first = state.doc.lineAt(range.from).number;
    const last = state.doc.lineAt(range.to).number;
    for (let line = first; line <= last; line += 1) lines.add(line);
  }
  return lines;
}

function nonOverlapping(ranges: PreviewReplacement[]): PreviewReplacement[] {
  const sorted = ranges.filter((range) => range.to > range.from).sort((left, right) => left.from - right.from || left.to - right.to);
  const result: PreviewReplacement[] = [];
  for (const range of sorted) {
    const previous = result.at(-1);
    if (previous && range.from < previous.to) continue;
    if (previous && range.from === previous.from && range.to === previous.to) continue;
    result.push(range);
  }
  return result;
}

function uniqueLines(lines: PreviewLine[]): PreviewLine[] {
  const seen = new Set<string>();
  return lines.filter((line) => {
    const key = `${line.from}:${line.className}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function taskInput(target: EventTarget | null): HTMLInputElement | null {
  return target instanceof HTMLInputElement && target.dataset.taskFrom ? target : null;
}

function toggleTaskAt(view: EditorView, from: number, checked: boolean): void {
  if (!Number.isFinite(from) || !/^\[[ xX]\]$/.test(view.state.doc.sliceString(from, from + 3))) return;
  view.dispatch({ changes: { from: from + 1, to: from + 2, insert: checked ? 'x' : ' ' } });
  view.focus();
}
