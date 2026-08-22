/**
 * 3-pane shell — left sidebar (file tree + recent), main editor (with tabs),
 * right dock (backlinks). Toggles via the UI store.
 */

import {
  lazy,
  Suspense,
  useEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";
import * as Dialog from "@radix-ui/react-dialog";
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
  Ellipsis,
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
import type {
  PromotedLearningNoteDraft,
  PromotedSessionNoteDraft,
} from "@/features/agent-memory/types";
import { promoteLearningNote } from "@/features/agent-memory/promote-learning-note";
import { promoteSessionNote } from "@/features/agent-memory/promote-session-note";
import {
  linkSessionToCanvas,
  type SessionCanvasLinkRequest,
} from "@/features/agent-memory/link-session-canvas";
import { primaryModifierLabel, shortcutLabel } from "@/shared/lib/shortcut";
import { useMediaQuery } from "@/shared/hooks/useMediaQuery";

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
  const setSidebarOpen = useUIStore((s) => s.setSidebarOpen);
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
  const [canvasTargetPath, setCanvasTargetPath] = useState<string | null>(null);
  const [collectionRequest, setCollectionRequest] =
    useState<CollectionRequest | null>(null);
  const [workspacesOpen, setWorkspacesOpen] = useState(false);
  const [agentMemoryOpen, setAgentMemoryOpen] = useState(false);
  const isNarrowViewport = useMediaQuery("(max-width: 767px)");
  const sidebarToggleRef = useRef<HTMLButtonElement>(null);
  const desktopSidebarPreference = useRef(isNarrowViewport ? true : sidebarOpen);
  const wasNarrowViewport = useRef(isNarrowViewport);
  const [splitPercent, setSplitPercent] = useState(() => {
    const saved = Number(localStorage.getItem("ley:split-percent"));
    return Number.isFinite(saved) && saved >= 28 && saved <= 72 ? saved : 50;
  });

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  useEffect(() => {
    if (wasNarrowViewport.current === isNarrowViewport) {
      if (!isNarrowViewport) desktopSidebarPreference.current = sidebarOpen;
      return;
    }

    if (isNarrowViewport) {
      desktopSidebarPreference.current = sidebarOpen;
      setSidebarOpen(false);
    } else {
      setSidebarOpen(desktopSidebarPreference.current);
    }
    wasNarrowViewport.current = isNarrowViewport;
  }, [isNarrowViewport, setSidebarOpen, sidebarOpen]);

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

  async function promoteSessionToNote(draft: PromotedSessionNoteDraft) {
    const { page: note } = await promoteSessionNote(draft);
    const nav = useNavStore.getState();
    nav.openPage(note.id);
    nav.pushRecent(note.id);
    setAgentMemoryOpen(false);
  }

  async function linkAgentSessionToCanvas(request: SessionCanvasLinkRequest) {
    const result = await linkSessionToCanvas(request);
    setAgentMemoryOpen(false);
    setCanvasTargetPath(result.canvas.path);
    setCanvasOpen(true);
  }

  function openCanvas() {
    setCanvasTargetPath(null);
    setCanvasOpen(true);
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
  const modifier = primaryModifierLabel();
  const sidebarContents = (
    <>
      <FileTree onNewPage={(folder) => {
        if (isNarrowViewport) setSidebarOpen(false);
        setNewNoteFolder(folder ?? "");
        setNewNoteOpen(true);
      }} />
      <div className="mx-2 border-t border-border" />
      <BookmarksPane onOpenSearch={openSearch} onOpenCollection={(search) => setCollectionRequest({ query: search.query, title: search.name, savedSearchId: search.id, table: search.table })} />
      <div className="mx-2 border-t border-border" />
      <RecentPane />
      <div className="mx-2 border-t border-border" />
      <TagPane />
    </>
  );

  return (
    <div className="flex h-full w-full flex-col bg-background text-foreground">
      {/* Title bar */}
      <header className="app-chrome flex h-11 shrink-0 items-center justify-between gap-3 px-3">
        <div className="flex min-w-0 items-center gap-2">
          <Button
            ref={sidebarToggleRef}
            size="sm"
            variant="ghost"
            onClick={toggleSidebar}
            aria-label="Toggle sidebar"
            aria-expanded={sidebarOpen}
            aria-controls={isNarrowViewport ? "mobile-vault-navigation" : undefined}
            title="Toggle sidebar"
          >
            <PanelLeft size={14} aria-hidden="true" />
          </Button>
          <span className="text-body font-semibold tracking-tight">Ley</span>
          <span
            className="hidden max-w-36 truncate text-micro text-muted-foreground sm:inline"
            title={vaultName}
          >
            {vaultName}
          </span>
        </div>
        <nav
          className="flex min-w-0 items-center gap-1"
          aria-label="Workspace actions"
        >
          <button
            type="button"
            onClick={() => openSearch()}
            aria-label="Open note"
            title={`Open note (${shortcutLabel("O")})`}
            className="flex h-7 touch-manipulation items-center gap-1.5 rounded-md border border-border bg-surface-2/80 px-2 text-meta text-muted-foreground outline-none transition-[transform,background-color,border-color,color] hover:border-border-strong hover:bg-surface-3 hover:text-foreground active:scale-[0.98] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
          >
            <Search size={12} aria-hidden="true" />
            <span className="hidden sm:inline">Open note</span>
            <span className="hidden sm:flex">
              <Kbd>{modifier}</Kbd>
              <Kbd>O</Kbd>
            </span>
          </button>
          <span
            className="mx-0.5 hidden h-4 w-px bg-border md:block"
            aria-hidden="true"
          />
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              setNewNoteFolder("");
              setNewNoteOpen(true);
            }}
            aria-label="New note"
            title={`New note (${shortcutLabel("N")})`}
          >
            <FilePlus2 size={14} aria-hidden="true" />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={openDailyNote}
            aria-label="Open daily note"
            title={`Open today's daily note (${shortcutLabel("D")})`}
          >
            <CalendarPlus size={14} aria-hidden="true" />
          </Button>
          <span
            className="mx-0.5 hidden h-4 w-px bg-border sm:block"
            aria-hidden="true"
          />
          <Button
            size="sm"
            variant="ghost"
            className="hidden sm:inline-flex"
            onClick={openCanvas}
            aria-label="Canvas"
            title="Canvas"
          >
            <LayoutDashboard size={14} aria-hidden="true" />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="hidden sm:inline-flex"
            onClick={() => setWorkspacesOpen(true)}
            aria-label="Workspace layouts"
            title="Workspace layouts"
          >
            <PanelsTopLeft size={14} aria-hidden="true" />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="hidden sm:inline-flex"
            onClick={() => setAgentMemoryOpen(true)}
            aria-label="Agent Memory"
            title="Agent Memory"
          >
            <BrainCircuit size={14} aria-hidden="true" />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="hidden sm:inline-flex"
            onClick={() => setGraphOpen(true)}
            aria-label="Graph view"
            title={`Graph view (${shortcutLabel("G")})`}
          >
            <Network size={14} aria-hidden="true" />
          </Button>
          <span
            className="mx-0.5 hidden h-4 w-px bg-border md:block"
            aria-hidden="true"
          />
          <Button
            size="sm"
            variant="ghost"
            className="hidden sm:inline-flex"
            onClick={() => setSettingsOpen(true)}
            aria-label="Settings"
            title={`Settings (${shortcutLabel(",")})`}
          >
            <SettingsIcon size={14} aria-hidden="true" />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="hidden lg:inline-flex"
            onClick={toggleRightDock}
            aria-label="Toggle right dock"
          >
            <PanelRight size={14} aria-hidden="true" />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="sm:hidden"
            onClick={() => setCommandOpen(true)}
            aria-label="More workspace actions"
            title="More workspace actions"
          >
            <Ellipsis size={16} aria-hidden="true" />
          </Button>
        </nav>
      </header>

      {/* Main: sidebar / editor / right dock */}
      <div className="flex flex-1 overflow-hidden">
        {isNarrowViewport ? (
          <Dialog.Root open={sidebarOpen} onOpenChange={setSidebarOpen}>
            <Dialog.Portal>
              <Dialog.Overlay className="app-modal-overlay fixed inset-x-0 bottom-0 top-11 z-20" />
              <Dialog.Content id="mobile-vault-navigation" aria-describedby={undefined} onCloseAutoFocus={(event) => { event.preventDefault(); sidebarToggleRef.current?.focus(); }} className="app-sidebar app-mobile-sidebar fixed inset-y-11 left-0 z-30 flex w-[min(18rem,calc(100vw-3rem))] flex-col overflow-hidden border-r border-border shadow-menu outline-none">
                <div className="flex h-11 shrink-0 items-center justify-between border-b border-border/70 px-3">
                  <Dialog.Title className="text-body font-semibold tracking-tight">Vault navigation</Dialog.Title>
                  <Dialog.Close asChild><Button size="sm" variant="ghost" aria-label="Close sidebar" title="Close sidebar"><X size={14} aria-hidden="true" /></Button></Dialog.Close>
                </div>
                <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto overscroll-contain py-3 pb-[max(0.75rem,env(safe-area-inset-bottom))]">{sidebarContents}</div>
              </Dialog.Content>
            </Dialog.Portal>
          </Dialog.Root>
        ) : sidebarOpen ? (
          <aside className="app-sidebar flex w-64 shrink-0 flex-col gap-4 overflow-y-auto border-r border-border py-3">{sidebarContents}</aside>
        ) : null}

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
          <aside className="app-sidebar hidden w-80 shrink-0 flex-col border-l border-border lg:flex">
            <div className="flex h-10 shrink-0 items-center gap-1 border-b border-border/70 px-2">
              {(["backlinks", "outline", "history", "graph"] as const).map(
                (t) => (
                  <button
                    key={t}
                    type="button"
                    onClick={() => setRightDockTab(t)}
                    className={
                      rightDockTab === t
                        ? "rounded-md bg-surface-3 px-2 py-1 text-meta font-medium text-foreground shadow-sm outline-none transition-transform active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
                        : "rounded-md px-2 py-1 text-meta text-muted-foreground outline-none transition-[transform,background-color,color] hover:bg-surface-2 hover:text-foreground active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
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
              onOpenNote={(id) => {
                const nav = useNavStore.getState();
                nav.openPage(id);
                nav.pushRecent(id);
              }}
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
        onCanvas={openCanvas}
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
            <CanvasModal
              open
              initialPath={canvasTargetPath}
              onClose={() => {
                setCanvasOpen(false);
                setCanvasTargetPath(null);
              }}
            />
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
              onPromoteSession={promoteSessionToNote}
              onLinkSessionCanvas={linkAgentSessionToCanvas}
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
          className="rounded p-0.5 outline-none transition-transform hover:bg-surface-3 hover:text-foreground active:scale-90 motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
          aria-label="Close split"
          title="Close split"
        >
          <X size={12} aria-hidden="true" />
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
      className="relative z-10 hidden w-px shrink-0 cursor-col-resize touch-none bg-border outline-none before:absolute before:inset-y-0 before:-left-1 before:w-2 hover:bg-primary focus-visible:bg-primary focus-visible:ring-2 focus-visible:ring-primary/35 lg:block"
    />
  );
}
