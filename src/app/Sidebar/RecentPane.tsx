/**
 * RecentPane — shows the last 20 visited pages.
 */

import { FileClock } from 'lucide-react';
import { useNavStore } from '@/store/nav';
import { useRecentPages } from '@/hooks/usePages';
import { cn } from '@/lib/classnames';

export function RecentPane() {
  const recentIds = useNavStore((s) => s.recentPages);
  const openPage = useNavStore((s) => s.openPage);
  const pushRecent = useNavStore((s) => s.pushRecent);
  const activeTab = useNavStore((s) => s.activeTab);
  const pages = useRecentPages(recentIds);

  if (pages.length === 0) return null;

  return (
    <div className="flex flex-col gap-1 px-2">
      <div className="flex items-center gap-1.5 px-2 py-1 text-meta font-medium text-muted-foreground">
        <FileClock size={12} />
        <span>Recent</span>
      </div>
      {pages.map((p) => (
        <button
          key={p.id}
          type="button"
          onClick={() => {
            openPage(p.id);
            pushRecent(p.id);
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