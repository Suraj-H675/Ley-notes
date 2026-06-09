import { useEffect, useRef, useState, useCallback } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { EditorState, RangeSetBuilder, StateEffect } from '@codemirror/state';
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
import { parseCalloutBlocks, type CalloutBlock, type CalloutType } from './callout';
import { fetchTransclusionData } from '@/lib/markdown/transclusions';
import { db } from '@/lib/db';
import { blockDragHandleExtension } from './BlockDragHandle';
import { headingCollapseGutter } from './HeadingCollapse';
import { EditorFindBar, clearFindHighlights } from './EditorFindBar';

/**
 * Per-title data the transclusion widget needs. `null` means the target
 * note was not found. `undefined` means we haven't loaded it yet.
 */
export interface TransclusionData {
  title: string;
  plainText: string;
  exists: boolean;
}

/** State effect fired when the transclusion data map changes, so the
 * live-preview plugin rebuilds its decorations with the fresh data. */
const transclusionDataChanged = StateEffect.define<true>();

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

    // Title row
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

    // Body
    for (const line of this.block.body) {
      const bodyLine = document.createElement('div');
      bodyLine.textContent = line || ' ';
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
  // Captured at construction so eq() can detect when the data map changes.
  private readonly dataMapRef: Map<string, TransclusionData> | undefined;

  constructor(
    readonly title: string,
    readonly view: EditorView
  ) {
    super();
    this.dataMapRef = (view as any).transclusionData;
  }

  toDOM(): HTMLElement {
    const dataMap = this.dataMapRef;
    const data = dataMap?.get(this.title);
    const navigate = (this.view as any).someProp as
      | ((t: string) => void)
      | undefined;

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
      // The React layer hasn't pushed any data yet.
      el.textContent = `Loading ${this.title}…`;
      el.style.opacity = '0.55';
    } else if (!data || !data.exists) {
      // The data map is loaded but this title is missing → no such note.
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
    return true; // We handle the click ourselves
  }

  eq(other: TransclusionWidget): boolean {
    if (other.title !== this.title) return false;
    // Compare the dataMap captured at construction so a refresh after the
    // data effect fires triggers a DOM rebuild.
    return this.dataMapRef === other.dataMapRef;
  }
}

function buildDecorations(view: EditorView): DecorationSet {
  const ranges = parseInlineRanges(view.state.doc.toString());
  const builder = new RangeSetBuilder<Decoration>();
  const cursorLine = view.state.doc.lineAt(view.state.selection.main.head).number;
  for (const r of ranges) {
    const line = view.state.doc.lineAt(r.from).number;
    if (line === cursorLine) continue;
    if (r.kind === 'transclusion') {
      builder.add(
        r.from,
        r.to,
        Decoration.replace({
          widget: new TransclusionWidget(r.inner.text ?? '', view),
          inclusive: false,
        })
      );
      continue;
    }
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

/**
 * Block decorations (used for callouts) cannot be added via a ViewPlugin;
 * they need to be supplied by an EditorView.decorations.compute extension.
 */
const calloutDecorationExt = EditorView.decorations.compute(
  ['doc', 'selection'],
  (state) => {
    const callouts = parseCalloutBlocks(state.doc.toString());
    const cursorLine = state.doc.lineAt(state.selection.main.head).number;
    const builder = new RangeSetBuilder<Decoration>();
    for (const c of callouts) {
      if (c.startLine === cursorLine) continue;
      const startLine = state.doc.line(c.startLine);
      const replaceFrom = startLine.from;
      const replaceTo =
        c.endLine < state.doc.lines
          ? state.doc.line(c.endLine + 1).from
          : state.doc.length;
      builder.add(
        replaceFrom,
        replaceTo,
        Decoration.replace({
          widget: new CalloutWidget(c),
          block: true,
          inclusive: true,
        })
      );
    }
    return builder.finish();
  }
);

const livePreviewPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }
    update(update: ViewUpdate) {
      const dataChanged = update.transactions.some((t) =>
        t.effects.some((e) => e.is(transclusionDataChanged))
      );
      if (update.docChanged || update.selectionSet || dataChanged) {
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
  onEditorReady,
  placeholder = "Type '/' for commands, or '[[' to link another page",
  className,
  autoFocus,
}: MarkdownEditorProps) {
  const ref = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const [showFindBar, setShowFindBar] = useState(false);
  // Always-fresh refs to the latest callbacks/state. The wikilink widget
  // is created on decoration and reads from these so the closure doesn't
  // capture a stale value.
  const navigateRef = useRef(onWikilinkNavigate);
  navigateRef.current = onWikilinkNavigate;
  const hoverRef = useRef(onWikilinkHover);
  hoverRef.current = onWikilinkHover;
  const editorReadyRef = useRef(onEditorReady);
  editorReadyRef.current = onEditorReady;
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
        calloutDecorationExt,
        blockDragHandleExtension,
        headingCollapseGutter,
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

  // Sync onWikilinkNavigate into the editor so the widget can find it.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    (view as any).someProp = (title: string) => onWikilinkNavigate?.(title);
  }, [onWikilinkNavigate]);

  // Build the transclusion data map from live-queried nodes and push it
  // onto the editor view. Dispatch a StateEffect so the live-preview
  // plugin rebuilds decorations with the new data.
  //
  // Cycle guard: resolveAllTransclusions is called per-node with a fresh
  // visited set so that A→B→A is correctly detected (B's call finds A
  // already in the parent chain's visited set) and depth is capped at 5.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;

    const build = async () => {
      const map = new Map<string, TransclusionData>();
      for (const n of dbNodes ?? []) {
        const title = (n.title || '').trim();
        if (!title) continue;

        // Recursively resolve nested transclusions; each root call starts
        // its own visited set so sibling branches don't share visited state.
        await fetchTransclusionData(title, 0, new Set<string>());

        map.set(title, {
          title,
          plainText: n.plainText ?? '',
          exists: true,
        });
      }
      (view as any).transclusionData = map;
      view.dispatch({ effects: transclusionDataChanged.of(true) });
    };

    build();
  }, [dbNodes]);

  return (
    <div
      ref={ref}
      data-placeholder={placeholder}
      className={className}
    />
  );
}
