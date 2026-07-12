/**
 * Settings modal — theme, daily-note format, graph node limit. Persisted to
 * the `settings` table; theme also flips the document attribute live.
 */

import { useEffect, useState } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import { X, Download, FolderOpen, Upload } from 'lucide-react';
import { db } from '@/infrastructure/database/db';
import { useUIStore, type Theme } from '@/shared/state/ui';
import { exportVault } from '@/core/vault/export';
import { importVaultFromFile } from '@/core/vault/import';
import { Kbd } from '@/shared/components/Kbd';
import { cn } from '@/shared/lib/classnames';
import { getActiveVaultKind } from '@/infrastructure/vault/filesystem-vault';
import { listVaultTemplates } from '@/core/vault/templates';
import { format as formatDate } from 'date-fns';

export function SettingsModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const theme = useUIStore((s) => s.theme);
  const setTheme = useUIStore((s) => s.setTheme);
  const [importing, setImporting] = useState(false);
  const [transferStatus, setTransferStatus] = useState<string | null>(null);
  const [dailyFormatError, setDailyFormatError] = useState<string | null>(null);
  const filesystemVault = Boolean(getActiveVaultKind());

  const dailyFormat = useLiveQuery(
    async () => (await db.settings.get('daily-note-format'))?.value as string | undefined,
    [],
  );
  const templateFolder = useLiveQuery(
    async () => (await db.settings.get('template-folder'))?.value as string | undefined,
    [],
  );
  const vaultTemplates = useLiveQuery(listVaultTemplates, [], []);
  const dailyTemplatePath = useLiveQuery(
    async () => (await db.settings.get('daily-note-template-path'))?.value as string | undefined,
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
    try {
      formatDate(new Date(), value);
      await db.settings.put({ key: 'daily-note-format', value });
      setDailyFormatError(null);
    } catch {
      setDailyFormatError('That date format is not valid. Try yyyy-MM-dd.');
    }
  }

  async function handleExport() {
    setTransferStatus('Preparing archive…');
    try {
      const blob = await exportVault();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `ley-vault-${new Date().toISOString().slice(0, 10)}.zip`;
      a.click();
      URL.revokeObjectURL(url);
      setTransferStatus('Export ready.');
    } catch (error) {
      setTransferStatus(`Export failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async function handleImport(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setImporting(true);
    try {
      const count = await importVaultFromFile(file);
      setTransferStatus(`Imported ${count} pages.`);
    } catch (err) {
      setTransferStatus(`Import failed: ${err instanceof Error ? err.message : String(err)}`);
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
        className="flex max-h-[min(760px,92vh)] w-[520px] max-w-[92vw] flex-col overflow-hidden rounded-xl border border-border bg-surface-1"
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

        <div className="flex flex-col gap-6 overflow-y-auto p-4">
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
            <div className="mb-2 text-meta font-medium text-foreground">Templates</div>
            <label className="flex flex-col gap-1 text-meta text-muted-foreground-strong">
              Template folder
              <input
                type="text"
                defaultValue={templateFolder ?? 'templates'}
                key={templateFolder ?? 'templates'}
                onBlur={(event) => void db.settings.put({ key: 'template-folder', value: event.target.value.trim() || 'templates' })}
                className="h-8 rounded-md border border-border bg-surface-1 px-2 font-mono text-meta text-foreground focus:border-primary focus:outline-none"
                placeholder="templates"
              />
              <span className="text-micro text-muted-foreground">Markdown files in this vault folder appear in the new-note dialog.</span>
            </label>
          </section>

          <section>
            <div className="mb-2 text-meta font-medium text-foreground">Daily notes</div>
            <label className="flex flex-col gap-1 text-meta text-muted-foreground-strong">
              File name format
              <input
                type="text"
                defaultValue={dailyFormat ?? 'yyyy-MM-dd'}
                key={dailyFormat ?? 'default'}
                onBlur={(e) => void saveFormat(e.target.value)}
                className="h-8 rounded-md border border-border bg-surface-1 px-2 text-meta text-foreground focus:border-primary focus:outline-none"
                placeholder="yyyy-MM-dd"
              />
              <span className="text-micro text-muted-foreground">
                date-fns format tokens, for example yyyy-MM-dd or EEEE, MMMM do.
              </span>
              {dailyFormatError && <span className="text-micro text-destructive" role="alert">{dailyFormatError}</span>}
            </label>
            <label className="mt-3 flex flex-col gap-1 text-meta text-muted-foreground-strong">
              Daily note template
              <select value={dailyTemplatePath ?? ''} onChange={(event) => void db.settings.put({ key: 'daily-note-template-path', value: event.target.value })} className="h-8 rounded-md border border-border bg-surface-1 px-2 text-meta text-foreground focus:border-primary focus:outline-none">
                <option value="">Built-in daily note</option>
                {vaultTemplates.map((template) => <option key={template.id} value={template.path}>{template.title}</option>)}
              </select>
            </label>
          </section>

          <section>
            <div className="mb-2 text-meta font-medium text-foreground">Vault</div>
            {filesystemVault ? (
              <div className="rounded-lg border border-border bg-surface-2 p-3 text-meta text-muted-foreground">
                <div className="flex items-center gap-2 font-medium text-foreground"><FolderOpen size={14} className="text-secondary" />Already portable</div>
                <p className="mt-1 leading-relaxed">This vault is an ordinary folder. Back it up, sync it, zip it, or commit it with your normal filesystem tools.</p>
              </div>
            ) : <><div className="flex gap-2">
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
            {transferStatus && <div className="mt-1 text-meta text-secondary" role="status">{transferStatus}</div>}
            </>}
          </section>
        </div>

        <div className="flex items-center justify-between border-t border-border px-4 py-2 text-micro text-muted-foreground">
          <span>Local-first — {getActiveVaultKind() ? 'Markdown stays in your chosen folder.' : 'notes stay on this device.'}</span>
          <Kbd>esc</Kbd>
        </div>
      </div>
    </div>
  );
}
