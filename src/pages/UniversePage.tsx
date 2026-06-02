import { useNavigate } from 'react-router-dom';
import { useNodes, useEdges } from '@/hooks';
import { PageHeader } from '@/components/layout';
import { UniverseView } from '@/components/universe/UniverseView';
import { GraphSettingsPanel } from '@/components/universe/GraphSettingsPanel';
import { Sliders, X } from 'lucide-react';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { useUniverseStore } from '@/store';
import { cn } from '@/lib/utils';

export function UniversePage() {
  const navigate = useNavigate();
  const { nodes: dbNodes } = useNodes();
  const { edges: dbEdges } = useEdges();
  const { settings, update } = useGraphSettings('global');
  const panelVisible = settings?.panelVisible ?? true;
  const { setSelectedNodes } = useUniverseStore();

  return (
    <div className="flex h-full flex-col">
      <PageHeader
        title="Universe"
        subtitle={`${dbNodes.length} pages, ${dbEdges.length} edges`}
        actions={
          <button
            onClick={() =>
              settings && update({ ...settings, panelVisible: !settings.panelVisible })
            }
            className={cn(
              'flex h-7 w-7 items-center justify-center rounded-md border border-foreground/[0.08] text-muted-foreground/80 transition-colors hover:bg-foreground/[0.04] hover:text-foreground'
            )}
            aria-label="Toggle graph settings panel"
            title="Toggle graph settings panel"
          >
            {panelVisible ? <X className="h-3.5 w-3.5" /> : <Sliders className="h-3.5 w-3.5" />}
          </button>
        }
      />

      <main className="relative flex flex-1 overflow-hidden">
        {dbNodes.length === 0 ? (
          <div className="flex h-full w-full items-center justify-center text-muted-foreground">
            <div className="max-w-sm space-y-2 text-center">
              <p className="text-[15px] text-foreground/90">No pages yet</p>
              <p className="text-[13px] text-muted-foreground/70">
                Create some pages and link them. The graph will appear here.
              </p>
            </div>
          </div>
        ) : (
          <>
            <div className="flex-1">
              <UniverseView
                scope="global"
                onNodeClick={(id) => {
                  setSelectedNodes([id]);
                  navigate(`/page/${id}`);
                }}
              />
            </div>
            {panelVisible && <GraphSettingsPanel scope="global" />}
          </>
        )}
      </main>
    </div>
  );
}
