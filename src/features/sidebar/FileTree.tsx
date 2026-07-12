/**
 * FileTree — collapsible tree of pages, grouped by folder (the part of `path`
 * before the final segment). Right-click or the + button creates a new page;
 * rename/delete via the context menu.
 */

import { useMemo, useState } from 'react';
import * as ContextMenu from '@radix-ui/react-context-menu';
import * as Dialog from '@radix-ui/react-dialog';
import { ChevronRight, ChevronDown, FileText, FolderClosed, FolderOpen, Pencil, Plus, Trash2, X } from 'lucide-react';
import { usePages } from '@/features/notes/usePages';
import { useNavStore } from '@/shared/state/nav';
import { deletePage, renamePage } from '@/core/vault/pages';
import { cn } from '@/shared/lib/classnames';
import { useUIStore } from '@/shared/state/ui';

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

  // Sort: pages first, then folders alphabetically within each level.
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
  const tree = useMemo(
    () => buildTree((pages ?? []).map((p) => ({ id: p.id, title: p.title, path: p.path }))),
    [pages],
  );

  return (
    <div className="flex flex-col gap-1 px-2">
      <div className="flex items-center justify-between px-2 py-1">
        <span className="text-meta font-medium text-muted-foreground">Pages</span>
        <button
          type="button"
          onClick={() => onNewPage()}
          className="rounded-sm p-1 text-muted-foreground hover:bg-surface-3 hover:text-foreground"
          aria-label="New page"
        >
          <Plus size={14} />
        </button>
      </div>
      <Node node={tree} depth={0} onNewPage={onNewPage} />
    </div>
  );
}

function Node({
  node,
  depth,
  onNewPage,
}: {
  node: TreeNode;
  depth: number;
  onNewPage: (folder?: string) => void;
}) {
  return (
    <div>
      {node.children.map((child) => (
        <FolderNode
          key={child.path}
          node={child}
          depth={depth}
          onNewPage={(folder) => onNewPage(folder)}
        />
      ))}
      {node.pages.map((page) => (
        <PageNode key={page.id} page={page} depth={depth} />
      ))}
    </div>
  );
}

function FolderNode({
  node,
  depth,
  onNewPage,
}: {
  node: TreeNode;
  depth: number;
  onNewPage: (folder?: string) => void;
}) {
  const [open, setOpen] = useState(depth < 1);
  return (
    <div>
      <div className="group flex items-center rounded-sm hover:bg-surface-2" style={{ paddingLeft: 8 + depth * 12 }}>
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
        <Node node={node} depth={depth + 1} onNewPage={onNewPage} />
      )}
    </div>
  );
}

function PageNode({
  page,
  depth,
}: {
  page: { id: string; title: string; path: string };
  depth: number;
}) {
  const activeTab = useNavStore((s) => s.activeTab);
  const openPage = useNavStore((s) => s.openPage);
  const pushRecent = useNavStore((s) => s.pushRecent);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(page.title);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
            <ContextMenu.Item onSelect={() => setEditing(true)} className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-meta text-foreground outline-none data-[highlighted]:bg-surface-3">
              <Pencil size={13} /> Rename
            </ContextMenu.Item>
            <ContextMenu.Separator className="my-1 h-px bg-border" />
            <ContextMenu.Item onSelect={() => setDeleteOpen(true)} className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-meta text-destructive outline-none data-[highlighted]:bg-destructive/10">
              <Trash2 size={13} /> Move to trash
            </ContextMenu.Item>
          </ContextMenu.Content>
        </ContextMenu.Portal>
      </ContextMenu.Root>

      <Dialog.Root open={deleteOpen} onOpenChange={setDeleteOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 z-[90] bg-background/70 backdrop-blur-sm" />
          <Dialog.Content className="fixed left-1/2 top-1/2 z-[91] w-[420px] max-w-[calc(100vw-24px)] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-border bg-surface-1 p-5 shadow-menu outline-none">
            <div className="flex items-start justify-between gap-4">
              <div>
                <Dialog.Title className="text-body font-semibold text-foreground">Move note to trash?</Dialog.Title>
                <Dialog.Description className="mt-1 text-meta leading-relaxed text-muted-foreground">
                  “{page.title}” will be moved to the vault’s <span className="font-mono">.trash</span> folder when a filesystem vault is active.
                </Dialog.Description>
              </div>
              <Dialog.Close className="rounded-md p-1 text-muted-foreground hover:bg-surface-3"><X size={14} /></Dialog.Close>
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
