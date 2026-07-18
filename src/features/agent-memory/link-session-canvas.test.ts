import { beforeEach, describe, expect, it } from "vitest";
import { listCanvases } from "@/core/vault/canvas";
import { listPages } from "@/core/vault/pages";
import { db } from "@/infrastructure/database/db";
import { resetDb } from "@/test/helpers";
import {
  linkSessionToCanvas,
  type SessionCanvasLinkRequest,
} from "./link-session-canvas";

const request: SessionCanvasLinkRequest = {
  draft: {
    sessionId: "ses_test",
    projectId: "prj_test",
    title: "Session handoff",
    folder: "Agent Memory/Sessions",
    content: "## Handoff\n\nContinue here.\n",
    frontmatter: {
      "ley-source": "agent-memory",
      "ley-project-id": "prj_test",
      "ley-session-id": "ses_test",
    },
  },
  destination: { kind: "new", name: "Ley continuity" },
};

describe("session Canvas linking", () => {
  beforeEach(() => resetDb());

  it("creates an ordinary note and links it with a JSON Canvas file node", async () => {
    const result = await linkSessionToCanvas(request);

    expect(result.noteCreated).toBe(true);
    expect(result.cardAdded).toBe(true);
    expect(result.canvas.path).toBe("canvases/ley-continuity.canvas");
    expect(result.canvas.document.nodes).toEqual([
      expect.objectContaining({
        type: "file",
        file: result.page.path,
      }),
    ]);
  });

  it("reuses both the promoted note and Canvas card on retry", async () => {
    const first = await linkSessionToCanvas(request);
    const repeated = await linkSessionToCanvas({
      ...request,
      draft: { ...request.draft, title: "A different retry title" },
    });

    expect(repeated.noteCreated).toBe(false);
    expect(repeated.cardAdded).toBe(false);
    expect(repeated.page.id).toBe(first.page.id);
    expect(await listPages()).toHaveLength(1);
    expect((await listCanvases())[0].document.nodes).toHaveLength(1);
  });

  it("does not create a note when a chosen Canvas disappeared", async () => {
    await expect(
      linkSessionToCanvas({
        ...request,
        destination: {
          kind: "existing",
          path: "canvases/missing.canvas",
        },
      }),
    ).rejects.toThrow("no longer available");
    expect(await listPages()).toEqual([]);
  });

  it("does not create a note or overwrite a malformed destination", async () => {
    await db.settings.put({
      key: "canvas:canvases/damaged.canvas",
      value: { content: "{not json", updatedAt: 1 },
    });
    await expect(
      linkSessionToCanvas({
        ...request,
        destination: {
          kind: "existing",
          path: "canvases/damaged.canvas",
        },
      }),
    ).rejects.toThrow("not valid JSON Canvas");
    expect(await listPages()).toEqual([]);
    expect(
      (await db.settings.get("canvas:canvases/damaged.canvas"))?.value,
    ).toEqual({ content: "{not json", updatedAt: 1 });
  });
});
