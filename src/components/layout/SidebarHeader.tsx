import { useState } from 'react';
import { useWorkspaceStore } from '@/store';
import { Search, ChevronsLeft, ChevronsRight, Plus } from 'lucide-react';
import { useSearchStore } from '@/store';
import { useNavigate } from 'react-router-dom';
import { useNodes } from '@/hooks';

interface SidebarHeaderProps {
  onSearchClick: () => void;
}

export function SidebarHeader({ onSearchClick }: SidebarHeaderProps) {
  const { sidebarCollapsed, toggleSidebar } = useWorkspaceStore();
  const navigate = useNavigate();
  const { openSearch } = useSearchStore();
  const { createNode } = useNodes();
  const [creating, setCreating] = useState(false);

  const handleNewPage = async () => {
    if (creating) return;
    setCreating(true);
    try {
      const node = await createNode({ type: 'document', title: '' });
      navigate(`/page/${node.id}`);
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="flex items-center gap-1 px-2 py-2">
      <button
        onClick={toggleSidebar}
        aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground"
      >
        {sidebarCollapsed ? (
          <ChevronsRight className="h-3.5 w-3.5" />
        ) : (
          <ChevronsLeft className="h-3.5 w-3.5" />
        )}
      </button>

      <div className="flex-1 px-1.5 text-[13px] font-medium tracking-tight text-foreground/90">
        Knowledge
      </div>

      <button
        onClick={handleNewPage}
        disabled={creating}
        aria-label="New page"
        className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground disabled:opacity-50"
      >
        <Plus className="h-3.5 w-3.5" />
      </button>

      <button
        onClick={onSearchClick || openSearch}
        aria-label="Search"
        className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground"
      >
        <Search className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
