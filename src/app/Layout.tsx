/**
 * 3-pane shell — left sidebar (file tree + recent), main editor (with tabs),
 * right dock (backlinks). Toggles via the UI store.
 */

import { useEffect } from 'react';
import { PanelLeft, PanelRight, Search, CalendarPlus, Settings as SettingsIcon, Network } from 'lucide-react';
import { useUIStore } from '@/store/ui';
import { useNavStore } from '@/store/nav';
import { usePages, usePageById } from '@/hooks/usePages';
import { useSearchHotkey } from '@/hooks/useSearchHotkey';
import { useDailyNoteHotkey } from '@/hooks/useDailyNoteHotkey';
import { useSettingsHotkey } from '@/hooks/useSettingsHotkey';
import { useGraphHotkey } from '@/hooks/useGraphHotkey';
import { FileTree } from './Sidebar/FileTree';
import { RecentPane } from './Sidebar/RecentPane';
import { TagPane } from './Sidebar/TagPane';
import { EditorTabs } from './Editor/EditorTabs';
import { CodeMirrorEditor } from './Editor/CodeMirrorEditor';
import { BacklinksPanel } from './Backlinks/BacklinksPanel';
import { GraphView } from './Graph/GraphView';
import { GraphModal } from './Graph/GraphModal';
import { SearchModal } from './Search/SearchModal';
import { SettingsModal } from './Settings/SettingsModal';
import { Button } from '@/ui/Button';
import { EmptyState } from '@/ui/EmptyState';
import { Kbd } from '@/ui/Kbd';
import { startPageIndex } from '@/core/vault/page-index';
import { getOrCreateDailyNote } from '@/core/vault/daily-notes';

export function Layout() {
  const theme = useUIStore((s) => s.theme);
  const sidebarOpen = useUIStore((s) => s.sidebarOpen);
  const toggleSidebar = useUIStore((s) => s.toggleSidebar);
  const rightDockOpen = useUIStore((s) => s.rightDockOpen);
  const toggleRightDock = useUIStore((s) => s.toggleRightDock);
  const rightDockTab = useUIStore((s) => s.rightDockTab);
  const setRightDockTab = useUIStore((s) => s.setRightDockTab);
  const activeTab = useNavStore((s) => s.activeTab);
  const pages = usePages();
  const activePage = usePageById(activeTab);
  const [searchOpen, setSearchOpen] = useSearchHotkey();
  const [settingsOpen, setSettingsOpen] = useSettingsHotkey();
  const [graphOpen, setGraphOpen] = useGraphHotkey();

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

  // Kick off the page-index bridge so the CM autocomplete stays in sync.
  useEffect(() => startPageIndex(), []);

  // Global hotkeys (Cmd+P, Cmd+D, Cmd+,, Cmd+G).
  useSearchHotkey();
  useDailyNoteHotkey();
  useSettingsHotkey();
  useGraphHotkey();

  async function openDailyNote() {
    const note = await getOrCreateDailyNote();
    const nav = useNavStore.getState();
    nav.openPage(note.pageId);
    nav.pushRecent(note.pageId);
  }

  const isEmpty = (pages?.length ?? 0) === 0;

  return (
    <div className="flex h-full w-full flex-col bg-background text-foreground">
      {/* Title bar */}
      <header className="flex h-10 shrink-0 items-center justify-between border-b border-border bg-surface-1 px-3">
        <div className="flex items-center gap-2">
          <Button size="sm" variant="ghost" onClick={toggleSidebar} aria-label="Toggle sidebar">
            <PanelLeft size={14} />
          </Button>
          <span className="text-body font-semibold tracking-tight">Ley</span>
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => setSearchOpen(true)}
            className="flex items-center gap-1.5 rounded-md border border-border bg-surface-2 px-2 py-0.5 text-meta text-muted-foreground hover:bg-surface-3"
          >
            <Search size={12} />
            <span>Search</span>
            <Kbd>⌘</Kbd>
            <Kbd>P</Kbd>
          </button>
          <Button
            size="sm"
            variant="ghost"
            onClick={openDailyNote}
            aria-label="Open daily note"
            title="Open today's daily note"
          >
            <CalendarPlus size={14} />
            <Kbd>⌘D</Kbd>
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setGraphOpen(true)}
            aria-label="Graph view"
            title="Graph view (⌘G)"
          >
            <Network size={14} />
            <Kbd>⌘G</Kbd>
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setSettingsOpen(true)}
            aria-label="Settings"
            title="Settings (⌘,)"
          >
            <SettingsIcon size={14} />
          </Button>
          <Button size="sm" variant="ghost" onClick={toggleRightDock} aria-label="Toggle right dock">
            <PanelRight size={14} />
          </Button>
        </div>
      </header>

      {/* Main: sidebar / editor / right dock */}
      <div className="flex flex-1 overflow-hidden">
        {sidebarOpen && (
          <aside className="flex w-64 shrink-0 flex-col gap-4 overflow-y-auto border-r border-border bg-surface-1 py-3">
            <FileTree />
            <div className="mx-2 border-t border-border" />
            <RecentPane />
            <div className="mx-2 border-t border-border" />
            <TagPane />
          </aside>
        )}

        <main className="flex min-w-0 flex-1 flex-col">
          <EditorTabs />
          {activeTab && activePage ? (
            <CodeMirrorEditor key={activeTab} pageId={activeTab} initialContent={activePage.content} />
          ) : (
            <div className="flex flex-1 items-center justify-center">
              <EmptyState
                title={isEmpty ? 'No pages yet' : 'No page selected'}
                description={
                  isEmpty
                    ? 'Press the + button in the sidebar to create your first page.'
                    : 'Pick a page from the sidebar or recent list.'
                }
              />
            </div>
          )}
        </main>

        {rightDockOpen && (
          <aside className="flex w-80 shrink-0 flex-col border-l border-border bg-surface-1">
            <div className="flex h-9 shrink-0 items-center gap-1 border-b border-border px-2">
              {(['backlinks', 'graph'] as const).map((t) => (
                <button
                  key={t}
                  type="button"
                  onClick={() => setRightDockTab(t)}
                  className={
                    rightDockTab === t
                      ? 'rounded-sm bg-surface-3 px-2 py-0.5 text-meta font-medium text-foreground'
                      : 'rounded-sm px-2 py-0.5 text-meta text-muted-foreground hover:bg-surface-2 hover:text-foreground'
                  }
                >
                  {t.charAt(0).toUpperCase() + t.slice(1)}
                </button>
              ))}
            </div>
            <div className="min-h-0 flex-1">
              {rightDockTab === 'backlinks' ? (
                <BacklinksPanel pageId={activeTab} />
              ) : (
                <GraphView activePageId={activeTab} />
              )}
            </div>
          </aside>
        )}
      </div>
      <SearchModal open={searchOpen} onClose={() => setSearchOpen(false)} />
      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <GraphModal open={graphOpen} onClose={() => setGraphOpen(false)} />
    </div>
  );
}