/**
 * Native desktop vault bridge.
 *
 * Markdown files on disk are authoritative. Dexie contains a rebuildable
 * projection used by the legacy desktop workspace while Ley migrates to the
 * focused continuity product.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { db } from "@/infrastructure/database/db";
import type { Page } from "@/infrastructure/database/schema";
import { parseFrontmatter, getAliases } from "@/core/parser/frontmatter";
import { rebuildPageLinks } from "@/core/index/backlink";
import { rebuildPageTags } from "@/core/index/tag-index";
import {
  activeDataKind,
  filesystemDataKind,
  markActiveDataKind,
} from "@/infrastructure/database/active-vault-state";

const LAST_VAULT_KEY = "ley:last-filesystem-vault";

export interface VaultFileSnapshot {
  path: string;
  content: string;
  createdAt: number;
  updatedAt: number;
  /** Optional SHA-256 of the complete source, never a network-derived value. */
  sourceHash?: string;
}

export interface VaultPathChange {
  kind: "create" | "modify" | "remove" | "rename";
  /** Vault-relative destination/current path. */
  path: string;
  /** Vault-relative previous path, available only for a native rename event. */
  from?: string;
}

export interface VaultChange {
  /** Legacy path list retained for canvas and existing consumers. */
  paths: string[];
  changes: VaultPathChange[];
  /** The native watcher intentionally bounded an oversized event; rescan safely. */
  fullRescan?: boolean;
}

export interface VaultContinuityOptions {
  /** Only these currently-open IDs may survive an external removal. */
  openPageIds?: readonly string[];
  /** Explicit native rename data for this scan, if the platform supplied it. */
  changes?: readonly VaultPathChange[];
}

export interface CanvasFileSnapshot {
  path: string;
  content: string;
  updatedAt: number;
}

export interface DesktopVault {
  path: string;
  name: string;
  noteCount: number;
}

let activeVaultPath: string | null = null;
let desktopRefreshQueue: Promise<DesktopVault | null> = Promise.resolve(null);
let recentDesktopChanges: Array<{ at: number; change: VaultPathChange }> = [];

export function getActiveVaultPath(): string | null {
  return activeVaultPath;
}

export function getActiveVaultKind(): "desktop" | null {
  return activeVaultPath ? "desktop" : null;
}

export async function startDesktopVaultWatcher(
  onChange: (change: VaultChange) => void,
): Promise<() => void> {
  if (!activeVaultPath) return () => undefined;
  const watchedPath = activeVaultPath;
  const unlisten = await listen<VaultChange>("ley-vault-changed", (event) => {
    const now = Date.now();
    recentDesktopChanges = [
      ...recentDesktopChanges.filter((entry) => now - entry.at < 5_000),
      ...event.payload.changes.map((change) => ({ at: now, change })),
    ].slice(-256);
    onChange(event.payload);
  });
  try {
    await invoke("watch_vault", { vaultPath: watchedPath });
  } catch (error) {
    unlisten();
    throw error;
  }
  return () => {
    unlisten();
    void invoke("stop_watching_vault", { vaultPath: watchedPath }).catch(
      (error) => console.error("[vault] Could not stop filesystem watcher", error),
    );
  };
}

export async function restoreDesktopVault(): Promise<DesktopVault | null> {
  const path = localStorage.getItem(LAST_VAULT_KEY);
  if (!path) return null;
  try {
    return await loadDesktopVault(path);
  } catch (error) {
    console.warn("[vault] Could not restore the last vault:", error);
    localStorage.removeItem(LAST_VAULT_KEY);
    activeVaultPath = null;
    return null;
  }
}

export async function chooseDesktopVault(): Promise<DesktopVault | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Open folder as Ley vault",
  });
  if (!selected) return null;
  return loadDesktopVault(selected);
}

export function refreshDesktopVault(
  options?: VaultContinuityOptions,
): Promise<DesktopVault | null> {
  const path = activeVaultPath;
  const now = Date.now();
  recentDesktopChanges = recentDesktopChanges.filter(
    (entry) => now - entry.at < 5_000,
  );
  const continuity = {
    ...options,
    changes: [
      ...recentDesktopChanges.map((entry) => entry.change),
      ...(options?.changes ?? []),
    ],
  };
  const refresh = desktopRefreshQueue
    .catch(() => null)
    .then(() =>
      path && activeVaultPath === path
        ? loadDesktopVault(path, continuity)
        : null,
    );
  desktopRefreshQueue = refresh;
  return refresh;
}

async function loadDesktopVault(
  path: string,
  options?: VaultContinuityOptions,
): Promise<DesktopVault> {
  const snapshots = await invoke<VaultFileSnapshot[]>("scan_vault", {
    vaultPath: path,
  });
  activeVaultPath = path;
  localStorage.setItem(LAST_VAULT_KEY, path);
  await projectFilesIntoCache(path, snapshots, options);
  return {
    path,
    name: path.split(/[\\/]/).filter(Boolean).at(-1) ?? "Vault",
    noteCount: snapshots.length,
  };
}

export async function projectFilesIntoCache(
  vaultPath: string,
  snapshots: VaultFileSnapshot[],
  options: VaultContinuityOptions = {},
): Promise<void> {
  const kind = filesystemDataKind(vaultPath);
  const previousKind = await activeDataKind();
  const sameVault = previousKind === kind;
  const existing = sameVault
    ? (await db.pages.toArray()).filter((page) => page.deletedAt === null)
    : [];
  const existingByPath = new Map(existing.map((page) => [page.path, page]));
  const hashedSnapshots = await Promise.all(
    snapshots.map(async (snapshot) => ({
      ...snapshot,
      sourceHash:
        snapshot.sourceHash ?? (await hashVaultSource(snapshot.content)),
    })),
  );
  const snapshotPaths = new Set(
    hashedSnapshots.map((snapshot) => snapshot.path),
  );
  const { matched, claimedIds } = matchSnapshotsToExisting(
    existing,
    existingByPath,
    hashedSnapshots,
    snapshotPaths,
    options.changes,
  );

  const pages = hashedSnapshots.map((snapshot, index) =>
    pageFromSnapshot(vaultPath, snapshot, matched.get(index)),
  );
  const openPageIds = new Set(options.openPageIds ?? []);
  const removed = existing.filter(
    (page) => !claimedIds.has(page.id) && !snapshotPaths.has(page.path),
  );
  const missing = removed
    .filter((page) => openPageIds.has(page.id))
    .map((page) => ({ ...page, missingFromDisk: true }));
  const discardIds = removed
    .filter((page) => !openPageIds.has(page.id))
    .map((page) => page.id);

  await db.transaction(
    "rw",
    [db.pages, db.blocks, db.links, db.tags, db.assets, db.revisions],
    async () => {
      await Promise.all([db.blocks.clear(), db.links.clear(), db.tags.clear()]);
      if (!sameVault) {
        await Promise.all([
          db.pages.clear(),
          db.assets.clear(),
          db.revisions.clear(),
        ]);
      } else if (discardIds.length > 0) {
        await Promise.all([
          db.pages.bulkDelete(discardIds),
          ...discardIds.map((id) =>
            db.assets.where("pageId").equals(id).delete(),
          ),
          ...discardIds.map((id) =>
            db.revisions.where("pageId").equals(id).delete(),
          ),
        ]);
      }
      if (pages.length + missing.length > 0)
        await db.pages.bulkPut([...pages, ...missing]);
    },
  );
  await markActiveDataKind(kind);

  // Resolve links only after every page exists in the cache.
  for (const page of pages) {
    await rebuildPageLinks(page.id, page.content);
    await rebuildPageTags(page.id, page.content, page.frontmatter);
  }
  if (typeof window !== "undefined")
    window.dispatchEvent(new CustomEvent("ley:vault-projected"));
}

function matchSnapshotsToExisting(
  existing: Page[],
  existingByPath: Map<string, Page>,
  snapshots: VaultFileSnapshot[],
  snapshotPaths: Set<string>,
  changes: readonly VaultPathChange[] | undefined,
): { matched: Map<number, Page>; claimedIds: Set<string> } {
  const matched = new Map<number, Page>();
  const claimedIds = new Set<string>();
  matchSnapshotsByPath(snapshots, existingByPath, matched, claimedIds);
  matchSnapshotsByRename(
    snapshots,
    existingByPath,
    snapshotPaths,
    changes,
    matched,
    claimedIds,
  );
  matchSnapshotsByHash(existing, snapshots, snapshotPaths, matched, claimedIds);
  return { matched, claimedIds };
}

function matchSnapshotsByPath(
  snapshots: VaultFileSnapshot[],
  existingByPath: Map<string, Page>,
  matched: Map<number, Page>,
  claimedIds: Set<string>,
): void {
  for (const [index, snapshot] of snapshots.entries()) {
    const page = existingByPath.get(snapshot.path);
    if (!page) continue;
    matched.set(index, page);
    claimedIds.add(page.id);
  }
}

function matchSnapshotsByRename(
  snapshots: VaultFileSnapshot[],
  existingByPath: Map<string, Page>,
  snapshotPaths: Set<string>,
  changes: readonly VaultPathChange[] | undefined,
  matched: Map<number, Page>,
  claimedIds: Set<string>,
): void {
  const renameSources = buildRenameSources(changes);
  const originalRenameSource = (destination: string): string | null => {
    let current = destination;
    const visited = new Set<string>();
    while (!visited.has(current)) {
      visited.add(current);
      const sources = renameSources.get(current);
      if (!sources || sources.length !== 1) {
        return current === destination ? null : current;
      }
      current = sources[0];
      if (existingByPath.has(current)) return current;
    }
    return null;
  };
  for (const [index, snapshot] of snapshots.entries()) {
    if (matched.has(index)) continue;
    const source = originalRenameSource(snapshot.path);
    if (!source) continue;
    const page = existingByPath.get(source);
    if (!page || claimedIds.has(page.id) || snapshotPaths.has(source)) continue;
    matched.set(index, page);
    claimedIds.add(page.id);
  }
}

function buildRenameSources(
  changes: readonly VaultPathChange[] | undefined,
): Map<string, string[]> {
  const renameSources = new Map<string, string[]>();
  for (const change of changes ?? []) {
    if (change.kind !== "rename" || !change.from) continue;
    const sources = renameSources.get(change.path) ?? [];
    if (!sources.includes(change.from)) sources.push(change.from);
    renameSources.set(change.path, sources);
  }
  return renameSources;
}

function matchSnapshotsByHash(
  existing: Page[],
  snapshots: VaultFileSnapshot[],
  snapshotPaths: Set<string>,
  matched: Map<number, Page>,
  claimedIds: Set<string>,
): void {
  const previousByHash = new Map<string, Page[]>();
  for (const page of existing) {
    if (
      claimedIds.has(page.id) ||
      snapshotPaths.has(page.path) ||
      !page.sourceHash
    )
      continue;
    const candidates = previousByHash.get(page.sourceHash) ?? [];
    candidates.push(page);
    previousByHash.set(page.sourceHash, candidates);
  }
  const snapshotsByHash = new Map<string, number[]>();
  for (const [index, snapshot] of snapshots.entries()) {
    if (matched.has(index) || !snapshot.sourceHash) continue;
    const candidates = snapshotsByHash.get(snapshot.sourceHash) ?? [];
    candidates.push(index);
    snapshotsByHash.set(snapshot.sourceHash, candidates);
  }
  for (const [sourceHash, indexes] of snapshotsByHash) {
    const candidates = previousByHash.get(sourceHash);
    if (indexes.length !== 1 || candidates?.length !== 1) continue;
    matched.set(indexes[0], candidates[0]);
    claimedIds.add(candidates[0].id);
  }
}

function pageFromSnapshot(
  vaultPath: string,
  snapshot: VaultFileSnapshot,
  existing?: Page,
): Page {
  const parsed = parseFrontmatter(snapshot.content);
  const filename =
    snapshot.path.split("/").at(-1)?.replace(/\.md$/i, "") ?? "Untitled";
  const title =
    typeof parsed.frontmatter.title === "string" &&
    parsed.frontmatter.title.trim()
      ? parsed.frontmatter.title.trim()
      : filename;
  return {
    id: existing?.id ?? stableFileId(vaultPath, snapshot.path),
    title,
    lcTitle: title.toLowerCase(),
    path: snapshot.path,
    content: parsed.body,
    frontmatter: parsed.frontmatter,
    frontmatterError: parsed.error,
    aliases: getAliases(parsed.frontmatter),
    createdAt:
      existing?.createdAt ??
      (snapshot.createdAt || snapshot.updatedAt || Date.now()),
    updatedAt: snapshot.updatedAt || Date.now(),
    deletedAt: null,
    sourceHash: snapshot.sourceHash,
    missingFromDisk: undefined,
  };
}

/** Local SHA-256 for rename matching. An unavailable WebCrypto implementation
 * simply disables hash fallback rather than risking a weaker heuristic. */
export async function hashVaultSource(
  content: string,
): Promise<string | undefined> {
  if (!globalThis.crypto?.subtle) return undefined;
  try {
    const bytes = new TextEncoder().encode(content);
    const digest = await globalThis.crypto.subtle.digest("SHA-256", bytes);
    return `sha256:${Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
  } catch {
    return undefined;
  }
}

function stableFileId(vaultPath: string, path: string): string {
  const input = `${vaultPath}\0${path}`;
  let hash = 0x811c9dc5;
  for (let i = 0; i < input.length; i += 1) {
    hash ^= input.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return `file_${(hash >>> 0).toString(36)}`;
}

function requireActiveVaultPath(): string {
  if (!activeVaultPath) throw new Error("No desktop vault is open");
  return activeVaultPath;
}

export async function writeActiveVaultFile(
  relativePath: string,
  content: string,
): Promise<void> {
  await invoke("write_vault_file", {
    vaultPath: requireActiveVaultPath(),
    relativePath,
    content,
  });
}

export async function writeActiveVaultAttachment(
  relativePath: string,
  data: ArrayBuffer,
): Promise<boolean> {
  await invoke("write_vault_attachment", {
    vaultPath: requireActiveVaultPath(),
    relativePath,
    bytes: Array.from(new Uint8Array(data)),
  });
  return true;
}

export async function readActiveVaultAttachment(
  relativePath: string,
): Promise<ArrayBuffer | null> {
  if (!activeVaultPath) return null;
  const bytes = await invoke<number[]>("read_vault_attachment", {
    vaultPath: activeVaultPath,
    relativePath,
  });
  return Uint8Array.from(bytes).buffer;
}

export async function listActiveCanvasFiles(): Promise<
  CanvasFileSnapshot[] | null
> {
  if (!activeVaultPath) return null;
  return invoke<CanvasFileSnapshot[]>("scan_canvases", {
    vaultPath: activeVaultPath,
  });
}

export async function writeActiveCanvasFile(
  relativePath: string,
  content: string,
): Promise<boolean> {
  JSON.parse(content);
  if (!activeVaultPath) return false;
  await invoke("write_canvas_file", {
    vaultPath: activeVaultPath,
    relativePath,
    content,
  });
  return true;
}

export async function trashActiveCanvasFile(
  relativePath: string,
): Promise<boolean> {
  if (!activeVaultPath) return false;
  await invoke("trash_canvas_file", {
    vaultPath: activeVaultPath,
    relativePath,
  });
  return true;
}

export async function renameActiveVaultFile(
  from: string,
  to: string,
): Promise<void> {
  await invoke("rename_vault_file", {
    vaultPath: requireActiveVaultPath(),
    from,
    to,
  });
}

export async function trashActiveVaultFile(
  relativePath: string,
): Promise<void> {
  await invoke("trash_vault_file", {
    vaultPath: requireActiveVaultPath(),
    relativePath,
  });
}

export async function listActiveVaultTrash(): Promise<
  VaultFileSnapshot[] | null
> {
  if (!activeVaultPath) return null;
  const files = await invoke<
    Array<{
      path: string;
      content: string;
      createdAt: number;
      updatedAt: number;
    }>
  >("scan_trashed_vault_files", { vaultPath: activeVaultPath });
  return files.map((file) => ({
    path: file.path,
    content: file.content,
    createdAt: file.createdAt,
    updatedAt: file.updatedAt,
  }));
}

export async function restoreActiveVaultTrashFile(
  trashedPath: string,
): Promise<string | null> {
  if (!activeVaultPath) return null;
  return invoke<string>("restore_trashed_vault_file", {
    vaultPath: activeVaultPath,
    trashedPath,
  });
}

export async function readActiveVaultFile(
  relativePath: string,
): Promise<string | null> {
  if (!activeVaultPath) return null;
  return invoke<string>("read_vault_file", {
    vaultPath: activeVaultPath,
    relativePath,
  });
}
