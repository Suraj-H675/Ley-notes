import { useEffect, useState, type FormEvent } from "react";
import {
  AlertTriangle,
  ArrowRight,
  BookCheck,
  BrainCircuit,
  CheckCircle2,
  Download,
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
import {
  installSemanticModel,
  readSemanticModelSetup,
  searchAgentProjectMemory,
} from "./api";
import type {
  AgentProjectSearchResultKind,
  ProjectMemorySearch,
  ProjectMemorySearchResult,
  SemanticModelSetup,
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
  const [semanticSetup, setSemanticSetup] = useState<SemanticModelSetup | null>(
    null,
  );
  const [semanticInstalling, setSemanticInstalling] = useState(false);
  const [semanticError, setSemanticError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void readSemanticModelSetup()
      .then((setup) => {
        if (current) setSemanticSetup(setup);
      })
      .catch(() => {
        // Search remains fully functional in lexical mode when setup status is unavailable.
      });
    return () => {
      current = false;
    };
  }, []);

  async function enableSemanticSearch() {
    if (semanticInstalling) return;
    setSemanticInstalling(true);
    setSemanticError(null);
    try {
      await installSemanticModel();
      setSemanticSetup(await readSemanticModelSetup());
    } catch (cause) {
      setSemanticError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSemanticInstalling(false);
    }
  }

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

      <form onSubmit={(event) => void submit(event)} className="max-w-3xl">
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
        <div className="mt-2.5 flex items-center gap-2 px-1 text-micro text-muted-foreground">
          <BrainCircuit size={12} aria-hidden="true" />
          <span>
            Runs on this device. Captured text never leaves Ley.
            {semanticSetup?.status.state === "ready"
              ? " Hybrid retrieval is ready."
              : " Exact local search remains available."}
          </span>
        </div>
      </form>

      {semanticSetup && semanticSetup.status.state !== "ready" && (
        <section
          aria-labelledby="semantic-search-setup-title"
          className="max-w-3xl overflow-hidden rounded-sm border border-border bg-surface-1 shadow-sm"
        >
          <div className="flex flex-col gap-4 p-4 sm:flex-row sm:items-center sm:justify-between sm:p-5">
            <div className="flex min-w-0 items-start gap-3.5">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-sm border border-primary/25 bg-primary/7 text-primary">
                <BrainCircuit size={18} aria-hidden="true" />
              </div>
              <div className="min-w-0">
                <h2
                  id="semantic-search-setup-title"
                  className="text-body font-semibold text-foreground"
                >
                  {semanticSetup.status.state === "corrupt"
                    ? "Repair meaning-based search"
                    : "Find related ideas, even when the words differ"}
                </h2>
                <p className="mt-1 text-meta leading-relaxed text-muted-foreground">
                  Install Ley’s optional pinned local model. The one-time{" "}
                  {formatBytes(semanticSetup.totalBytes)} download comes from
                  Hugging Face only after you choose to install it; project text
                  and queries are never uploaded.
                </p>
                <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-micro text-muted-foreground">
                  <span>{shortModelName(semanticSetup.model.modelId)}</span>
                  <span aria-hidden="true">·</span>
                  <span>Verified before use</span>
                  <span aria-hidden="true">·</span>
                  <span>Inference stays on this device</span>
                </div>
              </div>
            </div>
            <Button
              type="button"
              className="shrink-0 self-start active:scale-[0.97] sm:self-center"
              disabled={semanticInstalling}
              onClick={() => void enableSemanticSearch()}
            >
              {semanticInstalling ? (
                <LoaderCircle
                  size={14}
                  className="animate-spin motion-reduce:animate-none"
                />
              ) : semanticSetup.status.state === "corrupt" ? (
                <CheckCircle2 size={14} aria-hidden="true" />
              ) : (
                <Download size={14} aria-hidden="true" />
              )}
              {semanticInstalling
                ? "Downloading & verifying…"
                : semanticSetup.status.state === "corrupt"
                  ? "Repair local model"
                  : "Enable semantic search"}
            </Button>
          </div>
          {semanticError && (
            <p
              role="alert"
              className="border-t border-destructive/20 bg-destructive/[0.045] px-5 py-3 text-meta text-destructive"
            >
              Installation did not complete. Exact local search is still
              available. {semanticError}
            </p>
          )}
        </section>
      )}

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

function formatBytes(bytes: number): string {
  if (bytes < 1_048_576) return `${Math.ceil(bytes / 1_024)} KB`;
  return `${Math.ceil(bytes / 1_048_576)} MiB`;
}

function shortModelName(modelId: string): string {
  return modelId.split("/").at(-1) ?? modelId;
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
          "group flex w-full items-start gap-3 rounded-sm border border-border bg-surface-1 p-4 transition-[border-color,background-color] duration-150 hover:border-primary/45 hover:bg-white/[0.03] motion-reduce:transition-none",
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
