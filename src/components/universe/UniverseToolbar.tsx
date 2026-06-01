import { useUniverseStore } from '@/store';
import { Button } from '@/components/ui';
import {
  Map,
  Maximize2,
  Tag,
  LayoutGrid,
  Circle,
  Network,
} from 'lucide-react';
import { cn } from '@/lib/utils';

interface UniverseToolbarProps {
  onFitView: () => void;
  className?: string;
}

export function UniverseToolbar({ onFitView, className }: UniverseToolbarProps) {
  const {
    showMiniMap,
    toggleMiniMap,
    showLabels,
    toggleLabels,
    layoutMode,
    setLayoutMode,
    filterType,
    setFilterType,
  } = useUniverseStore();

  const nodeTypes = ['document', 'task', 'project', 'concept'] as const;

  return (
    <div
      className={cn(
        'flex items-center gap-2 p-2 rounded-lg border bg-card shadow-md',
        className
      )}
    >
      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleMiniMap}
          className={cn(showMiniMap && 'bg-accent')}
          title="Toggle minimap"
        >
          <Map className="h-4 w-4" />
        </Button>

        <Button
          variant="ghost"
          size="icon"
          onClick={onFitView}
          title="Fit view"
        >
          <Maximize2 className="h-4 w-4" />
        </Button>
      </div>

      <div className="w-px h-6 bg-border" />

      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleLabels}
          className={cn(showLabels && 'bg-accent')}
          title="Toggle labels"
        >
          <Tag className="h-4 w-4" />
        </Button>
      </div>

      <div className="w-px h-6 bg-border" />

      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setLayoutMode('force')}
          className={cn(layoutMode === 'force' && 'bg-accent')}
          title="Force layout"
        >
          <Network className="h-4 w-4" />
        </Button>

        <Button
          variant="ghost"
          size="icon"
          onClick={() => setLayoutMode('circular')}
          className={cn(layoutMode === 'circular' && 'bg-accent')}
          title="Circular layout"
        >
          <Circle className="h-4 w-4" />
        </Button>

        <Button
          variant="ghost"
          size="icon"
          onClick={() => setLayoutMode('grid')}
          className={cn(layoutMode === 'grid' && 'bg-accent')}
          title="Grid layout"
        >
          <LayoutGrid className="h-4 w-4" />
        </Button>
      </div>

      <div className="w-px h-6 bg-border" />

      <div className="flex items-center gap-1">
        <Button
          variant={filterType === null ? 'secondary' : 'ghost'}
          size="sm"
          onClick={() => setFilterType(null)}
        >
          All
        </Button>
        {nodeTypes.map((type) => (
          <Button
            key={type}
            variant={filterType === type ? 'secondary' : 'ghost'}
            size="sm"
            onClick={() => setFilterType(type)}
            className="capitalize"
          >
            {type}
          </Button>
        ))}
      </div>
    </div>
  );
}
