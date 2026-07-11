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
import { EditorView, keymap, lineNumbers, highlightActiveLine, drawSelection } from '@codemirror/view';
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

import { wikiLinkDecoration, wikiLinkAutocomplete } from './extensions/wiki-links';

export interface MountOptions {
  initialDoc: string;
  onChange: (value: string) => void;
  onWikiLinkFollow?: (target: string) => void;
  /** When true, hide line numbers (used in compact cards). */
  compact?: boolean;
}

export interface EditorController {
  view: EditorView;
  getValue: () => string;
  setValue: (value: string) => void;
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
    padding: '12px 0',
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
      lineNumbers(),
      history(),
      drawSelection(),
      highlightActiveLine(),
      indentOnInput(),
      bracketMatching(),
      EditorView.lineWrapping,
      keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
      markdown(),
      syntaxHighlighting(cmHighlight),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      cmTheme,
      wikiLinkDecoration(),
      wikiLinkAutocomplete({
        onSelect: (target) => opts.onWikiLinkFollow?.(target),
      }),
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
    focus: () => view.focus(),
    destroy: () => view.destroy(),
  };
}