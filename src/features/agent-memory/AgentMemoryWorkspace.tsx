import { lazy, Suspense, useEffect, useState } from "react";
import {
  ArrowLeft,
  BookCheck,
  BrainCircuit,
  FileCheck2,
  Files,
  History,
  Inbox,
  MessageSquareWarning,
  RefreshCw,
  Scale,
  Search,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { Button } from "@/shared/components/Button";
import { cn } from "@/shared/lib/classnames";
import {
  chooseAgentProject,
  chooseLegacyAgentVault,
  connectAgentProject,
  forgetAgentProject,
  initializeAgentProject,
  inspectAgentProject,
  listAgentProjects,
  refreshAgentProject,
} from "./api";
import { ProjectsHub } from "./ProjectsHub";
import { ProjectOnboarding } from "./ProjectOnboarding";
import { ProjectOverview } from "./ProjectOverview";
import {
  errorMessage,
  ErrorNotice,
  KnowledgeSurfaceFallback,
  LargeEmpty,
  PageHeading,
  relativeTime,
  SessionStatus,
} from "./AgentMemoryPresentation";
import { SessionInspector } from "./SessionInspector";
import {
  LearningInspector,
  Lessons,
  ReviewInbox,
} from "./LearningWorkspace";
import type {
  AgentMemoryDashboard,
  AgentMemoryStorage,
  AgentProjectCatalog,
  AgentProjectInspection,
  AgentProjectSearchResult,
  ArtifactEvidenceReference,
  ProjectMemorySearchResult,
  SessionSummary,
} from "./types";

const LAST_AGENT_PROJECT_KEY = "ley:last-agent-project";
type Section =
  | "overview"
  | "search"
  | "sessions"
  | "decisions"
  | "problems"
  | "lessons"
  | "specifications"
  | "artifacts"
  | "review"
  | "privacy";

type ArtifactFocus = {
  path: string;
  evidence?: ArtifactEvidenceReference;
  requestId: number;
};

const ArtifactExplorer = lazy(() =>
  import("./ArtifactExplorer").then((module) => ({
    default: module.ArtifactExplorer,
  })),
);
const MemorySearch = lazy(() =>
  import("./MemorySearch").then((module) => ({
    default: module.MemorySearch,
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
const SpecificationsPanel = lazy(() =>
  import("./SpecificationsPanel").then((module) => ({
    default: module.SpecificationsPanel,
  })),
);
export function AgentMemoryWorkspace() {
  const [section, setSection] = useState<Section>("overview");
  const [projectPath, setProjectPath] = useState<string | null>(null);
  const [catalog, setCatalog] = useState<AgentProjectCatalog | null>(null);
  const [catalogBusy, setCatalogBusy] = useState(true);
  const [catalogRevision, setCatalogRevision] = useState(0);
  const [inspection, setInspection] = useState<AgentProjectInspection | null>(
    null,
  );
  const [inspectedPath, setInspectedPath] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [learningId, setLearningId] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [artifactFocus, setArtifactFocus] = useState<ArtifactFocus | null>(
    null,
  );

  useEffect(() => {
    if (projectPath || catalog) return;
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
  }, [catalog, catalogRevision, projectPath]);

  useEffect(() => {
    if (!projectPath || inspectedPath === projectPath) return;
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
  }, [inspectedPath, projectPath]);

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
    if (!destination) {
      setSection("overview");
      return;
    }
    if (
      destination.kind === "session" ||
      destination.kind === "revision" ||
      destination.kind === "decision" ||
      destination.kind === "problem"
    ) {
      setSection(
        destination.kind === "session" || destination.kind === "revision"
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
      if (destination.kind === "artifact") {
        if (destination.citation) {
          setArtifactFocus({
            path: destination.citation.artifactPath,
            evidence: destination.citation,
            requestId: Date.now(),
          });
          setSection("artifacts");
        } else {
          setArtifactFocus({ path: destination.title, requestId: Date.now() });
          setSection("artifacts");
        }
      } else if (destination.citation) {
        setArtifactFocus({
          path: destination.citation.artifactPath,
          evidence: destination.citation,
          requestId: Date.now(),
        });
        setSection("artifacts");
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
    setArtifactFocus({
      path: evidence.artifactPath,
      evidence,
      requestId: Date.now(),
    });
    setSection("artifacts");
  }

  function openMemoryResult(result: ProjectMemorySearchResult) {
    if (result.learningId) {
      setLearningId(result.learningId);
      return;
    }
    if (result.sessionId) {
      setSessionId(result.sessionId);
      return;
    }
    if (result.kind === "artifact") {
      if (result.citation) openEvidence(result.citation);
      else openArtifact(result.title);
      return;
    }
    if (result.citation) openEvidence(result.citation);
  }

  async function makeReady(kind: "initialize" | "connect" | "capture") {
    if (!projectPath) return;
    setBusy(true);
    setError(null);
    try {
      let dashboard: AgentMemoryDashboard;
      if (kind === "initialize") {
        if (inspection?.status !== "uninitialized") {
          throw new Error(
            "Review the current capture preview before initializing Agent Memory.",
          );
        }
        dashboard = await initializeAgentProject(
          projectPath,
          inspection.preview.approvalFingerprint,
        );
      } else if (kind === "connect") {
        const legacyVaultPath = await chooseLegacyAgentVault();
        if (!legacyVaultPath) return;
        dashboard = await connectAgentProject(projectPath, legacyVaultPath);
      } else {
        dashboard = await refreshAgentProject(projectPath);
      }
      setInspection({ status: "ready", dashboard });
      setSection("overview");
    } catch (cause) {
      setError(errorMessage(cause));
      if (kind === "initialize" || kind === "connect") {
        try {
          const next = await inspectAgentProject(projectPath);
          setInspection(next);
          setInspectedPath(projectPath);
        } catch {
          // Preserve the original initialization error; a later reopen retries inspection.
        }
      }
    } finally {
      setBusy(false);
    }
  }

  async function refresh() {
    await makeReady("capture");
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

  function changeSection(nextSection: Section) {
    if (nextSection === "artifacts") setArtifactFocus(null);
    setSection(nextSection);
  }

  function updateInspection(nextDashboard: AgentMemoryDashboard) {
    setInspection({ status: "ready", dashboard: nextDashboard });
  }

  function erasePrivacy(nextInspection: AgentProjectInspection) {
    setInspection(nextInspection);
    setSection("overview");
  }

  function eraseSession(nextDashboard: AgentMemoryDashboard) {
    updateInspection(nextDashboard);
    setSessionId(null);
  }

  function reviewLearning(nextDashboard: AgentMemoryDashboard) {
    updateInspection(nextDashboard);
    setLearningId(null);
  }

  return (
    <AgentMemoryWorkspaceView
      projectPath={projectPath}
      projectLabel={projectLabel}
      catalog={catalog}
      catalogBusy={catalogBusy}
      error={error}
      busy={busy}
      inspection={inspection}
      dashboard={dashboard}
      sessionId={sessionId}
      learningId={learningId}
      section={section}
      artifactFocus={artifactFocus}
      onChooseProject={chooseProject}
      onOpenProject={openProject}
      onForgetProject={removeProject}
      onReloadProjects={() => {
        setCatalog(null);
        setCatalogBusy(true);
        setError(null);
        setCatalogRevision((value) => value + 1);
      }}
      onReturnToProjects={returnToProjects}
      onMakeReady={makeReady}
      onRefresh={refresh}
      onSection={changeSection}
      onMemoryResult={openMemoryResult}
      onEvidence={openEvidence}
      onLearning={setLearningId}
      onSession={setSessionId}
      onSessionClose={() => setSessionId(null)}
      onLearningClose={() => setLearningId(null)}
      onSessionRenamed={updateInspection}
      onSessionErased={eraseSession}
      onLearningSession={(nextSessionId) => {
        setLearningId(null);
        setSessionId(nextSessionId);
      }}
      onLearningReviewed={reviewLearning}
      onPrivacyUpdated={updateInspection}
      onPrivacyErased={erasePrivacy}
    />
  );
}

interface AgentMemoryWorkspaceViewProps {
  projectPath: string | null;
  projectLabel: string | null;
  catalog: AgentProjectCatalog | null;
  catalogBusy: boolean;
  error: string | null;
  busy: boolean;
  inspection: AgentProjectInspection | null;
  dashboard: AgentMemoryDashboard | null;
  sessionId: string | null;
  learningId: string | null;
  section: Section;
  artifactFocus: ArtifactFocus | null;
  onChooseProject: () => Promise<void>;
  onOpenProject: (
    projectPath: string,
    destination?: AgentProjectSearchResult,
  ) => void;
  onForgetProject: (projectId: string) => Promise<void>;
  onReloadProjects: () => void;
  onReturnToProjects: () => void;
  onMakeReady: (kind: "initialize" | "connect" | "capture") => Promise<void>;
  onRefresh: () => Promise<void>;
  onSection: (section: Section) => void;
  onMemoryResult: (result: ProjectMemorySearchResult) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  onLearning: (id: string) => void;
  onSession: (id: string) => void;
  onSessionClose: () => void;
  onLearningClose: () => void;
  onSessionRenamed: (dashboard: AgentMemoryDashboard) => void;
  onSessionErased: (dashboard: AgentMemoryDashboard) => void;
  onLearningSession: (sessionId: string) => void;
  onLearningReviewed: (dashboard: AgentMemoryDashboard) => void;
  onPrivacyUpdated: (dashboard: AgentMemoryDashboard) => void;
  onPrivacyErased: (inspection: AgentProjectInspection) => void;
}

function AgentMemoryWorkspaceView({
  projectPath,
  projectLabel,
  catalog,
  catalogBusy,
  error,
  busy,
  inspection,
  dashboard,
  sessionId,
  learningId,
  section,
  artifactFocus,
  onChooseProject,
  onOpenProject,
  onForgetProject,
  onReloadProjects,
  onReturnToProjects,
  onMakeReady,
  onRefresh,
  onSection,
  onMemoryResult,
  onEvidence,
  onLearning,
  onSession,
  onSessionClose,
  onLearningClose,
  onSessionRenamed,
  onSessionErased,
  onLearningSession,
  onLearningReviewed,
  onPrivacyUpdated,
  onPrivacyErased,
}: AgentMemoryWorkspaceViewProps) {
  return (
    <div
      data-page="agent-memory-workspace"
      className="flex h-full min-h-0 flex-col overflow-hidden bg-background text-foreground"
    >
      <AgentMemoryHeader
        projectPath={projectPath}
        projectLabel={projectLabel}
        catalog={catalog}
        dashboard={dashboard}
        busy={busy}
        onReturnToProjects={onReturnToProjects}
        onRefresh={onRefresh}
      />
      <AgentMemoryBody
        projectPath={projectPath}
        catalog={catalog}
        catalogBusy={catalogBusy}
        error={error}
        busy={busy}
        inspection={inspection}
        section={section}
        artifactFocus={artifactFocus}
        onChooseProject={onChooseProject}
        onOpenProject={onOpenProject}
        onForgetProject={onForgetProject}
        onReloadProjects={onReloadProjects}
        onReturnToProjects={onReturnToProjects}
        onMakeReady={onMakeReady}
        onSection={onSection}
        onMemoryResult={onMemoryResult}
        onEvidence={onEvidence}
        onLearning={onLearning}
        onSession={onSession}
        onPrivacyUpdated={onPrivacyUpdated}
        onPrivacyErased={onPrivacyErased}
      />
      {dashboard && projectPath && (
        <SessionInspector
          key={`session-${sessionId ?? "closed"}`}
          sessionId={sessionId}
          projectPath={projectPath}
          onClose={onSessionClose}
          onEvidence={onEvidence}
          onRenamed={onSessionRenamed}
          onErased={onSessionErased}
        />
      )}
      {dashboard && projectPath && (
        <LearningInspector
          key={`learning-${learningId ?? "closed"}`}
          learningId={learningId}
          projectPath={projectPath}
          candidates={dashboard.allLearnings.learnings}
          candidatesOmitted={dashboard.allLearnings.omittedLearnings}
          onClose={onLearningClose}
          onLearning={onLearning}
          onSession={onLearningSession}
          onEvidence={onEvidence}
          onReviewed={onLearningReviewed}
        />
      )}
    </div>
  );
}

function AgentMemoryHeader({
  projectPath,
  projectLabel,
  catalog,
  dashboard,
  busy,
  onReturnToProjects,
  onRefresh,
}: {
  projectPath: string | null;
  projectLabel: string | null;
  catalog: AgentProjectCatalog | null;
  dashboard: AgentMemoryDashboard | null;
  busy: boolean;
  onReturnToProjects: () => void;
  onRefresh: () => Promise<void>;
}) {
  return (
    <header className="app-chrome flex h-14 shrink-0 items-center justify-between px-3 sm:px-5">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-md border border-primary/25 bg-primary/10 text-primary">
          <BrainCircuit size={17} aria-hidden="true" />
        </div>
        <div className="min-w-0">
          <h1 className="truncate text-body font-semibold tracking-tight">
            {projectPath ? "Agent Memory" : "Projects"}
          </h1>
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
            onClick={onReturnToProjects}
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
            onClick={() => void onRefresh()}
            title="Capture changed project files and rebuild memory"
          >
            <RefreshCw
              size={13}
              className={
                busy ? "animate-spin motion-reduce:animate-none" : undefined
              }
            />
            <span className="hidden sm:inline">Refresh snapshot</span>
          </Button>
        )}
      </div>
    </header>
  );
}

function AgentMemoryBody({
  projectPath,
  catalog,
  catalogBusy,
  error,
  busy,
  inspection,
  section,
  artifactFocus,
  onChooseProject,
  onOpenProject,
  onForgetProject,
  onReloadProjects,
  onReturnToProjects,
  onMakeReady,
  onSection,
  onMemoryResult,
  onEvidence,
  onLearning,
  onSession,
  onPrivacyUpdated,
  onPrivacyErased,
}: Pick<
  AgentMemoryWorkspaceViewProps,
  | "projectPath"
  | "catalog"
  | "catalogBusy"
  | "error"
  | "busy"
  | "inspection"
  | "section"
  | "artifactFocus"
  | "onChooseProject"
  | "onOpenProject"
  | "onForgetProject"
  | "onReloadProjects"
  | "onReturnToProjects"
  | "onMakeReady"
  | "onSection"
  | "onMemoryResult"
  | "onEvidence"
  | "onLearning"
  | "onSession"
  | "onPrivacyUpdated"
  | "onPrivacyErased"
>) {
  if (!projectPath) {
    return (
      <ProjectsHub
        catalog={catalog}
        loading={catalogBusy}
        error={error}
        onAdd={() => void onChooseProject()}
        onOpen={onOpenProject}
        onForget={(projectId) => void onForgetProject(projectId)}
        onReload={onReloadProjects}
      />
    );
  }
  if (!inspection || inspection.status !== "ready") {
    return (
      <ProjectOnboarding
        inspection={inspection}
        projectPath={projectPath}
        busy={busy}
        error={error}
        onChoose={() => void onChooseProject()}
        onForget={onReturnToProjects}
        onInitialize={() => void onMakeReady("initialize")}
        onConnect={() => void onMakeReady("connect")}
        onCapture={() => void onMakeReady("capture")}
      />
    );
  }
  return (
    <AgentMemoryReadyContent
      dashboard={inspection.dashboard}
      section={section}
      projectPath={projectPath}
      error={error}
      busy={busy}
      artifactFocus={artifactFocus}
      onSection={onSection}
      onChangeProject={onReturnToProjects}
      onMemoryResult={onMemoryResult}
      onEvidence={onEvidence}
      onLearning={onLearning}
      onSession={onSession}
      onPrivacyUpdated={onPrivacyUpdated}
      onPrivacyErased={onPrivacyErased}
    />
  );
}

function AgentMemoryReadyContent({
  dashboard,
  section,
  projectPath,
  error,
  busy,
  artifactFocus,
  onSection,
  onChangeProject,
  onMemoryResult,
  onEvidence,
  onLearning,
  onSession,
  onPrivacyUpdated,
  onPrivacyErased,
}: {
  dashboard: AgentMemoryDashboard;
  section: Section;
  projectPath: string;
  error: string | null;
  busy: boolean;
  artifactFocus: ArtifactFocus | null;
  onSection: (section: Section) => void;
  onChangeProject: () => void;
  onMemoryResult: (result: ProjectMemorySearchResult) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  onLearning: (id: string) => void;
  onSession: (id: string) => void;
  onPrivacyUpdated: (dashboard: AgentMemoryDashboard) => void;
  onPrivacyErased: (inspection: AgentProjectInspection) => void;
}) {
  return (
    <div className="flex min-h-0 flex-1 flex-col md:flex-row">
      <AgentMemoryNav
        section={section}
        dashboard={dashboard}
        busy={busy}
        onSection={onSection}
        onChangeProject={onChangeProject}
      />
      <main className="min-h-0 min-w-0 flex-1 overflow-y-auto overscroll-contain">
        <div className="mx-auto w-full max-w-6xl px-4 py-6 sm:px-6 sm:py-8 lg:px-10">
          <AgentMemorySectionContent
            section={section}
            projectPath={projectPath}
            dashboard={dashboard}
            error={error}
            artifactFocus={artifactFocus}
            onSection={onSection}
            onMemoryResult={onMemoryResult}
            onEvidence={onEvidence}
            onLearning={onLearning}
            onSession={onSession}
            onPrivacyUpdated={onPrivacyUpdated}
            onPrivacyErased={onPrivacyErased}
          />
        </div>
      </main>
    </div>
  );
}

function AgentMemorySectionContent({
  section,
  projectPath,
  dashboard,
  error,
  artifactFocus,
  onSection,
  onMemoryResult,
  onEvidence,
  onLearning,
  onSession,
  onPrivacyUpdated,
  onPrivacyErased,
}: {
  section: Section;
  projectPath: string;
  dashboard: AgentMemoryDashboard;
  error: string | null;
  artifactFocus: ArtifactFocus | null;
  onSection: (section: Section) => void;
  onMemoryResult: (result: ProjectMemorySearchResult) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  onLearning: (id: string) => void;
  onSession: (id: string) => void;
  onPrivacyUpdated: (dashboard: AgentMemoryDashboard) => void;
  onPrivacyErased: (inspection: AgentProjectInspection) => void;
}) {
  return (
    <>
      {error && <ErrorNotice message={error} />}
      {section === "overview" && (
        <ProjectOverview
          projectPath={projectPath}
          dashboard={dashboard}
          onOpenSession={() => onSection("sessions")}
          onOpenReview={() => onSection("review")}
          onEvidence={onEvidence}
          onLearning={onLearning}
          onSession={onSession}
        />
      )}
      {section === "search" && (
        <Suspense fallback={<KnowledgeSurfaceFallback />}>
          <MemorySearch
            projectPath={projectPath}
            projectName={dashboard.overview.projectName}
            onOpen={onMemoryResult}
          />
        </Suspense>
      )}
      {section === "sessions" && (
        <Sessions sessions={dashboard.sessions} onSession={onSession} />
      )}
      {(section === "decisions" || section === "problems") && (
        <Suspense fallback={<KnowledgeSurfaceFallback />}>
          <ProjectActivityExplorer
            mode={section}
            projectPath={projectPath}
            onSession={onSession}
            onEvidence={onEvidence}
          />
        </Suspense>
      )}
      {section === "lessons" && (
        <Lessons dashboard={dashboard} onLearning={onLearning} />
      )}
      {section === "specifications" && (
        <Suspense fallback={<KnowledgeSurfaceFallback />}>
          <SpecificationsPanel projectPath={projectPath} />
        </Suspense>
      )}
      {section === "artifacts" && (
        <Suspense fallback={<KnowledgeSurfaceFallback />}>
          <ArtifactExplorer
            key={`artifacts-${artifactFocus?.requestId ?? "browse"}`}
            projectPath={projectPath}
            focus={artifactFocus}
          />
        </Suspense>
      )}
      {section === "review" && (
        <ReviewInbox dashboard={dashboard} onLearning={onLearning} />
      )}
      {section === "privacy" && (
        <Suspense fallback={<KnowledgeSurfaceFallback />}>
          <CapturePrivacyPanel
            key={projectPath}
            projectPath={projectPath}
            dashboard={dashboard}
            onUpdated={onPrivacyUpdated}
            onErased={onPrivacyErased}
          />
        </Suspense>
      )}
    </>
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
    { id: "search", label: "Search memory", icon: Search },
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
      id: "specifications",
      label: "Specifications",
      icon: FileCheck2,
    },
    {
      id: "artifacts",
      label: "Artifacts",
      icon: Files,
      count: dashboard.overview.files,
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
                    "ml-auto rounded-sm px-1.5 text-micro tabular-nums",
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
          {agentMemoryStorageLabel(dashboard.storage)}
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
      className="w-full cursor-pointer rounded-md border border-border bg-surface-1 p-4 text-left shadow-panel hover:border-border-strong hover:bg-surface-2/55 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary sm:p-5"
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
          {session.checkpoints} checkpoints
          {((session.prompts ?? 0) > 0 || (session.responses ?? 0) > 0) &&
            ` · ${(session.prompts ?? 0) + (session.responses ?? 0)} turns`}
          {` · ${session.eventCount} events`}
        </span>
      </div>
    </button>
  );
}




function agentMemoryStorageLabel(storage: AgentMemoryStorage): string {
  return storage.kind === "native"
    ? "Ley private local storage"
    : storage.vaultName;
}
