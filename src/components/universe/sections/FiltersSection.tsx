import { useGraph } from '@/hooks/useGraph';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import type { GraphScope } from '@/types/graph-settings.types';

export interface FiltersSectionProps {
  scope: GraphScope;
}

export function FiltersSection({ scope }: FiltersSectionProps) {
  const { settings, update } = useGraphSettings(scope);
  const { nodeMap } = useGraph();

  if (!settings) return null;

  const allTags = Array.from(
    new Set(Array.from(nodeMap.values()).flatMap((n: any) => n.tags ?? []))
  ).sort();
  const allCollections = Array.from(
    new Set(Array.from(nodeMap.values()).flatMap((n: any) => n.collections ?? []))
  ).sort();

  const setQuery = (v: string) =>
    update({ ...settings, filters: { ...settings.filters, searchQuery: v } });

  const toggleTag = (t: string) => {
    const cur = settings.filters.selectedTags;
    update({
      ...settings,
      filters: {
        ...settings.filters,
        selectedTags: cur.includes(t) ? cur.filter((x) => x !== t) : [...cur, t],
      },
    });
  };

  const toggleCollection = (c: string) => {
    const cur = settings.filters.selectedCollections;
    update({
      ...settings,
      filters: {
        ...settings.filters,
        selectedCollections: cur.includes(c)
          ? cur.filter((x) => x !== c)
          : [...cur, c],
      },
    });
  };

  return (
    <div className="flex flex-col gap-3">
      <input
        type="text"
        value={settings.filters.searchQuery}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search…"
        className="w-full rounded-md border border-foreground/[0.08] bg-background/40 px-2.5 py-1.5 text-[12.5px] text-foreground placeholder:text-muted-foreground/60 focus:border-foreground/[0.18] focus:outline-none"
      />

      <label className="flex items-center gap-2 text-[12.5px] text-foreground/85">
        <input
          type="checkbox"
          checked={settings.filters.showOrphans}
          onChange={(e) =>
            update({
              ...settings,
              filters: { ...settings.filters, showOrphans: e.target.checked },
            })
          }
        />
        <span>Show orphans</span>
      </label>

      {allTags.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <div className="text-[10.5px] uppercase tracking-wider text-muted-foreground/70">
            Tags
          </div>
          <div className="flex flex-wrap gap-1">
            {allTags.map((t) => {
              const active = settings.filters.selectedTags.includes(t);
              return (
                <button
                  key={t}
                  type="button"
                  onClick={() => toggleTag(t)}
                  className={
                    'rounded-full border px-2 py-0.5 text-[10.5px] transition-colors ' +
                    (active
                      ? 'border-foreground/30 bg-foreground/[0.08] text-foreground'
                      : 'border-foreground/[0.08] text-foreground/75 hover:bg-foreground/[0.04]')
                  }
                >
                  {t}
                </button>
              );
            })}
          </div>
        </div>
      )}

      {allCollections.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <div className="text-[10.5px] uppercase tracking-wider text-muted-foreground/70">
            Collections
          </div>
          <div className="flex flex-wrap gap-1">
            {allCollections.map((c) => {
              const active = settings.filters.selectedCollections.includes(c);
              return (
                <button
                  key={c}
                  type="button"
                  onClick={() => toggleCollection(c)}
                  className={
                    'rounded-full border px-2 py-0.5 text-[10.5px] transition-colors ' +
                    (active
                      ? 'border-foreground/30 bg-foreground/[0.08] text-foreground'
                      : 'border-foreground/[0.08] text-foreground/75 hover:bg-foreground/[0.04]')
                  }
                >
                  {c}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
