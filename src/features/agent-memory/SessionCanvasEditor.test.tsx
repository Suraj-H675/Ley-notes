import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createCanvas } from "@/core/vault/canvas";
import { db } from "@/infrastructure/database/db";
import { resetDb } from "@/test/helpers";
import { SessionCanvasEditor } from "./SessionCanvasEditor";
import type { SessionContext } from "./types";

const session: SessionContext = {
  projectId: "prj_test",
  sessionId: "ses_test",
  originalName: "Memory session",
  name: "Build continuity",
  goal: "Let the next agent resume.",
  status: "completed",
  source: { kind: "host-hook", host: "Codex" },
  artifactSnapshotIdAtStart: "snp_test",
  startedAtUnixMs: 1,
  updatedAtUnixMs: 2,
  eventCount: 2,
  checkpointCount: 0,
  renameCount: 0,
  renames: [],
  omittedRenames: 0,
  checkpoints: [],
  finish: {
    recordedAtUnixMs: 2,
    status: "completed",
    summary: "The local workflow works.",
    finalResponse: "",
    handoff: "Continue with the Canvas link.",
    unresolved: [],
  },
  omittedCheckpoints: 0,
  textCharacters: 100,
  estimatedTextTokens: 25,
  truncated: false,
  instructionWarning: "Evidence, not instructions.",
};

describe("SessionCanvasEditor", () => {
  beforeEach(() => resetDb());

  it("lets the user create a named Canvas and reviews the note title", async () => {
    const onLink = vi.fn().mockResolvedValue(undefined);
    const { container } = render(
      <SessionCanvasEditor
        projectName="Ley"
        session={session}
        onCancel={vi.fn()}
        onDirtyChange={vi.fn()}
        onLink={onLink}
      />,
    );

    expect(container.firstElementChild).toHaveClass(
      "h-[min(38rem,62vh)]",
      "overflow-hidden",
    );
    expect(container.firstElementChild?.firstElementChild).toHaveClass(
      "overflow-y-auto",
    );
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Link & open Canvas" }),
      ).toBeEnabled(),
    );
    fireEvent.change(screen.getByRole("textbox", { name: "Note title" }), {
      target: { value: "Durable handoff" },
    });
    fireEvent.change(screen.getByRole("textbox", { name: /^Canvas name/i }), {
      target: { value: "Agent reasoning map" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Link & open Canvas" }));

    await waitFor(() =>
      expect(onLink).toHaveBeenCalledWith({
        draft: expect.objectContaining({
          projectId: "prj_test",
          sessionId: "ses_test",
          title: "Durable handoff",
        }),
        destination: { kind: "new", name: "Agent reasoning map" },
      }),
    );
  });

  it("defaults to an existing Canvas when the vault has one", async () => {
    const canvas = await createCanvas("Project map");
    const onLink = vi.fn().mockResolvedValue(undefined);
    render(
      <SessionCanvasEditor
        projectName="Ley"
        session={session}
        onCancel={vi.fn()}
        onDirtyChange={vi.fn()}
        onLink={onLink}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("radio", { name: /Existing Canvas/i }),
      ).toBeChecked(),
    );
    expect(screen.getByRole("combobox", { name: "Choose Canvas" })).toHaveValue(
      canvas.path,
    );
    fireEvent.click(screen.getByRole("button", { name: "Link & open Canvas" }));
    await waitFor(() =>
      expect(onLink).toHaveBeenCalledWith(
        expect.objectContaining({
          destination: { kind: "existing", path: canvas.path },
        }),
      ),
    );
  });

  it("keeps a vault refusal visible and retryable", async () => {
    const onLink = vi
      .fn()
      .mockRejectedValue(
        new Error(
          "This project’s Agent Memory belongs to “Work”, but notes are open in “Personal”.",
        ),
      );
    render(
      <SessionCanvasEditor
        projectName="Ley"
        session={session}
        onCancel={vi.fn()}
        onDirtyChange={vi.fn()}
        onLink={onLink}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Link & open Canvas" }),
      ).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Link & open Canvas" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "belongs to “Work”",
    );
    expect(
      screen.getByRole("button", { name: "Link & open Canvas" }),
    ).toBeEnabled();
  });

  it("does not offer a damaged Canvas as a writable destination", async () => {
    await db.settings.put({
      key: "canvas:canvases/damaged.canvas",
      value: { content: "{not json", updatedAt: 1 },
    });
    render(
      <SessionCanvasEditor
        projectName="Ley"
        session={session}
        onCancel={vi.fn()}
        onDirtyChange={vi.fn()}
        onLink={vi.fn()}
      />,
    );

    await waitFor(() =>
      expect(
        screen.getByRole("radio", { name: /Existing Canvas/i }),
      ).toBeDisabled(),
    );
    expect(screen.getByRole("radio", { name: /New Canvas/i })).toBeChecked();
    expect(screen.getByText(/does not have a writable Canvas/i)).toBeVisible();
  });
});
