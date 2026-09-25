/**
 * Native desktop settings for the legacy Markdown workspace while Ley migrates
 * to the focused continuity product.
 */

import { useEffect, useState, type RefObject } from "react";
import { useLiveQuery } from "dexie-react-hooks";
import * as Dialog from "@radix-ui/react-dialog";
import { X, FolderOpen, RefreshCw, Repeat2, RotateCcw } from "lucide-react";
import { db } from "@/infrastructure/database/db";
import { useUIStore, type Theme } from "@/shared/state/ui";
import { Kbd } from "@/shared/components/Kbd";
import { cn } from "@/shared/lib/classnames";
import {
  listActiveVaultTrash,
  type VaultFileSnapshot,
} from "@/infrastructure/vault/filesystem-vault";
import { listVaultTemplates } from "@/core/vault/templates";
import { format as formatDate } from "date-fns";
import { restoreTrashedFilesystemPage } from "@/core/vault/pages";

export function SettingsModal({
  open,
  vaultName,
  watcherStatus,
  onRefreshVault,
  onSwitchVault,
  onClose,
  onOpenNote,
  launcherRef,
}: {
  open: boolean;
  vaultName: string;
  watcherStatus: "inactive" | "starting" | "watching" | "error";
  onRefreshVault: () => Promise<{ noteCount: number } | null>;
  onSwitchVault: () => Promise<void>;
  onOpenNote: (id: string) => void;
  onClose: () => void;
  launcherRef: RefObject<HTMLButtonElement | null>;
}) {
  const theme = useUIStore((state) => state.theme);
  const setTheme = useUIStore((state) => state.setTheme);
  const [dailyFormatError, setDailyFormatError] = useState<string | null>(null);
  const [trashStatus, setTrashStatus] = useState<string | null>(null);
  const [vaultActionStatus, setVaultActionStatus] = useState<string | null>(null);
  const [vaultActionBusy, setVaultActionBusy] = useState(false);
  const [filesystemTrash, setFilesystemTrash] = useState<
    VaultFileSnapshot[] | null
  >(null);
  const [trashRequestId, setTrashRequestId] = useState(0);
  const [projectedAt, setProjectedAt] = useState(() => Date.now());

  const dailyFormat = useLiveQuery(
    async () =>
      (await db.settings.get("daily-note-format"))?.value as string | undefined,
    [],
  );
  const templateFolder = useLiveQuery(
    async () =>
      (await db.settings.get("template-folder"))?.value as string | undefined,
    [],
  );
  const vaultTemplates = useLiveQuery(listVaultTemplates, [], []);
  const dailyTemplatePath = useLiveQuery(
    async () =>
      (await db.settings.get("daily-note-template-path"))?.value as
        | string
        | undefined,
    [],
  );

  useEffect(() => {
    if (!open) return;
    const refresh = () => setProjectedAt(Date.now());
    window.addEventListener("ley:vault-projected", refresh);
    return () => window.removeEventListener("ley:vault-projected", refresh);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    let current = true;
    void listActiveVaultTrash()
      .then((files) => {
        if (current) setFilesystemTrash(files);
      })
      .catch((error) => {
        if (current)
          setTrashStatus(error instanceof Error ? error.message : String(error));
      });
    return () => {
      current = false;
    };
  }, [open, projectedAt, trashRequestId]);

  if (!open) return null;

  async function saveFormat(value: string) {
    try {
      formatDate(new Date(), value);
      await db.settings.put({ key: "daily-note-format", value });
      setDailyFormatError(null);
    } catch {
      setDailyFormatError("That date format is not valid. Try yyyy-MM-dd.");
    }
  }

  async function handleRestoreTrashed(path: string) {
    try {
      const restored = await restoreTrashedFilesystemPage(path);
      setFilesystemTrash(await listActiveVaultTrash());
      setTrashRequestId((current) => current + 1);
      await onRefreshVault();
      onClose();
      onOpenNote(restored.id);
      setTrashStatus(`Restored “${restored.title}” to ${restored.path}.`);
    } catch (cause) {
      setTrashStatus(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleRefreshVault() {
    setVaultActionBusy(true);
    setVaultActionStatus("Refreshing vault…");
    try {
      const refreshed = await onRefreshVault();
      setVaultActionStatus(
        refreshed
          ? `Up to date · ${refreshed.noteCount} ${refreshed.noteCount === 1 ? "note" : "notes"}.`
          : "Vault refresh is unavailable.",
      );
    } catch (cause) {
      setVaultActionStatus(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setVaultActionBusy(false);
    }
  }

  return (
    <Dialog.Root
      open={open}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-50" />
        <Dialog.Content
          aria-describedby={undefined}
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            launcherRef.current?.focus();
          }}
          className="app-modal-surface fixed left-1/2 top-1/2 z-[51] flex max-h-[min(760px,92vh)] w-[520px] max-w-[92vw] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-md border outline-none"
        >
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
            <SettingsAppearance theme={theme} setTheme={setTheme} />
            <SettingsTemplates templateFolder={templateFolder} />
            <SettingsDailyNotes
              dailyFormat={dailyFormat}
              dailyFormatError={dailyFormatError}
              saveFormat={saveFormat}
              dailyTemplatePath={dailyTemplatePath}
              vaultTemplates={vaultTemplates}
            />
            <SettingsVault
              vaultName={vaultName}
              watcherStatus={watcherStatus}
              vaultActionBusy={vaultActionBusy}
              handleRefreshVault={handleRefreshVault}
              vaultActionStatus={vaultActionStatus}
              setVaultActionStatus={setVaultActionStatus}
              onClose={onClose}
              onSwitchVault={onSwitchVault}
            />
            <FolderTrash
              filesystemTrash={filesystemTrash}
              handleRestoreTrashed={handleRestoreTrashed}
              trashStatus={trashStatus}
              setTrashStatus={setTrashStatus}
            />
          </div>

          <div className="flex items-center justify-between border-t border-border px-4 py-2 text-micro text-muted-foreground">
            <span>Local-first — Markdown stays in your chosen folder.</span>
            <Kbd>esc</Kbd>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function SettingsAppearance({
  theme,
  setTheme,
}: {
  theme: Theme;
  setTheme: (theme: Theme) => void;
}) {
  return (
    <section>
      <div className="mb-2 text-meta font-medium text-foreground">Appearance</div>
      <div className="flex gap-2">
        {(["dark", "light"] as Theme[]).map((nextTheme) => (
          <button
            key={nextTheme}
            type="button"
            onClick={() => {
              setTheme(nextTheme);
              void db.settings.put({ key: "theme", value: nextTheme });
            }}
            className={cn(
              "flex-1 rounded-sm border px-3 py-2 text-left text-meta outline-none transition-colors focus-visible:ring-2 focus-visible:ring-primary",
              theme === nextTheme
                ? "border-primary bg-primary/10 text-foreground"
                : "border-border bg-surface-2 text-muted-foreground-strong hover:bg-surface-3",
            )}
          >
            <div className="font-medium capitalize">{nextTheme}</div>
            <div className="mt-0.5 text-micro text-muted-foreground">
              {nextTheme === "dark"
                ? "Easier on the eyes at night"
                : "Bright, high contrast"}
            </div>
          </button>
        ))}
      </div>
    </section>
  );
}

function SettingsTemplates({ templateFolder }: { templateFolder?: string }) {
  return (
    <section>
      <div className="mb-2 text-meta font-medium text-foreground">Templates</div>
      <label className="flex flex-col gap-1 text-meta text-muted-foreground-strong">
        Template folder
        <input
          type="text"
          defaultValue={templateFolder ?? "templates"}
          key={templateFolder ?? "templates"}
          onBlur={(event) =>
            void db.settings.put({
              key: "template-folder",
              value: event.target.value.trim() || "templates",
            })
          }
          className="h-8 rounded-md border border-border bg-surface-1 px-2 font-mono text-meta text-foreground focus:border-primary focus:ring-2 focus:ring-primary/35 focus:outline-none"
          placeholder="templates"
        />
        <span className="text-micro text-muted-foreground">
          Markdown files in this vault folder appear in the new-note dialog.
        </span>
      </label>
    </section>
  );
}

function SettingsDailyNotes({
  dailyFormat,
  dailyFormatError,
  saveFormat,
  dailyTemplatePath,
  vaultTemplates,
}: {
  dailyFormat?: string;
  dailyFormatError: string | null;
  saveFormat: (value: string) => Promise<void>;
  dailyTemplatePath?: string;
  vaultTemplates: Array<{ id: string; path: string; title: string }>;
}) {
  return (
    <section>
      <div className="mb-2 text-meta font-medium text-foreground">Daily notes</div>
      <label className="flex flex-col gap-1 text-meta text-muted-foreground-strong">
        File name format
        <input
          type="text"
          defaultValue={dailyFormat ?? "yyyy-MM-dd"}
          key={dailyFormat ?? "default"}
          onBlur={(event) => void saveFormat(event.target.value)}
          className="h-8 rounded-md border border-border bg-surface-1 px-2 text-meta text-foreground focus:border-primary focus:ring-2 focus:ring-primary/35 focus:outline-none"
          placeholder="yyyy-MM-dd"
        />
        <span className="text-micro text-muted-foreground">
          date-fns format tokens, for example yyyy-MM-dd or EEEE, MMMM do.
        </span>
        {dailyFormatError && (
          <span className="text-micro text-destructive" role="alert">
            {dailyFormatError}
          </span>
        )}
      </label>
      <label className="mt-3 flex flex-col gap-1 text-meta text-muted-foreground-strong">
        Daily note template
        <select
          value={dailyTemplatePath ?? ""}
          onChange={(event) =>
            void db.settings.put({
              key: "daily-note-template-path",
              value: event.target.value,
            })
          }
          className="h-8 rounded-md border border-border bg-surface-1 px-2 text-meta text-foreground focus:border-primary focus:ring-2 focus:ring-primary/35 focus:outline-none"
        >
          <option value="">Built-in daily note</option>
          {vaultTemplates.map((template) => (
            <option key={template.id} value={template.path}>
              {template.title}
            </option>
          ))}
        </select>
      </label>
    </section>
  );
}

function SettingsVault({
  vaultName,
  watcherStatus,
  vaultActionBusy,
  handleRefreshVault,
  vaultActionStatus,
  setVaultActionStatus,
  onClose,
  onSwitchVault,
}: {
  vaultName: string;
  watcherStatus: "inactive" | "starting" | "watching" | "error";
  vaultActionBusy: boolean;
  handleRefreshVault: () => Promise<void>;
  vaultActionStatus: string | null;
  setVaultActionStatus: (value: string | null) => void;
  onClose: () => void;
  onSwitchVault: () => Promise<void>;
}) {
  const watching = watcherStatus === "watching";
  const unavailable = watcherStatus === "error";
  return (
    <section>
      <div className="mb-2 flex items-center justify-between gap-3">
        <span className="text-meta font-medium text-foreground">Vault</span>
        <span className="max-w-56 truncate font-mono text-micro text-muted-foreground">
          {vaultName}
        </span>
      </div>
      <div className="rounded-md border border-border bg-surface-2 p-3 text-meta text-muted-foreground">
        <div className="flex items-center gap-2 font-medium text-foreground">
          <FolderOpen size={14} className="text-secondary" />
          Filesystem vault
        </div>
        <p className="mt-1 leading-relaxed">
          This vault is an ordinary folder. Back it up, sync it, zip it, or commit
          it with your normal filesystem tools.
        </p>
        <div className="mt-2 flex items-center gap-1.5 text-micro text-muted-foreground">
          <span
            className={cn(
              "size-1.5 rounded-full",
              watching
                ? "bg-success"
                : unavailable
                  ? "bg-destructive"
                  : "bg-muted-foreground",
            )}
          />
          {watching
            ? "Watching external changes live"
            : unavailable
              ? "Live watcher unavailable · manual refresh remains available"
              : "Starting live filesystem watcher…"}
        </div>
        <button
          type="button"
          disabled={vaultActionBusy}
          onClick={() => void handleRefreshVault()}
          className="mt-3 flex items-center gap-1.5 rounded-md border border-border bg-background px-2.5 py-1.5 text-meta text-foreground outline-none transition-colors hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-primary disabled:opacity-50"
        >
          <RefreshCw
            size={13}
            className={
              vaultActionBusy ? "animate-spin motion-reduce:animate-none" : ""
            }
          />
          Refresh from folder
        </button>
      </div>
      <button
        type="button"
        onClick={() => {
          onClose();
          void onSwitchVault();
        }}
        className="mt-3 flex w-full items-center justify-center gap-1.5 rounded-md border border-border px-3 py-2 text-meta text-muted-foreground-strong outline-none transition-colors hover:bg-surface-2 hover:text-foreground focus-visible:ring-2 focus-visible:ring-primary"
      >
        <Repeat2 size={13} />
        Open another folder
      </button>
      {vaultActionStatus && (
        <button
          type="button"
          onClick={() => setVaultActionStatus(null)}
          className="mt-2 rounded-sm text-left text-micro text-secondary outline-none focus-visible:underline focus-visible:ring-2 focus-visible:ring-primary"
          role="status"
        >
          {vaultActionStatus}
        </button>
      )}
    </section>
  );
}

function FolderTrash({
  filesystemTrash,
  handleRestoreTrashed,
  trashStatus,
  setTrashStatus,
}: {
  filesystemTrash: VaultFileSnapshot[] | null;
  handleRestoreTrashed: (path: string) => Promise<void>;
  trashStatus: string | null;
  setTrashStatus: (value: string | null) => void;
}) {
  return (
    <section>
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="text-meta font-medium text-foreground">Folder trash</div>
        <span className="text-micro text-muted-foreground">
          {filesystemTrash?.length ?? 0}{" "}
          {filesystemTrash?.length === 1 ? "note" : "notes"}
        </span>
      </div>
      {filesystemTrash === null ? (
        <div className="rounded-sm border border-dashed border-border px-3 py-4 text-center text-meta text-muted-foreground">
          Checking .trash…
        </div>
      ) : filesystemTrash.length === 0 ? (
        <div className="rounded-sm border border-dashed border-border px-3 py-4 text-center text-meta text-muted-foreground">
          No Markdown notes are currently in .trash.
        </div>
      ) : (
        <div className="max-h-44 divide-y divide-border overflow-y-auto rounded-md border border-border bg-surface-2">
          {filesystemTrash.map((file) => (
            <div key={file.path} className="flex items-center gap-2 px-3 py-2">
              <div className="min-w-0 flex-1">
                <div className="truncate font-mono text-micro text-foreground">
                  {file.path}
                </div>
              </div>
              <button
                type="button"
                onClick={() => void handleRestoreTrashed(file.path)}
                className="rounded-md border border-border p-1.5 text-muted-foreground outline-none transition-colors hover:bg-surface-3 hover:text-foreground focus-visible:ring-2 focus-visible:ring-primary"
                aria-label={`Restore ${file.path}`}
                title="Restore"
              >
                <RotateCcw size={13} />
              </button>
            </div>
          ))}
        </div>
      )}
      {trashStatus && (
        <button
          type="button"
          onClick={() => setTrashStatus(null)}
          className="mt-2 rounded-sm text-left text-micro text-secondary outline-none focus-visible:underline focus-visible:ring-2 focus-visible:ring-primary"
          role="status"
        >
          {trashStatus}
        </button>
      )}
    </section>
  );
}
