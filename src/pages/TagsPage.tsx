import { useNavigate, useSearchParams } from 'react-router-dom';
import { useState, useMemo } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { Input } from '@/components/ui';
import { Tag, X, ArrowLeft } from 'lucide-react';
import { formatRelative } from '@/lib/utils';

interface TagEntry {
  tag: string;
  count: number;
  mostRecentNode?: { title: string; id: string; updatedAt: number };
}

export function TagsPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const activeTag = searchParams.get('tag');
  const [filter, setFilter] = useState('');

  const nodes = useLiveQuery(
    () => db.nodes.where('isArchived').equals(0).toArray(),
    [],
    []
  );

  // Build tag → count + most-recent-node map
  const tagData = useMemo((): Map<string, TagEntry> => {
    const map = new Map<string, TagEntry>();
    for (const node of nodes) {
      for (const tag of node.tags) {
        const existing = map.get(tag);
        if (!existing) {
          map.set(tag, { tag, count: 1, mostRecentNode: { title: node.title, id: node.id, updatedAt: node.updatedAt } });
        } else {
          existing.count += 1;
          if (node.updatedAt > (existing.mostRecentNode?.updatedAt ?? 0)) {
            existing.mostRecentNode = { title: node.title, id: node.id, updatedAt: node.updatedAt };
          }
        }
      }
    }
    return map;
  }, [nodes]);

  const sortedTags = useMemo((): TagEntry[] => {
    return Array.from(tagData.values())
      .filter((e) => e.tag.toLowerCase().includes(filter.toLowerCase()))
      .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag));
  }, [tagData, filter]);

  // When a tag is selected from the list, navigate to filter view
  const handleTagClick = (tag: string) => {
    navigate(`/tags?tag=${encodeURIComponent(tag)}`);
  };

  // Filtered nodes when viewing a specific tag
  const filteredNodes = useMemo(() => {
    if (!activeTag) return [];
    return nodes
      .filter((n) => n.tags.includes(activeTag))
      .sort((a, b) => b.updatedAt - a.updatedAt);
  }, [nodes, activeTag]);

  const handleClearFilter = () => setFilter('');

  const handleBack = () => navigate('/tags');

  return (
    <div className="h-full overflow-auto">
      <div className="mx-auto max-w-2xl px-8 py-10">
        {/* Header */}
        <div className="mb-6 flex items-center gap-3">
          <Tag className="h-5 w-5 flex-shrink-0 text-muted-foreground/70" />
          <h1 className="text-[22px] font-semibold tracking-[-0.015em] text-foreground">
            {activeTag ? `#${activeTag}` : 'Tags'}
          </h1>
          {activeTag && (
            <button
              onClick={handleBack}
              className="ml-auto flex items-center gap-1 text-[12.5px] text-muted-foreground/70 hover:text-foreground transition-colors"
            >
              <ArrowLeft className="h-3.5 w-3.5" />
              All tags
            </button>
          )}
        </div>

        {!activeTag ? (
          <>
            {/* Search */}
            <div className="relative mb-5">
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
            {sortedTags.length === 0 ? (
              <div className="py-12 text-center text-[13.5px] text-muted-foreground/60">
                {tagData.size === 0
                  ? 'No tags yet. Add tags to your pages to organize knowledge.'
                  : 'No matching tags.'}
              </div>
            ) : (
              <ul className="divide-y divide-border/40">
                {sortedTags.map(({ tag, count, mostRecentNode }) => (
                  <li key={tag}>
                    <button
                      onClick={() => handleTagClick(tag)}
                      className="group flex w-full items-center gap-3 px-2 py-3 text-left transition-colors hover:bg-accent/40"
                    >
                      <span className="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-accent/60">
                        <Tag className="h-3 w-3 text-foreground/70" />
                      </span>
                      <span className="flex-1 text-[13.5px] text-foreground/90">
                        {tag}
                      </span>
                      <span className="text-[11px] text-muted-foreground/60">
                        {count} {count === 1 ? 'page' : 'pages'}
                      </span>
                      {mostRecentNode && (
                        <span className="hidden text-right text-[11px] text-muted-foreground/50 sm:block min-w-0 ml-2">
                          <span className="truncate block max-w-[120px]">{mostRecentNode.title || 'Untitled'}</span>
                        </span>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </>
        ) : (
          <>
            {/* Nodes with the selected tag */}
            {filteredNodes.length === 0 ? (
              <div className="py-12 text-center text-[13.5px] text-muted-foreground/60">
                No pages with this tag.
              </div>
            ) : (
              <ul className="divide-y divide-border/40">
                {filteredNodes.map((node) => (
                  <li key={node.id}>
                    <button
                      onClick={() => navigate(`/page/${node.id}`)}
                      className="group flex w-full items-center gap-3 px-2 py-2.5 text-left transition-colors hover:bg-accent/40"
                    >
                      <span className="flex h-5 w-5 flex-shrink-0 items-center justify-center text-[12px] leading-none">
                        {node.emoji || (
                          <span className="block h-1.5 w-1.5 rounded-full bg-muted-foreground/40" />
                        )}
                      </span>
                      <span className="flex-1 truncate text-[13.5px] text-foreground/90">
                        {node.title || 'Untitled'}
                      </span>
                      <span className="text-[11px] capitalize text-muted-foreground/60">
                        {node.type}
                      </span>
                      <span className="hidden w-20 text-right text-[11px] text-muted-foreground/50 sm:block">
                        {formatRelative(node.updatedAt)}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </>
        )}
      </div>
    </div>
  );
}