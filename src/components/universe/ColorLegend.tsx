import { useGraph } from '@/hooks/useGraph';
import { useGraphSettings } from '@/hooks/useGraphSettings';
import { tagColor, collectionColor } from '@/lib/graph/tagColor';
import { linkCountColor, COMMUNITY_PALETTE } from '@/lib/graph/colors';
import type { ColorScheme, GraphScope } from '@/types/graph-settings.types';

export interface ColorLegendProps {
  scope: GraphScope;
}

export function ColorLegend({ scope }: ColorLegendProps) {
  const { settings } = useGraphSettings(scope);
  const { nodeMap, graph, communities } = useGraph();
  if (!settings) return null;

  const scheme: ColorScheme = settings.colorScheme;
  if (scheme === 'untyped') return null;

  let entries: { swatch: string; label: string }[] = [];

  if (scheme === 'tag') {
    const tags = Array.from(
      new Set(Array.from(nodeMap.values()).flatMap((n: any) => n.tags ?? []))
    ).sort();
    entries = tags.slice(0, 12).map((t) => ({ swatch: tagColor(t), label: t }));
  } else if (scheme === 'collection') {
    const cols = Array.from(
      new Set(Array.from(nodeMap.values()).flatMap((n: any) => n.collections ?? []))
    ).sort();
    entries = cols.slice(0, 12).map((c) => ({ swatch: collectionColor(c), label: c }));
  } else if (scheme === 'link-count') {
    let max = 0;
    graph.forEachNode((id) => {
      const d = graph.degree(id);
      if (d > max) max = d;
    });
    entries = [
      { swatch: linkCountColor(0, max), label: '0 links' },
      { swatch: linkCountColor(Math.round(max / 2), max), label: `${Math.round(max / 2)} links` },
      { swatch: linkCountColor(max, max), label: `${max} links` },
    ];
  } else if (scheme === 'community') {
    const part = communities?.communities ?? new Map<string, number>();
    const ids = Array.from(new Set(Array.from(part.values())));
    entries = ids.slice(0, COMMUNITY_PALETTE.length).map((c) => ({
      swatch: COMMUNITY_PALETTE[c % COMMUNITY_PALETTE.length],
      label: `Cluster ${c + 1}`,
    }));
  }

  if (entries.length === 0) return null;

  return (
    <div className="pointer-events-none absolute bottom-3 left-3 max-w-[240px] rounded-md border border-foreground/[0.08] bg-background/70 p-2.5 backdrop-blur">
      <div className="mb-1.5 text-[10.5px] uppercase tracking-wider text-muted-foreground/70">
        {scheme === 'link-count' ? 'Link count' : scheme}
      </div>
      <div className="flex flex-col gap-1">
        {entries.map((e, i) => (
          <div key={i} className="flex items-center gap-2 text-[11.5px] text-foreground/85">
            <span
              className="inline-block h-2.5 w-2.5 rounded-full"
              style={{ backgroundColor: e.swatch }}
            />
            <span className="truncate">{e.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
