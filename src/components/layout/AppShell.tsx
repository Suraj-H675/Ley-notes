import { Outlet, useLocation } from 'react-router-dom';
import { Sidebar } from './Sidebar';
import { TooltipProvider } from '@/components/ui';
import { useWorkspaceStore } from '@/store';

export function AppShell() {
  const { sidebarCollapsed, sidebarWidth } = useWorkspaceStore();
  const location = useLocation();

  return (
    <TooltipProvider delayDuration={200} skipDelayDuration={300}>
      <div className="flex h-screen overflow-hidden bg-background text-foreground">
        <Sidebar />
        <main
          className="flex-1 overflow-hidden transition-[margin] duration-200"
          style={{ marginLeft: sidebarCollapsed ? 0 : sidebarWidth }}
        >
          <div key={location.pathname} className="h-full animate-fade-in">
            <Outlet />
          </div>
        </main>
      </div>
    </TooltipProvider>
  );
}
