/**
 * EditorTabs — multi-tab strip above the editor. Click to activate, middle-click
 * or × button to close.
 */

import { X } from 'lucide-react';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';
import { useNavStore } from '@/shared/state/nav';
import { cn } from '@/shared/lib/classnames';

export function EditorTabs() {
  const openTabs = useNavStore((s) => s.openTabs);
  const activeTab = useNavStore((s) => s.activeTab);
  const setActiveTab = useNavStore((s) => s.setActiveTab);
  const closeTab = useNavStore((s) => s.closeTab);

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
    <div className="flex shrink-0 items-stretch overflow-x-auto border-b border-border bg-surface-1">
      {tabs.map((t) => (
        <div
          key={t.id}
          className={cn(
            'group flex h-8 shrink-0 items-center border-r border-border text-meta',
            activeTab === t.id
              ? 'bg-background text-foreground'
              : 'bg-surface-1 text-muted-foreground hover:bg-surface-2',
          )}
        >
          <button type="button" onClick={() => setActiveTab(t.id)} onAuxClick={(event) => { if (event.button === 1) closeTab(t.id); }} className="h-full max-w-44 truncate pl-3 pr-1 text-left">
            {t.title}
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              closeTab(t.id);
            }}
            className="mr-2 rounded-sm p-0.5 text-muted-foreground opacity-0 hover:bg-surface-3 hover:text-foreground focus:opacity-100 group-hover:opacity-100"
            aria-label="Close tab"
          >
            <X size={12} />
          </button>
        </div>
      ))}
    </div>
  );
}
