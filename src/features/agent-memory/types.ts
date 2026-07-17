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

export interface SessionContext {
  projectId: string;
  sessionId: string;
  name: string;
  goal: string;
  status: "active" | "completed" | "paused" | "abandoned";
  source: {
    kind: string;
    host?: string;
    agent?: string;
  };
  artifactSnapshotIdAtStart: string;
  startedAtUnixMs: number;
  updatedAtUnixMs: number;
  eventCount: number;
  checkpointCount: number;
  checkpoints: Array<{
    checkpointId: string;
    recordedAtUnixMs: number;
    summary: string;
    decisions: Array<{ id: string; title: string; decision: string }>;
    tasks: Array<{ id: string; title: string; status: string }>;
    problems: Array<{
      id: string;
      title: string;
      symptom: string;
      attempts: Array<{
        id: string;
        action: string;
        outcome: string;
        evidence: string;
      }>;
      latestAttemptOutcome?: string;
      resolution?: string;
      resolutionDetail?: {
        id: string;
        rootCause: string;
        change: string;
        verification: string;
      };
    }>;
    touchedArtifacts: Array<{
      artifactPath: string;
      artifactSnapshotId: string;
      contentHash: string;
      startLine: number;
      endLine: number;
    }>;
    commands: Array<{
      id: string;
      command: string;
      exitCode?: number;
      summary: string;
    }>;
    verification: Array<{
      id: string;
      kind: string;
      status: string;
      summary: string;
    }>;
    unresolved: string[];
  }>;
  finish?: {
    recordedAtUnixMs: number;
    status: string;
    summary: string;
    finalResponse: string;
    handoff: string;
    unresolved: string[];
  };
  omittedCheckpoints: number;
  textCharacters: number;
  estimatedTextTokens: number;
  truncated: boolean;
  instructionWarning: string;
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

export type AgentProjectCatalogState =
  | "ready"
  | "unbound"
  | "needs-capture"
  | "project-unavailable"
  | "vault-unavailable"
  | "identity-changed"
  | "memory-error";

export interface AgentProjectCatalogItem {
  projectId: string;
  projectPath: string;
  projectName: string;
  captureMode?: CaptureMode;
  state: AgentProjectCatalogState;
  lastOpenedAtUnixMs: number;
  vaultName?: string;
  files?: number;
  graphNodes?: number;
  sessions?: number;
  activeSessions?: number;
  reviewItems?: number;
  freshness?: string;
  statusDetail: string;
}

export interface AgentProjectCatalog {
  projects: AgentProjectCatalogItem[];
  totalProjects: number;
  omittedProjects: number;
  readyProjects: number;
  attentionProjects: number;
  privacyNotice: string;
}

export interface AgentCaptureSettings {
  projectId: string;
  projectName: string;
  mode: CaptureMode;
  approvedRoots: string[];
  respectGitignore: boolean;
  maxFileBytes: number;
  maxTotalBytes: number;
  storeRawTranscripts: boolean;
  ignoreFilePresent: boolean;
  captureFingerprint: string;
  eligibleFiles: number;
  eligibleBytes: number;
  skippedOversized: number;
  skippedTotalLimit: number;
  skippedSymlinks: number;
  privacyNotice: string;
}

export type AgentProjectSearchResultKind =
  | "session"
  | "decision"
  | "problem"
  | "learning"
  | "artifact"
  | "symbol"
  | "dependency";

export interface AgentProjectSearchResult {
  projectId: string;
  projectName: string;
  projectPath: string;
  kind: AgentProjectSearchResultKind;
  entityId: string;
  title: string;
  excerpt: string;
  updatedAtUnixMs: number;
  sessionId?: string;
  learningId?: string;
  citation?: GraphCitation;
  trustState?: string;
  freshness?: string;
}

export interface AgentProjectSearch {
  query: string;
  results: AgentProjectSearchResult[];
  searchedProjects: number;
  skippedProjects: number;
  totalObservedProjects: number;
  omittedProjects: number;
  truncated: boolean;
  liveSourceChecked: boolean;
  sourceBoundary: string;
  instructionWarning: string;
  privacyNotice: string;
}

export type ArtifactKind =
  "source" | "documentation" | "manifest" | "configuration" | "text";

export interface ProjectArtifactInventory {
  projectId: string;
  projectName: string;
  artifactSnapshotId: string;
  generatedAtUnixMs: number;
  captureMode: CaptureMode;
  query: string;
  artifacts: Array<{
    path: string;
    kind: ArtifactKind;
    language?: string;
    sourceBytes: number;
    storedBytes: number;
    lineCount: number;
    retainedSource: boolean;
    redactions: Array<{ kind: string; lines: number[] }>;
  }>;
  totalMatchingArtifacts: number;
  omittedArtifacts: number;
  skipped: Array<{
    path: string;
    reason: "binary" | "non-utf8" | "oversized" | "total-limit" | "symlink";
    bytes: number;
  }>;
  totalMatchingSkipped: number;
  omittedSkipped: number;
  liveSourceChecked: boolean;
  instructionWarning: string;
}

export type ProjectGraphNodeKind =
  | "project"
  | "file"
  | "symbol"
  | "dependency"
  | "external-symbol"
  | "external-module";

export interface GraphCitation {
  artifactPath: string;
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
  contentHash: string;
  artifactSnapshotId: string;
}

export interface ProjectGraphViewNode {
  id: string;
  kind: ProjectGraphNodeKind;
  name: string;
  path?: string;
  language?: string;
  symbolKind?: string;
  packageManager?: string;
  citation?: GraphCitation;
  provenance: "deterministic" | "user-authored" | "agent-authored" | "inferred";
  confidence: number;
  degree: number;
}

export interface ProjectGraphView {
  projectId: string;
  projectName: string;
  artifactSnapshotId: string;
  graphSnapshotId: string;
  generatedAtUnixMs: number;
  query: string;
  selection: string;
  nodes: ProjectGraphViewNode[];
  edges: Array<{
    id: string;
    kind: string;
    source: string;
    target: string;
    label?: string;
    citation?: GraphCitation;
    provenance: string;
    confidence: number;
  }>;
  totalNodes: number;
  totalEdges: number;
  matchingNodes: number;
  omittedNodes: number;
  omittedEdges: number;
  diagnostics: Array<{
    artifactPath: string;
    kind: string;
    message: string;
  }>;
  omittedDiagnostics: number;
  git?: {
    head?: string;
    branch?: string;
    upstream?: string;
    ahead: number;
    behind: number;
    changes: Array<{
      status: string;
      path: string;
      originalPath?: string;
    }>;
  };
  liveSourceChecked: boolean;
  instructionWarning: string;
}

export type ProjectProblemScope = "all" | "open" | "resolved";

export interface ProjectActivityCitation {
  artifactPath: string;
  artifactSnapshotId: string;
  contentHash: string;
  startLine: number;
  endLine: number;
}

export interface ProjectDecision {
  recordId: string;
  checkpointId: string;
  sessionId: string;
  sessionName: string;
  sessionStatus: SessionSummary["status"];
  recordedAtUnixMs: number;
  title: string;
  decision: string;
  rationale: string;
  alternatives: string[];
  omittedAlternatives: number;
  artifactCitations: ProjectActivityCitation[];
  omittedArtifactCitations: number;
  detailTruncated: boolean;
}

export interface ProjectProblemAttempt {
  id: string;
  action: string;
  outcome: "worked" | "failed" | "partial" | "no-effect" | "not-verified";
  evidence: string;
}

export interface ProjectProblem {
  recordId: string;
  checkpointId: string;
  sessionId: string;
  sessionName: string;
  sessionStatus: SessionSummary["status"];
  recordedAtUnixMs: number;
  title: string;
  symptom: string;
  expected: string;
  attempts: ProjectProblemAttempt[];
  totalAttempts: number;
  omittedAttempts: number;
  latestAttemptOutcome?: ProjectProblemAttempt["outcome"];
  resolution?: {
    id: string;
    rootCause: string;
    change: string;
    verification: string;
  };
  artifactCitations: ProjectActivityCitation[];
  omittedArtifactCitations: number;
  detailTruncated: boolean;
}

export interface ProjectActivityView {
  projectId: string;
  query: string;
  problemScope: ProjectProblemScope;
  decisions: ProjectDecision[];
  totalMatchingDecisions: number;
  omittedDecisions: number;
  problems: ProjectProblem[];
  totalMatchingProblems: number;
  omittedProblems: number;
  totalSessions: number;
  liveSourceChecked: boolean;
  sourceBoundary: string;
  instructionWarning: string;
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
      status: "vault-unavailable";
      projectId: string;
      projectName: string;
      captureMode: CaptureMode;
      previousVaultName: string;
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
  createdAtUnixMs: number;
  updatedAtUnixMs: number;
  validFromUnixMs: number;
  validUntilUnixMs?: number;
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
  historyCount: number;
  eventCount: number;
  omittedEvidence: number;
  omittedHistory: number;
  claimTruncated: boolean;
  truncated: boolean;
  instructionWarning: string;
}

export interface PromotedLearningNoteDraft {
  learningId: string;
  title: string;
  folder: string;
  content: string;
  frontmatter: Record<string, unknown>;
}
