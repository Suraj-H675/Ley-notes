import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CapturePrivacyPanel } from "./CapturePrivacyPanel";
import type {
  AgentCaptureSettings,
  AgentMemoryDashboard,
  AgentProjectInspection,
  ProjectAgentEgressPolicy,
} from "./types";

const api = vi.hoisted(() => ({
  chooseExportParent: vi.fn(),
  erase: vi.fn(),
  exportContinuity: vi.fn(),
  readEgress: vi.fn(),
  readSettings: vi.fn(),
  updateEgress: vi.fn(),
  updateMode: vi.fn(),
}));

vi.mock("./api", () => ({
  chooseAgentContinuityExportParent: api.chooseExportParent,
  eraseAgentProjectMemory: api.erase,
  exportAgentProjectContinuity: api.exportContinuity,
  readAgentEgressPolicy: api.readEgress,
  readAgentCaptureSettings: api.readSettings,
  updateAgentEgressPolicy: api.updateEgress,
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
  storage: {
    kind: "legacy-vault",
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

const egressPolicy: ProjectAgentEgressPolicy = {
  projectId: "prj_test",
  projectPolicy: "agent-ok",
  specificationOverrides: [],
  mountOverrides: [],
  connectorOverrides: [],
  privacyNotice: "OS-private sharing authority.",
};

const erasedInspection: AgentProjectInspection = {
  status: "needs-capture",
  projectId: "prj_test",
  projectName: "Ley",
  captureMode: "structured",
  storage: dashboard.storage,
};

describe("CapturePrivacyPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.chooseExportParent.mockResolvedValue(null);
    api.readSettings.mockResolvedValue(settings);
    api.readEgress.mockResolvedValue(egressPolicy);
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
      screen.getByText(/ordinary Markdown notes, Canvas documents/i),
    ).toBeVisible();
    expect(
      screen.getByText(/user-owned note or Canvas copy of erased Agent Memory remains/i),
    ).toBeVisible();
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

  it("requires explicit Full Evidence consent without granting transcript capture", async () => {
    const onUpdated = vi.fn();
    api.updateMode.mockResolvedValue(dashboard);
    api.readSettings
      .mockResolvedValueOnce(settings)
      .mockResolvedValueOnce({ ...settings, mode: "full-evidence" });

    render(
      <CapturePrivacyPanel
        projectPath="/projects/ley"
        dashboard={dashboard}
        onUpdated={onUpdated}
        onErased={vi.fn()}
      />,
    );

    fireEvent.click(
      await screen.findByRole("radio", { name: /Full Evidence/i }),
    );
    expect(
      screen.getByText(/does not authorize raw host transcript collection/i),
    ).toBeVisible();
    expect(
      screen.getByText(/host integrations may still add small Ley session\/checkpoint guidance/i),
    ).toBeVisible();

    const apply = screen.getByRole("button", { name: "Apply & recapture" });
    expect(apply).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox"));
    expect(apply).toBeEnabled();
    fireEvent.click(apply);

    await waitFor(() => {
      expect(api.updateMode).toHaveBeenCalledWith(
        "/projects/ley",
        "structured",
        "full-evidence",
        true,
      );
      expect(onUpdated).toHaveBeenCalledWith(dashboard);
    });
  });

  it("exports a portable continuity bundle only after choosing a destination", async () => {
    api.chooseExportParent.mockResolvedValue("/exports");
    api.exportContinuity.mockResolvedValue({
      projectId: "prj_test",
      destination: "/exports/ley-continuity-prj_test-1-2",
      eventCount: 7,
      artifactSnapshots: 2,
      evidenceBlobs: 3,
    });

    render(
      <CapturePrivacyPanel
        projectPath="/projects/ley"
        dashboard={dashboard}
        onUpdated={vi.fn()}
        onErased={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Export bundle…" }));
    await waitFor(() => {
      expect(api.exportContinuity).toHaveBeenCalledWith("/projects/ley", "/exports");
    });
    expect(await screen.findByText("Export complete")).toBeVisible();
    expect(screen.getByText(/7 events/)).toBeVisible();
    expect(screen.getByText(/2 cited snapshots/)).toBeVisible();
    expect(screen.getByText(/3 evidence blobs/)).toBeVisible();
  });

  it("does not export when the destination chooser is cancelled", async () => {
    render(
      <CapturePrivacyPanel
        projectPath="/projects/ley"
        dashboard={dashboard}
        onUpdated={vi.fn()}
        onErased={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Export bundle…" }));
    await waitFor(() => {
      expect(api.chooseExportParent).toHaveBeenCalledTimes(1);
    });
    expect(api.exportContinuity).not.toHaveBeenCalled();
  });

  it("updates project agent sharing while keeping confirm-per-use fail-closed", async () => {
    api.readEgress.mockResolvedValueOnce({
      ...egressPolicy,
      projectPolicy: "confirm-per-use",
      specificationOverrides: [
        {
          scopeKind: "specification",
          scopeId: "spec_11111111111111111111111111111111",
          policy: "never-send",
        },
      ],
    });
    api.updateEgress.mockResolvedValue({
      ...egressPolicy,
      projectPolicy: "local-model-only",
      specificationOverrides: [
        {
          scopeKind: "specification",
          scopeId: "spec_11111111111111111111111111111111",
          policy: "never-send",
        },
      ],
    });

    render(
      <CapturePrivacyPanel
        projectPath="/projects/ley"
        dashboard={dashboard}
        onUpdated={vi.fn()}
        onErased={vi.fn()}
      />,
    );

    expect(
      await screen.findByText(/No confirmation prompt exists yet/i),
    ).toBeVisible();
    expect(
      screen.getByText("spec_11111111111111111111111111111111"),
    ).toBeVisible();
    fireEvent.change(screen.getByLabelText("Agent context sharing policy"), {
      target: { value: "local-model-only" },
    });
    expect(
      screen.getByText(
        /does not verify that the downstream provider or runtime is actually local/i,
      ),
    ).toBeVisible();
    fireEvent.click(
      screen.getByRole("button", { name: "Apply sharing policy" }),
    );

    await waitFor(() => {
      expect(api.updateEgress).toHaveBeenCalledWith(
        "/projects/ley",
        "prj_test",
        "confirm-per-use",
        "local-model-only",
      );
    });
  });

  it("refreshes the visible sharing policy after a stale update is rejected", async () => {
    api.readEgress
      .mockResolvedValueOnce(egressPolicy)
      .mockResolvedValueOnce({
        ...egressPolicy,
        projectPolicy: "never-send",
      });
    api.updateEgress.mockRejectedValue(
      new Error(
        "Ley agent egress policy changed from expected 'agent-ok' to 'never-send'.",
      ),
    );

    render(
      <CapturePrivacyPanel
        projectPath="/projects/ley"
        dashboard={dashboard}
        onUpdated={vi.fn()}
        onErased={vi.fn()}
      />,
    );

    fireEvent.change(
      await screen.findByLabelText("Agent context sharing policy"),
      {
        target: { value: "local-model-only" },
      },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Apply sharing policy" }),
    );

    expect(
      await screen.findByText(
        /changed from expected 'agent-ok' to 'never-send'/i,
      ),
    ).toBeVisible();
    await waitFor(() => {
      expect(
        screen.getByLabelText("Agent context sharing policy"),
      ).toHaveValue("never-send");
    });
    expect(api.readEgress).toHaveBeenCalledTimes(2);
  });

  it("clears the policy view if a failed update cannot be refreshed safely", async () => {
    api.readEgress
      .mockResolvedValueOnce(egressPolicy)
      .mockRejectedValueOnce(new Error("egress authority unavailable"));
    api.updateEgress.mockRejectedValue(new Error("transition interrupted"));

    render(
      <CapturePrivacyPanel
        projectPath="/projects/ley"
        dashboard={dashboard}
        onUpdated={vi.fn()}
        onErased={vi.fn()}
      />,
    );

    fireEvent.change(
      await screen.findByLabelText("Agent context sharing policy"),
      { target: { value: "never-send" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Apply sharing policy" }),
    );

    expect(await screen.findByText(/transition interrupted/i)).toBeVisible();
    expect(
      await screen.findByText("Agent sharing policy is unavailable."),
    ).toBeVisible();
    expect(
      screen.queryByLabelText("Agent context sharing policy"),
    ).not.toBeInTheDocument();
  });
});
