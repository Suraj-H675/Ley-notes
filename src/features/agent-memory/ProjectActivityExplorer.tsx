import { useEffect, useState } from "react";
import {
  AlertTriangle,
  ArrowUpRight,
  CheckCircle2,
  ChevronDown,
  CircleAlert,
  Clock3,
  FileCode2,
  GitCommitHorizontal,
  Lightbulb,
  Search,
  ShieldCheck,
  Signpost,
  Split,
  XCircle,
} from "lucide-react";
import { cn } from "@/shared/lib/classnames";
import { readAgentProjectActivity } from "./api";
import type {
  ArtifactEvidenceReference,
  ProjectActivityCitation,
  ProjectActivityView,
  ProjectDecision,
  ProjectProblem,
  ProjectProblemAttempt,
  ProjectProblemScope,
} from "./types";

type ActivityMode = "decisions" | "problems";

export function ProjectActivityExplorer({
  mode,
  projectPath,
  onSession,
  onEvidence = () => undefined,
}: {
  mode: ActivityMode;
  projectPath: string;
  onSession: (sessionId: string) => void;
  onEvidence?: (evidence: ArtifactEvidenceReference) => void;
}) {
  const [query, setQuery] = useState("");
  const [deferredQuery, setDeferredQuery] = useState("");
  const [scope, setScope] = useState<ProjectProblemScope>("all");
  const [activity, setActivity] = useState<ProjectActivityView | null>(null);
  const [completedRequest, setCompletedRequest] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const requestKey = `${projectPath}\u0000${deferredQuery}\u0000${scope}`;
  const loading = completedRequest !== requestKey;
  const visibleActivity = loading ? null : activity;

  useEffect(() => {
    const timer = window.setTimeout(() => setDeferredQuery(query), 220);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    let current = true;
    void readAgentProjectActivity(projectPath, deferredQuery, scope)
      .then((next) => {
        if (current) {
          setActivity(next);
          setError(null);
        }
      })
      .catch((cause) => {
        if (current)
          setError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (current) setCompletedRequest(requestKey);
      });
    return () => {
      current = false;
    };
  }, [deferredQuery, projectPath, requestKey, scope]);

  const isDecisions = mode === "decisions";
  const shown = isDecisions
    ? (visibleActivity?.decisions.length ?? 0)
    : (visibleActivity?.problems.length ?? 0);
  const matching = isDecisions
    ? (visibleActivity?.totalMatchingDecisions ?? 0)
    : (visibleActivity?.totalMatchingProblems ?? 0);

  return (
    <section className="space-y-5" aria-labelledby={`${mode}-activity-title`}>
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <p className="mb-1 text-micro font-semibold uppercase tracking-[0.14em] text-primary">
            {isDecisions ? "Project direction" : "Evidence-backed recovery"}
          </p>
          <h2
            id={`${mode}-activity-title`}
            className="text-xl font-semibold tracking-tight"
          >
            {isDecisions ? "Decisions" : "Problems & outcomes"}
          </h2>
          <p className="mt-1 max-w-2xl text-meta leading-relaxed text-muted-foreground">
            {isDecisions
              ? "Review what agents chose, why they chose it, and which alternatives they left behind."
              : "Trace symptoms through attempted fixes to verified resolutions without losing the failures in between."}
          </p>
        </div>
        <label className="relative block w-full lg:w-80">
          <span className="sr-only">
            Search {isDecisions ? "decisions" : "problems and outcomes"}
          </span>
          <Search
            aria-hidden="true"
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
            size={15}
          />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={
              isDecisions
                ? "Decision, rationale, or session"
                : "Symptom, attempt, resolution"
            }
            className="h-10 w-full rounded-md border border-border bg-surface-1 pl-9 pr-3 text-meta outline-none transition focus:border-primary/60 focus:ring-2 focus:ring-primary/20"
          />
        </label>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border bg-surface-1 p-2">
        {isDecisions ? (
          <p className="px-2 text-meta font-medium">Newest decisions first</p>
        ) : (
          <div
            className="flex gap-1"
            role="tablist"
            aria-label="Problem resolution state"
          >
            {(["all", "open", "resolved"] as const).map((value) => (
              <ScopeTab
                key={value}
                value={value}
                active={scope === value}
                onClick={() => setScope(value)}
              />
            ))}
          </div>
        )}
        <p className="px-2 text-micro text-muted-foreground" aria-live="polite">
          {loading
            ? "Reading local session evidence…"
            : `Showing ${shown} of ${matching} matching`}
        </p>
      </div>

      {error && !loading ? (
        <ErrorState mode={mode} message={error} />
      ) : loading ? (
        <ActivitySkeleton />
      ) : isDecisions ? (
        <DecisionList
          activity={visibleActivity}
          onSession={onSession}
          onEvidence={onEvidence}
          query={deferredQuery}
        />
      ) : (
        <ProblemList
          activity={visibleActivity}
          onSession={onSession}
          onEvidence={onEvidence}
          query={deferredQuery}
          scope={scope}
        />
      )}

      {visibleActivity && (
        <div className="flex flex-col gap-2 rounded-md border border-border/80 bg-surface-1 px-4 py-3 text-micro text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <span className="inline-flex items-center gap-2">
            <ShieldCheck
              size={14}
              className="text-primary"
              aria-hidden="true"
            />
            Structured records across{" "}
            {visibleActivity.totalSessions.toLocaleString()}{" "}
            {visibleActivity.totalSessions === 1 ? "session" : "sessions"}
          </span>
          <span>Stored evidence · live project source not checked</span>
        </div>
      )}
    </section>
  );
}

function DecisionList({
  activity,
  onSession,
  onEvidence,
  query,
}: {
  activity: ProjectActivityView | null;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  query: string;
}) {
  if (!activity || activity.decisions.length === 0) {
    return (
      <EmptyState
        icon={Lightbulb}
        title={query ? "No decisions match" : "No decisions captured yet"}
        detail={
          query
            ? "Try a rationale, alternative, session name, or broader phrase."
            : "Structured decisions recorded at agent checkpoints will appear here."
        }
      />
    );
  }
  return (
    <div className="space-y-3">
      {activity.decisions.map((decision) => (
        <DecisionCard
          key={`${decision.sessionId}:${decision.recordId}`}
          decision={decision}
          onSession={onSession}
          onEvidence={onEvidence}
        />
      ))}
      {activity.omittedDecisions > 0 && (
        <OmissionNotice count={activity.omittedDecisions} noun="decisions" />
      )}
    </div>
  );
}

function DecisionCard({
  decision,
  onSession,
  onEvidence,
}: {
  decision: ProjectDecision;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  return (
    <article className="overflow-hidden rounded-md border border-border bg-surface-1 shadow-panel">
      <div className="p-4 sm:p-5">
        <ActivityMeta
          sessionName={decision.sessionName}
          sessionStatus={decision.sessionStatus}
          recordedAt={decision.recordedAtUnixMs}
          onSession={() => onSession(decision.sessionId)}
        />
        <div className="mt-4 flex items-start gap-3">
          <span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
            <Signpost size={16} aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <h3 className="text-body font-semibold">{decision.title}</h3>
            <p className="mt-1 whitespace-pre-wrap text-meta leading-6 text-muted-foreground-strong">
              {decision.decision}
            </p>
          </div>
        </div>
      </div>
      <details className="group border-t border-border/70">
        <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 text-meta font-medium text-muted-foreground outline-none hover:bg-surface-2 hover:text-foreground focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary sm:px-5">
          <span className="inline-flex items-center gap-2">
            <GitCommitHorizontal size={14} aria-hidden="true" />
            Rationale & evidence
          </span>
          <ChevronDown
            size={14}
            aria-hidden="true"
            className="transition-transform group-open:rotate-180"
          />
        </summary>
        <div className="space-y-4 bg-surface-2/55 px-4 py-4 sm:px-5">
          {decision.rationale && (
            <DetailBlock label="Rationale" value={decision.rationale} />
          )}
          {decision.alternatives.length > 0 && (
            <div>
              <p className="text-micro font-semibold uppercase tracking-wider text-muted-foreground">
                Alternatives considered
              </p>
              <ul className="mt-2 space-y-1.5 text-meta">
                {decision.alternatives.map((alternative, index) => (
                  <li key={index} className="flex items-start gap-2">
                    <Split
                      size={13}
                      className="mt-1 shrink-0 text-muted-foreground"
                      aria-hidden="true"
                    />
                    <span>{alternative}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}
          <CitationList
            citations={decision.artifactCitations}
            onEvidence={onEvidence}
          />
          {(decision.detailTruncated ||
            decision.omittedAlternatives > 0 ||
            decision.omittedArtifactCitations > 0) && (
            <TruncationNotice onSession={() => onSession(decision.sessionId)} />
          )}
        </div>
      </details>
    </article>
  );
}

function ProblemList({
  activity,
  onSession,
  onEvidence,
  query,
  scope,
}: {
  activity: ProjectActivityView | null;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  query: string;
  scope: ProjectProblemScope;
}) {
  if (!activity || activity.problems.length === 0) {
    return (
      <EmptyState
        icon={CircleAlert}
        title={
          query
            ? "No problems match"
            : scope === "open"
              ? "No open problems"
              : scope === "resolved"
                ? "No resolved problems"
                : "No problems captured yet"
        }
        detail={
          query
            ? "Try a symptom, attempted action, evidence note, or resolution."
            : "Structured problems and their attempted outcomes will appear here."
        }
      />
    );
  }
  return (
    <div className="space-y-3">
      {activity.problems.map((problem) => (
        <ProblemCard
          key={`${problem.sessionId}:${problem.recordId}`}
          problem={problem}
          onSession={onSession}
          onEvidence={onEvidence}
        />
      ))}
      {activity.omittedProblems > 0 && (
        <OmissionNotice count={activity.omittedProblems} noun="problems" />
      )}
    </div>
  );
}

function ProblemCard({
  problem,
  onSession,
  onEvidence,
}: {
  problem: ProjectProblem;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  const resolved = Boolean(problem.resolution);
  return (
    <article className="overflow-hidden rounded-md border border-border bg-surface-1 shadow-panel">
      <div className="p-4 sm:p-5">
        <ActivityMeta
          sessionName={problem.sessionName}
          sessionStatus={problem.sessionStatus}
          recordedAt={problem.recordedAtUnixMs}
          onSession={() => onSession(problem.sessionId)}
        />
        <div className="mt-4 flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-body font-semibold">{problem.title}</h3>
              <span
                className={cn(
                  "rounded-sm px-2 py-0.5 text-micro font-semibold",
                  resolved
                    ? "bg-success/10 text-success"
                    : "bg-warning/10 text-warning",
                )}
              >
                {resolved ? "Resolved" : "Open"}
              </span>
            </div>
            <p className="mt-1 whitespace-pre-wrap text-meta leading-6 text-muted-foreground-strong">
              {problem.symptom}
            </p>
          </div>
          {problem.latestAttemptOutcome && (
            <OutcomeBadge outcome={problem.latestAttemptOutcome} />
          )}
        </div>
        {problem.resolution && (
          <div className="mt-4 rounded-md border border-success/20 bg-success/7 p-3">
            <p className="flex items-center gap-2 text-micro font-semibold uppercase tracking-wider text-success">
              <CheckCircle2 size={13} aria-hidden="true" />
              Resolution
            </p>
            <p className="mt-1 whitespace-pre-wrap text-meta leading-6">
              {problem.resolution.change}
            </p>
          </div>
        )}
      </div>
      <details className="group border-t border-border/70">
        <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-4 py-3 text-meta font-medium text-muted-foreground outline-none hover:bg-surface-2 hover:text-foreground focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary sm:px-5">
          <span className="inline-flex items-center gap-2">
            <Clock3 size={14} aria-hidden="true" />
            {problem.totalAttempts}{" "}
            {problem.totalAttempts === 1 ? "attempt" : "attempts"} & evidence
          </span>
          <ChevronDown
            size={14}
            aria-hidden="true"
            className="transition-transform group-open:rotate-180"
          />
        </summary>
        <div className="space-y-4 bg-surface-2/55 px-4 py-4 sm:px-5">
          {problem.expected && (
            <DetailBlock label="Expected" value={problem.expected} />
          )}
          {problem.attempts.length > 0 && (
            <ol className="space-y-3 border-l border-border pl-4">
              {problem.attempts.map((attempt, index) => (
                <AttemptRow key={attempt.id} attempt={attempt} index={index} />
              ))}
            </ol>
          )}
          {problem.resolution && (
            <div className="grid gap-3 rounded-md border border-border bg-background/45 p-3 sm:grid-cols-2">
              <DetailBlock
                label="Root cause"
                value={problem.resolution.rootCause}
              />
              <DetailBlock
                label="Verification"
                value={
                  problem.resolution.verification || "Not recorded explicitly"
                }
              />
            </div>
          )}
          <CitationList
            citations={problem.artifactCitations}
            onEvidence={onEvidence}
          />
          {(problem.detailTruncated ||
            problem.omittedAttempts > 0 ||
            problem.omittedArtifactCitations > 0) && (
            <TruncationNotice onSession={() => onSession(problem.sessionId)} />
          )}
        </div>
      </details>
    </article>
  );
}

function ActivityMeta({
  sessionName,
  sessionStatus,
  recordedAt,
  onSession,
}: {
  sessionName: string;
  sessionStatus: string;
  recordedAt: number;
  onSession: () => void;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-2 text-micro text-muted-foreground">
      <button
        type="button"
        onClick={onSession}
        className="inline-flex min-w-0 items-center gap-1.5 rounded text-left font-medium text-foreground outline-none hover:text-primary focus-visible:ring-2 focus-visible:ring-primary"
      >
        <span className="truncate">{sessionName}</span>
        <ArrowUpRight size={12} className="shrink-0" aria-hidden="true" />
      </button>
      <span className="inline-flex items-center gap-2">
        <span>{humanize(sessionStatus)}</span>
        <span aria-hidden="true">·</span>
        <time dateTime={new Date(recordedAt).toISOString()}>
          {relativeTime(recordedAt)}
        </time>
      </span>
    </div>
  );
}

function AttemptRow({
  attempt,
  index,
}: {
  attempt: ProjectProblemAttempt;
  index: number;
}) {
  return (
    <li className="relative">
      <span className="absolute -left-[1.28rem] top-1.5 size-2 rounded-full border-2 border-surface-2 bg-muted-foreground" />
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-micro font-semibold uppercase tracking-wider text-muted-foreground">
          Attempt {index + 1}
        </p>
        <OutcomeBadge outcome={attempt.outcome} compact />
      </div>
      <p className="mt-1 whitespace-pre-wrap text-meta">{attempt.action}</p>
      {attempt.evidence && (
        <p className="mt-1 whitespace-pre-wrap text-micro leading-5 text-muted-foreground">
          {attempt.evidence}
        </p>
      )}
    </li>
  );
}

function OutcomeBadge({
  outcome,
  compact = false,
}: {
  outcome: ProjectProblemAttempt["outcome"];
  compact?: boolean;
}) {
  const positive = outcome === "worked";
  const negative = outcome === "failed" || outcome === "no-effect";
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-sm font-semibold",
        compact ? "px-1.5 py-0.5 text-micro" : "px-2 py-1 text-micro",
        positive
          ? "bg-success/10 text-success"
          : negative
            ? "bg-destructive/10 text-destructive"
            : "bg-warning/10 text-warning",
      )}
    >
      {positive ? (
        <CheckCircle2 size={11} aria-hidden="true" />
      ) : negative ? (
        <XCircle size={11} aria-hidden="true" />
      ) : (
        <CircleAlert size={11} aria-hidden="true" />
      )}
      {humanize(outcome)}
    </span>
  );
}

function CitationList({
  citations,
  onEvidence,
}: {
  citations: ProjectActivityCitation[];
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  if (citations.length === 0) return null;
  return (
    <div>
      <p className="text-micro font-semibold uppercase tracking-wider text-muted-foreground">
        Touched artifacts
      </p>
      <div className="mt-2 flex flex-wrap gap-1.5">
        {citations.map((citation) => (
          <button
            type="button"
            key={`${citation.artifactPath}:${citation.startLine}:${citation.endLine}`}
            onClick={() => onEvidence(citation)}
            className="inline-flex max-w-full touch-manipulation items-center gap-1.5 rounded-md border border-border bg-background/55 px-2 py-1 text-left font-mono text-[10px] text-muted-foreground outline-none transition-[transform,border-color,background-color,color] hover:border-primary/35 hover:bg-primary/7 hover:text-foreground active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
            title={`Snapshot ${citation.artifactSnapshotId} · ${shortId(citation.contentHash)}`}
          >
            <FileCode2 size={11} className="shrink-0" aria-hidden="true" />
            <span className="truncate">
              {citation.artifactPath}:{citation.startLine}
              {citation.endLine !== citation.startLine
                ? `–${citation.endLine}`
                : ""}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}

function TruncationNotice({ onSession }: { onSession: () => void }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border bg-background/45 px-3 py-2 text-micro text-muted-foreground">
      <span>This project view is bounded.</span>
      <button
        type="button"
        onClick={onSession}
        className="font-semibold text-primary outline-none hover:underline focus-visible:ring-2 focus-visible:ring-primary"
      >
        Open complete session
      </button>
    </div>
  );
}

function ScopeTab({
  value,
  active,
  onClick,
}: {
  value: ProjectProblemScope;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={active}
      onClick={onClick}
      className={cn(
        "h-8 rounded-md px-2.5 text-meta font-medium capitalize outline-none focus-visible:ring-2 focus-visible:ring-primary",
        active
          ? "bg-surface-3 text-foreground shadow-sm"
          : "text-muted-foreground hover:text-foreground",
      )}
    >
      {value}
    </button>
  );
}

function DetailBlock({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-micro font-semibold uppercase tracking-wider text-muted-foreground">
        {label}
      </p>
      <p className="mt-1 whitespace-pre-wrap text-meta leading-6">{value}</p>
    </div>
  );
}

function OmissionNotice({ count, noun }: { count: number; noun: string }) {
  return (
    <p className="rounded-md border border-dashed border-border bg-surface-1 px-4 py-3 text-center text-micro text-muted-foreground">
      {count.toLocaleString()} more {noun} omitted by the local response bound.
      Refine the search to inspect them.
    </p>
  );
}

function EmptyState({
  icon: Icon,
  title,
  detail,
}: {
  icon: typeof Lightbulb;
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

function ErrorState({
  mode,
  message,
}: {
  mode: ActivityMode;
  message: string;
}) {
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
        <p className="font-semibold">
          Could not read {mode === "decisions" ? "decisions" : "problems"}
        </p>
        <p className="mt-0.5 text-muted-foreground">{message}</p>
      </div>
    </div>
  );
}

function ActivitySkeleton() {
  return (
    <div className="space-y-3" aria-label="Loading project activity">
      {[0, 1, 2].map((item) => (
        <div
          key={item}
          className="h-40 animate-pulse rounded-md border border-border bg-surface-1"
        />
      ))}
    </div>
  );
}

function humanize(value: string): string {
  return value
    .replaceAll("-", " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());
}

function relativeTime(timestamp: number): string {
  const delta = Date.now() - timestamp;
  const minutes = Math.max(0, Math.floor(delta / 60_000));
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function shortId(value: string): string {
  return value.length > 12 ? `${value.slice(0, 12)}…` : value;
}
