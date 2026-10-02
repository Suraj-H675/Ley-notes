import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LegacyRecoveryPage } from "./LegacyRecoveryPage";

const recovery = vi.hoisted(() => ({
  inspect: vi.fn(),
  build: vi.fn(),
  download: vi.fn(),
  erase: vi.fn(),
}));

vi.mock("./legacy-browser-recovery", () => ({
  inspectLegacyBrowserData: recovery.inspect,
  buildLegacyBrowserArchive: recovery.build,
  downloadLegacyBrowserArchive: recovery.download,
  eraseLegacyBrowserData: recovery.erase,
}));

describe("LegacyRecoveryPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    recovery.inspect.mockResolvedValue({
      status: "recoverable",
      databaseVersion: 2,
      activeDataKind: "browser-local",
      source: "active-browser-local",
      pages: 3,
      assets: 2,
      revisions: 4,
      assetBytes: 2_048,
      ambiguousActiveBackup: false,
    });
  });

  it("does not inspect IndexedDB until the user explicitly asks", async () => {
    render(<LegacyRecoveryPage />);
    expect(recovery.inspect).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Inspect legacy browser data" }));
    await waitFor(() => expect(recovery.inspect).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("3")).toBeVisible();
    expect(screen.getByText("2")).toBeVisible();
    expect(screen.getByText("4")).toBeVisible();
  });

  it("exports only after inspection and explicit user action", async () => {
    const archive = { exportedAt: "2026-10-02T00:00:00.000Z" };
    recovery.build.mockResolvedValue(archive);
    recovery.download.mockReturnValue("ley-legacy-browser-data.json");
    render(<LegacyRecoveryPage />);

    fireEvent.click(screen.getByRole("button", { name: "Inspect legacy browser data" }));
    fireEvent.click(await screen.findByRole("button", { name: "Export lossless JSON archive" }));

    await waitFor(() => expect(recovery.build).toHaveBeenCalledTimes(1));
    expect(recovery.download).toHaveBeenCalledWith(archive);
    expect(await screen.findByText(/Downloaded ley-legacy-browser-data\.json/)).toBeVisible();
  });

  it("requires exact destructive confirmation before erasure", async () => {
    recovery.erase.mockResolvedValue(undefined);
    recovery.inspect
      .mockResolvedValueOnce({
        status: "empty",
        databaseVersion: 2,
        activeDataKind: null,
        pages: 0,
        assets: 0,
        revisions: 0,
        assetBytes: 0,
        ambiguousActiveBackup: false,
      })
      .mockResolvedValueOnce({
        status: "absent",
        pages: 0,
        assets: 0,
        revisions: 0,
        assetBytes: 0,
        ambiguousActiveBackup: false,
      });
    render(<LegacyRecoveryPage />);

    fireEvent.click(screen.getByRole("button", { name: "Inspect legacy browser data" }));
    const eraseButton = await screen.findByRole("button", {
      name: "Permanently erase legacy data",
    });
    expect(eraseButton).toBeDisabled();
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "ERASE LEGACY DATA" },
    });
    expect(eraseButton).toBeEnabled();
    fireEvent.click(eraseButton);

    await waitFor(() => expect(recovery.erase).toHaveBeenCalledTimes(1));
    expect(await screen.findByText(/erased from this site origin/i)).toBeVisible();
  });
});
