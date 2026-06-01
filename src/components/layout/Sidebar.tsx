import { useNavigate } from 'react-router-dom';
import { ScrollArea } from '@/components/ui';
import { useWorkspaceStore, useSearchStore } from '@/store';
import { useNodes, useCollections } from '@/hooks';
import { SidebarHeader } from './SidebarHeader';
import { SidebarCollections } from './SidebarCollections';
import { SidebarFooter } from './SidebarFooter';
import { ResizeHandle } from './ResizeHandle';
import { cn } from '@/lib/utils';
import { Home, ListTodo, Globe, ChevronRight } from 'lucide-react';

export function Sidebar() {
  const navigate = useNavigate();
  const { sidebarCollapsed, sidebarWidth, toggleSidebar, setSidebarWidth } = useWorkspaceStore();
  const { openSearch } = useSearchStore();
  const { nodes } = useNodes();
  const { collections } = useCollections();

  const recentNodes = nodes
    .filter((n) => n.type === 'document')
    .sort((a, b) => b.updatedAt - a.updatedAt)
    .slice(0, 5);

  return (
    <>
      <aside
        className={cn(
          'fixed left-0 top-0 z-40 h-screen flex flex-col border-r bg-card transition-all duration-300',
          sidebarCollapsed ? 'w-0 overflow-hidden' : ''
        )}
        style={{ width: sidebarCollapsed ? 0 : sidebarWidth }}
      >
        <SidebarHeader onSearchClick={openSearch} />

        <ScrollArea className="flex-1">
          <nav className="p-3 space-y-4">
            <div className="space-y-1">
              <NavItem
                icon={<Home className="h-4 w-4" />}
                label="Home"
                onClick={() => navigate('/')}
              />
              <NavItem
                icon={<ListTodo className="h-4 w-4" />}
                label="Tasks"
                onClick={() => navigate('/tasks')}
              />
              <NavItem
                icon={<Globe className="h-4 w-4" />}
                label="Universe"
                onClick={() => navigate('/universe')}
              />
            </div>

            <SidebarCollections collections={collections} />

            {recentNodes.length > 0 && (
              <div className="space-y-1">
                <h3 className="px-2 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  Recent
                </h3>
                {recentNodes.map((node) => (
                  <NavItem
                    key={node.id}
                    icon={<span>{node.emoji || '📄'}</span>}
                    label={node.title || 'Untitled'}
                    onClick={() => navigate(`/page/${node.id}`)}
                  />
                ))}
              </div>
            )}
          </nav>
        </ScrollArea>

        <SidebarFooter />

        <ResizeHandle
          onResize={setSidebarWidth}
          defaultWidth={sidebarWidth}
        />
      </aside>

      {sidebarCollapsed && (
        <button
          onClick={toggleSidebar}
          className="fixed left-2 top-2 z-50 flex h-8 w-8 items-center justify-center rounded-md border bg-card hover:bg-accent"
        >
          <ChevronRight className="h-4 w-4" />
        </button>
      )}
    </>
  );
}

interface NavItemProps {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  active?: boolean;
}

function NavItem({ icon, label, onClick, active }: NavItemProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-accent hover:text-accent-foreground',
        active && 'bg-accent text-accent-foreground'
      )}
    >
      {icon}
      <span className="truncate">{label}</span>
    </button>
  );
}
