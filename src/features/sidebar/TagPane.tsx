/**
 * TagPane — sidebar list of all tags in the vault with their counts. Click
 * a tag to filter the graph or search.
 */

import { useState } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { ChevronDown, ChevronRight, Hash } from 'lucide-react';
import { db } from '@/infrastructure/database/db';
import { cn } from '@/shared/lib/classnames';
import { tagSegments } from '@/core/parser/tags';
import { useTagFilter } from '@/shared/state/tag-filter';

export function TagPane() {
  const [expanded, setExpanded] = useState(false);
  const tags = useLiveQuery(async () => {
    const rows = await db.tags.toArray();
    const counts = new Map<string, number>();
    for (const r of rows) counts.set(r.tag, (counts.get(r.tag) ?? 0) + 1);
    return [...counts.entries()]
      .map(([tag, count]) => ({ tag, count }))
      .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag));
  }, []);

  const activeTag = useTagFilter((s) => s.activeTag);
  const setActiveTag = useTagFilter((s) => s.setActiveTag);

  if (!tags || tags.length === 0) return null;

  return (
    <div className="flex flex-col gap-1 px-2">
      <div className="flex items-center justify-between">
        <button type="button" onClick={() => setExpanded((value) => !value)} className="flex flex-1 items-center gap-1.5 rounded px-2 py-1 text-meta font-medium text-muted-foreground hover:bg-surface-2 hover:text-foreground" aria-expanded={expanded}>
          {expanded ? <ChevronDown size={11} /> : <ChevronRight size={11} />}<Hash size={12} /><span>Tags</span><span className="ml-auto text-micro text-subtle-foreground">{tags.length}</span>
        </button>
        {activeTag && (
          <button
            type="button"
            onClick={() => setActiveTag(null)}
            className="text-micro text-muted-foreground hover:text-foreground"
          >
            Clear
          </button>
        )}
      </div>
      {expanded && tags.map(({ tag, count }) => {
        const segs = tagSegments(tag);
        return (
          <button
            key={tag}
            type="button"
            onClick={() => setActiveTag(activeTag === tag ? null : tag)}
            className={cn(
              'flex w-full items-center gap-1.5 rounded-sm px-2 py-0.5 text-left text-meta',
              activeTag === tag
                ? 'bg-secondary/15 text-secondary'
                : 'text-muted-foreground-strong hover:bg-surface-2 hover:text-foreground',
            )}
          >
            <Hash size={11} className="shrink-0 text-subtle-foreground" />
            <span className="truncate">
              {segs.map((s, i) => (
                <span key={i}>
                  {i > 0 && <span className="text-subtle-foreground">/</span>}
                  {s}
                </span>
              ))}
            </span>
            <span className="ml-auto rounded-full bg-surface-3 px-1.5 py-0.5 text-micro text-muted-foreground">
              {count}
            </span>
          </button>
        );
      })}
    </div>
  );
}
