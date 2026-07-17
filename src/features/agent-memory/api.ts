import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AgentCaptureSettings,
  AgentMemoryDashboard,
  AgentProjectSearch,
  AgentProjectCatalog,
  AgentProjectInspection,
  LearningAction,
  LearningContext,
  ProjectActivityView,
  ProjectArtifactInventory,
  ProjectGraphView,
  ProjectProblemScope,
  SessionContext,
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

export function inspectAgentProject(
  projectPath: string,
): Promise<AgentProjectInspection> {
  return invoke("inspect_agent_project", { projectPath });
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

export function readAgentProjectGraphView(
  projectPath: string,
  query = "",
): Promise<ProjectGraphView> {
  return invoke("read_agent_project_graph_view", {
    projectPath,
    query,
    maxNodes: 180,
    maxEdges: 600,
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
  action: LearningAction,
  note: string,
): Promise<AgentMemoryDashboard> {
  return invoke("review_agent_learning", {
    projectPath,
    learningId,
    action,
    note,
  });
}
