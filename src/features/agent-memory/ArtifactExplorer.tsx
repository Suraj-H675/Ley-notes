import { useEffect, useState } from "react";
import {
  AlertTriangle,
  ArchiveX,
  FileCode2,
  FileLock2,
  Search,
  ShieldCheck,
} from "lucide-react";
import { cn } from "@/shared/lib/classnames";
import { readAgentArtifacts } from "./api";
import type { ProjectArtifactInventory } from "./types";

type InventoryView = "captured" | "skipped";

export function ArtifactExplorer({
  projectPath,
  focus = null,
}: {
  projectPath: string;
  focus?: { path: string; requestId: number } | null;
}) {
  const [query, setQuery] = useState(focus?.path ?? "");
  const [deferredQuery, setDeferredQuery] = useState(focus?.path ?? "");
  const [view, setView] = useState<InventoryView>("captured");
  const [inventory, setInventory] = useState<ProjectArtifactInventory | null>(
    null,
  );
  const [completedRequest, setCompletedRequest] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const requestKey = `${projectPath}\u0000${deferredQuery}`;
  const loading = completedRequest !== requestKey;

  useEffect(() => {
    const timer = window.setTimeout(() => setDeferredQuery(query), 220);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    let current = true;
    void readAgentArtifacts(projectPath, deferredQuery)
      .then((next) => {
        if (current) {
          setInventory(next);
          setError(null);
        }
      })
      .catch((cause) => {
        if (current) setError(errorMessage(cause));
      })
      .finally(() => {
        if (current) setCompletedRequest(requestKey);
      });
    return () => {
      current = false;
    };
  }, [deferredQuery, projectPath, requestKey]);

  const shown =
    view === "captured"
      ? (inventory?.artifacts.length ?? 0)
      : (inventory?.skipped.length ?? 0);
  const matching =
    view === "captured"
      ? (inventory?.totalMatchingArtifacts ?? 0)
      : (inventory?.totalMatchingSkipped ?? 0);

  return (
    <section className="space-y-5" aria-labelledby="artifacts-title">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <p className="mb-1 text-micro font-semibold uppercase tracking-[0.14em] text-primary">
            Snapshot evidence
          </p>
          <h2
            id="artifacts-title"
            className="text-xl font-semibold tracking-tight"
          >
            Artifacts
          </h2>
          <p className="mt-1 max-w-2xl text-meta leading-relaxed text-muted-foreground">
            Files Ley retained, redacted, or deliberately excluded from the
            latest deterministic capture.
          </p>
        </div>
        <label className="relative block w-full lg:w-80">
          <span className="sr-only">Search captured artifacts</span>
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
            size={15}
          />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Path, language, or kind"
            className="h-10 w-full rounded-md border border-border bg-surface-1 pl-9 pr-3 text-meta outline-none transition focus:border-primary/60 focus:ring-2 focus:ring-primary/20"
          />
        </label>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border bg-surface-1 p-2">
        <div className="flex gap-1" role="tablist" aria-label="Artifact status">
          <InventoryTab
            active={view === "captured"}
            onClick={() => setView("captured")}
            label="Captured"
            count={inventory?.totalMatchingArtifacts}
          />
          <InventoryTab
            active={view === "skipped"}
            onClick={() => setView("skipped")}
            label="Excluded"
            count={inventory?.totalMatchingSkipped}
          />
        </div>
        <p className="px-2 text-micro text-muted-foreground" aria-live="polite">
          {loading
            ? "Reading local snapshot…"
            : `Showing ${shown} of ${matching} matching`}
        </p>
      </div>

      {focus && (
        <div className="flex items-start gap-3 rounded-md border border-primary/20 bg-primary/6 px-4 py-3 text-meta">
          <FileCode2
            size={17}
            className="mt-0.5 shrink-0 text-primary"
            aria-hidden="true"
          />
          <div className="min-w-0">
            <p className="font-semibold">Following a memory reference</p>
            <p className="mt-0.5 break-all font-mono text-micro text-muted-foreground">
              {focus.path}
            </p>
          </div>
        </div>
      )}

      {error && !loading ? (
        <ErrorState message={error} />
      ) : !inventory && loading ? (
        <ArtifactSkeleton />
      ) : view === "captured" ? (
        <CapturedArtifacts inventory={inventory} focusPath={focus?.path} />
      ) : (
        <SkippedArtifacts inventory={inventory} />
      )}

      {inventory && (
        <div className="flex flex-col gap-2 rounded-md border border-border/80 bg-surface-1 px-4 py-3 text-micro text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <span className="inline-flex items-center gap-2">
            <ShieldCheck
              size={14}
              className="text-primary"
              aria-hidden="true"
            />
            Stored in your local vault · source snapshot{" "}
            {shortId(inventory.artifactSnapshotId)}
          </span>
          <span>Live source not checked in this view</span>
        </div>
      )}
    </section>
  );
}

function CapturedArtifacts({
  inventory,
  focusPath,
}: {
  inventory: ProjectArtifactInventory | null;
  focusPath?: string;
}) {
  if (!inventory || inventory.artifacts.length === 0) {
    return (
      <EmptyState
        icon={FileCode2}
        title="No captured artifacts match"
        detail="Try a broader path, language, or artifact kind."
      />
    );
  }
  return (
    <div className="overflow-hidden rounded-md border border-border bg-surface-1">
      <div className="hidden grid-cols-[minmax(0,1fr)_8rem_7rem_6rem] gap-4 border-b border-border px-4 py-2 text-micro font-semibold uppercase tracking-wider text-muted-foreground sm:grid">
        <span>Artifact</span>
        <span>Kind</span>
        <span>Size</span>
        <span>Lines</span>
      </div>
      <div className="divide-y divide-border/70">
        {inventory.artifacts.map((artifact) => (
          <details
            key={artifact.path}
            open={artifact.path === focusPath}
            className={cn(
              "group",
              artifact.path === focusPath && "bg-primary/[0.035]",
            )}
          >
            <summary className="grid cursor-pointer list-none gap-2 px-4 py-3 outline-none hover:bg-surface-2 focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary sm:grid-cols-[minmax(0,1fr)_8rem_7rem_6rem] sm:items-center sm:gap-4">
              <span className="flex min-w-0 items-center gap-2.5">
                <FileCode2
                  size={15}
                  className="shrink-0 text-primary"
                  aria-hidden="true"
                />
                <span className="min-w-0">
                  <span className="block truncate text-meta font-medium">
                    {artifact.path}
                  </span>
                  <span className="text-micro text-muted-foreground sm:hidden">
                    {humanize(artifact.kind)} ·{" "}
                    {formatBytes(artifact.sourceBytes)} ·{" "}
                    {artifact.lineCount.toLocaleString()} lines
                  </span>
                </span>
                {artifact.redactions.length > 0 && (
                  <span
                    className="rounded-full bg-warning/12 px-1.5 py-0.5 text-[10px] font-semibold text-warning"
                    title="Sensitive values were redacted before storage"
                  >
                    redacted
                  </span>
                )}
              </span>
              <span className="hidden text-meta text-muted-foreground sm:block">
                {artifact.language ?? humanize(artifact.kind)}
              </span>
              <span className="hidden text-meta text-muted-foreground sm:block">
                {formatBytes(artifact.sourceBytes)}
              </span>
              <span className="hidden text-meta text-muted-foreground sm:block">
                {artifact.lineCount.toLocaleString()}
              </span>
            </summary>
            <div className="grid gap-3 bg-surface-2/60 px-4 py-3 text-micro text-muted-foreground sm:grid-cols-3">
              <Metric
                label="Stored"
                value={formatBytes(artifact.storedBytes)}
              />
              <Metric
                label="Evidence"
                value={
                  artifact.retainedSource
                    ? "Source retained locally"
                    : "Metadata only"
                }
              />
              <Metric
                label="Redaction findings"
                value={
                  artifact.redactions.length === 0
                    ? "None"
                    : artifact.redactions
                        .map(
                          (finding) =>
                            `${humanize(finding.kind)} · ${finding.lines.length} ${finding.lines.length === 1 ? "line" : "lines"}`,
                        )
                        .join(", ")
                }
              />
            </div>
          </details>
        ))}
      </div>
      {inventory.omittedArtifacts > 0 && (
        <p className="border-t border-border px-4 py-3 text-center text-micro text-muted-foreground">
          {inventory.omittedArtifacts.toLocaleString()} more matches omitted by
          the local response bound. Refine the search to inspect them.
        </p>
      )}
    </div>
  );
}

function SkippedArtifacts({
  inventory,
}: {
  inventory: ProjectArtifactInventory | null;
}) {
  if (!inventory || inventory.skipped.length === 0) {
    return (
      <EmptyState
        icon={ArchiveX}
        title="No excluded artifacts match"
        detail="Binary, oversized, non-UTF-8, symlink, and capture-limit exclusions appear here."
      />
    );
  }
  return (
    <div className="overflow-hidden rounded-md border border-border bg-surface-1">
      <div className="divide-y divide-border/70">
        {inventory.skipped.map((artifact) => (
          <div
            key={`${artifact.reason}:${artifact.path}`}
            className="flex items-center justify-between gap-4 px-4 py-3"
          >
            <span className="flex min-w-0 items-center gap-2.5">
              <FileLock2
                size={15}
                className="shrink-0 text-muted-foreground"
                aria-hidden="true"
              />
              <span className="truncate text-meta font-medium">
                {artifact.path}
              </span>
            </span>
            <span className="shrink-0 text-right text-micro text-muted-foreground">
              {humanize(artifact.reason)} · {formatBytes(artifact.bytes)}
            </span>
          </div>
        ))}
      </div>
      {inventory.omittedSkipped > 0 && (
        <p className="border-t border-border px-4 py-3 text-center text-micro text-muted-foreground">
          {inventory.omittedSkipped.toLocaleString()} more exclusions omitted.
        </p>
      )}
    </div>
  );
}

function InventoryTab({
  active,
  onClick,
  label,
  count,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
  count?: number;
}) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={active}
      onClick={onClick}
      className={cn(
        "inline-flex h-8 items-center gap-2 rounded-md px-2.5 text-meta font-medium outline-none focus-visible:ring-2 focus-visible:ring-primary",
        active
          ? "bg-surface-3 text-foreground shadow-sm"
          : "text-muted-foreground hover:text-foreground",
      )}
    >
      {label}
      {count !== undefined && (
        <span className="rounded-full bg-background/70 px-1.5 text-[10px]">
          {count}
        </span>
      )}
    </button>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <span>
      <span className="mb-0.5 block font-semibold text-foreground">
        {label}
      </span>
      {value}
    </span>
  );
}

function EmptyState({
  icon: Icon,
  title,
  detail,
}: {
  icon: typeof FileCode2;
  title: string;
  detail: string;
}) {
  return (
    <div className="flex min-h-56 flex-col items-center justify-center rounded-md border border-dashed border-border bg-surface-1 px-5 text-center">
      <Icon
        size={24}
        className="mb-3 text-muted-foreground"
        aria-hidden="true"
      />
      <h3 className="text-body font-semibold">{title}</h3>
      <p className="mt-1 max-w-md text-meta text-muted-foreground">{detail}</p>
    </div>
  );
}

function ErrorState({ message }: { message: string }) {
  return (
    <div
      role="alert"
      className="flex items-start gap-3 rounded-md border border-destructive/25 bg-destructive/8 p-4 text-meta"
    >
      <AlertTriangle
        size={17}
        className="mt-0.5 shrink-0 text-destructive"
        aria-hidden="true"
      />
      <div>
        <p className="font-semibold">Could not read artifact inventory</p>
        <p className="mt-0.5 text-muted-foreground">{message}</p>
      </div>
    </div>
  );
}

function ArtifactSkeleton() {
  return (
    <div
      className="space-y-px overflow-hidden rounded-md border border-border bg-surface-1"
      aria-label="Loading artifacts"
    >
      {[0, 1, 2, 3].map((item) => (
        <div key={item} className="h-14 animate-pulse bg-surface-2/70" />
      ))}
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1_024) return `${bytes} B`;
  if (bytes < 1_048_576) return `${(bytes / 1_024).toFixed(1)} KB`;
  return `${(bytes / 1_048_576).toFixed(1)} MB`;
}

function humanize(value: string): string {
  return value
    .replaceAll("-", " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

function shortId(value: string): string {
  return value.length > 12 ? `${value.slice(0, 12)}…` : value;
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
