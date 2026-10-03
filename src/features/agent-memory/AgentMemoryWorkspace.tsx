import { lazy, Suspense, useEffect, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  BookCheck,
  BrainCircuit,
  Cable,
  Check,
  CheckCircle2,
  ChevronRight,
  CircleDot,
  Clock3,
  FileCheck2,
  FileCode2,
  Files,
  FolderOpen,
  History,
  Inbox,
  LockKeyhole,
  MessageSquareWarning,
  PencilLine,
  RefreshCw,
  RotateCcw,
  Scale,
  Search,
  ShieldCheck,
  Sparkles,
  X,
  XCircle,
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
  readAgentLearning,
  refreshAgentProject,
  reviewAgentLearning,
} from "./api";
import { ProjectsHub } from "./ProjectsHub";
import {
  absoluteTime,
  CompactEmpty,
  errorMessage,
  ErrorNotice,
  humanize,
  KnowledgeSurfaceFallback,
  relativeTime,
  SessionStatus,
} from "./AgentMemoryPresentation";
import { SessionInspector } from "./SessionInspector";
import type {
  AgentInitialCapturePreview,
  AgentMemoryDashboard,
  AgentMemoryStorage,
  AgentProjectCatalog,
  AgentProjectInspection,
  AgentProjectSearchResult,
  ArtifactEvidenceReference,
  LearningAction,
  LearningContext,
  LearningSummary,
  ProjectMemorySearchResult,
  ResumeSession,
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
const AgentBriefPreview = lazy(() =>
  import("./AgentBriefPreview").then((module) => ({
    default: module.AgentBriefPreview,
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
const LearningCorrectionEditor = lazy(() =>
  import("./LearningCorrectionEditor").then((module) => ({
    default: module.LearningCorrectionEditor,
  })),
);
export function AgentMemoryWorkspace({
  open,
  onClose,
  closable = true,
}: {
  open: boolean;
  onClose: () => void;
  closable?: boolean;
}) {
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
    if (!open || projectPath || catalog) return;
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
  }, [catalog, catalogRevision, open, projectPath]);

  useEffect(() => {
    if (!open || !projectPath || inspectedPath === projectPath) return;
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
  }, [inspectedPath, open, projectPath]);

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

  if (!open) return null;

  return (
    <AgentMemoryWorkspaceView
      open={open}
      onClose={onClose}
      closable={closable}
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
  open: boolean;
  onClose: () => void;
  closable: boolean;
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
  open,
  onClose,
  closable,
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
          <AgentMemoryHeader
            projectPath={projectPath}
            projectLabel={projectLabel}
            catalog={catalog}
            dashboard={dashboard}
            busy={busy}
            onReturnToProjects={onReturnToProjects}
            onRefresh={onRefresh}
            onClose={onClose}
            closable={closable}
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
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
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
  onClose,
  closable,
}: {
  projectPath: string | null;
  projectLabel: string | null;
  catalog: AgentProjectCatalog | null;
  dashboard: AgentMemoryDashboard | null;
  busy: boolean;
  onReturnToProjects: () => void;
  onRefresh: () => Promise<void>;
  onClose: () => void;
  closable: boolean;
}) {
  return (
    <header className="app-chrome flex h-14 shrink-0 items-center justify-between px-3 sm:px-5">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-md border border-primary/25 bg-primary/10 text-primary">
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
        {closable && (
          <Button
            size="sm"
            variant="ghost"
            onClick={onClose}
            aria-label="Close Agent Memory"
            title="Close Agent Memory"
          >
            <X size={16} />
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
        <Overview
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

function Overview({
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

function LearningInspector({
  learningId,
  projectPath,
  candidates,
  candidatesOmitted,
  onClose,
  onLearning,
  onSession,
  onEvidence,
  onReviewed,
}: {
  learningId: string | null;
  projectPath: string;
  candidates: LearningSummary[];
  candidatesOmitted: number;
  onClose: () => void;
  onLearning: (learningId: string) => void;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
  onReviewed: (dashboard: AgentMemoryDashboard) => void;
}) {
  const [learning, setLearning] = useState<LearningContext | null>(null);
  const [action, setAction] = useState<LearningAction | null>(null);
  const [note, setNote] = useState("");
  const [replacementLearningId, setReplacementLearningId] = useState("");
  const [correcting, setCorrecting] = useState(false);
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
    action === "contest" ||
    action === "reject" ||
    action === "mark-stale" ||
    action === "supersede";
  const selectedReplacement =
    action === "supersede"
      ? candidates.find(
          (candidate) => candidate.learningId === replacementLearningId,
        )
      : undefined;
  const canSubmit =
    action &&
    (!noteRequired || note.trim().length > 0) &&
    (action !== "supersede" || selectedReplacement !== undefined);
  const terminal =
    learning?.state === "rejected" || learning?.state === "superseded";
  const replacementSummary =
    learning?.supersededBy === undefined
      ? undefined
      : candidates.find(
          (candidate) => candidate.learningId === learning.supersededBy,
        );

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
        action === "supersede" ? replacementLearningId : null,
        selectedReplacement?.eventCount ?? null,
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
    setReplacementLearningId("");
    setCorrecting(true);
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
          className="app-modal-surface fixed inset-x-3 bottom-3 top-3 z-[81] mx-auto flex max-w-3xl flex-col overflow-hidden rounded-sm border outline-none focus-visible:ring-2 focus-visible:ring-primary sm:inset-x-6 sm:bottom-6 sm:top-6"
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
          <LearningInspectorBody
            learning={learning}
            busy={busy}
            error={error}
            replacementSummary={replacementSummary}
            onLearning={onLearning}
            onSession={onSession}
            onEvidence={onEvidence}
          />
          <LearningInspectorFooter
            learning={learning}
            terminal={terminal}
            correcting={correcting}
            action={action}
            note={note}
            replacementLearningId={replacementLearningId}
            candidates={candidates}
            candidatesOmitted={candidatesOmitted}
            noteRequired={noteRequired}
            canSubmit={Boolean(canSubmit)}
            busy={busy}
            error={error}
            projectPath={projectPath}
            onReviewed={onReviewed}
            onBeginCorrection={beginCorrection}
            onSetAction={setAction}
            onSetNote={setNote}
            onSetReplacementLearningId={setReplacementLearningId}
            onSetError={setError}
            onSetCorrecting={setCorrecting}
            onSubmitReview={submitReview}
          />
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function LearningInspectorBody({
  learning,
  busy,
  error,
  replacementSummary,
  onLearning,
  onSession,
  onEvidence,
}: {
  learning: LearningContext | null;
  busy: boolean;
  error: string | null;
  replacementSummary?: LearningSummary;
  onLearning: (learningId: string) => void;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-4 sm:p-6">
      {busy && !learning ? (
        <div className="py-20 text-center text-meta text-muted-foreground">
          Reading cited memory…
        </div>
      ) : error && !learning ? (
        <ErrorNotice message={error} />
      ) : learning ? (
        <div className="space-y-6">
          <LearningOverview
            learning={learning}
            replacementSummary={replacementSummary}
            onLearning={onLearning}
          />
          <LearningOriginLineage learning={learning} onSession={onSession} />
          <LearningEvidence
            learning={learning}
            onSession={onSession}
            onEvidence={onEvidence}
          />
          <LearningApplications learning={learning} onSession={onSession} />
          <LearningHistory learning={learning} />
          {learning.claimTruncated && (
            <p className="rounded-md border border-warning/25 bg-warning/8 p-3 text-micro text-muted-foreground">
              The title or guidance was truncated to keep this inspector
              bounded. Use the CLI for the complete claim before correcting or
              reviewing it.
            </p>
          )}
          <p className="rounded-md border border-border bg-background/35 p-3 text-micro leading-5 text-muted-foreground">
            <MessageSquareWarning
              size={13}
              className="mr-2 inline text-secondary"
            />
            {learning.instructionWarning}
          </p>
        </div>
      ) : null}
    </div>
  );
}

function LearningOverview({
  learning,
  replacementSummary,
  onLearning,
}: {
  learning: LearningContext;
  replacementSummary?: LearningSummary;
  onLearning: (learningId: string) => void;
}) {
  return (
    <>
      <div className="flex flex-wrap gap-2">
        <StatusPill
          tone={learning.trustedForReuse ? "success" : "warning"}
          label={humanize(learning.trustState)}
          icon={learning.trustedForReuse ? CheckCircle2 : AlertTriangle}
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
          tone={learning.freshness === "current" ? "success" : "warning"}
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
        <dl className="mt-2 grid gap-2 rounded-md border border-border bg-background/35 p-3 text-meta sm:grid-cols-3">
          <div>
            <dt className="text-micro text-muted-foreground">Created</dt>
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
            <dt className="text-micro text-muted-foreground">Ledger</dt>
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
        {learning.supersededBy && (
          <p className="mt-2 text-micro text-muted-foreground">
            Superseded by{" "}
            {replacementSummary ? (
              <button
                type="button"
                onClick={() => onLearning(replacementSummary.learningId)}
                className="font-medium text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
              >
                {replacementSummary.title} ·{" "}
                {humanize(replacementSummary.state)} ·{" "}
                {compactId(replacementSummary.learningId)}
              </button>
            ) : (
              <span className="font-mono text-foreground">
                {learning.supersededBy}
              </span>
            )}
            .
          </p>
        )}
      </section>
    </>
  );
}

function LearningEvidence({
  learning,
  onSession,
  onEvidence,
}: {
  learning: LearningContext;
  onSession: (sessionId: string) => void;
  onEvidence: (evidence: ArtifactEvidenceReference) => void;
}) {
  return (
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
              className="rounded-md border border-border bg-background/35 p-3"
            >
              <div className="flex flex-wrap items-center justify-between gap-2 text-micro text-muted-foreground">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="rounded bg-surface-3 px-1.5 py-0.5 font-medium text-muted-foreground-strong">
                    {humanize(evidence.recordType)}
                  </span>
                  <span>{relativeTime(evidence.sessionUpdatedAtUnixMs)}</span>
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
                      title={
                        artifact.mediaType
                          ? `Original ${artifact.mediaType} evidence · snapshot ${artifact.artifactSnapshotId}`
                          : `${artifact.artifactPath}:${artifact.startLine}-${artifact.endLine} · snapshot ${artifact.artifactSnapshotId}`
                      }
                      onClick={() => onEvidence(artifact)}
                      className="max-w-full touch-manipulation truncate rounded-sm border border-border bg-surface-2 px-2 py-1 text-left font-mono text-micro text-muted-foreground outline-none transition-[transform,border-color,background-color,color] hover:border-primary/35 hover:bg-primary/7 hover:text-foreground active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
                    >
                      {artifact.artifactPath}
                      {artifact.mediaType
                        ? ` · ${artifact.mediaType}`
                        : `:${artifact.startLine}`}
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
          {learning.omittedEvidence === 1 ? "reference is" : "references are"}{" "}
          omitted from this bounded inspector.
        </p>
      )}
    </section>
  );
}

function LearningOriginLineage({
  learning,
  onSession,
}: {
  learning: LearningContext;
  onSession: (sessionId: string) => void;
}) {
  return (
    <section>
      <h3 className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
        Origin lineage · {learning.originSourceCount}
      </h3>
      <dl className="mt-2 grid gap-2 rounded-md border border-border bg-background/35 p-3 text-meta sm:grid-cols-3">
        <div>
          <dt className="text-micro text-muted-foreground">Resolution</dt>
          <dd className="mt-0.5 font-medium">
            {learning.originLineage.mechanicallyResolved
              ? "Mechanically resolved"
              : "Incomplete lineage"}
          </dd>
        </div>
        <div>
          <dt className="text-micro text-muted-foreground">
            Causal completeness
          </dt>
          <dd className="mt-0.5 font-medium">
            {learning.originLineage.causalCompletenessProven
              ? "Proven"
              : "Not proven"}
          </dd>
        </div>
        <div>
          <dt className="text-micro text-muted-foreground">
            Automatic authority ceiling
          </dt>
          <dd className="mt-0.5 font-medium">
            {humanize(learning.originLineage.automaticAuthorityCeiling)}
          </dd>
        </div>
      </dl>
      {learning.originLineage.sources.length > 0 && (
        <div className="mt-2 space-y-2">
          {learning.originLineage.sources.map((source, index) => {
            const key = learningOriginSourceKey(source, index);
            return (
              <div
                key={key}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border bg-background/35 p-3 text-micro"
              >
                <div className="min-w-0">
                  <span className="font-medium text-muted-foreground-strong">
                    {humanize(source.kind)}
                  </span>
                  <p className="mt-0.5 break-all font-mono text-muted-foreground">
                    {learningOriginSourceHandle(source)}
                  </p>
                </div>
                {source.kind === "captured-artifact" ? (
                  <span
                    className="text-micro font-medium text-muted-foreground"
                    title="This lineage row is an exact provenance identity, not a complete text/media read citation. Open the matching evidence reference below when available."
                  >
                    Provenance handle
                  </span>
                ) : (
                  <button
                    type="button"
                    onClick={() => onSession(source.sessionId)}
                    className="touch-manipulation rounded font-semibold text-primary outline-none transition-transform hover:underline active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
                  >
                    Open session
                  </button>
                )}
              </div>
            );
          })}
        </div>
      )}
      {learning.omittedOriginSources > 0 && (
        <p className="mt-2 text-micro text-muted-foreground">
          {learning.omittedOriginSources} origin sources are outside this
          bounded view.
        </p>
      )}
      <p className="mt-2 text-micro leading-5 text-muted-foreground">
        Lineage records where this learning came from. It does not increase
        authority or prove that the recorded sources are causally complete.
      </p>
    </section>
  );
}

function learningOriginSourceKey(
  source: LearningContext["originLineage"]["sources"][number],
  index: number,
): string {
  switch (source.kind) {
    case "session-record":
      return `${source.kind}:${source.sessionId}:${source.recordId}`;
    case "captured-artifact":
      return `${source.kind}:${source.artifactSnapshotId}:${source.artifactPath}`;
    case "turn-evidence":
      return `${source.kind}:${source.sessionId}:${source.recordId}`;
    case "tool-evidence":
      return `${source.kind}:${source.sessionId}:${source.recordId}`;
    case "recovery-candidate":
      return `${source.kind}:${source.sessionId}:${source.candidateFingerprint}`;
    default:
      return `origin:${index}`;
  }
}

function learningOriginSourceHandle(
  source: LearningContext["originLineage"]["sources"][number],
): string {
  switch (source.kind) {
    case "session-record":
      return `${source.sessionId} · ${source.recordType} · ${source.recordId}`;
    case "captured-artifact":
      return `${source.artifactPath} · ${source.artifactSnapshotId}`;
    case "turn-evidence":
      return `${source.sessionId} · ${source.recordId}`;
    case "tool-evidence":
      return `${source.sessionId} · ${source.recordId}`;
    case "recovery-candidate":
      return `${source.sessionId} · ${source.candidateFingerprint}`;
  }
}

function LearningHistory({ learning }: { learning: LearningContext }) {
  if (learning.history.length === 0) return null;
  return (
    <section>
      <h3 className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
        Review history
      </h3>
      <ol className="mt-2 space-y-2">
        {learning.history.map((entry) => (
          <li key={entry.eventId} className="flex gap-3 text-meta">
            <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-border-strong" />
            <span className="min-w-0 break-words">
              <span className="font-medium">{humanize(entry.action)}</span> by{" "}
              {humanize(entry.actor)} ·{" "}
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
          Showing {learning.history.length} of {learning.historyCount} immutable
          history events.
        </p>
      )}
    </section>
  );
}

function LearningApplications({
  learning,
  onSession,
}: {
  learning: LearningContext;
  onSession: (sessionId: string) => void;
}) {
  if (learning.applicationObservations.length === 0) return null;

  return (
    <section>
      <h3 className="text-micro font-semibold uppercase tracking-[0.14em] text-muted-foreground">
        Procedure application history · {learning.applicationObservationCount}
      </h3>
      <div className="mt-2 space-y-2">
        {learning.applicationObservations.map((application) => (
          <article
            key={application.observationId}
            className="rounded-md border border-border bg-background/35 p-3"
          >
            <div className="flex flex-wrap items-center justify-between gap-2 text-micro text-muted-foreground">
              <div className="flex flex-wrap items-center gap-2">
                <span
                  className={cn(
                    "rounded px-1.5 py-0.5 font-medium",
                    application.learningVersionMatchesCurrent
                      ? "bg-success/10 text-success"
                      : "bg-warning/10 text-warning",
                  )}
                >
                  {application.learningVersionMatchesCurrent
                    ? "Exact current version"
                    : "Older learning version"}
                </span>
                <time>{relativeTime(application.recordedAtUnixMs)}</time>
              </div>
              <button
                type="button"
                onClick={() => onSession(application.sessionId)}
                className="touch-manipulation rounded font-semibold text-primary outline-none transition-transform hover:underline active:scale-[0.97] motion-reduce:transform-none focus-visible:ring-2 focus-visible:ring-primary"
              >
                Open session
              </button>
            </div>
            {application.taskExcerpt && (
              <p className="mt-2 text-meta leading-5 text-muted-foreground-strong">
                {application.taskExcerpt}
              </p>
            )}
            <p className="mt-2 text-micro leading-5 text-muted-foreground">
              Typed verification outcomes · {application.passedVerifications}{" "}
              passed · {application.failedVerifications} failed ·{" "}
              {application.skippedVerifications} skipped ·{" "}
              {application.unknownVerifications} unknown
            </p>
            <p className="mt-2 text-micro leading-5 text-muted-foreground">
              Caller-declared application only. Procedure following, condition
              applicability, context usage, and causation remain unproven; no
              trust or ranking change was applied.
            </p>
          </article>
        ))}
      </div>
      {learning.omittedApplicationObservations > 0 && (
        <p className="mt-2 text-micro text-muted-foreground">
          {learning.omittedApplicationObservations} older application
          observations are outside this bounded view.
        </p>
      )}
      <p className="mt-2 text-micro leading-5 text-muted-foreground">
        {learning.applicationClaimNotice}
      </p>
    </section>
  );
}

function LearningInspectorFooter({
  learning,
  terminal,
  correcting,
  action,
  note,
  replacementLearningId,
  candidates,
  candidatesOmitted,
  noteRequired,
  canSubmit,
  busy,
  error,
  projectPath,
  onReviewed,
  onBeginCorrection,
  onSetAction,
  onSetNote,
  onSetReplacementLearningId,
  onSetError,
  onSetCorrecting,
  onSubmitReview,
}: {
  learning: LearningContext | null;
  terminal: boolean;
  correcting: boolean;
  action: LearningAction | null;
  note: string;
  replacementLearningId: string;
  candidates: LearningSummary[];
  candidatesOmitted: number;
  noteRequired: boolean;
  canSubmit: boolean;
  busy: boolean;
  error: string | null;
  projectPath: string;
  onReviewed: (dashboard: AgentMemoryDashboard) => void;
  onBeginCorrection: () => void;
  onSetAction: (action: LearningAction | null) => void;
  onSetNote: (note: string) => void;
  onSetReplacementLearningId: (learningId: string) => void;
  onSetError: (error: string | null) => void;
  onSetCorrecting: (correcting: boolean) => void;
  onSubmitReview: () => Promise<void>;
}) {
  if (!learning) return null;
  const replacementCandidates = candidates.filter(
    (candidate) =>
      candidate.learningId !== learning.learningId &&
      candidate.state !== "rejected" &&
      candidate.state !== "superseded",
  );
  return (
    <div className="shrink-0 border-t border-border bg-surface-1 p-4 sm:p-5">
      {learning.claimTruncated ? (
        <p className="text-meta text-muted-foreground">
          This bounded view omits part of the claim. Inspect the complete CLI
          projection before correcting or reviewing it.
        </p>
      ) : terminal ? (
        <p className="text-meta text-muted-foreground">
          This {humanize(learning.state)} learning is preserved as terminal
          history. Create a new learning rather than rewriting it.
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
              onSetCorrecting(false);
              onSetError(null);
            }}
            onCorrected={onReviewed}
          />
        </Suspense>
      ) : !action ? (
        <div className="flex flex-wrap items-center gap-2">
          <span className="mr-auto text-meta font-medium">Your decision</span>
          <Button size="sm" variant="outline" onClick={onBeginCorrection}>
            <PencilLine size={13} />
            Correct
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => onSetAction("mark-stale")}
          >
            <RotateCcw size={13} />
            Mark stale
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => onSetAction("contest")}
          >
            <AlertTriangle size={13} />
            Contest
          </Button>
          <Button
            size="sm"
            variant="destructive"
            onClick={() => onSetAction("reject")}
          >
            <XCircle size={13} />
            Reject
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={replacementCandidates.length === 0}
            title={
              replacementCandidates.length === 0
                ? "No non-terminal replacement is available in this bounded Desktop list. Use the CLI if another learning exists."
                : undefined
            }
            onClick={() => onSetAction("supersede")}
          >
            <ArrowRight size={13} />
            Supersede
          </Button>
          <Button
            size="sm"
            variant="primary"
            onClick={() => onSetAction("confirm")}
          >
            <Check size={13} />
            Confirm
          </Button>
        </div>
      ) : (
        <div>
          {action === "supersede" && (
            <label className="mb-3 block text-meta font-medium">
              <span className="block">Replacement learning · required</span>
              <select
                aria-label="Replacement learning"
                value={replacementLearningId}
                disabled={busy}
                onChange={(event) =>
                  onSetReplacementLearningId(event.target.value)
                }
                className="mt-2 w-full rounded-md border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-primary"
              >
                <option value="">Choose a replacement…</option>
                {replacementCandidates.map((candidate) => (
                  <option
                    key={candidate.learningId}
                    value={candidate.learningId}
                  >
                    {candidate.title} · {humanize(candidate.state)} ·{" "}
                    {humanize(candidate.trustState)} ·{" "}
                    {humanize(candidate.freshness)} ·{" "}
                    {compactId(candidate.learningId)}
                  </option>
                ))}
              </select>
              <span className="mt-1 block text-micro font-normal leading-4 text-muted-foreground">
                The old learning remains immutable history and points to the
                selected replacement. Ley still validates existence and
                supersession cycles before writing. This picker uses the bounded
                Desktop learning list; use the CLI if the intended replacement
                is not shown.
                {candidatesOmitted > 0
                  ? ` ${candidatesOmitted} additional ${candidatesOmitted === 1 ? "learning is" : "learnings are"} omitted from this list.`
                  : ""}
              </span>
            </label>
          )}
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
            onChange={(event) => onSetNote(event.target.value)}
            placeholder={reviewPlaceholder(action)}
            rows={2}
            className="mt-2 w-full resize-none rounded-md border border-border bg-background/45 px-3 py-2 text-meta text-foreground outline-none placeholder:text-subtle-foreground focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-primary"
          />
          {error && (
            <p className="mt-2 text-micro text-destructive" role="alert">
              {error}
            </p>
          )}
          <div className="mt-3 flex justify-end gap-2">
            <Button
              size="sm"
              variant="ghost"
              disabled={busy}
              onClick={() => {
                onSetAction(null);
                onSetNote("");
                onSetReplacementLearningId("");
                onSetError(null);
              }}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              variant={
                action === "reject" || action === "supersede"
                  ? "destructive"
                  : "primary"
              }
              disabled={busy || !canSubmit}
              onClick={() => void onSubmitReview()}
            >
              {busy ? "Saving…" : actionLabel(action)}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

function onboardingCopy(
  inspection: AgentProjectInspection | null,
  projectPath: string | null,
): { title: string; body: string } {
  if (!inspection) {
    return projectPath
      ? {
          title: "Opening local project",
          body: "Ley is validating this project’s local identity, continuity authority, and captured memory.",
        }
      : {
          title: "Choose a project",
          body: "Ley only reads a project after you choose its folder. It never scans neighboring folders or discovers projects silently.",
        };
  }
  switch (inspection.status) {
    case "uninitialized":
      return {
        title: "Review capture before enabling Agent Memory",
        body: `Nothing has been initialized or written yet. Review what Ley would capture from “${inspection.suggestedName}”. Durable continuity will stay in Ley’s private local app storage.`,
      };
    case "unbound":
      return {
        title: "Reconnect historical Ley data",
        body: `“${inspection.projectName}” is a pre-cutover initialized project without a private vault binding. Choose the existing legacy Ley vault that already contains captured memory for this exact project. Ley will not create a new legacy vault.`,
      };
    case "vault-unavailable":
      return {
        title: "Reconnect legacy migration source",
        body: `“${inspection.projectName}” was connected to “${inspection.previousVaultName}”, which moved or is unavailable. Choose that legacy Ley vault explicitly to finish migration into native continuity.`,
      };
    case "needs-capture":
      return {
        title: "Create the first snapshot",
        body:
          inspection.storage.kind === "legacy-vault"
            ? `“${inspection.projectName}” is connected to “${inspection.storage.vaultName}” but has not been captured yet.`
            : `“${inspection.projectName}” has native local continuity authority but has not been captured yet.`,
      };
    case "ready":
      return { title: "Project ready", body: "This project is ready." };
  }
}

function InitialCapturePreviewCard({
  preview,
  storageLabel,
}: {
  preview: AgentInitialCapturePreview;
  storageLabel: string;
}) {
  const hasHardBoundSkips =
    preview.skippedOversized > 0 ||
    preview.skippedTotalLimit > 0 ||
    preview.skippedSymlinks > 0;
  return (
    <div className="mt-4 rounded-md border border-border bg-background/35 p-4">
      <div className="flex items-start gap-3">
        <ShieldCheck size={16} className="mt-0.5 shrink-0 text-primary" />
        <div className="min-w-0 flex-1">
          <p className="text-meta font-semibold">Proposed capture boundary</p>
          <p className="mt-1 text-micro leading-5 text-muted-foreground">
            Structured capture · {preview.eligibleFiles.toLocaleString()}{" "}
            eligible {preview.eligibleFiles === 1 ? "file" : "files"} ·{" "}
            {formatOnboardingBytes(preview.eligibleBytes)} · durable memory in{" "}
            {storageLabel}
          </p>
        </div>
      </div>

      <div className="mt-3 grid gap-2 text-micro sm:grid-cols-2">
        <div className="rounded-sm bg-surface-1 px-3 py-2">
          <span className="text-muted-foreground">Effective roots</span>
          <p className="mt-0.5 font-mono text-foreground">
            {preview.approvedRoots.join(", ")}
          </p>
        </div>
        <div className="rounded-sm bg-surface-1 px-3 py-2">
          <span className="text-muted-foreground">Capture limits</span>
          <p className="mt-0.5 text-foreground">
            {formatOnboardingBytes(preview.maxFileBytes)} per file ·{" "}
            {formatOnboardingBytes(preview.maxTotalBytes)} total
          </p>
        </div>
      </div>

      {preview.includedPaths.length > 0 && (
        <div className="mt-3">
          <p className="text-micro font-medium text-muted-foreground">
            Eligible path sample
          </p>
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {preview.includedPaths.map((path) => (
              <span
                key={path}
                className="max-w-full truncate rounded bg-surface-2 px-2 py-1 font-mono text-micro text-muted-foreground-strong"
                title={path}
              >
                {path}
              </span>
            ))}
            {preview.omittedIncludedPaths > 0 && (
              <span className="rounded bg-surface-2 px-2 py-1 text-micro text-muted-foreground">
                +{preview.omittedIncludedPaths.toLocaleString()} more
              </span>
            )}
          </div>
        </div>
      )}

      {hasHardBoundSkips && (
        <p className="mt-3 text-micro leading-5 text-muted-foreground">
          Also skipped by hard bounds:{" "}
          {preview.skippedOversized.toLocaleString()} oversized,{" "}
          {preview.skippedTotalLimit.toLocaleString()} beyond total limit,{" "}
          {preview.skippedSymlinks.toLocaleString()} symlink
          {preview.skippedSymlinks === 1 ? "" : "s"}.
        </p>
      )}
      <p className="mt-3 text-micro leading-5 text-muted-foreground">
        {preview.exclusionNotice}
      </p>
      <p className="mt-2 text-micro leading-5 text-muted-foreground-strong">
        {preview.privacyNotice}
      </p>
    </div>
  );
}

function ProjectOnboarding({
  inspection,
  projectPath,
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
  busy: boolean;
  error: string | null;
  onChoose: () => void;
  onForget: () => void;
  onInitialize: () => void;
  onConnect: () => void;
  onCapture: () => void;
}) {
  const copy = onboardingCopy(inspection, projectPath);
  const primaryAction = inspection
    ? onboardingPrimaryAction(inspection, onInitialize, onConnect, onCapture)
    : null;
  return (
    <main className="min-h-0 flex-1 overflow-y-auto px-4 py-10 sm:px-6">
      <div className="mx-auto flex min-h-full max-w-xl items-center justify-center">
        <div className="w-full rounded-sm border border-border bg-surface-1 p-6 shadow-panel sm:p-8">
          <div className="flex size-11 items-center justify-center rounded-md border border-primary/20 bg-primary/10 text-primary">
            {inspection ? <BrainCircuit size={21} /> : <FolderOpen size={21} />}
          </div>
          <h2 className="mt-5 text-2xl font-semibold tracking-[-0.035em]">
            {copy.title}
          </h2>
          <p className="mt-2 text-body leading-6 text-muted-foreground-strong">
            {copy.body}
          </p>
          {projectPath && (
            <p
              className="mt-4 truncate rounded-sm bg-background/40 px-3 py-2 font-mono text-micro text-muted-foreground"
              title={projectPath}
            >
              {projectPath}
            </p>
          )}
          {inspection?.status === "uninitialized" && (
            <InitialCapturePreviewCard
              preview={inspection.preview}
              storageLabel="Ley’s private local app storage"
            />
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
                  onClick={primaryAction?.onClick}
                >
                  {busy ? (
                    <RefreshCw
                      size={14}
                      className="animate-spin motion-reduce:animate-none"
                    />
                  ) : (
                    <ArrowRight size={14} />
                  )}
                  {busy ? "Preparing memory…" : primaryAction?.label}
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

function onboardingPrimaryAction(
  inspection: AgentProjectInspection,
  onInitialize: () => void,
  onConnect: () => void,
  onCapture: () => void,
) {
  switch (inspection.status) {
    case "uninitialized":
      return { onClick: onInitialize, label: "Approve, initialize & capture" };
    case "unbound":
      return { onClick: onConnect, label: "Reconnect & migrate" };
    case "vault-unavailable":
      return { onClick: onConnect, label: "Reconnect & migrate" };
    default:
      return { onClick: onCapture, label: "Capture project" };
  }
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
        "group w-full rounded-md border border-border bg-surface-1 p-4 text-left shadow-panel hover:border-border-strong hover:bg-surface-2/55 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary",
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
        "inline-flex items-center gap-1.5 rounded-sm border px-2 py-0.5 text-micro font-medium",
        tone === "success" && "border-success/20 bg-success/10 text-success",
        tone === "warning" && "border-warning/20 bg-warning/10 text-warning",
        tone === "neutral" &&
          "border-border bg-surface-2 text-muted-foreground-strong",
      )}
    >
      <Icon size={11} />
      {label}
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
          ? "bg-success ring-success/10"
          : rejected
            ? "bg-destructive ring-destructive/10"
            : "bg-warning ring-warning/10",
      )}
    />
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
    <div className="rounded-sm border border-dashed border-border bg-surface-1/45 px-6 py-14 text-center">
      <Icon size={22} className="mx-auto text-subtle-foreground" />
      <h3 className="mt-3 text-meta font-semibold text-foreground">{title}</h3>
      <p className="mx-auto mt-1 max-w-md text-meta leading-relaxed text-muted-foreground">
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

function formatOnboardingBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${unit}`;
}

function agentMemoryStorageLabel(storage: AgentMemoryStorage): string {
  return storage.kind === "native"
    ? "Ley private local storage"
    : storage.vaultName;
}

function isoTime(unixMs: number): string {
  return new Date(unixMs).toISOString();
}

function compactId(value: string): string {
  return value.length <= 16 ? value : `${value.slice(0, 12)}…`;
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

function actionLabel(action: LearningAction): string {
  return action === "mark-stale" ? "Mark stale" : humanize(action);
}

function reviewPlaceholder(action: LearningAction): string {
  if (action === "confirm") return "Explain why this is useful or reliable…";
  if (action === "contest") return "Describe what is uncertain or conflicting…";
  if (action === "reject") return "Explain why agents should not reuse this…";
  if (action === "supersede")
    return "Explain why the replacement should be used instead…";
  return "Describe what changed or became outdated…";
}
