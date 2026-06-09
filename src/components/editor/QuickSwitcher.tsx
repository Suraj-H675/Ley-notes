import { useEffect, useRef, useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { Search } from 'lucide-react';
import { useSearchStore } from '@/store';
import { db } from '@/lib/db';
import { formatRelative } from '@/lib/utils/date';
import { cn } from '@/lib/utils';

const MAX_RESULTS = 8;

export function QuickSwitcher() {
  const navigate = useNavigate();
  const { quickSwitcherOpen, closeQuickSwitcher } = useSearchStore();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Fetch non-archived nodes sorted by updatedAt (most recent first)
  const allNodes = useLiveQuery(
    () => db.nodes.where('isArchived').equals(0).reverse().sortBy('updatedAt'),
    []
  );

  // Filter nodes by query (case-insensitive title match)
  const results = allNodes
    ? allNodes
        .filter((n) =>
          query.trim() === ''
            ? true
            : n.title.toLowerCase().includes(query.toLowerCase())
        )
        .slice(0, MAX_RESULTS)
    : [];

  // Reset state when opening
  useEffect(() => {
    if (quickSwitcherOpen) {
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [quickSwitcherOpen]);

  // Reset selected index when results change
  useEffect(() => {
    setSelectedIndex(0);
  }, [results.length]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        closeQuickSwitcher();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
        return;
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        const node = results[selectedIndex];
        if (node) {
          navigate(`/page/${node.id}`);
          closeQuickSwitcher();
        }
        return;
      }
    },
    [results, selectedIndex, navigate, closeQuickSwitcher]
  );

  if (!quickSwitcherOpen) return null;

  return (
    <div className="fixed inset-0 z-50">
      <div
        className="absolute inset-0 bg-background/40 backdrop-blur-sm"
        onClick={closeQuickSwitcher}
      />
      <div className="absolute left-1/2 top-[20%] w-full max-w-md -translate-x-1/2 animate-slide-down">
        <div className="overflow-hidden rounded-lg border border-border/80 bg-popover shadow-menu">
          {/* Search input */}
          <div className="flex items-center gap-2 border-b border-border/60 px-3 py-3">
            <Search className="h-4 w-4 flex-shrink-0 text-muted-foreground/60" />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Search pages..."
              className="h-5 w-full bg-transparent text-[14px] text-foreground outline-none placeholder:text-muted-foreground/60"
            />
            <kbd className="rounded border border-border/60 bg-background/40 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground/70">
              esc
            </kbd>
          </div>

          {/* Results list */}
          <div className="max-h-80 overflow-y-auto py-1.5">
            {results.length === 0 ? (
              <div className="py-6 text-center text-[13px] text-muted-foreground/70">
                No pages found.
              </div>
            ) : (
              results.map((node, index) => (
                <button
                  key={node.id}
                  onClick={() => {
                    navigate(`/page/${node.id}`);
                    closeQuickSwitcher();
                  }}
                  className={cn(
                    'flex w-full items-center gap-2.5 px-3 py-2 text-left text-[13px] transition-colors',
                    index === selectedIndex
                      ? 'bg-accent text-foreground'
                      : 'text-foreground/85 hover:bg-accent/60'
                  )}
                >
                  <span className="flex h-5 w-5 flex-shrink-0 items-center justify-center text-[15px] leading-none">
                    {node.emoji || (
                      <span className="block h-1.5 w-1.5 rounded-full bg-muted-foreground/40" />
                    )}
                  </span>
                  <span className="min-w-0 flex-1 truncate">{node.title || 'Untitled'}</span>
                  <span className="flex-shrink-0 rounded bg-muted-foreground/10 px-1.5 py-0.5 text-[10px] capitalize text-muted-foreground/80">
                    {node.type}
                  </span>
                  <span className="flex-shrink-0 text-[11px] text-muted-foreground/60">
                    {formatRelative(node.updatedAt)}
                  </span>
                </button>
              ))
            )}
          </div>

          {/* Footer hint */}
          <div className="flex items-center justify-between border-t border-border/40 px-3 py-2 text-[11px] text-muted-foreground/50">
            <span>
              <kbd className="rounded border border-border/40 bg-background/40 px-1 py-0.5 font-mono text-[10px]">
                ↑↓
              </kbd>{' '}
              navigate
            </span>
            <span>
              <kbd className="rounded border border-border/40 bg-background/40 px-1 py-0.5 font-mono text-[10px]">
                ↵
              </kbd>{' '}
              open
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}