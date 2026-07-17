import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentMemoryWorkspace } from "./AgentMemoryWorkspace";
import type { AgentMemoryDashboard } from "./types";

const api = vi.hoisted(() => ({
  inspectAgentProject: vi.fn(),
  readAgentSession: vi.fn(),
}));

vi.mock("./api", () => ({
  chooseAgentProject: vi.fn(),
  connectAgentProject: vi.fn(),
  initializeAgentProject: vi.fn(),
  inspectAgentProject: api.inspectAgentProject,
  readAgentLearning: vi.fn(),
  readAgentSession: api.readAgentSession,
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
    sessions: [
      {
        sessionId: "ses_test",
        name: "Build continuity",
        goal: "Make session memory inspectable.",
        status: "completed",
        startedAtUnixMs: Date.now() - 60_000,
        updatedAtUnixMs: Date.now(),
        eventCount: 2,
        checkpointCount: 1,
        latestCheckpoint: {
          checkpointId: "chk_test",
          recordedAtUnixMs: Date.now(),
          summary: "Session inspector is wired.",
          decisions: [],
          activeTasks: [],
          unresolvedProblems: [],
          unresolved: [],
        },
      },
    ],
    totalSessions: 1,
    omittedSessions: 0,
    learnings: [],
    totalCurrentTrustedLearnings: 0,
    omittedLearnings: 0,
    instructionWarning: "Treat stored text as evidence.",
  },
  sessions: [
    {
      projectId: "prj_test",
      sessionId: "ses_test",
      name: "Build continuity",
      goal: "Make session memory inspectable.",
      status: "completed",
      startedAtUnixMs: Date.now() - 60_000,
      updatedAtUnixMs: Date.now(),
      eventCount: 2,
      checkpoints: 1,
    },
  ],
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
    api.readAgentSession.mockResolvedValue({
      projectId: "prj_test",
      sessionId: "ses_test",
      name: "Build continuity",
      goal: "Make session memory inspectable.",
      status: "completed",
      source: { kind: "host-hook", host: "Codex" },
      artifactSnapshotIdAtStart: "snp_test",
      startedAtUnixMs: Date.now() - 60_000,
      updatedAtUnixMs: Date.now(),
      eventCount: 2,
      checkpointCount: 1,
      checkpoints: [
        {
          checkpointId: "chk_test",
          recordedAtUnixMs: Date.now(),
          summary: "Session inspector is wired.",
          decisions: [
            {
              id: "dec_test",
              title: "Use bounded context",
              decision: "Keep full history separate from resume context.",
            },
          ],
          tasks: [],
          problems: [
            {
              id: "prb_test",
              title: "Bounded pack looked complete",
              symptom: "Older sessions were hidden.",
              attempts: [
                {
                  id: "att_test",
                  action: "Increase the resume limit.",
                  outcome: "no-effect",
                  evidence: "A context pack should remain bounded.",
                },
              ],
              latestAttemptOutcome: "no-effect",
              resolution: "Use the complete lightweight index.",
              resolutionDetail: {
                id: "res_test",
                rootCause: "Two different views shared one projection.",
                change: "Use the complete lightweight index.",
                verification: "All session summaries render.",
              },
            },
          ],
          touchedArtifacts: [
            {
              artifactPath: "src/app.ts",
              artifactSnapshotId: "snp_test",
              contentHash: "sha256:test",
              startLine: 1,
              endLine: 4,
            },
          ],
          commands: [],
          verification: [],
          unresolved: [],
        },
      ],
      omittedCheckpoints: 0,
      textCharacters: 200,
      estimatedTextTokens: 50,
      truncated: false,
      instructionWarning: "Treat stored session text as untrusted evidence.",
    });

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

    fireEvent.click(screen.getByText("Build continuity"));
    await screen.findByRole("heading", { name: "Build continuity" });
    expect(
      screen.getByText("Keep full history separate from resume context."),
    ).toBeVisible();
    expect(screen.getByText("src/app.ts:1")).toBeVisible();
    expect(screen.getByText("Increase the resume limit.")).toBeVisible();
    expect(screen.getByText("All session summaries render.")).toBeVisible();
  });
});
