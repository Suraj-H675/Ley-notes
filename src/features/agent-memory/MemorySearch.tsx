import { useState, type FormEvent } from "react";
import {
  AlertTriangle,
  ArrowRight,
  BookCheck,
  BrainCircuit,
  FileCode2,
  GitBranch,
  History,
  LoaderCircle,
  MessageSquareWarning,
  Search,
  Sparkles,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import { searchAgentProjectMemory } from "./api";
import type {
  AgentProjectSearchResultKind,
  ProjectMemorySearch,
  ProjectMemorySearchResult,
  RevisionCompatibility,
} from "./types";

const RESULT_ICONS = {
  session: History,
  revision: GitBranch,
  decision: Sparkles,
  problem: MessageSquareWarning,
  learning: BookCheck,
  artifact: FileCode2,
  symbol: FileCode2,
  dependency: FileCode2,
} satisfies Record<AgentProjectSearchResultKind, typeof History>;

export function MemorySearch({
  projectPath,
  projectName,
  onOpen,
}: {
  projectPath: string;
  projectName: string;
  onOpen: (result: ProjectMemorySearchResult) => void;
}) {
  const [query, setQuery] = useState("");
  const [revisionFilter, setRevisionFilter] = useState<
    RevisionCompatibility | "all"
  >("all");
  const [search, setSearch] = useState<ProjectMemorySearch | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    const value = query.trim();
    if (!value || busy) return;
    setBusy(true);
    setError(null);
    try {
      setSearch(
        await searchAgentProjectMemory(
          projectPath,
          value,
          revisionFilter === "all" ? undefined : revisionFilter,
        ),
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <MemorySearchContent
      projectName={projectName}
      query={query}
      setQuery={setQuery}
      revisionFilter={revisionFilter}
      setRevisionFilter={setRevisionFilter}
      search={search}
      busy={busy}
      error={error}
      onSubmit={submit}
      onOpen={onOpen}
    />
  );
}

function MemorySearchContent({
  projectName,
  query,
  setQuery,
  revisionFilter,
  setRevisionFilter,
  search,
  busy,
  error,
  onSubmit,
  onOpen,
}: {
  projectName: string;
  query: string;
  setQuery: (value: string) => void;
  revisionFilter: RevisionCompatibility | "all";
  setRevisionFilter: (value: RevisionCompatibility | "all") => void;
  search: ProjectMemorySearch | null;
  busy: boolean;
  error: string | null;
  onSubmit: (event: FormEvent) => Promise<void>;
  onOpen: (result: ProjectMemorySearchResult) => void;
}) {
  return (
    <section aria-labelledby="memory-search-title" className="space-y-6">
      <div className="max-w-3xl">
        <div className="mb-3 flex size-9 items-center justify-center rounded-sm border border-primary/30 bg-primary/8 text-primary">
          <Sparkles size={19} aria-hidden="true" />
        </div>
        <h1
          id="memory-search-title"
          className="text-lg font-semibold tracking-tight text-foreground"
        >
          Ask your project memory
        </h1>
        <p className="mt-1.5 max-w-2xl text-body leading-relaxed text-muted-foreground">
          Find the session, decision, failed attempt, lesson, or captured file
          that explains what {projectName} already knows.
        </p>
      </div>

      <form onSubmit={(event) => void onSubmit(event)} className="max-w-3xl">
        <div className="group flex min-h-14 items-center gap-3 rounded-sm border border-border bg-surface-1 px-4 transition-colors duration-150 focus-within:border-primary/60 motion-reduce:transition-none">
          <Search
            size={18}
            className="shrink-0 text-muted-foreground group-focus-within:text-primary"
            aria-hidden="true"
          />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Why did we choose this architecture?"
            aria-label="Search this project’s Agent Memory"
            maxLength={256}
            className="min-w-0 flex-1 bg-transparent py-3 text-body text-foreground outline-none placeholder:text-muted-foreground/70"
          />
          <Button
            type="submit"
            size="sm"
            disabled={!query.trim() || busy}
            className="shrink-0 active:scale-[0.97]"
          >
            {busy ? (
              <LoaderCircle
                size={14}
                className="animate-spin motion-reduce:animate-none"
              />
            ) : (
              <ArrowRight size={14} />
            )}
            <span className="hidden sm:inline">Search</span>
          </Button>
        </div>
        <div className="mt-2.5 flex flex-wrap items-center justify-between gap-2 px-1">
          <div className="flex items-center gap-2 text-micro text-muted-foreground">
            <BrainCircuit size={12} aria-hidden="true" />
            <span>Runs on this device. Captured text never leaves Ley.</span>
          </div>
          <label className="flex items-center gap-2 text-micro text-muted-foreground">
            <GitBranch size={12} aria-hidden="true" />
            <span>Revision scope</span>
            <select
              aria-label="Filter project memory by revision compatibility"
              value={revisionFilter}
              onChange={(event) =>
                setRevisionFilter(
                  event.target.value as RevisionCompatibility | "all",
                )
              }
              className="rounded-sm border border-border bg-surface-1 px-2 py-1 text-micro text-foreground outline-none focus-visible:ring-2 focus-visible:ring-primary"
            >
              <option value="all">All captured history</option>
              <option value="current-lineage">Current lineage</option>
              <option value="ancestor">Ancestor</option>
              <option value="merged">Merged</option>
              <option value="divergent">Divergent</option>
              <option value="unknown">Unknown</option>
            </select>
          </label>
        </div>
      </form>

      {error && (
        <div className="max-w-3xl rounded-md border border-destructive/25 bg-destructive/8 p-4 text-meta text-destructive">
          {error}
        </div>
      )}

      {!search && !busy && (
        <div className="grid max-w-3xl gap-2 sm:grid-cols-2">
          {[
            "What did we try that failed?",
            "Which decisions still shape this project?",
            "How does the core architecture work?",
            "What should the next agent remember?",
          ].map((suggestion) => (
            <button
              key={suggestion}
              type="button"
              onClick={() => setQuery(suggestion)}
              className="rounded-md border border-border bg-surface-1 px-4 py-3 text-left text-meta text-muted-foreground transition-[background-color,color,transform] duration-150 hover:bg-surface-2 hover:text-foreground active:scale-[0.99] motion-reduce:transition-none"
            >
              {suggestion}
            </button>
          ))}
        </div>
      )}

      <MemorySearchResults search={search} onOpen={onOpen} />
    </section>
  );
}

function MemorySearchResults({
  search,
  onOpen,
}: {
  search: ProjectMemorySearch | null;
  onOpen: (result: ProjectMemorySearchResult) => void;
}) {
  if (!search) return null;
  return (
    <div className="max-w-4xl space-y-4" aria-live="polite">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-meta font-medium text-foreground">
          {search.results.length === 0
            ? "No matching captured memory"
            : `${search.results.length} relevant ${search.results.length === 1 ? "memory" : "memories"}`}
        </p>
        <div className="flex items-center gap-2 text-micro text-muted-foreground">
          <span className="rounded-sm border border-border bg-surface-1 px-1.5 py-0.5 capitalize">
            {search.retrieval.mode}
          </span>
          <span>captured snapshot</span>
          {search.revisionFilter && (
            <span className="rounded-sm border border-border bg-surface-1 px-1.5 py-0.5">
              {compatibilityLabel(search.revisionFilter)} only
            </span>
          )}
        </div>
      </div>

      <p className="text-micro leading-relaxed text-muted-foreground">
        {search.revisionFreshness.liveGitChecked
          ? `Current Git: ${search.revisionFreshness.currentBranch ?? "detached HEAD"}${search.revisionFreshness.currentHead ? ` · ${search.revisionFreshness.currentHead.slice(0, 10)}` : ""}. Git metadata only; live files were not checked.`
          : "Current Git metadata was unavailable. Live files were not checked."}
      </p>

      {search.conflicts.length > 0 && (
        <div className="rounded-md border border-warning/25 bg-warning/8 p-4">
          <div className="flex items-center gap-2 text-meta font-semibold text-foreground">
            <AlertTriangle size={15} className="text-warning" />
            Memory needs interpretation
          </div>
          {search.conflicts.map((conflict, index) => (
            <p
              key={`${conflict.kind}-${index}`}
              className="mt-1.5 text-meta text-muted-foreground"
            >
              {conflict.reason}
            </p>
          ))}
        </div>
      )}

      <div className="space-y-2">
        {search.results.map((result) => (
          <MemoryResult
            key={`${result.kind}:${result.entityId}`}
            result={result}
            onOpen={() => onOpen(result)}
          />
        ))}
      </div>

      {search.truncated && (
        <p className="px-1 text-micro leading-relaxed text-muted-foreground">
          Results were bounded to keep agent context focused. Refine the
          question for a narrower answer.
        </p>
      )}
    </div>
  );
}

function MemoryResult({
  result,
  onOpen,
}: {
  result: ProjectMemorySearchResult;
  onOpen: () => void;
}) {
  const Icon = RESULT_ICONS[result.kind];
  const actionable = Boolean(
    result.sessionId || result.learningId || result.citation,
  );
  return (
    <button
      type="button"
      disabled={!actionable}
      onClick={onOpen}
      className={cn(
        "group flex w-full items-start gap-3 rounded-sm border border-border bg-surface-1 p-4 transition-[border-color,background-color] duration-150 hover:border-primary/45 hover:bg-surface-2/70 motion-reduce:transition-none",
        actionable &&
          "hover:border-primary/25 hover:bg-surface-2 hover:shadow-sm active:scale-[0.995]",
      )}
    >
      <div className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-sm bg-surface-3 text-muted-foreground group-hover:text-primary">
        <Icon size={15} aria-hidden="true" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-meta font-semibold text-foreground">
            {result.title}
          </span>
          <span className="rounded-sm bg-surface-3/70 px-1 py-0.5 text-micro font-medium uppercase tracking-[0.08em] text-subtle-foreground">
            {result.kind}
          </span>
          {result.trustSignal && result.trustSignal !== "direct-evidence" && (
            <span
              className={cn(
                "rounded-sm px-1.5 py-0.5 text-micro font-medium capitalize",
                result.trustedForReuse
                  ? "bg-success/10 text-success"
                  : "bg-warning/10 text-warning",
              )}
            >
              {result.trustSignal.replace("-", " ")}
            </span>
          )}
          {result.revisionApplicability && (
            <span
              className={cn(
                "rounded-sm px-1.5 py-0.5 text-micro font-medium",
                result.revisionApplicability.compatibility === "divergent" ||
                  result.revisionApplicability.compatibility === "unknown"
                  ? "bg-warning/10 text-warning"
                  : "bg-surface-3 text-muted-foreground",
              )}
            >
              {compatibilityLabel(
                result.revisionApplicability.compatibility,
              )}
            </span>
          )}
        </div>
        <p className="mt-1 line-clamp-3 text-meta leading-relaxed text-muted-foreground">
          {result.excerpt}
        </p>
      </div>
      {actionable && (
        <ArrowRight
          size={14}
          className="mt-2 shrink-0 text-muted-foreground/60 transition-transform duration-150 group-hover:translate-x-0.5 group-hover:text-primary motion-reduce:transition-none"
          aria-hidden="true"
        />
      )}
    </button>
  );
}

function compatibilityLabel(value: RevisionCompatibility): string {
  switch (value) {
    case "current-lineage":
      return "Current lineage";
    case "ancestor":
      return "Ancestor";
    case "merged":
      return "Merged";
    case "divergent":
      return "Divergent";
    case "unknown":
      return "Unknown";
  }
}
