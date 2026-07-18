import { lazy, Suspense, useEffect, useState, type ReactNode } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  BookCheck,
  BrainCircuit,
  Check,
  CheckCircle2,
  ChevronRight,
  CircleDot,
  Clock3,
  FileCode2,
  FilePlus2,
  Files,
  FolderOpen,
  GitBranch,
  History,
  Inbox,
  LockKeyhole,
  MessageSquareWarning,
  Network,
  PencilLine,
  RefreshCw,
  RotateCcw,
  Scale,
  ShieldCheck,
  Sparkles,
  X,
  XCircle,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import {
  chooseAgentProject,
  connectAgentProject,
  forgetAgentProject,
  initializeAgentProject,
  inspectAgentProject,
  listAgentProjects,
  readAgentLearning,
  readAgentSession,
  refreshAgentProject,
  reviewAgentLearning,
  verifyAgentProjectNoteVault,
} from "./api";
import { ProjectsHub } from "./ProjectsHub";
import type {
  AgentMemoryDashboard,
  AgentProjectCatalog,
  AgentProjectInspection,
  AgentProjectSearchResult,
  ArtifactEvidenceReference,
  LearningAction,
  LearningContext,
  LearningSummary,
  PromotedLearningNoteDraft,
  PromotedSessionNoteDraft,
  ResumeSession,
  SessionContext,
  SessionSummary,
} from "./types";

const LAST_AGENT_PROJECT_KEY = "ley:last-agent-project";
type Section =
  | "overview"
  | "sessions"
  | "decisions"
  | "problems"
  | "lessons"
  | "artifacts"
  | "graph"
  | "review"
  | "privacy";

const ArtifactExplorer = lazy(() =>
  import("./ArtifactExplorer").then((module) => ({
    default: module.ArtifactExplorer,
  })),
);
const ProjectKnowledgeGraph = lazy(() =>
  import("./ProjectKnowledgeGraph").then((module) => ({
    default: module.ProjectKnowledgeGraph,
  })),
);
const ProjectActivityExplorer = lazy(() =>
  import("./ProjectActivityExplorer").then((module) => ({
    default: module.ProjectActivityExplorer,
  })),
);
const CapturePrivacyPanel = lazy(() =>
  import("./CapturePrivacyPanel").then((module) => ({
    default: module.CapturePrivacyPanel,
  })),
);
const SessionRenameEditor = lazy(() =>
  import("./SessionRenameEditor").then((module) => ({
    default: module.SessionRenameEditor,
  })),
);
const SessionPromotionEditor = lazy(() =>
  import("./SessionPromotionEditor").then((module) => ({
    default: module.SessionPromotionEditor,
  })),
);
const LearningCorrectionEditor = lazy(() =>
  import("./LearningCorrectionEditor").then((module) => ({
    default: module.LearningCorrectionEditor,
  })),
);
const LearningPromotionEditor = lazy(() =>
  import("./LearningPromotionEditor").then((module) => ({
    default: module.LearningPromotionEditor,
  })),
);

export function AgentMemoryWorkspace({
  open,
  vaultMode,
  vaultPath,
  vaultName,
  onClose,
  onPromoteLearning,
  onPromoteSession,
}: {
  open: boolean;
  vaultMode: "desktop" | "browser-folder" | "browser-local";
  vaultPath: string;
  vaultName: string;
  onClose: () => void;
  onPromoteLearning: (draft: PromotedLearningNoteDraft) => Promise<void>;
  onPromoteSession: (draft: PromotedSessionNoteDraft) => Promise<void>;
}) {
  const [section, setSection] = useState<Section>("overview");
  const [projectPath, setProjectPath] = useState<string | null>(null);
  const [catalog, setCatalog] = useState<AgentProjectCatalog | null>(null);
  const [catalogBusy, setCatalogBusy] = useState(vaultMode === "desktop");
  const [catalogRevision, setCatalogRevision] = useState(0);
  const [inspection, setInspection] = useState<AgentProjectInspection | null>(
    null,
  );
  const [inspectedPath, setInspectedPath] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [learningId, setLearningId] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [artifactFocus, setArtifactFocus] = useState<{
    path: string;
    requestId: number;
  } | null>(null);
  const [graphFocus, setGraphFocus] = useState<{
    evidence?: ArtifactEvidenceReference;
    graphSnapshotId?: string;
    requestId: number;
  } | null>(null);

  useEffect(() => {
    if (!open || vaultMode !== "desktop" || projectPath || catalog) return;
    let current = true;
    const legacyProjectPath =
      localStorage.getItem(LAST_AGENT_PROJECT_KEY) ?? undefined;
    void listAgentProjects(legacyProjectPath)
      .then((next) => {
        if (!current) return;
        setCatalog(next);
        localStorage.removeItem(LAST_AGENT_PROJECT_KEY);
        setCatalogBusy(false);
      })
      .catch((cause) => {
        if (!current) return;
        setError(errorMessage(cause));
        setCatalogBusy(false);
      });
    return () => {
      current = false;
    };
  }, [catalog, catalogRevision, open, projectPath, vaultMode]);

  useEffect(() => {
    if (
      !open ||
      vaultMode !== "desktop" ||
      !projectPath ||
      inspectedPath === projectPath
    )
      return;
    let current = true;
    void inspectAgentProject(projectPath)
      .then((next) => {
        if (!current) return;
        setInspection(next);
        setInspectedPath(projectPath);
        setBusy(false);
      })
      .catch((cause) => {
        if (!current) return;
        setInspectedPath(projectPath);
        setError(errorMessage(cause));
        setBusy(false);
      });
    return () => {
      current = false;
    };
  }, [inspectedPath, open, projectPath, vaultMode]);

  async function chooseProject() {
    setError(null);
    try {
      const selected = await chooseAgentProject();
      if (selected) {
        setBusy(true);
        setInspection(null);
        setInspectedPath(null);
        setProjectPath(selected);
      }
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  function openProject(
    nextProjectPath: string,
    destination?: AgentProjectSearchResult,
  ) {
    setBusy(true);
    setError(null);
    setInspection(null);
    setInspectedPath(null);
    setProjectPath(nextProjectPath);
    setArtifactFocus(null);
    setGraphFocus(null);
    if (!destination) {
      setSection("overview");
      return;
    }
    if (
      destination.kind === "session" ||
      destination.kind === "decision" ||
      destination.kind === "problem"
    ) {
      setSection(
        destination.kind === "session"
          ? "sessions"
          : destination.kind === "decision"
            ? "decisions"
            : "problems",
      );
      setSessionId(destination.sessionId ?? null);
    } else if (destination.kind === "learning") {
      setSection("lessons");
      setLearningId(destination.learningId ?? null);
    } else {
      setSection(destination.kind === "artifact" ? "artifacts" : "graph");
      if (destination.kind === "artifact") {
        setArtifactFocus({
          path: destination.citation?.artifactPath ?? destination.title,
          requestId: Date.now(),
        });
      } else if (destination.citation) {
        setGraphFocus({
          evidence: destination.citation,
          requestId: Date.now(),
        });
      }
    }
  }

  function openArtifact(path: string) {
    setSessionId(null);
    setLearningId(null);
    setArtifactFocus({ path, requestId: Date.now() });
    setSection("artifacts");
  }

  function openEvidence(evidence: ArtifactEvidenceReference) {
    setSessionId(null);
    setLearningId(null);
    setGraphFocus({ evidence, requestId: Date.now() });
    setSection("graph");
  }

  function openProjectRevision(graphSnapshotId: string) {
    setSessionId(null);
    setLearningId(null);
    setGraphFocus({ graphSnapshotId, requestId: Date.now() });
    setSection("graph");
  }

  async function makeReady(kind: "initialize" | "connect" | "capture") {
    if (!projectPath) return;
    setBusy(true);
    setError(null);
    try {
      const dashboard =
        kind === "initialize"
          ? await initializeAgentProject(projectPath, vaultPath)
          : kind === "connect"
            ? await connectAgentProject(projectPath, vaultPath)
            : await refreshAgentProject(projectPath);
      setInspection({ status: "ready", dashboard });
      setSection("overview");
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  async function refresh() {
    await makeReady("capture");
  }

  async function promoteLearningToBoundVault(draft: PromotedLearningNoteDraft) {
    if (!projectPath) throw new Error("Open a project before linking a note.");
    await verifyAgentProjectNoteVault(projectPath, vaultPath);
    await onPromoteLearning(draft);
  }

  async function promoteSessionToBoundVault(draft: PromotedSessionNoteDraft) {
    if (!projectPath) throw new Error("Open a project before linking a note.");
    await verifyAgentProjectNoteVault(projectPath, vaultPath);
    await onPromoteSession(draft);
  }

  function returnToProjects() {
    setProjectPath(null);
    setInspection(null);
    setInspectedPath(null);
    setError(null);
    setLearningId(null);
    setSessionId(null);
    setSection("overview");
    setCatalog(null);
    setCatalogBusy(true);
    setCatalogRevision((value) => value + 1);
  }

  async function removeProject(projectId: string) {
    setCatalogBusy(true);
    setError(null);
    try {
      setCatalog(await forgetAgentProject(projectId));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setCatalogBusy(false);
    }
  }

  const dashboard =
    inspection?.status === "ready" ? inspection.dashboard : null;
  const projectLabel =
    dashboard?.overview.projectName ??
    (inspection && "projectName" in inspection ? inspection.projectName : null);

  if (!open) return null;

  return (
    <Dialog.Root
      open={open}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-[59] bg-background" />
        <Dialog.Content
          data-page="agent-memory-workspace"
          className="fixed inset-0 z-[60] flex min-h-0 flex-col overflow-hidden bg-background text-foreground outline-none"
          aria-describedby={undefined}
        >
          <header className="app-chrome flex h-14 shrink-0 items-center justify-between px-3 sm:px-5">
            <div className="flex min-w-0 items-center gap-3">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-primary/25 bg-primary/10 text-primary">
                <BrainCircuit size={17} aria-hidden="true" />
              </div>
              <div className="min-w-0">
                <Dialog.Title className="truncate text-body font-semibold tracking-tight">
                  {projectPath ? "Agent Memory" : "Projects"}
                </Dialog.Title>
                <p className="truncate text-micro text-muted-foreground">
                  {projectLabel
                    ? `${projectLabel} · local project memory`
                    : catalog
                      ? `${catalog.totalProjects.toLocaleString()} local project ${catalog.totalProjects === 1 ? "memory" : "memories"}`
                      : "Continuity for your coding agents"}
                </p>
              </div>
            </div>
            <div className="flex items-center gap-1.5">
              {projectPath && (
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={returnToProjects}
                  title="Back to Projects"
                >
                  <ArrowLeft size={13} />
                  <span className="hidden sm:inline">Projects</span>
                </Button>
              )}
              {dashboard && (
                <Button
                  size="sm"
                  variant="outline"
                  disabled={busy}
                  onClick={() => void refresh()}
                  title="Capture changed project files and rebuild memory"
                >
                  <RefreshCw
                    size={13}
                    className={
                      busy
                        ? "animate-spin motion-reduce:animate-none"
                        : undefined
                    }
                  />
                  <span className="hidden sm:inline">Refresh snapshot</span>
                </Button>
              )}
              <Button
                size="sm"
                variant="ghost"
                onClick={onClose}
                aria-label="Close Agent Memory"
                title="Close Agent Memory"
              >
                <X size={16} />
              </Button>
            </div>
          </header>

          {vaultMode !== "desktop" ? (
            <BrowserBoundary vaultMode={vaultMode} vaultName={vaultName} />
          ) : !projectPath ? (
            <ProjectsHub
              catalog={catalog}
              loading={catalogBusy}
              error={error}
              onAdd={() => void chooseProject()}
              onOpen={openProject}
              onForget={(projectId) => void removeProject(projectId)}
              onReload={() => {
                setCatalog(null);
                setCatalogBusy(true);
                setError(null);
                setCatalogRevision((value) => value + 1);
              }}
            />
          ) : !inspection || inspection.status !== "ready" ? (
            <ProjectOnboarding
              inspection={inspection}
              projectPath={projectPath}
              vaultName={vaultName}
              busy={busy}
              error={error}
              onChoose={() => void chooseProject()}
              onForget={returnToProjects}
              onInitialize={() => void makeReady("initialize")}
              onConnect={() => void makeReady("connect")}
              onCapture={() => void makeReady("capture")}
            />
          ) : (
            <div className="flex min-h-0 flex-1 flex-col md:flex-row">
              <AgentMemoryNav
                section={section}
                dashboard={inspection.dashboard}
                busy={busy}
                onSection={(nextSection) => {
                  if (nextSection === "artifacts") setArtifactFocus(null);
                  if (nextSection === "graph") setGraphFocus(null);
                  setSection(nextSection);
                }}
                onChangeProject={returnToProjects}
              />
              <main className="min-h-0 min-w-0 flex-1 overflow-y-auto overscroll-contain">
                <div className="mx-auto w-full max-w-6xl px-4 py-6 sm:px-6 sm:py-8 lg:px-10">
                  {error && <ErrorNotice message={error} />}
                  {section === "overview" && (
                    <Overview
                      dashboard={inspection.dashboard}
                      onOpenSession={() => setSection("sessions")}
                      onOpenReview={() => setSection("review")}
                      onLearning={setLearningId}
                      onSession={setSessionId}
                    />
                  )}
                  {section === "sessions" && (
                    <Sessions
                      sessions={inspection.dashboard.sessions}
                      onSession={setSessionId}
                    />
                  )}
                  {(section === "decisions" || section === "problems") &&
                    projectPath && (
                      <Suspense fallback={<KnowledgeSurfaceFallback />}>
                        <ProjectActivityExplorer
                          mode={section}
                          projectPath={projectPath}
                          onSession={setSessionId}
                          onEvidence={openEvidence}
                        />
                      </Suspense>
                    )}
                  {section === "lessons" && (
                    <Lessons
                      dashboard={inspection.dashboard}
                      onLearning={setLearningId}
                    />
                  )}
                  {section === "artifacts" && projectPath && (
                    <Suspense fallback={<KnowledgeSurfaceFallback />}>
                      <ArtifactExplorer
                        key={`artifacts-${artifactFocus?.requestId ?? "browse"}`}
                        projectPath={projectPath}
                        focus={artifactFocus}
                      />
                    </Suspense>
                  )}
                  {section === "graph" && projectPath && (
                    <Suspense fallback={<KnowledgeSurfaceFallback />}>
                      <ProjectKnowledgeGraph
                        key={`graph-${graphFocus?.requestId ?? "browse"}`}
                        projectPath={projectPath}
                        focus={graphFocus}
                        onOpenArtifact={openArtifact}
                      />
                    </Suspense>
                  )}
                  {section === "review" && (
                    <ReviewInbox
                      dashboard={inspection.dashboard}
                      onLearning={setLearningId}
                    />
                  )}
                  {section === "privacy" && projectPath && (
                    <Suspense fallback={<KnowledgeSurfaceFallback />}>
                      <CapturePrivacyPanel
                        key={projectPath}
                        projectPath={projectPath}
                        dashboard={inspection.dashboard}
                        onUpdated={(next) =>
                          setInspection({ status: "ready", dashboard: next })
                        }
                        onErased={(next) => {
                          setInspection(next);
                          setSection("overview");
                        }}
                      />
                    </Suspense>
                  )}
                </div>
              </main>
            </div>
          )}

          {dashboard && projectPath && (
            <SessionInspector
              key={`session-${sessionId ?? "closed"}`}
              sessionId={sessionId}
              projectPath={projectPath}
              projectName={dashboard.overview.projectName}
              onClose={() => setSessionId(null)}
              onEvidence={openEvidence}
              onProjectRevision={openProjectRevision}
              onPromote={promoteSessionToBoundVault}
              onRenamed={(next) =>
                setInspection({ status: "ready", dashboard: next })
              }
            />
          )}

          {dashboard && projectPath && (
            <LearningInspector
              key={`learning-${learningId ?? "closed"}`}
              learningId={learningId}
              projectPath={projectPath}
              projectName={dashboard.overview.projectName}
              onClose={() => setLearningId(null)}
              onSession={(nextSessionId) => {
                setLearningId(null);
                setSessionId(nextSessionId);
              }}
              onArtifact={openArtifact}
              onPromote={promoteLearningToBoundVault}
              onReviewed={(next) => {
                setInspection({ status: "ready", dashboard: next });
                setLearningId(null);
              }}
            />
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function AgentMemoryNav({
  section,
  dashboard,
  busy,
  onSection,
  onChangeProject,
}: {
  section: Section;
  dashboard: AgentMemoryDashboard;
  busy: boolean;
  onSection: (section: Section) => void;
  onChangeProject: () => void;
}) {
  const items: Array<{
    id: Section;
    label: string;
    icon: typeof Sparkles;
    count?: number;
  }> = [
    { id: "overview", label: "Overview", icon: Sparkles },
    {
      id: "sessions",
      label: "Sessions",
      icon: History,
      count: dashboard.sessions.length,
    },
    {
      id: "decisions",
      label: "Decisions",
      icon: Scale,
    },
    {
      id: "problems",
      label: "Problems & outcomes",
      icon: MessageSquareWarning,
    },
    {
      id: "lessons",
      label: "Lessons",
      icon: BookCheck,
      count: dashboard.allLearnings.totalMatching,
    },
    {
      id: "artifacts",
      label: "Artifacts",
      icon: Files,
      count: dashboard.overview.files,
    },
    {
      id: "graph",
      label: "Project graph",
      icon: Network,
      count: dashboard.overview.graphNodes,
    },
    {
      id: "review",
      label: "Review",
      icon: Inbox,
      count: dashboard.reviewInbox.totalMatching,
    },
    {
      id: "privacy",
      label: "Capture & privacy",
      icon: ShieldCheck,
    },
  ];
  return (
    <aside className="app-sidebar shrink-0 border-b border-border md:flex md:w-56 md:flex-col md:border-b-0 md:border-r">
      <nav
        className="flex gap-1 overflow-x-auto p-2 md:flex-col md:overflow-visible md:p-3"
        aria-label="Agent Memory sections"
      >
        {items.map((item) => {
          const Icon = item.icon;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onSection(item.id)}
              aria-current={section === item.id ? "page" : undefined}
              className={cn(
                "flex h-9 shrink-0 items-center gap-2 rounded-md px-2.5 text-meta font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary",
                section === item.id
                  ? "bg-primary/12 text-foreground"
                  : "text-muted-foreground hover:bg-surface-2 hover:text-foreground",
              )}
            >
              <Icon
                size={14}
                className={section === item.id ? "text-primary" : undefined}
              />
              {item.label}
              {item.count !== undefined && (
                <span
                  className={cn(
                    "ml-auto rounded-full px-1.5 text-micro tabular-nums",
                    section === item.id
                      ? "bg-primary/15 text-primary"
                      : "bg-surface-3 text-muted-foreground",
                  )}
                >
                  {item.count}
                </span>
              )}
            </button>
          );
        })}
      </nav>
      <div className="mt-auto hidden border-t border-border p-3 md:block">
        <p className="truncate text-meta font-medium">
          {dashboard.overview.projectName}
        </p>
        <p className="mt-0.5 truncate text-micro text-muted-foreground">
          {dashboard.binding.vaultName}
        </p>
        <button
          type="button"
          disabled={busy}
          onClick={onChangeProject}
          className="mt-2 text-micro font-medium text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
        >
          Change project
        </button>
      </div>
    </aside>
  );
}

function KnowledgeSurfaceFallback() {
  return (
    <div
      className="flex min-h-80 items-center justify-center rounded-xl border border-border bg-surface-1 text-meta text-muted-foreground"
      aria-label="Loading project knowledge"
    >
      Loading local project knowledge…
    </div>
  );
}

function Overview({
  dashboard,
  onOpenSession,
  onOpenReview,
  onLearning,
  onSession,
}: {
  dashboard: AgentMemoryDashboard;
  onOpenSession: () => void;
  onOpenReview: () => void;
  onLearning: (id: string) => void;
  onSession: (id: string) => void;
}) {
  const { overview, resume, reviewInbox } = dashboard;
  const active = resume.sessions.filter(
    (session) => session.status === "active" || session.status === "paused",
  );
  return (
    <div className="space-y-8">
      <section className="relative overflow-hidden rounded-2xl border border-border bg-surface-1 p-5 shadow-panel sm:p-7">
        <div className="pointer-events-none absolute -right-20 -top-24 size-64 rounded-full bg-primary/10 blur-3xl" />
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
            icon={GitBranch}
            label="Project graph"
            value={overview.graphNodes}
            detail={`${overview.graphEdges} deterministic links`}
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
          <div className="overflow-hidden rounded-xl border border-border bg-surface-1 shadow-panel">
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
          <div className="overflow-hidden rounded-xl border border-border bg-surface-1 shadow-panel">
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

      <p className="rounded-lg border border-border bg-surface-1 px-4 py-3 text-micro leading-5 text-muted-foreground">
        <ShieldCheck size={13} className="mr-2 inline text-primary" />
        Stored text is evidence, never executable policy. Ley excludes known
        secret files, keeps projects isolated, and only marks explicitly
        reviewed current lessons as reusable.
      </p>
    </div>
  );
}

function Sessions({
  sessions,
  onSession,
}: {
  sessions: SessionSummary[];
  onSession: (id: string) => void;
}) {
  return (
    <section aria-labelledby="sessions-title">
      <PageHeading
        eyebrow="Continuity timeline"
        title="Sessions"
        description={`${sessions.length} structured agent ${sessions.length === 1 ? "session" : "sessions"} captured for this project.`}
      />
      <div className="mt-6 space-y-3">
        {sessions.length === 0 ? (
          <LargeEmpty
            icon={History}
            title="No sessions yet"
            body="When an agent starts a Ley session, its goal, checkpoints, decisions, verification, and handoff will appear here."
          />
        ) : (
          sessions.map((session) => (
            <SessionSummaryCard
              key={session.sessionId}
              session={session}
              onClick={() => onSession(session.sessionId)}
            />
          ))
        )}
      </div>
    </section>
  );
}

function Lessons({
  dashboard,
  onLearning,
}: {
  dashboard: AgentMemoryDashboard;
  onLearning: (id: string) => void;
}) {
  const learnings = dashboard.allLearnings.learnings;
  return (
    <section aria-labelledby="lessons-title">
      <PageHeading
        eyebrow="Procedural memory"
        title="Lessons"
        description={`Evidence-backed guidance remains reviewable, temporal, and separate from ordinary notes. Showing ${learnings.length} of ${dashboard.allLearnings.totalMatching}.`}
      />
      <div className="mt-6 grid gap-3 lg:grid-cols-2">
        {learnings.length === 0 ? (
          <div className="lg:col-span-2">
            <LargeEmpty
              icon={BookCheck}
              title="No lessons proposed"
              body="Agents can propose learnings from cited session records. Nothing becomes trusted until evidence or your explicit confirmation supports it."
            />
          </div>
        ) : (
          learnings.map((learning) => (
            <LearningCard
              key={learning.learningId}
              learning={learning}
              onClick={() => onLearning(learning.learningId)}
            />
          ))
        )}
      </div>
    </section>
  );
}

function ReviewInbox({
  dashboard,
  onLearning,
}: {
  dashboard: AgentMemoryDashboard;
  onLearning: (id: string) => void;
}) {
  const inbox = dashboard.reviewInbox;
  return (
    <section aria-labelledby="review-inbox-title">
      <PageHeading
        eyebrow="Human authority"
        title="Review inbox"
        description={`Confirm useful guidance, contest uncertain claims, reject false memory, or mark guidance stale. Showing ${inbox.learnings.length} of ${inbox.totalMatching}.`}
      />
      <div className="mt-6 space-y-3">
        {inbox.learnings.length === 0 ? (
          <LargeEmpty
            icon={CheckCircle2}
            title="You’re all caught up"
            body="No proposed, contested, source-changed, or stale lessons need review."
          />
        ) : (
          inbox.learnings.map((learning) => (
            <LearningCard
              key={learning.learningId}
              learning={learning}
              onClick={() => onLearning(learning.learningId)}
              wide
            />
          ))
        )}
      </div>
    </section>
  );
}

function SessionInspector({
  sessionId,
  projectPath,
  projectName,
  onClose,
  onEvidence,
  onProjectRevision,
  onPromote,
  onRenamed,
}: {
  sessionId: string | null;
  projectPath: string;
  projectName: string;
  onClose: () => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  onProjectRevision: (graphSnapshotId: string) => void;
  onPromote: (draft: PromotedSessionNoteDraft) => Promise<void>;
  onRenamed: (dashboard: AgentMemoryDashboard) => void;
}) {
  const [session, setSession] = useState<SessionContext | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [renameDirty, setRenameDirty] = useState(false);
  const [promoting, setPromoting] = useState(false);
  const [promotionDirty, setPromotionDirty] = useState(false);
  const [busy, setBusy] = useState(Boolean(sessionId));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!sessionId) return;
    let current = true;
    void readAgentSession(projectPath, sessionId)
      .then((next) => {
        if (current) {
          setSession(next);
          setError(null);
        }
      })
      .catch((cause) => {
        if (current) setError(errorMessage(cause));
      })
      .finally(() => {
        if (current) setBusy(false);
      });
    return () => {
      current = false;
    };
  }, [projectPath, sessionId]);

  return (
    <Dialog.Root
      open={Boolean(sessionId)}
      onOpenChange={(next) => {
        if (
          !next &&
          ((!(renaming && renameDirty) && !(promoting && promotionDirty)) ||
            window.confirm("Discard your unsaved note or rename changes?"))
        ) {
          onClose();
        }
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-[80]" />
        <Dialog.Content
          className="app-modal-surface fixed inset-x-3 bottom-3 top-3 z-[81] mx-auto flex max-w-4xl flex-col overflow-hidden rounded-2xl border outline-none sm:inset-x-6 sm:bottom-6 sm:top-6"
          aria-describedby={undefined}
        >
          <div className="flex shrink-0 items-center justify-between gap-3 border-b border-border px-4 py-3 sm:px-5">
            <div className="min-w-0">
              <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                Session provenance
              </p>
              <Dialog.Title className="mt-0.5 truncate text-body font-semibold">
                {session?.name ?? "Loading session…"}
              </Dialog.Title>
            </div>
            <div className="flex shrink-0 items-center gap-1">
              {session && (
                <Button
                  size="sm"
                  variant={promoting ? "outline" : "ghost"}
                  className="h-8"
                  aria-expanded={promoting}
                  aria-controls="session-note-link-panel"
                  aria-label="Link session to notes"
                  onClick={() => {
                    if (
                      ((promoting && promotionDirty) ||
                        (renaming && renameDirty)) &&
                      !window.confirm("Discard your unsaved changes?")
                    ) {
                      return;
                    }
                    setPromotionDirty(false);
                    setRenameDirty(false);
                    setRenaming(false);
                    setPromoting((current) => !current);
                  }}
                >
                  <FilePlus2 size={13} aria-hidden="true" />
                  <span className="hidden min-[420px]:inline">To notes</span>
                </Button>
              )}
              {session && (
                <Button
                  size="sm"
                  variant={renaming ? "outline" : "ghost"}
                  className="h-8"
                  aria-expanded={renaming}
                  onClick={() => {
                    if (
                      ((renaming && renameDirty) ||
                        (promoting && promotionDirty)) &&
                      !window.confirm("Discard your unsaved changes?")
                    ) {
                      return;
                    }
                    setRenameDirty(false);
                    setPromotionDirty(false);
                    setPromoting(false);
                    setRenaming((current) => !current);
                  }}
                >
                  <PencilLine size={13} aria-hidden="true" />
                  Rename
                </Button>
              )}
              <Dialog.Close
                className="rounded-md p-1.5 text-muted-foreground hover:bg-surface-3 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
                aria-label="Close session inspector"
              >
                <X size={15} />
              </Dialog.Close>
            </div>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain p-4 sm:p-6">
            {busy && !session ? (
              <div className="py-20 text-center text-meta text-muted-foreground">
                Replaying structured session events…
              </div>
            ) : error && !session ? (
              <ErrorNotice message={error} />
            ) : (
              session && (
                <div className="space-y-7">
                  {error && (
                    <p
                      className="rounded-lg border border-destructive/25 bg-destructive/8 p-3 text-micro text-destructive"
                      role="alert"
                    >
                      {error}
                    </p>
                  )}
                  <section className="rounded-xl border border-border bg-background/35 p-4 sm:p-5">
                    <div className="flex flex-wrap items-center gap-3">
                      <SessionStatus status={session.status} />
                      <span className="text-micro text-muted-foreground">
                        {sourceLabel(session)}
                      </span>
                      <span className="text-micro text-muted-foreground">
                        {new Intl.DateTimeFormat(undefined, {
                          dateStyle: "medium",
                          timeStyle: "short",
                        }).format(session.startedAtUnixMs)}
                      </span>
                    </div>
                    <h3 className="mt-4 text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                      Goal
                    </h3>
                    <p className="mt-1 whitespace-pre-wrap text-body leading-6">
                      {session.goal}
                    </p>
                    <div className="mt-4 flex flex-wrap gap-x-5 gap-y-1 text-micro text-muted-foreground">
                      <span>{session.checkpointCount} checkpoints</span>
                      <span>{session.eventCount} immutable events</span>
                      <span>~{session.estimatedTextTokens} context tokens</span>
                    </div>
                  </section>

                  {session.renameCount > 0 && (
                    <section aria-labelledby="session-naming-history-title">
                      <SectionLabel
                        id="session-naming-history-title"
                        icon={PencilLine}
                        label="Naming history"
                      />
                      <div className="mt-2 rounded-xl border border-border bg-surface-1 p-4 shadow-panel">
                        <div className="border-b border-border pb-3">
                          <p className="text-micro font-medium text-muted-foreground">
                            Original name
                          </p>
                          <p className="mt-1 text-meta font-medium">
                            {session.originalName}
                          </p>
                        </div>
                        <ol className="mt-3 space-y-3">
                          {session.renames.map((rename, index) => (
                            <li
                              key={`${rename.recordedAtUnixMs}:${index}`}
                              className="grid gap-1 sm:grid-cols-[minmax(0,1fr)_auto]"
                            >
                              <div className="min-w-0">
                                <p className="break-words text-meta font-medium">
                                  {rename.name}
                                </p>
                                <p className="mt-0.5 whitespace-pre-wrap text-micro leading-5 text-muted-foreground">
                                  {rename.note}
                                </p>
                              </div>
                              <time className="text-micro text-muted-foreground">
                                {relativeTime(rename.recordedAtUnixMs)}
                              </time>
                            </li>
                          ))}
                        </ol>
                        {session.omittedRenames > 0 && (
                          <p className="mt-3 border-t border-border pt-3 text-micro text-muted-foreground">
                            {session.omittedRenames} older renames are preserved
                            in the session log but omitted from this bounded
                            view.
                          </p>
                        )}
                      </div>
                    </section>
                  )}

                  {session.finish && (
                    <section aria-labelledby="session-outcome-title">
                      <SectionLabel
                        id="session-outcome-title"
                        icon={CheckCircle2}
                        label="Outcome & handoff"
                      />
                      <div className="mt-2 rounded-xl border border-border bg-surface-1 p-4 shadow-panel">
                        <p className="text-meta leading-5 text-muted-foreground-strong">
                          {session.finish.summary}
                        </p>
                        {session.finish.handoff && (
                          <p className="mt-3 rounded-lg bg-primary/7 px-3 py-2 text-meta leading-5">
                            <span className="font-medium text-primary">
                              Handoff:
                            </span>{" "}
                            {session.finish.handoff}
                          </p>
                        )}
                        {session.finish.finalResponse && (
                          <details className="mt-3 border-t border-border pt-3">
                            <summary className="cursor-pointer rounded text-micro font-medium text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary">
                              Final response
                            </summary>
                            <p className="mt-2 whitespace-pre-wrap text-micro leading-5 text-muted-foreground-strong">
                              {session.finish.finalResponse}
                            </p>
                          </details>
                        )}
                        {session.finish.unresolved.length > 0 && (
                          <MemoryList
                            title="Still unresolved"
                            items={session.finish.unresolved}
                            tone="warning"
                          />
                        )}
                      </div>
                    </section>
                  )}

                  <section aria-labelledby="checkpoint-timeline-title">
                    <SectionLabel
                      id="checkpoint-timeline-title"
                      icon={History}
                      label="Checkpoint timeline"
                    />
                    <div className="mt-3 space-y-4">
                      {session.checkpoints.length === 0 ? (
                        <CompactEmpty
                          icon={History}
                          title="No checkpoints yet"
                          body="This session has started but has not recorded structured progress."
                        />
                      ) : (
                        session.checkpoints.map((checkpoint, index) => (
                          <article
                            key={checkpoint.checkpointId}
                            className="relative rounded-xl border border-border bg-surface-1 p-4 shadow-panel sm:p-5"
                          >
                            <div className="flex flex-wrap items-center justify-between gap-2">
                              <span className="text-micro font-semibold uppercase tracking-[0.12em] text-primary">
                                Checkpoint {index + 1}
                              </span>
                              <time className="text-micro text-muted-foreground">
                                {relativeTime(checkpoint.recordedAtUnixMs)}
                              </time>
                            </div>
                            <p className="mt-2 whitespace-pre-wrap text-body leading-6">
                              {checkpoint.summary}
                            </p>

                            <div className="mt-4 grid gap-3 lg:grid-cols-2">
                              {checkpoint.decisions.length > 0 && (
                                <RecordGroup
                                  icon={GitBranch}
                                  title="Decisions"
                                  count={checkpoint.decisions.length}
                                >
                                  {checkpoint.decisions.map((decision) => (
                                    <RecordItem
                                      key={decision.id}
                                      title={decision.title}
                                      body={decision.decision}
                                    />
                                  ))}
                                </RecordGroup>
                              )}
                              {checkpoint.tasks.length > 0 && (
                                <RecordGroup
                                  icon={BookCheck}
                                  title="Tasks"
                                  count={checkpoint.tasks.length}
                                >
                                  {checkpoint.tasks.map((task) => (
                                    <RecordItem
                                      key={task.id}
                                      title={task.title}
                                      meta={humanize(task.status)}
                                    />
                                  ))}
                                </RecordGroup>
                              )}
                              {checkpoint.problems.length > 0 && (
                                <RecordGroup
                                  icon={AlertTriangle}
                                  title="Problems & outcomes"
                                  count={checkpoint.problems.length}
                                >
                                  {checkpoint.problems.map((problem) => (
                                    <ProblemItem
                                      key={problem.id}
                                      problem={problem}
                                    />
                                  ))}
                                </RecordGroup>
                              )}
                              {checkpoint.verification.length > 0 && (
                                <RecordGroup
                                  icon={ShieldCheck}
                                  title="Verification"
                                  count={checkpoint.verification.length}
                                >
                                  {checkpoint.verification.map(
                                    (verification) => (
                                      <RecordItem
                                        key={verification.id}
                                        title={humanize(verification.kind)}
                                        body={verification.summary}
                                        meta={humanize(verification.status)}
                                      />
                                    ),
                                  )}
                                </RecordGroup>
                              )}
                            </div>

                            {checkpoint.projectRevision && (
                              <ProjectRevisionButton
                                revision={checkpoint.projectRevision}
                                onOpen={onProjectRevision}
                              />
                            )}

                            {checkpoint.touchedArtifacts.length > 0 && (
                              <div className="mt-4 border-t border-border pt-4">
                                <p className="text-micro font-medium text-muted-foreground">
                                  Cited artifacts
                                </p>
                                <div className="mt-2 flex flex-wrap gap-1.5">
                                  {checkpoint.touchedArtifacts.map(
                                    (artifact) => (
                                      <button
                                        type="button"
                                        key={`${artifact.artifactPath}:${artifact.startLine}`}
                                        title={`${artifact.artifactPath}:${artifact.startLine}-${artifact.endLine}`}
                                        onClick={() => onEvidence(artifact)}
                                        className="max-w-full touch-manipulation truncate rounded-md border border-border bg-surface-2 px-2 py-1 text-left font-mono text-micro text-muted-foreground outline-none transition-[transform,border-color,background-color,color] hover:border-primary/35 hover:bg-primary/7 hover:text-foreground active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
                                      >
                                        {artifact.artifactPath}:
                                        {artifact.startLine}
                                      </button>
                                    ),
                                  )}
                                </div>
                              </div>
                            )}

                            {checkpoint.commands.length > 0 && (
                              <details className="mt-4 border-t border-border pt-4">
                                <summary className="cursor-pointer rounded text-micro font-medium text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary">
                                  Commands · {checkpoint.commands.length}
                                </summary>
                                <div className="mt-2 space-y-2">
                                  {checkpoint.commands.map((command) => (
                                    <div
                                      key={command.id}
                                      className="overflow-hidden rounded-lg border border-border bg-background/45"
                                    >
                                      <code className="block overflow-x-auto px-3 py-2 font-mono text-micro text-foreground">
                                        {command.command}
                                      </code>
                                      {(command.summary ||
                                        command.exitCode !== undefined) && (
                                        <p className="border-t border-border px-3 py-2 text-micro text-muted-foreground">
                                          {command.exitCode !== undefined
                                            ? `Exit ${command.exitCode}`
                                            : "Exit not recorded"}
                                          {command.summary
                                            ? ` · ${command.summary}`
                                            : ""}
                                        </p>
                                      )}
                                    </div>
                                  ))}
                                </div>
                              </details>
                            )}

                            {checkpoint.unresolved.length > 0 && (
                              <MemoryList
                                title="Unresolved at this checkpoint"
                                items={checkpoint.unresolved}
                                tone="warning"
                              />
                            )}
                          </article>
                        ))
                      )}
                    </div>
                  </section>

                  {session.omittedCheckpoints > 0 && (
                    <p className="rounded-lg border border-amber-500/20 bg-amber-500/10 px-3 py-2 text-micro text-amber-500">
                      {session.omittedCheckpoints} older checkpoints were
                      omitted from this bounded view.
                    </p>
                  )}
                  <p className="rounded-lg border border-border bg-background/35 p-3 text-micro leading-5 text-muted-foreground">
                    <MessageSquareWarning
                      size={13}
                      className="mr-2 inline text-secondary"
                    />
                    {session.instructionWarning}
                  </p>
                </div>
              )
            )}
          </div>
          {renaming && session && (
            <div className="shrink-0 border-t border-border bg-surface-1">
              <Suspense fallback={<KnowledgeSurfaceFallback />}>
                <SessionRenameEditor
                  projectPath={projectPath}
                  session={session}
                  onCancel={() => {
                    if (
                      renameDirty &&
                      !window.confirm("Discard your unsaved session rename?")
                    ) {
                      return;
                    }
                    setRenameDirty(false);
                    setRenaming(false);
                  }}
                  onDirtyChange={setRenameDirty}
                  onRenamed={(dashboard) => {
                    onRenamed(dashboard);
                    setRenameDirty(false);
                    setRenaming(false);
                    setBusy(true);
                    setError(null);
                    void readAgentSession(projectPath, session.sessionId)
                      .then(setSession)
                      .catch((cause) => setError(errorMessage(cause)))
                      .finally(() => setBusy(false));
                  }}
                />
              </Suspense>
            </div>
          )}
          {promoting && session && (
            <div
              id="session-note-link-panel"
              className="shrink-0 border-t border-border bg-surface-1"
            >
              <Suspense fallback={<KnowledgeSurfaceFallback />}>
                <SessionPromotionEditor
                  projectName={projectName}
                  session={session}
                  onCancel={() => {
                    if (
                      promotionDirty &&
                      !window.confirm("Discard your unsaved session note?")
                    ) {
                      return;
                    }
                    setPromotionDirty(false);
                    setPromoting(false);
                  }}
                  onDirtyChange={setPromotionDirty}
                  onPromote={onPromote}
                />
              </Suspense>
            </div>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function ProjectRevisionButton({
  revision,
  onOpen,
}: {
  revision: NonNullable<
    SessionContext["checkpoints"][number]["projectRevision"]
  >;
  onOpen: (graphSnapshotId: string) => void;
}) {
  const shortHead = revision.head?.slice(0, 10);
  const changeLabel =
    revision.trackedChanges === 0
      ? "clean tracked tree"
      : `${revision.trackedChanges.toLocaleString()} tracked change${revision.trackedChanges === 1 ? "" : "s"}`;
  return (
    <div className="mt-4 border-t border-border pt-4">
      <p className="text-micro font-medium text-muted-foreground">
        Captured Project Revision
      </p>
      <button
        type="button"
        onClick={() => onOpen(revision.graphSnapshotId)}
        className="mt-2 flex w-full touch-manipulation items-center gap-3 rounded-xl border border-border bg-surface-2 px-3 py-2.5 text-left outline-none transition-[transform,border-color,background-color] hover:border-primary/35 hover:bg-primary/7 active:scale-[0.99] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
        title="Open the exact Project Graph capture used by this checkpoint"
        aria-label={`Open captured project revision ${shortHead ?? revision.graphSnapshotId}`}
      >
        <span className="grid size-8 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary">
          <GitBranch size={16} aria-hidden="true" />
        </span>
        <span className="min-w-0 flex-1">
          <span
            className="block truncate font-mono text-meta font-medium text-foreground"
            translate="no"
          >
            {shortHead ?? "Snapshot only"}
            {revision.branch ? ` · ${revision.branch}` : ""}
          </span>
          <span className="mt-0.5 block text-micro text-muted-foreground">
            Captured {absoluteTime(revision.capturedAtUnixMs)} · {changeLabel}
          </span>
        </span>
        <ChevronRight
          size={16}
          className="shrink-0 text-muted-foreground"
          aria-hidden="true"
        />
      </button>
    </div>
  );
}

function LearningInspector({
  learningId,
  projectPath,
  projectName,
  onClose,
  onSession,
  onArtifact,
  onPromote,
  onReviewed,
}: {
  learningId: string | null;
  projectPath: string;
  projectName: string;
  onClose: () => void;
  onSession: (sessionId: string) => void;
  onArtifact: (path: string) => void;
  onPromote: (draft: PromotedLearningNoteDraft) => Promise<void>;
  onReviewed: (dashboard: AgentMemoryDashboard) => void;
}) {
  const [learning, setLearning] = useState<LearningContext | null>(null);
  const [action, setAction] = useState<LearningAction | null>(null);
  const [note, setNote] = useState("");
  const [correcting, setCorrecting] = useState(false);
  const [promoting, setPromoting] = useState(false);
  const [busy, setBusy] = useState(Boolean(learningId));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!learningId) return;
    let current = true;
    void readAgentLearning(projectPath, learningId)
      .then((next) => {
        if (current) setLearning(next);
      })
      .catch((cause) => {
        if (current) setError(errorMessage(cause));
      })
      .finally(() => {
        if (current) setBusy(false);
      });
    return () => {
      current = false;
    };
  }, [learningId, projectPath]);

  const noteRequired =
    action === "contest" || action === "reject" || action === "mark-stale";
  const canSubmit = action && (!noteRequired || note.trim().length > 0);
  const terminal =
    learning?.state === "rejected" || learning?.state === "superseded";

  async function submitReview() {
    if (!learningId || !learning || !action || !canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      const dashboard = await reviewAgentLearning(
        projectPath,
        learningId,
        learning.eventCount,
        action,
        note.trim(),
      );
      onReviewed(dashboard);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
    }
  }

  function beginCorrection() {
    if (!learning) return;
    setAction(null);
    setNote("");
    setCorrecting(true);
    setError(null);
  }

  function beginPromotion() {
    if (!learning?.trustedForReuse) return;
    setAction(null);
    setNote("");
    setCorrecting(false);
    setPromoting(true);
    setError(null);
  }

  return (
    <Dialog.Root
      open={Boolean(learningId)}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="app-modal-overlay fixed inset-0 z-[80]" />
        <Dialog.Content
          className="app-modal-surface fixed inset-x-3 bottom-3 top-3 z-[81] mx-auto flex max-w-3xl flex-col overflow-hidden rounded-2xl border outline-none focus-visible:ring-2 focus-visible:ring-primary sm:inset-x-6 sm:bottom-6 sm:top-6"
          aria-describedby={undefined}
        >
          <div className="flex shrink-0 items-center justify-between border-b border-border px-4 py-3 sm:px-5">
            <div>
              <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                Provenance inspector
              </p>
              <Dialog.Title className="mt-0.5 text-body font-semibold">
                {learning?.title ?? "Loading learning…"}
              </Dialog.Title>
            </div>
            <Dialog.Close
              className="rounded-md p-1.5 text-muted-foreground hover:bg-surface-3 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
              aria-label="Close learning inspector"
            >
              <X size={15} />
            </Dialog.Close>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto p-4 sm:p-6">
            {busy && !learning ? (
              <div className="py-20 text-center text-meta text-muted-foreground">
                Reading cited memory…
              </div>
            ) : error && !learning ? (
              <ErrorNotice message={error} />
            ) : (
              learning && (
                <div className="space-y-6">
                  <div className="flex flex-wrap gap-2">
                    <StatusPill
                      tone={learning.trustedForReuse ? "success" : "warning"}
                      label={humanize(learning.trustState)}
                      icon={
                        learning.trustedForReuse ? CheckCircle2 : AlertTriangle
                      }
                    />
                    <StatusPill
                      tone="neutral"
                      label={humanize(learning.provenance)}
                      icon={BrainCircuit}
                    />
                    <StatusPill
                      tone="neutral"
                      label={`${learning.confidencePercent}% confidence`}
                      icon={CircleDot}
                    />
                    <StatusPill
                      tone={
                        learning.freshness === "current" ? "success" : "warning"
                      }
                      label={humanize(learning.freshness)}
                      icon={RefreshCw}
                    />
                  </div>
                  <section>
                    <h3 className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                      Guidance
                    </h3>
                    <p className="mt-2 whitespace-pre-wrap break-words text-body leading-6 text-foreground">
                      {learning.guidance}
                    </p>
                  </section>
                  <section>
                    <h3 className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                      Version timeline
                    </h3>
                    <dl className="mt-2 grid gap-2 rounded-lg border border-border bg-background/35 p-3 text-meta sm:grid-cols-3">
                      <div>
                        <dt className="text-micro text-muted-foreground">
                          Created
                        </dt>
                        <dd className="mt-0.5 font-medium">
                          <time
                            dateTime={isoTime(learning.createdAtUnixMs)}
                            title={absoluteTime(learning.createdAtUnixMs)}
                          >
                            {relativeTime(learning.createdAtUnixMs)}
                          </time>
                        </dd>
                      </div>
                      <div>
                        <dt className="text-micro text-muted-foreground">
                          Current version
                        </dt>
                        <dd className="mt-0.5 font-medium">
                          <time
                            dateTime={isoTime(learning.validFromUnixMs)}
                            title={absoluteTime(learning.validFromUnixMs)}
                          >
                            {relativeTime(learning.validFromUnixMs)}
                          </time>
                        </dd>
                      </div>
                      <div>
                        <dt className="text-micro text-muted-foreground">
                          Ledger
                        </dt>
                        <dd className="mt-0.5 font-medium">
                          {learning.eventCount} immutable{" "}
                          {learning.eventCount === 1 ? "event" : "events"}
                        </dd>
                      </div>
                    </dl>
                    {learning.validUntilUnixMs !== undefined && (
                      <p className="mt-2 text-micro text-muted-foreground">
                        This version stopped being valid{" "}
                        <time
                          dateTime={isoTime(learning.validUntilUnixMs)}
                          title={absoluteTime(learning.validUntilUnixMs)}
                        >
                          {relativeTime(learning.validUntilUnixMs)}
                        </time>
                        .
                      </p>
                    )}
                  </section>
                  <section>
                    <h3 className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                      Evidence · {learning.evidenceCount}
                    </h3>
                    <div className="mt-2 space-y-2">
                      {learning.evidence.length === 0 ? (
                        <p className="text-meta text-muted-foreground">
                          No evidence is available.
                        </p>
                      ) : (
                        learning.evidence.map((evidence) => (
                          <div
                            key={`${evidence.sessionId}:${evidence.recordId}`}
                            className="rounded-lg border border-border bg-background/35 p-3"
                          >
                            <div className="flex flex-wrap items-center justify-between gap-2 text-micro text-muted-foreground">
                              <div className="flex flex-wrap items-center gap-2">
                                <span className="rounded bg-surface-3 px-1.5 py-0.5 font-medium text-muted-foreground-strong">
                                  {humanize(evidence.recordType)}
                                </span>
                                <span>
                                  {relativeTime(
                                    evidence.sessionUpdatedAtUnixMs,
                                  )}
                                </span>
                              </div>
                              <button
                                type="button"
                                onClick={() => onSession(evidence.sessionId)}
                                className="touch-manipulation rounded font-semibold text-primary outline-none transition-transform hover:underline active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
                              >
                                Open session
                              </button>
                            </div>
                            {evidence.note && (
                              <p className="mt-2 text-meta leading-5 text-muted-foreground-strong">
                                {evidence.note}
                              </p>
                            )}
                            {evidence.artifacts.length > 0 && (
                              <div className="mt-2 flex flex-wrap gap-1.5">
                                {evidence.artifacts.map((artifact) => (
                                  <button
                                    type="button"
                                    key={`${artifact.artifactPath}:${artifact.startLine}`}
                                    title={`${artifact.artifactPath}:${artifact.startLine}-${artifact.endLine}`}
                                    onClick={() =>
                                      onArtifact(artifact.artifactPath)
                                    }
                                    className="max-w-full touch-manipulation truncate rounded-md border border-border bg-surface-2 px-2 py-1 text-left font-mono text-micro text-muted-foreground outline-none transition-[transform,border-color,background-color,color] hover:border-primary/35 hover:bg-primary/7 hover:text-foreground active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
                                  >
                                    {artifact.artifactPath}:{artifact.startLine}
                                  </button>
                                ))}
                              </div>
                            )}
                          </div>
                        ))
                      )}
                    </div>
                    {learning.omittedEvidence > 0 && (
                      <p className="mt-2 text-micro text-muted-foreground">
                        {learning.omittedEvidence} older evidence{" "}
                        {learning.omittedEvidence === 1
                          ? "reference is"
                          : "references are"}{" "}
                        omitted from this bounded inspector.
                      </p>
                    )}
                  </section>
                  {learning.history.length > 0 && (
                    <section>
                      <h3 className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                        Review history
                      </h3>
                      <ol className="mt-2 space-y-2">
                        {learning.history.map((entry) => (
                          <li
                            key={entry.eventId}
                            className="flex gap-3 text-meta"
                          >
                            <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-border-strong" />
                            <span className="min-w-0 break-words">
                              <span className="font-medium">
                                {humanize(entry.action)}
                              </span>{" "}
                              by {humanize(entry.actor)} ·{" "}
                              <time
                                dateTime={isoTime(entry.recordedAtUnixMs)}
                                title={absoluteTime(entry.recordedAtUnixMs)}
                              >
                                {relativeTime(entry.recordedAtUnixMs)}
                              </time>
                              {entry.note ? ` — ${entry.note}` : ""}
                            </span>
                          </li>
                        ))}
                      </ol>
                      {learning.omittedHistory > 0 && (
                        <p className="mt-2 text-micro text-muted-foreground">
                          Showing {learning.history.length} of{" "}
                          {learning.historyCount} immutable history events.
                        </p>
                      )}
                    </section>
                  )}
                  {learning.claimTruncated && (
                    <p className="rounded-lg border border-warning/25 bg-warning/8 p-3 text-micro text-muted-foreground">
                      The title or guidance was truncated to keep this inspector
                      bounded. Use the CLI for the complete claim before
                      correcting or reviewing it.
                    </p>
                  )}
                  <p className="rounded-lg border border-border bg-background/35 p-3 text-micro leading-5 text-muted-foreground">
                    <MessageSquareWarning
                      size={13}
                      className="mr-2 inline text-secondary"
                    />
                    {learning.instructionWarning}
                  </p>
                </div>
              )
            )}
          </div>
          {learning && (
            <div className="shrink-0 border-t border-border bg-surface-1 p-4 sm:p-5">
              {learning.claimTruncated ? (
                <p className="text-meta text-muted-foreground">
                  This bounded view omits part of the claim. Inspect the
                  complete CLI projection before correcting or reviewing it.
                </p>
              ) : terminal ? (
                <p className="text-meta text-muted-foreground">
                  This {humanize(learning.state)} learning is preserved as
                  terminal history. Create a new learning rather than rewriting
                  it.
                </p>
              ) : correcting ? (
                <Suspense
                  fallback={
                    <p className="py-4 text-center text-meta text-muted-foreground">
                      Loading correction editor…
                    </p>
                  }
                >
                  <LearningCorrectionEditor
                    projectPath={projectPath}
                    learning={learning}
                    onCancel={() => {
                      setCorrecting(false);
                      setError(null);
                    }}
                    onCorrected={onReviewed}
                  />
                </Suspense>
              ) : promoting ? (
                <Suspense
                  fallback={
                    <p className="py-4 text-center text-meta text-muted-foreground">
                      Loading note preview…
                    </p>
                  }
                >
                  <LearningPromotionEditor
                    projectName={projectName}
                    learning={learning}
                    onCancel={() => {
                      setPromoting(false);
                      setError(null);
                    }}
                    onPromote={async (draft) => {
                      await onPromote(draft);
                      onClose();
                    }}
                  />
                </Suspense>
              ) : !action ? (
                <div className="flex flex-wrap items-center gap-2">
                  <span className="mr-auto text-meta font-medium">
                    Your decision
                  </span>
                  {learning.trustedForReuse && (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={beginPromotion}
                    >
                      <FilePlus2 size={13} aria-hidden="true" />
                      Promote to note
                    </Button>
                  )}
                  <Button size="sm" variant="outline" onClick={beginCorrection}>
                    <PencilLine size={13} />
                    Correct
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => setAction("mark-stale")}
                  >
                    <RotateCcw size={13} />
                    Mark stale
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => setAction("contest")}
                  >
                    <AlertTriangle size={13} />
                    Contest
                  </Button>
                  <Button
                    size="sm"
                    variant="destructive"
                    onClick={() => setAction("reject")}
                  >
                    <XCircle size={13} />
                    Reject
                  </Button>
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={() => setAction("confirm")}
                  >
                    <Check size={13} />
                    Confirm
                  </Button>
                </div>
              ) : (
                <div>
                  <label
                    htmlFor="learning-review-note"
                    className="block text-meta font-medium"
                  >
                    {actionLabel(action)}
                    <span className="ml-1 font-normal text-muted-foreground">
                      {noteRequired ? "· note required" : "· note optional"}
                    </span>
                  </label>
                  <textarea
                    id="learning-review-note"
                    name="learning-review-note"
                    autoComplete="off"
                    value={note}
                    onChange={(event) => setNote(event.target.value)}
                    placeholder={reviewPlaceholder(action)}
                    rows={2}
                    className="mt-2 w-full resize-none rounded-lg border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none placeholder:text-subtle-foreground focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary"
                  />
                  {error && (
                    <p
                      className="mt-2 text-micro text-destructive"
                      role="alert"
                    >
                      {error}
                    </p>
                  )}
                  <div className="mt-3 flex justify-end gap-2">
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={busy}
                      onClick={() => {
                        setAction(null);
                        setNote("");
                        setError(null);
                      }}
                    >
                      Cancel
                    </Button>
                    <Button
                      size="sm"
                      variant={action === "reject" ? "destructive" : "primary"}
                      disabled={busy || !canSubmit}
                      onClick={() => void submitReview()}
                    >
                      {busy ? "Saving…" : actionLabel(action)}
                    </Button>
                  </div>
                </div>
              )}
            </div>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function ProjectOnboarding({
  inspection,
  projectPath,
  vaultName,
  busy,
  error,
  onChoose,
  onForget,
  onInitialize,
  onConnect,
  onCapture,
}: {
  inspection: AgentProjectInspection | null;
  projectPath: string | null;
  vaultName: string;
  busy: boolean;
  error: string | null;
  onChoose: () => void;
  onForget: () => void;
  onInitialize: () => void;
  onConnect: () => void;
  onCapture: () => void;
}) {
  const title = !inspection
    ? projectPath
      ? "Opening local project"
      : "Choose a project"
    : inspection.status === "uninitialized"
      ? "Initialize Agent Memory"
      : inspection.status === "unbound"
        ? "Connect this project"
        : inspection.status === "vault-unavailable"
          ? "Reconnect this project"
          : "Create the first snapshot";
  const body = !inspection
    ? projectPath
      ? "Ley is validating this project’s local identity, private vault binding, and captured memory."
      : "Ley only reads a project after you choose its folder. It never scans neighboring folders or discovers projects silently."
    : inspection.status === "uninitialized"
      ? `Ley will add a small .ley folder to “${inspection.suggestedName}”, use Structured capture, bind durable memory to “${vaultName}”, and create the first redacted snapshot.`
      : inspection.status === "unbound"
        ? `“${inspection.projectName}” is initialized but has no private vault binding. Connect it to “${vaultName}” and capture its approved files.`
        : inspection.status === "vault-unavailable"
          ? `“${inspection.projectName}” was connected to “${inspection.previousVaultName}”, which moved or is unavailable. Reconnect it to the open vault, “${vaultName}”, and rebuild its local snapshot.`
          : inspection.status === "needs-capture"
            ? `“${inspection.projectName}” is connected to “${inspection.binding.vaultName}” but has not been captured yet.`
            : "This project is ready.";
  return (
    <main className="min-h-0 flex-1 overflow-y-auto px-4 py-10 sm:px-6">
      <div className="mx-auto flex min-h-full max-w-xl items-center justify-center">
        <div className="w-full rounded-2xl border border-border bg-surface-1 p-6 shadow-panel sm:p-8">
          <div className="flex size-11 items-center justify-center rounded-xl border border-primary/20 bg-primary/10 text-primary">
            {inspection ? <BrainCircuit size={21} /> : <FolderOpen size={21} />}
          </div>
          <h2 className="mt-5 text-2xl font-semibold tracking-[-0.035em]">
            {title}
          </h2>
          <p className="mt-2 text-body leading-6 text-muted-foreground-strong">
            {body}
          </p>
          {projectPath && (
            <p
              className="mt-4 truncate rounded-md bg-background/40 px-3 py-2 font-mono text-micro text-muted-foreground"
              title={projectPath}
            >
              {projectPath}
            </p>
          )}
          {error && (
            <div className="mt-4">
              <ErrorNotice message={error} />
            </div>
          )}
          <div className="mt-6 flex flex-wrap gap-2">
            {!inspection ? (
              <>
                <Button variant="primary" disabled={busy} onClick={onChoose}>
                  {busy ? (
                    <RefreshCw
                      size={14}
                      className="animate-spin motion-reduce:animate-none"
                    />
                  ) : (
                    <FolderOpen size={14} />
                  )}
                  {busy ? "Opening…" : "Choose project folder"}
                </Button>
                {projectPath && (
                  <Button variant="outline" onClick={onForget}>
                    Back to projects
                  </Button>
                )}
              </>
            ) : (
              <>
                <Button
                  variant="primary"
                  disabled={busy}
                  onClick={
                    inspection.status === "uninitialized"
                      ? onInitialize
                      : inspection.status === "unbound" ||
                          inspection.status === "vault-unavailable"
                        ? onConnect
                        : onCapture
                  }
                >
                  {busy ? (
                    <RefreshCw
                      size={14}
                      className="animate-spin motion-reduce:animate-none"
                    />
                  ) : (
                    <ArrowRight size={14} />
                  )}
                  {busy
                    ? "Preparing memory…"
                    : inspection.status === "uninitialized"
                      ? "Initialize & capture"
                      : inspection.status === "unbound"
                        ? "Connect & capture"
                        : inspection.status === "vault-unavailable"
                          ? "Reconnect & capture"
                          : "Capture project"}
                </Button>
                <Button variant="outline" disabled={busy} onClick={onChoose}>
                  Choose another
                </Button>
                <Button variant="ghost" disabled={busy} onClick={onForget}>
                  Back to projects
                </Button>
              </>
            )}
          </div>
          <div className="mt-6 border-t border-border pt-4">
            <p className="flex gap-2 text-micro leading-5 text-muted-foreground">
              <ShieldCheck size={14} className="mt-0.5 shrink-0 text-primary" />
              Known credentials, private keys, environment files, build output,
              and ignored paths are excluded before durable memory is written.
            </p>
          </div>
        </div>
      </div>
    </main>
  );
}

function BrowserBoundary({
  vaultMode,
  vaultName,
}: {
  vaultMode: "browser-folder" | "browser-local";
  vaultName: string;
}) {
  return (
    <main className="min-h-0 flex-1 overflow-y-auto px-4 py-10 sm:px-6">
      <div className="mx-auto flex min-h-full max-w-xl items-center justify-center">
        <div className="w-full rounded-2xl border border-border bg-surface-1 p-6 shadow-panel sm:p-8">
          <div className="flex size-11 items-center justify-center rounded-xl border border-secondary/20 bg-secondary/10 text-secondary">
            <LockKeyhole size={21} />
          </div>
          <p className="mt-5 text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
            Honest local boundary
          </p>
          <h2 className="mt-1 text-2xl font-semibold tracking-[-0.035em]">
            Agent Memory needs the desktop app
          </h2>
          <p className="mt-3 text-body leading-6 text-muted-foreground-strong">
            This{" "}
            {vaultMode === "browser-folder"
              ? `browser folder vault, “${vaultName},”`
              : "browser-local vault"}{" "}
            can edit notes, but a web page cannot safely read coding projects or
            serve local agents through stdio MCP.
          </p>
          <div className="mt-5 rounded-lg border border-border bg-background/35 p-4">
            <p className="text-meta font-medium">
              Your browser notes still remain fully usable.
            </p>
            <p className="mt-1 text-micro leading-5 text-muted-foreground">
              Open the same filesystem vault in Ley Desktop to initialize
              projects, capture structured sessions, review lessons, and connect
              Claude, Codex, Gemini, or another MCP client.
            </p>
          </div>
        </div>
      </div>
    </main>
  );
}

function SessionSummaryCard({
  session,
  onClick,
}: {
  session: SessionSummary;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="w-full cursor-pointer rounded-xl border border-border bg-surface-1 p-4 text-left shadow-panel hover:border-border-strong hover:bg-surface-2/55 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary sm:p-5"
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <SessionStatus status={session.status} />
            <span className="text-micro text-muted-foreground">
              {relativeTime(session.updatedAtUnixMs)}
            </span>
          </div>
          <h3 className="mt-2 text-body font-semibold">{session.name}</h3>
          <p className="mt-1 text-meta leading-5 text-muted-foreground-strong">
            {session.goal}
          </p>
        </div>
        <span className="shrink-0 text-micro tabular-nums text-muted-foreground">
          {session.checkpoints} checkpoints · {session.eventCount} events
        </span>
      </div>
    </button>
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

function LearningCard({
  learning,
  onClick,
  wide = false,
}: {
  learning: LearningSummary;
  onClick: () => void;
  wide?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "group w-full rounded-xl border border-border bg-surface-1 p-4 text-left shadow-panel hover:border-border-strong hover:bg-surface-2/55 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary",
        wide && "sm:p-5",
      )}
    >
      <div className="flex items-start gap-3">
        <TrustDot learning={learning} />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="rounded bg-surface-3 px-1.5 py-0.5 text-micro font-medium text-muted-foreground-strong">
              {humanize(learning.kind)}
            </span>
            <span className="text-micro text-muted-foreground">
              {humanize(learning.trustState)}
            </span>
            <span className="text-micro text-muted-foreground">
              · {learning.confidencePercent}%
            </span>
          </div>
          <h3 className="mt-2 text-body font-semibold">{learning.title}</h3>
          <p
            className={cn(
              "mt-1 text-meta leading-5 text-muted-foreground-strong",
              wide ? "line-clamp-3" : "line-clamp-2",
            )}
          >
            {learning.guidanceExcerpt}
          </p>
          <p className="mt-3 text-micro text-muted-foreground">
            {humanize(learning.provenance)} · {learning.corroboratingSessions}{" "}
            corroborating{" "}
            {learning.corroboratingSessions === 1 ? "session" : "sessions"} ·{" "}
            {relativeTime(learning.updatedAtUnixMs)}
          </p>
        </div>
        <ChevronRight
          size={15}
          className="mt-1 shrink-0 text-subtle-foreground group-hover:text-foreground"
        />
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
    <div className="rounded-xl border border-border bg-surface-1 p-4 shadow-panel">
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

function SectionLabel({
  id,
  icon: Icon,
  label,
}: {
  id: string;
  icon: typeof History;
  label: string;
}) {
  return (
    <h3
      id={id}
      className="flex items-center gap-2 text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground"
    >
      <Icon size={14} className="text-primary" />
      {label}
    </h3>
  );
}

function RecordGroup({
  icon: Icon,
  title,
  count,
  children,
}: {
  icon: typeof History;
  title: string;
  count: number;
  children: ReactNode;
}) {
  return (
    <section className="rounded-lg border border-border bg-background/30 p-3">
      <h4 className="flex items-center gap-2 text-micro font-medium text-muted-foreground">
        <Icon size={13} />
        {title}
        <span className="ml-auto tabular-nums">{count}</span>
      </h4>
      <div className="mt-2 space-y-2">{children}</div>
    </section>
  );
}

function RecordItem({
  title,
  body,
  meta,
}: {
  title: string;
  body?: string;
  meta?: string;
}) {
  return (
    <div className="rounded-md bg-surface-2/70 px-3 py-2">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <p className="text-meta font-medium">{title}</p>
        {meta && (
          <span className="text-micro text-muted-foreground">{meta}</span>
        )}
      </div>
      {body && (
        <p className="mt-1 whitespace-pre-wrap text-micro leading-5 text-muted-foreground-strong">
          {body}
        </p>
      )}
    </div>
  );
}

function ProblemItem({
  problem,
}: {
  problem: SessionContext["checkpoints"][number]["problems"][number];
}) {
  return (
    <div className="rounded-md bg-surface-2/70 px-3 py-2">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <p className="text-meta font-medium">{problem.title}</p>
        <span className="text-micro text-muted-foreground">
          {problem.resolutionDetail
            ? "Resolved"
            : problem.latestAttemptOutcome
              ? `Latest · ${humanize(problem.latestAttemptOutcome)}`
              : "Unresolved"}
        </span>
      </div>
      <p className="mt-1 whitespace-pre-wrap text-micro leading-5 text-muted-foreground-strong">
        {problem.symptom}
      </p>
      {problem.attempts.length > 0 && (
        <ol className="mt-3 space-y-2 border-l border-border pl-3">
          {problem.attempts.map((attempt, index) => (
            <li key={attempt.id}>
              <p className="text-micro font-medium">
                Attempt {index + 1} · {humanize(attempt.outcome)}
              </p>
              <p className="mt-0.5 text-micro leading-5 text-muted-foreground-strong">
                {attempt.action}
              </p>
              {attempt.evidence && (
                <p className="mt-0.5 text-micro italic leading-5 text-muted-foreground">
                  Evidence: {attempt.evidence}
                </p>
              )}
            </li>
          ))}
        </ol>
      )}
      {problem.resolutionDetail && (
        <div className="mt-3 rounded-md border border-emerald-500/15 bg-emerald-500/7 px-3 py-2">
          <p className="text-micro font-medium text-emerald-500">Resolution</p>
          <p className="mt-1 text-micro leading-5 text-muted-foreground-strong">
            <span className="font-medium text-foreground">Root cause:</span>{" "}
            {problem.resolutionDetail.rootCause}
          </p>
          <p className="mt-1 text-micro leading-5 text-muted-foreground-strong">
            <span className="font-medium text-foreground">Changed:</span>{" "}
            {problem.resolutionDetail.change}
          </p>
          {problem.resolutionDetail.verification && (
            <p className="mt-1 text-micro leading-5 text-muted-foreground-strong">
              <span className="font-medium text-foreground">Verified:</span>{" "}
              {problem.resolutionDetail.verification}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function MemoryList({
  title,
  items,
  tone,
}: {
  title: string;
  items: string[];
  tone: "warning" | "neutral";
}) {
  return (
    <div className="mt-3">
      <p
        className={cn(
          "text-micro font-medium",
          tone === "warning" ? "text-amber-500" : "text-muted-foreground",
        )}
      >
        {title}
      </p>
      <ul className="mt-1 space-y-1 text-micro leading-5 text-muted-foreground-strong">
        {items.map((item, index) => (
          <li key={`${index}:${item}`} className="flex gap-2">
            <span className="mt-2 size-1 shrink-0 rounded-full bg-border-strong" />
            <span>{item}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

function PageHeading({
  eyebrow,
  title,
  description,
}: {
  eyebrow: string;
  title: string;
  description: string;
}) {
  return (
    <div>
      <p className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
        {eyebrow}
      </p>
      <h2 className="mt-1 text-2xl font-semibold tracking-[-0.035em]">
        {title}
      </h2>
      <p className="mt-2 max-w-2xl text-body leading-6 text-muted-foreground-strong">
        {description}
      </p>
    </div>
  );
}

function StatusPill({
  tone,
  label,
  icon: Icon,
}: {
  tone: "success" | "warning" | "neutral";
  label: string;
  icon: typeof ShieldCheck;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2 py-1 text-micro font-medium",
        tone === "success" &&
          "border-emerald-500/20 bg-emerald-500/10 text-emerald-500",
        tone === "warning" &&
          "border-amber-500/20 bg-amber-500/10 text-amber-500",
        tone === "neutral" &&
          "border-border bg-surface-2 text-muted-foreground-strong",
      )}
    >
      <Icon size={11} />
      {label}
    </span>
  );
}

function SessionStatus({
  status,
  compact = false,
}: {
  status: ResumeSession["status"];
  compact?: boolean;
}) {
  const color =
    status === "active"
      ? "bg-emerald-500"
      : status === "paused"
        ? "bg-amber-500"
        : status === "completed"
          ? "bg-primary"
          : "bg-subtle-foreground";
  if (compact)
    return (
      <span
        className={cn("block size-2 rounded-full", color)}
        title={humanize(status)}
      />
    );
  return (
    <span className="inline-flex items-center gap-1.5 text-micro font-medium text-muted-foreground-strong">
      <span className={cn("size-2 rounded-full", color)} />
      {humanize(status)}
    </span>
  );
}

function TrustDot({
  learning,
}: {
  learning: Pick<LearningSummary, "trustState" | "freshness">;
}) {
  const trusted =
    learning.trustState === "trusted" && learning.freshness === "current";
  const rejected = learning.trustState === "rejected";
  return (
    <span
      className={cn(
        "mt-1.5 size-2.5 shrink-0 rounded-full ring-4",
        trusted
          ? "bg-emerald-500 ring-emerald-500/10"
          : rejected
            ? "bg-destructive ring-destructive/10"
            : "bg-amber-500 ring-amber-500/10",
      )}
    />
  );
}

function CompactEmpty({
  icon: Icon,
  title,
  body,
}: {
  icon: typeof History;
  title: string;
  body: string;
}) {
  return (
    <div className="px-5 py-6 text-center">
      <Icon size={18} className="mx-auto text-subtle-foreground" />
      <p className="mt-2 text-meta font-medium">{title}</p>
      <p className="mx-auto mt-1 max-w-sm text-micro leading-5 text-muted-foreground">
        {body}
      </p>
    </div>
  );
}

function LargeEmpty({
  icon: Icon,
  title,
  body,
}: {
  icon: typeof History;
  title: string;
  body: string;
}) {
  return (
    <div className="rounded-xl border border-dashed border-border bg-surface-1/50 px-6 py-14 text-center">
      <Icon size={22} className="mx-auto text-subtle-foreground" />
      <h3 className="mt-3 text-body font-semibold">{title}</h3>
      <p className="mx-auto mt-1 max-w-md text-meta leading-5 text-muted-foreground">
        {body}
      </p>
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

function ErrorNotice({ message }: { message: string }) {
  return (
    <div
      role="alert"
      className="flex gap-2 rounded-lg border border-destructive/25 bg-destructive/10 px-3 py-2 text-meta text-destructive"
    >
      <AlertTriangle size={15} className="mt-0.5 shrink-0" />
      <span>{message}</span>
    </div>
  );
}

function relativeTime(unixMs: number): string {
  const deltaSeconds = Math.round((unixMs - Date.now()) / 1000);
  const formatter = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });
  if (Math.abs(deltaSeconds) < 60)
    return formatter.format(deltaSeconds, "second");
  const deltaMinutes = Math.round(deltaSeconds / 60);
  if (Math.abs(deltaMinutes) < 60)
    return formatter.format(deltaMinutes, "minute");
  const deltaHours = Math.round(deltaMinutes / 60);
  if (Math.abs(deltaHours) < 24) return formatter.format(deltaHours, "hour");
  const deltaDays = Math.round(deltaHours / 24);
  if (Math.abs(deltaDays) < 30) return formatter.format(deltaDays, "day");
  const deltaMonths = Math.round(deltaDays / 30);
  if (Math.abs(deltaMonths) < 12) return formatter.format(deltaMonths, "month");
  return formatter.format(Math.round(deltaMonths / 12), "year");
}

function isoTime(unixMs: number): string {
  return new Date(unixMs).toISOString();
}

function absoluteTime(unixMs: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(unixMs);
}

function humanize(value: string): string {
  return value
    .replace(/-/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function actionLabel(action: LearningAction): string {
  return action === "mark-stale" ? "Mark stale" : humanize(action);
}

function reviewPlaceholder(action: LearningAction): string {
  if (action === "confirm") return "Explain why this is useful or reliable…";
  if (action === "contest") return "Describe what is uncertain or conflicting…";
  if (action === "reject") return "Explain why agents should not reuse this…";
  return "Describe what changed or became outdated…";
}

function sourceLabel(session: SessionContext): string {
  const parts = [
    humanize(session.source.kind),
    session.source.host,
    session.source.agent,
  ].filter((part): part is string => Boolean(part));
  return parts.join(" · ");
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
