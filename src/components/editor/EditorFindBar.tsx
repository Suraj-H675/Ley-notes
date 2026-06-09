import { useEffect, useRef, useState, useCallback } from 'react';
import { Decoration, DecorationSet, EditorView, ViewPlugin, ViewUpdate } from '@codemirror/view';
import { RangeSetBuilder, StateEffect } from '@codemirror/state';

const findHighlightMark = Decoration.mark({ class: 'cm-find-match' });
const findActiveMark = Decoration.mark({ class: 'cm-find-match cm-find-match-active' });

/** State effect to clear all find highlights */
export const clearFindHighlights = StateEffect.define<true>();

/** Find state stored in plugin instance */
interface FindState {
  matches: Array<{ from: number; to: number }>;
  activeIndex: number;
  query: string;
}

export const findHighlightPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    findState: FindState | null;

    constructor(_view: EditorView) {
      this.decorations = Decoration.none;
      this.findState = null;
    }

    update(update: ViewUpdate) {
      // Clear on doc changes
      if (update.docChanged && this.findState) {
        this.findState = null;
        this.decorations = Decoration.none;
      }
      // Process effects
      for (const e of update.transactions) {
        for (const effect of e.effects) {
          if (effect.is(clearFindHighlights)) {
            this.findState = null;
            this.decorations = Decoration.none;
          }
        }
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);

/** Update find highlights based on current find state */
function applyFindHighlights(
  view: EditorView,
  matches: Array<{ from: number; to: number }>,
  activeIndex: number
) {
  if (matches.length === 0) {
    view.dispatch({ effects: clearFindHighlights.of(true) });
    return;
  }

  const builder = new RangeSetBuilder<Decoration>();
  for (let i = 0; i < matches.length; i++) {
    const m = matches[i];
    if (i === activeIndex) {
      builder.add(m.from, m.to, findActiveMark);
    } else {
      builder.add(m.from, m.to, findHighlightMark);
    }
  }
  const decos = builder.finish();
  view.dispatch({
    effects: clearFindHighlights.of(true),
  });
  // Directly set decorations via plugin instance
  const plugin = view.plugin(findHighlightPlugin);
  if (plugin) {
    (plugin as any).decorations = decos;
    view.requestMeasure();
  }
}

/** Find all matches of a query in document text */
export function findAllMatches(doc: string, query: string): Array<{ from: number; to: number }> {
  if (!query) return [];
  const matches: Array<{ from: number; to: number }> = [];
  try {
    const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(escaped, 'gi');
    let match: RegExpExecArray | null;
    while ((match = regex.exec(doc)) !== null) {
      matches.push({ from: match.index, to: match.index + match[0].length });
      if (match[0].length === 0) {
        regex.lastIndex++;
      }
    }
  } catch {
    // Invalid regex
  }
  return matches;
}

/** Navigate to a specific match */
export function navigateToMatch(
  view: EditorView,
  matches: Array<{ from: number; to: number }>,
  index: number
) {
  if (matches.length === 0) return;
  const clampedIndex = Math.max(0, Math.min(index, matches.length - 1));
  const match = matches[clampedIndex];
  view.dispatch({
    selection: { anchor: match.from, head: match.to },
    scrollIntoView: true,
  });
}

export function EditorFindBar({
  view,
  onClose,
}: {
  view: EditorView;
  onClose: () => void;
}) {
  const [query, setQuery] = useState('');
  const [matches, setMatches] = useState<Array<{ from: number; to: number }>>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const matchesRef = useRef(matches);
  matchesRef.current = matches;
  const activeIndexRef = useRef(activeIndex);
  activeIndexRef.current = activeIndex;

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const goToNext = useCallback(() => {
    const currentMatches = matchesRef.current;
    if (currentMatches.length === 0) return;
    const nextIndex = (activeIndexRef.current + 1) % currentMatches.length;
    setActiveIndex(nextIndex);
    navigateToMatch(view, currentMatches, nextIndex);
    applyFindHighlights(view, currentMatches, nextIndex);
  }, [view]);

  const goToPrev = useCallback(() => {
    const currentMatches = matchesRef.current;
    if (currentMatches.length === 0) return;
    const prevIndex =
      (activeIndexRef.current - 1 + currentMatches.length) % currentMatches.length;
    setActiveIndex(prevIndex);
    navigateToMatch(view, currentMatches, prevIndex);
    applyFindHighlights(view, currentMatches, prevIndex);
  }, [view]);

  const handleClose = useCallback(() => {
    view.dispatch({ effects: clearFindHighlights.of(true) });
    onClose();
  }, [view, onClose]);

  // Handle input change with debounce
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setQuery(value);

    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      const doc = view.state.doc.toString();
      const found = findAllMatches(doc, value);
      setMatches(found);
      setActiveIndex(0);
      if (found.length > 0) {
        navigateToMatch(view, found, 0);
      }
      applyFindHighlights(view, found, 0);
    }, 150);
  };

  // Keyboard handling
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      handleClose();
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) {
        goToPrev();
      } else {
        goToNext();
      }
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      goToNext();
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      goToPrev();
      return;
    }
  };

  const matchCount = matches.length;
  const currentMatch = matchCount > 0 ? activeIndex + 1 : 0;

  return (
    <div className="editor-find-bar" onKeyDown={handleKeyDown}>
      <div className="editor-find-bar-inner">
        <input
          ref={inputRef}
          type="text"
          className="editor-find-input"
          placeholder="Find in note…"
          value={query}
          onChange={handleInput}
          aria-label="Find in note"
        />
        <span className="editor-find-counter" aria-live="polite">
          {matchCount > 0 ? `${currentMatch} of ${matchCount}` : query ? 'No matches' : ''}
        </span>
        <button
          className="editor-find-btn"
          onClick={goToPrev}
          disabled={matchCount === 0}
          title="Previous match (Shift+Enter)"
          aria-label="Previous match"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M7 3L3 7L7 11" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
        <button
          className="editor-find-btn"
          onClick={goToNext}
          disabled={matchCount === 0}
          title="Next match (Enter)"
          aria-label="Next match"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M7 3L11 7L7 11" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
        <button
          className="editor-find-btn editor-find-close"
          onClick={handleClose}
          title="Close (Escape)"
          aria-label="Close find bar"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M3 3L11 11M11 3L3 11" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </button>
      </div>
    </div>
  );
}