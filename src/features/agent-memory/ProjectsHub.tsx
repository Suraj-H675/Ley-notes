import { useState } from "react";
import {
  AlertTriangle,
  ArrowUpRight,
  BrainCircuit,
  CheckCircle2,
  CircleDot,
  Files,
  FolderOpen,
  History,
  Inbox,
  LockKeyhole,
  Network,
  RefreshCw,
  Search,
  ShieldCheck,
  Unplug,
  X,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import type {
  AgentProjectCatalog,
  AgentProjectCatalogItem,
  AgentProjectCatalogState,
} from "./types";

export function ProjectsHub({
  catalog,
  loading,
  error,
  onAdd,
  onOpen,
  onForget,
  onReload,
}: {
  catalog: AgentProjectCatalog | null;
  loading: boolean;
  error: string | null;
  onAdd: () => void;
  onOpen: (projectPath: string) => void;
  onForget: (projectId: string) => void;
  onReload: () => void;
}) {
  const [query, setQuery] = useState("");
  const normalized = query.trim().toLocaleLowerCase();
  const projects =
    catalog?.projects.filter((project) => {
      if (!normalized) return true;
      return [
        project.projectName,
        project.projectPath,
        project.vaultName ?? "",
        project.state,
      ].some((value) => value.toLocaleLowerCase().includes(normalized));
    }) ?? [];

  return (
    <main className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
      <div className="mx-auto w-full max-w-6xl px-4 py-6 sm:px-6 sm:py-8 lg:px-10">
        <section className="relative overflow-hidden rounded-2xl border border-border bg-surface-1 p-5 shadow-panel sm:p-7">
          <div className="pointer-events-none absolute -right-20 -top-24 size-64 rounded-full bg-primary/10 blur-3xl" />
          <div className="relative flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
            <div className="max-w-2xl">
              <p className="text-micro font-semibold uppercase tracking-[0.14em] text-primary">
                Local agent memory
              </p>
              <h2 className="mt-1 text-2xl font-semibold tracking-[-0.035em] sm:text-3xl">
                Pick up any project without starting over
              </h2>
              <p className="mt-3 text-body leading-6 text-muted-foreground-strong">
                Ley remembers only projects you explicitly open. Each project
                keeps its own cited sessions, decisions, problems, lessons, and
                deterministic graph inside its bound filesystem vault.
              </p>
            </div>
            <Button variant="primary" onClick={onAdd}>
              <FolderOpen size={14} />
              Add project
            </Button>
          </div>
        </section>

        <section
          className="mt-5 grid gap-3 sm:grid-cols-3"
          aria-label="Project memory summary"
        >
          <SummaryTile
            icon={BrainCircuit}
            label="Known projects"
            value={catalog?.totalProjects ?? 0}
            detail="Explicitly opened on this device"
          />
          <SummaryTile
            icon={CheckCircle2}
            label="Ready in view"
            value={catalog?.readyProjects ?? 0}
            detail="Recent and locally available"
            positive
          />
          <SummaryTile
            icon={AlertTriangle}
            label="Attention in view"
            value={catalog?.attentionProjects ?? 0}
            detail="Reconnect, capture, or relocate"
            attention
          />
        </section>

        <section className="mt-8" aria-labelledby="known-projects-title">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h2
                id="known-projects-title"
                className="text-xl font-semibold tracking-tight"
              >
                Projects
              </h2>
              <p className="mt-1 text-meta text-muted-foreground">
                Recent first · paths stay in Ley’s owner-private device catalog
              </p>
            </div>
            <div className="flex w-full gap-2 sm:w-auto">
              <label className="relative min-w-0 flex-1 sm:w-72">
                <span className="sr-only">Find a project</span>
                <Search
                  size={15}
                  aria-hidden="true"
                  className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
                />
                <input
                  type="search"
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="Name, folder, vault, or status"
                  className="h-10 w-full rounded-lg border border-border bg-surface-1 pl-9 pr-3 text-meta outline-none transition focus:border-primary/60 focus:ring-2 focus:ring-primary/20"
                />
              </label>
              <Button
                variant="outline"
                size="md"
                className="size-10 shrink-0 px-0"
                onClick={onReload}
                disabled={loading}
                aria-label="Refresh project list"
                title="Refresh project list"
              >
                <RefreshCw
                  size={14}
                  className={
                    loading
                      ? "animate-spin motion-reduce:animate-none"
                      : undefined
                  }
                />
              </Button>
            </div>
          </div>

          {error && (
            <div
              role="alert"
              className="mt-4 flex items-start gap-3 rounded-xl border border-destructive/25 bg-destructive/8 p-4 text-meta"
            >
              <AlertTriangle
                size={17}
                className="mt-0.5 shrink-0 text-destructive"
                aria-hidden="true"
              />
              <div>
                <p className="font-semibold">Could not read local projects</p>
                <p className="mt-0.5 text-muted-foreground">{error}</p>
              </div>
            </div>
          )}

          {loading && !catalog ? (
            <div
              className="mt-4 grid gap-3 md:grid-cols-2"
              aria-label="Loading local projects"
            >
              {[0, 1, 2, 3].map((item) => (
                <div
                  key={item}
                  className="h-56 animate-pulse rounded-xl border border-border bg-surface-1"
                />
              ))}
            </div>
          ) : projects.length > 0 ? (
            <div className="mt-4 grid gap-3 md:grid-cols-2">
              {projects.map((project) => (
                <ProjectCard
                  key={project.projectId}
                  project={project}
                  onOpen={() => onOpen(project.projectPath)}
                  onForget={() => onForget(project.projectId)}
                />
              ))}
            </div>
          ) : (
            <EmptyProjects query={query} onAdd={onAdd} />
          )}

          {catalog && catalog.omittedProjects > 0 && (
            <p className="mt-3 rounded-xl border border-dashed border-border bg-surface-1 px-4 py-3 text-center text-micro text-muted-foreground">
              {catalog.omittedProjects.toLocaleString()} older projects are
              omitted by the local response bound.
            </p>
          )}
        </section>

        <div className="mt-8 flex flex-col gap-3 rounded-xl border border-border bg-surface-1 px-4 py-4 text-micro text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <span className="inline-flex items-start gap-2">
            <ShieldCheck
              size={14}
              className="mt-0.5 shrink-0 text-primary"
              aria-hidden="true"
            />
            {catalog?.privacyNotice ??
              "Ley never scans neighboring folders for projects."}
          </span>
          <span className="inline-flex items-center gap-2 whitespace-nowrap">
            <LockKeyhole size={13} aria-hidden="true" />
            No account · no knowledge cloud
          </span>
        </div>
      </div>
    </main>
  );
}

function SummaryTile({
  icon: Icon,
  label,
  value,
  detail,
  positive = false,
  attention = false,
}: {
  icon: typeof BrainCircuit;
  label: string;
  value: number;
  detail: string;
  positive?: boolean;
  attention?: boolean;
}) {
  return (
    <div className="rounded-xl border border-border bg-surface-1 p-4 shadow-panel">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-micro font-medium text-muted-foreground">{label}</p>
          <p className="mt-1 text-2xl font-semibold tabular-nums">{value}</p>
        </div>
        <span
          className={cn(
            "flex size-8 items-center justify-center rounded-lg",
            positive
              ? "bg-emerald-500/10 text-emerald-500"
              : attention
                ? "bg-warning/10 text-warning"
                : "bg-primary/10 text-primary",
          )}
        >
          <Icon size={15} aria-hidden="true" />
        </span>
      </div>
      <p className="mt-2 text-micro text-muted-foreground">{detail}</p>
    </div>
  );
}

function ProjectCard({
  project,
  onOpen,
  onForget,
}: {
  project: AgentProjectCatalogItem;
  onOpen: () => void;
  onForget: () => void;
}) {
  const unavailable =
    project.state === "project-unavailable" ||
    project.state === "identity-changed";
  const status = projectStatus(project.state);
  const StatusIcon = status.icon;
  return (
    <article className="group overflow-hidden rounded-xl border border-border bg-surface-1 shadow-panel transition hover:border-border-strong">
      <button
        type="button"
        onClick={onOpen}
        disabled={unavailable}
        className="block w-full p-4 text-left outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary disabled:cursor-default sm:p-5"
      >
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-3">
            <span className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-primary/20 bg-primary/10 text-primary">
              <BrainCircuit size={17} aria-hidden="true" />
            </span>
            <div className="min-w-0">
              <h3 className="truncate text-body font-semibold">
                {project.projectName}
              </h3>
              <p
                className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground"
                title={project.projectPath}
              >
                {project.projectPath}
              </p>
            </div>
          </div>
          {!unavailable && (
            <ArrowUpRight
              size={15}
              className="shrink-0 text-muted-foreground transition group-hover:text-primary"
              aria-hidden="true"
            />
          )}
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <span
            className={cn(
              "inline-flex items-center gap-1.5 rounded-full px-2 py-1 text-micro font-semibold",
              status.className,
            )}
          >
            <StatusIcon size={11} aria-hidden="true" />
            {status.label}
          </span>
          {project.vaultName && (
            <span className="truncate rounded-full bg-surface-3 px-2 py-1 text-micro text-muted-foreground">
              {project.vaultName}
            </span>
          )}
          <span className="text-micro text-muted-foreground">
            {relativeTime(project.lastOpenedAtUnixMs)}
          </span>
        </div>

        {project.state === "ready" ? (
          <div className="mt-4 grid grid-cols-3 gap-2">
            <Metric
              icon={History}
              value={project.sessions ?? 0}
              label="sessions"
            />
            <Metric
              icon={Files}
              value={project.files ?? 0}
              label="files"
            />
            <Metric
              icon={Inbox}
              value={project.reviewItems ?? 0}
              label="review"
            />
          </div>
        ) : (
          <p className="mt-4 line-clamp-2 text-meta leading-5 text-muted-foreground">
            {project.statusDetail}
          </p>
        )}

        {project.state === "ready" && (
          <div className="mt-4 flex flex-wrap items-center gap-x-4 gap-y-1 text-micro text-muted-foreground">
            <span className="inline-flex items-center gap-1.5">
              <CircleDot size={11} aria-hidden="true" />
              {project.activeSessions ?? 0} active
            </span>
            <span className="inline-flex items-center gap-1.5">
              <Network size={11} aria-hidden="true" />
              {(project.graphNodes ?? 0).toLocaleString()} graph nodes
            </span>
            <span>{humanize(project.freshness ?? "stored")}</span>
          </div>
        )}
      </button>
      <div className="flex items-center justify-between border-t border-border/70 px-4 py-2.5 sm:px-5">
        <span className="text-micro text-muted-foreground">
          {project.captureMode
            ? `${humanize(project.captureMode)} capture`
            : "Folder unavailable"}
        </span>
        <button
          type="button"
          onClick={onForget}
          className="inline-flex items-center gap-1.5 rounded px-1.5 py-1 text-micro font-medium text-muted-foreground outline-none hover:bg-surface-2 hover:text-foreground focus-visible:ring-2 focus-visible:ring-primary"
          aria-label={`Remove ${project.projectName} from Projects`}
          title="Remove from this device list; project memory is not deleted"
        >
          <X size={11} aria-hidden="true" />
          Remove
        </button>
      </div>
    </article>
  );
}

function Metric({
  icon: Icon,
  value,
  label,
}: {
  icon: typeof History;
  value: number;
  label: string;
}) {
  return (
    <div className="rounded-lg bg-background/45 p-2.5">
      <p className="flex items-center gap-1.5 text-micro text-muted-foreground">
        <Icon size={11} aria-hidden="true" />
        {label}
      </p>
      <p className="mt-1 text-body font-semibold tabular-nums">
        {value.toLocaleString()}
      </p>
    </div>
  );
}

function EmptyProjects({ query, onAdd }: { query: string; onAdd: () => void }) {
  return (
    <div className="mt-4 flex min-h-72 flex-col items-center justify-center rounded-xl border border-dashed border-border bg-surface-1 px-5 text-center">
      {query ? (
        <Search
          size={24}
          className="mb-3 text-muted-foreground"
          aria-hidden="true"
        />
      ) : (
        <FolderOpen
          size={24}
          className="mb-3 text-muted-foreground"
          aria-hidden="true"
        />
      )}
      <h3 className="text-body font-semibold">
        {query ? "No projects match" : "No projects remembered yet"}
      </h3>
      <p className="mt-1 max-w-md text-meta text-muted-foreground">
        {query
          ? "Try the project name, folder, vault, or readiness state."
          : "Choose one coding project explicitly. Ley will not search your device for repositories."}
      </p>
      {!query && (
        <Button variant="primary" className="mt-4" onClick={onAdd}>
          <FolderOpen size={14} />
          Add first project
        </Button>
      )}
    </div>
  );
}

function projectStatus(state: AgentProjectCatalogState): {
  label: string;
  icon: typeof CheckCircle2;
  className: string;
} {
  switch (state) {
    case "ready":
      return {
        label: "Ready",
        icon: CheckCircle2,
        className: "bg-emerald-500/10 text-emerald-500",
      };
    case "unbound":
      return {
        label: "Connect vault",
        icon: Unplug,
        className: "bg-warning/10 text-warning",
      };
    case "needs-capture":
      return {
        label: "Capture needed",
        icon: RefreshCw,
        className: "bg-warning/10 text-warning",
      };
    case "vault-unavailable":
      return {
        label: "Vault unavailable",
        icon: Unplug,
        className: "bg-destructive/10 text-destructive",
      };
    case "project-unavailable":
      return {
        label: "Folder unavailable",
        icon: FolderOpen,
        className: "bg-destructive/10 text-destructive",
      };
    case "identity-changed":
      return {
        label: "Identity changed",
        icon: ShieldCheck,
        className: "bg-destructive/10 text-destructive",
      };
    case "memory-error":
      return {
        label: "Needs repair",
        icon: AlertTriangle,
        className: "bg-destructive/10 text-destructive",
      };
  }
}

function relativeTime(timestamp: number): string {
  const delta = Math.max(0, Date.now() - timestamp);
  const minutes = Math.floor(delta / 60_000);
  if (minutes < 1) return "opened just now";
  if (minutes < 60) return `opened ${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `opened ${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `opened ${days}d ago`;
}

function humanize(value: string): string {
  return value
    .replaceAll("-", " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());
}
