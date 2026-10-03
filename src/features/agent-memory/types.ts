export type CaptureMode = "minimal" | "structured" | "full-evidence";
export type LearningAction =
  | "confirm"
  | "contest"
  | "reject"
  | "mark-stale"
  | "supersede";

export type ApprovedSourceState = "current" | "changed" | "missing";
export type ApprovedSourceKind = "project-file" | "imported-snapshot";
export type ApprovedSourceLegacyIssueReason = "changed" | "missing" | "invalid";

export interface ApprovedSourceApproval {
  projectId: string;
  sourceId: string;
  sourceKind: ApprovedSourceKind;
  displayName: string;
  projectRelativePath?: string;
  contentHash: string;
  approvedAtUnixMs: number;
}

export interface ApprovedSourceAuthority {
  approval: ApprovedSourceApproval;
  state: ApprovedSourceState;
  currentContentHash?: string;
}

export interface ApprovedSourceLegacyIssue {
  projectId: string;
  sourceId: string;
  displayName: string;
  approvedContentHash: string;
  approvedAtUnixMs: number;
  reason: ApprovedSourceLegacyIssueReason;
}

export interface ApprovedSourceAuthorityList {
  projectId: string;
  sources: ApprovedSourceAuthority[];
  current: number;
  changed: number;
  missing: number;
  legacyIssues: ApprovedSourceLegacyIssue[];
  privacyNotice: string;
}

export type AgentMemoryStorage =
  | {
      kind: "native";
      projectId: string;
    }
  | {
      kind: "legacy-vault";
      projectId: string;
      vaultName: string;
      source: "persisted" | "override";
    };

export interface MemoryOverview {
  projectId: string;
  projectName: string;
  captureMode: CaptureMode;
  artifactSnapshotId: string;
  graphSnapshotId?: string;
  artifactGeneratedAtUnixMs: number;
  graphGeneratedAtUnixMs?: number;
  files: number;
  retainedSourceFiles: number;
  skippedFiles: number;
  graphNodes?: number;
  graphEdges?: number;
  graphDiagnostics?: number;
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
  sourceKind: "manual-cli" | "host-hook" | "mcp" | "import";
  sourceHost?: string;
  startedAtUnixMs: number;
  updatedAtUnixMs: number;
  eventCount: number;
  checkpoints: number;
  prompts?: number;
  responses?: number;
}

export type RevisionCompatibility =
  "current-lineage" | "ancestor" | "merged" | "divergent" | "unknown";

export interface RevisionApplicability {
  compatibility: RevisionCompatibility;
  capturedHead?: string;
  capturedBranch?: string;
}

export interface ProjectRevisionFreshness {
  liveGitChecked: boolean;
  capturedHead?: string;
  capturedBranch?: string;
  currentHead?: string;
  currentBranch?: string;
  trackedWorktreeChanges?: number;
  captureCompatibility: RevisionCompatibility;
  capturedHeadMatchesCurrent: boolean;
  capturedBranchMatchesCurrent?: boolean;
}

export interface SessionContextUtilityIncludedRecord {
  source: "specification" | "active-project-memory" | "mounted-reference";
  entityId: string;
  kind?: string;
  sessionId?: string;
  learningId?: string;
  learningKind?: string;
  learningEventCount?: number;
  specificationId?: string;
  mountId?: string;
  sourceProjectId?: string;
}

export interface SessionContextUtilityOutcome {
  eventId: string;
  recordedAtUnixMs: number;
  kind: "checkpoint" | "session-finish";
  completedTasks: number;
  blockedTasks: number;
  cancelledTasks: number;
  resolvedProblems: number;
  helpedAttempts: number;
  noEffectAttempts: number;
  worsenedAttempts: number;
  unknownAttempts: number;
  passedVerifications: number;
  failedVerifications: number;
  skippedVerifications: number;
  unknownVerifications: number;
  sessionStatus?: "active" | "completed" | "paused" | "abandoned";
  unresolvedCount: number;
}

export interface SessionContextUnobservedUtilityBinding {
  bindingId: string;
  eventId: string;
  recordedAtUnixMs: number;
  contextPackId: string;
  artifactSnapshotId: string;
  graphSnapshotId: string;
  egressTarget?: "cloud" | "local";
  maxResults: number;
  maxTokens: number;
  estimatedTokens: number;
  includedRecordCount: number;
  omittedIncludedRecords: number;
  contextPackRevalidated: boolean;
  contextUsageProven: boolean;
  terminalFinishEventId?: string;
}

export interface SessionContextUtilityObservation {
  id: string;
  eventId: string;
  recordedAtUnixMs: number;
  expectedEventCount: number;
  bindingId: string;
  contextPackId: string;
  taskExcerpt: string;
  artifactSnapshotId: string;
  graphSnapshotId: string;
  egressTarget?: "cloud" | "local";
  maxResults: number;
  maxTokens: number;
  estimatedTokens: number;
  includedRecords: SessionContextUtilityIncludedRecord[];
  omittedIncludedRecords: number;
  downstreamEventIds: string[];
  downstreamOutcomes: SessionContextUtilityOutcome[];
  claimedAppliedLearningIds: string[];
  contextPackRevalidated: boolean;
  contextUsageProven: boolean;
  causalUtilityProven: boolean;
  trustChangesApplied: boolean;
  rankingChangesApplied: boolean;
}

export interface SessionContext {
  projectionSchemaVersion: number;
  schemaVersion: number;
  projectId: string;
  sessionId: string;
  originalName: string;
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
  promptCount: number;
  responseCount: number;
  retainedTurnCount: number;
  omittedTurnCount: number;
  renameCount: number;
  renames: Array<{
    recordedAtUnixMs: number;
    name: string;
    note: string;
  }>;
  omittedRenames: number;
  contextUtilityBindingCount: number;
  contextUtilityObservationCount: number;
  observedContextUtilityBindingCount: number;
  unobservedContextUtilityBindingCount: number;
  unobservedContextUtilityBindings: SessionContextUnobservedUtilityBinding[];
  omittedUnobservedContextUtilityBindings: number;
  contextUtilityObservations: SessionContextUtilityObservation[];
  omittedContextUtilityObservations: number;
  checkpoints: Array<{
    checkpointId: string;
    recordedAtUnixMs: number;
    summary: string;
    projectRevision?: {
      graphSnapshotId: string;
      artifactSnapshotId: string;
      capturedAtUnixMs: number;
      head?: string;
      branch?: string;
      trackedChanges: number;
    };
    revisionApplicability?: RevisionApplicability;
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
      mediaType?: ArtifactMediaType;
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
      command?: string;
      evidenceArtifacts: ArtifactEvidenceReference[];
      evidenceArtifactsOmitted: number;
    }>;
    unresolved: string[];
  }>;
  finish?: {
    eventId: string;
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
  revisionFreshness: ProjectRevisionFreshness;
  liveSourceChecked: boolean;
  sourceBoundary: string;
  instructionWarning: string;
}
export interface SessionTurnsContext {
  projectionSchemaVersion: number;
  schemaVersion: number;
  projectId: string;
  sessionId: string;
  promptCount: number;
  responseCount: number;
  retainedTurnCount: number;
  omittedMinimalCount: number;
  omittedCapacityCount: number;
  turns: Array<{
    recordId: string;
    eventId: string;
    recordedAtUnixMs: number;
    kind: "user-prompt" | "assistant-response";
    origin: "host-hook" | "manual-cli";
    host?: string;
    turnReference?: string;
    captureMode: CaptureMode;
    retention: "captured" | "omitted-minimal" | "omitted-capacity";
    text?: string;
    truncatedAtCapture: boolean;
    truncatedForContext: boolean;
    sourceBoundary: "untrusted-user-prompt" | "untrusted-agent-output";
  }>;
  omittedTurns: number;
  textCharacters: number;
  estimatedTextTokens: number;
  truncated: boolean;
  liveSourceChecked: boolean;
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
  eventCount: number;
  supersededBy?: string;
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
  storage: AgentMemoryStorage;
  overview: MemoryOverview;
  resume: ProjectResume;
  sessions: SessionSummary[];
  reviewInbox: LearningList;
  allLearnings: LearningList;
}

export interface AgentContinuityExport {
  projectId: string;
  destination: string;
  eventCount: number;
  artifactSnapshots: number;
  evidenceBlobs: number;
}

export interface SessionMemoryErasure {
  projectId: string;
  sessionId: string;
  sessionName: string;
  erasedLearningIds: string[];
  ordinaryNotesPreserved: boolean;
  canvasDocumentsPreserved: boolean;
  projectEvidencePreserved: boolean;
}

export interface AgentSessionErasure {
  dashboard: AgentMemoryDashboard;
  erasure: SessionMemoryErasure;
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
  ignoreFilePresent: boolean;
  captureFingerprint: string;
  eligibleFiles: number;
  eligibleBytes: number;
  skippedOversized: number;
  skippedTotalLimit: number;
  skippedSymlinks: number;
  privacyNotice: string;
}

export type AgentEgressPolicy =
  | "agent-ok"
  | "confirm-per-use"
  | "local-model-only"
  | "never-send";
export type AgentEgressTarget = "cloud" | "local";

export interface AgentEgressScopePolicy {
  scopeKind: "project" | "specification" | "context-mount" | "external-connector";
  scopeId: string;
  policy: AgentEgressPolicy;
}

export interface ProjectAgentEgressPolicy {
  projectId: string;
  projectPolicy: AgentEgressPolicy;
  specificationOverrides: AgentEgressScopePolicy[];
  mountOverrides: AgentEgressScopePolicy[];
  connectorOverrides: AgentEgressScopePolicy[];
  privacyNotice: string;
}

export interface AgentBriefPreviewItem {
  kind: string;
  entityId: string;
  title: string;
  excerpt: string;
  sessionId?: string;
  learningId?: string;
  citation?: GraphCitation;
  authority: string;
  trustedForReuse: boolean;
  revisionApplicability?: string;
  estimatedTokens: number;
}

export interface AgentBriefPreviewSpecification {
  specificationId: string;
  relativePath: string;
  contentHash: string;
  source: string;
  relevanceScore: number;
  exactMatch: boolean;
  estimatedTokens: number;
}

export interface AgentBriefPreview {
  contextPackId: string;
  createdAtUnixMs: number;
  projectId: string;
  projectName: string;
  task: string;
  evidenceState: string;
  premiseAdjudication: {
    state: string;
    warnings: Array<{ kind: string; message: string }>;
    omittedWarnings: number;
  };
  egressTarget?: AgentEgressTarget;
  egressExclusions?: Array<{
    scopeKind: string;
    scopeId: string;
    policy: AgentEgressPolicy;
    blockReason: string;
  }>;
  egressCoverage?: {
    target: AgentEgressTarget;
    blockedSpecifications: number;
    blockedMounts: number;
    blockedExternalConnectors: number;
    blockedHistoricalSources: number;
    blockedPolicyBundleSources: number;
    historicalMemoryWithheld: boolean;
    withheldDerivedResults: number;
  };
  maxTokens: number;
  estimatedTokens: number;
  specifications: AgentBriefPreviewSpecification[];
  items: AgentBriefPreviewItem[];
  gaps: Array<{ kind: string; message: string }>;
  coverage: {
    returnedItems: number;
    returnedConflicts: number;
    returnedExclusions: number;
    returnedGaps: number;
    omittedGaps: number;
    omittedConflicts: number;
    omittedExclusions: number;
    searchTruncated: boolean;
    sourceTruncated: boolean;
  };
  liveSourceChecked: boolean;
  sourceBoundary: string;
  instructionWarning: string;
  privacyNotice: string;
  [key: string]: unknown;
}

export interface AgentInitialCaptureSkippedPath {
  path: string;
  reason: "oversized" | "total-limit" | "symlink";
}

export interface AgentInitialCapturePreview {
  mode: CaptureMode;
  approvedRoots: string[];
  respectGitignore: boolean;
  maxFileBytes: number;
  maxTotalBytes: number;
  captureFingerprint: string;
  planFingerprint: string;
  approvalFingerprint: string;
  eligibleFiles: number;
  eligibleBytes: number;
  includedPaths: string[];
  omittedIncludedPaths: number;
  skippedOversized: number;
  skippedTotalLimit: number;
  skippedSymlinks: number;
  skippedPaths: AgentInitialCaptureSkippedPath[];
  omittedSkippedPaths: number;
  exclusionNotice: string;
  privacyNotice: string;
}

export type AgentProjectSearchResultKind =
  | "session"
  | "revision"
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

export type ProjectMemoryTrustSignal =
  | "direct-evidence"
  | "trusted-current"
  | "unverified"
  | "contested"
  | "superseded"
  | "rejected"
  | "stale";

export interface ProjectMemorySearchResult {
  kind: AgentProjectSearchResultKind;
  entityId: string;
  title: string;
  excerpt: string;
  updatedAtUnixMs: number;
  sessionId?: string;
  learningId?: string;
  citation?: GraphCitation;
  learningState?: string;
  learningTrustState?: string;
  learningFreshness?: string;
  trustSignal?: ProjectMemoryTrustSignal;
  revisionApplicability?: RevisionApplicability;
  trustedForReuse: boolean;
  truncated: boolean;
  ranking: {
    lexicalRank?: number;
    semanticRank?: number;
    artifactHybridRank?: number;
    reciprocalRankScore: number;
    temporalContribution: number;
    trustContribution: number;
    finalScore: number;
  };
}

export interface ProjectMemorySearch {
  projectId: string;
  projectName: string;
  artifactSnapshotId: string;
  graphSnapshotId: string;
  capturedAtUnixMs: number;
  query: string;
  revisionFilter?: RevisionCompatibility;
  maxTokens: number;
  estimatedTokens: number;
  results: ProjectMemorySearchResult[];
  conflicts: Array<{
    kind: "learning-state" | "content-disagreement";
    entityIds: string[];
    learningIds: string[];
    reason: string;
  }>;
  coverage: {
    candidateLimit: number;
    collectedCandidates: number;
    omittedCandidates: number;
    revisionFilteredCandidates: number;
    omittedResults: number;
    omittedConflicts: number;
    truncatedResultContent: number;
    sourceTruncated: boolean;
  };
  truncated: boolean;
  retrieval: {
    mode: "lexical" | "semantic" | "hybrid";
    boundedRerankMode: "lexical" | "semantic" | "hybrid";
    artifactContextMode: "lexical" | "semantic" | "hybrid";
    boundedRerankFallbackReason?: string;
    artifactContextFallbackReason?: string;
  };
  revisionFreshness: ProjectRevisionFreshness;
  freshness: string;
  liveSourceChecked: boolean;
  sourceBoundary: string;
  instructionWarning: string;
  privacyNotice: string;
}

export type ArtifactKind =
  "source" | "documentation" | "manifest" | "configuration" | "text" | "image";

export type ArtifactMediaType = "png" | "jpeg" | "webp";

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
    mediaType?: ArtifactMediaType;
    contentHash: string;
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
    reason:
      | "binary"
      | "invalid-media"
      | "media-requires-full-evidence"
      | "non-utf8"
      | "oversized"
      | "total-limit"
      | "symlink";
    bytes: number;
  }>;
  totalMatchingSkipped: number;
  omittedSkipped: number;
  liveSourceChecked: boolean;
  instructionWarning: string;
}

export interface GraphCitation {
  artifactPath: string;
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
  contentHash: string;
  artifactSnapshotId: string;
  mediaType?: ArtifactMediaType;
}

export interface ProjectEvidenceExcerpt {
  projectId: string;
  artifactSnapshotId: string;
  artifactPath: string;
  text: string;
  citation: GraphCitation;
  truncated: boolean;
  freshness: string;
  liveSourceChecked: boolean;
  sourceBoundary: string;
  warning: string;
}

export interface AgentMediaEvidence {
  artifactPath: string;
  artifactSnapshotId: string;
  contentHash: string;
  mediaType: ArtifactMediaType;
  mimeType: string;
  sourceBytes: number;
  dataUrl: string;
  evidenceRole: "original-media";
  sourceBoundary: string;
  liveSourceChecked: false;
  derivedDescriptionIncluded: false;
}

export type ProjectProblemScope = "all" | "open" | "resolved";

export interface ProjectActivityCitation {
  artifactPath: string;
  artifactSnapshotId: string;
  contentHash: string;
  mediaType?: ArtifactMediaType;
  startLine: number;
  endLine: number;
}

export interface ArtifactEvidenceReference {
  artifactPath: string;
  artifactSnapshotId: string;
  contentHash: string;
  mediaType?: ArtifactMediaType;
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
  | {
      status: "uninitialized";
      suggestedName: string;
      preview: AgentInitialCapturePreview;
    }
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
      storage: AgentMemoryStorage;
    }
  | { status: "ready"; dashboard: AgentMemoryDashboard };

export interface LearningContext {
  projectionSchemaVersion: number;
  schemaVersion: number;
  projectId: string;
  learningId: string;
  kind: string;
  title: string;
  guidance: string;
  state: string;
  trustState: string;
  trustedForReuse: boolean;
  provenance: string;
  originLineage: {
    mechanicallyResolved: boolean;
    causalCompletenessProven: boolean;
    omittedSources: number;
    automaticAuthorityCeiling: string;
    sources: Array<
      | {
          kind: "session-record";
          sessionId: string;
          recordId: string;
          recordType: string;
        }
      | {
          kind: "captured-artifact";
          artifactSnapshotId: string;
          artifactPath: string;
          contentHash: string;
        }
      | { kind: "turn-evidence"; sessionId: string; recordId: string }
      | { kind: "tool-evidence"; sessionId: string; recordId: string }
      | {
          kind: "recovery-candidate";
          sessionId: string;
          candidateFingerprint: string;
        }
    >;
  };
  originSourceCount: number;
  omittedOriginSources: number;
  confidencePercent: number;
  freshness: string;
  freshnessBasis: string;
  liveSourceChecked: boolean;
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
      artifactSnapshotId: string;
      contentHash: string;
      mediaType?: ArtifactMediaType;
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
  omittedArtifacts: number;
  omittedHistory: number;
  applicationObservationCount: number;
  applicationObservations: Array<{
    sessionId: string;
    observationId: string;
    recordedAtUnixMs: number;
    bindingId: string;
    contextPackId: string;
    learningEventCount: number;
    learningVersionMatchesCurrent: boolean;
    taskExcerpt: string;
    downstreamEventIds: string[];
    downstreamOutcomes: SessionContextUtilityOutcome[];
    passedVerifications: number;
    failedVerifications: number;
    skippedVerifications: number;
    unknownVerifications: number;
    procedureFollowedProven: boolean;
    conditionApplicabilityProven: boolean;
    contextUsageProven: boolean;
    causalUtilityProven: boolean;
    trustChangesApplied: boolean;
    rankingChangesApplied: boolean;
  }>;
  omittedApplicationObservations: number;
  applicationClaimNotice: string;
  supersededBy?: string;
  textCharacters: number;
  estimatedTextTokens: number;
  claimTruncated: boolean;
  truncated: boolean;
  sourceBoundary: string;
  instructionWarning: string;
}
