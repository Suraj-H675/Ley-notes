import { useNavigate, useLocation } from 'react-router-dom';
import { ScrollArea } from '@/components/ui';
import { useWorkspaceStore, useSearchStore } from '@/store';
import { useNodes, useCollections } from '@/hooks';
import { SidebarHeader } from './SidebarHeader';
import { SidebarCollections } from './SidebarCollections';
import { SidebarFooter } from './SidebarFooter';
import { ResizeHandle } from './ResizeHandle';
import { cn } from '@/lib/utils';
import {
  Home,
  ListTodo,
  Globe,
  Folder,
  Settings,
  ChevronsRight,
} from 'lucide-react';
import type { ReactNode } from 'react';

const TOP_ITEMS: { label: string; to: string; icon: ReactNode; shortcut?: string }[] = [
  { label: 'Search', to: '__search__', icon: <SearchIcon />, shortcut: '⌘K' },
  { label: 'Home', to: '/', icon: <Home className="h-3.5 w-3.5" /> },
  { label: 'Tasks', to: '/tasks', icon: <ListTodo className="h-3.5 w-3.5" /> },
  { label: 'Projects', to: '/projects', icon: <Folder className="h-3.5 w-3.5" /> },
  { label: 'Universe', to: '/universe', icon: <Globe className="h-3.5 w-3.5" /> },
  { label: 'Settings', to: '/settings', icon: <Settings className="h-3.5 w-3.5" /> },
];

function SearchIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="11" cy="11" r="8" />
      <path d="m21 21-4.3-4.3" />
    </svg>
  );
}

export function Sidebar() {
  const navigate = useNavigate();
  const location = useLocation();
  const { sidebarCollapsed, sidebarWidth, toggleSidebar, setSidebarWidth } =
    useWorkspaceStore();
  const { openSearch } = useSearchStore();
  const { nodes } = useNodes();
  const { collections } = useCollections();

  const recentNodes = nodes
    .filter((n) => n.type === 'document')
    .sort((a, b) => b.updatedAt - a.updatedAt)
    .slice(0, 8);

  const isActive = (to: string) => {
    if (to === '/') return location.pathname === '/';
    return location.pathname.startsWith(to);
  };

  return (
    <>
      <aside
        className={cn(
          'fixed left-0 top-0 z-40 flex h-screen flex-col bg-card/40 backdrop-blur transition-all duration-200',
          sidebarCollapsed && 'w-0 overflow-hidden'
        )}
        style={{ width: sidebarCollapsed ? 0 : sidebarWidth }}
      >
        <SidebarHeader onSearchClick={openSearch} />

        <ScrollArea className="flex-1">
          <div className="px-1.5 pb-4">
            <div className="space-y-0.5">
              {TOP_ITEMS.map((item) => {
                const isSearch = item.to === '__search__';
                const active = !isSearch && isActive(item.to);
                return (
                  <button
                    key={item.label}
                    onClick={() => (isSearch ? openSearch() : navigate(item.to))}
                    className={cn(
                      'group flex w-full items-center gap-2 rounded px-2 py-[5px] text-left text-[13px] transition-colors',
                      active
                        ? 'bg-accent/70 text-foreground'
                        : 'text-foreground/75 hover:bg-accent/50'
                    )}
                  >
                    <span
                      className={cn(
                        'flex h-3.5 w-3.5 flex-shrink-0 items-center justify-center',
                        active ? 'text-foreground' : 'text-muted-foreground/70'
                      )}
                    >
                      {item.icon}
                    </span>
                    <span className="flex-1 truncate">{item.label}</span>
                    {item.shortcut && (
                      <kbd className="ml-auto rounded border border-border/60 bg-background/40 px-1 py-px font-mono text-[10px] text-muted-foreground/70">
                        {item.shortcut}
                      </kbd>
                    )}
                  </button>
                );
              })}
            </div>

            <SidebarCollections collections={collections} />

            {recentNodes.length > 0 && (
              <div className="mt-3 space-y-0.5">
                <div className="px-2 py-1 text-[11px] font-medium text-muted-foreground/70">
                  Recent
                </div>
                {recentNodes.map((node) => (
                  <button
                    key={node.id}
                    onClick={() => navigate(`/page/${node.id}`)}
                    className={cn(
                      'group flex w-full items-center gap-2 rounded px-2 py-[5px] text-left text-[13px] transition-colors',
                      isActive(`/page/${node.id}`)
                        ? 'bg-accent/70 text-foreground'
                        : 'text-foreground/75 hover:bg-accent/50'
                    )}
                  >
                    <span className="flex h-3.5 w-3.5 flex-shrink-0 items-center justify-center text-[12px] leading-none">
                      {node.emoji || <span className="block h-1 w-1 rounded-full bg-muted-foreground/40" />}
                    </span>
                    <span className="flex-1 truncate">
                      {node.title || 'Untitled'}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </ScrollArea>

        <SidebarFooter />

        <ResizeHandle onResize={setSidebarWidth} defaultWidth={sidebarWidth} />
      </aside>

      {sidebarCollapsed && (
        <button
          onClick={toggleSidebar}
          className="fixed left-2 top-2 z-50 flex h-7 w-7 items-center justify-center rounded-md border border-border/60 bg-card text-muted-foreground/70 shadow-panel transition-colors hover:bg-accent hover:text-foreground"
        >
          <ChevronsRight className="h-3.5 w-3.5" />
        </button>
      )}
    </>
  );
}
