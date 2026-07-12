import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import {
  CalendarDays,
  FilePlus2,
  FolderSearch,
  Moon,
  LayoutDashboard,
  Network,
  PanelLeft,
  PanelRight,
  Search,
  Settings,
  Sun,
} from 'lucide-react';
import { Kbd } from '@/shared/components/Kbd';

interface PaletteCommand {
  id: string;
  label: string;
  detail: string;
  icon: ReactNode;
  keywords?: string;
  shortcut?: string;
  run: () => void | Promise<void>;
}

export function CommandPalette({
  open,
  onClose,
  onNewNote,
  onQuickSwitcher,
  onDailyNote,
  onGraph,
  onCanvas,
  onSettings,
  onToggleSidebar,
  onToggleRightDock,
  onSetTheme,
}: {
  open: boolean;
  onClose: () => void;
  onNewNote: () => void;
  onQuickSwitcher: () => void;
  onDailyNote: () => void | Promise<void>;
  onGraph: () => void;
  onCanvas: () => void;
  onSettings: () => void;
  onToggleSidebar: () => void;
  onToggleRightDock: () => void;
  onSetTheme: (theme: 'light' | 'dark') => void;
}) {
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands = useMemo<PaletteCommand[]>(
    () => [
      { id: 'new-note', label: 'Create new note', detail: 'Add a Markdown note to the vault', icon: <FilePlus2 size={15} />, shortcut: '⌘N', run: onNewNote },
      { id: 'quick-switcher', label: 'Open quick switcher', detail: 'Find a note by title or content', icon: <FolderSearch size={15} />, shortcut: '⌘O', keywords: 'search find note', run: onQuickSwitcher },
      { id: 'daily', label: "Open today's daily note", detail: 'Capture thoughts and activity for today', icon: <CalendarDays size={15} />, shortcut: '⌘D', run: onDailyNote },
      { id: 'graph', label: 'Open graph view', detail: 'Explore the connections in this vault', icon: <Network size={15} />, shortcut: '⌘G', run: onGraph },
      { id: 'canvas', label: 'Open canvas', detail: 'Arrange notes and thoughts on a spatial board', icon: <LayoutDashboard size={15} />, keywords: 'board spatial json canvas', run: onCanvas },
      { id: 'toggle-left', label: 'Toggle left sidebar', detail: 'Show or hide the file explorer', icon: <PanelLeft size={15} />, run: onToggleSidebar },
      { id: 'toggle-right', label: 'Toggle right sidebar', detail: 'Show or hide contextual panels', icon: <PanelRight size={15} />, run: onToggleRightDock },
      { id: 'light', label: 'Use light theme', detail: 'Switch the workspace appearance', icon: <Sun size={15} />, run: () => onSetTheme('light') },
      { id: 'dark', label: 'Use dark theme', detail: 'Switch the workspace appearance', icon: <Moon size={15} />, run: () => onSetTheme('dark') },
      { id: 'settings', label: 'Open settings', detail: 'Configure Ley and this vault', icon: <Settings size={15} />, shortcut: '⌘,', run: onSettings },
    ],
    [onCanvas, onDailyNote, onGraph, onNewNote, onQuickSwitcher, onSetTheme, onSettings, onToggleRightDock, onToggleSidebar],
  );

  const matches = useMemo(() => {
    const needle = query.toLowerCase().replace(/\s+/g, '');
    if (!needle) return commands;
    return commands.filter((command) => fuzzyMatch(`${command.label} ${command.detail} ${command.keywords ?? ''}`, needle));
  }, [commands, query]);

  useEffect(() => {
    if (!open) return;
    queueMicrotask(() => {
      setQuery('');
      setSelected(0);
      inputRef.current?.focus();
    });
  }, [open]);

  if (!open) return null;

  async function run(command: PaletteCommand) {
    onClose();
    await command.run();
  }

  return (
    <div role="dialog" aria-modal="true" aria-label="Command palette" className="fixed inset-0 z-[70] flex items-start justify-center bg-background/65 pt-[12vh]" onMouseDown={onClose}>
      <div className="w-[600px] max-w-[calc(100vw-24px)] overflow-hidden rounded-xl border border-border bg-surface-1 shadow-menu" onMouseDown={(event) => event.stopPropagation()}>
        <div className="flex items-center gap-2 border-b border-border px-3">
          <Search size={15} className="text-muted-foreground" />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => { setQuery(event.target.value); setSelected(0); }}
            onKeyDown={(event) => {
              if (event.key === 'Escape') onClose();
              if (event.key === 'ArrowDown') { event.preventDefault(); setSelected((value) => Math.min(value + 1, matches.length - 1)); }
              if (event.key === 'ArrowUp') { event.preventDefault(); setSelected((value) => Math.max(value - 1, 0)); }
              if (event.key === 'Enter' && matches[selected]) { event.preventDefault(); void run(matches[selected]); }
            }}
            placeholder="Type a command…"
            className="h-12 flex-1 bg-transparent text-body text-foreground outline-none placeholder:text-subtle-foreground"
          />
          <Kbd>esc</Kbd>
        </div>
        <div className="max-h-[430px] overflow-y-auto p-1.5">
          {matches.length === 0 ? (
            <div className="px-3 py-10 text-center text-meta text-muted-foreground">No matching commands</div>
          ) : matches.map((command, index) => (
            <button
              key={command.id}
              type="button"
              onMouseEnter={() => setSelected(index)}
              onClick={() => void run(command)}
              className={`flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-left ${selected === index ? 'bg-surface-3 text-foreground' : 'text-muted-foreground-strong hover:bg-surface-2'}`}
            >
              <span className="text-secondary">{command.icon}</span>
              <span className="min-w-0 flex-1">
                <span className="block text-meta font-medium">{command.label}</span>
                <span className="block truncate text-micro text-muted-foreground">{command.detail}</span>
              </span>
              {command.shortcut && <Kbd>{command.shortcut}</Kbd>}
            </button>
          ))}
        </div>
        <div className="flex items-center justify-between border-t border-border px-3 py-2 text-micro text-muted-foreground">
          <span>Commands are available from anywhere in Ley</span>
          <span className="flex gap-1"><Kbd>↑</Kbd><Kbd>↓</Kbd><Kbd>↵</Kbd></span>
        </div>
      </div>
    </div>
  );
}

function fuzzyMatch(haystack: string, needle: string): boolean {
  const source = haystack.toLowerCase();
  let at = 0;
  for (const character of needle) {
    at = source.indexOf(character, at);
    if (at === -1) return false;
    at += 1;
  }
  return true;
}
