import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SessionPromotionEditor } from "./SessionPromotionEditor";
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

describe("SessionPromotionEditor", () => {
  it("previews and submits an explicit user-owned handoff note", async () => {
    const onPromote = vi.fn().mockResolvedValue(undefined);
    const onDirtyChange = vi.fn();
    render(
      <SessionPromotionEditor
        projectName="Ley"
        session={session}
        onCancel={vi.fn()}
        onDirtyChange={onDirtyChange}
        onPromote={onPromote}
      />,
    );

    expect(screen.getByText("Continue with the Canvas link.")).toBeVisible();
    fireEvent.change(screen.getByRole("textbox", { name: "Note title" }), {
      target: { value: "Agent handoff" },
    });
    expect(onDirtyChange).toHaveBeenCalledWith(true);
    fireEvent.click(screen.getByRole("button", { name: "Create & open note" }));

    await waitFor(() =>
      expect(onPromote).toHaveBeenCalledWith(
        expect.objectContaining({
          sessionId: "ses_test",
          projectId: "prj_test",
          title: "Agent handoff",
          folder: "Agent Memory/Sessions",
          content: expect.stringContaining("## Handoff"),
          frontmatter: expect.objectContaining({
            "ley-source": "agent-memory",
            "ley-session-id": "ses_test",
          }),
        }),
      ),
    );
  });

  it("keeps a cross-vault refusal visible and retryable", async () => {
    const onPromote = vi
      .fn()
      .mockRejectedValue(
        "This project’s Agent Memory belongs to “Work”, but notes are open in “Personal”. Open the bound vault before creating a linked note.",
      );
    render(
      <SessionPromotionEditor
        projectName="Ley"
        session={session}
        onCancel={vi.fn()}
        onDirtyChange={vi.fn()}
        onPromote={onPromote}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Create & open note" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "belongs to “Work”",
    );
    expect(
      screen.getByRole("button", { name: "Create & open note" }),
    ).toBeEnabled();
  });
});
