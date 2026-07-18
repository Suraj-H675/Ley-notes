import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createCanvas } from "@/core/vault/canvas";
import { resetDb } from "@/test/helpers";
import { CanvasModal } from "./CanvasModal";

vi.mock("@/features/notes/usePages", () => ({
  usePages: () => [],
}));

describe("CanvasModal destination", () => {
  beforeEach(() => resetDb());

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
