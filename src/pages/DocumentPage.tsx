import { useCallback, useEffect, useState, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useNode } from '@/hooks';
import { Button, ConfirmDialog, toast } from '@/components/ui';
import { Tooltip, TooltipTrigger, TooltipContent } from '@/components/ui';
import {
  ArrowLeft,
  MoreHorizontal,
  History,
  Trash2,
  Archive,
  Link as LinkIcon,
  Copy,
} from 'lucide-react';
import { BlockEditor } from '@/components/editor';
import { BacklinkPanel } from '@/components/editor/BacklinkPanel';
import { extractText } from '@/lib/editor';
import { useWorkspaceStore } from '@/store';
import { db } from '@/lib/db';
import type { JSONContent } from '@tiptap/react';
import { formatRelative } from '@/lib/utils';
import { cn } from '@/lib/utils';

type DialogKind = 'archive' | 'delete' | null;

export function DocumentPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { node, updateNode } = useNode(id || null);
  const { addToRecentNodes, setLastOpenedNode } = useWorkspaceStore();
  const [menuOpen, setMenuOpen] = useState(false);
  const [dialog, setDialog] = useState<DialogKind>(null);
  const titleRef = useRef<HTMLDivElement>(null);

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

  const handleTitleChange = () => {
    if (!id || !titleRef.current) return;
    const newTitle = titleRef.current.textContent || '';
    if (newTitle !== node?.title) {
      updateNode({ title: newTitle });
    }
  };

  const handleArchive = async () => {
    if (!id) return;
    await db.nodes.update(id, { isArchived: 1, updatedAt: Date.now() });
    toast('Page archived', { kind: 'success' });
    navigate('/');
  };

  const handleDelete = async () => {
    if (!id) return;
    await db.nodes.delete(id);
    toast('Page deleted', { kind: 'success' });
    navigate('/');
  };

  const handleCopyLink = async () => {
    if (!id) return;
    await navigator.clipboard.writeText(`${window.location.origin}/page/${id}`);
    toast('Link copied', { kind: 'success' });
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
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              onClick={() => navigate(-1)}
              aria-label="Back"
              className="flex h-6 w-6 items-center justify-center rounded text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground"
            >
              <ArrowLeft className="h-3.5 w-3.5" />
            </button>
          </TooltipTrigger>
          <TooltipContent>Back</TooltipContent>
        </Tooltip>

        <div className="flex items-center gap-0.5">
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => navigate(`/page/${id}/revisions`)}
                className="flex h-7 items-center gap-1.5 rounded px-2 text-[12px] text-muted-foreground/80 transition-colors hover:bg-accent hover:text-foreground"
              >
                <History className="h-3 w-3" />
                History
              </button>
            </TooltipTrigger>
            <TooltipContent>Revision history</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={handleCopyLink}
                aria-label="Copy link"
                className="flex h-7 w-7 items-center justify-center rounded text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground"
              >
                <LinkIcon className="h-3.5 w-3.5" />
              </button>
            </TooltipTrigger>
            <TooltipContent>Copy link</TooltipContent>
          </Tooltip>

          <div className="relative">
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  onClick={() => setMenuOpen((v) => !v)}
                  aria-label="More"
                  className="flex h-7 w-7 items-center justify-center rounded text-muted-foreground/70 transition-colors hover:bg-accent hover:text-foreground"
                >
                  <MoreHorizontal className="h-3.5 w-3.5" />
                </button>
              </TooltipTrigger>
              <TooltipContent>More actions</TooltipContent>
            </Tooltip>
            {menuOpen && (
              <>
                <div
                  className="fixed inset-0 z-10"
                  onClick={() => setMenuOpen(false)}
                />
                <div className="absolute right-0 top-8 z-20 w-48 overflow-hidden rounded-md border border-border/80 bg-popover p-1 shadow-menu animate-slide-down">
                  <MenuItem
                    icon={<Copy className="h-3.5 w-3.5" />}
                    label="Copy link"
                    onClick={() => {
                      handleCopyLink();
                      setMenuOpen(false);
                    }}
                  />
                  <div className="my-1 h-px bg-border/60" />
                  <MenuItem
                    icon={<Archive className="h-3.5 w-3.5" />}
                    label="Archive"
                    onClick={() => {
                      setMenuOpen(false);
                      setDialog('archive');
                    }}
                  />
                  <MenuItem
                    icon={<Trash2 className="h-3.5 w-3.5" />}
                    label="Delete page"
                    destructive
                    onClick={() => {
                      setMenuOpen(false);
                      setDialog('delete');
                    }}
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
            ref={titleRef}
            contentEditable
            suppressContentEditableWarning
            onBlur={handleTitleChange}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                (e.currentTarget as HTMLElement).blur();
              }
              if (e.key === 'Escape') {
                e.preventDefault();
                if (titleRef.current && node) {
                  titleRef.current.textContent = node.title;
                }
                (e.currentTarget as HTMLElement).blur();
              }
            }}
            className={cn(
              'min-h-[40px] whitespace-pre-wrap break-words rounded-sm text-[28px] font-semibold tracking-[-0.015em]',
              'text-foreground/95 outline-none transition-colors',
              'hover:bg-accent/30 focus:bg-accent/40',
              'empty:before:content-["Untitled"] empty:before:text-muted-foreground/40'
            )}
            data-placeholder="Untitled"
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

      <ConfirmDialog
        open={dialog === 'archive'}
        onOpenChange={(o) => setDialog(o ? 'archive' : null)}
        title="Archive this page?"
        description="It will be hidden from the workspace but not deleted. You can find archived pages later."
        confirmLabel="Archive"
        onConfirm={handleArchive}
      />
      <ConfirmDialog
        open={dialog === 'delete'}
        onOpenChange={(o) => setDialog(o ? 'delete' : null)}
        title="Delete this page?"
        description="This will permanently remove the page and all its links from your workspace. This cannot be undone."
        confirmLabel="Delete"
        destructive
        onConfirm={handleDelete}
      />
    </div>
  );
}

function MenuItem({
  icon,
  label,
  onClick,
  destructive,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  destructive?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-[13px] transition-colors',
        destructive
          ? 'text-destructive hover:bg-destructive/10'
          : 'text-foreground/85 hover:bg-accent'
      )}
    >
      <span
        className={cn(
          'flex h-3.5 w-3.5 items-center justify-center',
          destructive ? 'text-destructive' : 'text-muted-foreground/80'
        )}
      >
        {icon}
      </span>
      <span>{label}</span>
    </button>
  );
}
