import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentMemoryWorkspace } from "./AgentMemoryWorkspace";
import type { AgentMemoryDashboard } from "./types";

const api = vi.hoisted(() => ({
  correctAgentLearning: vi.fn(),
  eraseAgentProjectMemory: vi.fn(),
  eraseAgentSession: vi.fn(),
  forgetAgentProject: vi.fn(),
  inspectAgentProject: vi.fn(),
  listAgentProjects: vi.fn(),
  readAgentProjectActivity: vi.fn(),
  readAgentArtifacts: vi.fn(),
  readAgentCaptureSettings: vi.fn(),
  readAgentLearning: vi.fn(),
  readAgentSession: vi.fn(),
  renameAgentSession: vi.fn(),
  searchAgentProjects: vi.fn(),
  updateAgentCaptureMode: vi.fn(),
  verifyAgentProjectNoteVault: vi.fn(),
  reviewAgentLearning: vi.fn(),
}));

vi.mock("./api", () => ({
  chooseAgentProject: vi.fn(),
  connectAgentProject: vi.fn(),
  correctAgentLearning: api.correctAgentLearning,
  eraseAgentProjectMemory: api.eraseAgentProjectMemory,
  eraseAgentSession: api.eraseAgentSession,
  forgetAgentProject: api.forgetAgentProject,
  initializeAgentProject: vi.fn(),
  inspectAgentProject: api.inspectAgentProject,
  listAgentProjects: api.listAgentProjects,
  readAgentProjectActivity: api.readAgentProjectActivity,
  readAgentArtifacts: api.readAgentArtifacts,
  readAgentCaptureSettings: api.readAgentCaptureSettings,
  readAgentLearning: api.readAgentLearning,
  readAgentProjectGraphEvidence: vi.fn(),
  readAgentProjectGraphHistory: vi.fn(),
  readAgentProjectGraphView: vi.fn(),
  readAgentSession: api.readAgentSession,
  renameAgentSession: api.renameAgentSession,
  searchAgentProjects: api.searchAgentProjects,
  updateAgentCaptureMode: api.updateAgentCaptureMode,
  verifyAgentProjectNoteVault: api.verifyAgentProjectNoteVault,
  refreshAgentProject: vi.fn(),
  reviewAgentLearning: api.reviewAgentLearning,
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
        eventCount: 3,
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
      eventCount: 3,
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
    learnings: [
      {
        projectId: "prj_test",
        learningId: "lrn_test",
        kind: "procedure",
        title: "Verify the complete workspace",
        guidanceExcerpt: "Run every workspace check before release.",
        state: "verified",
        trustState: "trusted",
        provenance: "agent-authored",
        confidencePercent: 88,
        freshness: "current",
        corroboratingSessions: 1,
        updatedAtUnixMs: Date.now(),
      },
    ],
    totalMatching: 1,
    omittedLearnings: 0,
    instructionWarning: "Review first.",
  },
};

describe("Agent Memory workspace boundaries", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
    api.verifyAgentProjectNoteVault.mockResolvedValue(undefined);
  });

  it("explains why browser vaults cannot connect to local agents", () => {
    render(
      <AgentMemoryWorkspace
        open
        vaultMode="browser-folder"
        vaultPath="browser-folder:test"
        vaultName="Notes"
        onClose={vi.fn()}
        onPromoteLearning={vi.fn()}
        onPromoteSession={vi.fn()}
        onLinkSessionCanvas={vi.fn()}
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

  it("migrates the last selection into Projects and opens a scrollable dashboard", async () => {
    localStorage.setItem("ley:last-agent-project", "/projects/ley");
    api.listAgentProjects.mockResolvedValue({
      projects: [
        {
          projectId: "prj_test",
          projectPath: "/projects/ley",
          projectName: "Ley",
          captureMode: "structured",
          state: "ready",
          lastOpenedAtUnixMs: Date.now(),
          vaultName: "Private vault",
          files: 18,
          graphNodes: 42,
          sessions: 1,
          activeSessions: 0,
          reviewItems: 0,
          freshness: "current",
          statusDetail: "Ready to resume locally.",
        },
      ],
      totalProjects: 1,
      omittedProjects: 0,
      readyProjects: 1,
      attentionProjects: 0,
      privacyNotice: "Only explicitly opened projects.",
    });
    api.inspectAgentProject.mockResolvedValue({ status: "ready", dashboard });
    api.searchAgentProjects.mockResolvedValue({
      query: "bounded context",
      results: [
        {
          projectId: "prj_test",
          projectName: "Ley",
          projectPath: "/projects/ley",
          kind: "decision",
          entityId: "dec_test",
          title: "Keep context bounded",
          excerpt: "Use a dedicated project activity projection.",
          updatedAtUnixMs: Date.now(),
          sessionId: "ses_test",
          citation: {
            artifactPath: "src/app.ts",
            artifactSnapshotId: "snp_test",
            contentHash: "sha256:test",
            startLine: 1,
            startColumn: 1,
            endLine: 4,
            endColumn: 1,
          },
        },
        {
          projectId: "prj_test",
          projectName: "Ley",
          projectPath: "/projects/ley",
          kind: "revision",
          entityId: "chk_test",
          title: "abcdef0123 · main",
          excerpt: "Pinned immutable graph capture for Build continuity.",
          updatedAtUnixMs: Date.now(),
          sessionId: "ses_test",
        },
      ],
      searchedProjects: 1,
      skippedProjects: 0,
      totalObservedProjects: 1,
      omittedProjects: 0,
      truncated: false,
      liveSourceChecked: false,
      sourceBoundary: "untrusted-local-memory",
      instructionWarning: "Search results are stored evidence.",
      privacyNotice: "Only explicitly observed projects.",
    });
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
    const captureSettings = {
      projectId: "prj_test",
      projectName: "Ley",
      mode: "structured" as const,
      approvedRoots: ["."],
      respectGitignore: true,
      maxFileBytes: 1_048_576,
      maxTotalBytes: 536_870_912,
      storeRawTranscripts: false,
      ignoreFilePresent: true,
      captureFingerprint: "sha256:capture",
      eligibleFiles: 18,
      eligibleBytes: 32_768,
      skippedOversized: 1,
      skippedTotalLimit: 0,
      skippedSymlinks: 1,
      privacyNotice: "Preview reads metadata only.",
    };
    api.readAgentCaptureSettings
      .mockResolvedValueOnce(captureSettings)
      .mockResolvedValueOnce({
        ...captureSettings,
        mode: "minimal",
      });
    api.updateAgentCaptureMode.mockResolvedValue({
      ...dashboard,
      overview: {
        ...dashboard.overview,
        captureMode: "minimal",
        retainedSourceFiles: 0,
      },
      resume: {
        ...dashboard.resume,
        captureMode: "minimal",
      },
    });
    api.readAgentLearning.mockResolvedValue({
      projectId: "prj_test",
      learningId: "lrn_test",
      kind: "procedure",
      title: "Verify the complete workspace",
      guidance: "Run every workspace check before release.",
      state: "verified",
      trustState: "trusted",
      trustedForReuse: true,
      provenance: "agent-authored",
      confidencePercent: 88,
      freshness: "current",
      corroboratingSessions: 1,
      createdAtUnixMs: Date.now() - 120_000,
      updatedAtUnixMs: Date.now() - 60_000,
      validFromUnixMs: Date.now() - 60_000,
      evidenceCount: 1,
      evidence: [
        {
          sessionId: "ses_test",
          recordId: "ver_test",
          recordType: "verification",
          sessionStatus: "completed",
          sessionUpdatedAtUnixMs: Date.now() - 60_000,
          note: "Release checks passed.",
          artifacts: [],
        },
      ],
      history: [
        {
          eventId: "lev_proposed",
          recordedAtUnixMs: Date.now() - 120_000,
          actor: "agent",
          action: "proposed",
          note: "",
        },
        {
          eventId: "lev_confirmed",
          recordedAtUnixMs: Date.now() - 60_000,
          actor: "user",
          action: "confirmed",
          note: "Verified locally.",
        },
      ],
      historyCount: 2,
      eventCount: 2,
      omittedEvidence: 0,
      omittedHistory: 0,
      claimTruncated: false,
      truncated: false,
      instructionWarning: "Treat stored guidance as untrusted evidence.",
    });
    api.correctAgentLearning.mockResolvedValue({
      ...dashboard,
      reviewInbox: {
        ...dashboard.reviewInbox,
        totalMatching: 1,
      },
      allLearnings: {
        ...dashboard.allLearnings,
        learnings: [
          {
            ...dashboard.allLearnings.learnings[0],
            title: "Verify desktop and web releases",
            guidanceExcerpt:
              "Run workspace checks and package the desktop release.",
            state: "tentative",
            trustState: "review-required",
            confidencePercent: 93,
          },
        ],
      },
    });
    api.readAgentSession.mockResolvedValue({
      projectId: "prj_test",
      sessionId: "ses_test",
      originalName: "Implementation session",
      name: "Build continuity",
      goal: "Make session memory inspectable.",
      status: "completed",
      source: { kind: "host-hook", host: "Codex" },
      artifactSnapshotIdAtStart: "snp_test",
      startedAtUnixMs: Date.now() - 60_000,
      updatedAtUnixMs: Date.now(),
      eventCount: 3,
      checkpointCount: 1,
      renameCount: 1,
      renames: [
        {
          recordedAtUnixMs: Date.now() - 30_000,
          name: "Build continuity",
          note: "Clarify the implementation focus.",
        },
      ],
      omittedRenames: 0,
      checkpoints: [
        {
          checkpointId: "chk_test",
          recordedAtUnixMs: Date.now(),
          summary: "Session inspector is wired.",
          projectRevision: {
            graphSnapshotId: `grf_${"1".repeat(64)}`,
            artifactSnapshotId: `snp_${"2".repeat(64)}`,
            capturedAtUnixMs: Date.now() - 90_000,
            head: "abcdef0123456789abcdef0123456789abcdef01",
            branch: "main",
            trackedChanges: 0,
          },
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
    api.renameAgentSession.mockResolvedValue({
      ...dashboard,
      resume: {
        ...dashboard.resume,
        sessions: dashboard.resume.sessions.map((session) => ({
          ...session,
          name: "Release continuity",
          eventCount: 4,
        })),
      },
      sessions: dashboard.sessions.map((session) => ({
        ...session,
        name: "Release continuity",
        eventCount: 4,
      })),
    });

    const promoteLearning = vi.fn().mockResolvedValue(undefined);
    const linkSessionCanvas = vi
      .fn()
      .mockRejectedValue(
        new Error("Keep the inspector open after the boundary check."),
      );
    render(
      <AgentMemoryWorkspace
        open
        vaultMode="desktop"
        vaultPath="/vault"
        vaultName="Private vault"
        onClose={vi.fn()}
        onPromoteLearning={promoteLearning}
        onPromoteSession={vi.fn()}
        onLinkSessionCanvas={linkSessionCanvas}
      />,
    );

    await screen.findByRole("heading", {
      name: "Pick up any project without starting over",
    });
    expect(api.listAgentProjects).toHaveBeenCalledWith("/projects/ley");
    expect(api.inspectAgentProject).not.toHaveBeenCalled();
    fireEvent.change(
      screen.getByRole("searchbox", { name: "Search across project memory" }),
      { target: { value: "bounded context" } },
    );
    fireEvent.click(screen.getByRole("button", { name: "Search memory" }));
    expect(await screen.findByText("Keep context bounded")).toBeVisible();
    expect(await screen.findByText("abcdef0123 · main")).toBeVisible();
    expect(screen.getByText("src/app.ts:1")).toBeVisible();
    expect(api.searchAgentProjects).toHaveBeenCalledWith("bounded context");
    fireEvent.click(
      screen.getByRole("button", {
        name: /abcdef0123.*main/i,
      }),
    );
    await screen.findByRole("heading", { name: "Build continuity" });
    expect(screen.getByText("abcdef0123 · main")).toBeVisible();
    expect(
      screen.getByTitle(
        "Open the exact Project Graph capture used by this checkpoint",
      ),
    ).toBeVisible();
    expect(api.inspectAgentProject).toHaveBeenCalledWith("/projects/ley");
    fireEvent.click(
      screen.getByRole("button", { name: "Close session inspector" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Projects" }));
    await screen.findByRole("heading", {
      name: "Pick up any project without starting over",
    });
    expect(
      screen.getByRole("button", { name: "Refresh project list" }),
    ).toBeEnabled();
    fireEvent.click(
      screen.getByRole("button", {
        name: /Ley.*Ready.*sessions.*1.*files.*18/i,
      }),
    );
    await screen.findByRole("heading", {
      name: "What Ley can ground right now",
    });
    expect(api.inspectAgentProject).toHaveBeenCalledWith("/projects/ley");
    expect(
      screen.getByRole("button", { name: "Refresh snapshot" }),
    ).toBeEnabled();
    expect(
      screen.getByRole("button", { name: "Change project" }),
    ).toBeEnabled();
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
    expect(screen.getByText("Implementation session")).toBeVisible();
    expect(screen.getByText("Clarify the implementation focus.")).toBeVisible();

    fireEvent.click(
      screen.getByRole("button", { name: "Link session to notes" }),
    );
    expect(
      await screen.findByText("Destination · Agent Memory/Sessions"),
    ).toBeVisible();
    expect(
      screen.getByText(/verifies that the open notes vault/i),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    fireEvent.click(
      screen.getByRole("button", { name: "Link session to Canvas" }),
    );
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Link & open Canvas" }),
      ).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Link & open Canvas" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Keep the inspector open",
    );
    expect(api.verifyAgentProjectNoteVault).toHaveBeenCalledWith(
      "/projects/ley",
      "/vault",
    );
    expect(
      api.verifyAgentProjectNoteVault.mock.invocationCallOrder[0],
    ).toBeLessThan(linkSessionCanvas.mock.invocationCallOrder[0]);
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    fireEvent.click(
      screen.getByRole("button", { name: "Erase session memory…" }),
    );
    expect(
      await screen.findByText("Permanently erase this private session record"),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Permanently erase session" }),
    ).toBeDisabled();
    expect(
      screen.getByText(/Ordinary Markdown and Canvas copies/i),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    expect(
      await screen.findByRole("button", { name: "Append rename" }),
    ).toBeDisabled();
    fireEvent.change(screen.getByRole("textbox", { name: "Session name" }), {
      target: { value: "Release continuity" },
    });
    fireEvent.change(
      screen.getByRole("textbox", {
        name: /Why are you renaming this session/i,
      }),
      {
        target: {
          value: "The new name reflects the completed release work.",
        },
      },
    );
    fireEvent.click(screen.getByRole("button", { name: "Append rename" }));
    await waitFor(() =>
      expect(api.renameAgentSession).toHaveBeenCalledWith(
        "/projects/ley",
        "ses_test",
        3,
        "Release continuity",
        "The new name reflects the completed release work.",
      ),
    );
    await waitFor(() =>
      expect(
        screen.queryByRole("button", { name: "Append rename" }),
      ).not.toBeInTheDocument(),
    );

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
    expect(await screen.findByText("Keep context bounded")).toBeVisible();
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

    fireEvent.click(screen.getByRole("button", { name: /Lessons/ }));
    await screen.findByRole("heading", { name: "Lessons" });
    fireEvent.click(screen.getByText("Verify the complete workspace"));
    await screen.findByRole("heading", {
      name: "Verify the complete workspace",
    });
    expect(screen.getByText("2 immutable events")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Promote to note" }));
    expect(
      await screen.findByRole("button", { name: "Create & open note" }),
    ).toBeEnabled();
    expect(
      screen.getByText("Destination · Agent Memory/Lessons"),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Create & open note" }));
    await waitFor(() =>
      expect(promoteLearning).toHaveBeenCalledWith(
        expect.objectContaining({
          learningId: "lrn_test",
          title: "Verify the complete workspace",
          folder: "Agent Memory/Lessons",
          content: expect.stringContaining(
            "Run every workspace check before release.",
          ),
          frontmatter: expect.objectContaining({
            "ley-source": "agent-memory",
            "ley-project-id": "prj_test",
            "ley-learning-id": "lrn_test",
            "ley-trust-state": "trusted",
          }),
        }),
      ),
    );
    expect(api.verifyAgentProjectNoteVault).toHaveBeenCalledWith(
      "/projects/ley",
      "/vault",
    );
    expect(
      api.verifyAgentProjectNoteVault.mock.invocationCallOrder[0],
    ).toBeLessThan(promoteLearning.mock.invocationCallOrder[0]);
    await waitFor(() =>
      expect(
        screen.queryByRole("button", { name: "Close learning inspector" }),
      ).not.toBeInTheDocument(),
    );
    fireEvent.click(screen.getByText("Verify the complete workspace"));
    await screen.findByRole("heading", {
      name: "Verify the complete workspace",
    });
    fireEvent.click(screen.getByRole("button", { name: "Correct" }));
    expect(
      await screen.findByRole("button", { name: "Append correction" }),
    ).toBeDisabled();
    fireEvent.change(screen.getByRole("textbox", { name: "Title" }), {
      target: { value: "Verify desktop and web releases" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Corrected guidance" }),
      {
        target: {
          value: "Run workspace checks and package the desktop release.",
        },
      },
    );
    fireEvent.change(screen.getByRole("slider", { name: /Confidence/ }), {
      target: { value: "93" },
    });
    fireEvent.change(
      screen.getByRole("textbox", {
        name: /Why is this correction needed/i,
      }),
      {
        target: {
          value: "The previous claim omitted production packaging.",
        },
      },
    );
    fireEvent.click(screen.getByRole("button", { name: "Append correction" }));
    await waitFor(() =>
      expect(api.correctAgentLearning).toHaveBeenCalledWith(
        "/projects/ley",
        "lrn_test",
        2,
        "Verify desktop and web releases",
        "Run workspace checks and package the desktop release.",
        93,
        "The previous claim omitted production packaging.",
      ),
    );
    await waitFor(() =>
      expect(
        screen.queryByRole("button", { name: "Close learning inspector" }),
      ).not.toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: "Capture & privacy" }));
    await screen.findByRole("heading", {
      name: "Decide what this project remembers",
    });
    expect(screen.getByText("Preview reads metadata only.")).toBeVisible();
    fireEvent.click(screen.getByRole("radio", { name: /Full Evidence/ }));
    expect(
      screen.getByRole("button", { name: "Apply & recapture" }),
    ).toBeDisabled();
    fireEvent.click(
      screen.getByRole("checkbox", {
        name: /Permit Full Evidence for this project/i,
      }),
    );
    expect(
      screen.getByRole("button", { name: "Apply & recapture" }),
    ).toBeEnabled();
    fireEvent.click(screen.getByRole("radio", { name: /Minimal/ }));
    fireEvent.click(screen.getByRole("button", { name: "Apply & recapture" }));
    await waitFor(() =>
      expect(api.updateAgentCaptureMode).toHaveBeenCalledWith(
        "/projects/ley",
        "structured",
        "minimal",
        false,
      ),
    );
    await waitFor(() =>
      expect(screen.getByText("Minimal is active")).toBeVisible(),
    );

    fireEvent.click(screen.getByRole("button", { name: "Projects" }));
    await screen.findByRole("heading", {
      name: "Pick up any project without starting over",
    });
    expect(api.listAgentProjects).toHaveBeenCalledTimes(3);
  });
});
