import { useNavigate } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { useWorkspaceStore } from '@/store';
import { Button } from '@/components/ui';
import { ArrowLeft, FolderOpen, Plus, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import { nanoid } from 'nanoid';

export function CollectionsPage() {
  const navigate = useNavigate();
  const { expandedCollections, toggleCollection } = useWorkspaceStore();

  const collections = useLiveQuery(() => db.collections.toArray(), []);

  const collectionNodes = useLiveQuery(
    async () => {
      if (!collections) return { counts: new Map<string, number>(), nodesByCollection: new Map<string, typeof nodes>() };
      const nodes = await db.nodes.where('isArchived').equals(0).toArray();
      const counts = new Map<string, number>();
      const nodesByCollection = new Map<string, typeof nodes>();

      nodes.forEach((node) => {
        node.collections.forEach((colId) => {
          counts.set(colId, (counts.get(colId) || 0) + 1);
          if (!nodesByCollection.has(colId)) {
            nodesByCollection.set(colId, []);
          }
          nodesByCollection.get(colId)!.push(node);
        });
      });
      return { counts, nodesByCollection };
    },
    [collections]
  );

  const handleCreateCollection = async () => {
    const name = prompt('Collection name:');
    if (!name) return;
    const now = Date.now();
    await db.collections.add({
      id: nanoid(),
      name,
      emoji: '📂',
      createdAt: now,
      updatedAt: now,
    });
  };

  if (!collections) {
    return null;
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
        <h1 className="text-xl font-semibold">Collections</h1>
        <div className="flex-1" />
        <Button size="sm" onClick={handleCreateCollection}>
          <Plus className="h-4 w-4 mr-2" />
          New Collection
        </Button>
      </header>

      <main className="flex-1 overflow-auto p-8">
        <div className="max-w-2xl mx-auto space-y-2">
          {collections.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <FolderOpen className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>No collections yet. Create your first collection to organize your knowledge.</p>
            </div>
          ) : (
            collections.map((collection) => {
              const isExpanded = expandedCollections.includes(collection.id);
              const nodeCount = collectionNodes?.counts?.get(collection.id) || 0;
              const collectionNodeList = collectionNodes?.nodesByCollection?.get(collection.id) || [];

              return (
                <div key={collection.id} className="border rounded-lg overflow-hidden">
                  <button
                    onClick={() => toggleCollection(collection.id)}
                    className="w-full flex items-center gap-3 p-4 hover:bg-accent transition-colors"
                  >
                    <ChevronRight
                      className={cn(
                        'h-4 w-4 text-muted-foreground transition-transform',
                        isExpanded && 'rotate-90'
                      )}
                    />
                    <span className="text-xl">{collection.emoji || '📂'}</span>
                    <div className="flex-1 text-left">
                      <p className="font-medium">{collection.name}</p>
                      <p className="text-xs text-muted-foreground">
                        {nodeCount} item{nodeCount !== 1 ? 's' : ''}
                      </p>
                    </div>
                  </button>

                  {isExpanded && nodeCount > 0 && (
                    <div className="border-t bg-accent/30 p-2 space-y-1">
                      {collectionNodeList.map((node) => (
                        <button
                          key={node.id}
                          onClick={() => navigate(`/page/${node.id}`)}
                          className="w-full flex items-center gap-2 px-2 py-2 rounded text-sm text-left hover:bg-accent transition-colors"
                        >
                          <span className="text-lg">{node.emoji || '📄'}</span>
                          <div className="flex-1 min-w-0">
                            <p className="truncate font-medium">{node.title || 'Untitled'}</p>
                            <p className="text-xs text-muted-foreground">{node.type}</p>
                          </div>
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>
      </main>
    </div>
  );
}
