import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  CalendarDays,
  FilePlus2,
  FolderSearch,
  Moon,
  LayoutDashboard,
  Network,
  PanelLeft,
  PanelRight,
  PanelsTopLeft,
  Search,
  Settings,
  Bookmark,
  BrainCircuit,
  Sun,
  TableProperties,
} from "lucide-react";
import { Kbd } from "@/shared/components/Kbd";
import { shortcutLabel } from "@/shared/lib/shortcut";
import * as Dialog from "@radix-ui/react-dialog";

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
  onCollection,
  onWorkspaces,
  onAgentMemory,
  onSettings,
  onToggleSidebar,
  onToggleRightDock,
  onSetTheme,
  activeNoteBookmarked,
  onToggleBookmark,
}: {
  open: boolean;
  onClose: () => void;
  onNewNote: () => void;
  onQuickSwitcher: () => void;
  onDailyNote: () => void | Promise<void>;
  onGraph: () => void;
  onCanvas: () => void;
  onCollection: () => void;
  onWorkspaces: () => void;
  onAgentMemory: () => void;
  onSettings: () => void;
  onToggleSidebar: () => void;
  onToggleRightDock: () => void;
  onSetTheme: (theme: "light" | "dark") => void;
  activeNoteBookmarked: boolean | null;
  onToggleBookmark: () => void | Promise<void>;
}) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands = useMemo<PaletteCommand[]>(
    () => [
      {
        id: "new-note",
        label: "Create new note",
        detail: "Add a Markdown note to the vault",
        icon: <FilePlus2 size={15} />,
        shortcut: shortcutLabel("N"),
        run: onNewNote,
      },
      {
        id: "quick-switcher",
        label: "Open quick switcher",
        detail: "Find a note by title or content",
        icon: <FolderSearch size={15} />,
        shortcut: shortcutLabel("O"),
        keywords: "search find note",
        run: onQuickSwitcher,
      },
      {
        id: "daily",
        label: "Open today's daily note",
        detail: "Capture thoughts and activity for today",
        icon: <CalendarDays size={15} />,
        shortcut: shortcutLabel("D"),
        run: onDailyNote,
      },
      {
        id: "graph",
        label: "Open graph view",
        detail: "Explore the connections in this vault",
        icon: <Network size={15} />,
        shortcut: shortcutLabel("G"),
        run: onGraph,
      },
      {
        id: "canvas",
        label: "Open canvas",
        detail: "Arrange notes and thoughts on a spatial board",
        icon: <LayoutDashboard size={15} />,
        keywords: "board spatial json canvas",
        run: onCanvas,
      },
      {
        id: "collection",
        label: "Open notes as a table",
        detail: "Sort and edit Markdown properties across the vault",
        icon: <TableProperties size={15} />,
        keywords: "base database collection properties table",
        run: onCollection,
      },
      {
        id: "workspaces",
        label: "Manage workspace layouts",
        detail: "Save or restore tabs, split panes, and sidebars",
        icon: <PanelsTopLeft size={15} />,
        keywords: "workspace layout panes tabs writing research",
        run: onWorkspaces,
      },
      {
        id: "agent-memory",
        label: "Open Agent Memory",
        detail:
          "Review project sessions, continuity, and evidence-backed lessons",
        icon: <BrainCircuit size={15} />,
        keywords: "agents mcp sessions lessons second brain context",
        run: onAgentMemory,
      },
      ...(activeNoteBookmarked === null
        ? []
        : [
            {
              id: "note-bookmark",
              label: activeNoteBookmarked
                ? "Remove active note bookmark"
                : "Bookmark active note",
              detail: "Keep this note one click away in Bookmarks",
              icon: (
                <Bookmark
                  size={15}
                  className={activeNoteBookmarked ? "fill-current" : undefined}
                />
              ),
              keywords: "star favorite bookmark pin",
              run: onToggleBookmark,
            },
          ]),
      {
        id: "toggle-left",
        label: "Toggle left sidebar",
        detail: "Show or hide the file explorer",
        icon: <PanelLeft size={15} />,
        run: onToggleSidebar,
      },
      {
        id: "toggle-right",
        label: "Toggle right sidebar",
        detail: "Show or hide contextual panels",
        icon: <PanelRight size={15} />,
        run: onToggleRightDock,
      },
      {
        id: "light",
        label: "Use light theme",
        detail: "Switch the workspace appearance",
        icon: <Sun size={15} />,
        run: () => onSetTheme("light"),
      },
      {
        id: "dark",
        label: "Use dark theme",
        detail: "Switch the workspace appearance",
        icon: <Moon size={15} />,
        run: () => onSetTheme("dark"),
      },
      {
        id: "settings",
        label: "Open settings",
        detail: "Configure Ley and this vault",
        icon: <Settings size={15} />,
        shortcut: shortcutLabel(","),
        run: onSettings,
      },
    ],
    [
      activeNoteBookmarked,
      onAgentMemory,
      onCanvas,
      onCollection,
      onDailyNote,
      onGraph,
      onNewNote,
      onQuickSwitcher,
      onSetTheme,
      onSettings,
      onToggleBookmark,
      onToggleRightDock,
      onToggleSidebar,
      onWorkspaces,
    ],
  );

  const matches = useMemo(() => {
    const needle = query.toLowerCase().replace(/\s+/g, "");
    if (!needle) return commands;
    return commands.filter((command) =>
      fuzzyMatch(
        `${command.label} ${command.detail} ${command.keywords ?? ""}`,
        needle,
      ),
    );
  }, [commands, query]);

  useEffect(() => {
    if (!open) return;
    queueMicrotask(() => {
      setQuery("");
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
    <Dialog.Root
      open={open}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-[70]" />
        <Dialog.Content
          aria-describedby={undefined}
          className="app-modal-surface app-modal-top fixed left-1/2 top-[12vh] z-[71] w-[600px] max-w-[calc(100vw-24px)] -translate-x-1/2 overflow-hidden rounded-xl border outline-none"
        >
          <Dialog.Title className="sr-only">Command palette</Dialog.Title>
          <div className="flex items-center gap-2 border-b border-border px-3">
            <Search size={15} className="text-muted-foreground" />
            <input
              ref={inputRef}
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setSelected(0);
              }}
              onKeyDown={(event) => {
                if (event.key === "Escape") onClose();
                if (event.key === "ArrowDown") {
                  event.preventDefault();
                  setSelected((value) =>
                    Math.min(value + 1, matches.length - 1),
                  );
                }
                if (event.key === "ArrowUp") {
                  event.preventDefault();
                  setSelected((value) => Math.max(value - 1, 0));
                }
                if (event.key === "Enter" && matches[selected]) {
                  event.preventDefault();
                  void run(matches[selected]);
                }
              }}
              placeholder="Type a command…"
              className="h-12 flex-1 bg-transparent text-body text-foreground outline-none placeholder:text-subtle-foreground"
            />
            <Kbd>esc</Kbd>
          </div>
          <div className="max-h-[430px] overflow-y-auto p-1.5">
            {matches.length === 0 ? (
              <div className="px-3 py-10 text-center text-meta text-muted-foreground">
                No matching commands
              </div>
            ) : (
              matches.map((command, index) => (
                <button
                  key={command.id}
                  type="button"
                  onMouseEnter={() => setSelected(index)}
                  onClick={() => void run(command)}
                  className={`flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-left ${selected === index ? "bg-surface-3 text-foreground" : "text-muted-foreground-strong hover:bg-surface-2"}`}
                >
                  <span className="text-secondary">{command.icon}</span>
                  <span className="min-w-0 flex-1">
                    <span className="block text-meta font-medium">
                      {command.label}
                    </span>
                    <span className="block truncate text-micro text-muted-foreground">
                      {command.detail}
                    </span>
                  </span>
                  {command.shortcut && <Kbd>{command.shortcut}</Kbd>}
                </button>
              ))
            )}
          </div>
          <div className="flex items-center justify-between border-t border-border px-3 py-2 text-micro text-muted-foreground">
            <span>Commands are available from anywhere in Ley</span>
            <span className="flex gap-1">
              <Kbd>↑</Kbd>
              <Kbd>↓</Kbd>
              <Kbd>↵</Kbd>
            </span>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
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
