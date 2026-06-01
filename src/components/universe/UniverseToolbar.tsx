import { useUniverseStore } from '@/store';
import {
  Map,
  Maximize2,
  TextCursorInput,
  Network,
  Circle,
  LayoutGrid,
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
        'flex items-center gap-1 rounded-md border border-border/60 bg-card/70 p-0.5 backdrop-blur',
        className
      )}
    >
      <ToolGroup>
        <ToolButton active={showMiniMap} onClick={toggleMiniMap} label="Minimap">
          <Map className="h-3.5 w-3.5" />
        </ToolButton>
        <ToolButton onClick={onFitView} label="Fit view">
          <Maximize2 className="h-3.5 w-3.5" />
        </ToolButton>
      </ToolGroup>

      <Divider />

      <ToolGroup>
        <ToolButton active={showLabels} onClick={toggleLabels} label="Labels">
          <TextCursorInput className="h-3.5 w-3.5" />
        </ToolButton>
      </ToolGroup>

      <Divider />

      <ToolGroup>
        <ToolButton
          active={layoutMode === 'force'}
          onClick={() => setLayoutMode('force')}
          label="Force"
        >
          <Network className="h-3.5 w-3.5" />
        </ToolButton>
        <ToolButton
          active={layoutMode === 'circular'}
          onClick={() => setLayoutMode('circular')}
          label="Circular"
        >
          <Circle className="h-3.5 w-3.5" />
        </ToolButton>
        <ToolButton
          active={layoutMode === 'grid'}
          onClick={() => setLayoutMode('grid')}
          label="Grid"
        >
          <LayoutGrid className="h-3.5 w-3.5" />
        </ToolButton>
      </ToolGroup>

      <Divider />

      <div className="flex items-center gap-0.5 px-1">
        <FilterPill
          active={filterType === null}
          onClick={() => setFilterType(null)}
        >
          All
        </FilterPill>
        {nodeTypes.map((type) => (
          <FilterPill
            key={type}
            active={filterType === type}
            onClick={() => setFilterType(filterType === type ? null : type)}
            className="capitalize"
          >
            {type}
          </FilterPill>
        ))}
      </div>
    </div>
  );
}

function ToolGroup({ children }: { children: React.ReactNode }) {
  return <div className="flex items-center gap-0.5 p-0.5">{children}</div>;
}

function ToolButton({
  active,
  onClick,
  label,
  children,
}: {
  active?: boolean;
  onClick: () => void;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      title={label}
      aria-label={label}
      className={cn(
        'flex h-6 w-6 items-center justify-center rounded text-muted-foreground/80 transition-colors',
        active ? 'bg-accent text-foreground' : 'hover:bg-accent/60 hover:text-foreground'
      )}
    >
      {children}
    </button>
  );
}

function FilterPill({
  active,
  onClick,
  className,
  children,
}: {
  active: boolean;
  onClick: () => void;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'rounded px-1.5 py-0.5 text-[11.5px] transition-colors',
        active
          ? 'bg-accent text-foreground'
          : 'text-muted-foreground/75 hover:bg-accent/40 hover:text-foreground',
        className
      )}
    >
      {children}
    </button>
  );
}

function Divider() {
  return <span className="mx-0.5 h-4 w-px bg-border/60" />;
}
