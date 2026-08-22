import { db } from "@/infrastructure/database/db";
import {
  listActiveCanvasFiles,
  trashActiveCanvasFile,
  writeActiveCanvasFile,
} from "@/infrastructure/vault/filesystem-vault";
import { nanoid } from "@/shared/lib/nanoid";
import { slugify } from "@/shared/lib/slug";
import { isImagePath, isSafeAttachmentPath } from "./attachments";

export interface CanvasTextNode {
  id: string;
  type: "text";
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  color?: string;
}

export interface CanvasFileNode {
  id: string;
  type: "file";
  file: string;
  x: number;
  y: number;
  width: number;
  height: number;
  color?: string;
  subpath?: string;
}

export interface CanvasLinkNode {
  id: string;
  type: "link";
  url: string;
  x: number;
  y: number;
  width: number;
  height: number;
  color?: string;
}

export interface CanvasGroupNode {
  id: string;
  type: "group";
  x: number;
  y: number;
  width: number;
  height: number;
  color?: string;
  label?: string;
  background?: string;
  backgroundStyle?: "cover" | "ratio" | "repeat";
}

export type CanvasNode =
  CanvasTextNode | CanvasFileNode | CanvasLinkNode | CanvasGroupNode;

export type CanvasSide = "top" | "right" | "bottom" | "left";
export type CanvasEnd = "none" | "arrow";

const CANVAS_COLORS: Record<string, string> = {
  "1": "#ef4444",
  "2": "#f97316",
  "3": "#eab308",
  "4": "#22c55e",
  "5": "#06b6d4",
  "6": "#a855f7",
};

export function canvasColorValue(color?: unknown): string | undefined {
  if (typeof color !== "string" || !color) return undefined;
  if (CANVAS_COLORS[color]) return CANVAS_COLORS[color];
  return /^#(?:[\da-f]{3}|[\da-f]{4}|[\da-f]{6}|[\da-f]{8})$/i.test(color)
    ? color
    : undefined;
}

export function canvasBackgroundPresentation(
  background?: unknown,
  backgroundStyle?: unknown,
): {
  backgroundImage: string;
  backgroundPosition: string;
  backgroundRepeat: string;
  backgroundSize: string;
} | null {
  if (
    typeof background !== "string" ||
    !isSafeAttachmentPath(background) ||
    !isImagePath(background)
  )
    return null;

  const mode =
    backgroundStyle === "cover" ||
    backgroundStyle === "ratio" ||
    backgroundStyle === "repeat"
      ? backgroundStyle
      : "cover";
  return {
    backgroundImage: `url("${background.replaceAll('"', "%22")}")`,
    backgroundPosition: "center",
    backgroundRepeat: mode === "repeat" ? "repeat" : "no-repeat",
    backgroundSize:
      mode === "cover" ? "cover" : mode === "ratio" ? "contain" : "auto",
  };
}

export interface CanvasEdge {
  id: string;
  fromNode: string;
  toNode: string;
  fromSide?: CanvasSide;
  fromEnd?: CanvasEnd;
  toSide?: CanvasSide;
  toEnd?: CanvasEnd;
  label?: string;
  color?: string;
}

export interface CanvasDocument {
  nodes: CanvasNode[];
  edges: CanvasEdge[];
}

export interface CanvasSummary {
  path: string;
  name: string;
  updatedAt: number;
  document: CanvasDocument;
  readError?: "invalid-json";
}

export async function listCanvases(): Promise<CanvasSummary[]> {
  const filesystem = await listActiveCanvasFiles();
  if (filesystem)
    return filesystem.map((file) =>
      summary(file.path, file.content, file.updatedAt),
    );
  const rows = (await db.settings.toArray()).filter((row) =>
    row.key.startsWith("canvas:"),
  );
  return rows
    .map((row) => {
      const value = row.value as { content?: string; updatedAt?: number };
      return summary(
        row.key.slice("canvas:".length),
        value.content ?? '{"nodes":[],"edges":[]}',
        value.updatedAt ?? 0,
      );
    })
    .sort((left, right) => left.name.localeCompare(right.name));
}

export async function createCanvas(name: string): Promise<CanvasSummary> {
  const cleanName = name.trim() || "Untitled canvas";
  const path = `canvases/${slugify(cleanName)}.canvas`;
  const existing = (await listCanvases()).find(
    (canvas) => canvas.path.toLowerCase() === path.toLowerCase(),
  );
  if (existing) {
    assertCanvasWritable(existing);
    return existing;
  }
  const document: CanvasDocument = { nodes: [], edges: [] };
  await saveCanvas(path, document);
  return { path, name: cleanName, updatedAt: Date.now(), document };
}

export async function saveCanvas(
  path: string,
  document: CanvasDocument,
): Promise<void> {
  const normalized = normalizeCanvas(document);
  const content = JSON.stringify(normalized, null, 2);
  if (!(await writeActiveCanvasFile(path, content))) {
    await db.settings.put({
      key: `canvas:${path}`,
      value: { content, updatedAt: Date.now() },
    });
  }
}

export async function deleteCanvas(path: string): Promise<void> {
  if (!(await trashActiveCanvasFile(path)))
    await db.settings.delete(`canvas:${path}`);
}

export async function addFileToCanvas(
  canvasPath: string,
  filePath: string,
): Promise<{ canvas: CanvasSummary; added: boolean }> {
  const canvases = await listCanvases();
  const canvas = canvases.find((candidate) => candidate.path === canvasPath);
  if (!canvas) {
    throw new Error(
      "That Canvas is no longer available. Choose another Canvas and try again.",
    );
  }
  assertCanvasWritable(canvas);
  const alreadyLinked = canvas.document.nodes.some(
    (node) => node.type === "file" && node.file === filePath,
  );
  if (alreadyLinked) return { canvas, added: false };

  const document: CanvasDocument = {
    ...canvas.document,
    nodes: [
      ...canvas.document.nodes,
      newFileCanvasNode(
        filePath,
        nextCanvasCardPosition(canvasContentNodeCount(canvas.document.nodes)),
      ),
    ],
  };
  await saveCanvas(canvas.path, document);
  return {
    canvas: { ...canvas, document, updatedAt: Date.now() },
    added: true,
  };
}

function assertCanvasWritable(canvas: CanvasSummary): void {
  if (canvas.readError) {
    throw new Error(
      `“${canvas.name}” is not valid JSON Canvas. Ley left it unchanged; repair it or choose another Canvas.`,
    );
  }
}

export function nextCanvasCardPosition(index: number): {
  x: number;
  y: number;
} {
  return {
    x: 80 + (index % 3) * 340,
    y: 80 + Math.floor(index / 3) * 220,
  };
}

export function canvasContentNodeCount(nodes: CanvasNode[]): number {
  return nodes.filter((node) => node.type !== "group").length;
}

export function newTextCanvasNode(position: {
  x: number;
  y: number;
}): CanvasTextNode {
  return {
    id: nanoid(),
    type: "text",
    text: "New thought",
    x: position.x,
    y: position.y,
    width: 280,
    height: 160,
  };
}

export function newFileCanvasNode(
  file: string,
  position: { x: number; y: number },
): CanvasFileNode {
  return {
    id: nanoid(),
    type: "file",
    file,
    x: position.x,
    y: position.y,
    width: 280,
    height: 120,
  };
}

export function newLinkCanvasNode(
  url: string,
  position: { x: number; y: number },
): CanvasLinkNode {
  return {
    id: nanoid(),
    type: "link",
    url,
    x: position.x,
    y: position.y,
    width: 320,
    height: 120,
  };
}

export function newGroupCanvasNode(position: {
  x: number;
  y: number;
}): CanvasGroupNode {
  return {
    id: nanoid(),
    type: "group",
    label: "New group",
    x: position.x,
    y: position.y,
    width: 1040,
    height: 480,
  };
}

function summary(
  path: string,
  content: string,
  updatedAt: number,
): CanvasSummary {
  const filename =
    path
      .split("/")
      .at(-1)
      ?.replace(/\.canvas$/i, "") ?? "Untitled canvas";
  let parsed: unknown;
  let readError: CanvasSummary["readError"];
  try {
    parsed = JSON.parse(content);
  } catch {
    parsed = { nodes: [], edges: [] };
    readError = "invalid-json";
  }
  return {
    path,
    name: filename,
    updatedAt,
    document: normalizeCanvas(parsed),
    ...(readError ? { readError } : {}),
  };
}

export function normalizeCanvas(input: unknown): CanvasDocument {
  if (!input || typeof input !== "object") return { nodes: [], edges: [] };
  const candidate = input as Partial<CanvasDocument>;
  const nodes = Array.isArray(candidate.nodes)
    ? candidate.nodes.flatMap(normalizeCanvasNode)
    : [];
  const ids = new Set(nodes.map((node) => node.id));
  const edges = Array.isArray(candidate.edges)
    ? candidate.edges.flatMap((edge) => normalizeCanvasEdge(edge, ids))
    : [];
  return { nodes, edges };
}

function normalizeCanvasNode(node: unknown): CanvasNode[] {
  if (!node || typeof node !== "object") return [];
  const value = node as Record<string, unknown>;
  if (typeof value.id !== "string" || !hasGeometry(value)) return [];
  const base = {
    id: value.id,
    x: value.x,
    y: value.y,
    width: value.width,
    height: value.height,
    ...(canvasColorValue(value.color) ? { color: value.color as string } : {}),
  };
  if (value.type === "text" && typeof value.text === "string")
    return [{ ...base, type: "text", text: value.text }];
  if (value.type === "file" && typeof value.file === "string")
    return [
      {
        ...base,
        type: "file",
        file: value.file,
        ...(typeof value.subpath === "string"
          ? { subpath: value.subpath }
          : {}),
      },
    ];
  if (value.type === "link" && typeof value.url === "string")
    return [{ ...base, type: "link", url: value.url }];
  if (value.type === "group")
    return [
      {
        ...base,
        type: "group",
        ...(typeof value.label === "string" ? { label: value.label } : {}),
        ...(typeof value.background === "string" &&
        canvasBackgroundPresentation(value.background, value.backgroundStyle)
          ? { background: value.background }
          : {}),
        ...(isBackgroundStyle(value.backgroundStyle)
          ? { backgroundStyle: value.backgroundStyle }
          : {}),
      },
    ];
  return [];
}

function normalizeCanvasEdge(
  edge: unknown,
  nodeIds: Set<string>,
): CanvasEdge[] {
  if (!edge || typeof edge !== "object") return [];
  const value = edge as Record<string, unknown>;
  if (
    typeof value.id !== "string" ||
    typeof value.fromNode !== "string" ||
    typeof value.toNode !== "string"
  )
    return [];
  if (!nodeIds.has(value.fromNode) || !nodeIds.has(value.toNode)) return [];
  return [
    {
      id: value.id,
      fromNode: value.fromNode,
      toNode: value.toNode,
      ...(isCanvasSide(value.fromSide) ? { fromSide: value.fromSide } : {}),
      ...(isCanvasEnd(value.fromEnd) ? { fromEnd: value.fromEnd } : {}),
      ...(isCanvasSide(value.toSide) ? { toSide: value.toSide } : {}),
      ...(isCanvasEnd(value.toEnd) ? { toEnd: value.toEnd } : {}),
      ...(canvasColorValue(value.color)
        ? { color: value.color as string }
        : {}),
      ...(typeof value.label === "string" ? { label: value.label } : {}),
    },
  ];
}

function hasGeometry(
  value: Record<string, unknown>,
): value is Record<"x" | "y" | "width" | "height", number> &
  Record<string, unknown> {
  return (
    Number.isFinite(value.x) &&
    Number.isFinite(value.y) &&
    Number.isFinite(value.width) &&
    Number.isFinite(value.height) &&
    (value.width as number) > 0 &&
    (value.height as number) > 0
  );
}

function isCanvasSide(value: unknown): value is CanvasSide {
  return (
    value === "top" ||
    value === "right" ||
    value === "bottom" ||
    value === "left"
  );
}

function isCanvasEnd(value: unknown): value is CanvasEnd {
  return value === "none" || value === "arrow";
}

function isBackgroundStyle(
  value: unknown,
): value is NonNullable<CanvasGroupNode["backgroundStyle"]> {
  return value === "cover" || value === "ratio" || value === "repeat";
}
