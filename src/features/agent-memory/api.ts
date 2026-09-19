import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AgentCaptureSettings,
  AgentMediaEvidence,
  AgentMemoryDashboard,
  AgentSessionErasure,
  AgentProjectSearch,
  AgentProjectCatalog,
  AgentProjectInspection,
  LearningAction,
  LearningContext,
  ProjectActivityView,
  ProjectArtifactInventory,
  ProjectGraphEvidenceExcerpt,
  ProjectGraphFilters,
  ProjectGraphHistory,
  ProjectGraphView,
  GraphCitation,
  ProjectProblemScope,
  ProjectMemorySearch,
  RevisionCompatibility,
  SemanticModelInstallation,
  SemanticModelSetup,
  SessionContext,
  SessionTurnsContext,
  SpecificationAuthorityList,
} from "./types";

export async function chooseAgentProject(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Choose a project for Agent Memory",
  });
  return selected ?? null;
}

export function listAgentProjects(
  legacyProjectPath?: string,
): Promise<AgentProjectCatalog> {
  return invoke("list_agent_projects", { legacyProjectPath });
}

export function forgetAgentProject(
  projectId: string,
): Promise<AgentProjectCatalog> {
  return invoke("forget_agent_project", { projectId });
}

export function searchAgentProjects(
  query: string,
): Promise<AgentProjectSearch> {
  return invoke("search_agent_projects", { query });
}

export function searchAgentProjectMemory(
  projectPath: string,
  query: string,
  revisionFilter?: RevisionCompatibility,
): Promise<ProjectMemorySearch> {
  return invoke("search_agent_project_memory", {
    projectPath,
    query,
    revisionFilter,
  });
}

export function readSemanticModelSetup(): Promise<SemanticModelSetup> {
  return invoke("semantic_model_status");
}

export function installSemanticModel(): Promise<SemanticModelInstallation> {
  return invoke("install_semantic_model");
}

export function readAgentCaptureSettings(
  projectPath: string,
): Promise<AgentCaptureSettings> {
  return invoke("read_agent_capture_settings", { projectPath });
}

export function updateAgentCaptureMode(
  projectPath: string,
  expectedMode: AgentCaptureSettings["mode"],
  mode: AgentCaptureSettings["mode"],
  fullEvidenceConsent: boolean,
): Promise<AgentMemoryDashboard> {
  return invoke("update_agent_capture_mode", {
    projectPath,
    expectedMode,
    mode,
    fullEvidenceConsent,
  });
}

export function eraseAgentProjectMemory(
  projectPath: string,
): Promise<AgentProjectInspection> {
  return invoke("erase_agent_project_memory", { projectPath });
}

export function inspectAgentProject(
  projectPath: string,
): Promise<AgentProjectInspection> {
  return invoke("inspect_agent_project", { projectPath });
}

export function verifyAgentProjectNoteVault(
  projectPath: string,
  openVaultPath: string,
): Promise<void> {
  return invoke("verify_agent_project_note_vault", {
    projectPath,
    openVaultPath,
  });
}

export function readAgentProjectSpecifications(
  projectPath: string,
): Promise<SpecificationAuthorityList> {
  return invoke("read_agent_project_specifications", { projectPath });
}

export function approveAgentProjectSpecification(
  projectPath: string,
  openVaultPath: string,
  specificationId: string,
  relativePath: string,
): Promise<SpecificationAuthorityList> {
  return invoke("approve_agent_project_specification", {
    projectPath,
    openVaultPath,
    specificationId,
    relativePath,
  });
}

export function revokeAgentProjectSpecification(
  projectPath: string,
  openVaultPath: string,
  specificationId: string,
): Promise<SpecificationAuthorityList> {
  return invoke("revoke_agent_project_specification", {
    projectPath,
    openVaultPath,
    specificationId,
  });
}

export function initializeAgentProject(
  projectPath: string,
  vaultPath: string,
): Promise<AgentMemoryDashboard> {
  return invoke("initialize_agent_project", { projectPath, vaultPath });
}

export function connectAgentProject(
  projectPath: string,
  vaultPath: string,
): Promise<AgentMemoryDashboard> {
  return invoke("connect_agent_project", { projectPath, vaultPath });
}

export function refreshAgentProject(
  projectPath: string,
): Promise<AgentMemoryDashboard> {
  return invoke("refresh_agent_project", { projectPath });
}

export function readAgentLearning(
  projectPath: string,
  learningId: string,
): Promise<LearningContext> {
  return invoke("read_agent_learning", { projectPath, learningId });
}

export function readAgentSession(
  projectPath: string,
  sessionId: string,
): Promise<SessionContext> {
  return invoke("read_agent_session", { projectPath, sessionId });
}

export function readAgentSessionTurns(
  projectPath: string,
  sessionId: string,
): Promise<SessionTurnsContext> {
  return invoke("read_agent_session_turns", { projectPath, sessionId });
}

export function renameAgentSession(
  projectPath: string,
  sessionId: string,
  expectedEventCount: number,
  name: string,
  note: string,
): Promise<AgentMemoryDashboard> {
  return invoke("rename_agent_session", {
    projectPath,
    sessionId,
    expectedEventCount,
    name,
    note,
  });
}

export function eraseAgentSession(
  projectPath: string,
  sessionId: string,
  expectedEventCount: number,
  expectedName: string,
): Promise<AgentSessionErasure> {
  return invoke("erase_agent_session", {
    projectPath,
    sessionId,
    expectedEventCount,
    expectedName,
  });
}

export function readAgentArtifacts(
  projectPath: string,
  query = "",
): Promise<ProjectArtifactInventory> {
  return invoke("read_agent_artifacts", {
    projectPath,
    query,
    maxResults: 300,
  });
}

export function readAgentMediaEvidence(
  projectPath: string,
  artifactPath: string,
  artifactSnapshotId: string,
  contentHash: string,
): Promise<AgentMediaEvidence> {
  return invoke("read_agent_media_evidence", {
    projectPath,
    artifactPath,
    artifactSnapshotId,
    contentHash,
    maxBytes: 1_048_576,
  });
}

export function readAgentProjectGraphView(
  projectPath: string,
  query = "",
  graphSnapshotId?: string,
  filters?: ProjectGraphFilters,
): Promise<ProjectGraphView> {
  return invoke("read_agent_project_graph_view", {
    projectPath,
    graphSnapshotId,
    query,
    maxNodes: 180,
    maxEdges: 600,
    filters,
  });
}

export function readAgentProjectGraphHistory(
  projectPath: string,
): Promise<ProjectGraphHistory> {
  return invoke("read_agent_project_graph_history", {
    projectPath,
    maxResults: 100,
  });
}

export function readAgentProjectGraphEvidence(
  projectPath: string,
  graphSnapshotId: string,
  citation: GraphCitation,
): Promise<ProjectGraphEvidenceExcerpt> {
  return invoke("read_agent_project_graph_evidence", {
    projectPath,
    graphSnapshotId,
    citation,
    contextLines: 3,
    maxCharacters: 8_000,
  });
}

export function readAgentCitedEvidence(
  projectPath: string,
  citation: GraphCitation,
): Promise<ProjectGraphEvidenceExcerpt> {
  return invoke("read_agent_cited_evidence", {
    projectPath,
    citation,
    contextLines: 3,
    maxCharacters: 8_000,
  });
}

export function readAgentProjectActivity(
  projectPath: string,
  query = "",
  problemScope: ProjectProblemScope = "all",
): Promise<ProjectActivityView> {
  return invoke("read_agent_project_activity", {
    projectPath,
    query,
    problemScope,
    maxResults: 120,
  });
}

export function reviewAgentLearning(
  projectPath: string,
  learningId: string,
  expectedEventCount: number,
  action: LearningAction,
  note: string,
): Promise<AgentMemoryDashboard> {
  return invoke("review_agent_learning", {
    projectPath,
    learningId,
    expectedEventCount,
    action,
    note,
  });
}

export function correctAgentLearning(
  projectPath: string,
  learningId: string,
  expectedEventCount: number,
  title: string,
  guidance: string,
  confidencePercent: number,
  note: string,
): Promise<AgentMemoryDashboard> {
  return invoke("correct_agent_learning", {
    projectPath,
    learningId,
    expectedEventCount,
    title,
    guidance,
    confidencePercent,
    note,
  });
}
