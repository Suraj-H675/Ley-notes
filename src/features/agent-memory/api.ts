import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AgentMemoryDashboard,
  AgentProjectInspection,
  LearningAction,
  LearningContext,
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
