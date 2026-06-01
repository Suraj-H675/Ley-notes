import { useEffect, useRef, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { Command } from 'cmdk';
import { useSearchStore } from '@/store';
import { useCommands } from '@/hooks/useCommands';
import { db } from '@/lib/db';
import { cn } from '@/lib/utils';

export function CommandPalette() {
  const navigate = useNavigate();
  const { isOpen, closeSearch, query, setQuery } = useSearchStore();
  const commands = useCommands();
  const inputRef = useRef<HTMLInputElement>(null);

  const searchResults = useLiveQuery(
    async () => {
      if (!query || query.length < 2) return [];

      const lowerQuery = query.toLowerCase();
      const nodes = await db.nodes
        .where('isArchived')
        .equals(0)
        .toArray();

      return nodes
        .filter(
          (n) =>
            n.title.toLowerCase().includes(lowerQuery) ||
            n.plainText.toLowerCase().includes(lowerQuery)
        )
        .slice(0, 10)
        .map((n) => ({
          id: n.id,
          title: n.title || 'Untitled',
          type: n.type,
          emoji: n.emoji,
        }));
    },
    [query]
  );

  const groupedCommands = useMemo(() => {
    const categories = {
      create: commands.filter((c) => c.category === 'create'),
      navigation: commands.filter((c) => c.category === 'navigation'),
      action: commands.filter((c) => c.category === 'action'),
    };
    return categories;
  }, [commands]);

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [isOpen]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        if (isOpen) {
          closeSearch();
        } else {
          useSearchStore.getState().openSearch();
        }
      }

      if (e.key === 'Escape' && isOpen) {
        e.preventDefault();
        closeSearch();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, closeSearch]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50">
      <div
        className="absolute inset-0 bg-background/80 backdrop-blur-sm"
        onClick={closeSearch}
      />
      <div className="absolute left-1/2 top-[20%] -translate-x-1/2 w-full max-w-xl">
        <Command
          className={cn(
            'relative w-full rounded-lg border bg-popover shadow-lg overflow-hidden',
            'animate-in fade-in-0 zoom-in-95 slide-in-from-top-4'
          )}
        >
          <div className="flex items-center border-b px-3">
            <span className="text-muted-foreground mr-2">›</span>
            <Command.Input
              ref={inputRef}
              value={query}
              onValueChange={setQuery}
              placeholder="Search or type a command..."
              className="w-full py-3 px-1 text-sm bg-transparent outline-none placeholder:text-muted-foreground"
            />
          </div>

          <Command.List className="max-h-[300px] overflow-y-auto p-2">
            <Command.Empty className="py-6 text-center text-sm text-muted-foreground">
              No results found.
            </Command.Empty>

            {query === '' && (
              <>
                {groupedCommands.create.length > 0 && (
                  <Command.Group heading="Create">
                    {groupedCommands.create.map((cmd) => (
                      <Command.Item
                        key={cmd.id}
                        value={cmd.label}
                        onSelect={() => {
                          cmd.execute();
                          closeSearch();
                        }}
                        className="flex items-center gap-2 px-2 py-2 rounded-md text-sm cursor-pointer hover:bg-accent"
                      >
                        <span className="text-lg">
                          {getCommandIcon(cmd.icon)}
                        </span>
                        <span>{cmd.label}</span>
                      </Command.Item>
                    ))}
                  </Command.Group>
                )}

                {groupedCommands.navigation.length > 0 && (
                  <Command.Group heading="Navigate">
                    {groupedCommands.navigation.map((cmd) => (
                      <Command.Item
                        key={cmd.id}
                        value={cmd.label}
                        onSelect={() => {
                          cmd.execute();
                          closeSearch();
                        }}
                        className="flex items-center gap-2 px-2 py-2 rounded-md text-sm cursor-pointer hover:bg-accent"
                      >
                        <span className="text-lg">
                          {getCommandIcon(cmd.icon)}
                        </span>
                        <span>{cmd.label}</span>
                      </Command.Item>
                    ))}
                  </Command.Group>
                )}

                {groupedCommands.action.length > 0 && (
                  <Command.Group heading="Actions">
                    {groupedCommands.action.map((cmd) => (
                      <Command.Item
                        key={cmd.id}
                        value={cmd.label}
                        onSelect={() => {
                          cmd.execute();
                          closeSearch();
                        }}
                        className="flex items-center gap-2 px-2 py-2 rounded-md text-sm cursor-pointer hover:bg-accent"
                      >
                        <span className="text-lg">
                          {getCommandIcon(cmd.icon)}
                        </span>
                        <span>{cmd.label}</span>
                      </Command.Item>
                    ))}
                  </Command.Group>
                )}
              </>
            )}

            {query !== '' && searchResults && searchResults.length > 0 && (
              <Command.Group heading="Pages">
                {searchResults.map((node) => (
                  <Command.Item
                    key={node.id}
                    value={node.title}
                    onSelect={() => {
                      navigate(`/page/${node.id}`);
                      closeSearch();
                    }}
                    className="flex items-center gap-2 px-2 py-2 rounded-md text-sm cursor-pointer hover:bg-accent"
                  >
                    <span className="text-lg">{node.emoji || '📄'}</span>
                    <div className="flex-1 min-w-0">
                      <p className="truncate">{node.title}</p>
                      <p className="text-xs text-muted-foreground capitalize">
                        {node.type}
                      </p>
                    </div>
                  </Command.Item>
                ))}
              </Command.Group>
            )}
          </Command.List>

          <div className="border-t px-3 py-2 text-xs text-muted-foreground">
            <span>Press </span>
            <kbd className="px-1 py-0.5 rounded bg-accent text-xs">Esc</kbd>
            <span> to close</span>
          </div>
        </Command>
      </div>
    </div>
  );
}

function getCommandIcon(icon: string | undefined): string {
  const iconMap: Record<string, string> = {
    Search: '🔍',
    FilePlus: '📄',
    CheckSquare: '✅',
    FolderPlus: '📁',
    Lightbulb: '💡',
    Sidebar: '📑',
    Globe: '🌌',
    Home: '🏠',
    ListTodo: '📋',
  };
  return iconMap[icon || ''] || '▸';
}
