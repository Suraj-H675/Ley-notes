import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentBriefPreview } from "./AgentBriefPreview";
import type {
  AgentBriefPreview as AgentBriefPreviewResult,
  AgentEgressTarget,
} from "./types";

const api = vi.hoisted(() => ({
  preview: vi.fn(),
}));

vi.mock("./api", () => ({
  previewAgentBrief: api.preview,
}));

const citation = {
  artifactPath: "src/cache.ts",
  startLine: 4,
  startColumn: 1,
  endLine: 8,
  endColumn: 1,
  contentHash: "sha256:cache",
  artifactSnapshotId: "snp_cache",
};

function result(target: AgentEgressTarget): AgentBriefPreviewResult {
  return {
    contextPackId: "ctx_test",
    createdAtUnixMs: 1,
    projectId: "prj_test",
    projectName: "Ley",
    task: "Fix cache invalidation",
    evidenceState: "good-evidence",
    premiseAdjudication: {
      state: "no-detected-mismatch",
      warnings: [],
      omittedWarnings: 0,
    },
    egressTarget: target,
    egressExclusions: [],
    egressCoverage: {
      target,
      blockedSpecifications: 0,
      blockedMounts: 0,
      blockedExternalConnectors: 0,
      blockedHistoricalSources: 0,
      blockedPolicyBundleSources: 0,
      historicalMemoryWithheld: false,
      withheldDerivedResults: 0,
    },
    maxTokens: 1500,
    estimatedTokens: 240,
    specifications: [],
    items: [
      {
        kind: "decision",
        entityId: "dec_test",
        title: "Invalidate by namespace",
        excerpt: "Cache invalidation uses a namespaced key.",
        citation,
        authority: "historical-project-memory",
        trustedForReuse: false,
        estimatedTokens: 42,
      },
    ],
    gaps: [],
    coverage: {
      returnedItems: 1,
      returnedConflicts: 0,
      returnedExclusions: 0,
      returnedGaps: 0,
      omittedGaps: 0,
      omittedConflicts: 0,
      omittedExclusions: 0,
      searchTruncated: false,
      sourceTruncated: false,
    },
    liveSourceChecked: false,
    sourceBoundary: "mixed-authority-context",
    instructionWarning: "Revalidate consequential current-state claims.",
    privacyNotice: "Bounded local continuity preview.",
  };
}

describe("AgentBriefPreview", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.preview.mockImplementation(
      async (
        _projectPath: string,
        _projectId: string,
        _task: string,
        target: AgentEgressTarget,
      ) => result(target),
    );
  });

  it("previews the canonical cloud brief and opens cited evidence", async () => {
    const onEvidence = vi.fn();
    render(
      <AgentBriefPreview
        projectPath="/projects/ley"
        projectId="prj_test"
        onEvidence={onEvidence}
      />,
    );

    fireEvent.change(screen.getByPlaceholderText("What should the next agent work on?"), {
      target: { value: "Fix cache invalidation" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Preview agent brief" }));

    await waitFor(() => {
      expect(api.preview).toHaveBeenCalledWith(
        "/projects/ley",
        "prj_test",
        "Fix cache invalidation",
        "cloud",
      );
    });
    expect(await screen.findByText("Invalidate by namespace")).toBeVisible();
    expect(screen.getByText("240 / 1500 tokens")).toBeVisible();
    expect(screen.getByText("Raw canonical compiler payload")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Open cited evidence" }));
    expect(onEvidence).toHaveBeenCalledWith(citation);

    fireEvent.change(
      screen.getByPlaceholderText("What should the next agent work on?"),
      { target: { value: "Different task" } },
    );
    expect(screen.queryByText("Invalidate by namespace")).not.toBeInTheDocument();
  });

  it("uses an explicitly selected local target instead of inferring locality", async () => {
    render(
      <AgentBriefPreview
        projectPath="/projects/ley"
        projectId="prj_test"
        onEvidence={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByPlaceholderText("What should the next agent work on?"), {
      target: { value: "Fix cache invalidation" },
    });
    fireEvent.change(screen.getByLabelText("Agent brief egress target"), {
      target: { value: "local" },
    });
    expect(
      screen.getByText(/does not attest that the downstream provider or runtime is actually local/i),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Preview agent brief" }));

    await waitFor(() => {
      expect(api.preview).toHaveBeenCalledWith(
        "/projects/ley",
        "prj_test",
        "Fix cache invalidation",
        "local",
      );
    });

    fireEvent.change(screen.getByLabelText("Agent brief egress target"), {
      target: { value: "cloud" },
    });
    expect(screen.queryByText("Invalidate by namespace")).not.toBeInTheDocument();
  });
});
