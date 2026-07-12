/**
 * 3-pane shell — left sidebar (file tree + recent), main editor (with tabs),
 * right dock (backlinks). Toggles via the UI store.
 */

import { lazy, Suspense, useEffect, useState } from 'react';
import { PanelLeft, PanelRight, Search, CalendarPlus, FilePlus2, Settings as SettingsIcon, LayoutDashboard, Network } from 'lucide-react';
import { useUIStore } from '@/shared/state/ui';
import { useNavStore } from '@/shared/state/nav';
import { usePages, usePageById } from '@/features/notes/usePages';
import { useSearchHotkey } from '@/features/search/useSearchHotkey';
import { useDailyNoteHotkey } from '@/features/notes/useDailyNoteHotkey';
import { useSettingsHotkey } from '@/features/settings/useSettingsHotkey';
import { useGraphHotkey } from '@/features/graph/useGraphHotkey';
import { useCommandPaletteHotkey } from '@/features/commands/useCommandPaletteHotkey';
import { FileTree } from '@/features/sidebar/FileTree';
import { RecentPane } from '@/features/sidebar/RecentPane';
import { TagPane } from '@/features/sidebar/TagPane';
import { EditorTabs } from '@/features/editor/EditorTabs';
import { BacklinksPanel } from '@/features/backlinks/BacklinksPanel';
import { SearchModal } from '@/features/search/SearchModal';
import { CommandPalette } from '@/features/commands/CommandPalette';
import { NewNoteModal } from '@/features/editor/NewNoteModal';
import { Button } from '@/shared/components/Button';
import { EmptyState } from '@/shared/components/EmptyState';
import { Kbd } from '@/shared/components/Kbd';
import { startPageIndex } from '@/core/vault/page-index';
import { startSearchIndex } from '@/core/index/search';
import { getOrCreateDailyNote } from '@/core/vault/daily-notes';
import { db } from '@/infrastructure/database/db';
import { OutlinePanel } from '@/features/outline/OutlinePanel';
import { RevisionPanel } from '@/features/history/RevisionPanel';
import { FeatureErrorBoundary } from '@/shared/components/FeatureErrorBoundary';

const GraphView = lazy(() => import('@/features/graph/GraphView').then((module) => ({ default: module.GraphView })));
const GraphModal = lazy(() => import('@/features/graph/GraphModal').then((module) => ({ default: module.GraphModal })));
const SettingsModal = lazy(() => import('@/features/settings/SettingsModal').then((module) => ({ default: module.SettingsModal })));
const NoteWorkspace = lazy(() => import('@/features/editor/NoteWorkspace').then((module) => ({ default: module.NoteWorkspace })));
const CanvasModal = lazy(() => import('@/features/canvas/CanvasModal').then((module) => ({ default: module.CanvasModal })));

export function Layout({
  vaultMode,
  vaultName,
  onRefreshVault,
  onSwitchVault,
}: {
  vaultMode: 'desktop' | 'browser-folder' | 'browser-local';
  vaultName: string;
  onRefreshVault: () => Promise<{ noteCount: number } | null>;
  onSwitchVault: () => Promise<void>;
}) {
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
  const [commandOpen, setCommandOpen] = useCommandPaletteHotkey();
  const [newNoteOpen, setNewNoteOpen] = useState(false);
  const [newNoteFolder, setNewNoteFolder] = useState('');
  const [canvasOpen, setCanvasOpen] = useState(false);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

  // Keep the disposable navigation and search indexes synchronized with the vault.
  useEffect(() => {
    const stopPageIndex = startPageIndex();
    const stopSearchIndex = startSearchIndex();
    return () => {
      stopPageIndex();
      stopSearchIndex();
    };
  }, []);

  // Global hotkeys not already owned by their modal hooks.
  useDailyNoteHotkey();

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'n') {
        event.preventDefault();
        setNewNoteFolder('');
        setNewNoteOpen(true);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

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
          <Button size="sm" variant="ghost" onClick={toggleSidebar} aria-label="Toggle sidebar" title="Toggle sidebar">
            <PanelLeft size={14} />
          </Button>
          <span className="text-body font-semibold tracking-tight">Ley</span>
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => setSearchOpen(true)}
            aria-label="Open note"
            title="Open note (⌘O)"
            className="flex items-center gap-1.5 rounded-md border border-border bg-surface-2 px-2 py-0.5 text-meta text-muted-foreground hover:bg-surface-3"
          >
            <Search size={12} />
            <span className="hidden sm:inline">Open note</span>
            <span className="hidden sm:flex"><Kbd>⌘</Kbd><Kbd>O</Kbd></span>
          </button>
          <Button size="sm" variant="ghost" onClick={() => { setNewNoteFolder(''); setNewNoteOpen(true); }} aria-label="New note" title="New note (⌘N)">
            <FilePlus2 size={14} /><span className="hidden xl:inline"><Kbd>⌘N</Kbd></span>
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={openDailyNote}
            aria-label="Open daily note"
            title="Open today's daily note"
          >
            <CalendarPlus size={14} />
            <span className="hidden md:inline"><Kbd>⌘D</Kbd></span>
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setCanvasOpen(true)} aria-label="Canvas" title="Canvas">
            <LayoutDashboard size={14} />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setGraphOpen(true)}
            aria-label="Graph view"
            title="Graph view (⌘G)"
          >
            <Network size={14} />
            <span className="hidden md:inline"><Kbd>⌘G</Kbd></span>
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
          <Button size="sm" variant="ghost" className="hidden lg:inline-flex" onClick={toggleRightDock} aria-label="Toggle right dock">
            <PanelRight size={14} />
          </Button>
        </div>
      </header>

      {/* Main: sidebar / editor / right dock */}
      <div className="flex flex-1 overflow-hidden">
        {sidebarOpen && (
          <aside className="fixed inset-y-10 left-0 z-30 flex w-72 shrink-0 flex-col gap-4 overflow-y-auto border-r border-border bg-surface-1 py-3 shadow-menu md:static md:w-64 md:shadow-none">
            <FileTree onNewPage={(folder) => { setNewNoteFolder(folder ?? ''); setNewNoteOpen(true); }} />
            <div className="mx-2 border-t border-border" />
            <RecentPane />
            <div className="mx-2 border-t border-border" />
            <TagPane />
          </aside>
        )}

        <main className="flex min-w-0 flex-1 flex-col">
          <EditorTabs />
          {activeTab && activePage?.id === activeTab ? (
            <FeatureErrorBoundary feature="Editor" resetKey={activeTab}><Suspense fallback={<PanelLoading label="Opening note…" />}><NoteWorkspace key={activeTab} page={activePage} /></Suspense></FeatureErrorBoundary>
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
          <aside className="hidden w-80 shrink-0 flex-col border-l border-border bg-surface-1 lg:flex">
            <div className="flex h-9 shrink-0 items-center gap-1 border-b border-border px-2">
              {(['backlinks', 'outline', 'history', 'graph'] as const).map((t) => (
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
              ) : rightDockTab === 'outline' ? (
                <OutlinePanel page={activePage} />
              ) : rightDockTab === 'history' ? (
                <RevisionPanel pageId={activeTab} />
              ) : (
                <Suspense fallback={<PanelLoading label="Building local graph…" />}><GraphView activePageId={activeTab} onOpenFullGraph={() => setGraphOpen(true)} /></Suspense>
              )}
            </div>
          </aside>
        )}
      </div>
      <SearchModal open={searchOpen} onClose={() => setSearchOpen(false)} />
      {settingsOpen && <FeatureErrorBoundary feature="Settings" overlay><Suspense fallback={null}><SettingsModal open vaultMode={vaultMode} vaultName={vaultName} onRefreshVault={onRefreshVault} onSwitchVault={onSwitchVault} onClose={() => setSettingsOpen(false)} /></Suspense></FeatureErrorBoundary>}
      {graphOpen && <FeatureErrorBoundary feature="Graph" overlay><Suspense fallback={null}><GraphModal open onClose={() => setGraphOpen(false)} /></Suspense></FeatureErrorBoundary>}
      <CommandPalette
        open={commandOpen}
        onClose={() => setCommandOpen(false)}
        onNewNote={() => { setNewNoteFolder(''); setNewNoteOpen(true); }}
        onQuickSwitcher={() => setSearchOpen(true)}
        onDailyNote={openDailyNote}
        onGraph={() => setGraphOpen(true)}
        onCanvas={() => setCanvasOpen(true)}
        onSettings={() => setSettingsOpen(true)}
        onToggleSidebar={toggleSidebar}
        onToggleRightDock={toggleRightDock}
        onSetTheme={(nextTheme) => { useUIStore.getState().setTheme(nextTheme); void db.settings.put({ key: 'theme', value: nextTheme }); }}
      />
      <NewNoteModal open={newNoteOpen} initialFolder={newNoteFolder} onClose={() => setNewNoteOpen(false)} />
      {canvasOpen && <FeatureErrorBoundary feature="Canvas" overlay><Suspense fallback={null}><CanvasModal open onClose={() => setCanvasOpen(false)} /></Suspense></FeatureErrorBoundary>}
    </div>
  );
}

function PanelLoading({ label }: { label: string }) {
  return <div className="flex h-full items-center justify-center text-micro text-muted-foreground">{label}</div>;
}
