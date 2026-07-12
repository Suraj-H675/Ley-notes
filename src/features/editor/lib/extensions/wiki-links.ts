/** Wiki-link decorations, modifier-click navigation, and vault completion. */

import { autocompletion, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete';
import { RangeSetBuilder, type Extension } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, type DecorationSet } from '@codemirror/view';
import { extractWikiLinks } from '@/core/parser/wiki-links';
import { getPageIndex, resolveTitleSync } from '@/core/vault/page-index';

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
      decorations: (plugin) => plugin.decorations,
      eventHandlers: {
        click(event, view) {
          // Editing stays predictable: a normal click positions the cursor;
          // Ctrl/Cmd-click follows the link, matching desktop note editors.
          if (!event.metaKey && !event.ctrlKey) return;
          const position = view.posAtCoords({ x: event.clientX, y: event.clientY });
          if (position === null) return;
          const link = extractWikiLinks(view.state.doc.toString()).find((candidate) =>
            !candidate.isEmbed
            && position >= candidate.position
            && position <= candidate.position + candidate.raw.length,
          );
          if (!link) return;
          view.contentDOM.dispatchEvent(new CustomEvent('ley:follow-link', {
            detail: { target: link.target },
            bubbles: true,
          }));
        },
      },
    },
  );
}

function rebuild(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  for (const link of extractWikiLinks(view.state.doc.toString())) {
    if (link.isEmbed) continue;
    const decoration = resolveTitleSync(link.target) ? WIKI_LINK_DECO : WIKI_LINK_GHOST;
    builder.add(link.position, link.position + link.raw.length, decoration);
  }
  return builder.finish();
}

// CodeMirror's official completion lifecycle owns positioning, keyboard
// precedence, screen-reader semantics, and dismissal. Ley only supplies the
// vault-specific trigger and options.
export function wikiLinkAutocomplete(): Extension {
  return autocompletion({
    override: [wikiLinkCompletions],
    activateOnTyping: true,
    icons: false,
    maxRenderedOptions: 8,
  });
}

function wikiLinkCompletions(context: CompletionContext): CompletionResult | null {
  const line = context.state.doc.lineAt(context.pos);
  const before = context.state.doc.sliceString(line.from, context.pos);
  const match = /\[\[([^[\]\n|#^]*)$/.exec(before);
  if (!match) return null;

  const query = match[1].toLowerCase();
  const options = getPageIndex()
    .filter((entry) => !query || entry.lcTitle.includes(query) || entry.aliases.some((alias) => alias.toLowerCase().includes(query)))
    .sort((left, right) => relevance(left, query) - relevance(right, query) || right.updatedAt - left.updatedAt || left.title.localeCompare(right.title))
    .slice(0, 50)
    .map((entry) => ({
      label: entry.title,
      detail: entry.aliases.length > 0 ? entry.aliases.join(', ') : undefined,
      apply: `${entry.title}]]`,
      type: 'text',
    }));

  return {
    from: context.pos - match[1].length,
    options,
    validFor: /^[^[\]\n|#^]*$/,
  };
}

function relevance(entry: { lcTitle: string; aliases: string[] }, query: string): number {
  if (!query || entry.lcTitle === query) return 0;
  if (entry.lcTitle.startsWith(query)) return 1;
  if (entry.aliases.some((alias) => alias.toLowerCase().startsWith(query))) return 2;
  return 3;
}
