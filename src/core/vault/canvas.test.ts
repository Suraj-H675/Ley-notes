import { beforeEach, describe, expect, it } from "vitest";
import { db } from "@/infrastructure/database/db";
import { resetDb } from "@/test/helpers";
import {
  addFileToCanvas,
  createCanvas,
  deleteCanvas,
  listCanvases,
  newGroupCanvasNode,
  newLinkCanvasNode,
  newTextCanvasNode,
  canvasBackgroundPresentation,
  normalizeCanvas,
  saveCanvas,
} from "./canvas";

describe("JSON Canvas persistence", () => {
  beforeEach(async () => resetDb());

  it("creates and updates an interoperable browser-local canvas document", async () => {
    const canvas = await createCanvas("Research map");
    expect(canvas.path).toBe("canvases/research-map.canvas");

    const node = newTextCanvasNode({ x: 40, y: 80 });
    node.text = "Question → evidence → conclusion";
    await saveCanvas(canvas.path, { nodes: [node], edges: [] });

    const [restored] = await listCanvases();
    expect(restored.document.nodes).toEqual([node]);
    expect(restored.document.edges).toEqual([]);

    await deleteCanvas(canvas.path);
    expect(await listCanvases()).toEqual([]);
  });

  it("adds a standard file node once and reuses it on retries", async () => {
    const canvas = await createCanvas("Agent map");
    const first = await addFileToCanvas(
      canvas.path,
      "Agent Memory/Sessions/Handoff.md",
    );
    const repeated = await addFileToCanvas(
      canvas.path,
      "Agent Memory/Sessions/Handoff.md",
    );

    expect(first.added).toBe(true);
    expect(repeated.added).toBe(false);
    expect(repeated.canvas.document.nodes).toEqual([
      expect.objectContaining({
        type: "file",
        file: "Agent Memory/Sessions/Handoff.md",
        x: 80,
        y: 80,
        width: 280,
        height: 120,
      }),
    ]);
    expect((await listCanvases())[0].document.nodes).toHaveLength(1);
  });

  it("refuses to recreate a Canvas that disappeared before the write", async () => {
    await expect(
      addFileToCanvas(
        "canvases/missing.canvas",
        "Agent Memory/Sessions/Handoff.md",
      ),
    ).rejects.toThrow("no longer available");
    expect(await listCanvases()).toEqual([]);
  });

  it("never overwrites malformed Canvas content through a file-card shortcut", async () => {
    await db.settings.put({
      key: "canvas:canvases/damaged.canvas",
      value: { content: "{not json", updatedAt: 1 },
    });

    await expect(
      addFileToCanvas(
        "canvases/damaged.canvas",
        "Agent Memory/Sessions/Handoff.md",
      ),
    ).rejects.toThrow("not valid JSON Canvas");
    await expect(createCanvas("Damaged")).rejects.toThrow(
      "not valid JSON Canvas",
    );
    expect(
      (await db.settings.get("canvas:canvases/damaged.canvas"))?.value,
    ).toEqual({ content: "{not json", updatedAt: 1 });
  });

  it("round-trips every JSON Canvas 1.0 node and edge field Ley supports", () => {
    const document = {
      nodes: [
        {
          id: "group",
          type: "group",
          label: "Evidence",
          x: 0,
          y: 0,
          width: 800,
          height: 500,
          color: "4",
          background: "attachments/grid.png",
          backgroundStyle: "repeat",
        },
        {
          id: "text",
          type: "text",
          text: "# Claim",
          x: 40,
          y: 80,
          width: 280,
          height: 180,
          color: "#334455",
        },
        {
          id: "file",
          type: "file",
          file: "Research/Study.md",
          subpath: "#Results",
          x: 360,
          y: 80,
          width: 280,
          height: 180,
        },
        {
          id: "link",
          type: "link",
          url: "https://jsoncanvas.org",
          x: 200,
          y: 300,
          width: 320,
          height: 120,
          color: "5",
        },
      ],
      edges: [
        {
          id: "edge",
          fromNode: "text",
          fromSide: "right",
          fromEnd: "none",
          toNode: "file",
          toSide: "left",
          toEnd: "arrow",
          label: "supports",
          color: "6",
        },
      ],
    } as const;

    expect(normalizeCanvas(document)).toEqual(document);
  });

  it("keeps only safe image attachments as group backgrounds", async () => {
    const canvas = await createCanvas("Visual map");
    const group = newGroupCanvasNode({ x: 0, y: 0 });
    group.background = "attachments/photo.png";
    group.backgroundStyle = "ratio";
    const unsafe = newGroupCanvasNode({ x: 0, y: 0 });
    unsafe.id = "unsafe-group";
    unsafe.background = "../../../etc/passwd.png";

    await saveCanvas(canvas.path, {
      nodes: [group, unsafe],
      edges: [],
    });

    const restored = (await listCanvases())[0].document.nodes;
    expect(
      canvasBackgroundPresentation(group.background, group.backgroundStyle),
    ).toEqual({
      backgroundImage: 'url("attachments/photo.png")',
      backgroundPosition: "center",
      backgroundRepeat: "no-repeat",
      backgroundSize: "contain",
    });
    expect(restored[0]).toMatchObject({
      id: group.id,
      background: "attachments/photo.png",
      backgroundStyle: "ratio",
    });
    expect(restored[1]).toMatchObject({
      id: "unsafe-group",
    });
    const restoredUnsafe = restored[1];
    expect(restoredUnsafe.type).toBe("group");
    if (restoredUnsafe.type === "group") {
      expect(restoredUnsafe.background).toBeUndefined();
      expect(restoredUnsafe.backgroundStyle).toBeUndefined();
    }
  });

  it("rejects malformed geometry and dangling edges without discarding valid content", () => {
    expect(
      normalizeCanvas({
        nodes: [
          {
            id: "valid",
            type: "text",
            text: "Keep me",
            color: "url(https://tracker.invalid/pixel)",
            x: 0,
            y: 0,
            width: 200,
            height: 100,
          },
          {
            id: "bad",
            type: "text",
            text: "No size",
            x: 0,
            y: 0,
            width: 0,
            height: 100,
          },
          { id: "future", type: "video", x: 0, y: 0, width: 200, height: 100 },
        ],
        edges: [{ id: "dangling", fromNode: "valid", toNode: "bad" }],
      }),
    ).toEqual({
      nodes: [
        {
          id: "valid",
          type: "text",
          text: "Keep me",
          x: 0,
          y: 0,
          width: 200,
          height: 100,
        },
      ],
      edges: [],
    });
  });

  it("creates useful interoperable defaults for spatial groups and web references", () => {
    expect(newGroupCanvasNode({ x: 40, y: 40 })).toMatchObject({
      type: "group",
      label: "New group",
      x: 40,
      y: 40,
      width: 1040,
      height: 480,
    });
    expect(
      newLinkCanvasNode("https://jsoncanvas.org", { x: 80, y: 300 }),
    ).toMatchObject({
      type: "link",
      url: "https://jsoncanvas.org",
      width: 320,
      height: 120,
    });
  });
});
