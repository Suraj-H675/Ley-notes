import { useState } from 'react';
import { ChevronDown, ChevronRight, Star } from 'lucide-react';
import { useFavoritePages } from './useFavorites';
import { useNavStore } from '@/shared/state/nav';
import { cn } from '@/shared/lib/classnames';
import { useUIStore } from '@/shared/state/ui';

export function FavoritesPane() {
  const [expanded, setExpanded] = useState(true);
  const pages = useFavoritePages();
  const activeTab = useNavStore((state) => state.activeTab);

  function openPage(pageId: string) {
    const nav = useNavStore.getState();
    nav.openPage(pageId);
    nav.pushRecent(pageId);
    if (window.matchMedia('(max-width: 767px)').matches) {
      useUIStore.getState().setSidebarOpen(false);
    }
  }

  return (
    <div className="flex flex-col gap-1 px-2">
      <button type="button" onClick={() => setExpanded((value) => !value)} className="flex items-center gap-1.5 rounded px-2 py-1 text-meta font-medium text-muted-foreground hover:bg-surface-2 hover:text-foreground" aria-expanded={expanded}>
        {expanded ? <ChevronDown size={11} /> : <ChevronRight size={11} />}<Star size={12} className="fill-current" /><span>Favorites</span><span className="ml-auto text-micro text-subtle-foreground">{pages.length}</span>
      </button>
      {expanded && pages.map((page) => (
        <button key={page.id} type="button" onClick={() => openPage(page.id)} className={cn('flex items-center gap-1.5 rounded-sm px-2 py-0.5 text-left text-meta', activeTab === page.id ? 'bg-surface-3 text-foreground' : 'text-muted-foreground-strong hover:bg-surface-2 hover:text-foreground')}>
          <Star size={10} className="shrink-0 fill-current text-secondary" /><span className="truncate">{page.title}</span>
        </button>
      ))}
      {expanded && pages.length === 0 && <p className="px-2 py-1 text-micro leading-relaxed text-muted-foreground">Star an important note to keep it here.</p>}
    </div>
  );
}
