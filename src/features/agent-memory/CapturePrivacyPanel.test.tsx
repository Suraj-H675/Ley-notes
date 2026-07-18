import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CapturePrivacyPanel } from "./CapturePrivacyPanel";
import type {
  AgentCaptureSettings,
  AgentMemoryDashboard,
  AgentProjectInspection,
} from "./types";

const api = vi.hoisted(() => ({
  erase: vi.fn(),
  readSettings: vi.fn(),
  updateMode: vi.fn(),
}));

vi.mock("./api", () => ({
  eraseAgentProjectMemory: api.erase,
  readAgentCaptureSettings: api.readSettings,
  updateAgentCaptureMode: api.updateMode,
}));

const settings: AgentCaptureSettings = {
  projectId: "prj_test",
  projectName: "Ley",
  mode: "structured",
  approvedRoots: ["."],
  respectGitignore: true,
  maxFileBytes: 1_048_576,
  maxTotalBytes: 536_870_912,
  storeRawTranscripts: false,
  ignoreFilePresent: true,
  captureFingerprint: "sha256:test",
  eligibleFiles: 18,
  eligibleBytes: 32_000,
  skippedOversized: 1,
  skippedTotalLimit: 0,
  skippedSymlinks: 0,
  privacyNotice: "Local vault only.",
};

const dashboard = {
  binding: {
    projectId: "prj_test",
    vaultName: "Private vault",
    source: "persisted",
  },
  overview: {
    projectId: "prj_test",
    projectName: "Ley",
    captureMode: "structured",
    files: 18,
    retainedSourceFiles: 15,
  },
} as AgentMemoryDashboard;

const erasedInspection: AgentProjectInspection = {
  status: "needs-capture",
  projectId: "prj_test",
  projectName: "Ley",
  captureMode: "structured",
  binding: dashboard.binding,
};

describe("CapturePrivacyPanel memory erasure", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.readSettings.mockResolvedValue(settings);
    api.erase.mockResolvedValue(erasedInspection);
  });

  it("requires the exact project name before permanently erasing local memory", async () => {
    const onErased = vi.fn();
    render(
      <CapturePrivacyPanel
        projectPath="/projects/ley"
        dashboard={dashboard}
        onUpdated={vi.fn()}
        onErased={onErased}
      />,
    );

    fireEvent.click(await screen.findByText("Erase memory…"));
    expect(screen.getByText("This cannot be undone by Ley")).toBeVisible();
    expect(
      screen.getByText(/backups, filesystem snapshots, or copies/i),
    ).toBeVisible();

    const eraseButton = screen.getByRole("button", {
      name: "Permanently erase memory",
    });
    expect(eraseButton).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Type Ley to confirm/), {
      target: { value: "ley" },
    });
    expect(eraseButton).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Type Ley to confirm/), {
      target: { value: "Ley" },
    });
    expect(eraseButton).toBeEnabled();
    fireEvent.click(eraseButton);

    await waitFor(() => {
      expect(api.erase).toHaveBeenCalledWith("/projects/ley");
      expect(onErased).toHaveBeenCalledWith(erasedInspection);
    });
  });
});
