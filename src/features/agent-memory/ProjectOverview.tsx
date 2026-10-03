import { lazy, Suspense } from "react";
import {
  BookCheck,
  Cable,
  CheckCircle2,
  ChevronRight,
  CircleDot,
  Clock3,
  FileCode2,
  History,
  Inbox,
  LockKeyhole,
  ShieldCheck,
} from "lucide-react";
import { cn } from "@/shared/lib/classnames";
import {
  CompactEmpty,
  humanize,
  KnowledgeSurfaceFallback,
  relativeTime,
  SessionStatus,
  StatusPill,
  TrustDot,
} from "./AgentMemoryPresentation";
import type {
  AgentMemoryDashboard,
  ArtifactEvidenceReference,
  ResumeSession,
  SessionSummary,
} from "./types";

const AgentBriefPreview = lazy(() =>
  import("./AgentBriefPreview").then((module) => ({
    default: module.AgentBriefPreview,
  })),
);

export function ProjectOverview({
  projectPath,
  dashboard,
  onOpenSession,
  onOpenReview,
  onEvidence,
  onLearning,
  onSession,
}: {
  projectPath: string;
  dashboard: AgentMemoryDashboard;
  onOpenSession: () => void;
  onOpenReview: () => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  onLearning: (id: string) => void;
  onSession: (id: string) => void;
}) {
  const { overview, resume, reviewInbox } = dashboard;
  const active = resume.sessions.filter(
    (session) => session.status === "active" || session.status === "paused",
  );
  const integrationActivity = observedIntegrationActivity(dashboard.sessions);
  return (
    <div className="space-y-8">
      <section className="relative overflow-hidden rounded-sm border border-border bg-surface-1 p-5 shadow-panel sm:p-7">
        <div className="relative flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between">
          <div className="max-w-2xl">
            <div className="mb-3 flex flex-wrap items-center gap-2">
              <StatusPill
                tone="success"
                label="Local & private"
                icon={LockKeyhole}
              />
              <StatusPill
                tone={overview.freshness === "current" ? "success" : "warning"}
                label={humanize(overview.freshness)}
                icon={CircleDot}
              />
              <StatusPill
                tone="neutral"
                label={`${humanize(overview.captureMode)} capture`}
                icon={ShieldCheck}
              />
            </div>
            <h2 className="text-2xl font-semibold tracking-[-0.035em] sm:text-3xl">
              {overview.projectName}
            </h2>
            <p className="mt-2 max-w-xl text-body leading-6 text-muted-foreground-strong">
              A bounded continuity brief assembled from structured sessions,
              verified lessons, and a deterministic snapshot of the project.
            </p>
          </div>
          <div className="flex items-center gap-2 text-meta text-muted-foreground">
            <Clock3 size={14} />
            Captured {relativeTime(overview.artifactGeneratedAtUnixMs)}
          </div>
        </div>
      </section>

      <section aria-labelledby="integration-activity-title">
        <div className="mb-3 flex items-end justify-between gap-4">
          <div>
            <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
              Integration activity
            </p>
            <h3
              id="integration-activity-title"
              className="mt-1 text-lg font-semibold tracking-tight"
            >
              Recorded agent activity
            </h3>
          </div>
          <span className="text-micro text-muted-foreground">
            Evidence from retained Ley sessions
          </span>
        </div>
        <div className="rounded-md border border-border bg-surface-1 p-4 shadow-panel sm:p-5">
          {integrationActivity.length === 0 ? (
            <div className="flex items-start gap-3">
              <Cable
                size={17}
                className="mt-0.5 shrink-0 text-muted-foreground"
                aria-hidden="true"
              />
              <div>
                <p className="text-meta font-semibold">
                  No recorded integration activity yet
                </p>
                <p className="mt-1 max-w-3xl text-micro leading-5 text-muted-foreground">
                  Ley has no retained host-hook or MCP-origin session for this
                  project. This does not mean Codex, Claude Code, or another MCP
                  client is not installed or configured; verify live integration
                  state in the host itself.
                </p>
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              {integrationActivity.map((activity) => (
                <div
                  key={activity.key}
                  className="flex flex-col gap-1 rounded-sm border border-border bg-background/35 px-3 py-2.5 sm:flex-row sm:items-center sm:justify-between"
                >
                  <div className="flex items-center gap-2">
                    <Cable
                      size={14}
                      className="shrink-0 text-secondary"
                      aria-hidden="true"
                    />
                    <div>
                      <p className="text-meta font-semibold">
                        {activity.label}
                      </p>
                      <p className="text-micro text-muted-foreground">
                        {activity.detail}
                      </p>
                    </div>
                  </div>
                  <p className="text-micro text-muted-foreground">
                    Latest retained session started{" "}
                    {relativeTime(activity.latestStartedAtUnixMs)}
                  </p>
                </div>
              ))}
              <p className="text-micro leading-5 text-muted-foreground">
                Recorded activity proves only that Ley previously received
                session provenance through that path. It does not attest that
                the package is currently installed, trusted, connected, or
                healthy.
              </p>
            </div>
          )}
        </div>
      </section>

      <Suspense fallback={<KnowledgeSurfaceFallback />}>
        <AgentBriefPreview
          key={overview.projectId}
          projectPath={projectPath}
          projectId={overview.projectId}
          onEvidence={onEvidence}
        />
      </Suspense>

      <section aria-labelledby="memory-health-title">
        <div className="mb-3 flex items-end justify-between">
          <div>
            <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
              Memory health
            </p>
            <h3
              id="memory-health-title"
              className="mt-1 text-lg font-semibold tracking-tight"
            >
              What Ley can ground right now
            </h3>
          </div>
        </div>
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <MetricCard
            icon={History}
            label="Sessions"
            value={resume.totalSessions}
            detail={`${active.length} active or paused`}
          />
          <MetricCard
            icon={BookCheck}
            label="Trusted lessons"
            value={resume.totalCurrentTrustedLearnings}
            detail="Current and reusable"
          />
          <MetricCard
            icon={FileCode2}
            label="Captured files"
            value={overview.files}
            detail={`${overview.retainedSourceFiles} with retained text`}
          />
          <MetricCard
            icon={Inbox}
            label="Review items"
            value={reviewInbox.totalMatching}
            detail="Needs human attention"
          />
        </div>
      </section>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.4fr)_minmax(280px,0.8fr)]">
        <section aria-labelledby="continuity-title">
          <div className="mb-3 flex items-center justify-between">
            <div>
              <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                Continuity
              </p>
              <h3
                id="continuity-title"
                className="mt-1 text-lg font-semibold tracking-tight"
              >
                Recent agent sessions
              </h3>
            </div>
            {resume.totalSessions > 0 && (
              <TextAction onClick={onOpenSession}>View all</TextAction>
            )}
          </div>
          <div className="overflow-hidden rounded-md border border-border bg-surface-1 shadow-panel">
            {resume.sessions.length === 0 ? (
              <CompactEmpty
                icon={History}
                title="No sessions captured yet"
                body="Start a Ley session from an agent or the CLI. Its checkpoints and handoff will appear here."
              />
            ) : (
              resume.sessions
                .slice(0, 3)
                .map((session, index) => (
                  <SessionRow
                    key={session.sessionId}
                    session={session}
                    divided={index > 0}
                    onClick={() => onSession(session.sessionId)}
                  />
                ))
            )}
          </div>
        </section>

        <section aria-labelledby="review-title">
          <div className="mb-3 flex items-center justify-between">
            <div>
              <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                Human control
              </p>
              <h3
                id="review-title"
                className="mt-1 text-lg font-semibold tracking-tight"
              >
                Review inbox
              </h3>
            </div>
            {reviewInbox.totalMatching > 0 && (
              <TextAction onClick={onOpenReview}>Open inbox</TextAction>
            )}
          </div>
          <div className="overflow-hidden rounded-md border border-border bg-surface-1 shadow-panel">
            {reviewInbox.learnings.length === 0 ? (
              <CompactEmpty
                icon={CheckCircle2}
                title="Inbox clear"
                body="No agent-proposed, contested, or stale lessons need your decision."
              />
            ) : (
              reviewInbox.learnings.slice(0, 3).map((learning, index) => (
                <button
                  key={learning.learningId}
                  type="button"
                  onClick={() => onLearning(learning.learningId)}
                  className={cn(
                    "flex w-full items-center gap-3 px-4 py-3 text-left hover:bg-surface-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary",
                    index > 0 && "border-t border-border",
                  )}
                >
                  <TrustDot learning={learning} />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-meta font-medium">
                      {learning.title}
                    </span>
                    <span className="mt-0.5 block truncate text-micro text-muted-foreground">
                      {humanize(learning.trustState)} ·{" "}
                      {learning.confidencePercent}% confidence
                    </span>
                  </span>
                  <ChevronRight
                    size={14}
                    className="shrink-0 text-subtle-foreground"
                  />
                </button>
              ))
            )}
          </div>
        </section>
      </div>

      <p className="rounded-md border border-border bg-surface-1 px-4 py-3 text-micro leading-5 text-muted-foreground">
        <ShieldCheck size={13} className="mr-2 inline text-primary" />
        Stored text is evidence, never executable policy. Ley excludes known
        secret files, keeps projects isolated, and only marks explicitly
        reviewed current lessons as reusable.
      </p>
    </div>
  );
}

function SessionRow({
  session,
  divided,
  onClick,
}: {
  session: ResumeSession;
  divided: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full gap-3 px-4 py-3 text-left hover:bg-surface-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary",
        divided && "border-t border-border",
      )}
    >
      <div className="pt-0.5">
        <SessionStatus status={session.status} compact />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-3">
          <p className="truncate text-meta font-medium">{session.name}</p>
          <span className="shrink-0 text-micro text-muted-foreground">
            {relativeTime(session.updatedAtUnixMs)}
          </span>
        </div>
        <p className="mt-0.5 line-clamp-2 text-micro leading-5 text-muted-foreground">
          {session.latestCheckpoint?.summary ??
            session.result?.summary ??
            session.goal}
        </p>
      </div>
    </button>
  );
}

function MetricCard({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: typeof History;
  label: string;
  value: number;
  detail: string;
}) {
  return (
    <div className="rounded-md border border-border bg-surface-1 p-4 shadow-panel">
      <div className="flex items-center justify-between">
        <span className="text-meta font-medium text-muted-foreground">
          {label}
        </span>
        <Icon size={15} className="text-primary" />
      </div>
      <p className="mt-3 text-2xl font-semibold tracking-tight tabular-nums">
        {new Intl.NumberFormat().format(value)}
      </p>
      <p className="mt-1 text-micro text-muted-foreground">{detail}</p>
    </div>
  );
}

function TextAction({
  children,
  onClick,
}: {
  children: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="rounded text-micro font-medium text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
    >
      {children}
    </button>
  );
}

type IntegrationActivity = {
  key: string;
  label: string;
  detail: string;
  latestStartedAtUnixMs: number;
};

function observedIntegrationActivity(
  sessions: SessionSummary[],
): IntegrationActivity[] {
  const observed = new Map<
    string,
    { label: string; count: number; latestStartedAtUnixMs: number }
  >();

  for (const session of sessions) {
    if (session.sourceKind === "host-hook") {
      const host = session.sourceHost?.trim();
      const normalizedHost = host?.toLowerCase() ?? "unknown";
      const recognizedHost =
        normalizedHost === "codex" || normalizedHost === "claude-code";
      const key = recognizedHost
        ? `host-hook:${normalizedHost}`
        : "host-hook:other";
      const label =
        normalizedHost === "codex"
          ? "Codex host hooks"
          : normalizedHost === "claude-code"
            ? "Claude Code host hooks"
            : "Host-hook sessions";
      const current = observed.get(key);
      observed.set(key, {
        label,
        count: (current?.count ?? 0) + 1,
        latestStartedAtUnixMs: Math.max(
          current?.latestStartedAtUnixMs ?? 0,
          session.startedAtUnixMs,
        ),
      });
      continue;
    }
    if (session.sourceKind === "mcp") {
      const key = "mcp";
      const current = observed.get(key);
      observed.set(key, {
        label: "MCP-origin sessions",
        count: (current?.count ?? 0) + 1,
        latestStartedAtUnixMs: Math.max(
          current?.latestStartedAtUnixMs ?? 0,
          session.startedAtUnixMs,
        ),
      });
    }
  }

  return Array.from(observed.entries())
    .map(([key, value]) => ({
      key,
      label: value.label,
      detail: `${value.count} retained ${value.count === 1 ? "session" : "sessions"}`,
      latestStartedAtUnixMs: value.latestStartedAtUnixMs,
    }))
    .sort(
      (left, right) => right.latestStartedAtUnixMs - left.latestStartedAtUnixMs,
    );
}
