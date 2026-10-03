import { lazy, Suspense } from "react";
import { ArrowUpRight, CircleDot, Clock3, LockKeyhole } from "lucide-react";
import {
  humanize,
  KnowledgeSurfaceFallback,
  relativeTime,
  SessionStatus,
} from "./AgentMemoryPresentation";
import type {
  AgentMemoryDashboard,
  ArtifactEvidenceReference,
} from "./types";

const AgentBriefPreview = lazy(() =>
  import("./AgentBriefPreview").then((module) => ({
    default: module.AgentBriefPreview,
  })),
);

export function ProjectOverview({
  projectPath,
  dashboard,
  onEvidence,
  onSession,
}: {
  projectPath: string;
  dashboard: AgentMemoryDashboard;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  onSession: (id: string) => void;
}) {
  const { overview, resume } = dashboard;
  const activeSession = resume.sessions.find(
    (session) => session.status === "active" || session.status === "paused",
  );

  return (
    <div className="space-y-7">
      <header className="border-b border-border pb-5">
        <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
          Continue
        </p>
        <h2 className="mt-1 text-2xl font-semibold tracking-[-0.035em] sm:text-3xl">
          {overview.projectName}
        </h2>
        <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-micro text-muted-foreground">
          <span className="inline-flex items-center gap-1.5">
            <LockKeyhole size={12} aria-hidden="true" />
            Local project memory
          </span>
          <span className="inline-flex items-center gap-1.5">
            <CircleDot size={12} aria-hidden="true" />
            {humanize(overview.freshness)}
          </span>
          <span>{humanize(overview.captureMode)} capture</span>
          <span className="inline-flex items-center gap-1.5">
            <Clock3 size={12} aria-hidden="true" />
            Captured {relativeTime(overview.artifactGeneratedAtUnixMs)}
          </span>
        </div>
      </header>

      {activeSession && (
        <section aria-labelledby="active-handoff-title">
          <div className="mb-2">
            <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
              Active handoff
            </p>
            <h3
              id="active-handoff-title"
              className="mt-1 text-body font-semibold tracking-tight"
            >
              Resume current work
            </h3>
          </div>
          <button
            type="button"
            onClick={() => onSession(activeSession.sessionId)}
            className="group flex w-full items-start gap-3 border-y border-border px-1 py-3 text-left outline-none hover:bg-surface-1/60 focus-visible:ring-2 focus-visible:ring-primary"
          >
            <span className="pt-0.5">
              <SessionStatus status={activeSession.status} compact />
            </span>
            <span className="min-w-0 flex-1">
              <span className="flex items-baseline justify-between gap-3">
                <span className="truncate text-meta font-semibold">
                  {activeSession.name}
                </span>
                <span className="shrink-0 text-micro text-muted-foreground">
                  {relativeTime(activeSession.updatedAtUnixMs)}
                </span>
              </span>
              <span className="mt-1 block line-clamp-2 text-meta leading-5 text-muted-foreground">
                {activeSession.latestCheckpoint?.summary ??
                  activeSession.result?.summary ??
                  activeSession.goal}
              </span>
            </span>
            <ArrowUpRight
              size={14}
              className="mt-1 shrink-0 text-subtle-foreground transition group-hover:text-primary"
              aria-hidden="true"
            />
          </button>
        </section>
      )}

      <Suspense fallback={<KnowledgeSurfaceFallback />}>
        <AgentBriefPreview
          key={overview.projectId}
          projectPath={projectPath}
          projectId={overview.projectId}
          onEvidence={onEvidence}
        />
      </Suspense>
    </div>
  );
}
