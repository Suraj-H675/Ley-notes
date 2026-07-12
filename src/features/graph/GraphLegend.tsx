/**
 * GraphLegend — community/cluster legend with toggle visibility (Graphify
 * pattern). Click a community to show/hide its nodes. Master checkbox toggles
 * all at once with indeterminate state when partial.
 */

import { Check, ChevronDown, ChevronRight } from 'lucide-react';
import { useState } from 'react';
import { cn } from '@/shared/lib/classnames';

export interface GraphLegendProps {
  /** Map of community id → count. */
  communitySizes: Map<number, number>;
  /** Set of community ids currently hidden. */
  hiddenCommunities: Set<number>;
  onToggle: (communityId: number) => void;
  onToggleAll: (hide: boolean) => void;
}

const PALETTE = [
  'hsl(217 70% 62%)',
  'hsl(265 55% 65%)',
  'hsl(150 50% 55%)',
  'hsl(35 70% 60%)',
  'hsl(0 55% 58%)',
  'hsl(195 60% 55%)',
  'hsl(50 65% 55%)',
  'hsl(290 50% 65%)',
  'hsl(105 45% 55%)',
  'hsl(330 60% 65%)',
];

export function GraphLegend({
  communitySizes,
  hiddenCommunities,
  onToggle,
  onToggleAll,
}: GraphLegendProps) {
  const [open, setOpen] = useState(true);

  const communities = [...communitySizes.entries()]
    .map(([id, count]) => ({ id, count }))
    .sort((a, b) => b.count - a.count);

  const total = communities.length;
  const hidden = hiddenCommunities.size;
  const allChecked = hidden === 0;
  const indeterminate = hidden > 0 && hidden < total;

  return (
    <div className="border-t border-border bg-surface-1">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-1 px-3 py-2 text-left text-meta font-semibold uppercase tracking-wide text-muted-foreground hover:bg-surface-2"
      >
        {open ? <ChevronDown size={11} /> : <ChevronRight size={11} />}
        <span>Communities ({total})</span>
      </button>
      {open && (
        <div className="px-2 pb-2">
          <label className="mb-1 flex cursor-pointer items-center gap-2 rounded-sm px-1.5 py-1 text-meta hover:bg-surface-2">
            <input
              type="checkbox"
              checked={allChecked}
              ref={(el) => {
                if (el) el.indeterminate = indeterminate;
              }}
              onChange={(e) => onToggleAll(!e.target.checked)}
              className="h-3 w-3 rounded-sm border border-border bg-surface-1 accent-primary"
            />
            <span className="text-muted-foreground-strong">Toggle all</span>
          </label>
          <ul className="flex max-h-48 flex-col gap-0.5 overflow-y-auto">
            {communities.map(({ id, count }) => {
              const isHidden = hiddenCommunities.has(id);
              return (
                <li key={id}>
                  <button
                    type="button"
                    onClick={() => onToggle(id)}
                    className={cn(
                      'flex w-full items-center gap-1.5 rounded-sm px-1.5 py-0.5 text-left text-meta',
                      isHidden ? 'opacity-40' : 'hover:bg-surface-2',
                    )}
                  >
                    <span
                      className="inline-flex h-3 w-3 shrink-0 items-center justify-center rounded-sm border"
                      style={{
                        backgroundColor: isHidden ? 'transparent' : PALETTE[id % PALETTE.length],
                        borderColor: PALETTE[id % PALETTE.length],
                      }}
                    >
                      {!isHidden && <Check size={9} className="text-white" />}
                    </span>
                    <span className="truncate text-foreground">Cluster {id + 1}</span>
                    <span className="ml-auto text-micro text-muted-foreground">{count}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}

export function communityColor(cId: number): string {
  return PALETTE[cId % PALETTE.length];
}