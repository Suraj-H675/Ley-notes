import { Outlet } from 'react-router-dom';
import { Sidebar } from './Sidebar';
import { TooltipProvider } from '@/components/ui';
import { useWorkspaceStore } from '@/store';

export function AppShell() {
  const { sidebarCollapsed, sidebarWidth } = useWorkspaceStore();

  return (
    <TooltipProvider>
      <div className="flex h-screen overflow-hidden bg-background text-foreground">
        <Sidebar />
        <main
          className="flex-1 overflow-hidden transition-all duration-200"
          style={{ marginLeft: sidebarCollapsed ? 0 : sidebarWidth }}
        >
          <Outlet />
        </main>
      </div>
    </TooltipProvider>
  );
}
