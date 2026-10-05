import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { UpdateControl } from "./UpdateControl";

const mocks = vi.hoisted(() => ({
  check: vi.fn(),
  getVersion: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-updater", () => ({ check: mocks.check }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: mocks.getVersion }));

beforeEach(() => {
  vi.clearAllMocks();
  mocks.getVersion.mockResolvedValue("0.1.0");
});

it("does not contact the updater until the user explicitly checks", async () => {
  const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
  mocks.check.mockResolvedValue({ version: "0.2.0", downloadAndInstall });

  render(<UpdateControl showVersion />);
  expect(await screen.findByText("Ley Desktop 0.1.0")).toBeVisible();
  expect(mocks.check).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
  expect(
    await screen.findByRole("button", { name: "Install 0.2.0" }),
  ).toBeVisible();
  expect(mocks.check).toHaveBeenCalledTimes(1);
  expect(downloadAndInstall).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Install 0.2.0" }));
  await waitFor(() => expect(downloadAndInstall).toHaveBeenCalledTimes(1));
  expect(await screen.findByText(/0.2.0 is installed/i)).toBeVisible();
});

it("reports the current release without fabricating an available update", async () => {
  mocks.check.mockResolvedValue(null);
  render(<UpdateControl />);
  fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
  expect(
    await screen.findByRole("button", { name: "Up to date" }),
  ).toBeVisible();
});
