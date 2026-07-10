/**
 * EditorTabs — multi-tab strip above the editor. Click to activate, middle-click
 * or × button to close.
 */

import { X } from 'lucide-react';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/data/db';
import { useNavStore } from '@/store/nav';
import { cn } from '@/lib/classnames';

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
          role="button"
          tabIndex={0}
          onClick={() => setActiveTab(t.id)}
          onAuxClick={(e) => {
            if (e.button === 1) closeTab(t.id);
          }}
          className={cn(
            'group flex h-8 shrink-0 cursor-pointer items-center gap-1.5 border-r border-border px-3 text-meta',
            activeTab === t.id
              ? 'bg-background text-foreground'
              : 'bg-surface-1 text-muted-foreground hover:bg-surface-2',
          )}
        >
          <span className="truncate max-w-40">{t.title}</span>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              closeTab(t.id);
            }}
            className="rounded-sm p-0.5 text-muted-foreground opacity-0 hover:bg-surface-3 hover:text-foreground group-hover:opacity-100"
            aria-label="Close tab"
          >
            <X size={12} />
          </button>
        </div>
      ))}
    </div>
  );
}