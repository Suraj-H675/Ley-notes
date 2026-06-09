import { useEffect, useRef, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { Command } from 'cmdk';
import { useSearchStore } from '@/store';
import { useCommands } from '@/hooks/useCommands';
import { db } from '@/lib/db';
import {
  FilePlus,
  CheckSquare,
  FolderPlus,
  Lightbulb,
  PanelLeft,
  Globe,
  Home,
  ListTodo,
  Search,
} from 'lucide-react';
import type { ReactNode } from 'react';

const ICON_MAP: Record<string, ReactNode> = {
  Search: <Search className="h-3.5 w-3.5" />,
  FilePlus: <FilePlus className="h-3.5 w-3.5" />,
  CheckSquare: <CheckSquare className="h-3.5 w-3.5" />,
  FolderPlus: <FolderPlus className="h-3.5 w-3.5" />,
  Lightbulb: <Lightbulb className="h-3.5 w-3.5" />,
  Sidebar: <PanelLeft className="h-3.5 w-3.5" />,
  Globe: <Globe className="h-3.5 w-3.5" />,
  Home: <Home className="h-3.5 w-3.5" />,
  ListTodo: <ListTodo className="h-3.5 w-3.5" />,
};

function CommandIcon({ name }: { name?: string }) {
  if (name && ICON_MAP[name]) return <>{ICON_MAP[name]}</>;
  return <Search className="h-3.5 w-3.5" />;
}

export function CommandPalette() {
  const navigate = useNavigate();
  const { isOpen, closeSearch, query, setQuery } = useSearchStore();
  const commands = useCommands();
  const inputRef = useRef<HTMLInputElement>(null);

  const searchResults = useLiveQuery(
    async () => {
      if (!query || query.length < 2) return [];
      const lowerQuery = query.toLowerCase();
      const nodes = await db.nodes.where('isArchived').equals(0).toArray();
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
    return {
      create: commands.filter((c) => c.category === 'create'),
      navigation: commands.filter((c) => c.category === 'navigation'),
      action: commands.filter((c) => c.category === 'action'),
    };
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

  const renderItem = (item: {
    id: string;
    icon?: string;
    label: string;
    execute: () => void | Promise<void>;
  }) => (
    <Command.Item
      key={item.id}
      value={item.label}
      onSelect={() => {
        item.execute();
        closeSearch();
      }}
      className="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-[13px] text-foreground/85 aria-selected:bg-accent aria-selected:text-foreground"
    >
      <span className="flex h-4 w-4 flex-shrink-0 items-center justify-center text-muted-foreground/80">
        <CommandIcon name={item.icon} />
      </span>
      <span className="truncate">{item.label}</span>
    </Command.Item>
  );

  return (
    <div className="fixed inset-0 z-50">
      <div
        className="absolute inset-0 bg-background/40 backdrop-blur-sm"
        onClick={closeSearch}
      />
      <div className="absolute left-1/2 top-[20%] w-full max-w-xl -translate-x-1/2 animate-slide-down">
        <Command className="overflow-hidden rounded-lg border border-border/80 bg-popover shadow-menu">
          <div className="flex items-center px-3">
            <Search className="h-3.5 w-3.5 text-muted-foreground/60" />
            <Command.Input
              ref={inputRef}
              value={query}
              onValueChange={setQuery}
              placeholder="Search pages, or type a command..."
              className="h-10 w-full bg-transparent px-2.5 text-[13px] text-foreground outline-none placeholder:text-muted-foreground/60"
            />
            <kbd className="rounded border border-border/60 bg-background/40 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground/70">
              esc
            </kbd>
          </div>

          <Command.List className="max-h-[320px] overflow-y-auto px-1.5 pb-2">
            <Command.Empty className="py-6 text-center text-[13px] text-muted-foreground/70">
              No results{query ? ` for '${query}'` : ''}. Press Enter to create a page with this title.
            </Command.Empty>

            {query === '' && (
              <>
                {groupedCommands.create.length > 0 && (
                  <Command.Group
                    heading="Create"
                    className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:tracking-wide [&_[cmdk-group-heading]]:text-muted-foreground/60"
                  >
                    {groupedCommands.create.map(renderItem)}
                  </Command.Group>
                )}

                {groupedCommands.navigation.length > 0 && (
                  <Command.Group
                    heading="Navigate"
                    className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:tracking-wide [&_[cmdk-group-heading]]:text-muted-foreground/60"
                  >
                    {groupedCommands.navigation.map(renderItem)}
                  </Command.Group>
                )}

                {groupedCommands.action.length > 0 && (
                  <Command.Group
                    heading="Actions"
                    className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:tracking-wide [&_[cmdk-group-heading]]:text-muted-foreground/60"
                  >
                    {groupedCommands.action.map(renderItem)}
                  </Command.Group>
                )}
              </>
            )}

            {query !== '' && searchResults && searchResults.length > 0 && (
              <Command.Group
                heading="Pages"
                className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:tracking-wide [&_[cmdk-group-heading]]:text-muted-foreground/60"
              >
                {searchResults.map((node) => (
                  <Command.Item
                    key={node.id}
                    value={node.title}
                    onSelect={() => {
                      navigate(`/page/${node.id}`);
                      closeSearch();
                    }}
                    className="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-[13px] text-foreground/85 aria-selected:bg-accent aria-selected:text-foreground"
                  >
                    <span className="flex h-4 w-4 flex-shrink-0 items-center justify-center text-[13px] leading-none">
                      {node.emoji || (
                        <span className="block h-1 w-1 rounded-full bg-muted-foreground/40" />
                      )}
                    </span>
                    <span className="truncate">{node.title}</span>
                    <span className="ml-auto text-[11px] capitalize text-muted-foreground/60">
                      {node.type}
                    </span>
                  </Command.Item>
                ))}
              </Command.Group>
            )}
          </Command.List>
        </Command>
      </div>
    </div>
  );
}
