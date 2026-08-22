/**
 * Settings modal — theme, daily-note format, graph node limit. Persisted to
 * the `settings` table; theme also flips the document attribute live.
 */

import { useEffect, useRef, useState } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Download, FolderOpen, RefreshCw, Repeat2, RotateCcw, ShieldCheck, Trash2, Upload } from 'lucide-react';
import { db } from '@/infrastructure/database/db';
import { useUIStore, type Theme } from '@/shared/state/ui';
import { exportVault } from '@/core/vault/export';
import { importVaultFromFile } from '@/core/vault/import';
import { Kbd } from '@/shared/components/Kbd';
import { cn } from '@/shared/lib/classnames';
import {
  getActiveVaultKind,
  listActiveVaultTrash,
  type VaultFileSnapshot,
} from '@/infrastructure/vault/filesystem-vault';
import { listVaultTemplates } from '@/core/vault/templates';
import { format as formatDate } from 'date-fns';
import {
  listDeletedPages,
  permanentlyDeletePage,
  restorePage,
  restoreTrashedFilesystemPage,
} from '@/core/vault/pages';
import { useNavStore } from '@/shared/state/nav';
import {
  browserStoragePersistenceStatus,
  requestBrowserStoragePersistence,
  type BrowserStoragePersistence,
} from '@/infrastructure/database/browser-local-vault';

export function SettingsModal({
  open,
  vaultMode,
  vaultName,
  watcherStatus,
  onRefreshVault,
  onSwitchVault,
  onClose,
  onOpenNote,
}: {
  open: boolean;
  vaultMode: 'desktop' | 'browser-folder' | 'browser-local';
  vaultName: string;
  watcherStatus: 'inactive' | 'starting' | 'watching' | 'error';
  onRefreshVault: () => Promise<{ noteCount: number } | null>;
  onSwitchVault: () => Promise<void>;
  onOpenNote: (id: string) => void;
  onClose: () => void;
}) {
  const openPage = useNavStore((s) => s.openPage);
  const theme = useUIStore((s) => s.theme);
  const setTheme = useUIStore((s) => s.setTheme);
  const [importing, setImporting] = useState(false);
  const [transferStatus, setTransferStatus] = useState<string | null>(null);
  const [dailyFormatError, setDailyFormatError] = useState<string | null>(null);
  const [trashStatus, setTrashStatus] = useState<string | null>(null);
  const [eraseArmed, setEraseArmed] = useState<string | null>(null);
  const [vaultActionStatus, setVaultActionStatus] = useState<string | null>(null);
  const [vaultActionBusy, setVaultActionBusy] = useState(false);
  const [filesystemTrash, setFilesystemTrash] = useState<VaultFileSnapshot[] | null>(null);
  const [trashRequestId, setTrashRequestId] = useState(0);
  const [projectedAt, setProjectedAt] = useState(() => Date.now());
  const [storagePersistence, setStoragePersistence] = useState<BrowserStoragePersistence>('unavailable');
  const eraseTimer = useRef<number | null>(null);
  const filesystemVault = vaultMode !== 'browser-local';

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
  const deletedPages = useLiveQuery(listDeletedPages, [], []);

  useEffect(() => () => {
    if (eraseTimer.current !== null) window.clearTimeout(eraseTimer.current);
  }, []);

  useEffect(() => {
    if (open && vaultMode === 'browser-local') {
      void browserStoragePersistenceStatus().then(setStoragePersistence);
    }
  }, [open, vaultMode]);

  useEffect(() => {
    if (!open || !filesystemVault) return;
    const refresh = () => setProjectedAt(Date.now());
    window.addEventListener('ley:vault-projected', refresh);
    return () => window.removeEventListener('ley:vault-projected', refresh);
  }, [filesystemVault, open]);

  useEffect(() => {
    if (!open || !filesystemVault) return;
    let current = true;
    void listActiveVaultTrash()
      .then((files) => { if (current) setFilesystemTrash(files); })
      .catch((error) => { if (current) setTrashStatus(error instanceof Error ? error.message : String(error)); });
    return () => { current = false; };
  }, [filesystemVault, open, projectedAt, trashRequestId, vaultMode]);

  function reloadFilesystemTrash() {
    setTrashRequestId((current) => current + 1);
  }

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

  async function handleRestore(pageId: string) {
    try {
      const page = await restorePage(pageId);
      setTrashStatus(`Restored “${page.title}”.`);
    } catch (cause) {
      setTrashStatus(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleErase(pageId: string) {
    if (eraseArmed !== pageId) {
      setEraseArmed(pageId);
      if (eraseTimer.current !== null) window.clearTimeout(eraseTimer.current);
      eraseTimer.current = window.setTimeout(() => setEraseArmed((current) => current === pageId ? null : current), 4000);
      return;
    }
    try {
      await permanentlyDeletePage(pageId);
      setEraseArmed(null);
      setTrashStatus('The note was permanently deleted.');
    } catch (cause) {
      setTrashStatus(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleRestoreTrashed(path: string) {
    try {
      const restored = await restoreTrashedFilesystemPage(path);
      const files = await listActiveVaultTrash();
      setFilesystemTrash(files);
      reloadFilesystemTrash();
      if (filesystemVault) await onRefreshVault();
      openPage(restored.id);
      onOpenNote(restored.id);
      setTrashStatus(`Restored “${restored.title}” to ${restored.path}.`);
    } catch (cause) {
      setTrashStatus(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleRefreshVault() {
    setVaultActionBusy(true);
    setVaultActionStatus('Refreshing vault…');
    try {
      const refreshed = await onRefreshVault();
      setVaultActionStatus(refreshed ? `Up to date · ${refreshed.noteCount} ${refreshed.noteCount === 1 ? 'note' : 'notes'}.` : 'Vault refresh is unavailable.');
    } catch (cause) {
      setVaultActionStatus(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setVaultActionBusy(false);
    }
  }

  return (
    <Dialog.Root open={open} onOpenChange={(next) => { if (!next) onClose(); }}>
      <Dialog.Portal>
        <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-50" />
        <Dialog.Content aria-describedby={undefined} className="app-modal-surface fixed left-1/2 top-1/2 z-[51] flex max-h-[min(760px,92vh)] w-[520px] max-w-[92vw] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-xl border outline-none">
        <div className="flex items-center justify-between border-b border-border px-4 py-3">
          <Dialog.Title className="text-body font-semibold">Settings</Dialog.Title>
          <Dialog.Close
            aria-label="Close settings"
            className="rounded-sm p-1 text-muted-foreground hover:bg-surface-3 hover:text-foreground"
          >
            <X size={14} />
          </Dialog.Close>
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
            <div className="mb-2 flex items-center justify-between gap-3"><span className="text-meta font-medium text-foreground">Vault</span><span className="max-w-56 truncate font-mono text-micro text-muted-foreground">{vaultName}</span></div>
            {filesystemVault ? (
              <div className="rounded-lg border border-border bg-surface-2 p-3 text-meta text-muted-foreground">
                <div className="flex items-center gap-2 font-medium text-foreground"><FolderOpen size={14} className="text-secondary" />Already portable</div>
                <p className="mt-1 leading-relaxed">This vault is an ordinary folder. Back it up, sync it, zip it, or commit it with your normal filesystem tools.</p>
                <div className="mt-2 flex items-center gap-1.5 text-micro text-muted-foreground"><span className={`size-1.5 rounded-full ${vaultMode === 'desktop' && watcherStatus === 'watching' ? 'bg-emerald-400' : watcherStatus === 'error' ? 'bg-destructive' : 'bg-muted-foreground'}`} />{vaultMode === 'desktop' ? watcherStatus === 'watching' ? 'Watching external changes live' : watcherStatus === 'error' ? 'Live watcher unavailable · focus and manual refresh remain active' : 'Starting live filesystem watcher…' : 'Refreshes after returning to Ley or on request'}</div>
                <button type="button" disabled={vaultActionBusy} onClick={() => void handleRefreshVault()} className="mt-3 flex items-center gap-1.5 rounded-md border border-border bg-background px-2.5 py-1.5 text-meta text-foreground hover:bg-surface-3 disabled:opacity-50"><RefreshCw size={13} className={vaultActionBusy ? 'animate-spin' : ''} />Refresh from folder</button>
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
            <div className="mt-3 rounded-lg border border-border bg-surface-2 p-3 text-meta text-muted-foreground">
              <div className="flex items-center gap-2 font-medium text-foreground"><ShieldCheck size={14} className="text-secondary" />On-device browser storage</div>
              <p className="mt-1 leading-relaxed">Ley stores this vault on this device. {storagePersistence === 'persistent' ? 'This browser has protected Ley from automatic storage cleanup.' : 'Your browser may clear this vault under storage pressure or when site data is removed, so keep ZIP backups.'}</p>
              {storagePersistence === 'best-effort' && <button type="button" onClick={() => void requestBrowserStoragePersistence().then(setStoragePersistence)} className="mt-3 rounded-md border border-border bg-background px-2.5 py-1.5 text-meta text-foreground hover:bg-surface-3">Protect local storage</button>}
            </div>
            {transferStatus && <div className="mt-1 text-meta text-secondary" role="status">{transferStatus}</div>}
            </>}
            <button type="button" onClick={() => { onClose(); void onSwitchVault(); }} className="mt-3 flex w-full items-center justify-center gap-1.5 rounded-md border border-border px-3 py-2 text-meta text-muted-foreground-strong hover:bg-surface-2 hover:text-foreground"><Repeat2 size={13} />{vaultMode === 'desktop' ? 'Open another folder' : 'Change vault or storage'}</button>
            {vaultActionStatus && <button type="button" onClick={() => setVaultActionStatus(null)} className="mt-2 text-left text-micro text-secondary" role="status">{vaultActionStatus}</button>}
          </section>

          {!filesystemVault && <section>
            <div className="mb-2 flex items-center justify-between gap-3">
              <div className="text-meta font-medium text-foreground">Recycle bin</div>
              <span className="text-micro text-muted-foreground">{deletedPages.length} {deletedPages.length === 1 ? 'note' : 'notes'}</span>
            </div>
            {deletedPages.length === 0 ? (
              <div className="rounded-lg border border-dashed border-border px-3 py-4 text-center text-meta text-muted-foreground">Deleted browser-local notes can be restored here.</div>
            ) : (
              <div className="max-h-44 divide-y divide-border overflow-y-auto rounded-lg border border-border bg-surface-2">
                {deletedPages.map((page) => <div key={page.id} className="flex items-center gap-2 px-3 py-2">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-meta font-medium text-foreground">{page.title}</div>
                    <div className="truncate font-mono text-micro text-muted-foreground">{page.path}</div>
                  </div>
                  <button type="button" onClick={() => void handleRestore(page.id)} className="rounded-md border border-border p-1.5 text-muted-foreground hover:bg-surface-3 hover:text-foreground" aria-label={`Restore ${page.title}`} title="Restore"><RotateCcw size={13} /></button>
                  <button type="button" onClick={() => void handleErase(page.id)} className={cn('flex items-center gap-1 rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive', eraseArmed === page.id && 'bg-destructive text-white hover:bg-destructive hover:text-white')} aria-label={`Permanently delete ${page.title}`} title={eraseArmed === page.id ? 'Click again to permanently delete' : 'Permanently delete'}><Trash2 size={13} />{eraseArmed === page.id && <span className="text-micro">Confirm</span>}</button>
                </div>)}
              </div>
            )}
            {trashStatus && <button type="button" onClick={() => setTrashStatus(null)} className="mt-2 text-left text-micro text-secondary" role="status">{trashStatus}</button>}
          </section>}

          {filesystemVault && <section>
            <div className="mb-2 flex items-center justify-between gap-3">
              <div className="text-meta font-medium text-foreground">Folder trash</div>
              <span className="text-micro text-muted-foreground">{filesystemTrash?.length ?? 0} {filesystemTrash?.length === 1 ? 'note' : 'notes'}</span>
            </div>
            {filesystemTrash === null ? (
              <div className="rounded-lg border border-dashed border-border px-3 py-4 text-center text-meta text-muted-foreground">Checking .trash…</div>
            ) : filesystemTrash.length === 0 ? (
              <div className="rounded-lg border border-dashed border-border px-3 py-4 text-center text-meta text-muted-foreground">Trashed folder notes can be restored here.</div>
            ) : (
              <div className="max-h-44 divide-y divide-border overflow-y-auto rounded-lg border border-border bg-surface-2">
                {filesystemTrash.map((file) => <div key={file.path} className="flex items-center gap-2 px-3 py-2">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-meta font-medium text-foreground">{file.path.split('/').at(-1)?.replace(/\.md$/i, '')}</div>
                    <div className="truncate font-mono text-micro text-muted-foreground">{file.path}</div>
                  </div>
                  <button type="button" onClick={() => void handleRestoreTrashed(file.path)} className="rounded-md border border-border p-1.5 text-muted-foreground hover:bg-surface-3 hover:text-foreground" aria-label={`Restore ${file.path}`} title="Restore"><RotateCcw size={13} /></button>
                </div>)}
              </div>
            )}
          </section>}
        </div>

        <div className="flex items-center justify-between border-t border-border px-4 py-2 text-micro text-muted-foreground">
          <span>Local-first — {getActiveVaultKind() ? 'Markdown stays in your chosen folder.' : 'notes stay on this device.'}</span>
          <Kbd>esc</Kbd>
        </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
