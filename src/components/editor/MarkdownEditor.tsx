import { useEffect, useRef, useState, useCallback } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { EditorState, RangeSetBuilder, StateEffect, StateField } from '@codemirror/state';
import {
  EditorView,
  Decoration,
  WidgetType,
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
import { parseCalloutBlocks, type CalloutBlock, type CalloutType } from './callout';
import { fetchTransclusionData } from '@/lib/markdown/transclusions';
import { db } from '@/lib/db';
import { blockDragHandleExtension } from './BlockDragHandle';
import { headingCollapseGutter } from './HeadingCollapse';
import { EditorFindBar, clearFindHighlights, findHighlightPlugin } from './EditorFindBar';

/**
 * Per-title data the transclusion widget needs. `null` means the target
 * note was not found. `undefined` means we haven't loaded it yet.
 */
export interface TransclusionData {
  title: string;
  plainText: string;
  exists: boolean;
}

export interface MarkdownEditorProps {
  content: string;
  onChange: (markdown: string) => void;
  onSave?: () => void;
  onWikilinkNavigate?: (title: string) => void;
  onWikilinkHover?: (info: { title: string; rect: DOMRect } | null) => void;
  /** Called once with the CodeMirror EditorView when the editor is mounted.
   * Use this to imperatively scroll the editor to a character offset. */
  onEditorReady?: (view: EditorView) => void;
  placeholder?: string;
  className?: string;
  autoFocus?: boolean;
}

class InlineWidget extends WidgetType {
  private readonly view: EditorView | null;

  constructor(
    readonly kind: InlineKind,
    readonly text: string,
    readonly href?: string,
    view: EditorView | null = null
  ) {
    super();
    this.view = view;
  }

  toDOM(): HTMLElement {
    const el = document.createElement('span');
    el.textContent = this.text;
    el.setAttribute('data-inline', this.kind);
    if (this.href !== undefined) {
      el.setAttribute('data-href', this.href);
    }
    if (this.kind === 'wikilink') {
      el.style.cursor = 'pointer';
      el.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();
        const navigate = (this.view as any).someProp as ((t: string) => void) | undefined;
        navigate?.(this.href ?? this.text);
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
    return true;
  }
}

class TaskCheckboxWidget extends WidgetType {
  private readonly view: EditorView | null;

  constructor(
    readonly checked: boolean,
    readonly task: TaskLineMatch,
    view: EditorView | null
  ) {
    super();
    this.view = view;
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
      // Toggle the checkbox character in the editor document
      const newChar = this.checked ? ' ' : 'x';
      this.view?.dispatch({
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
    return true;
  }

  eq(other: TaskCheckboxWidget): boolean {
    return other.checked === this.checked;
  }
}

const CALLOUT_COLORS: Record<CalloutType, { bg: string; border: string; fg: string }> = {
  note: { bg: 'hsl(217 30% 22%)', border: 'hsl(217 60% 55%)', fg: 'hsl(217 80% 80%)' },
  tip: { bg: 'hsl(150 30% 20%)', border: 'hsl(150 55% 50%)', fg: 'hsl(150 70% 75%)' },
  info: { bg: 'hsl(200 30% 22%)', border: 'hsl(200 60% 55%)', fg: 'hsl(200 80% 80%)' },
  warning: { bg: 'hsl(35 35% 22%)', border: 'hsl(35 80% 50%)', fg: 'hsl(35 90% 75%)' },
  danger: { bg: 'hsl(0 40% 24%)', border: 'hsl(0 70% 50%)', fg: 'hsl(0 90% 80%)' },
  important: { bg: 'hsl(265 30% 22%)', border: 'hsl(265 55% 55%)', fg: 'hsl(265 80% 80%)' },
  example: { bg: 'hsl(170 30% 22%)', border: 'hsl(170 50% 50%)', fg: 'hsl(170 70% 75%)' },
  question: { bg: 'hsl(180 30% 22%)', border: 'hsl(180 50% 50%)', fg: 'hsl(180 70% 75%)' },
  success: { bg: 'hsl(140 35% 20%)', border: 'hsl(140 60% 45%)', fg: 'hsl(140 80% 75%)' },
  failure: { bg: 'hsl(345 35% 24%)', border: 'hsl(345 70% 50%)', fg: 'hsl(345 85% 80%)' },
  bug: { bg: 'hsl(335 35% 24%)', border: 'hsl(335 70% 55%)', fg: 'hsl(335 85% 80%)' },
  quote: { bg: 'hsl(220 15% 18%)', border: 'hsl(220 15% 50%)', fg: 'hsl(220 20% 80%)' },
};

class CalloutWidget extends WidgetType {
  constructor(readonly block: CalloutBlock) {
    super();
  }

  toDOM(): HTMLElement {
    const colors = CALLOUT_COLORS[this.block.type];
    const root = document.createElement('div');
    root.setAttribute('data-callout', '');
    root.setAttribute('data-callout-type', this.block.type);
    root.style.background = colors.bg;
    root.style.borderLeft = `3px solid ${colors.border}`;
    root.style.borderRadius = '6px';
    root.style.padding = '10px 14px';
    root.style.margin = '4px 0';
    root.style.fontSize = '13px';
    root.style.lineHeight = '1.5';

    const titleRow = document.createElement('div');
    titleRow.style.display = 'flex';
    titleRow.style.alignItems = 'baseline';
    titleRow.style.gap = '6px';
    titleRow.style.fontWeight = '600';
    titleRow.style.color = colors.fg;
    titleRow.style.marginBottom = this.block.body.length > 0 ? '6px' : '0';

    const typePill = document.createElement('span');
    typePill.style.textTransform = 'uppercase';
    typePill.style.fontSize = '10.5px';
    typePill.style.letterSpacing = '0.05em';
    typePill.style.padding = '1px 6px';
    typePill.style.borderRadius = '3px';
    typePill.style.background = colors.border;
    typePill.style.color = 'hsl(0 0% 10%)';
    typePill.textContent = this.block.type;
    titleRow.appendChild(typePill);

    if (this.block.title) {
      const titleText = document.createElement('span');
      titleText.textContent = this.block.title;
      titleText.style.color = 'hsl(0 0% 95%)';
      titleRow.appendChild(titleText);
    }

    root.appendChild(titleRow);

    for (const line of this.block.body) {
      const bodyLine = document.createElement('div');
      bodyLine.textContent = line || ' ';
      bodyLine.style.color = 'hsl(0 0% 88%)';
      root.appendChild(bodyLine);
    }

    return root;
  }

  ignoreEvents(): boolean {
    return false;
  }

  eq(other: CalloutWidget): boolean {
    return (
      other.block.type === this.block.type &&
      other.block.title === this.block.title &&
      other.block.startLine === this.block.startLine &&
      other.block.endLine === this.block.endLine
    );
  }
}

class TransclusionWidget extends WidgetType {
  private readonly dataMap: Map<string, TransclusionData> | undefined;
  private readonly view: EditorView | null;

  constructor(
    readonly title: string,
    dataMap: Map<string, TransclusionData> | undefined,
    view: EditorView | null
  ) {
    super();
    this.dataMap = dataMap;
    this.view = view;
  }

  toDOM(): HTMLElement {
    const dataMap = this.dataMap;
    const data = dataMap?.get(this.title);
    const navigate = this.view ? (this.view as any).someProp as ((t: string) => void) | undefined : undefined;

    const el = document.createElement('span');
    el.setAttribute('data-transclusion', '');
    el.setAttribute('data-transclusion-title', this.title);
    el.setAttribute('data-testid', 'transclusion');
    el.style.display = 'inline-block';
    el.style.padding = '6px 10px';
    el.style.margin = '2px 0';
    el.style.borderLeft = '3px solid hsl(265 55% 65%)';
    el.style.background = 'hsl(265 25% 20%)';
    el.style.borderRadius = '4px';
    el.style.fontSize = '12px';
    el.style.lineHeight = '1.45';
    el.style.verticalAlign = 'middle';
    el.style.maxWidth = '100%';
    el.style.cursor = navigate ? 'pointer' : 'default';

    if (dataMap === undefined) {
      el.textContent = `Loading ${this.title}…`;
      el.style.opacity = '0.55';
    } else if (!data || !data.exists) {
      el.textContent = `Note not found: ${this.title}`;
      el.style.opacity = '0.55';
      el.style.fontStyle = 'italic';
    } else {
      const titleEl = document.createElement('div');
      titleEl.style.fontWeight = '600';
      titleEl.style.color = 'hsl(265 80% 80%)';
      titleEl.style.marginBottom = '2px';
      titleEl.textContent = data.title;

      const previewEl = document.createElement('div');
      previewEl.style.color = 'hsl(0 0% 80%)';
      previewEl.style.fontSize = '11.5px';
      const text = data.plainText || '';
      previewEl.textContent =
        text.length > 200 ? text.slice(0, 200) + '…' : text;

      el.appendChild(titleEl);
      el.appendChild(previewEl);
    }

    if (navigate) {
      el.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();
        navigate(this.title);
      });
    }

    return el;
  }

  ignoreEvents(): boolean {
    return true;
  }

  eq(other: TransclusionWidget): boolean {
    if (other.title !== this.title) return false;
    return this.dataMap === other.dataMap;
  }
}

function buildDecorations(
  state: EditorState,
  transclusionMap: Map<string, TransclusionData>,
  view: EditorView | null
): RangeSetBuilder<Decoration> {
  const ranges = parseInlineRanges(state.doc.toString());
  const builder = new RangeSetBuilder<Decoration>();
  const cursorLine = state.doc.lineAt(state.selection.main.head).number;
  for (const r of ranges) {
    const line = state.doc.lineAt(r.from).number;
    if (line === cursorLine) continue;
    if (r.kind === 'transclusion') {
      builder.add(
        r.from,
        r.to,
        Decoration.replace({
          widget: new TransclusionWidget(r.inner.text ?? '', transclusionMap, view),
          inclusive: false,
        })
      );
      continue;
    }
    builder.add(
      r.from,
      r.to,
      Decoration.replace({
        widget: new InlineWidget(r.kind, r.inner.text ?? '', r.href, view),
        inclusive: false,
      })
    );
  }
  const doc = state.doc;
  for (let i = 1; i <= doc.lines; i++) {
    const line = doc.line(i);
    const task = findTaskLine(line.text, 0);
    if (!task) continue;
    if (i === cursorLine) continue;
    const replaceTo = line.from + task.checkboxTo - task.lineStart + 1;
    builder.add(
      line.from,
      replaceTo,
      Decoration.replace({
        widget: new TaskCheckboxWidget(
          task.checked,
          {
            lineStart: line.from,
            checkboxFrom: line.from + (task.checkboxFrom - task.lineStart),
            checkboxTo: line.from + (task.checkboxTo - task.lineStart),
            checked: task.checked,
          },
          view
        ),
        inclusive: false,
      })
    );
  }
  return builder;
}

const allDecorationsExt = EditorView.decorations.compute(
  ['doc', 'selection'],
  (state) => {
    const transclusionMap = state.field(transclusionDataStateField);
    const view: EditorView | null = (state as any).doc?.view ?? null;
    const decorations = buildDecorations(state, transclusionMap, view);
    const callouts = parseCalloutBlocks(state.doc.toString());
    const cursorLine = state.doc.lineAt(state.selection.main.head).number;
    for (const c of callouts) {
      if (c.startLine === cursorLine) continue;
      const startLine = state.doc.line(c.startLine);
      const replaceFrom = startLine.from;
      const replaceTo =
        c.endLine < state.doc.lines
          ? state.doc.line(c.endLine + 1).from
          : state.doc.length;
      decorations.add(
        replaceFrom,
        replaceTo,
        Decoration.replace({
          widget: new CalloutWidget(c),
          block: true,
          inclusive: true,
        })
      );
    }
    return decorations.finish();
  }
);

const transclusionDataField = StateEffect.define<Map<string, TransclusionData>>();

const transclusionDataStateField = StateField.define<Map<string, TransclusionData>>({
  create() {
    return new Map();
  },
  update(value, tr) {
    return tr.effects.reduce((acc, eff) => {
      if (eff.is(transclusionDataField)) return eff.value;
      return acc;
    }, value);
  },
});

export function MarkdownEditor({
  content,
  onChange,
  onSave,
  onWikilinkNavigate,
  onWikilinkHover,
  onEditorReady,
  placeholder = "Start writing, or type [[ to link another page",
  className,
  autoFocus,
}: MarkdownEditorProps) {
  const ref = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const [showFindBar, setShowFindBar] = useState(false);
  const hoverRef = useRef(onWikilinkHover);
  hoverRef.current = onWikilinkHover;
  const editorReadyRef = useRef(onEditorReady);
  editorReadyRef.current = onEditorReady;
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
                validFor: /[\]a-zA-Z0-9 _-]*/,
              };
            },
          ],
        }),
        transclusionDataStateField,
        allDecorationsExt,
        blockDragHandleExtension,
        headingCollapseGutter,
        findHighlightPlugin,
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
              if (el.hasAttribute?.('data-transclusion')) {
                const title =
                  el.getAttribute('data-transclusion-title') || '';
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
          {
            key: 'Mod-f',
            preventDefault: true,
            run: () => {
              setShowFindBar(true);
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
    editorReadyRef.current?.(view);

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

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    (view as any).someProp = (title: string) => onWikilinkNavigate?.(title);
  }, [onWikilinkNavigate]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;

    const build = async () => {
      const map = new Map<string, TransclusionData>();
      for (const n of dbNodes ?? []) {
        const title = (n.title || '').trim();
        if (!title) continue;
        await fetchTransclusionData(title, 0, new Set<string>());
        map.set(title, {
          title,
          plainText: n.plainText ?? '',
          exists: true,
        });
      }
      view.dispatch({
        effects: transclusionDataField.of(map),
      });
    };

    build();
  }, [dbNodes]);

  const handleFindClose = useCallback(() => {
    setShowFindBar(false);
    viewRef.current?.dispatch({ effects: clearFindHighlights.of(true) });
  }, []);

  return (
    <div className="editor-find-wrapper" style={{ position: 'relative' }}>
      {showFindBar && viewRef.current && (
        <EditorFindBar view={viewRef.current} onClose={handleFindClose} />
      )}
      <div
        ref={ref}
        data-placeholder={placeholder}
        className={className}
      />
    </div>
  );
}
