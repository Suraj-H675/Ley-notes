import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createCanvas } from "@/core/vault/canvas";
import { resetDb } from "@/test/helpers";
import { CanvasModal } from "./CanvasModal";

const canvasFiles = vi.hoisted(
  () => new Map<string, { content: string; updatedAt: number }>(),
);

vi.mock("@/infrastructure/vault/filesystem-vault", async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/infrastructure/vault/filesystem-vault')>()),
  listActiveCanvasFiles: vi.fn(async () =>
    [...canvasFiles.entries()].map(([path, value]) => ({ path, ...value })),
  ),
  writeActiveCanvasFile: vi.fn(async (path: string, content: string) => {
    canvasFiles.set(path, { content, updatedAt: Date.now() });
    return true;
  }),
  trashActiveCanvasFile: vi.fn(async (path: string) => {
    canvasFiles.delete(path);
    return true;
  }),
}));

vi.mock("@/features/notes/usePages", () => ({
  usePages: () => [],
}));

describe("CanvasModal destination", () => {
  beforeEach(async () => {
    canvasFiles.clear();
    await resetDb();
  });

  it("opens the exact Canvas requested by an originating workflow", async () => {
    await createCanvas("First map");
    const target = await createCanvas("Session map");

    render(<CanvasModal open initialPath={target.path} onClose={vi.fn()} />);

    await waitFor(() =>
      expect(screen.getByRole("button", { name: "session-map" })).toHaveClass(
        "bg-surface-3",
      ),
    );
  });
});
