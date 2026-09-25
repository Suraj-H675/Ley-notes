import { beforeEach, describe, expect, it, vi } from "vitest";
import { listCanvases } from "@/core/vault/canvas";
import { listPages } from "@/core/vault/pages";
import { resetDb } from "@/test/helpers";
import {
  linkSessionToCanvas,
  type SessionCanvasLinkRequest,
} from "./link-session-canvas";

const canvasFiles = vi.hoisted(
  () => new Map<string, { content: string; updatedAt: number }>(),
);

vi.mock("@/infrastructure/vault/filesystem-vault", async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/infrastructure/vault/filesystem-vault')>()),
  writeActiveVaultFile: vi.fn(async () => undefined),
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
  beforeEach(async () => {
    canvasFiles.clear();
    await resetDb();
  });

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
    canvasFiles.set("canvases/damaged.canvas", {
      content: "{not json",
      updatedAt: 1,
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
    expect(canvasFiles.get("canvases/damaged.canvas")).toEqual({
      content: "{not json",
      updatedAt: 1,
    });
  });
});
