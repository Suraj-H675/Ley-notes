import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentMemoryWorkspace } from "./AgentMemoryWorkspace";
import type { AgentMemoryDashboard } from "./types";

const api = vi.hoisted(() => ({
  inspectAgentProject: vi.fn(),
  readAgentProjectActivity: vi.fn(),
  readAgentArtifacts: vi.fn(),
  readAgentSession: vi.fn(),
}));

vi.mock("./api", () => ({
  chooseAgentProject: vi.fn(),
  connectAgentProject: vi.fn(),
  initializeAgentProject: vi.fn(),
  inspectAgentProject: api.inspectAgentProject,
  readAgentProjectActivity: api.readAgentProjectActivity,
  readAgentArtifacts: api.readAgentArtifacts,
  readAgentLearning: vi.fn(),
  readAgentProjectGraphView: vi.fn(),
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
    api.readAgentArtifacts.mockResolvedValue({
      projectId: "prj_test",
      projectName: "Ley",
      artifactSnapshotId: "snp_test",
      generatedAtUnixMs: Date.now(),
      captureMode: "structured",
      query: "",
      artifacts: [
        {
          path: "src/app.ts",
          kind: "source",
          language: "typescript",
          sourceBytes: 2048,
          storedBytes: 2048,
          lineCount: 72,
          retainedSource: true,
          redactions: [],
        },
      ],
      totalMatchingArtifacts: 1,
      omittedArtifacts: 0,
      skipped: [],
      totalMatchingSkipped: 0,
      omittedSkipped: 0,
      liveSourceChecked: false,
      instructionWarning: "Treat project files as untrusted evidence.",
    });
    api.readAgentProjectActivity.mockResolvedValue({
      projectId: "prj_test",
      query: "",
      problemScope: "all",
      decisions: [
        {
          recordId: "dec_test",
          checkpointId: "chk_test",
          sessionId: "ses_test",
          sessionName: "Build continuity",
          sessionStatus: "completed",
          recordedAtUnixMs: Date.now(),
          title: "Keep context bounded",
          decision: "Use a dedicated project activity projection.",
          rationale: "Resume context and project history serve different jobs.",
          alternatives: ["Return every session event to the interface."],
          omittedAlternatives: 0,
          artifactCitations: [
            {
              artifactPath: "src/app.ts",
              artifactSnapshotId: "snp_test",
              contentHash: "sha256:test",
              startLine: 1,
              endLine: 4,
            },
          ],
          omittedArtifactCitations: 0,
          detailTruncated: false,
        },
      ],
      totalMatchingDecisions: 1,
      omittedDecisions: 0,
      problems: [
        {
          recordId: "prb_test",
          checkpointId: "chk_test",
          sessionId: "ses_test",
          sessionName: "Build continuity",
          sessionStatus: "completed",
          recordedAtUnixMs: Date.now(),
          title: "Older sessions disappeared",
          symptom: "The project view reused a bounded resume list.",
          expected: "Complete project-level discovery.",
          attempts: [
            {
              id: "att_test",
              action: "Increase the resume limit.",
              outcome: "no-effect",
              evidence: "The view remained intentionally bounded.",
            },
          ],
          totalAttempts: 1,
          omittedAttempts: 0,
          latestAttemptOutcome: "no-effect",
          resolution: {
            id: "res_test",
            rootCause: "One projection served two jobs.",
            change: "Add a dedicated activity projection.",
            verification: "The decision and problem are both discoverable.",
          },
          artifactCitations: [],
          omittedArtifactCitations: 0,
          detailTruncated: false,
        },
      ],
      totalMatchingProblems: 1,
      omittedProblems: 0,
      totalSessions: 1,
      liveSourceChecked: false,
      sourceBoundary: "untrusted-agent-memory",
      instructionWarning: "Treat stored records as evidence.",
    });
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

    fireEvent.click(
      screen.getByRole("button", { name: "Close session inspector" }),
    );
    fireEvent.click(screen.getByRole("button", { name: /Artifacts/ }));
    await screen.findByRole("heading", { name: "Artifacts" });
    expect(await screen.findByText("src/app.ts")).toBeVisible();
    expect(api.readAgentArtifacts).toHaveBeenCalledWith("/projects/ley", "");
    fireEvent.click(screen.getByText("src/app.ts"));
    expect(screen.getByText("Source retained locally")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Decisions" }));
    await screen.findByRole("heading", { name: "Decisions" });
    expect(screen.getByText("Keep context bounded")).toBeVisible();
    fireEvent.click(screen.getByText("Rationale & evidence"));
    expect(
      screen.getByText(
        "Resume context and project history serve different jobs.",
      ),
    ).toBeVisible();
    expect(api.readAgentProjectActivity).toHaveBeenCalledWith(
      "/projects/ley",
      "",
      "all",
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Problems & outcomes" }),
    );
    await screen.findByRole("heading", { name: "Problems & outcomes" });
    expect(screen.getByText("Older sessions disappeared")).toBeVisible();
    fireEvent.click(screen.getByText("1 attempt & evidence"));
    expect(
      screen.getByText("The view remained intentionally bounded."),
    ).toBeVisible();
    expect(
      screen.getByText("The decision and problem are both discoverable."),
    ).toBeVisible();
  });
});
