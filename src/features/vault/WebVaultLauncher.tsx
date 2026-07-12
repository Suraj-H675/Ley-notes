import { ArrowRight, Database, FolderOpen, Globe2, ShieldCheck } from 'lucide-react';

export function WebVaultLauncher({
  folderSupported,
  busy,
  error,
  onOpenFolder,
  onBrowserLocal,
}: {
  folderSupported: boolean;
  busy: boolean;
  error: string | null;
  onOpenFolder: () => void;
  onBrowserLocal: () => void;
}) {
  return (
    <main className="relative flex min-h-full items-center justify-center overflow-y-auto bg-background px-5 py-12 text-foreground">
      <div className="pointer-events-none absolute inset-0 [background:radial-gradient(circle_at_50%_15%,hsl(var(--primary)/0.17),transparent_43%)]" />
      <section className="relative w-full max-w-2xl">
        <div className="mb-8 text-center">
          <div className="mx-auto mb-4 flex size-11 items-center justify-center rounded-xl bg-primary text-lg font-semibold text-primary-foreground">L</div>
          <h1 className="text-3xl font-semibold tracking-[-0.035em]">Where should your knowledge live?</h1>
          <p className="mx-auto mt-3 max-w-lg text-sm leading-6 text-muted-foreground">Both options stay on this device. A folder vault gives you portable Markdown files; browser-local is the compatibility fallback.</p>
        </div>

        <div className="grid gap-3 md:grid-cols-2">
          <button type="button" disabled={!folderSupported || busy} onClick={onOpenFolder} className="group flex min-h-52 flex-col rounded-xl border border-primary/35 bg-primary/8 p-5 text-left transition hover:border-primary hover:bg-primary/12 disabled:pointer-events-none disabled:opacity-45">
            <span className="flex size-10 items-center justify-center rounded-lg bg-primary/15 text-primary"><FolderOpen size={19} /></span>
            <span className="mt-5 font-semibold">Open a folder</span>
            <span className="mt-1 text-meta leading-5 text-muted-foreground">Read and write ordinary `.md` files. Best for ownership, Git, backups, and interoperability.</span>
            <span className="mt-auto flex items-center gap-1 pt-5 text-micro font-medium text-primary">Recommended <ArrowRight size={12} className="transition-transform group-hover:translate-x-0.5" /></span>
          </button>

          <button type="button" disabled={busy} onClick={onBrowserLocal} className="group flex min-h-52 flex-col rounded-xl border border-border bg-surface-1 p-5 text-left transition hover:border-border-strong hover:bg-surface-2 disabled:pointer-events-none disabled:opacity-45">
            <span className="flex size-10 items-center justify-center rounded-lg bg-surface-3 text-secondary"><Database size={19} /></span>
            <span className="mt-5 font-semibold">Use browser-local vault</span>
            <span className="mt-1 text-meta leading-5 text-muted-foreground">Store notes in this browser. Works broadly, but requires ZIP export for portable Markdown files.</span>
            <span className="mt-auto flex items-center gap-1 pt-5 text-micro text-muted-foreground">Compatibility mode <ArrowRight size={12} className="transition-transform group-hover:translate-x-0.5" /></span>
          </button>
        </div>

        {!folderSupported && <div className="mt-3 rounded-lg border border-border bg-surface-1 px-4 py-3 text-meta text-muted-foreground"><Globe2 size={14} className="mr-2 inline text-secondary" />This browser does not expose folder access. Browser-local mode is still fully offline.</div>}
        {error && <div className="mt-3 rounded-lg bg-destructive/10 px-4 py-3 text-meta text-destructive">{error}</div>}
        <div className="mt-6 flex items-center justify-center gap-2 text-micro text-subtle-foreground"><ShieldCheck size={13} /> Ley never uploads either vault without an explicit future sync action.</div>
      </section>
    </main>
  );
}
