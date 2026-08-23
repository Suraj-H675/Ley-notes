/**
 * FileTree — collapsible tree of pages, grouped by folder (the part of `path`
 * before the final segment). Right-click or the + button creates a new page;
 * rename/delete via the context menu.
 */

import { useMemo, useState, type DragEvent } from 'react';
import * as ContextMenu from '@radix-ui/react-context-menu';
import * as Dialog from '@radix-ui/react-dialog';
import { Bookmark, ChevronRight, ChevronDown, Columns2, Copy, FileText, FolderClosed, FolderInput, FolderOpen, Link2, Pencil, Plus, Trash2, X } from 'lucide-react';
import { usePages } from '@/features/notes/usePages';
import { useNavStore } from '@/shared/state/nav';
import { deletePage, duplicatePage, movePage, renamePage } from '@/core/vault/pages';
import { cn } from '@/shared/lib/classnames';
import { useUIStore } from '@/shared/state/ui';
import { useTagFilter } from '@/shared/state/tag-filter';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';
import { getActiveVaultKind } from '@/infrastructure/vault/filesystem-vault';
import { togglePageBookmark } from '@/core/vault/note-bookmarks';
import { useBookmarkedPageIds } from '@/features/bookmarks/useNoteBookmarks';

interface TreeNode {
  name: string;
  path: string;
  pages: Array<{ id: string; title: string; path: string }>;
  children: TreeNode[];
}

function buildTree(pages: Array<{ id: string; title: string; path: string }>): TreeNode {
  const root: TreeNode = { name: '', path: '', pages: [], children: [] };

  for (const p of pages) {
    const segments = p.path.split('/');
    segments.pop(); // drop the filename
    let cur = root;
    for (const seg of segments) {
      let next = cur.children.find((c) => c.name === seg);
      if (!next) {
        next = { name: seg, path: cur.path ? `${cur.path}/${seg}` : seg, pages: [], children: [] };
        cur.children.push(next);
      }
      cur = next;
    }
    cur.pages.push({ id: p.id, title: p.title, path: p.path });
  }

  // Match familiar vault explorers: folders first, then pages, alphabetically.
  function sortNode(node: TreeNode) {
    node.children.sort((a, b) => a.name.localeCompare(b.name));
    node.pages.sort((a, b) => a.title.localeCompare(b.title));
    for (const c of node.children) sortNode(c);
  }
  sortNode(root);
  return root;
}

export function FileTree({ onNewPage }: { onNewPage: (folder?: string) => void }) {
  const pages = usePages();
  const activeTag = useTagFilter((state) => state.activeTag);
  const taggedPageIds = useLiveQuery(async () => activeTag
    ? new Set((await db.tags.where('tag').equals(activeTag).toArray()).map((tag) => tag.pageId))
    : null, [activeTag]);
  const visiblePages = useMemo(
    () => activeTag ? (pages ?? []).filter((page) => taggedPageIds?.has(page.id)) : (pages ?? []),
    [activeTag, pages, taggedPageIds],
  );
  const tree = useMemo(
    () => buildTree(visiblePages.map((p) => ({ id: p.id, title: p.title, path: p.path }))),
    [visiblePages],
  );
  const folders = useMemo(() => collectFolders(tree), [tree]);
  const [notice, setNotice] = useState<{ kind: 'success' | 'error'; message: string } | null>(null);
  const bookmarkedPageIds = useBookmarkedPageIds();
  const bookmarkedPages = useMemo(() => new Set(bookmarkedPageIds), [bookmarkedPageIds]);

  async function handleMove(pageId: string, folder: string) {
    try {
      const moved = await movePage(pageId, folder);
      setNotice({ kind: 'success', message: `Moved “${moved.title}” to ${folder || 'the vault root'}.` });
    } catch (cause) {
      setNotice({ kind: 'error', message: cause instanceof Error ? cause.message : String(cause) });
      throw cause;
    }
  }

  return (
    <div className="flex flex-col gap-1 px-2">
      <div
        className="flex items-center justify-between rounded-md px-2 py-1"
        onDragOver={(event) => { if (hasDraggedPage(event)) event.preventDefault(); }}
        onDrop={(event) => { event.preventDefault(); const pageId = draggedPageId(event); if (pageId) void handleMove(pageId, '').catch(() => undefined); }}
        title="Drop a note here to move it to the vault root"
      >
        <span className="truncate text-meta font-medium text-muted-foreground">{activeTag ? `Pages · #${activeTag}` : 'Pages'}</span>
        <button
          type="button"
          onClick={() => onNewPage()}
          className="rounded-sm p-1 text-muted-foreground hover:bg-surface-3 hover:text-foreground"
          aria-label="New page"
        >
          <Plus size={14} />
        </button>
      </div>
      <Node node={tree} depth={0} folders={folders} bookmarks={bookmarkedPages} onNewPage={onNewPage} onMovePage={handleMove} />
      {notice && <div className={`mx-2 flex items-start gap-2 rounded-md px-2 py-1.5 text-micro leading-relaxed ${notice.kind === 'error' ? 'bg-destructive/10 text-destructive' : 'bg-primary/10 text-muted-foreground-strong'}`} role="status"><span className="min-w-0 flex-1">{notice.message}</span><button type="button" onClick={() => setNotice(null)} aria-label="Dismiss message" className="shrink-0 rounded-sm opacity-70 hover:opacity-100"><X size={11} /></button></div>}
      {activeTag && visiblePages.length === 0 && <p className="px-2 py-3 text-micro leading-relaxed text-muted-foreground">No notes currently use #{activeTag}.</p>}
    </div>
  );
}

function Node({
  node,
  depth,
  folders,
  bookmarks,
  onNewPage,
  onMovePage,
}: {
  node: TreeNode;
  depth: number;
  folders: string[];
  bookmarks: Set<string>;
  onNewPage: (folder?: string) => void;
  onMovePage: (pageId: string, folder: string) => Promise<void>;
}) {
  return (
    <div>
      {node.children.map((child) => (
        <FolderNode
          key={child.path}
          node={child}
          depth={depth}
          folders={folders}
          bookmarks={bookmarks}
          onNewPage={(folder) => onNewPage(folder)}
          onMovePage={onMovePage}
        />
      ))}
      {node.pages.map((page) => (
        <PageNode key={page.id} page={page} depth={depth} folders={folders} bookmarked={bookmarks.has(page.id)} onMovePage={onMovePage} />
      ))}
    </div>
  );
}

function FolderNode({
  node,
  depth,
  folders,
  bookmarks,
  onNewPage,
  onMovePage,
}: {
  node: TreeNode;
  depth: number;
  folders: string[];
  bookmarks: Set<string>;
  onNewPage: (folder?: string) => void;
  onMovePage: (pageId: string, folder: string) => Promise<void>;
}) {
  const [open, setOpen] = useState(depth < 1);
  const [dragOver, setDragOver] = useState(false);
  return (
    <div>
      <div
        className={cn('group flex items-center rounded-sm hover:bg-surface-2', dragOver && 'bg-primary/15 ring-1 ring-inset ring-primary/40')}
        style={{ paddingLeft: 8 + depth * 12 }}
        onDragOver={(event) => { if (!hasDraggedPage(event)) return; event.preventDefault(); event.stopPropagation(); setDragOver(true); }}
        onDragLeave={() => setDragOver(false)}
        onDrop={(event) => { event.preventDefault(); event.stopPropagation(); setDragOver(false); const pageId = draggedPageId(event); if (pageId) void onMovePage(pageId, node.path).catch(() => undefined); }}
      >
        <button
          type="button"
          onClick={() => setOpen((value) => !value)}
          className="flex min-w-0 flex-1 items-center gap-1 py-0.5 text-left text-meta"
          aria-expanded={open}
        >
          {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          {open ? <FolderOpen size={13} /> : <FolderClosed size={13} />}
          <span className="truncate text-muted-foreground-strong">{node.name}</span>
        </button>
        <button type="button" onClick={() => onNewPage(node.path)} className="mr-1 rounded-sm p-0.5 text-muted-foreground opacity-0 hover:bg-surface-3 hover:text-foreground focus:opacity-100 group-hover:opacity-100" aria-label={`New page in ${node.name}`}>
          <Plus size={11} />
        </button>
      </div>
      {open && (
        <Node node={node} depth={depth + 1} folders={folders} bookmarks={bookmarks} onNewPage={onNewPage} onMovePage={onMovePage} />
      )}
    </div>
  );
}

function PageNode({
  page,
  depth,
  folders,
  bookmarked,
  onMovePage,
}: {
  page: { id: string; title: string; path: string };
  depth: number;
  folders: string[];
  bookmarked: boolean;
  onMovePage: (pageId: string, folder: string) => Promise<void>;
}) {
  const activeTab = useNavStore((s) => s.activeTab);
  const primaryTab = useNavStore((s) => s.primaryTab);
  const openPage = useNavStore((s) => s.openPage);
  const openInSplit = useNavStore((s) => s.openInSplit);
  const pushRecent = useNavStore((s) => s.pushRecent);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(page.title);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [moveOpen, setMoveOpen] = useState(false);
  const [destination, setDestination] = useState(page.path.includes('/') ? page.path.split('/').slice(0, -1).join('/') : '');
  const [error, setError] = useState<string | null>(null);
  const filesystemVault = Boolean(getActiveVaultKind());

  async function handleRename() {
    const trimmed = draft.trim();
    if (!trimmed || trimmed === page.title) {
      setEditing(false);
      setDraft(page.title);
      return;
    }
    try {
      await renamePage(page.id, trimmed);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setDraft(page.title);
    }
    setEditing(false);
  }

  async function handleDelete() {
    await deletePage(page.id);
    useNavStore.getState().closeTab(page.id);
    setDeleteOpen(false);
  }

  async function handleMove() {
    try {
      await onMovePage(page.id, destination);
      setMoveOpen(false);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleDuplicate() {
    try {
      const copy = await duplicatePage(page.id);
      const nav = useNavStore.getState();
      nav.openPage(copy.id);
      nav.pushRecent(copy.id);
      if (window.matchMedia('(max-width: 767px)').matches) useUIStore.getState().setSidebarOpen(false);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  const isActive = activeTab === page.id;

  if (editing) {
    return (
      <div style={{ paddingLeft: 8 + depth * 12 + 14 }} className="py-0.5">
        <input
          autoFocus
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={handleRename}
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleRename();
            if (e.key === 'Escape') {
              setEditing(false);
              setDraft(page.title);
            }
          }}
          className="w-full rounded-sm border border-primary bg-surface-1 px-1 text-meta text-foreground focus:outline-none"
        />
      </div>
    );
  }

  return (
    <>
      <ContextMenu.Root>
        <ContextMenu.Trigger asChild>
          <button
            type="button"
            draggable
            onDragStart={(event) => { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData('application/x-ley-page-id', page.id); }}
            onClick={() => {
              openPage(page.id);
              pushRecent(page.id);
              if (window.matchMedia('(max-width: 767px)').matches) useUIStore.getState().setSidebarOpen(false);
            }}
            onDoubleClick={() => setEditing(true)}
            className={cn(
              'group flex w-full items-center gap-1 rounded-sm py-0.5 pr-2 text-left text-meta',
              isActive
                ? 'bg-surface-3 text-foreground'
                : 'text-muted-foreground-strong hover:bg-surface-2 hover:text-foreground',
            )}
            style={{ paddingLeft: 8 + depth * 12 + 14 }}
          >
            <FileText size={13} className="shrink-0" />
            <span className="truncate">{page.title}</span>
          </button>
        </ContextMenu.Trigger>
        <ContextMenu.Portal>
          <ContextMenu.Content className="z-[80] min-w-44 rounded-lg border border-border bg-surface-1 p-1 shadow-menu">
            {page.id !== primaryTab && <ContextMenu.Item onSelect={() => { openInSplit(page.id); if (window.matchMedia('(max-width: 767px)').matches) useUIStore.getState().setSidebarOpen(false); }} className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-meta text-foreground outline-none data-[highlighted]:bg-surface-3">
              <Columns2 size={13} /> Open in split
            </ContextMenu.Item>}
            {page.id !== primaryTab && <ContextMenu.Separator className="my-1 h-px bg-border" />}
            <ContextMenu.Item onSelect={() => setEditing(true)} className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-meta text-foreground outline-none data-[highlighted]:bg-surface-3">
              <Pencil size={13} /> Rename
            </ContextMenu.Item>
            <ContextMenu.Item onSelect={() => { setDestination(page.path.includes('/') ? page.path.split('/').slice(0, -1).join('/') : ''); setMoveOpen(true); }} className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-meta text-foreground outline-none data-[highlighted]:bg-surface-3">
              <FolderInput size={13} /> Move to…
            </ContextMenu.Item>
            <ContextMenu.Item onSelect={() => void handleDuplicate()} className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-meta text-foreground outline-none data-[highlighted]:bg-surface-3">
              <Copy size={13} /> Duplicate
            </ContextMenu.Item>
            <ContextMenu.Item onSelect={() => void navigator.clipboard.writeText(`[[${page.title}]]`).catch((cause) => setError(cause instanceof Error ? cause.message : String(cause)))} className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-meta text-foreground outline-none data-[highlighted]:bg-surface-3">
              <Link2 size={13} /> Copy wiki link
            </ContextMenu.Item>
            <ContextMenu.Item onSelect={() => void togglePageBookmark(page.id)} className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-meta text-foreground outline-none data-[highlighted]:bg-surface-3">
              <Bookmark size={13} className={bookmarked ? 'fill-current text-secondary' : undefined} /> {bookmarked ? 'Remove bookmark' : 'Bookmark note'}
            </ContextMenu.Item>
            <ContextMenu.Separator className="my-1 h-px bg-border" />
            <ContextMenu.Item onSelect={() => setDeleteOpen(true)} className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-meta text-destructive outline-none data-[highlighted]:bg-destructive/10">
              <Trash2 size={13} /> Move to trash
            </ContextMenu.Item>
          </ContextMenu.Content>
        </ContextMenu.Portal>
      </ContextMenu.Root>
      {error && !moveOpen && !deleteOpen && <div className="mx-2 my-1 flex items-start gap-2 rounded-md bg-destructive/10 px-2 py-1.5 text-micro text-destructive" role="alert"><span className="min-w-0 flex-1">{error}</span><button type="button" onClick={() => setError(null)} aria-label="Dismiss error"><X size={11} /></button></div>}

      <Dialog.Root open={moveOpen} onOpenChange={(open) => { setMoveOpen(open); if (!open) setError(null); }}>
        <Dialog.Portal>
          <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-[90]" />
          <Dialog.Content className="app-modal-surface fixed left-1/2 top-1/2 z-[91] w-[420px] max-w-[calc(100vw-24px)] -translate-x-1/2 -translate-y-1/2 rounded-md border p-5 outline-none">
            <div className="flex items-start justify-between gap-4">
              <div>
                <Dialog.Title className="text-body font-semibold text-foreground">Move “{page.title}”</Dialog.Title>
                <Dialog.Description className="mt-1 text-meta leading-relaxed text-muted-foreground">Choose an existing folder or type a new nested path. Leave it empty for the vault root.</Dialog.Description>
              </div>
              <Dialog.Close aria-label="Close move dialog" className="rounded-md p-1 text-muted-foreground hover:bg-surface-3"><X size={14} /></Dialog.Close>
            </div>
            <label className="mt-4 block text-meta font-medium text-muted-foreground-strong" htmlFor={`move-${page.id}`}>Destination folder</label>
            <input id={`move-${page.id}`} autoFocus value={destination} list={`folders-${page.id}`} onChange={(event) => setDestination(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') void handleMove(); }} placeholder="Vault root" className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 font-mono text-meta text-foreground outline-none focus:border-primary" />
            <datalist id={`folders-${page.id}`}>{folders.map((folder) => <option key={folder} value={folder} />)}</datalist>
            {error && <p className="mt-3 text-meta text-destructive" role="alert">{error}</p>}
            <div className="mt-5 flex justify-end gap-2">
              <Dialog.Close className="rounded-md border border-border px-3 py-1.5 text-meta text-foreground hover:bg-surface-2">Cancel</Dialog.Close>
              <button type="button" onClick={() => void handleMove()} className="rounded-md bg-primary px-3 py-1.5 text-meta font-medium text-primary-foreground hover:opacity-90">Move note</button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>

      <Dialog.Root open={deleteOpen} onOpenChange={setDeleteOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-[90]" />
          <Dialog.Content className="app-modal-surface fixed left-1/2 top-1/2 z-[91] w-[420px] max-w-[calc(100vw-24px)] -translate-x-1/2 -translate-y-1/2 rounded-md border p-5 outline-none">
            <div className="flex items-start justify-between gap-4">
              <div>
                <Dialog.Title className="text-body font-semibold text-foreground">Move note to trash?</Dialog.Title>
                <Dialog.Description className="mt-1 text-meta leading-relaxed text-muted-foreground">
                  {filesystemVault ? <>“{page.title}” will be moved to the vault’s <span className="font-mono">.trash</span> folder.</> : <>“{page.title}” will move to the recycle bin in Settings, where it can be restored.</>}
                </Dialog.Description>
              </div>
              <Dialog.Close aria-label="Close delete dialog" className="rounded-md p-1 text-muted-foreground hover:bg-surface-3"><X size={14} /></Dialog.Close>
            </div>
            {error && <p className="mt-3 text-meta text-destructive">{error}</p>}
            <div className="mt-5 flex justify-end gap-2">
              <Dialog.Close className="rounded-md border border-border px-3 py-1.5 text-meta text-foreground hover:bg-surface-2">Cancel</Dialog.Close>
              <button type="button" onClick={() => void handleDelete().catch((cause) => setError(cause instanceof Error ? cause.message : String(cause)))} className="rounded-md bg-destructive px-3 py-1.5 text-meta font-medium text-white hover:opacity-90">Move to trash</button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </>
  );
}

function collectFolders(root: TreeNode): string[] {
  const folders: string[] = [];
  const visit = (node: TreeNode) => {
    for (const child of node.children) {
      folders.push(child.path);
      visit(child);
    }
  };
  visit(root);
  return folders;
}

function hasDraggedPage(event: DragEvent): boolean {
  return event.dataTransfer.types.includes('application/x-ley-page-id');
}

function draggedPageId(event: DragEvent): string {
  return event.dataTransfer.getData('application/x-ley-page-id');
}
