import { PageHeader, PageContainer } from '@/components/layout';
import { useWorkspaceStore } from '@/store';
import { Sun, Moon, Monitor, Trash2, Sparkles, Database } from 'lucide-react';
import { db } from '@/lib/db';
import { seedDemoData } from '@/scripts/seedDemoData';
import { cn } from '@/lib/utils';
import { useState } from 'react';

const THEME_OPTIONS: { value: 'light' | 'dark' | 'system'; label: string; icon: typeof Sun }[] = [
  { value: 'light', label: 'Light', icon: Sun },
  { value: 'dark', label: 'Dark', icon: Moon },
  { value: 'system', label: 'System', icon: Monitor },
];

export function SettingsPage() {
  const { theme, setTheme } = useWorkspaceStore();
  const [resetting, setResetting] = useState(false);

  const handleReset = async () => {
    if (!window.confirm('Erase all local data? This cannot be undone.')) return;
    setResetting(true);
    try {
      await db.transaction('rw', [db.nodes, db.edges, db.collections, db.revisions], async () => {
        await db.nodes.clear();
        await db.edges.clear();
        await db.collections.clear();
        await db.revisions.clear();
      });
    } finally {
      setResetting(false);
    }
  };

  const handleSeed = async () => {
    if (!window.confirm('Replace current data with demo content?')) return;
    await seedDemoData();
  };

  return (
    <div className="flex h-full flex-col">
      <PageHeader title="Settings" back />

      <PageContainer>
        <Section title="Appearance">
          <Row label="Theme" description="How the interface looks on this device.">
            <div className="flex items-center gap-1 rounded-md border border-border/60 p-0.5">
              {THEME_OPTIONS.map((opt) => {
                const Icon = opt.icon;
                const active = theme === opt.value;
                return (
                  <button
                    key={opt.value}
                    onClick={() => setTheme(opt.value)}
                    className={cn(
                      'flex h-7 items-center gap-1.5 rounded px-2.5 text-[12.5px] transition-colors',
                      active
                        ? 'bg-accent text-foreground'
                        : 'text-muted-foreground/80 hover:text-foreground'
                    )}
                  >
                    <Icon className="h-3.5 w-3.5" />
                    {opt.label}
                  </button>
                );
              })}
            </div>
          </Row>
        </Section>

        <Section title="Data">
          <Row
            label="Load demo"
            description="Replace the current workspace with a curated set of pages, tasks, and edges."
          >
            <button
              onClick={handleSeed}
              className="inline-flex h-7 items-center gap-1.5 rounded-md border border-border/80 bg-background/40 px-2.5 text-[12.5px] text-foreground/80 transition-colors hover:bg-accent"
            >
              <Sparkles className="h-3.5 w-3.5" />
              Load demo
            </button>
          </Row>
          <Row
            label="Erase all data"
            description="Permanently remove every page, task, collection, and revision from this device."
          >
            <button
              onClick={handleReset}
              disabled={resetting}
              className="inline-flex h-7 items-center gap-1.5 rounded-md border border-destructive/40 px-2.5 text-[12.5px] text-destructive transition-colors hover:bg-destructive/10 disabled:opacity-50"
            >
              <Trash2 className="h-3.5 w-3.5" />
              {resetting ? 'Erasing...' : 'Erase'}
            </button>
          </Row>
        </Section>

        <Section title="About">
          <div className="space-y-1 px-4 py-3 text-[12.5px]">
            <div className="flex items-center gap-2 text-foreground/90">
              <Database className="h-3.5 w-3.5 text-muted-foreground/60" />
              Local-first. Data lives in your browser only.
            </div>
            <p className="text-muted-foreground/70">
              Knowledge Universe v0.1. A workspace where documents, tasks, projects, and concepts become a navigable graph.
            </p>
          </div>
        </Section>
      </PageContainer>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mb-8">
      <h2 className="mb-1.5 px-1 text-[11px] font-medium uppercase tracking-wider text-muted-foreground/60">
        {title}
      </h2>
      <div className="overflow-hidden rounded-lg border border-border/60">{children}</div>
    </section>
  );
}

function Row({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-border/40 px-4 py-3 last:border-b-0">
      <div className="min-w-0 flex-1">
        <div className="text-[13px] text-foreground/90">{label}</div>
        {description && (
          <div className="mt-0.5 text-[12px] text-muted-foreground/65">{description}</div>
        )}
      </div>
      <div className="flex-shrink-0">{children}</div>
    </div>
  );
}
