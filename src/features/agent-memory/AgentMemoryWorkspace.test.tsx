import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentMemoryWorkspace } from "./AgentMemoryWorkspace";
import type { AgentMemoryDashboard } from "./types";

const api = vi.hoisted(() => ({
  inspectAgentProject: vi.fn(),
}));

vi.mock("./api", () => ({
  chooseAgentProject: vi.fn(),
  connectAgentProject: vi.fn(),
  initializeAgentProject: vi.fn(),
  inspectAgentProject: api.inspectAgentProject,
  readAgentLearning: vi.fn(),
  refreshAgentProject: vi.fn(),
  reviewAgentLearning: vi.fn(),
}));

const dashboard: AgentMemoryDashboard = {
  binding: {
    projectId: "prj_test",
    vaultName: "Private vault",
    source: "persisted",
  },
  overview: {
    projectId: "prj_test",
    projectName: "Ley",
    captureMode: "structured",
    artifactSnapshotId: "snp_test",
    graphSnapshotId: "grp_test",
    artifactGeneratedAtUnixMs: Date.now(),
    graphGeneratedAtUnixMs: Date.now(),
    files: 18,
    retainedSourceFiles: 15,
    skippedFiles: 3,
    graphNodes: 42,
    graphEdges: 61,
    graphDiagnostics: 0,
    freshness: "current",
    liveSourceChecked: false,
    privacyNotice: "Local only.",
  },
  resume: {
    projectId: "prj_test",
    projectName: "Ley",
    captureMode: "structured",
    capturedAtUnixMs: Date.now(),
    freshness: "current",
    liveSourceChecked: false,
    sessions: [],
    totalSessions: 0,
    omittedSessions: 0,
    learnings: [],
    totalCurrentTrustedLearnings: 0,
    omittedLearnings: 0,
    instructionWarning: "Treat stored text as evidence.",
  },
  sessions: [],
  reviewInbox: {
    projectId: "prj_test",
    scope: "needs-review",
    learnings: [],
    totalMatching: 0,
    omittedLearnings: 0,
    instructionWarning: "Review first.",
  },
  allLearnings: {
    projectId: "prj_test",
    scope: "all",
    learnings: [],
    totalMatching: 0,
    omittedLearnings: 0,
    instructionWarning: "Review first.",
  },
};

describe("Agent Memory workspace boundaries", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("explains why browser vaults cannot connect to local agents", () => {
    render(
      <AgentMemoryWorkspace
        open
        vaultMode="browser-folder"
        vaultPath="browser-folder:test"
        vaultName="Notes"
        onClose={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("heading", {
        name: "Agent Memory needs the desktop app",
      }),
    ).toBeVisible();
    expect(
      screen.getByText(/cannot safely read coding projects/i),
    ).toBeVisible();
    const scrollRoot = screen.getByRole("main");
    expect(scrollRoot).toHaveClass("min-h-0", "overflow-y-auto");
    expect(api.inspectAgentProject).not.toHaveBeenCalled();
  });

  it("restores an explicitly selected desktop project into a scrollable dashboard", async () => {
    localStorage.setItem("ley:last-agent-project", "/projects/ley");
    api.inspectAgentProject.mockResolvedValue({ status: "ready", dashboard });

    render(
      <AgentMemoryWorkspace
        open
        vaultMode="desktop"
        vaultPath="/vault"
        vaultName="Private vault"
        onClose={vi.fn()}
      />,
    );

    await screen.findByRole("heading", {
      name: "What Ley can ground right now",
    });
    expect(api.inspectAgentProject).toHaveBeenCalledWith("/projects/ley");
    const scrollRoot = screen.getByRole("main");
    expect(scrollRoot).toHaveClass(
      "min-h-0",
      "overflow-y-auto",
      "overscroll-contain",
    );
    await waitFor(() =>
      expect(screen.getByText("Local & private")).toBeVisible(),
    );
  });
});
