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
      setSearch(await searchAgentProjectMemory(projectPath, value));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="memory-search-title" className="space-y-6">
      <div className="max-w-3xl">
        <div className="mb-3 flex size-10 items-center justify-center rounded-xl border border-primary/20 bg-primary/10 text-primary shadow-sm">
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

      <form onSubmit={(event) => void submit(event)} className="max-w-3xl">
        <div className="group flex min-h-14 items-center gap-3 rounded-2xl border border-border bg-surface-1 px-4 shadow-sm transition-[border-color,box-shadow,transform] duration-200 focus-within:border-primary/50 focus-within:shadow-[0_10px_35px_-18px_hsl(var(--primary)/0.45)] motion-reduce:transition-none">
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
        <div className="mt-2.5 flex items-center gap-2 px-1 text-micro text-muted-foreground">
          <BrainCircuit size={12} aria-hidden="true" />
          <span>Runs on this device. Captured text never leaves Ley.</span>
        </div>
      </form>

      {error && (
        <div className="max-w-3xl rounded-xl border border-destructive/25 bg-destructive/8 p-4 text-meta text-destructive">
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
              className="rounded-xl border border-border bg-surface-1 px-4 py-3 text-left text-meta text-muted-foreground transition-[background-color,color,transform] duration-150 hover:bg-surface-2 hover:text-foreground active:scale-[0.99] motion-reduce:transition-none"
            >
              {suggestion}
            </button>
          ))}
        </div>
      )}

      {search && (
        <div className="max-w-4xl space-y-4" aria-live="polite">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-meta font-medium text-foreground">
              {search.results.length === 0
                ? "No matching captured memory"
                : `${search.results.length} relevant ${search.results.length === 1 ? "memory" : "memories"}`}
            </p>
            <div className="flex items-center gap-2 text-micro text-muted-foreground">
              <span className="rounded-full border border-border bg-surface-1 px-2 py-1 capitalize">
                {search.retrieval.mode}
              </span>
              <span>captured snapshot</span>
            </div>
          </div>

          {search.conflicts.length > 0 && (
            <div className="rounded-xl border border-warning/25 bg-warning/8 p-4">
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

          {(search.truncated ||
            search.retrieval.boundedRerankFallbackReason) && (
            <p className="px-1 text-micro leading-relaxed text-muted-foreground">
              {search.retrieval.boundedRerankFallbackReason
                ? `Semantic ranking unavailable: ${search.retrieval.boundedRerankFallbackReason}. Exact local search still ran.`
                : "Results were bounded to keep agent context focused. Refine the question for a narrower answer."}
            </p>
          )}
        </div>
      )}
    </section>
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
        "group flex w-full items-start gap-3 rounded-2xl border border-border bg-surface-1 p-4 text-left shadow-[0_1px_0_hsl(var(--foreground)/0.03)] transition-[border-color,background-color,transform,box-shadow] duration-150 motion-reduce:transition-none",
        actionable &&
          "hover:border-primary/25 hover:bg-surface-2 hover:shadow-sm active:scale-[0.995]",
      )}
    >
      <div className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg bg-surface-3 text-muted-foreground group-hover:text-primary">
        <Icon size={15} aria-hidden="true" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-meta font-semibold text-foreground">
            {result.title}
          </span>
          <span className="rounded-full bg-surface-3 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
            {result.kind}
          </span>
          {result.trustSignal && result.trustSignal !== "direct-evidence" && (
            <span
              className={cn(
                "rounded-full px-1.5 py-0.5 text-[10px] font-medium capitalize",
                result.trustedForReuse
                  ? "bg-success/10 text-success"
                  : "bg-warning/10 text-warning",
              )}
            >
              {result.trustSignal.replace("-", " ")}
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
