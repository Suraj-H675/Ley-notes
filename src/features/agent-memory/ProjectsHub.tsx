import { useState } from "react";
import {
  AlertTriangle,
  ArrowUpRight,
  BookCheck,
  CheckCircle2,
  CircleDot,
  Ellipsis,
  Files,
  FolderOpen,
  GitCommitHorizontal,
  History,
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
import { humanize } from "./AgentMemoryPresentation";
import { searchAgentProjects } from "./api";
import type {
  AgentProjectCatalog,
  AgentProjectCatalogItem,
  AgentProjectCatalogState,
  AgentProjectSearch,
  AgentProjectSearchResult,
  AgentProjectSearchResultKind,
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
  onOpen: (projectPath: string, destination?: AgentProjectSearchResult) => void;
  onForget: (projectId: string) => void;
  onReload: () => void;
}) {
  const [filterQuery, setFilterQuery] = useState("");
  const [memoryQuery, setMemoryQuery] = useState("");
  const [search, setSearch] = useState<AgentProjectSearch | null>(null);
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const normalized = filterQuery.trim().toLocaleLowerCase();
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

  async function searchMemory() {
    const query = memoryQuery.trim();
    if (!query || searching) return;
    setSearching(true);
    setSearchError(null);
    try {
      setSearch(await searchAgentProjects(query));
    } catch (cause) {
      setSearchError(
        cause instanceof Error
          ? cause.message
          : typeof cause === "string"
            ? cause
            : "Cross-project search failed.",
      );
    } finally {
      setSearching(false);
    }
  }

  return (
    <ProjectsHubContent
      catalog={catalog}
      loading={loading}
      error={error}
      onAdd={onAdd}
      onOpen={onOpen}
      onForget={onForget}
      onReload={onReload}
      filterQuery={filterQuery}
      setFilterQuery={setFilterQuery}
      memoryQuery={memoryQuery}
      setMemoryQuery={setMemoryQuery}
      search={search}
      searching={searching}
      searchError={searchError}
      onSearch={searchMemory}
      projects={projects}
    />
  );
}

function ProjectsHubContent({
  catalog,
  loading,
  error,
  onAdd,
  onOpen,
  onForget,
  onReload,
  filterQuery,
  setFilterQuery,
  memoryQuery,
  setMemoryQuery,
  search,
  searching,
  searchError,
  onSearch,
  projects,
}: {
  catalog: AgentProjectCatalog | null;
  loading: boolean;
  error: string | null;
  onAdd: () => void;
  onOpen: (projectPath: string, destination?: AgentProjectSearchResult) => void;
  onForget: (projectId: string) => void;
  onReload: () => void;
  filterQuery: string;
  setFilterQuery: (value: string) => void;
  memoryQuery: string;
  setMemoryQuery: (value: string) => void;
  search: AgentProjectSearch | null;
  searching: boolean;
  searchError: string | null;
  onSearch: () => Promise<void>;
  projects: AgentProjectCatalogItem[];
}) {
  return (
    <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
      <div className="mx-auto w-full max-w-5xl px-4 py-6 sm:px-6 sm:py-8 lg:px-10">
        <header className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
              Projects
            </p>
            <h2 className="mt-1 text-2xl font-semibold tracking-[-0.035em] sm:text-3xl">
              Continue where you left off
            </h2>
            <p className="mt-2 max-w-2xl text-meta leading-6 text-muted-foreground">
              Ley remembers only projects you explicitly open. Search retained
              history or continue straight into a project.
            </p>
          </div>
          <Button variant="primary" onClick={onAdd} className="self-start sm:self-auto">
            <FolderOpen size={14} />
            Add project
          </Button>
        </header>

        <CrossProjectSearch
          memoryQuery={memoryQuery}
          setMemoryQuery={setMemoryQuery}
          searching={searching}
          searchError={searchError}
          onSearch={onSearch}
          search={search}
          onOpen={onOpen}
        />

        <section className="mt-8" aria-labelledby="known-projects-title">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h2
                id="known-projects-title"
                className="text-xl font-semibold tracking-tight"
              >
                Your projects
              </h2>
              <p className="mt-1 text-meta text-muted-foreground">
                {catalog
                  ? `${catalog.readyProjects} ready${catalog.attentionProjects > 0 ? ` · ${catalog.attentionProjects} need attention` : ""}`
                  : "Recent first · local device catalog only"}
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
                  value={filterQuery}
                  onChange={(event) => setFilterQuery(event.target.value)}
                  placeholder="Name, folder, vault, or status"
                  className="h-10 w-full rounded-md border border-border bg-surface-1 pl-9 pr-3 text-meta outline-none transition focus:border-primary/60 focus:ring-2 focus:ring-primary/20"
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
              className="mt-4 flex items-start gap-3 rounded-md border border-destructive/25 bg-destructive/8 p-4 text-meta"
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
              className="mt-4 overflow-hidden rounded-md border border-border bg-surface-1"
              aria-label="Loading local projects"
            >
              {[0, 1, 2].map((item) => (
                <div
                  key={item}
                  className="h-20 animate-pulse border-b border-border bg-surface-1/60 last:border-b-0"
                />
              ))}
            </div>
          ) : projects.length > 0 ? (
            <div className="mt-4 overflow-visible rounded-md border border-border bg-surface-1">
              {projects.map((project, index) => (
                <ProjectRow
                  key={project.projectId}
                  project={project}
                  divided={index > 0}
                  onOpen={() => onOpen(project.projectPath)}
                  onForget={() => onForget(project.projectId)}
                />
              ))}
            </div>
          ) : (
            <EmptyProjects query={filterQuery} onAdd={onAdd} />
          )}

          {catalog && catalog.omittedProjects > 0 && (
            <p className="mt-3 rounded-sm border border-dashed border-border bg-surface-1/45 px-4 py-3 text-center text-micro text-muted-foreground">
              {catalog.omittedProjects.toLocaleString()} older projects are
              omitted by the local response bound.
            </p>
          )}
        </section>

        <div className="mt-8 flex flex-col gap-2 border-t border-border pt-4 text-micro text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
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
    </div>
  );
}

function CrossProjectSearch({
  memoryQuery,
  setMemoryQuery,
  searching,
  searchError,
  onSearch,
  search,
  onOpen,
}: {
  memoryQuery: string;
  setMemoryQuery: (value: string) => void;
  searching: boolean;
  searchError: string | null;
  onSearch: () => Promise<void>;
  search: AgentProjectSearch | null;
  onOpen: (projectPath: string, destination?: AgentProjectSearchResult) => void;
}) {
  return (
    <section
      aria-labelledby="memory-search-title"
      className="mt-6"
    >
      <div>
        <div>
          <h2
            id="memory-search-title"
            className="text-body font-semibold tracking-tight"
          >
            Search across projects
          </h2>
          <p className="mt-0.5 text-meta text-muted-foreground">
            Sessions, decisions, problems, lessons, and captured files · local
            history only
          </p>
        </div>
        <form
          className="mt-4 flex flex-col gap-2 sm:flex-row"
          onSubmit={(event) => {
            event.preventDefault();
            void onSearch();
          }}
        >
          <label className="relative min-w-0 flex-1">
            <span className="sr-only">Search across project memory</span>
            <Search
              size={15}
              aria-hidden="true"
              className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
            />
            <input
              type="search"
              value={memoryQuery}
              maxLength={256}
              onChange={(event) => setMemoryQuery(event.target.value)}
              placeholder="What did we decide about offline sync?"
              className="h-11 w-full rounded-md border border-border bg-surface-1 pl-9 pr-3 text-meta outline-none transition focus:border-primary/60 focus:ring-2 focus:ring-primary/20"
            />
          </label>
          <Button
            type="submit"
            variant="primary"
            disabled={!memoryQuery.trim() || searching}
          >
            {searching ? (
              <RefreshCw
                size={14}
                className="animate-spin motion-reduce:animate-none"
              />
            ) : (
              <Search size={14} />
            )}
            {searching ? "Searching locally" : "Search memory"}
          </Button>
        </form>
        {searchError && (
          <p className="mt-2 text-meta text-destructive" role="alert">
            {searchError}
          </p>
        )}
      </div>
      {search && (
        <div className="mt-3 rounded-md border border-border bg-surface-1">
          <SearchResults
            search={search}
            onOpen={(result) => onOpen(result.projectPath, result)}
          />
        </div>
      )}
    </section>
  );
}

function SearchResults({
  search,
  onOpen,
}: {
  search: AgentProjectSearch;
  onOpen: (result: AgentProjectSearchResult) => void;
}) {
  return (
    <div className="p-4 sm:p-5" aria-live="polite">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-meta font-semibold">
          {search.results.length === 0
            ? `No captured memory matched “${search.query}”`
            : `${search.results.length} top ${search.results.length === 1 ? "result" : "results"} for “${search.query}”`}
        </p>
        <p className="text-micro text-muted-foreground">
          {search.searchedProjects} searched
          {search.skippedProjects > 0
            ? ` · ${search.skippedProjects} unavailable`
            : ""}
        </p>
      </div>
      {search.results.length > 0 && (
        <div className="mt-3 grid gap-2 lg:grid-cols-2">
          {search.results.map((result) => (
            <SearchResultCard
              key={`${result.projectId}:${result.kind}:${result.entityId}`}
              result={result}
              onOpen={() => onOpen(result)}
            />
          ))}
        </div>
      )}
      <div className="mt-3 flex flex-col gap-1 text-micro text-muted-foreground sm:flex-row sm:justify-between">
        <span>{search.instructionWarning}</span>
        <span className="shrink-0">
          {search.truncated
            ? "Top bounded matches shown"
            : "All matches in bound shown"}
        </span>
      </div>
    </div>
  );
}

function SearchResultCard({
  result,
  onOpen,
}: {
  result: AgentProjectSearchResult;
  onOpen: () => void;
}) {
  const meta = resultKindMeta(result.kind);
  const Icon = meta.icon;
  return (
    <button
      type="button"
      onClick={onOpen}
      className="group min-w-0 touch-manipulation overflow-hidden rounded-md border border-border bg-background/45 p-3.5 text-left outline-none transition-[transform,border-color,background-color] hover:border-primary/35 hover:bg-surface-2 active:scale-[0.99] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <span
            className={cn(
              "flex size-7 shrink-0 items-center justify-center rounded-md",
              meta.className,
            )}
          >
            <Icon size={13} aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <p className="truncate text-meta font-semibold">{result.title}</p>
            <p className="mt-0.5 truncate text-micro text-muted-foreground">
              {result.projectName} · {meta.label}
            </p>
          </div>
        </div>
        <ArrowUpRight
          size={13}
          className="shrink-0 text-muted-foreground transition group-hover:text-primary"
          aria-hidden="true"
        />
      </div>
      <p className="mt-2 line-clamp-2 text-meta leading-5 text-muted-foreground-strong">
        {result.excerpt}
      </p>
      <div className="mt-2 flex flex-wrap items-center gap-2 text-micro text-muted-foreground">
        {result.citation && (
          <span className="truncate font-mono">
            {result.citation.artifactPath}:{result.citation.startLine}
          </span>
        )}
        {result.trustState && <span>{humanize(result.trustState)}</span>}
        {result.freshness && <span>{humanize(result.freshness)}</span>}
      </div>
    </button>
  );
}

function resultKindMeta(kind: AgentProjectSearchResultKind) {
  switch (kind) {
    case "session":
      return {
        label: "Session",
        icon: History,
        className: "bg-sky-500/10 text-sky-500",
      };
    case "revision":
      return {
        label: "Captured revision",
        icon: GitCommitHorizontal,
        className: "bg-indigo-500/10 text-indigo-500",
      };
    case "decision":
      return {
        label: "Decision",
        icon: CheckCircle2,
        className: "bg-violet-500/10 text-violet-500",
      };
    case "problem":
      return {
        label: "Problem & outcome",
        icon: AlertTriangle,
        className: "bg-warning/10 text-warning",
      };
    case "learning":
      return {
        label: "Lesson",
        icon: BookCheck,
        className: "bg-success/10 text-success",
      };
    case "artifact":
      return {
        label: "Artifact",
        icon: Files,
        className: "bg-primary/10 text-primary",
      };
    case "symbol":
      return {
        label: "Symbol",
        icon: CircleDot,
        className: "bg-fuchsia-500/10 text-fuchsia-500",
      };
    case "dependency":
      return {
        label: "Dependency",
        icon: Network,
        className: "bg-cyan-500/10 text-cyan-500",
      };
  }
}

function ProjectRow({
  project,
  divided,
  onOpen,
  onForget,
}: {
  project: AgentProjectCatalogItem;
  divided: boolean;
  onOpen: () => void;
  onForget: () => void;
}) {
  const [confirmingRemoval, setConfirmingRemoval] = useState(false);
  const unavailable =
    project.state === "project-unavailable" ||
    project.state === "identity-changed";
  const status = projectStatus(project.state);
  const StatusIcon = status.icon;
  return (
    <article
      className={cn(
        "group relative flex items-stretch",
        divided && "border-t border-border",
      )}
    >
      <button
        type="button"
        onClick={onOpen}
        disabled={unavailable}
        className="flex min-w-0 flex-1 items-center gap-3 px-4 py-3.5 text-left outline-none hover:bg-surface-2/60 focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary disabled:cursor-default disabled:hover:bg-transparent sm:px-5"
      >
        <span className="flex size-8 shrink-0 items-center justify-center rounded-md border border-border bg-background/45 text-muted-foreground">
          <FolderOpen size={15} aria-hidden="true" />
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex min-w-0 items-center gap-2">
            <span className="truncate text-meta font-semibold text-foreground">
              {project.projectName}
            </span>
            <span
              className={cn(
                "inline-flex shrink-0 items-center gap-1 rounded-sm px-1.5 py-0.5 text-micro font-medium",
                status.className,
              )}
            >
              <StatusIcon size={10} aria-hidden="true" />
              {status.label}
            </span>
          </span>
          <span
            className="mt-1 block truncate font-mono text-micro text-muted-foreground"
            title={project.projectPath}
          >
            {project.projectPath}
          </span>
          <span className="mt-1 block text-micro text-muted-foreground">
            {project.state === "ready"
              ? [
                  `${project.sessions ?? 0} sessions`,
                  `${project.files ?? 0} files`,
                  project.reviewItems
                    ? `${project.reviewItems} to review`
                    : null,
                  relativeTime(project.lastOpenedAtUnixMs),
                ]
                  .filter(Boolean)
                  .join(" · ")
              : project.statusDetail}
          </span>
        </span>
        {!unavailable && (
          <ArrowUpRight
            size={14}
            className="shrink-0 text-subtle-foreground transition group-hover:text-primary"
            aria-hidden="true"
          />
        )}
      </button>
      <details className="relative flex shrink-0 items-center border-l border-border/70">
        <summary
          className="flex h-full min-h-16 w-11 list-none cursor-pointer items-center justify-center text-muted-foreground outline-none hover:bg-surface-2 hover:text-foreground focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary [&::-webkit-details-marker]:hidden"
          aria-label={`Actions for ${project.projectName}`}
          title={`Actions for ${project.projectName}`}
        >
          <Ellipsis size={15} aria-hidden="true" />
        </summary>
        <div className="absolute right-1 top-full z-20 mt-1 w-56 rounded-md border border-border bg-surface-2 p-1.5 shadow-panel">
          {confirmingRemoval ? (
            <div className="p-1">
              <p className="text-meta font-medium text-foreground">
                Remove {project.projectName} from Projects?
              </p>
              <p className="mt-1 text-micro leading-4 text-muted-foreground">
                This only removes the device-list entry. Ley memory is not
                erased.
              </p>
              <div className="mt-3 flex gap-2">
                <button
                  type="button"
                  onClick={() => {
                    setConfirmingRemoval(false);
                    onForget();
                  }}
                  className="rounded bg-destructive/12 px-2 py-1.5 text-micro font-semibold text-destructive outline-none hover:bg-destructive/18 focus-visible:ring-2 focus-visible:ring-destructive"
                >
                  Confirm removal
                </button>
                <button
                  type="button"
                  onClick={() => setConfirmingRemoval(false)}
                  className="rounded px-2 py-1.5 text-micro font-medium text-muted-foreground outline-none hover:bg-surface-3 hover:text-foreground focus-visible:ring-2 focus-visible:ring-primary"
                >
                  Cancel
                </button>
              </div>
            </div>
          ) : (
            <button
              type="button"
              onClick={() => setConfirmingRemoval(true)}
              className="flex w-full items-center gap-2 rounded px-2 py-2 text-left text-meta text-muted-foreground outline-none hover:bg-surface-3 hover:text-foreground focus-visible:ring-2 focus-visible:ring-primary"
            >
              <X size={13} aria-hidden="true" />
              <span>
                <span className="block font-medium">Remove from Projects</span>
                <span className="mt-0.5 block text-micro text-subtle-foreground">
                  Ley memory is not deleted
                </span>
              </span>
            </button>
          )}
        </div>
      </details>
    </article>
  );
}

function EmptyProjects({ query, onAdd }: { query: string; onAdd: () => void }) {
  return (
    <div className="mt-4 flex min-h-72 flex-col items-center justify-center rounded-sm border border-dashed border-border bg-surface-1/45 px-5 text-center">
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
        className: "bg-success/10 text-success",
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
