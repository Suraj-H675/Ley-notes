import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui';
import { Settings, Moon, Sun } from 'lucide-react';
import { useWorkspaceStore } from '@/store';

export function SidebarFooter() {
  const navigate = useNavigate();
  const { theme, setTheme } = useWorkspaceStore();

  const toggleTheme = () => {
    setTheme(theme === 'dark' ? 'light' : 'dark');
  };

  return (
    <div className="border-t p-3">
      <div className="flex items-center justify-between">
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={() => navigate('/settings')}
        >
          <Settings className="h-4 w-4" />
        </Button>

        <Button variant="ghost" size="icon" className="h-8 w-8" onClick={toggleTheme}>
          {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </Button>
      </div>
    </div>
  );
}
