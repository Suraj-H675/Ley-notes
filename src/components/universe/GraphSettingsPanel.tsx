import { useGraphSettings } from '@/hooks/useGraphSettings';
import { CollapsibleSection } from './CollapsibleSection';
import { GroupsSection } from './sections/GroupsSection';
import { FiltersSection } from './sections/FiltersSection';
import { DisplaySection } from './sections/DisplaySection';
import { PhysicsSection } from './sections/PhysicsSection';
import { cn } from '@/lib/utils';
import type { GraphScope } from '@/types/graph-settings.types';

export interface GraphSettingsPanelProps {
  scope: GraphScope;
  className?: string;
}

export function GraphSettingsPanel({ scope, className }: GraphSettingsPanelProps) {
  const { settings, update } = useGraphSettings(scope);

  if (!settings) {
    return (
      <aside
        className={cn(
          'w-[280px] shrink-0 border-l border-foreground/[0.06] bg-foreground/[0.015]',
          className
        )}
      >
        <div className="p-4 text-[12px] text-muted-foreground/70">Loading…</div>
      </aside>
    );
  }

  const toggle = (key: keyof typeof settings.panelSectionsOpen) =>
    update({
      ...settings,
      panelSectionsOpen: {
        ...settings.panelSectionsOpen,
        [key]: !settings.panelSectionsOpen[key],
      },
    });

  return (
    <aside
      className={cn(
        'flex w-[280px] shrink-0 flex-col border-l border-foreground/[0.06] bg-foreground/[0.015] text-foreground',
        className
      )}
    >
      <div className="border-b border-foreground/[0.06] px-4 py-2.5 text-[12px] font-medium uppercase tracking-wider text-foreground/85">
        Graph settings
      </div>

      <div className="flex-1 overflow-y-auto">
        <CollapsibleSection
          title="Groups"
          open={settings.panelSectionsOpen.groups}
          onToggle={() => toggle('groups')}
        >
          <GroupsSection scope={scope} />
        </CollapsibleSection>
        <CollapsibleSection
          title="Filters"
          open={settings.panelSectionsOpen.filters}
          onToggle={() => toggle('filters')}
        >
          <FiltersSection scope={scope} />
        </CollapsibleSection>
        <CollapsibleSection
          title="Display"
          open={settings.panelSectionsOpen.display}
          onToggle={() => toggle('display')}
        >
          <DisplaySection scope={scope} />
        </CollapsibleSection>
        <CollapsibleSection
          title="Physics"
          open={settings.panelSectionsOpen.physics}
          onToggle={() => toggle('physics')}
        >
          <PhysicsSection scope={scope} />
        </CollapsibleSection>
      </div>
    </aside>
  );
}
