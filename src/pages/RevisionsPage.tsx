import { useState, useEffect } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { PageHeader, PageContainer } from '@/components/layout';
import { RotateCcw } from 'lucide-react';
import { formatRelative, formatDate } from '@/lib/utils';
import { cn } from '@/lib/utils';

export function RevisionsPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [selectedRevisionId, setSelectedRevisionId] = useState<string | null>(null);

  const node = useLiveQuery(() => (id ? db.nodes.get(id) : undefined), [id]);

  const revisions = useLiveQuery(
    async () => {
      if (!id) return [];
      return db.revisions.where('nodeId').equals(id).reverse().sortBy('createdAt');
    },
    [id]
  );

  const selectedRevision = useLiveQuery(
    async () => (selectedRevisionId ? db.revisions.get(selectedRevisionId) : null),
    [selectedRevisionId]
  );

  // Auto-select the most recent revision on first load
  useEffect(() => {
    if (!selectedRevisionId && revisions && revisions.length > 0) {
      setSelectedRevisionId(revisions[0].id);
    }
  }, [revisions, selectedRevisionId]);

  const handleRestore = async () => {
    if (!selectedRevision || !id) return;
    const { updateNode } = await import('@/lib/db');
    await updateNode(id, {
      content: selectedRevision.content,
      plainText: selectedRevision.plainText,
    });
    navigate(`/page/${id}`);
  };

  if (!node) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="space-y-3 text-center">
          <h2 className="text-[20px] font-semibold tracking-tight">Page not found</h2>
          <p className="text-[13px] text-muted-foreground/80">
            This page does not exist or has been deleted.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Revision history"
        subtitle={node.title || 'Untitled'}
        actions={
          selectedRevision && (
            <button
              onClick={handleRestore}
              className="inline-flex h-7 items-center gap-1.5 rounded-md bg-foreground px-2.5 text-[12.5px] font-medium text-background transition-opacity hover:opacity-90"
            >
              <RotateCcw className="h-3 w-3" />
              Restore this version
            </button>
          )
        }
      />

      <div className="flex min-h-0 flex-1">
        {/* Revision list */}
        <div className="w-72 flex-shrink-0 overflow-y-auto border-r border-border/40">
          {revisions && revisions.length > 0 ? (
            <ul className="px-3 py-2">
              {revisions.map((revision, idx) => (
                <li key={revision.id}>
                  <button
                    onClick={() => setSelectedRevisionId(revision.id)}
                    className={cn(
                      'group flex w-full flex-col gap-0.5 rounded px-2 py-2 text-left transition-colors',
                      selectedRevisionId === revision.id
                        ? 'bg-accent/60'
                        : 'hover:bg-accent/30'
                    )}
                  >
                    <div className="flex items-baseline justify-between gap-2">
                      <span className="text-[12.5px] text-foreground/90">
                        {idx === 0 ? 'Current' : `Version ${revisions.length - idx}`}
                      </span>
                      <span className="text-[11px] tabular-nums text-muted-foreground/55">
                        {formatRelative(revision.createdAt)}
                      </span>
                    </div>
                    <span className="truncate text-[11.5px] text-muted-foreground/65">
                      {revision.plainText.slice(0, 60) || 'Empty'}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <div className="px-6 py-12 text-center">
              <p className="text-[12.5px] text-muted-foreground/70">
                No revisions saved yet.
              </p>
            </div>
          )}
        </div>

        {/* Preview */}
        <PageContainer className="flex-1 overflow-y-auto">
          {selectedRevision ? (
            <div>
              <div className="mb-6 flex items-center gap-2 border-b border-border/40 pb-3 text-[12px] text-muted-foreground/70">
                <span className="rounded bg-accent/60 px-1.5 py-0.5 text-foreground/80">
                  {formatDate(selectedRevision.createdAt)}
                </span>
                <span>Read-only preview</span>
              </div>
              <div className="prose prose-invert prose-sm max-w-none text-foreground/90">
                {selectedRevision.plainText || 'No content'}
              </div>
            </div>
          ) : (
            <div className="flex h-full items-center justify-center">
              <p className="text-[13px] text-muted-foreground/60">
                Select a revision from the list to preview it.
              </p>
            </div>
          )}
        </PageContainer>
      </div>
    </div>
  );
}
