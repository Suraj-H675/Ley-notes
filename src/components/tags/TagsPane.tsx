import { useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { Input } from '@/components/ui';
import { cn } from '@/lib/utils';
import { Tag, X } from 'lucide-react';

interface TagEntry {
  tag: string;
  count: number;
}

export function TagsPane() {
  const navigate = useNavigate();
  const [filter, setFilter] = useState('');

  const nodes = useLiveQuery(
    () => db.nodes.where('isArchived').equals(0).toArray(),
    [],
    []
  );

  const tagCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const node of nodes) {
      for (const tag of node.tags) {
        counts.set(tag, (counts.get(tag) ?? 0) + 1);
      }
    }
    return counts;
  }, [nodes]);

  const sortedTags = useMemo((): TagEntry[] => {
    const entries: TagEntry[] = [];
    for (const [tag, count] of tagCounts) {
      entries.push({ tag, count });
    }
    return entries
      .filter((e) => e.tag.toLowerCase().includes(filter.toLowerCase()))
      .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag));
  }, [tagCounts, filter]);

  const handleTagClick = (tag: string) => {
    navigate(`/?tag=${encodeURIComponent(tag)}`);
  };

  const handleClearFilter = () => setFilter('');

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center gap-2 px-1 py-2">
        <Tag className="h-3.5 w-3.5 flex-shrink-0 text-muted-foreground/70" />
        <span className="text-[13px] font-medium text-foreground/85">Tags</span>
        <span className="ml-auto text-[11px] text-muted-foreground/50">
          {tagCounts.size}
        </span>
      </div>

      {/* Search input */}
      <div className="relative px-1 pb-2">
        <Input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter tags…"
          className="pr-7"
        />
        {filter && (
          <button
            onClick={handleClearFilter}
            className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/50 hover:text-muted-foreground"
          >
            <X className="h-3 w-3" />
          </button>
        )}
      </div>

      {/* Tag list */}
      <div className="flex-1 overflow-y-auto px-1">
        {sortedTags.length === 0 ? (
          <div className="py-6 text-center text-[12.5px] text-muted-foreground/60">
            {tagCounts.size === 0
              ? 'No tags yet. Add tags to your pages to organize knowledge.'
              : 'No matching tags.'}
          </div>
        ) : (
          <div className="flex flex-wrap gap-1.5 py-0.5">
            {sortedTags.map(({ tag, count }) => (
              <button
                key={tag}
                onClick={() => handleTagClick(tag)}
                className={cn(
                  'inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11.5px] transition-colors',
                  'border-border/60 bg-background/50 text-foreground/80',
                  'hover:border-foreground/30 hover:bg-accent/60 hover:text-foreground'
                )}
              >
                <span className="truncate max-w-[120px]">{tag}</span>
                <span
                  className={cn(
                    'flex-shrink-0 rounded-full px-1 py-0 leading-none text-[10px]',
                    'bg-accent/70 text-muted-foreground/80'
                  )}
                >
                  {count}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}