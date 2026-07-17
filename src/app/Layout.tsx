/**
 * 3-pane shell — left sidebar (file tree + recent), main editor (with tabs),
 * right dock (backlinks). Toggles via the UI store.
 */

import {
  lazy,
  Suspense,
  useEffect,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";
import {
  PanelLeft,
  PanelRight,
  PanelsTopLeft,
  Search,
  CalendarPlus,
  FilePlus2,
  Settings as SettingsIcon,
  LayoutDashboard,
  Network,
  X,
  BrainCircuit,
} from "lucide-react";
import { useUIStore } from "@/shared/state/ui";
import { useNavStore } from "@/shared/state/nav";
import { usePages, usePageById } from "@/features/notes/usePages";
import { useSearchHotkey } from "@/features/search/useSearchHotkey";
import { useDailyNoteHotkey } from "@/features/notes/useDailyNoteHotkey";
import { useSettingsHotkey } from "@/features/settings/useSettingsHotkey";
import { useGraphHotkey } from "@/features/graph/useGraphHotkey";
import { useCommandPaletteHotkey } from "@/features/commands/useCommandPaletteHotkey";
import { FileTree } from "@/features/sidebar/FileTree";
import { RecentPane } from "@/features/sidebar/RecentPane";
import { BookmarksPane } from "@/features/bookmarks/BookmarksPane";
import { TagPane } from "@/features/sidebar/TagPane";
import { EditorTabs } from "@/features/editor/EditorTabs";
import { BacklinksPanel } from "@/features/backlinks/BacklinksPanel";
import { SearchModal } from "@/features/search/SearchModal";
import { CommandPalette } from "@/features/commands/CommandPalette";
import { NewNoteModal } from "@/features/editor/NewNoteModal";
import { Button } from "@/shared/components/Button";
import { EmptyState } from "@/shared/components/EmptyState";
import { Kbd } from "@/shared/components/Kbd";
import { startPageIndex } from "@/core/vault/page-index";
import { startSearchIndex } from "@/core/index/search";
import { getOrCreateDailyNote } from "@/core/vault/daily-notes";
import { db } from "@/infrastructure/database/db";
import { OutlinePanel } from "@/features/outline/OutlinePanel";
import { RevisionPanel } from "@/features/history/RevisionPanel";
import { FeatureErrorBoundary } from "@/shared/components/FeatureErrorBoundary";
import { useIsPageBookmarked } from "@/features/bookmarks/useNoteBookmarks";
import { togglePageBookmark } from "@/core/vault/note-bookmarks";
import { startNavigationSession } from "@/core/vault/navigation-session";
import type { CollectionRequest } from "@/features/collections/CollectionModal";
import type { PromotedLearningNoteDraft } from "@/features/agent-memory/types";
import { promoteLearningNote } from "@/features/agent-memory/promote-learning-note";

const GraphView = lazy(() =>
  import("@/features/graph/GraphView").then((module) => ({
    default: module.GraphView,
  })),
);
const GraphModal = lazy(() =>
  import("@/features/graph/GraphModal").then((module) => ({
    default: module.GraphModal,
  })),
);
const SettingsModal = lazy(() =>
  import("@/features/settings/SettingsModal").then((module) => ({
    default: module.SettingsModal,
  })),
);
const NoteWorkspace = lazy(() =>
  import("@/features/editor/NoteWorkspace").then((module) => ({
    default: module.NoteWorkspace,
  })),
);
const CanvasModal = lazy(() =>
  import("@/features/canvas/CanvasModal").then((module) => ({
    default: module.CanvasModal,
  })),
);
const CollectionModal = lazy(() =>
  import("@/features/collections/CollectionModal").then((module) => ({
    default: module.CollectionModal,
  })),
);
const WorkspaceModal = lazy(() =>
  import("@/features/workspaces/WorkspaceModal").then((module) => ({
    default: module.WorkspaceModal,
  })),
);
const AgentMemoryWorkspace = lazy(() =>
  import("@/features/agent-memory/AgentMemoryWorkspace").then((module) => ({
    default: module.AgentMemoryWorkspace,
  })),
);

export function Layout({
  vaultMode,
  vaultKey,
  vaultName,
  watcherStatus,
  onRefreshVault,
  onSwitchVault,
}: {
  vaultMode: "desktop" | "browser-folder" | "browser-local";
  vaultKey: string;
  vaultName: string;
  watcherStatus: "inactive" | "starting" | "watching" | "error";
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
  const primaryTab = useNavStore((s) => s.primaryTab);
  const secondaryTab = useNavStore((s) => s.secondaryTab);
  const activePane = useNavStore((s) => s.activePane);
  const focusPane = useNavStore((s) => s.focusPane);
  const closeSplit = useNavStore((s) => s.closeSplit);
  const pages = usePages();
  const activePage = usePageById(activeTab);
  const primaryPage = usePageById(primaryTab);
  const secondaryPage = usePageById(secondaryTab);
  const activePageBookmarked = useIsPageBookmarked(activeTab);
  const [searchOpen, setSearchOpen] = useSearchHotkey();
  const [searchInitialQuery, setSearchInitialQuery] = useState("");
  const [settingsOpen, setSettingsOpen] = useSettingsHotkey();
  const [graphOpen, setGraphOpen] = useGraphHotkey();
  const [commandOpen, setCommandOpen] = useCommandPaletteHotkey();
  const [newNoteOpen, setNewNoteOpen] = useState(false);
  const [newNoteFolder, setNewNoteFolder] = useState("");
  const [canvasOpen, setCanvasOpen] = useState(false);
  const [collectionRequest, setCollectionRequest] =
    useState<CollectionRequest | null>(null);
  const [workspacesOpen, setWorkspacesOpen] = useState(false);
  const [agentMemoryOpen, setAgentMemoryOpen] = useState(false);
  const [splitPercent, setSplitPercent] = useState(() => {
    const saved = Number(localStorage.getItem("ley:split-percent"));
    return Number.isFinite(saved) && saved >= 28 && saved <= 72 ? saved : 50;
  });

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
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

  useEffect(() => {
    let current = true;
    let dispose: () => void = () => undefined;
    void startNavigationSession(() => current)
      .then((stop) => {
        if (!current) {
          stop();
          return;
        }
        dispose = stop;
      })
      .catch((error) =>
        console.error(
          "[navigation] Could not restore workspace session",
          error,
        ),
      );
    return () => {
      current = false;
      dispose();
    };
  }, [vaultKey]);

  // Global hotkeys not already owned by their modal hooks.
  useDailyNoteHotkey();

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setNewNoteFolder("");
        setNewNoteOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  async function openDailyNote() {
    const note = await getOrCreateDailyNote();
    const nav = useNavStore.getState();
    nav.openPage(note.pageId);
    nav.pushRecent(note.pageId);
  }

  async function promoteLearningToNote(draft: PromotedLearningNoteDraft) {
    const { page: note } = await promoteLearningNote(draft);
    const nav = useNavStore.getState();
    nav.openPage(note.id);
    nav.pushRecent(note.id);
    setAgentMemoryOpen(false);
  }

  function openSearch(query = "") {
    setSearchInitialQuery(query);
    setSearchOpen(true);
  }

  function closeSearch() {
    setSearchOpen(false);
    setSearchInitialQuery("");
  }

  function changeSplitPercent(value: number) {
    setSplitPercent(value);
    localStorage.setItem("ley:split-percent", String(value));
  }

  const isEmpty = (pages?.length ?? 0) === 0;

  return (
    <div className="flex h-full w-full flex-col bg-background text-foreground">
      {/* Title bar */}
      <header className="flex h-10 shrink-0 items-center justify-between border-b border-border bg-surface-1 px-3">
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="ghost"
            onClick={toggleSidebar}
            aria-label="Toggle sidebar"
            title="Toggle sidebar"
          >
            <PanelLeft size={14} />
          </Button>
          <span className="text-body font-semibold tracking-tight">Ley</span>
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => openSearch()}
            aria-label="Open note"
            title="Open note (⌘O)"
            className="flex items-center gap-1.5 rounded-md border border-border bg-surface-2 px-2 py-0.5 text-meta text-muted-foreground hover:bg-surface-3"
          >
            <Search size={12} />
            <span className="hidden sm:inline">Open note</span>
            <span className="hidden sm:flex">
              <Kbd>⌘</Kbd>
              <Kbd>O</Kbd>
            </span>
          </button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              setNewNoteFolder("");
              setNewNoteOpen(true);
            }}
            aria-label="New note"
            title="New note (⌘N)"
          >
            <FilePlus2 size={14} />
            <span className="hidden xl:inline">
              <Kbd>⌘N</Kbd>
            </span>
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={openDailyNote}
            aria-label="Open daily note"
            title="Open today's daily note"
          >
            <CalendarPlus size={14} />
            <span className="hidden md:inline">
              <Kbd>⌘D</Kbd>
            </span>
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setCanvasOpen(true)}
            aria-label="Canvas"
            title="Canvas"
          >
            <LayoutDashboard size={14} />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="hidden sm:inline-flex"
            onClick={() => setWorkspacesOpen(true)}
            aria-label="Workspace layouts"
            title="Workspace layouts"
          >
            <PanelsTopLeft size={14} />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setAgentMemoryOpen(true)}
            aria-label="Agent Memory"
            title="Agent Memory"
          >
            <BrainCircuit size={14} />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setGraphOpen(true)}
            aria-label="Graph view"
            title="Graph view (⌘G)"
          >
            <Network size={14} />
            <span className="hidden md:inline">
              <Kbd>⌘G</Kbd>
            </span>
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
          <Button
            size="sm"
            variant="ghost"
            className="hidden lg:inline-flex"
            onClick={toggleRightDock}
            aria-label="Toggle right dock"
          >
            <PanelRight size={14} />
          </Button>
        </div>
      </header>

      {/* Main: sidebar / editor / right dock */}
      <div className="flex flex-1 overflow-hidden">
        {sidebarOpen && (
          <aside className="fixed inset-y-10 left-0 z-30 flex w-72 shrink-0 flex-col gap-4 overflow-y-auto border-r border-border bg-surface-1 py-3 shadow-menu md:static md:w-64 md:shadow-none">
            <FileTree
              onNewPage={(folder) => {
                setNewNoteFolder(folder ?? "");
                setNewNoteOpen(true);
              }}
            />
            <div className="mx-2 border-t border-border" />
            <BookmarksPane
              onOpenSearch={openSearch}
              onOpenCollection={(search) =>
                setCollectionRequest({
                  query: search.query,
                  title: search.name,
                  savedSearchId: search.id,
                  table: search.table,
                })
              }
            />
            <div className="mx-2 border-t border-border" />
            <RecentPane />
            <div className="mx-2 border-t border-border" />
            <TagPane />
          </aside>
        )}

        <main className="flex min-w-0 flex-1 flex-col">
          <EditorTabs />
          {primaryTab && primaryPage?.id === primaryTab ? (
            <div className="flex min-h-0 flex-1">
              <section
                className={`${secondaryTab && activePane !== "primary" ? "hidden lg:flex" : "flex"} min-w-0 flex-1 flex-col ${secondaryTab ? "lg:flex-none" : ""}`}
                style={
                  secondaryTab ? { flexBasis: `${splitPercent}%` } : undefined
                }
                aria-label="Primary note pane"
                onPointerDownCapture={() => focusPane("primary")}
              >
                {secondaryTab && (
                  <PaneHeader
                    label="Primary"
                    active={activePane === "primary"}
                  />
                )}
                <FeatureErrorBoundary
                  feature="Primary editor"
                  resetKey={primaryTab}
                >
                  <Suspense fallback={<PanelLoading label="Opening note…" />}>
                    <NoteWorkspace
                      key={`primary:${primaryTab}`}
                      page={primaryPage}
                      pane="primary"
                    />
                  </Suspense>
                </FeatureErrorBoundary>
              </section>
              {secondaryTab && secondaryPage?.id === secondaryTab && (
                <>
                  <SplitDivider
                    value={splitPercent}
                    onChange={setSplitPercent}
                  />
                  <section
                    className={`${activePane !== "secondary" ? "hidden lg:flex" : "flex"} min-w-0 flex-1 flex-col lg:flex-none`}
                    style={{ flexBasis: `${100 - splitPercent}%` }}
                    aria-label="Secondary note pane"
                    onPointerDownCapture={() => focusPane("secondary")}
                  >
                    <PaneHeader
                      label="Reference"
                      active={activePane === "secondary"}
                      onClose={closeSplit}
                    />
                    <FeatureErrorBoundary
                      feature="Secondary editor"
                      resetKey={secondaryTab}
                    >
                      <Suspense
                        fallback={<PanelLoading label="Opening reference…" />}
                      >
                        <NoteWorkspace
                          key={`secondary:${secondaryTab}`}
                          page={secondaryPage}
                          pane="secondary"
                        />
                      </Suspense>
                    </FeatureErrorBoundary>
                  </section>
                </>
              )}
            </div>
          ) : (
            <div className="flex flex-1 items-center justify-center">
              <EmptyState
                title={isEmpty ? "No pages yet" : "No page selected"}
                description={
                  isEmpty
                    ? "Press the + button in the sidebar to create your first page."
                    : "Pick a page from the sidebar or recent list."
                }
              />
            </div>
          )}
        </main>

        {rightDockOpen && (
          <aside className="hidden w-80 shrink-0 flex-col border-l border-border bg-surface-1 lg:flex">
            <div className="flex h-9 shrink-0 items-center gap-1 border-b border-border px-2">
              {(["backlinks", "outline", "history", "graph"] as const).map(
                (t) => (
                  <button
                    key={t}
                    type="button"
                    onClick={() => setRightDockTab(t)}
                    className={
                      rightDockTab === t
                        ? "rounded-sm bg-surface-3 px-2 py-0.5 text-meta font-medium text-foreground"
                        : "rounded-sm px-2 py-0.5 text-meta text-muted-foreground hover:bg-surface-2 hover:text-foreground"
                    }
                  >
                    {t.charAt(0).toUpperCase() + t.slice(1)}
                  </button>
                ),
              )}
            </div>
            <div className="min-h-0 flex-1">
              {rightDockTab === "backlinks" ? (
                <BacklinksPanel pageId={activeTab} />
              ) : rightDockTab === "outline" ? (
                <OutlinePanel page={activePage} />
              ) : rightDockTab === "history" ? (
                <RevisionPanel pageId={activeTab} />
              ) : (
                <Suspense
                  fallback={<PanelLoading label="Building local graph…" />}
                >
                  <GraphView
                    activePageId={activeTab}
                    onOpenFullGraph={() => setGraphOpen(true)}
                  />
                </Suspense>
              )}
            </div>
          </aside>
        )}
      </div>
      <SearchModal
        open={searchOpen}
        initialQuery={searchInitialQuery}
        onClose={closeSearch}
        onOpenCollection={(query) =>
          setCollectionRequest({
            query,
            title: query.trim() ? "Query collection" : "All notes",
          })
        }
      />
      {settingsOpen && (
        <FeatureErrorBoundary feature="Settings" overlay>
          <Suspense fallback={null}>
            <SettingsModal
              open
              vaultMode={vaultMode}
              vaultName={vaultName}
              watcherStatus={watcherStatus}
              onRefreshVault={onRefreshVault}
              onSwitchVault={onSwitchVault}
              onClose={() => setSettingsOpen(false)}
            />
          </Suspense>
        </FeatureErrorBoundary>
      )}
      {graphOpen && (
        <FeatureErrorBoundary feature="Graph" overlay>
          <Suspense fallback={null}>
            <GraphModal open onClose={() => setGraphOpen(false)} />
          </Suspense>
        </FeatureErrorBoundary>
      )}
      <CommandPalette
        open={commandOpen}
        onClose={() => setCommandOpen(false)}
        onNewNote={() => {
          setNewNoteFolder("");
          setNewNoteOpen(true);
        }}
        onQuickSwitcher={() => openSearch()}
        onDailyNote={openDailyNote}
        onGraph={() => setGraphOpen(true)}
        onCanvas={() => setCanvasOpen(true)}
        onCollection={() =>
          setCollectionRequest({ query: "", title: "All notes" })
        }
        onWorkspaces={() => setWorkspacesOpen(true)}
        onAgentMemory={() => setAgentMemoryOpen(true)}
        onSettings={() => setSettingsOpen(true)}
        onToggleSidebar={toggleSidebar}
        onToggleRightDock={toggleRightDock}
        onSetTheme={(nextTheme) => {
          useUIStore.getState().setTheme(nextTheme);
          void db.settings.put({ key: "theme", value: nextTheme });
        }}
        activeNoteBookmarked={activeTab ? activePageBookmarked : null}
        onToggleBookmark={async () => {
          if (activeTab) await togglePageBookmark(activeTab);
        }}
      />
      <NewNoteModal
        open={newNoteOpen}
        initialFolder={newNoteFolder}
        onClose={() => setNewNoteOpen(false)}
      />
      {canvasOpen && (
        <FeatureErrorBoundary feature="Canvas" overlay>
          <Suspense fallback={null}>
            <CanvasModal open onClose={() => setCanvasOpen(false)} />
          </Suspense>
        </FeatureErrorBoundary>
      )}
      {collectionRequest && (
        <FeatureErrorBoundary feature="Collection" overlay>
          <Suspense fallback={null}>
            <CollectionModal
              key={`${collectionRequest.savedSearchId ?? "query"}:${collectionRequest.query}`}
              request={collectionRequest}
              onClose={() => setCollectionRequest(null)}
            />
          </Suspense>
        </FeatureErrorBoundary>
      )}
      {workspacesOpen && (
        <FeatureErrorBoundary feature="Workspace layouts" overlay>
          <Suspense fallback={null}>
            <WorkspaceModal
              open
              splitPercent={splitPercent}
              onSplitPercentChange={changeSplitPercent}
              onClose={() => setWorkspacesOpen(false)}
            />
          </Suspense>
        </FeatureErrorBoundary>
      )}
      {agentMemoryOpen && (
        <FeatureErrorBoundary feature="Agent Memory" overlay>
          <Suspense
            fallback={<div className="fixed inset-0 z-[60] bg-background" />}
          >
            <AgentMemoryWorkspace
              open
              vaultMode={vaultMode}
              vaultPath={vaultKey}
              vaultName={vaultName}
              onClose={() => setAgentMemoryOpen(false)}
              onPromoteLearning={promoteLearningToNote}
            />
          </Suspense>
        </FeatureErrorBoundary>
      )}
    </div>
  );
}

function PanelLoading({ label }: { label: string }) {
  return (
    <div className="flex h-full items-center justify-center text-micro text-muted-foreground">
      {label}
    </div>
  );
}

function PaneHeader({
  label,
  active,
  onClose,
}: {
  label: string;
  active: boolean;
  onClose?: () => void;
}) {
  return (
    <div
      className={`flex h-7 shrink-0 items-center justify-between border-b px-3 text-micro ${active ? "border-primary/40 bg-primary/5 text-foreground" : "border-border bg-surface-1 text-muted-foreground"}`}
    >
      <span className="font-medium">{label}</span>
      {onClose && (
        <button
          type="button"
          onClick={onClose}
          className="rounded p-0.5 hover:bg-surface-3 hover:text-foreground"
          aria-label="Close split"
          title="Close split"
        >
          <X size={12} />
        </button>
      )}
    </div>
  );
}

function SplitDivider({
  value,
  onChange,
}: {
  value: number;
  onChange: (value: number) => void;
}) {
  function commit(next: number) {
    const clamped = Math.max(28, Math.min(72, next));
    onChange(clamped);
    localStorage.setItem("ley:split-percent", String(clamped));
  }

  function startResize(event: ReactPointerEvent<HTMLDivElement>) {
    const container = event.currentTarget.parentElement;
    if (!container) return;
    const divider = event.currentTarget;
    divider.setPointerCapture(event.pointerId);
    let latest = value;
    const move = (moveEvent: PointerEvent) => {
      const bounds = container.getBoundingClientRect();
      if (bounds.width > 0) {
        latest = Math.max(
          28,
          Math.min(
            72,
            ((moveEvent.clientX - bounds.left) / bounds.width) * 100,
          ),
        );
        onChange(latest);
      }
    };
    const stop = () => {
      divider.removeEventListener("pointermove", move);
      divider.removeEventListener("pointerup", stop);
      divider.removeEventListener("pointercancel", stop);
      localStorage.setItem("ley:split-percent", String(latest));
    };
    divider.addEventListener("pointermove", move);
    divider.addEventListener("pointerup", stop);
    divider.addEventListener("pointercancel", stop);
  }

  return (
    <div
      role="separator"
      aria-label="Resize note panes"
      aria-orientation="vertical"
      aria-valuemin={28}
      aria-valuemax={72}
      aria-valuenow={Math.round(value)}
      tabIndex={0}
      onPointerDown={startResize}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft" || event.key === "ArrowRight")
          event.preventDefault();
        if (event.key === "ArrowLeft") commit(value - 2);
        if (event.key === "ArrowRight") commit(value + 2);
      }}
      className="relative z-10 hidden w-px shrink-0 cursor-col-resize bg-border outline-none before:absolute before:inset-y-0 before:-left-1 before:w-2 hover:bg-primary focus:bg-primary lg:block"
    />
  );
}
