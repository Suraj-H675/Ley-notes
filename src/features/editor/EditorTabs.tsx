/**
 * EditorTabs — multi-tab strip above the editor. Click to activate, middle-click
 * or × button to close.
 */

import { Columns2, X } from 'lucide-react';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';
import { useNavStore } from '@/shared/state/nav';
import { cn } from '@/shared/lib/classnames';

export function EditorTabs() {
  const openTabs = useNavStore((s) => s.openTabs);
  const activeTab = useNavStore((s) => s.activeTab);
  const primaryTab = useNavStore((s) => s.primaryTab);
  const secondaryTab = useNavStore((s) => s.secondaryTab);
  const setActiveTab = useNavStore((s) => s.setActiveTab);
  const closeTab = useNavStore((s) => s.closeTab);
  const openInSplit = useNavStore((s) => s.openInSplit);

  const tabs = useLiveQuery(
    async () => {
      if (openTabs.length === 0) return [];
      const rows = await db.pages.where('id').anyOf(openTabs).toArray();
      const byId = new Map<string, (typeof rows)[number]>();
      for (const p of rows) byId.set(p.id, p);
      const result: NonNullable<(typeof rows)[number]>[] = [];
      for (const id of openTabs) {
        const p = byId.get(id);
        if (p && p.deletedAt === null) result.push(p);
      }
      return result;
    },
    [openTabs],
  );

  if (!tabs || tabs.length === 0) return null;

  return (
    <div className="flex shrink-0 items-stretch overflow-x-auto border-b border-border bg-[#141416]">
      {tabs.map((t) => (
        <div
          key={t.id}
          className={cn(
            'group flex h-8 shrink-0 items-center border-r border-border text-meta',
            activeTab === t.id
              ? 'bg-[#1a1a1c] text-foreground shadow-[inset_0_-2px_0_hsl(41_34%_66%)]'
              : t.id === primaryTab || t.id === secondaryTab
                ? 'bg-surface-2 text-foreground'
              : 'bg-transparent text-muted-foreground hover:bg-surface-2',
          )}
        >
          <button type="button" onClick={() => setActiveTab(t.id)} onAuxClick={(event) => { if (event.button === 1 && !t.missingFromDisk) closeTab(t.id); }} className="h-full max-w-44 truncate pl-3 pr-1 text-left tracking-tight">
            {t.title}{t.missingFromDisk ? ' · missing' : ''}
          </button>
          {t.id !== primaryTab && (
            <button
              type="button"
              onClick={(event) => { event.stopPropagation(); openInSplit(t.id); }}
              className="rounded-sm p-0.5 text-muted-foreground opacity-0 hover:bg-surface-3 hover:text-foreground focus:opacity-100 group-hover:opacity-100"
              aria-label={`Open ${t.title} in split`}
              title="Open in split"
            >
              <Columns2 size={12} />
            </button>
          )}
          {!t.missingFromDisk && <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              closeTab(t.id);
            }}
            className="mr-2 rounded-sm p-0.5 text-muted-foreground opacity-0 hover:bg-surface-3 hover:text-foreground focus:opacity-100 group-hover:opacity-100"
            aria-label="Close tab"
          >
            <X size={12} />
          </button>}
        </div>
      ))}
    </div>
  );
}
