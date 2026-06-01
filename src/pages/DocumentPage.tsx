import { useCallback, useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useNode } from '@/hooks';
import { Button } from '@/components/ui';
import {
  ArrowLeft,
  MoreHorizontal,
  History,
  Trash2,
  Check,
} from 'lucide-react';
import { BlockEditor } from '@/components/editor';
import { BacklinkPanel } from '@/components/editor/BacklinkPanel';
import { extractText } from '@/lib/editor';
import { useWorkspaceStore } from '@/store';
import { db } from '@/lib/db';
import type { JSONContent } from '@tiptap/react';
import { formatRelative } from '@/lib/utils';
import { cn } from '@/lib/utils';

export function DocumentPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { node, updateNode } = useNode(id || null);
  const { addToRecentNodes, setLastOpenedNode } = useWorkspaceStore();
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    if (id) {
      setLastOpenedNode(id);
      addToRecentNodes(id);
    }
  }, [id, setLastOpenedNode, addToRecentNodes]);

  const handleContentUpdate = useCallback(
    async (content: JSONContent) => {
      if (!id) return;
      const plainText = extractText(content);
      await updateNode({ content, plainText });
    },
    [id, updateNode]
  );

  const handleTitleChange = (e: React.FocusEvent<HTMLDivElement>) => {
    const newTitle = e.currentTarget.textContent || '';
    if (id && newTitle !== node?.title) {
      updateNode({ title: newTitle });
    }
  };

  const handleArchive = async () => {
    if (!id) return;
    setMenuOpen(false);
    if (!window.confirm('Archive this page?')) return;
    await db.nodes.update(id, { isArchived: 1, updatedAt: Date.now() });
    navigate('/');
  };

  const handleDelete = async () => {
    if (!id) return;
    setMenuOpen(false);
    if (!window.confirm('Delete this page permanently?')) return;
    await db.nodes.delete(id);
    navigate('/');
  };

  if (!node) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="space-y-3 text-center">
          <h2 className="text-[20px] font-semibold tracking-tight">Page not found</h2>
          <p className="text-[13px] text-muted-foreground/80">
            This page does not exist or has been deleted.
          </p>
          <Button variant="outline" size="sm" onClick={() => navigate('/')}>
            <ArrowLeft className="mr-1.5 h-3.5 w-3.5" />
            Back home
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="mx-auto flex w-full max-w-3xl items-center justify-between px-8 pt-3">
        <button
          onClick={() => navigate(-1)}
          aria-label="Back"
          className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
        </button>

        <div className="flex items-center gap-1">
          <button
            onClick={() => navigate(`/page/${id}/revisions`)}
            className="flex h-6 items-center gap-1.5 rounded px-2 text-[12px] text-muted-foreground/80 transition-colors hover:bg-accent hover:text-foreground"
          >
            <History className="h-3 w-3" />
            History
          </button>
          <div className="relative">
            <button
              onClick={() => setMenuOpen((v) => !v)}
              aria-label="More"
              className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground"
            >
              <MoreHorizontal className="h-3.5 w-3.5" />
            </button>
            {menuOpen && (
              <>
                <div
                  className="fixed inset-0 z-10"
                  onClick={() => setMenuOpen(false)}
                />
                <div className="absolute right-0 top-7 z-20 w-44 overflow-hidden rounded-md border border-border/80 bg-popover p-1 shadow-menu">
                  <MenuItem
                    icon={<Trash2 className="h-3.5 w-3.5" />}
                    label="Delete page"
                    onClick={handleDelete}
                  />
                  <MenuItem
                    icon={<Check className="h-3.5 w-3.5" />}
                    label="Archive"
                    onClick={handleArchive}
                  />
                </div>
              </>
            )}
          </div>
        </div>
      </div>

      <main className="flex-1 overflow-auto">
        <div className="mx-auto max-w-3xl px-8 pb-24 pt-6">
          <div
            contentEditable
            suppressContentEditableWarning
            onBlur={handleTitleChange}
            className={cn(
              'min-h-[40px] whitespace-pre-wrap break-words text-[28px] font-semibold tracking-[-0.015em]',
              'text-foreground/95 outline-none',
              !node.title && "before:content-['Untitled'] before:text-muted-foreground/40"
            )}
          >
            {node.title}
          </div>

          <div className="mt-1.5 flex items-center gap-2 text-[12px] text-muted-foreground/70">
            <span className="capitalize">{node.type}</span>
            <span className="text-muted-foreground/30">/</span>
            <span>Updated {formatRelative(node.updatedAt)}</span>
            {node.tags.length > 0 && (
              <>
                <span className="text-muted-foreground/30">/</span>
                <span className="truncate">{node.tags.join(', ')}</span>
              </>
            )}
          </div>

          <div className="mt-8">
            <BlockEditor
              content={node.content}
              onUpdate={handleContentUpdate}
              placeholder="Type '/' for commands, or '[[' to link another page"
            />
          </div>

          <div className="mt-16 border-t border-border/40 pt-8">
            <BacklinkPanel nodeId={node.id} />
          </div>
        </div>
      </main>
    </div>
  );
}

function MenuItem({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-[13px] text-foreground/85 transition-colors hover:bg-accent"
    >
      <span className="flex h-3.5 w-3.5 items-center justify-center text-muted-foreground/80">
        {icon}
      </span>
      <span>{label}</span>
    </button>
  );
}
