import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui';
import { ArrowLeft } from 'lucide-react';
import { useWorkspaceStore } from '@/store';

export function SettingsPage() {
  const navigate = useNavigate();
  const { theme, setTheme } = useWorkspaceStore();

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center gap-4 border-b p-4">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => navigate(-1)}
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <h1 className="text-xl font-semibold">Settings</h1>
      </header>

      <main className="flex-1 overflow-auto p-8">
        <div className="max-w-2xl mx-auto space-y-8">
          <section className="space-y-4">
            <h2 className="text-lg font-semibold">Appearance</h2>
            <div className="space-y-2">
              <label className="text-sm font-medium">Theme</label>
              <div className="flex gap-2">
                <Button
                  variant={theme === 'dark' ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setTheme('dark')}
                >
                  Dark
                </Button>
                <Button
                  variant={theme === 'light' ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setTheme('light')}
                >
                  Light
                </Button>
                <Button
                  variant={theme === 'system' ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => setTheme('system')}
                >
                  System
                </Button>
              </div>
            </div>
          </section>

          <section className="space-y-4">
            <h2 className="text-lg font-semibold">About</h2>
            <div className="text-sm text-muted-foreground space-y-2">
              <p>Knowledge Universe v0.1.0</p>
              <p>
                A local-first workspace for your knowledge graph.
              </p>
            </div>
          </section>
        </div>
      </main>
    </div>
  );
}
