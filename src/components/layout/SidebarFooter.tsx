import { useWorkspaceStore } from '@/store';
import { Sun, Moon, Monitor } from 'lucide-react';
import { cn } from '@/lib/utils';

export function SidebarFooter() {
  const { theme, setTheme } = useWorkspaceStore();

  const options: { value: 'light' | 'dark' | 'system'; icon: typeof Sun; label: string }[] = [
    { value: 'light', icon: Sun, label: 'Light' },
    { value: 'dark', icon: Moon, label: 'Dark' },
    { value: 'system', icon: Monitor, label: 'System' },
  ];

  return (
    <div className="flex items-center gap-0.5 border-t border-border/40 px-2 py-2">
      {options.map((opt) => {
        const Icon = opt.icon;
        const active = theme === opt.value;
        return (
          <button
            key={opt.value}
            onClick={() => setTheme(opt.value)}
            aria-label={opt.label}
            className={cn(
              'flex h-6 flex-1 items-center justify-center rounded text-muted-foreground/60 transition-colors',
              active
                ? 'bg-accent text-foreground'
                : 'hover:bg-accent/50 hover:text-foreground'
            )}
          >
            <Icon className="h-3.5 w-3.5" />
          </button>
        );
      })}
    </div>
  );
}
