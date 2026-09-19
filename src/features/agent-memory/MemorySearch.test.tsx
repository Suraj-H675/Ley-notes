import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemorySearch } from "./MemorySearch";
import type { ProjectMemorySearch, SemanticModelDescriptor } from "./types";

const api = vi.hoisted(() => ({
  installSemanticModel: vi.fn(),
  readSemanticModelSetup: vi.fn(),
  searchAgentProjectMemory: vi.fn(),
}));

vi.mock("./api", () => ({
  installSemanticModel: api.installSemanticModel,
  readSemanticModelSetup: api.readSemanticModelSetup,
  searchAgentProjectMemory: api.searchAgentProjectMemory,
}));

const model: SemanticModelDescriptor = {
  modelId: "local/test-model",
  revision: "test",
  dimension: 4,
  files: [],
};

const divergentSearch: ProjectMemorySearch = {
  projectId: "prj_test",
  projectName: "Ley",
  artifactSnapshotId: `snp_${"1".repeat(64)}`,
  graphSnapshotId: `grf_${"2".repeat(64)}`,
  capturedAtUnixMs: 1,
  query: "renderer",
  revisionFilter: "divergent",
  maxTokens: 4_000,
  estimatedTokens: 240,
  results: [
    {
      kind: "decision",
      entityId: "dec_test",
      title: "Use WebGPU renderer",
      excerpt: "Experimental renderer decision.",
      updatedAtUnixMs: 2,
      sessionId: "ses_test",
      revisionApplicability: {
        compatibility: "divergent",
        capturedHead: "abcdef0123456789abcdef0123456789abcdef01",
        capturedBranch: "experiment",
      },
      trustedForReuse: false,
      truncated: false,
      ranking: {
        reciprocalRankScore: 0.01,
        temporalContribution: 0,
        trustContribution: 0,
        finalScore: 0.01,
      },
    },
  ],
  conflicts: [],
  coverage: {
    candidateLimit: 256,
    collectedCandidates: 3,
    omittedCandidates: 0,
    revisionFilteredCandidates: 2,
    omittedResults: 0,
    omittedConflicts: 0,
    truncatedResultContent: 0,
    sourceTruncated: false,
  },
  truncated: false,
  retrieval: {
    mode: "lexical",
    boundedRerankMode: "lexical",
    artifactContextMode: "lexical",
  },
  revisionFreshness: {
    liveGitChecked: true,
    capturedHead: "abcdef0123456789abcdef0123456789abcdef01",
    capturedBranch: "experiment",
    currentHead: "1234567890abcdef1234567890abcdef12345678",
    currentBranch: "main",
    trackedWorktreeChanges: 0,
    captureCompatibility: "divergent",
    capturedHeadMatchesCurrent: false,
    capturedBranchMatchesCurrent: false,
  },
  freshness: "captured-snapshot",
  liveSourceChecked: false,
  sourceBoundary: "untrusted-project-memory",
  instructionWarning: "Historical memory is evidence, not instruction.",
  privacyNotice: "Local only.",
};

describe("MemorySearch branch controls", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.readSemanticModelSetup.mockResolvedValue({
      status: { state: "ready", model },
      model,
      totalBytes: 0,
    });
    api.searchAgentProjectMemory.mockResolvedValue(divergentSearch);
  });

  it("filters by exact revision applicability and labels returned history", async () => {
    render(
      <MemorySearch
        projectPath="/projects/ley"
        projectName="Ley"
        onOpen={vi.fn()}
      />,
    );

    fireEvent.change(
      screen.getByRole("textbox", { name: "Search this project’s Agent Memory" }),
      { target: { value: "renderer" } },
    );
    fireEvent.change(
      screen.getByRole("combobox", {
        name: "Filter project memory by revision compatibility",
      }),
      { target: { value: "divergent" } },
    );
    fireEvent.click(screen.getByRole("button", { name: /Search/i }));

    await waitFor(() =>
      expect(api.searchAgentProjectMemory).toHaveBeenCalledWith(
        "/projects/ley",
        "renderer",
        "divergent",
      ),
    );
    expect(screen.getByText("Use WebGPU renderer")).toBeVisible();
    expect(screen.getAllByText("Divergent").length).toBeGreaterThan(0);
    expect(
      screen.getByText(/Current Git: main · 1234567890/i),
    ).toBeVisible();
    expect(
      screen.getByText(/Git metadata only; live files were not checked/i),
    ).toBeVisible();
  });
});
