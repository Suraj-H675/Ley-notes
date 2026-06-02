import { useGraphSettings } from '@/hooks/useGraphSettings';
import { cn } from '@/lib/utils';
import type { ColorScheme, GraphScope } from '@/types/graph-settings.types';

const SCHEMES: { value: ColorScheme; label: string; hint: string }[] = [
  { value: 'untyped', label: 'Untyped', hint: 'All nodes one color' },
  { value: 'tag', label: 'Tag', hint: 'Color by first tag' },
  { value: 'collection', label: 'Collection', hint: 'Color by collection' },
  { value: 'link-count', label: 'Links', hint: 'Gradient by degree' },
  { value: 'community', label: 'Community', hint: 'Louvain clusters' },
];

export interface GroupsSectionProps {
  scope: GraphScope;
}

export function GroupsSection({ scope }: GroupsSectionProps) {
  const { settings, update } = useGraphSettings(scope);
  if (!settings) return null;

  return (
    <div className="flex flex-col gap-1.5">
      {SCHEMES.map((s) => {
        const active = settings.colorScheme === s.value;
        return (
          <button
            key={s.value}
            type="button"
            onClick={() => update({ ...settings, colorScheme: s.value })}
            className={cn(
              'flex items-center justify-between rounded-md px-2.5 py-1.5 text-left text-[12.5px] transition-colors',
              active
                ? 'bg-foreground/[0.06] text-foreground'
                : 'text-foreground/85 hover:bg-foreground/[0.04]'
            )}
          >
            <div className="flex flex-col">
              <span>{s.label}</span>
              <span className="text-[10.5px] text-muted-foreground/70">{s.hint}</span>
            </div>
            {active && <span className="text-[10.5px] text-foreground/70">●</span>}
          </button>
        );
      })}
    </div>
  );
}
