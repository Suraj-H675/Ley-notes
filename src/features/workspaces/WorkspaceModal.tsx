import { useState } from 'react';
import { useLiveQuery } from 'dexie-react-hooks';
import * as Dialog from '@radix-ui/react-dialog';
import {
  Check,
  Columns2,
  PanelLeft,
  PanelRight,
  PanelsTopLeft,
  Pencil,
  RefreshCw,
  Save,
  Trash2,
  X,
} from 'lucide-react';
import { applyNavigationLayout, captureNavigationLayout } from '@/core/vault/navigation-session';
import {
  deleteWorkspaceLayout,
  parseWorkspaceLayoutsSetting,
  renameWorkspaceLayout,
  replaceWorkspaceLayout,
  saveWorkspaceLayout,
  workspaceLayoutsDataKey,
  type NamedWorkspace,
  type WorkspaceShellLayout,
} from '@/core/vault/workspace-layouts';
import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';
import { Button } from '@/shared/components/Button';
import { useUIStore } from '@/shared/state/ui';

interface WorkspaceModalProps {
  open: boolean;
  splitPercent: number;
  onSplitPercentChange: (value: number) => void;
  onClose: () => void;
}

export function WorkspaceModal({ open, splitPercent, onSplitPercentChange, onClose }: WorkspaceModalProps) {
  const workspaces = useLiveQuery(async () => {
    const key = workspaceLayoutsDataKey(await activeDataKind());
    return parseWorkspaceLayoutsSetting((await db.settings.get(key))?.value);
  }, []) ?? [];
  const [name, setName] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState('');
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  function currentShell(): WorkspaceShellLayout {
    const ui = useUIStore.getState();
    return {
      sidebarOpen: ui.sidebarOpen,
      rightDockOpen: ui.rightDockOpen,
      rightDockTab: ui.rightDockTab,
      splitPercent,
    };
  }

  async function captureCurrent() {
    const navigation = await captureNavigationLayout();
    if (navigation.openTabs.length === 0) throw new Error('Open at least one note before saving a workspace.');
    return { navigation, shell: currentShell() };
  }

  async function saveCurrent() {
    setBusyAction('save');
    setError(null);
    try {
      const snapshot = await captureCurrent();
      const saved = await saveWorkspaceLayout(name, snapshot.navigation, snapshot.shell);
      setName('');
      setStatus(`${saved.name} saved`);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function load(workspace: NamedWorkspace) {
    setBusyAction(`load:${workspace.id}`);
    setError(null);
    try {
      const applied = await applyNavigationLayout(workspace.navigation);
      if (!applied) throw new Error('None of this workspace’s notes are available in the current vault.');
      const ui = useUIStore.getState();
      ui.setSidebarOpen(workspace.shell.sidebarOpen);
      ui.setRightDockOpen(workspace.shell.rightDockOpen);
      ui.setRightDockTab(workspace.shell.rightDockTab);
      onSplitPercentChange(workspace.shell.splitPercent);
      setStatus(`${workspace.name} loaded`);
      onClose();
    } catch (cause) {
      setError(message(cause));
      setBusyAction(null);
    }
  }

  async function replace(workspace: NamedWorkspace) {
    setBusyAction(`replace:${workspace.id}`);
    setError(null);
    try {
      const snapshot = await captureCurrent();
      await replaceWorkspaceLayout(workspace.id, snapshot.navigation, snapshot.shell);
      setStatus(`${workspace.name} updated to the current layout`);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function commitRename(workspace: NamedWorkspace) {
    setBusyAction(`rename:${workspace.id}`);
    setError(null);
    try {
      await renameWorkspaceLayout(workspace.id, renameDraft);
      setEditingId(null);
      setStatus('Workspace renamed');
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function remove(workspace: NamedWorkspace) {
    if (confirmDeleteId !== workspace.id) {
      setConfirmDeleteId(workspace.id);
      return;
    }
    setBusyAction(`delete:${workspace.id}`);
    setError(null);
    try {
      await deleteWorkspaceLayout(workspace.id);
      setConfirmDeleteId(null);
      setStatus(`${workspace.name} deleted`);
    } catch (cause) {
      setError(message(cause));
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <Dialog.Root open={open} onOpenChange={(next) => { if (!next) onClose(); }}>
      <Dialog.Portal>
        <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-[80]" />
        <Dialog.Content aria-describedby="workspace-description" className="app-modal-surface fixed left-1/2 top-1/2 z-[81] flex max-h-[calc(100vh-24px)] w-[min(720px,calc(100vw-24px))] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-xl border outline-none">
          <header className="flex shrink-0 items-start gap-3 border-b border-border px-4 py-4 sm:px-5">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-secondary/20 bg-secondary/10 text-secondary">
              <PanelsTopLeft size={19} />
            </div>
            <div className="min-w-0 flex-1">
              <Dialog.Title className="text-body font-semibold text-foreground">Workspace layouts</Dialog.Title>
              <Dialog.Description id="workspace-description" className="mt-0.5 text-meta leading-relaxed text-muted-foreground">
                Save named arrangements for writing, research, planning, or review.
              </Dialog.Description>
            </div>
            <Dialog.Close className="rounded-md p-2 text-muted-foreground hover:bg-surface-3 hover:text-foreground" aria-label="Close workspace layouts"><X size={15} /></Dialog.Close>
          </header>

          <form className="flex shrink-0 flex-col gap-2 border-b border-border bg-background/35 px-4 py-3 sm:flex-row sm:px-5" onSubmit={(event) => { event.preventDefault(); void saveCurrent(); }}>
            <label className="sr-only" htmlFor="workspace-name">Workspace name</label>
            <input id="workspace-name" value={name} onChange={(event) => setName(event.target.value)} placeholder="Name this layout, e.g. Deep research" maxLength={80} className="h-9 min-w-0 flex-1 rounded-md border border-border bg-background px-3 text-body text-foreground outline-none placeholder:text-subtle-foreground focus:border-primary" />
            <Button type="submit" variant="primary" className="h-9 shrink-0" disabled={busyAction !== null || !name.trim()}><Save size={13} />{busyAction === 'save' ? 'Saving…' : 'Save current layout'}</Button>
          </form>

          <div className="min-h-0 flex-1 overflow-y-auto p-3 sm:p-4">
            {workspaces.length === 0 ? (
              <div className="mx-auto flex max-w-sm flex-col items-center px-4 py-10 text-center">
                <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-2xl border border-border bg-surface-2 text-subtle-foreground"><PanelsTopLeft size={22} /></div>
                <h2 className="text-body font-medium text-foreground">No saved layouts yet</h2>
                <p className="mt-1 text-meta leading-relaxed text-muted-foreground">Arrange your tabs, split panes, and sidebars, then save the view above.</p>
              </div>
            ) : (
              <div className="space-y-2">
                {workspaces.map((workspace) => (
                  <WorkspaceRow
                    key={workspace.id}
                    workspace={workspace}
                    editing={editingId === workspace.id}
                    renameDraft={renameDraft}
                    confirmDelete={confirmDeleteId === workspace.id}
                    busy={busyAction?.endsWith(workspace.id) ?? false}
                    onRenameDraft={setRenameDraft}
                    onLoad={() => void load(workspace)}
                    onReplace={() => void replace(workspace)}
                    onStartRename={() => { setEditingId(workspace.id); setRenameDraft(workspace.name); setConfirmDeleteId(null); }}
                    onCancelRename={() => setEditingId(null)}
                    onCommitRename={() => void commitRename(workspace)}
                    onDelete={() => void remove(workspace)}
                  />
                ))}
              </div>
            )}
          </div>

          <footer className="flex min-h-10 shrink-0 items-center justify-between gap-3 border-t border-border px-4 py-2 text-micro sm:px-5">
            <span className={error ? 'text-destructive' : 'text-muted-foreground'} role={error ? 'alert' : 'status'}>{error ?? status ?? `${workspaces.length} saved ${workspaces.length === 1 ? 'layout' : 'layouts'} in this vault`}</span>
            <span className="hidden shrink-0 text-subtle-foreground sm:inline">Notes are matched by ID, then path</span>
          </footer>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

interface WorkspaceRowProps {
  workspace: NamedWorkspace;
  editing: boolean;
  renameDraft: string;
  confirmDelete: boolean;
  busy: boolean;
  onRenameDraft: (value: string) => void;
  onLoad: () => void;
  onReplace: () => void;
  onStartRename: () => void;
  onCancelRename: () => void;
  onCommitRename: () => void;
  onDelete: () => void;
}

function WorkspaceRow({ workspace, editing, renameDraft, confirmDelete, busy, onRenameDraft, onLoad, onReplace, onStartRename, onCancelRename, onCommitRename, onDelete }: WorkspaceRowProps) {
  const tabs = workspace.navigation.openTabs.length;
  const split = Boolean(workspace.navigation.secondaryTab);
  return (
    <article className="group rounded-lg border border-border bg-background/55 p-3 transition-colors hover:border-secondary/25 hover:bg-background">
      <div className="flex min-w-0 items-start gap-3">
        <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-surface-2 text-secondary">{split ? <Columns2 size={15} /> : <PanelsTopLeft size={15} />}</div>
        <div className="min-w-0 flex-1">
          {editing ? (
            <form className="flex gap-1.5" onSubmit={(event) => { event.preventDefault(); onCommitRename(); }}>
              <input autoFocus value={renameDraft} onChange={(event) => onRenameDraft(event.target.value)} onKeyDown={(event) => { if (event.key === 'Escape') onCancelRename(); }} maxLength={80} aria-label={`Rename ${workspace.name}`} className="h-7 min-w-0 flex-1 rounded-md border border-primary bg-background px-2 text-meta text-foreground outline-none" />
              <button type="submit" disabled={busy || !renameDraft.trim()} className="rounded-md px-2 text-secondary hover:bg-secondary/10 disabled:opacity-40" aria-label="Save workspace name"><Check size={13} /></button>
              <button type="button" onClick={onCancelRename} className="rounded-md px-2 text-muted-foreground hover:bg-surface-3" aria-label="Cancel rename"><X size={13} /></button>
            </form>
          ) : (
            <div className="flex min-w-0 items-baseline gap-2">
              <h2 className="truncate text-body font-medium text-foreground">{workspace.name}</h2>
              <time className="shrink-0 text-micro text-subtle-foreground" dateTime={new Date(workspace.updatedAt).toISOString()}>{formatUpdated(workspace.updatedAt)}</time>
            </div>
          )}
          <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-micro text-muted-foreground">
            <span>{tabs} {tabs === 1 ? 'tab' : 'tabs'} · {split ? 'split panes' : 'single pane'}</span>
            <span className="inline-flex items-center gap-1"><PanelLeft size={10} />{workspace.shell.sidebarOpen ? 'Sidebar on' : 'Sidebar off'}</span>
            <span className="inline-flex items-center gap-1"><PanelRight size={10} />{workspace.shell.rightDockOpen ? workspace.shell.rightDockTab : 'Dock off'}</span>
          </div>
        </div>
      </div>
      {!editing && <div className="mt-3 flex flex-wrap items-center gap-1.5 border-t border-border/70 pt-2.5">
        <Button size="sm" variant="primary" onClick={onLoad} disabled={busy}>Load</Button>
        <Button size="sm" variant="ghost" onClick={onReplace} disabled={busy} title="Replace with the layout currently on screen"><RefreshCw size={12} />Update</Button>
        <Button size="sm" variant="ghost" onClick={onStartRename} disabled={busy}><Pencil size={12} />Rename</Button>
        <Button size="sm" variant={confirmDelete ? 'destructive' : 'ghost'} className="sm:ml-auto" onClick={onDelete} disabled={busy}><Trash2 size={12} />{confirmDelete ? 'Confirm delete' : 'Delete'}</Button>
      </div>}
    </article>
  );
}

function formatUpdated(updatedAt: number): string {
  const elapsed = Date.now() - updatedAt;
  if (elapsed < 60_000) return 'just now';
  if (elapsed < 3_600_000) return `${Math.floor(elapsed / 60_000)}m ago`;
  if (elapsed < 86_400_000) return `${Math.floor(elapsed / 3_600_000)}h ago`;
  return new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(updatedAt);
}

function message(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
