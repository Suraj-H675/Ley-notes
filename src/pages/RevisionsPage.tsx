import { useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { Button } from '@/components/ui';
import { ArrowLeft, Clock, RotateCcw } from 'lucide-react';
import { formatRelative } from '@/lib/utils';
import { cn } from '@/lib/utils';

export function RevisionsPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [selectedRevisionId, setSelectedRevisionId] = useState<string | null>(null);

  const node = useLiveQuery(
    () => (id ? db.nodes.get(id) : undefined),
    [id]
  );

  const revisions = useLiveQuery(
    async () => {
      if (!id) return [];
      return db.revisions
        .where('nodeId')
        .equals(id)
        .reverse()
        .sortBy('createdAt');
    },
    [id]
  );

  const selectedRevision = useLiveQuery(
    async () => {
      if (!selectedRevisionId) return null;
      return db.revisions.get(selectedRevisionId);
    },
    [selectedRevisionId]
  );

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
      <div className="h-full flex items-center justify-center">
        <div className="text-center space-y-4">
          <h2 className="text-2xl font-bold">Node not found</h2>
          <p className="text-muted-foreground">
            This node doesn't exist or has been deleted.
          </p>
          <Button variant="outline" onClick={() => navigate('/')}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            Go Home
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center gap-4 border-b p-4">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => navigate(-1)}
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div className="flex-1">
          <h1 className="text-xl font-semibold">Revision History</h1>
          <p className="text-sm text-muted-foreground">
            {node.emoji && <span className="mr-1">{node.emoji}</span>}
            {node.title || 'Untitled'}
          </p>
        </div>
        {selectedRevision && (
          <Button size="sm" onClick={handleRestore}>
            <RotateCcw className="h-4 w-4 mr-2" />
            Restore this version
          </Button>
        )}
      </header>

      <div className="flex-1 flex overflow-hidden">
        {/* Revision list */}
        <div className="w-80 border-r overflow-y-auto p-4">
          {revisions && revisions.length > 0 ? (
            <div className="space-y-2">
              {revisions.map((revision) => (
                <button
                  key={revision.id}
                  onClick={() => setSelectedRevisionId(revision.id)}
                  className={cn(
                    'w-full text-left p-3 rounded-lg border transition-colors',
                    selectedRevisionId === revision.id
                      ? 'bg-accent border-primary'
                      : 'hover:bg-accent/50'
                  )}
                >
                  <div className="flex items-center gap-2 text-sm font-medium">
                    <Clock className="h-4 w-4 text-muted-foreground" />
                    {formatRelative(revision.createdAt)}
                  </div>
                  <p className="text-xs text-muted-foreground mt-1 truncate">
                    {revision.plainText.slice(0, 100) || 'Empty'}
                  </p>
                </button>
              ))}
            </div>
          ) : (
            <div className="text-center py-8 text-muted-foreground">
              <Clock className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p className="text-sm">No revisions yet</p>
            </div>
          )}
        </div>

        {/* Preview */}
        <div className="flex-1 overflow-y-auto p-8">
          {selectedRevision ? (
            <div className="max-w-3xl mx-auto">
              <div className="text-sm text-muted-foreground mb-4">
                Preview from {formatRelative(selectedRevision.createdAt)}
              </div>
              <div className="prose prose-sm max-w-none">
                {selectedRevision.plainText || 'No content'}
              </div>
            </div>
          ) : (
            <div className="h-full flex items-center justify-center text-muted-foreground">
              <div className="text-center">
                <Clock className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p className="text-lg font-medium">Select a revision</p>
                <p className="text-sm">Choose a version from the list to preview</p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
