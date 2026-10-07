import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { ChronicleCaptureControl } from "./ChronicleCaptureControl";

const api = vi.hoisted(() => ({
  readChronicleCaptureTargets: vi.fn(),
  requestChronicleCapture: vi.fn(),
  revokeChronicleCapture: vi.fn(),
}));

vi.mock("./chronicle-api", () => api);

const disabled = {
  projectId: "prj_test",
  locatorId: "wcp_test",
  localPath: "/projects/ley",
  mode: null,
  grantId: null,
  authorized: false,
  effective: false,
  inactiveReason: null,
};

beforeEach(() => {
  vi.clearAllMocks();
  api.readChronicleCaptureTargets.mockResolvedValue([disabled]);
});

it("reads only the selected project and requires an explicit enable action", async () => {
  api.requestChronicleCapture.mockResolvedValue(true);
  api.readChronicleCaptureTargets
    .mockResolvedValueOnce([disabled])
    .mockResolvedValueOnce([
      {
        ...disabled,
        mode: "structured",
        grantId: "cgr_test",
        authorized: true,
        effective: true,
      },
    ]);

  render(<ChronicleCaptureControl projectId="prj_test" />);

  expect(await screen.findByText("/projects/ley")).toBeVisible();
  expect(api.readChronicleCaptureTargets).toHaveBeenCalledWith("prj_test");
  expect(api.requestChronicleCapture).not.toHaveBeenCalled();
  expect(
    screen.getByText(
      /Connecting Codex or importing project files does not grant capture permission/i,
    ),
  ).toBeVisible();

  fireEvent.click(screen.getByRole("button", { name: /Review and enable/i }));
  await waitFor(() =>
    expect(api.requestChronicleCapture).toHaveBeenCalledWith(
      disabled,
      "structured",
    ),
  );
  expect(
    await screen.findByText(
      /Codex capture is enabled with structured retention/i,
    ),
  ).toBeVisible();
});

it("shows an ineffective retained grant without calling it active and can revoke it", async () => {
  const inactive = {
    ...disabled,
    mode: "structured",
    grantId: "cgr_moved",
    authorized: true,
    effective: false,
    inactiveReason: "working-copy-moved",
  };
  api.readChronicleCaptureTargets
    .mockResolvedValueOnce([inactive])
    .mockResolvedValueOnce([{ ...disabled }]);
  api.revokeChronicleCapture.mockResolvedValue(undefined);

  render(<ChronicleCaptureControl projectId="prj_test" />);

  expect(
    await screen.findByText(
      /retained permission exists, but it is not currently effective/i,
    ),
  ).toBeVisible();
  expect(screen.getByText(/working copy moved/i)).toBeVisible();
  expect(screen.queryByText(/capture is enabled/i)).not.toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Stop capture" }));
  await waitFor(() =>
    expect(api.revokeChronicleCapture).toHaveBeenCalledWith(inactive),
  );
});

it("keeps an existing working copy's retention mode unless the user changes it", async () => {
  const fullEvidence = {
    ...disabled,
    mode: "full-evidence",
    grantId: "cgr_full",
    authorized: true,
    effective: true,
  };
  api.readChronicleCaptureTargets
    .mockResolvedValueOnce([fullEvidence])
    .mockResolvedValueOnce([fullEvidence]);
  api.requestChronicleCapture.mockResolvedValue(true);

  render(<ChronicleCaptureControl projectId="prj_test" />);

  expect(await screen.findByDisplayValue("Full evidence")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Review permission" }));
  await waitFor(() =>
    expect(api.requestChronicleCapture).toHaveBeenCalledWith(
      fullEvidence,
      "full-evidence",
    ),
  );
});
