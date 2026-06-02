import { useEffect, useRef } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { EditorState, RangeSetBuilder } from '@codemirror/state';
import {
  EditorView,
  Decoration,
  DecorationSet,
  WidgetType,
  ViewPlugin,
  ViewUpdate,
  keymap,
  drawSelection,
  highlightActiveLine,
} from '@codemirror/view';
import { autocompletion, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching, foldGutter, foldKeymap } from '@codemirror/language';
import { placeholder as cmPlaceholder } from '@codemirror/view';
import { parseInlineRanges, type InlineKind } from './inline-ranges';
import { wikilinkSource } from './wikilink-source';
import { findTaskLine, type TaskLineMatch } from './task-list';
import { db } from '@/lib/db';

export interface MarkdownEditorProps {
  content: string;
  onChange: (markdown: string) => void;
  onSave?: () => void;
  onWikilinkNavigate?: (title: string) => void;
  onWikilinkHover?: (info: { title: string; rect: DOMRect } | null) => void;
  placeholder?: string;
  className?: string;
  autoFocus?: boolean;
}

class InlineWidget extends WidgetType {
  constructor(
    readonly kind: InlineKind,
    readonly text: string,
    readonly href?: string,
    readonly onNavigate?: (title: string) => void
  ) {
    super();
  }

  toDOM(): HTMLElement {
    const el = document.createElement('span');
    el.textContent = this.text;
    el.setAttribute('data-inline', this.kind);
    if (this.href !== undefined) {
      el.setAttribute('data-href', this.href);
    }
    if (this.kind === 'wikilink' && this.onNavigate) {
      el.style.cursor = 'pointer';
      el.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();
        this.onNavigate?.(this.href ?? this.text);
      });
    }
    switch (this.kind) {
      case 'strong':
        el.style.fontWeight = '600';
        break;
      case 'em':
        el.style.fontStyle = 'italic';
        break;
      case 'code':
        el.style.fontFamily =
          'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace';
        el.style.fontSize = '0.9em';
        el.style.padding = '0.1em 0.35em';
        el.style.borderRadius = '4px';
        el.style.background = 'rgba(127,127,127,0.15)';
        break;
      case 'strike':
        el.style.textDecoration = 'line-through';
        el.style.opacity = '0.6';
        break;
      case 'highlight':
        el.style.background = 'hsl(48 80% 70% / 0.35)';
        el.style.padding = '0 0.15em';
        el.style.borderRadius = '3px';
        break;
      case 'link':
        el.style.color = 'hsl(217 70% 62%)';
        el.style.textDecoration = 'underline';
        el.style.cursor = 'pointer';
        break;
      case 'wikilink':
        el.style.color = 'hsl(265 55% 65%)';
        el.style.textDecoration = 'underline';
        el.style.textDecorationStyle = 'dotted';
        el.style.cursor = 'pointer';
        break;
    }
    return el;
  }

  ignoreEvents(): boolean {
    // CodeMirror otherwise tries to position the cursor at the click
    // coords, which throws when the widget sits outside the doc text flow.
    return true;
  }
}

class TaskCheckboxWidget extends WidgetType {
  constructor(
    readonly checked: boolean,
    readonly view: EditorView,
    readonly task: TaskLineMatch
  ) {
    super();
  }

  toDOM(): HTMLElement {
    const wrap = document.createElement('label');
    wrap.className = 'cm-task-checkbox-wrap';
    wrap.style.display = 'inline-flex';
    wrap.style.alignItems = 'center';
    wrap.style.marginRight = '0.5ch';
    wrap.style.cursor = 'pointer';
    wrap.contentEditable = 'false';
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = this.checked;
    cb.setAttribute('data-task-checkbox', '');
    cb.style.margin = '0';
    cb.style.pointerEvents = 'auto';
    cb.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      const newChar = this.checked ? ' ' : 'x';
      // The character inside the [ ] brackets is at checkboxFrom + 1.
      this.view.dispatch({
        changes: {
          from: this.task.checkboxFrom + 1,
          to: this.task.checkboxFrom + 2,
          insert: newChar,
        },
      });
    });
    wrap.appendChild(cb);
    return wrap;
  }

  ignoreEvents(): boolean {
    return true; // We handle the click ourselves
  }

  eq(other: TaskCheckboxWidget): boolean {
    return other.checked === this.checked;
  }
}

function buildDecorations(view: EditorView): DecorationSet {
  const ranges = parseInlineRanges(view.state.doc.toString());
  const builder = new RangeSetBuilder<Decoration>();
  const cursorLine = view.state.doc.lineAt(view.state.selection.main.head).number;
  for (const r of ranges) {
    const line = view.state.doc.lineAt(r.from).number;
    if (line === cursorLine) continue;
    builder.add(
      r.from,
      r.to,
      Decoration.replace({
        widget: new InlineWidget(
          r.kind,
          r.inner.text ?? '',
          r.href,
          (title) => ((view as any).someProp as ((t: string) => void) | undefined)?.(title)
        ),
        inclusive: false,
      })
    );
  }
  // Task list checkboxes: replace "- [ ] " or "- [x] " with a checkbox widget
  // for every line that is a task.
  const doc = view.state.doc;
  for (let i = 1; i <= doc.lines; i++) {
    const line = doc.line(i);
    const task = findTaskLine(line.text, 0);
    if (!task) continue;
    if (i === cursorLine) continue;
    // Replace from the line start up to and including the closing bracket + space.
    const replaceTo = line.from + task.checkboxTo - task.lineStart + 1;
    builder.add(
      line.from,
      replaceTo,
      Decoration.replace({
        widget: new TaskCheckboxWidget(task.checked, view, {
          lineStart: line.from,
          checkboxFrom: line.from + (task.checkboxFrom - task.lineStart),
          checkboxTo: line.from + (task.checkboxTo - task.lineStart),
          checked: task.checked,
        }),
        inclusive: false,
      })
    );
  }
  return builder.finish();
}

const livePreviewPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }
    update(update: ViewUpdate) {
      if (update.docChanged || update.selectionSet) {
        this.decorations = buildDecorations(update.view);
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);

export function MarkdownEditor({
  content,
  onChange,
  onSave,
  onWikilinkNavigate,
  onWikilinkHover,
  placeholder = "Type '/' for commands, or '[[' to link another page",
  className,
  autoFocus,
}: MarkdownEditorProps) {
  const ref = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  // Always-fresh refs to the latest callbacks/state. The wikilink widget
  // is created on decoration and reads from these so the closure doesn't
  // capture a stale value.
  const navigateRef = useRef(onWikilinkNavigate);
  navigateRef.current = onWikilinkNavigate;
  const hoverRef = useRef(onWikilinkHover);
  hoverRef.current = onWikilinkHover;
  // Nodes for the wikilink autocomplete. Live-queried at the React layer.
  const dbNodes = useLiveQuery(
    async () => (await db.nodes.toArray()).filter((n) => !n.isArchived),
    [],
    []
  );
  const dbNodesRef = useRef(dbNodes);
  dbNodesRef.current = dbNodes;

  useEffect(() => {
    if (!ref.current) return;

    const state = EditorState.create({
      doc: content,
      extensions: [
        history(),
        drawSelection(),
        highlightActiveLine(),
        bracketMatching(),
        foldGutter(),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        markdown(),
        cmPlaceholder(placeholder),
        autocompletion({
          override: [
            (context: CompletionContext): CompletionResult | null => {
              const doc = context.state.doc.toString();
              const before = doc.slice(0, context.pos);
              const after = doc.slice(context.pos);
              const dbNodesList = dbNodesRef.current ?? [];
              const nodeOptions = dbNodesList
                .filter((n) => !n.isArchived)
                .map((n) => ({ id: n.id, title: n.title || 'Untitled' }));
              const result = wikilinkSource(
                {
                  textBefore: before,
                  textAfter: after,
                  pos: context.pos,
                  explicit: context.explicit,
                  state: { doc: context.state.doc },
                },
                nodeOptions
              );
              if (!result) return null;
              return {
                from: result.from,
                to: result.to,
                options: result.options.map((o) => ({
                  label: o.label,
                  apply: o.apply,
                  detail: o.detail,
                })),
                validFor: /[\]a-zA-Z0-9 _-]*/, // accept closing bracket too
              };
            },
          ],
        }),
        livePreviewPlugin,
        EditorView.domEventHandlers({
          mousemove(event, view) {
            const target = event.target as HTMLElement | null;
            if (!target) return;
            let el: HTMLElement | null = target;
            while (el && el !== view.contentDOM) {
              const inlineKind = el.getAttribute?.('data-inline');
              if (inlineKind === 'wikilink' || inlineKind === 'link') {
                const title =
                  el.getAttribute('data-href') || el.textContent || '';
                const rect = el.getBoundingClientRect();
                hoverRef.current?.({ title, rect });
                return;
              }
              el = el.parentElement;
            }
            hoverRef.current?.(null);
          },
          mouseleave() {
            hoverRef.current?.(null);
          },
        }),
        keymap.of([
          ...defaultKeymap,
          ...historyKeymap,
          ...foldKeymap,
          indentWithTab,
          {
            key: 'Mod-s',
            preventDefault: true,
            run: () => {
              onSave?.();
              return true;
            },
          },
        ]),
        EditorView.lineWrapping,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onChange(update.state.doc.toString());
          }
        }),
      ],
    });

    const view = new EditorView({
      state,
      parent: ref.current,
    });
    viewRef.current = view;

    if (autoFocus) {
      view.focus();
    }

    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (current === content) return;
    view.dispatch({
      changes: { from: 0, to: current.length, insert: content },
    });
  }, [content]);

  // Sync onWikilinkNavigate into the editor so the widget can find it.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    (view as any).someProp = (title: string) => onWikilinkNavigate?.(title);
  }, [onWikilinkNavigate]);

  return (
    <div
      ref={ref}
      data-placeholder={placeholder}
      className={className}
    />
  );
}
