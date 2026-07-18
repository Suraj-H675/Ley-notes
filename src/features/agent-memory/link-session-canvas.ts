import {
  addFileToCanvas,
  createCanvas,
  listCanvases,
  type CanvasSummary,
} from "@/core/vault/canvas";
import type { Page } from "@/infrastructure/database/schema";
import { promoteSessionNote } from "./promote-session-note";
import type { PromotedSessionNoteDraft } from "./types";

export type SessionCanvasDestination =
  { kind: "existing"; path: string } | { kind: "new"; name: string };

export interface SessionCanvasLinkRequest {
  draft: PromotedSessionNoteDraft;
  destination: SessionCanvasDestination;
}

export interface SessionCanvasLinkResult {
  page: Page;
  canvas: CanvasSummary;
  noteCreated: boolean;
  cardAdded: boolean;
}

export async function linkSessionToCanvas({
  draft,
  destination,
}: SessionCanvasLinkRequest): Promise<SessionCanvasLinkResult> {
  const destinationCanvas =
    destination.kind === "existing"
      ? await requireExistingCanvas(destination.path)
      : await createCanvas(destination.name);
  const { page, created: noteCreated } = await promoteSessionNote(draft);
  const { canvas, added: cardAdded } = await addFileToCanvas(
    destinationCanvas.path,
    page.path,
  );
  return { page, canvas, noteCreated, cardAdded };
}

async function requireExistingCanvas(path: string): Promise<CanvasSummary> {
  const canvas = (await listCanvases()).find(
    (candidate) => candidate.path === path,
  );
  if (!canvas) {
    throw new Error(
      "That Canvas is no longer available. Choose another Canvas and try again.",
    );
  }
  if (canvas.readError) {
    throw new Error(
      `“${canvas.name}” is not valid JSON Canvas. Ley left it unchanged; repair it or choose another Canvas.`,
    );
  }
  return canvas;
}
