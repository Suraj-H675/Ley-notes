import { useWorkspaceStore } from '@/store';
import { Button } from '@/components/ui';
import { Search, Menu } from 'lucide-react';

interface SidebarHeaderProps {
  onSearchClick: () => void;
}

export function SidebarHeader({ onSearchClick }: SidebarHeaderProps) {
  const { toggleSidebar } = useWorkspaceStore();

  return (
    <div className="flex items-center gap-2 border-b p-3">
      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8"
        onClick={toggleSidebar}
      >
        <Menu className="h-4 w-4" />
      </Button>

      <div className="flex-1">
        <h1 className="text-sm font-semibold">Knowledge Universe</h1>
      </div>

      <Button
        variant="ghost"
        size="icon"
        className="h-8 w-8"
        onClick={onSearchClick}
      >
        <Search className="h-4 w-4" />
      </Button>
    </div>
  );
}
