import { db } from '@/infrastructure/database/db';
import { listActiveCanvasFiles, trashActiveCanvasFile, writeActiveCanvasFile } from '@/infrastructure/vault/filesystem-vault';
import { nanoid } from '@/shared/lib/nanoid';
import { slugify } from '@/shared/lib/slug';

export interface CanvasTextNode {
  id: string;
  type: 'text';
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  color?: string;
}

export interface CanvasFileNode {
  id: string;
  type: 'file';
  file: string;
  x: number;
  y: number;
  width: number;
  height: number;
  color?: string;
}

export type CanvasNode = CanvasTextNode | CanvasFileNode;

export interface CanvasEdge {
  id: string;
  fromNode: string;
  toNode: string;
  fromSide?: 'top' | 'right' | 'bottom' | 'left';
  toSide?: 'top' | 'right' | 'bottom' | 'left';
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
}

export async function listCanvases(): Promise<CanvasSummary[]> {
  const filesystem = await listActiveCanvasFiles();
  if (filesystem) return filesystem.map((file) => summary(file.path, file.content, file.updatedAt));
  const rows = (await db.settings.toArray()).filter((row) => row.key.startsWith('canvas:'));
  return rows.map((row) => {
    const value = row.value as { content?: string; updatedAt?: number };
    return summary(row.key.slice('canvas:'.length), value.content ?? '{"nodes":[],"edges":[]}', value.updatedAt ?? 0);
  }).sort((left, right) => left.name.localeCompare(right.name));
}

export async function createCanvas(name: string): Promise<CanvasSummary> {
  const cleanName = name.trim() || 'Untitled canvas';
  const path = `canvases/${slugify(cleanName)}.canvas`;
  const existing = (await listCanvases()).find((canvas) => canvas.path.toLowerCase() === path.toLowerCase());
  if (existing) return existing;
  const document: CanvasDocument = { nodes: [], edges: [] };
  await saveCanvas(path, document);
  return { path, name: cleanName, updatedAt: Date.now(), document };
}

export async function saveCanvas(path: string, document: CanvasDocument): Promise<void> {
  const normalized = normalizeCanvas(document);
  const content = JSON.stringify(normalized, null, 2);
  if (!await writeActiveCanvasFile(path, content)) {
    await db.settings.put({ key: `canvas:${path}`, value: { content, updatedAt: Date.now() } });
  }
}

export async function deleteCanvas(path: string): Promise<void> {
  if (!await trashActiveCanvasFile(path)) await db.settings.delete(`canvas:${path}`);
}

export function newTextCanvasNode(position: { x: number; y: number }): CanvasTextNode {
  return { id: nanoid(), type: 'text', text: 'New thought', x: position.x, y: position.y, width: 280, height: 160 };
}

export function newFileCanvasNode(file: string, position: { x: number; y: number }): CanvasFileNode {
  return { id: nanoid(), type: 'file', file, x: position.x, y: position.y, width: 280, height: 120 };
}

function summary(path: string, content: string, updatedAt: number): CanvasSummary {
  const filename = path.split('/').at(-1)?.replace(/\.canvas$/i, '') ?? 'Untitled canvas';
  let parsed: unknown;
  try { parsed = JSON.parse(content); } catch { parsed = { nodes: [], edges: [] }; }
  return { path, name: filename, updatedAt, document: normalizeCanvas(parsed) };
}

function normalizeCanvas(input: unknown): CanvasDocument {
  if (!input || typeof input !== 'object') return { nodes: [], edges: [] };
  const candidate = input as Partial<CanvasDocument>;
  const nodes = Array.isArray(candidate.nodes) ? candidate.nodes.filter(isCanvasNode) : [];
  const ids = new Set(nodes.map((node) => node.id));
  const edges = Array.isArray(candidate.edges)
    ? candidate.edges.filter((edge): edge is CanvasEdge => Boolean(edge && typeof edge.id === 'string' && ids.has(edge.fromNode) && ids.has(edge.toNode)))
    : [];
  return { nodes, edges };
}

function isCanvasNode(node: unknown): node is CanvasNode {
  if (!node || typeof node !== 'object') return false;
  const value = node as { id?: unknown; type?: unknown; text?: unknown; file?: unknown; x?: unknown; y?: unknown; width?: unknown; height?: unknown };
  return typeof value.id === 'string'
    && (value.type === 'text' || value.type === 'file')
    && typeof value.x === 'number' && typeof value.y === 'number'
    && typeof value.width === 'number' && typeof value.height === 'number'
    && (value.type === 'text' ? typeof value.text === 'string' : typeof value.file === 'string');
}
