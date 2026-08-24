/**
 * RecentPane — shows the last 20 visited pages.
 */

import { useState } from 'react';
import { ChevronDown, ChevronRight, FileClock } from 'lucide-react';
import { useNavStore } from '@/shared/state/nav';
import { useRecentPages } from '@/features/notes/usePages';
import { cn } from '@/shared/lib/classnames';
import { useUIStore } from '@/shared/state/ui';

export function RecentPane() {
  const [expanded, setExpanded] = useState(true);
  const recentIds = useNavStore((s) => s.recentPages);
  const openPage = useNavStore((s) => s.openPage);
  const pushRecent = useNavStore((s) => s.pushRecent);
  const activeTab = useNavStore((s) => s.activeTab);
  const pages = useRecentPages(recentIds);

  if (pages.length === 0) return null;

  return (
    <div className="flex flex-col gap-1 px-2">
      <button type="button" onClick={() => setExpanded((value) => !value)} className="flex items-center gap-1.5 rounded-sm px-2 py-1 text-meta font-medium uppercase tracking-[0.08em] text-subtle-foreground hover:bg-surface-2 hover:text-foreground" aria-expanded={expanded}>
        {expanded ? <ChevronDown size={11} /> : <ChevronRight size={11} />}<FileClock size={12} /><span>Recent</span><span className="ml-auto text-micro text-subtle-foreground">{pages.length}</span>
      </button>
      {expanded && pages.map((p) => (
        <button
          key={p.id}
          type="button"
          onClick={() => {
            openPage(p.id);
            pushRecent(p.id);
            if (window.matchMedia('(max-width: 767px)').matches) useUIStore.getState().setSidebarOpen(false);
          }}
          className={cn(
            'flex items-center gap-1.5 rounded-sm px-2 py-0.5 text-left text-meta',
            activeTab === p.id
              ? 'bg-surface-3 text-foreground'
              : 'text-muted-foreground-strong hover:bg-surface-2 hover:text-foreground',
          )}
        >
          <span className="truncate">{p.title}</span>
        </button>
      ))}
    </div>
  );
}
