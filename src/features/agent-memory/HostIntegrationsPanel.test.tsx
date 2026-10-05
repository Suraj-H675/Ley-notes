import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { HostIntegrationsPanel } from "./HostIntegrationsPanel";

const api = vi.hoisted(() => ({
  connectAgentHost: vi.fn(),
  inspectAgentProject: vi.fn(),
  readAgentHostIntegrations: vi.fn(),
}));

vi.mock("./api", () => api);

const detectedCodex = {
  id: "codex" as const,
  displayName: "Codex",
  detected: true,
  executablePath: "/usr/bin/codex",
  version: "codex-cli test",
  configured: false,
  enabled: false,
  managedByLeyDesktop: false,
  restartRequired: false,
  reviewRequired: false,
  statusDetail: "Codex is available.",
};

beforeEach(() => {
  vi.clearAllMocks();
  api.readAgentHostIntegrations.mockResolvedValue([detectedCodex]);
});

it("connects only after the user acts and keeps trust/runtime status separate", async () => {
  api.connectAgentHost.mockResolvedValue({
    ...detectedCodex,
    configured: true,
    enabled: true,
    managedByLeyDesktop: true,
    restartRequired: true,
    reviewRequired: true,
    statusDetail: "Ley is configured for this project.",
  });
  api.inspectAgentProject.mockResolvedValue({
    status: "ready",
    dashboard: {
      sessions: [
        {
          sourceKind: "host-hook",
          sourceHost: "codex",
        },
      ],
    },
  });

  render(<HostIntegrationsPanel projectPath="/projects/ley" />);
  expect(await screen.findByText("Codex is available.")).toBeVisible();
  expect(api.connectAgentHost).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Connect Codex" }));
  await waitFor(() =>
    expect(api.connectAgentHost).toHaveBeenCalledWith("/projects/ley", "codex"),
  );
  expect(await screen.findByText("Restart required")).toBeVisible();
  expect(screen.getByText("Review hooks")).toBeVisible();
  expect(api.inspectAgentProject).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Run smoke check" }));
  expect(
    await screen.findByText(/Smoke check passed: Ley retained a Codex hook event/i),
  ).toBeVisible();
  expect(api.inspectAgentProject).toHaveBeenCalledWith("/projects/ley");
});

it("does not call missing retained activity a successful smoke check", async () => {
  api.readAgentHostIntegrations.mockResolvedValue([
    {
      ...detectedCodex,
      configured: true,
      enabled: true,
      managedByLeyDesktop: true,
      reviewRequired: true,
    },
  ]);
  api.inspectAgentProject.mockResolvedValue({
    status: "ready",
    dashboard: { sessions: [] },
  });

  render(<HostIntegrationsPanel projectPath="/projects/ley" />);
  fireEvent.click(await screen.findByRole("button", { name: "Run smoke check" }));
  expect(
    await screen.findByText(/No Codex hook activity is retained yet/i),
  ).toBeVisible();
  expect(screen.queryByText(/Smoke check passed/i)).not.toBeInTheDocument();
});
