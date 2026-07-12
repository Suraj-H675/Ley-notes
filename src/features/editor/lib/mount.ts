/**
 * CodeMirror editor factory. Returns a tiny controller with imperative methods
 * (get value, set value, focus, destroy) and a few callbacks (onChange).
 *
 * The set of extensions is layered:
 *  1. base — lineNumbers, history, drawSelection, keymap
 *  2. markdown mode — @codemirror/lang-markdown
 *  3. theme — obsidian-like dark/light
 *  4. wiki-links — decoration + autocomplete
 *  5. frontmatter fold (added by extensions/frontmatter.ts)
 *
 * This factory intentionally takes plain arguments rather than a full React
 * component so it can be reused in previews, the search modal, etc.
 */

import { EditorState } from '@codemirror/state';
import { EditorView, keymap, highlightActiveLine, drawSelection } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import {
  syntaxHighlighting,
  defaultHighlightStyle,
  indentOnInput,
  bracketMatching,
  HighlightStyle,
} from '@codemirror/language';
import { tags as t } from '@lezer/highlight';
import { highlightSelectionMatches, openSearchPanel, search, searchKeymap } from '@codemirror/search';

import { wikiLinkDecoration, wikiLinkAutocomplete } from './extensions/wiki-links';
import { applyEditorFormat, editorFormattingKeymap, type EditorFormat } from './formatting';

export interface MountOptions {
  initialDoc: string;
  onChange: (value: string) => void;
  /** When true, hide line numbers (used in compact cards). */
  compact?: boolean;
}

export interface EditorController {
  view: EditorView;
  getValue: () => string;
  setValue: (value: string) => void;
  insertText: (value: string) => void;
  format: (format: EditorFormat) => void;
  openSearch: () => void;
  focus: () => void;
  destroy: () => void;
}

/**
 * Obsidian-inspired CM6 theme: muted foreground on subtle bg, accent on cursor line.
 * Color values read from CSS variables so theme switching is reactive.
 */
const cmTheme = EditorView.theme({
  '&': {
    height: '100%',
    fontSize: '14px',
    backgroundColor: 'transparent',
    color: 'hsl(var(--foreground))',
  },
  '.cm-content': {
    fontFamily: 'var(--font-sans)',
    caretColor: 'hsl(var(--foreground))',
    padding: '24px 0 120px',
  },
  '.cm-line': {
    padding: '0 16px',
  },
  '.cm-gutters': {
    backgroundColor: 'transparent',
    color: 'hsl(var(--subtle-foreground))',
    border: 'none',
  },
  '.cm-activeLineGutter': {
    backgroundColor: 'transparent',
    color: 'hsl(var(--muted-foreground-strong))',
  },
  '.cm-activeLine': {
    backgroundColor: 'hsl(var(--surface-1) / 0.4)',
  },
  '.cm-cursor': {
    borderLeftColor: 'hsl(var(--foreground))',
    borderLeftWidth: '2px',
  },
  '.cm-selectionBackground, ::selection': {
    backgroundColor: 'hsl(var(--primary) / 0.25)',
  },
  '&.cm-focused .cm-selectionBackground': {
    backgroundColor: 'hsl(var(--primary) / 0.3)',
  },
  '.cm-tooltip-autocomplete': {
    overflow: 'hidden',
    border: '1px solid hsl(var(--border))',
    borderRadius: '8px',
    backgroundColor: 'hsl(var(--surface-2))',
    color: 'hsl(var(--foreground))',
    boxShadow: 'var(--shadow-popover)',
    fontFamily: 'var(--font-sans)',
  },
  '.cm-tooltip-autocomplete > ul': {
    maxHeight: '240px',
    padding: '4px',
  },
  '.cm-tooltip-autocomplete > ul > li': {
    borderRadius: '5px',
    padding: '5px 8px',
    fontSize: '13px',
  },
  '.cm-tooltip-autocomplete > ul > li[aria-selected]': {
    backgroundColor: 'hsl(var(--surface-3))',
    color: 'hsl(var(--foreground))',
  },
  '.cm-panels-top': {
    borderBottom: '1px solid hsl(var(--border))',
  },
  // CodeMirror defaults sticky panels to z-index 300. Keep them inside the
  // editor's stacking layer so Ley's mobile sidebar remains a true overlay.
  '.cm-panels': {
    zIndex: '1 !important',
  },
  '.cm-panel.cm-search': {
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    gap: '6px',
    padding: '10px 38px 10px 12px',
    backgroundColor: 'hsl(var(--surface-1))',
    color: 'hsl(var(--foreground))',
    fontFamily: 'var(--font-sans)',
  },
  '.cm-panel.cm-search br': {
    flexBasis: '100%',
    height: '0',
  },
  '.cm-panel.cm-search .cm-textfield': {
    height: '30px',
    minWidth: '120px',
    flex: '1 1 220px',
    margin: '0',
    border: '1px solid hsl(var(--border))',
    borderRadius: '6px',
    backgroundColor: 'hsl(var(--background))',
    color: 'hsl(var(--foreground))',
    padding: '0 9px',
    fontFamily: 'var(--font-sans)',
    fontSize: '12px',
    outline: 'none',
  },
  '.cm-panel.cm-search .cm-textfield:focus': {
    borderColor: 'hsl(var(--primary))',
    boxShadow: '0 0 0 2px hsl(var(--primary) / 0.15)',
  },
  '.cm-panel.cm-search .cm-button': {
    height: '28px',
    margin: '0',
    border: '1px solid hsl(var(--border))',
    borderRadius: '6px',
    backgroundImage: 'none',
    backgroundColor: 'hsl(var(--surface-2))',
    color: 'hsl(var(--foreground))',
    padding: '0 9px',
    fontFamily: 'var(--font-sans)',
    fontSize: '11px',
    textTransform: 'capitalize',
  },
  '.cm-panel.cm-search .cm-button:hover': {
    backgroundColor: 'hsl(var(--surface-3))',
  },
  '.cm-panel.cm-search label': {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '3px',
    margin: '0 2px 0 0',
    color: 'hsl(var(--muted-foreground))',
    fontSize: '10px',
  },
  '.cm-panel.cm-search input[type=checkbox]': {
    margin: '0',
    accentColor: 'hsl(var(--primary))',
  },
  '.cm-panel.cm-search [name=close]': {
    top: '9px',
    right: '11px',
    width: '24px',
    height: '24px',
    borderRadius: '5px',
    color: 'hsl(var(--muted-foreground))',
    fontSize: '18px',
    lineHeight: '20px',
  },
  '.cm-panel.cm-search [name=close]:hover': {
    backgroundColor: 'hsl(var(--surface-3))',
    color: 'hsl(var(--foreground))',
  },
  '.cm-searchMatch': {
    backgroundColor: 'hsl(var(--secondary) / 0.28)',
    outline: '1px solid hsl(var(--secondary) / 0.35)',
  },
  '.cm-searchMatch-selected': {
    backgroundColor: 'hsl(var(--primary) / 0.38)',
    outline: '1px solid hsl(var(--primary) / 0.65)',
  },
});

/**
 * Wiki-link and embed visual treatment: secondary color, underline on hover.
 */
const cmHighlight = HighlightStyle.define([
  {
    tag: t.link,
    color: 'hsl(var(--secondary))',
    textDecoration: 'underline',
  },
  {
    tag: t.heading,
    color: 'hsl(var(--foreground))',
    fontWeight: '600',
  },
  {
    tag: t.strong,
    fontWeight: '600',
  },
  {
    tag: t.emphasis,
    fontStyle: 'italic',
  },
  {
    tag: t.monospace,
    color: 'hsl(var(--secondary-foreground))',
    backgroundColor: 'hsl(var(--muted))',
    borderRadius: '3px',
    padding: '0 3px',
  },
]);

export function mountEditor(parent: HTMLElement, opts: MountOptions): EditorController {
  const state = EditorState.create({
    doc: opts.initialDoc,
    extensions: [
      history(),
      drawSelection(),
      highlightActiveLine(),
      indentOnInput(),
      bracketMatching(),
      EditorView.lineWrapping,
      search({ top: true }),
      highlightSelectionMatches({ highlightWordAroundCursor: true }),
      keymap.of([...editorFormattingKeymap(), ...searchKeymap.filter((binding) => binding.key !== 'Mod-d'), ...defaultKeymap, ...historyKeymap, indentWithTab]),
      markdown(),
      syntaxHighlighting(cmHighlight),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      cmTheme,
      wikiLinkDecoration(),
      wikiLinkAutocomplete(),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) opts.onChange(update.state.doc.toString());
      }),
    ],
  });

  const view = new EditorView({ state, parent });

  return {
    view,
    getValue: () => view.state.doc.toString(),
    setValue: (value) => {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value },
      });
    },
    insertText: (value) => {
      const selection = view.state.selection.main;
      view.dispatch({
        changes: { from: selection.from, to: selection.to, insert: value },
        selection: { anchor: selection.from + value.length },
      });
      view.focus();
    },
    format: (format) => { applyEditorFormat(view, format); },
    openSearch: () => { openSearchPanel(view); },
    focus: () => view.focus(),
    destroy: () => view.destroy(),
  };
}
