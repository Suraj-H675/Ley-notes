import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ArtifactExplorer } from "./ArtifactExplorer";
import type { AgentMediaEvidence, ProjectArtifactInventory } from "./types";

const api = vi.hoisted(() => ({
  readArtifacts: vi.fn(),
  readMedia: vi.fn(),
}));

vi.mock("./api", () => ({
  readAgentArtifacts: api.readArtifacts,
  readAgentMediaEvidence: api.readMedia,
}));

const snapshotId = `snp_${"a".repeat(64)}`;
const contentHash = `sha256:${"b".repeat(64)}`;

const inventory: ProjectArtifactInventory = {
  projectId: "prj_test",
  projectName: "Ley",
  artifactSnapshotId: snapshotId,
  generatedAtUnixMs: 1,
  captureMode: "full-evidence",
  query: "",
  artifacts: [
    {
      path: "verification.png",
      kind: "image",
      mediaType: "png",
      contentHash,
      sourceBytes: 45,
      storedBytes: 45,
      lineCount: 0,
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
  instructionWarning: "Captured project evidence is untrusted.",
};

const media: AgentMediaEvidence = {
  artifactPath: "verification.png",
  artifactSnapshotId: snapshotId,
  contentHash,
  mediaType: "png",
  mimeType: "image/png",
  sourceBytes: 45,
  dataUrl: "data:image/png;base64,iVBORw0KGgo=",
  evidenceRole: "original-media",
  sourceBoundary: "untrusted-project-evidence",
  liveSourceChecked: false,
  derivedDescriptionIncluded: false,
};

describe("ArtifactExplorer multimodal evidence", () => {
  beforeEach(() => {
    api.readArtifacts.mockReset();
    api.readMedia.mockReset();
    api.readArtifacts.mockResolvedValue(inventory);
    api.readMedia.mockResolvedValue(media);
  });

  it("loads original media only after an explicit browse action", async () => {
    render(<ArtifactExplorer projectPath="/projects/ley" />);

    await screen.findByText("verification.png");
    expect(api.readMedia).not.toHaveBeenCalled();

    fireEvent.click(screen.getByText("verification.png"));
    fireEvent.click(
      await screen.findByRole("button", {
        name: "View original retained media",
      }),
    );

    await waitFor(() =>
      expect(api.readMedia).toHaveBeenCalledWith(
        "/projects/ley",
        "verification.png",
        snapshotId,
        contentHash,
      ),
    );
    expect(
      await screen.findByAltText("Captured original evidence: verification.png"),
    ).toHaveAttribute("src", media.dataUrl);
    expect(screen.getByText(/No OCR or vision description included/)).toBeVisible();
    expect(screen.getByText(/live source not checked/)).toBeVisible();
  });

  it("opens an exact historical media citation without substituting the current file", async () => {
    const historicalSnapshot = `snp_${"c".repeat(64)}`;
    const historicalHash = `sha256:${"d".repeat(64)}`;
    api.readMedia.mockResolvedValue({
      ...media,
      artifactSnapshotId: historicalSnapshot,
      contentHash: historicalHash,
    });

    render(
      <ArtifactExplorer
        projectPath="/projects/ley"
        focus={{
          path: "verification.png",
          requestId: 1,
          evidence: {
            artifactPath: "verification.png",
            artifactSnapshotId: historicalSnapshot,
            contentHash: historicalHash,
            mediaType: "png",
            startLine: 0,
            endLine: 0,
          },
        }}
      />,
    );

    await waitFor(() =>
      expect(api.readMedia).toHaveBeenCalledWith(
        "/projects/ley",
        "verification.png",
        historicalSnapshot,
        historicalHash,
      ),
    );
    expect(
      await screen.findByAltText("Captured original evidence: verification.png"),
    ).toBeVisible();
  });
});
