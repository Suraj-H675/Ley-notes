/**
 * Wiki-link CodeMirror extensions:
 *  - wikiLinkDecoration: visual styling + click-to-follow for [[...]] links.
 *  - wikiLinkAutocomplete: a lightweight [[ trigger that suggests page titles.
 *
 * These are NOT full syntax-tree decorations (which would require extending the
 * markdown parser). Instead we use view plugin decorations which recompute on
 * every doc change. For documents up to ~10k lines this is fast enough; if we
 * hit perf issues we move to a syntax-tree extension.
 *
 * Both extensions read the page index synchronously via the bridge in
 * @/core/vault/page-index.ts — populated by the React app via Dexie's
 * liveQuery.
 */

import {
  Decoration,
  EditorView,
  ViewPlugin,
  type DecorationSet,
} from '@codemirror/view';
import { RangeSetBuilder } from '@codemirror/state';
import type { Extension } from '@codemirror/state';

import { extractWikiLinks } from '@/core/parser/wiki-links';
import { resolveTitleSync, subscribePageIndex, getPageIndex } from '@/core/vault/page-index';

const WIKI_LINK_DECO = Decoration.mark({ class: 'cm-wikilink' });
const WIKI_LINK_GHOST = Decoration.mark({ class: 'cm-wikilink cm-wikilink-ghost' });

export function wikiLinkDecoration(): Extension {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      constructor(public view: EditorView) {
        this.decorations = rebuild(this.view);
      }
      update(update: import('@codemirror/view').ViewUpdate) {
        if (update.docChanged) this.decorations = rebuild(update.view);
      }
    },
    {
      decorations: (v) => v.decorations,
      eventHandlers: {
        click(e, view) {
          const pos = view.posAtCoords({ x: e.clientX, y: e.clientY });
          if (pos === null) return;
          const links = extractWikiLinks(view.state.doc.toString());
          for (const l of links) {
            if (l.isEmbed) continue;
            if (pos >= l.position && pos <= l.position + l.raw.length) {
              const target = resolveTitleSync(l.target) ?? l.target;
              view.contentDOM.dispatchEvent(
                new CustomEvent('ley:follow-link', {
                  detail: { target },
                  bubbles: true,
                }),
              );
              return;
            }
          }
        },
      },
    },
  );
}

function rebuild(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const links = extractWikiLinks(view.state.doc.toString());
  for (const l of links) {
    if (l.isEmbed) continue; // embeds handled in Phase 2
    const from = l.position;
    const to = l.position + l.raw.length;
    const resolved = resolveTitleSync(l.target);
    builder.add(from, to, resolved ? WIKI_LINK_DECO : WIKI_LINK_GHOST);
  }
  return builder.finish();
}

// ---------------------------------------------------------------------------
// Autocomplete panel.
// ---------------------------------------------------------------------------

export interface WikiLinkAutocompleteOptions {
  onSelect?: (target: string) => void;
  /** Override the panel host (useful in tests). */
  host?: HTMLElement;
}

export function wikiLinkAutocomplete(opts: WikiLinkAutocompleteOptions): Extension {
  let panel: HTMLDivElement | null = null;
  let anchor: { from: number; to: number } | null = null;
  let matches: Array<{ title: string }> = [];
  let selectedIndex = 0;
  let unsubscribeBridge: (() => void) | null = null;

  function ensurePanel(view: EditorView): HTMLDivElement {
    if (panel) return panel;
    const el = document.createElement('div');
    el.className = 'cm-wikilink-panel';
    el.style.position = 'absolute';
    el.style.zIndex = '50';
    el.style.minWidth = '180px';
    el.style.maxHeight = '240px';
    el.style.overflowY = 'auto';
    el.style.padding = '4px';
    el.style.borderRadius = '6px';
    el.style.boxShadow = 'var(--shadow-popover)';
    el.style.backgroundColor = 'hsl(var(--surface-2))';
    el.style.color = 'hsl(var(--foreground))';
    el.style.fontSize = '13px';
    el.style.fontFamily = 'var(--font-sans)';
    view.contentDOM.parentElement?.appendChild(el);
    panel = el;
    return el;
  }

  function hide() {
    if (panel) panel.style.display = 'none';
    anchor = null;
    matches = [];
    selectedIndex = 0;
  }

  function renderPanel(view: EditorView) {
    if (!anchor || matches.length === 0) {
      hide();
      return;
    }
    const el = ensurePanel(view);
    el.style.display = 'block';
    el.innerHTML = matches
      .map(
        (m, i) =>
          `<div class="cm-wikilink-item${i === selectedIndex ? ' is-selected' : ''}" data-idx="${i}">${escapeHtml(m.title)}</div>`,
      )
      .join('');

    const coords = view.coordsAtPos(anchor.from);
    if (coords) {
      const parentRect = view.contentDOM.parentElement!.getBoundingClientRect();
      el.style.left = `${coords.left - parentRect.left}px`;
      el.style.top = `${coords.bottom - parentRect.top + 4}px`;
    }

    el.onmousedown = (e) => {
      e.preventDefault();
      const t = e.target as HTMLElement;
      const idx = Number(t.dataset.idx);
      if (!Number.isNaN(idx) && matches[idx]) {
        accept(view, matches[idx].title);
      }
    };
  }

  function accept(view: EditorView, title: string) {
    if (!anchor) return;
    const insert = `${title}]]`;
    view.dispatch({
      changes: { from: anchor.from, to: view.state.selection.main.head, insert },
      selection: { anchor: anchor.from + insert.length },
    });
    hide();
    opts.onSelect?.(title);
  }

  function computeMatches(partial: string): Array<{ title: string }> {
    const q = partial.toLowerCase();
    if (!q) return [];
    const out: Array<{ title: string }> = [];
    for (const e of getPageIndex()) {
      if (
        e.lcTitle.includes(q) ||
        e.aliases.some((a) => a.toLowerCase().includes(q))
      ) {
        out.push({ title: e.title });
        if (out.length >= 8) break;
      }
    }
    return out;
  }

  return ViewPlugin.fromClass(
    class {
      constructor(public view: EditorView) {
        // Re-render the panel when the bridge refreshes (only if open).
        unsubscribeBridge = subscribePageIndex(() => {
          if (!anchor) return;
          const head = view.state.selection.main.head;
          const line = view.state.doc.lineAt(head);
          const before = view.state.doc.sliceString(line.from, head);
          const m = /\[\[([^[\]\n|#^]*)$/.exec(before);
          if (m) matches = computeMatches(m[1]);
          renderPanel(view);
        });
      }
      update(update: import('@codemirror/view').ViewUpdate) {
        const view = update.view;
        if (!update.docChanged && !update.selectionSet) return;
        const head = view.state.selection.main.head;
        const line = view.state.doc.lineAt(head);
        const before = view.state.doc.sliceString(line.from, head);
        const m = /\[\[([^[\]\n|#^]*)$/.exec(before);
        if (!m) {
          hide();
          return;
        }
        const partial = m[1];
        anchor = { from: head - partial.length, to: head };
        matches = computeMatches(partial);
        selectedIndex = Math.min(selectedIndex, Math.max(0, matches.length - 1));
        renderPanel(view);
      }
      destroy() {
        if (panel) panel.remove();
        panel = null;
        unsubscribeBridge?.();
        unsubscribeBridge = null;
      }
    },
    {
      eventHandlers: {
        keydown(e, view) {
          if (!anchor || matches.length === 0) return false;
          if (e.key === 'Escape') {
            e.preventDefault();
            hide();
            return true;
          }
          if (e.key === 'ArrowDown') {
            e.preventDefault();
            selectedIndex = (selectedIndex + 1) % matches.length;
            renderPanel(view);
            return true;
          }
          if (e.key === 'ArrowUp') {
            e.preventDefault();
            selectedIndex = (selectedIndex - 1 + matches.length) % matches.length;
            renderPanel(view);
            return true;
          }
          if (e.key === 'Enter' || e.key === 'Tab') {
            e.preventDefault();
            accept(view, matches[selectedIndex].title);
            return true;
          }
          return false;
        },
      },
    },
  );
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}