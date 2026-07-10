/**
 * Settings modal — theme, daily-note format, graph node limit. Persisted to
 * the `settings` table; theme also flips the document attribute live.
 */

import { useEffect, useState } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { X, Download, Upload, Sparkles } from 'lucide-react';
import { db } from '@/data/db';
import { useUIStore, type Theme } from '@/store/ui';
import { exportVault } from '@/core/vault/export';
import { importVaultFromFile } from '@/core/vault/import';
import { seedDemoContent } from '@/data/demo-content';
import { Kbd } from '@/ui/Kbd';
import { cn } from '@/lib/classnames';

export function SettingsModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const theme = useUIStore((s) => s.theme);
  const setTheme = useUIStore((s) => s.setTheme);
  const [importing, setImporting] = useState(false);
  const [demoStatus, setDemoStatus] = useState<string | null>(null);

  const dailyFormat = useLiveQuery(
    async () => (await db.settings.get('daily-note-format'))?.value as string | undefined,
    [],
  );

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  if (!open) return null;

  async function saveFormat(value: string) {
    await db.settings.put({ key: 'daily-note-format', value });
  }

  async function handleExport() {
    const blob = await exportVault();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `ley-vault-${new Date().toISOString().slice(0, 10)}.zip`;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function handleImport(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setImporting(true);
    try {
      const count = await importVaultFromFile(file);
      alert(`Imported ${count} pages.`);
    } catch (err) {
      alert(`Import failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setImporting(false);
      e.target.value = '';
    }
  }

  return (
    <div
      role="dialog"
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ backgroundColor: 'hsl(var(--background) / 0.6)' }}
      onClick={onClose}
    >
      <div
        className="flex w-[480px] max-w-[92vw] flex-col overflow-hidden rounded-lg border border-border bg-surface-1"
        style={{ boxShadow: 'var(--shadow-menu)' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-border px-4 py-3">
          <span className="text-body font-semibold">Settings</span>
          <button
            type="button"
            onClick={onClose}
            className="rounded-sm p-1 text-muted-foreground hover:bg-surface-3 hover:text-foreground"
          >
            <X size={14} />
          </button>
        </div>

        <div className="flex flex-col gap-6 p-4">
          <section>
            <div className="mb-2 text-meta font-medium text-foreground">Appearance</div>
            <div className="flex gap-2">
              {(['dark', 'light'] as Theme[]).map((t) => (
                <button
                  key={t}
                  type="button"
                  onClick={() => {
                    setTheme(t);
                    db.settings.put({ key: 'theme', value: t });
                  }}
                  className={cn(
                    'flex-1 rounded-md border px-3 py-2 text-left text-meta',
                    theme === t
                      ? 'border-primary bg-primary/10 text-foreground'
                      : 'border-border bg-surface-2 text-muted-foreground-strong hover:bg-surface-3',
                  )}
                >
                  <div className="font-medium capitalize">{t}</div>
                  <div className="mt-0.5 text-micro text-muted-foreground">
                    {t === 'dark' ? 'Easier on the eyes at night' : 'Bright, high contrast'}
                  </div>
                </button>
              ))}
            </div>
          </section>

          <section>
            <div className="mb-2 text-meta font-medium text-foreground">Daily notes</div>
            <label className="flex flex-col gap-1 text-meta text-muted-foreground-strong">
              File name format
              <input
                type="text"
                defaultValue={dailyFormat ?? 'yyyy-MM-dd'}
                key={dailyFormat ?? 'default'}
                onBlur={(e) => saveFormat(e.target.value)}
                className="h-8 rounded-md border border-border bg-surface-1 px-2 text-meta text-foreground focus:border-primary focus:outline-none"
                placeholder="yyyy-MM-dd"
              />
              <span className="text-micro text-muted-foreground">
                date-fns format tokens: yyyy-MM-dd, yyyy/MM/dd, etc.
              </span>
            </label>
          </section>

          <section>
            <div className="mb-2 text-meta font-medium text-foreground">Vault</div>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={handleExport}
                className="flex flex-1 items-center justify-center gap-1.5 rounded-md border border-border bg-surface-2 px-3 py-2 text-meta text-foreground hover:bg-surface-3"
              >
                <Download size={13} />
                Export to ZIP
              </button>
              <label
                className={cn(
                  'flex flex-1 cursor-pointer items-center justify-center gap-1.5 rounded-md border border-border bg-surface-2 px-3 py-2 text-meta text-foreground hover:bg-surface-3',
                  importing && 'pointer-events-none opacity-50',
                )}
              >
                <Upload size={13} />
                {importing ? 'Importing…' : 'Import ZIP'}
                <input
                  type="file"
                  accept=".zip"
                  className="hidden"
                  onChange={handleImport}
                />
              </label>
            </div>
            <div className="mt-1 text-micro text-muted-foreground">
              Exports use Obsidian-compatible folders (.md files). Imports accept any Obsidian vault ZIP.
            </div>
          </section>

          <section>
            <div className="mb-2 text-meta font-medium text-foreground">Demo content</div>
            <button
              type="button"
              onClick={async () => {
                setDemoStatus('Adding…');
                try {
                  const added = await seedDemoContent();
                  setDemoStatus(added > 0 ? `Added ${added} pages.` : 'All demo pages already present.');
                } catch (err) {
                  setDemoStatus(`Failed: ${err instanceof Error ? err.message : String(err)}`);
                }
              }}
              className="flex w-full items-center justify-center gap-1.5 rounded-md border border-border bg-surface-2 px-3 py-2 text-meta text-foreground hover:bg-surface-3"
            >
              <Sparkles size={13} />
              Add demo vault
            </button>
            <div className="mt-1 text-micro text-muted-foreground">
              Adds ~25 sample notes (productivity, engineering, learning, daily notes) for trying out the
              graph. Idempotent — skips pages that already exist.
            </div>
            {demoStatus && (
              <div className="mt-1 text-meta text-secondary">{demoStatus}</div>
            )}
          </section>
        </div>

        <div className="flex items-center justify-between border-t border-border px-4 py-2 text-micro text-muted-foreground">
          <span>Local-first — your data never leaves this browser.</span>
          <Kbd>esc</Kbd>
        </div>
      </div>
    </div>
  );
}