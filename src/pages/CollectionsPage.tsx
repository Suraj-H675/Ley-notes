import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/lib/db';
import { useWorkspaceStore } from '@/store';
import { ConfirmDialog, toast } from '@/components/ui';
import { PageHeader, PageContainer, ListSection } from '@/components/layout';
import { ChevronRight, Plus, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { nanoid } from 'nanoid';

export function CollectionsPage() {
  const navigate = useNavigate();
  const { expandedCollections, toggleCollection } = useWorkspaceStore();

  const collections = useLiveQuery(() => db.collections.toArray(), []);

  const collectionNodes = useLiveQuery(
    async () => {
      if (!collections) {
        return { counts: new Map<string, number>(), nodesByCollection: new Map<string, any[]>() };
      }
      const nodes = await db.nodes.where('isArchived').equals(0).toArray();
      const counts = new Map<string, number>();
      const nodesByCollection = new Map<string, any[]>();
      nodes.forEach((node) => {
        node.collections.forEach((colId) => {
          counts.set(colId, (counts.get(colId) || 0) + 1);
          if (!nodesByCollection.has(colId)) nodesByCollection.set(colId, []);
          nodesByCollection.get(colId)!.push(node);
        });
      });
      return { counts, nodesByCollection };
    },
    [collections]
  );

  const [pendingDelete, setPendingDelete] = useState<{ id: string; name: string } | null>(null);

  if (!collections) return null;

  const handleDelete = async (id: string, name: string) => {
    await db.transaction('rw', [db.collections, db.nodes], async () => {
      const nodesInCollection = await db.nodes.where('collections').equals(id).toArray();
      for (const node of nodesInCollection) {
        await db.nodes.update(node.id, {
          collections: node.collections.filter((c) => c !== id),
          updatedAt: Date.now(),
        });
      }
      await db.collections.delete(id);
    });
    toast(`Deleted "${name}"`, { kind: 'success' });
  };

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Collections"
        subtitle={`${collections.length} total`}
        actions={<NewCollectionButton />}
      />

      <PageContainer>
        <ListSection title="All collections">
          {collections.length === 0 ? (
            <EmptyState />
          ) : (
            <ul className="divide-y divide-border/40">
              {collections.map((collection) => (
                <CollectionRow
                  key={collection.id}
                  collection={collection}
                  isExpanded={expandedCollections.includes(collection.id)}
                  onToggle={() => toggleCollection(collection.id)}
                  nodeCount={collectionNodes?.counts?.get(collection.id) || 0}
                  nodes={collectionNodes?.nodesByCollection?.get(collection.id) || []}
                  onOpenNode={(id) => navigate(`/page/${id}`)}
                  onDelete={() => setPendingDelete({ id: collection.id, name: collection.name })}
                />
              ))}
            </ul>
          )}
        </ListSection>
      </PageContainer>

      <ConfirmDialog
        open={!!pendingDelete}
        onOpenChange={(o) => !o && setPendingDelete(null)}
        title={`Delete "${pendingDelete?.name}"?`}
        description="Pages in this collection will not be deleted, just unlinked from it."
        confirmLabel="Delete"
        destructive
        onConfirm={async () => {
          if (pendingDelete) {
            await handleDelete(pendingDelete.id, pendingDelete.name);
            setPendingDelete(null);
          }
        }}
      />
    </div>
  );
}

function CollectionRow({
  collection,
  isExpanded,
  onToggle,
  nodeCount,
  nodes,
  onOpenNode,
  onDelete,
}: {
  collection: { id: string; name: string; emoji?: string };
  isExpanded: boolean;
  onToggle: () => void;
  nodeCount: number;
  nodes: any[];
  onOpenNode: (id: string) => void;
  onDelete: () => void;
}) {
  return (
    <li>
      <div className="group/row flex items-center gap-2 px-1 py-2 transition-colors hover:bg-accent/30">
        <button
          onClick={onToggle}
          aria-label={isExpanded ? 'Collapse' : 'Expand'}
          className="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded text-muted-foreground/60"
        >
          <ChevronRight
            className={cn('h-3.5 w-3.5 transition-transform', isExpanded && 'rotate-90')}
          />
        </button>
        <button onClick={onToggle} className="flex min-w-0 flex-1 items-center gap-2.5 text-left">
          <span className="flex h-5 w-5 flex-shrink-0 items-center justify-center text-[14px] leading-none">
            {collection.emoji || (
              <span className="block h-1.5 w-1.5 rounded-full bg-muted-foreground/40" />
            )}
          </span>
          <span className="flex-1 truncate text-[13.5px] text-foreground/90">{collection.name}</span>
          <span className="rounded bg-accent/60 px-1.5 py-0.5 text-[11px] tabular-nums text-muted-foreground/70">
            {nodeCount}
          </span>
        </button>
        <button
          onClick={onDelete}
          aria-label="Delete collection"
          className="flex h-6 w-6 flex-shrink-0 items-center justify-center rounded text-muted-foreground/50 opacity-0 transition-opacity hover:bg-accent hover:text-foreground group-hover/row:opacity-100"
        >
          <Trash2 className="h-3 w-3" />
        </button>
      </div>
      {isExpanded && nodes.length > 0 && (
        <ul className="ml-7 space-y-0.5 pb-1.5">
          {nodes.map((node) => (
            <li key={node.id}>
              <button
                onClick={() => onOpenNode(node.id)}
                className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-[13px] text-foreground/80 transition-colors hover:bg-accent/40"
              >
                <span className="flex h-3.5 w-3.5 flex-shrink-0 items-center justify-center text-[12px] leading-none">
                  {node.emoji || (
                    <span className="block h-1 w-1 rounded-full bg-muted-foreground/40" />
                  )}
                </span>
                <span className="flex-1 truncate">{node.title || 'Untitled'}</span>
                <span className="text-[11px] capitalize text-muted-foreground/55">{node.type}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}

function NewCollectionButton() {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState('');

  const handleCreate = async () => {
    if (!name.trim()) {
      setEditing(false);
      return;
    }
    const now = Date.now();
    await db.collections.add({
      id: nanoid(),
      name: name.trim(),
      createdAt: now,
      updatedAt: now,
    });
    setName('');
    setEditing(false);
  };

  if (!editing) {
    return (
      <button
        onClick={() => setEditing(true)}
        className="inline-flex h-7 items-center gap-1 rounded-md bg-foreground px-2.5 text-[12.5px] font-medium text-background transition-opacity hover:opacity-90"
      >
        <Plus className="h-3 w-3" />
        New collection
      </button>
    );
  }

  return (
    <input
      autoFocus
      value={name}
      onChange={(e) => setName(e.target.value)}
      onBlur={handleCreate}
      onKeyDown={(e) => {
        if (e.key === 'Enter') handleCreate();
        if (e.key === 'Escape') {
          setName('');
          setEditing(false);
        }
      }}
      placeholder="Collection name"
      className="h-7 w-44 rounded-md border border-border bg-background px-2 text-[13px] outline-none focus:ring-1 focus:ring-ring"
    />
  );
}

function EmptyState() {
  return (
    <div className="rounded-lg border border-dashed border-border/60 px-6 py-12 text-center">
      <p className="text-[13px] text-muted-foreground/70">
        No collections yet. Group related pages into a collection to find them faster.
      </p>
    </div>
  );
}
