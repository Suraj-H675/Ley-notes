export type CaptureMode = "minimal" | "structured" | "full-evidence";
export type LearningAction = "confirm" | "contest" | "reject" | "mark-stale";

export interface AgentMemoryBinding {
  projectId: string;
  vaultName: string;
  source: "persisted" | "override";
}

export interface MemoryOverview {
  projectId: string;
  projectName: string;
  captureMode: CaptureMode;
  artifactSnapshotId: string;
  graphSnapshotId: string;
  artifactGeneratedAtUnixMs: number;
  graphGeneratedAtUnixMs: number;
  files: number;
  retainedSourceFiles: number;
  skippedFiles: number;
  graphNodes: number;
  graphEdges: number;
  graphDiagnostics: number;
  freshness: string;
  liveSourceChecked: boolean;
  privacyNotice: string;
}

export interface ResumeCheckpoint {
  checkpointId: string;
  recordedAtUnixMs: number;
  summary: string;
  decisions: Array<{ recordId: string; title: string; decision: string }>;
  activeTasks: Array<{
    recordId: string;
    title: string;
    status: string;
    details: string;
  }>;
  unresolvedProblems: Array<{
    recordId: string;
    title: string;
    symptom: string;
  }>;
  unresolved: string[];
}

export interface ResumeSession {
  sessionId: string;
  name: string;
  goal: string;
  status: "active" | "completed" | "paused" | "abandoned";
  startedAtUnixMs: number;
  updatedAtUnixMs: number;
  eventCount: number;
  checkpointCount: number;
  latestCheckpoint?: ResumeCheckpoint;
  result?: {
    status: string;
    recordedAtUnixMs: number;
    summary: string;
    handoff: string;
    unresolved: string[];
  };
}

export interface SessionSummary {
  projectId: string;
  sessionId: string;
  name: string;
  goal: string;
  status: "active" | "completed" | "paused" | "abandoned";
  startedAtUnixMs: number;
  updatedAtUnixMs: number;
  eventCount: number;
  checkpoints: number;
}

export interface ResumeLearning {
  learningId: string;
  kind: string;
  title: string;
  guidance: string;
  state: string;
  trustState: string;
  trustedForReuse: boolean;
  provenance: string;
  confidencePercent: number;
  freshness: string;
  corroboratingSessions: number;
  updatedAtUnixMs: number;
}

export interface ProjectResume {
  projectId: string;
  projectName: string;
  captureMode: CaptureMode;
  capturedAtUnixMs: number;
  freshness: string;
  liveSourceChecked: boolean;
  sessions: ResumeSession[];
  totalSessions: number;
  omittedSessions: number;
  learnings: ResumeLearning[];
  totalCurrentTrustedLearnings: number;
  omittedLearnings: number;
  instructionWarning: string;
}

export interface LearningSummary {
  projectId: string;
  learningId: string;
  kind: string;
  title: string;
  guidanceExcerpt: string;
  state: string;
  trustState: string;
  provenance: string;
  confidencePercent: number;
  freshness: string;
  corroboratingSessions: number;
  updatedAtUnixMs: number;
}

export interface LearningList {
  projectId: string;
  scope: string;
  learnings: LearningSummary[];
  totalMatching: number;
  omittedLearnings: number;
  instructionWarning: string;
}

export interface AgentMemoryDashboard {
  binding: AgentMemoryBinding;
  overview: MemoryOverview;
  resume: ProjectResume;
  sessions: SessionSummary[];
  reviewInbox: LearningList;
  allLearnings: LearningList;
}

export type AgentProjectInspection =
  | { status: "uninitialized"; suggestedName: string }
  | {
      status: "unbound";
      projectId: string;
      projectName: string;
      captureMode: CaptureMode;
    }
  | {
      status: "needs-capture";
      projectId: string;
      projectName: string;
      captureMode: CaptureMode;
      binding: AgentMemoryBinding;
    }
  | { status: "ready"; dashboard: AgentMemoryDashboard };

export interface LearningContext {
  projectId: string;
  learningId: string;
  kind: string;
  title: string;
  guidance: string;
  state: string;
  trustState: string;
  trustedForReuse: boolean;
  provenance: string;
  confidencePercent: number;
  freshness: string;
  corroboratingSessions: number;
  updatedAtUnixMs: number;
  evidenceCount: number;
  evidence: Array<{
    sessionId: string;
    recordId: string;
    recordType: string;
    sessionStatus: string;
    sessionUpdatedAtUnixMs: number;
    note: string;
    artifacts: Array<{
      artifactPath: string;
      startLine: number;
      endLine: number;
    }>;
  }>;
  history: Array<{
    eventId: string;
    recordedAtUnixMs: number;
    actor: string;
    action: string;
    note: string;
  }>;
  instructionWarning: string;
}
