import { useWorkspaceStore } from '@/store';
import { ChevronRight, ChevronDown, Folder, FolderPlus } from 'lucide-react';
import { Button } from '@/components/ui';

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
    <div className="space-y-1">
      <div className="flex items-center justify-between px-2">
        <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Collections
        </h3>
        <Button variant="ghost" size="icon" className="h-6 w-6">
          <FolderPlus className="h-3 w-3" />
        </Button>
      </div>

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
  const { toggleCollection } = useWorkspaceStore();
  const isExpanded = expandedCollections.includes(collection.id);
  const childCollections = allCollections.filter((c) => c.parentId === collection.id);
  const hasChildren = childCollections.length > 0;

  return (
    <div>
      <button
        onClick={onToggle}
        className="flex w-full items-center gap-1 rounded-md px-2 py-1 text-sm hover:bg-accent hover:text-accent-foreground"
        style={{ paddingLeft: `${level * 12 + 8}px` }}
      >
        {hasChildren ? (
          isExpanded ? (
            <ChevronDown className="h-3 w-3 flex-shrink-0" />
          ) : (
            <ChevronRight className="h-3 w-3 flex-shrink-0" />
          )
        ) : (
          <span className="h-3 w-3" />
        )}
        <Folder className="h-4 w-4 flex-shrink-0" />
        <span className="truncate">{collection.name}</span>
      </button>

      {isExpanded &&
        childCollections.map((child) => (
          <CollectionItem
            key={child.id}
            collection={child}
            allCollections={allCollections}
            expandedCollections={expandedCollections}
            onToggle={() => toggleCollection(child.id)}
            level={level + 1}
          />
        ))}
    </div>
  );
}
