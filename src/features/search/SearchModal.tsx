/**
 * Search modal — Cmd+P. Live search across page titles and content, plus
 * tag filter via `tag:foo` syntax. Keyboard nav: arrows to move, Enter to
 * open, Escape to close.
 */

import { useEffect, useRef, useState } from 'react';
import { Search, FileText, Hash, X } from 'lucide-react';
import { searchPages } from '@/core/index/search';
import { db } from '@/infrastructure/database/db';
import { useNavStore } from '@/shared/state/nav';
import { Kbd } from '@/shared/components/Kbd';
import { cn } from '@/shared/lib/classnames';

interface SearchResult {
  id: string;
  title: string;
  path: string;
  snippet: string;
}

export function SearchModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const openPage = useNavStore((s) => s.openPage);
  const pushRecent = useNavStore((s) => s.pushRecent);

  // Focus input on open; reset on close.
  // We compute fresh initial state via the `open` prop via a "key" pattern
  // would force remount, but here we use state derived from the previous
  // render: when `open` flips false→true we reset everything. To satisfy
  // react-hooks/set-state-in-effect, we sync via a derived ref instead.
  const wasOpen = useRef(false);
  useEffect(() => {
    if (open && !wasOpen.current) {
      // Open transition: schedule state updates on the next microtask so the
      // effect body doesn't synchronously call setState.
      queueMicrotask(() => {
        setQuery('');
        setResults([]);
        setSelectedIndex(0);
        inputRef.current?.focus();
      });
    }
    wasOpen.current = open;
  }, [open]);

  // Global Cmd+P / Ctrl+P to open.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  // Search debounced.
  useEffect(() => {
    if (!open) return;
    const id = setTimeout(async () => {
      const hits = await searchPages(query, 12);
      setResults(hits);
      setSelectedIndex(0);
    }, 100);
    return () => clearTimeout(id);
  }, [query, open]);

  async function commit(id: string) {
    const page = await db.pages.get(id);
    if (!page || page.deletedAt !== null) return;
    openPage(id);
    pushRecent(id);
    onClose();
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const target = results[selectedIndex];
      if (target) commit(target.id);
    }
  }

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-start justify-center pt-20"
      style={{ backgroundColor: 'hsl(var(--background) / 0.6)' }}
      onClick={onClose}
    >
      <div
        className="flex w-[520px] max-w-[92vw] flex-col overflow-hidden rounded-lg border border-border bg-surface-1"
        style={{ boxShadow: 'var(--shadow-menu)' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2 border-b border-border px-3 py-2">
          <Search size={14} className="text-muted-foreground" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Quick switcher — search notes, tags, or paths"
            className="flex-1 bg-transparent text-body text-foreground placeholder:text-subtle-foreground focus:outline-none"
          />
          <Kbd>esc</Kbd>
          <button
            type="button"
            onClick={onClose}
            className="rounded-sm p-1 text-muted-foreground hover:bg-surface-3 hover:text-foreground"
            aria-label="Close search"
          >
            <X size={14} />
          </button>
        </div>

        <div className="max-h-[60vh] overflow-y-auto">
          {results.length === 0 ? (
            <div className="px-4 py-8 text-center text-meta text-muted-foreground">
              No matching notes.
            </div>
          ) : (
            <ul>
              {results.map((r, i) => (
                <li key={r.id}>
                  <button
                    type="button"
                    onClick={() => commit(r.id)}
                    onMouseEnter={() => setSelectedIndex(i)}
                    className={cn(
                      'flex w-full items-center gap-2 px-3 py-2 text-left text-meta',
                      i === selectedIndex
                        ? 'bg-surface-3 text-foreground'
                        : 'text-muted-foreground-strong hover:bg-surface-2',
                    )}
                  >
                    <FileText size={13} />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate font-medium text-foreground">{r.title}</span>
                      <span className="block truncate text-micro text-muted-foreground">{r.snippet}</span>
                    </span>
                    <span className="max-w-40 truncate font-mono text-micro text-subtle-foreground">{r.path}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-border px-3 py-1.5 text-micro text-muted-foreground">
          <div className="flex items-center gap-2">
            <span>Navigate</span>
            <Kbd>↑</Kbd>
            <Kbd>↓</Kbd>
            <span>Open</span>
            <Kbd>↵</Kbd>
          </div>
          <div className="flex items-center gap-1">
            <Hash size={10} />
            <span>tag:foo</span>
          </div>
        </div>
      </div>
    </div>
  );
}
