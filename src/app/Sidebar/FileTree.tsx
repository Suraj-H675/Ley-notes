/**
 * FileTree — collapsible tree of pages, grouped by folder (the part of `path`
 * before the final segment). Right-click or the + button creates a new page;
 * rename/delete via the context menu.
 */

import { useMemo, useState } from 'react';
import { ChevronRight, ChevronDown, FileText, FolderClosed, FolderOpen, Plus } from 'lucide-react';
import { usePages } from '@/hooks/usePages';
import { useNavStore } from '@/store/nav';
import { createPage, deletePage, renamePage } from '@/core/vault/pages';
import { db } from '@/data/db';
import { useLiveQuery } from 'dexie-react-hooks';
import { cn } from '@/lib/classnames';

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

export function FileTree() {
  const pages = usePages();
  const tree = useMemo(
    () => buildTree((pages ?? []).map((p) => ({ id: p.id, title: p.title, path: p.path }))),
    [pages],
  );

  async function handleNewPage(folder?: string) {
    const title = prompt('Page title:');
    if (!title) return;
    const page = await createPage({ title, folder });
    useNavStore.getState().openPage(page.id);
    useNavStore.getState().pushRecent(page.id);
  }

  return (
    <div className="flex flex-col gap-1 px-2">
      <div className="flex items-center justify-between px-2 py-1">
        <span className="text-meta font-medium text-muted-foreground">Pages</span>
        <button
          type="button"
          onClick={() => handleNewPage()}
          className="rounded-sm p-1 text-muted-foreground hover:bg-surface-3 hover:text-foreground"
          aria-label="New page"
        >
          <Plus size={14} />
        </button>
      </div>
      <Node node={tree} depth={0} onNewPage={handleNewPage} />
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
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className={cn(
          'group flex w-full items-center gap-1 rounded-sm py-0.5 pr-2 text-left text-meta hover:bg-surface-2',
        )}
        style={{ paddingLeft: 8 + depth * 12 }}
      >
        {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        {open ? <FolderOpen size={13} /> : <FolderClosed size={13} />}
        <span className="truncate text-muted-foreground-strong">{node.name}</span>
        <span
          role="button"
          tabIndex={0}
          onClick={(e) => {
            e.stopPropagation();
            onNewPage(node.path);
          }}
          className="ml-auto rounded-sm p-0.5 text-muted-foreground opacity-0 hover:bg-surface-3 hover:text-foreground group-hover:opacity-100"
          aria-label="New page in folder"
        >
          <Plus size={11} />
        </span>
      </button>
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

  async function handleRename() {
    const trimmed = draft.trim();
    if (!trimmed || trimmed === page.title) {
      setEditing(false);
      setDraft(page.title);
      return;
    }
    try {
      await renamePage(page.id, trimmed);
    } catch (e) {
      alert(e instanceof Error ? e.message : String(e));
      setDraft(page.title);
    }
    setEditing(false);
  }

  async function handleDelete() {
    if (!confirm(`Delete "${page.title}"?`)) return;
    await deletePage(page.id);
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
    <button
      type="button"
      onClick={() => {
        openPage(page.id);
        pushRecent(page.id);
      }}
      onDoubleClick={() => setEditing(true)}
      onContextMenu={(e) => {
        e.preventDefault();
        const action = prompt(`Action for "${page.title}":\n1. Rename\n2. Delete`);
        if (action === '1') setEditing(true);
        if (action === '2') handleDelete();
      }}
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
  );
}

// Suppress unused warning for liveQuery import — used implicitly via usePages.
void db;
void useLiveQuery;