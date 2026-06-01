import { useState } from 'react';
import { useWorkspaceStore } from '@/store';
import { ChevronDown, ChevronRight, Plus, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { db } from '@/lib/db';
import { nanoid } from 'nanoid';

interface Collection {
  id: string;
  name: string;
  emoji?: string;
  parentId?: string;
  createdAt: number;
  updatedAt: number;
}

interface SidebarCollectionsProps {
  collections: Collection[];
}

export function SidebarCollections({ collections }: SidebarCollectionsProps) {
  const { expandedCollections, toggleCollection } = useWorkspaceStore();
  const rootCollections = collections.filter((c) => !c.parentId);

  return (
    <div className="space-y-0.5">
      {rootCollections.length > 0 && (
        <div className="mb-1 mt-2 flex items-center justify-between px-2">
          <span className="text-[11px] font-medium text-muted-foreground/70">
            Collections
          </span>
          <NewCollectionButton />
        </div>
      )}

      {rootCollections.map((collection) => (
        <CollectionItem
          key={collection.id}
          collection={collection}
          allCollections={collections}
          expandedCollections={expandedCollections}
          onToggle={() => toggleCollection(collection.id)}
          level={0}
        />
      ))}

      {rootCollections.length === 0 && <NewCollectionRow />}
    </div>
  );
}

function NewCollectionButton() {
  const handleCreate = async () => {
    const name = window.prompt('Collection name');
    if (!name) return;
    const now = Date.now();
    await db.collections.add({
      id: nanoid(),
      name,
      createdAt: now,
      updatedAt: now,
    });
  };

  return (
    <button
      onClick={handleCreate}
      aria-label="New collection"
      className="flex h-5 w-5 items-center justify-center rounded text-muted-foreground/60 opacity-0 transition-opacity hover:bg-accent hover:text-foreground group-hover:opacity-100"
    >
      <Plus className="h-3 w-3" />
    </button>
  );
}

function NewCollectionRow() {
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
        className="flex w-full items-center gap-1.5 px-2 py-1 text-[13px] text-muted-foreground/70 transition-colors hover:bg-accent/50 hover:text-foreground"
      >
        <Plus className="h-3.5 w-3.5" />
        <span>Add a collection</span>
      </button>
    );
  }

  return (
    <div className="flex items-center gap-1 px-2 py-1">
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
        className="h-6 flex-1 rounded border border-border bg-background px-1.5 text-[13px] outline-none focus:ring-1 focus:ring-ring"
      />
    </div>
  );
}

interface CollectionItemProps {
  collection: Collection;
  allCollections: Collection[];
  expandedCollections: string[];
  onToggle: () => void;
  level: number;
}

function CollectionItem({
  collection,
  allCollections,
  expandedCollections,
  onToggle,
  level,
}: CollectionItemProps) {
  const isExpanded = expandedCollections.includes(collection.id);
  const childCollections = allCollections.filter((c) => c.parentId === collection.id);
  const hasChildren = childCollections.length > 0;

  const handleDelete = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!window.confirm(`Delete collection "${collection.name}"?`)) return;
    await db.transaction('rw', [db.collections, db.nodes], async () => {
      const nodesInCollection = await db.nodes
        .where('collections')
        .equals(collection.id)
        .toArray();
      for (const node of nodesInCollection) {
        await db.nodes.update(node.id, {
          collections: node.collections.filter((c) => c !== collection.id),
          updatedAt: Date.now(),
        });
      }
      await db.collections.delete(collection.id);
    });
  };

  return (
    <div className="group/row relative">
      <button
        onClick={onToggle}
        className={cn(
          'group flex w-full items-center gap-1 rounded px-1.5 py-1 text-[13px] text-foreground/80 transition-colors',
          'hover:bg-accent/60'
        )}
        style={{ paddingLeft: `${level * 10 + 6}px` }}
      >
        {hasChildren ? (
          isExpanded ? (
            <ChevronDown className="h-3 w-3 flex-shrink-0 text-muted-foreground/60" />
          ) : (
            <ChevronRight className="h-3 w-3 flex-shrink-0 text-muted-foreground/60" />
          )
        ) : (
          <span className="inline-block h-3 w-3 flex-shrink-0" />
        )}
        {collection.emoji ? (
          <span className="flex-shrink-0 text-[13px] leading-none">{collection.emoji}</span>
        ) : (
          <span className="flex h-3.5 w-3.5 flex-shrink-0 items-center justify-center text-muted-foreground/40">
            <span className="h-1 w-1 rounded-full bg-current" />
          </span>
        )}
        <span className="truncate">{collection.name}</span>

        <button
          onClick={handleDelete}
          aria-label="Delete collection"
          className="ml-auto flex h-5 w-5 flex-shrink-0 items-center justify-center rounded text-muted-foreground/50 opacity-0 transition-opacity hover:bg-accent hover:text-foreground group-hover/row:opacity-100"
        >
          <Trash2 className="h-3 w-3" />
        </button>
      </button>

      {isExpanded &&
        childCollections.map((child) => (
          <CollectionItem
            key={child.id}
            collection={child}
            allCollections={allCollections}
            expandedCollections={expandedCollections}
            onToggle={() => onToggle /* propagate to handle child via parent store */}
            level={level + 1}
          />
        ))}
    </div>
  );
}
