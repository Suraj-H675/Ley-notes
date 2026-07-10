/**
 * BacklinksPanel — shows incoming links for the active page, grouped by source.
 * Also lists uncreated (ghost) outgoing links so the user can promote them to pages.
 */

import { ArrowLeft, FilePlus2, Hash, FileText } from 'lucide-react';
import { useBacklinks, useGhostOutgoingLinks } from '@/hooks/useBacklinks';
import { usePageById } from '@/hooks/usePages';
import { useNavStore } from '@/store/nav';
import { createPage } from '@/core/vault/pages';
import { EmptyState } from '@/ui/EmptyState';

export function BacklinksPanel({ pageId }: { pageId: string | null }) {
  const backlinks = useBacklinks(pageId);
  const ghosts = useGhostOutgoingLinks(pageId);
  const page = usePageById(pageId);

  const openPage = useNavStore((s) => s.openPage);
  const pushRecent = useNavStore((s) => s.pushRecent);

  if (!pageId || !page) return null;

  async function handleCreateGhost(title: string) {
    const created = await createPage({ title });
    openPage(created.id);
    pushRecent(created.id);
  }

  // Group backlinks by source page.
  const grouped = new Map<string, { source: typeof page; count: number }>();
  for (const b of backlinks ?? []) {
    const cur = grouped.get(b.source.id);
    if (cur) cur.count += 1;
    else grouped.set(b.source.id, { source: b.source, count: 1 });
  }
  const groups = [...grouped.values()].sort((a, b) => b.count - a.count);

  return (
    <div className="flex h-full flex-col overflow-y-auto">
      <div className="px-4 pb-3 pt-4">
        <div className="flex items-center gap-1.5 text-meta font-medium text-muted-foreground">
          <ArrowLeft size={12} />
          <span>Backlinks ({groups.length})</span>
        </div>
      </div>

      {groups.length === 0 ? (
        <div className="px-4">
          <EmptyState
            title="No backlinks yet"
            description="Add [[Page]] references and they'll show up here."
          />
        </div>
      ) : (
        <div className="flex flex-col gap-2 px-4 pb-4">
          {groups.map(({ source, count }) => (
            <button
              key={source.id}
              type="button"
              onClick={() => {
                openPage(source.id);
                pushRecent(source.id);
              }}
              className="group flex items-center gap-2 rounded-md border border-border bg-surface-1 px-3 py-2 text-left hover:border-border-strong hover:bg-surface-2"
            >
              <FileText size={14} className="shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-meta font-medium text-foreground">{source.title}</div>
                <div className="truncate text-micro text-muted-foreground">
                  {source.content.slice(0, 80)}
                </div>
              </div>
              <span className="rounded-full bg-surface-3 px-1.5 py-0.5 text-micro text-muted-foreground-strong">
                {count}
              </span>
            </button>
          ))}
        </div>
      )}

      {(ghosts?.length ?? 0) > 0 && (
        <>
          <div className="border-t border-border px-4 pb-3 pt-4">
            <div className="flex items-center gap-1.5 text-meta font-medium text-muted-foreground">
              <Hash size={12} />
              <span>Uncreated links ({ghosts!.length})</span>
            </div>
          </div>
          <div className="flex flex-col gap-1 px-4 pb-4">
            {ghosts!.map((g) => (
              <button
                key={g.id}
                type="button"
                onClick={() => handleCreateGhost(g.targetTitle)}
                className="group flex items-center gap-2 rounded-md border border-dashed border-border bg-surface-1 px-3 py-1.5 text-left hover:border-secondary hover:bg-surface-2"
              >
                <FilePlus2 size={13} className="shrink-0 text-secondary" />
                <span className="truncate text-meta text-foreground">{g.targetTitle}</span>
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}