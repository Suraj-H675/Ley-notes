/**
 * Quick switcher — live search across note text and structured vault fields.
 * Keyboard navigation uses arrows, Enter opens in the focused pane,
 * Shift+Enter opens in split, and Escape closes.
 */

import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Search, FileText, Hash, X, Columns2, Folder, HelpCircle, ListFilter, Tags } from 'lucide-react';
import { searchPages } from '@/core/index/search';
import { db } from '@/infrastructure/database/db';
import { useNavStore } from '@/shared/state/nav';
import { Kbd } from '@/shared/components/Kbd';
import { cn } from '@/shared/lib/classnames';
import * as Dialog from '@radix-ui/react-dialog';

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
  const [syntaxOpen, setSyntaxOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const openPage = useNavStore((s) => s.openPage);
  const openInSplit = useNavStore((s) => s.openInSplit);
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
        setSyntaxOpen(false);
        inputRef.current?.focus();
      });
    }
    wasOpen.current = open;
  }, [open]);

  // Search debounced.
  useEffect(() => {
    if (!open) return;
    let current = true;
    const id = setTimeout(async () => {
      const hits = await searchPages(query, 12);
      if (!current) return;
      setResults(hits);
      setSelectedIndex(0);
    }, 100);
    return () => {
      current = false;
      clearTimeout(id);
    };
  }, [query, open]);

  async function commit(id: string, split = false) {
    const page = await db.pages.get(id);
    if (!page || page.deletedAt !== null) return;
    if (split) openInSplit(id);
    else openPage(id);
    pushRecent(id);
    onClose();
  }

  function addFilter(filter: string) {
    setQuery((current) => `${current.trim()}${current.trim() ? ' ' : ''}${filter}`);
    queueMicrotask(() => inputRef.current?.focus());
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
      if (target) commit(target.id, e.shiftKey);
    }
  }

  if (!open) return null;

  return (
    <Dialog.Root open={open} onOpenChange={(next) => { if (!next) onClose(); }}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-50 bg-background/60 backdrop-blur-sm" />
        <Dialog.Content aria-describedby={undefined} className="fixed left-1/2 top-20 z-[51] flex w-[520px] max-w-[92vw] -translate-x-1/2 flex-col overflow-hidden rounded-lg border border-border bg-surface-1 shadow-menu outline-none">
        <Dialog.Title className="sr-only">Open a note</Dialog.Title>
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
          <Dialog.Close
            className="rounded-sm p-1 text-muted-foreground hover:bg-surface-3 hover:text-foreground"
            aria-label="Close search"
          >
            <X size={14} />
          </Dialog.Close>
        </div>
        <div className="flex gap-1 overflow-x-auto border-b border-border bg-surface-2/50 px-3 py-1.5" aria-label="Search filters">
          <FilterChip icon={<Tags size={11} />} label="Tag" title="Add tag:work" onClick={() => addFilter('tag:')} />
          <FilterChip icon={<Folder size={11} />} label="Path" title={'Add path:"Project Alpha"'} onClick={() => addFilter('path:')} />
          <FilterChip icon={<FileText size={11} />} label="Title" title="Add title:roadmap" onClick={() => addFilter('title:')} />
          <FilterChip icon={<ListFilter size={11} />} label="Property" title="Add property:status=active" onClick={() => addFilter('property:')} />
          <FilterChip icon={<X size={11} />} label="Exclude" title="Prefix any filter with - to exclude it" onClick={() => addFilter('-tag:')} />
          <button type="button" onClick={() => setSyntaxOpen((value) => !value)} className="ml-auto flex shrink-0 items-center gap-1 rounded-md px-2 py-1 text-micro text-muted-foreground hover:bg-surface-1 hover:text-foreground" aria-expanded={syntaxOpen} aria-controls="search-syntax"><HelpCircle size={11} />Syntax</button>
        </div>
        {syntaxOpen && <div id="search-syntax" role="note" className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 border-b border-border bg-surface-1 px-4 py-3 text-micro text-muted-foreground">
          <code className="text-secondary">tag:work</code><span>Matches that tag and nested tags.</span>
          <code className="text-secondary">path:&quot;Project Alpha&quot;</code><span>Quotes preserve spaces.</span>
          <code className="text-secondary">property:status=active</code><span>Matches YAML property values; <code>[status:active]</code> also works.</span>
          <code className="text-secondary">-tag:archive</code><span>Prefix any filter with <code>-</code> to exclude it. Filters combine with AND.</span>
        </div>}

        <div className="max-h-[60vh] overflow-y-auto">
          {results.length === 0 ? (
            <div className="px-4 py-8 text-center text-meta text-muted-foreground">
              No matching notes.
            </div>
          ) : (
            <ul>
              {results.map((r, i) => (
                <li key={r.id}>
                  <div
                    onMouseEnter={() => setSelectedIndex(i)}
                    className={cn(
                      'group flex items-center text-meta',
                      i === selectedIndex
                        ? 'bg-surface-3 text-foreground'
                        : 'text-muted-foreground-strong hover:bg-surface-2',
                    )}
                  >
                    <button type="button" onClick={() => commit(r.id)} className="flex min-w-0 flex-1 items-center gap-2 px-3 py-2 text-left">
                      <FileText size={13} className="shrink-0" />
                      <span className="min-w-0 flex-1">
                        <span className="block truncate font-medium text-foreground">{r.title}</span>
                        <span className="block truncate text-micro text-muted-foreground">{r.snippet}</span>
                      </span>
                      <span className="hidden max-w-40 truncate font-mono text-micro text-subtle-foreground sm:block">{r.path}</span>
                    </button>
                    <button type="button" onClick={() => commit(r.id, true)} className="mr-2 rounded p-1.5 text-muted-foreground opacity-70 hover:bg-surface-1 hover:text-foreground sm:opacity-0 sm:group-hover:opacity-100 sm:focus:opacity-100" aria-label={`Open ${r.title} in split`} title="Open in split (Shift+Enter)"><Columns2 size={13} /></button>
                  </div>
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
            <span className="hidden sm:inline">Split</span>
            <span className="hidden sm:flex"><Kbd>⇧↵</Kbd></span>
          </div>
          <div className="flex items-center gap-1">
            <Hash size={10} />
            <span>Quotes and - exclude</span>
          </div>
        </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function FilterChip({ icon, label, title, onClick }: { icon: ReactNode; label: string; title: string; onClick: () => void }) {
  return <button type="button" onClick={onClick} title={title} className="flex shrink-0 items-center gap-1 rounded-md border border-border bg-surface-1 px-2 py-1 text-micro text-muted-foreground hover:border-primary/30 hover:text-foreground">{icon}{label}</button>;
}
