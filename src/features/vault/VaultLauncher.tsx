import { FolderOpen, ShieldCheck, FileText, ArrowRight } from 'lucide-react';
import type { DesktopVault } from '@/infrastructure/vault/filesystem-vault';

export function VaultLauncher({
  busy,
  error,
  onOpen,
}: {
  busy: boolean;
  error: string | null;
  onOpen: () => Promise<DesktopVault | null>;
}) {
  return (
    <main data-page="desktop-vault-launcher" className="relative flex h-full flex-col items-center overflow-y-auto overscroll-y-contain bg-background px-6 py-6 text-foreground">
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 opacity-55 [background-image:linear-gradient(to_right,hsl(var(--border)/0.35)_1px,transparent_1px)] [background-size:72px_100%]" />
      <section className="relative my-auto w-full max-w-xl rounded-sm border border-border bg-surface-1 p-8 shadow-popover">
        <div className="mb-8 flex items-center gap-3">
          <div className="flex size-10 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <span className="text-lg font-semibold">L</span>
          </div>
          <div>
            <h1 className="text-xl font-semibold tracking-tight">Open your Ley vault</h1>
            <p className="text-meta text-muted-foreground">Your folder is your second brain.</p>
          </div>
        </div>

          <button type="button" disabled={busy} onClick={() => void onOpen()} className="group flex w-full items-center gap-4 rounded-sm border border-border bg-surface-2 p-4 text-left transition hover:border-primary/70 hover:bg-surface-3 disabled:pointer-events-none disabled:opacity-60">
          <div className="flex size-10 shrink-0 items-center justify-center rounded-md bg-background text-primary">
            <FolderOpen size={19} />
          </div>
          <div className="min-w-0 flex-1">
            <div className="font-medium">Open folder as vault</div>
            <div className="mt-0.5 text-meta text-muted-foreground">
              Choose an existing Markdown folder or create a new empty folder.
            </div>
          </div>
          <ArrowRight size={17} className="text-muted-foreground transition-transform group-hover:translate-x-0.5" />
        </button>

        {error && <p className="mt-3 rounded-md bg-destructive/10 px-3 py-2 text-meta text-destructive">{error}</p>}

        <div className="mt-8 grid grid-cols-2 gap-4 border-t border-border/70 pt-6 text-meta text-muted-foreground">
          <div className="flex gap-2">
            <FileText size={15} className="mt-0.5 shrink-0 text-secondary" />
            <span>Plain `.md` files remain readable in any editor.</span>
          </div>
          <div className="flex gap-2">
            <ShieldCheck size={15} className="mt-0.5 shrink-0 text-secondary" />
            <span>No account, server, upload, or telemetry required.</span>
          </div>
        </div>
      </section>
    </main>
  );
}
