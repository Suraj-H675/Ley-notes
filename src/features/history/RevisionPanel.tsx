import { useState } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { Clock3, History, RotateCcw } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { db } from '@/infrastructure/database/db';
import { updatePageContent } from '@/core/vault/pages';

export function RevisionPanel({ pageId }: { pageId: string | null }) {
  const revisions = useLiveQuery(
    async () => pageId
      ? (await db.revisions.where('pageId').equals(pageId).toArray()).sort((left, right) => right.createdAt - left.createdAt)
      : [],
    [pageId],
  );
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);

  const effectiveSelectedId = revisions?.some((revision) => revision.id === selectedId)
    ? selectedId
    : (revisions?.[0]?.id ?? null);
  const selected = revisions?.find((revision) => revision.id === effectiveSelectedId) ?? null;

  if (!pageId) return <EmptyHistory message="Open a note to inspect its history." />;
  if (!revisions) return <EmptyHistory message="Loading snapshots…" />;
  if (revisions.length === 0) {
    return <EmptyHistory message="Ley saves sparse recovery snapshots as this note changes. The first one appears after the next meaningful edit." />;
  }

  async function restoreSelected() {
    if (!selected || !pageId) return;
    setStatus('Restoring…');
    try {
      await updatePageContent(pageId, selected.content);
      setStatus('Snapshot restored. Your previous version was preserved too.');
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="border-b border-border p-3">
        <div className="mb-2 flex items-center gap-2 text-meta font-medium text-foreground">
          <History size={14} className="text-secondary" /> File recovery
        </div>
        <div className="space-y-1">
          {revisions.map((revision) => (
            <button
              key={revision.id}
              type="button"
              onClick={() => { setSelectedId(revision.id); setStatus(null); }}
              className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-meta ${effectiveSelectedId === revision.id ? 'bg-surface-3 text-foreground' : 'text-muted-foreground-strong hover:bg-surface-2'}`}
            >
              <Clock3 size={12} />
              <span>{formatDistanceToNow(revision.createdAt, { addSuffix: true })}</span>
            </button>
          ))}
        </div>
      </div>
      {selected && (
        <div className="flex min-h-0 flex-1 flex-col p-3">
          <div className="mb-2 text-micro uppercase tracking-[0.12em] text-muted-foreground">Snapshot preview</div>
          <pre className="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-lg border border-border bg-background p-3 font-mono text-micro leading-relaxed text-muted-foreground-strong">{selected.content}</pre>
          {status && <p className="mt-2 text-micro text-muted-foreground">{status}</p>}
          <button type="button" onClick={() => void restoreSelected()} className="mt-3 flex items-center justify-center gap-1.5 rounded-md bg-primary px-3 py-2 text-meta font-medium text-primary-foreground hover:opacity-90">
            <RotateCcw size={13} /> Restore this version
          </button>
        </div>
      )}
    </div>
  );
}

function EmptyHistory({ message }: { message: string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 px-7 text-center text-meta leading-relaxed text-muted-foreground">
      <History size={22} className="text-subtle-foreground" />
      <p>{message}</p>
    </div>
  );
}
