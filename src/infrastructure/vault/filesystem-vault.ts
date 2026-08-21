/**
 * Desktop vault bridge.
 *
 * Markdown files on disk are authoritative. Dexie contains a rebuildable
 * projection used by the React UI, search, backlinks, tags, and graph.
 */

import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { db } from '@/infrastructure/database/db';
import type { Page } from '@/infrastructure/database/schema';
import { parseFrontmatter, getAliases } from '@/core/parser/frontmatter';
import { rebuildPageLinks } from '@/core/index/backlink';
import { rebuildPageTags } from '@/core/index/tag-index';
import { activeDataKind, filesystemDataKind, markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { nanoid } from '@/shared/lib/nanoid';

const LAST_VAULT_KEY = 'ley:last-filesystem-vault';
const BROWSER_HANDLE_KEY = 'browser-directory-handle';
const BROWSER_VAULT_ID_KEY = 'browser-directory-vault-id';
const BROWSER_VAULT_REGISTRY_KEY = 'browser-directory-vault-registry';

export interface VaultFileSnapshot {
  path: string;
  content: string;
  createdAt: number;
  updatedAt: number;
  /** Optional SHA-256 of the complete source, never a network-derived value. */
  sourceHash?: string;
}

export interface VaultPathChange {
  kind: 'create' | 'modify' | 'remove' | 'rename';
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
let activeBrowserHandle: LeyDirectoryHandle | null = null;
let activeBrowserVaultId: string | null = null;
let desktopRefreshQueue: Promise<DesktopVault | null> = Promise.resolve(null);
let browserRefreshQueue: Promise<DesktopVault | null> = Promise.resolve(null);
let recentDesktopChanges: Array<{ at: number; change: VaultPathChange }> = [];

interface LeyFileHandle {
  kind: 'file';
  name: string;
  getFile(): Promise<File>;
  createWritable(): Promise<{ write(data: string | Blob | ArrayBuffer): Promise<void>; close(): Promise<void> }>;
}

interface LeyDirectoryHandle {
  kind: 'directory';
  name: string;
  entries(): AsyncIterableIterator<[string, LeyFileHandle | LeyDirectoryHandle]>;
  getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<LeyDirectoryHandle>;
  getFileHandle(name: string, options?: { create?: boolean }): Promise<LeyFileHandle>;
  removeEntry(name: string, options?: { recursive?: boolean }): Promise<void>;
  queryPermission(options: { mode: 'readwrite' }): Promise<PermissionState>;
  requestPermission(options: { mode: 'readwrite' }): Promise<PermissionState>;
  isSameEntry?(other: LeyDirectoryHandle): Promise<boolean>;
}

interface BrowserVaultRegistryEntry {
  id: string;
  handle: LeyDirectoryHandle;
}

export function isDesktopApp(): boolean {
  return isTauri();
}

export function getActiveVaultPath(): string | null {
  return activeVaultPath;
}

export function isBrowserFolderSupported(): boolean {
  return 'showDirectoryPicker' in window;
}

export function getActiveVaultKind(): 'desktop' | 'browser-folder' | null {
  if (activeVaultPath) return 'desktop';
  if (activeBrowserHandle) return 'browser-folder';
  return null;
}

export function deactivateFilesystemVault(): void {
  activeVaultPath = null;
  activeBrowserHandle = null;
  activeBrowserVaultId = null;
  recentDesktopChanges = [];
}

export async function startDesktopVaultWatcher(onChange: (change: VaultChange) => void): Promise<() => void> {
  if (!activeVaultPath) return () => undefined;
  const watchedPath = activeVaultPath;
  const unlisten = await listen<VaultChange>('ley-vault-changed', (event) => {
    const now = Date.now();
    recentDesktopChanges = [
      ...recentDesktopChanges.filter((entry) => now - entry.at < 5_000),
      ...event.payload.changes.map((change) => ({ at: now, change })),
    ].slice(-256);
    onChange(event.payload);
  });
  try {
    await invoke('watch_vault', { vaultPath: watchedPath });
  } catch (error) {
    unlisten();
    throw error;
  }
  return () => {
    unlisten();
    void invoke('stop_watching_vault', { vaultPath: watchedPath }).catch((error) => console.error('[vault] Could not stop filesystem watcher', error));
  };
}

export async function restoreDesktopVault(): Promise<DesktopVault | null> {
  if (!isDesktopApp()) return null;
  const path = localStorage.getItem(LAST_VAULT_KEY);
  if (!path) return null;
  try {
    return await loadDesktopVault(path);
  } catch (error) {
    console.warn('[vault] Could not restore the last vault:', error);
    localStorage.removeItem(LAST_VAULT_KEY);
    activeVaultPath = null;
    return null;
  }
}

export async function chooseDesktopVault(): Promise<DesktopVault | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: 'Open folder as Ley vault',
  });
  if (!selected) return null;
  return loadDesktopVault(selected);
}

export async function chooseBrowserFolderVault(): Promise<DesktopVault | null> {
  if (!isBrowserFolderSupported()) return null;
  const picker = (window as typeof window & {
    showDirectoryPicker(options?: { mode?: 'readwrite' }): Promise<LeyDirectoryHandle>;
  }).showDirectoryPicker;
  const handle = await picker({ mode: 'readwrite' });
  const permission = await handle.requestPermission({ mode: 'readwrite' });
  if (permission !== 'granted') throw new Error('Ley needs read and write access to use this folder as a vault.');
  const vaultId = await resolveBrowserVaultId(handle);
  await db.settings.put({ key: BROWSER_HANDLE_KEY, value: handle });
  await db.settings.put({ key: BROWSER_VAULT_ID_KEY, value: vaultId });
  return loadBrowserFolderVault(handle, vaultId);
}

export async function restoreBrowserFolderVault(): Promise<DesktopVault | null> {
  if (!isBrowserFolderSupported()) return null;
  const row = await db.settings.get(BROWSER_HANDLE_KEY);
  const handle = row?.value as LeyDirectoryHandle | undefined;
  if (!handle || handle.kind !== 'directory') return null;
  if (await handle.queryPermission({ mode: 'readwrite' }) !== 'granted') return null;
  const storedId = (await db.settings.get(BROWSER_VAULT_ID_KEY))?.value;
  const vaultId = await resolveBrowserVaultId(handle, typeof storedId === 'string' ? storedId : undefined);
  if (storedId !== vaultId) await db.settings.put({ key: BROWSER_VAULT_ID_KEY, value: vaultId });
  return loadBrowserFolderVault(handle, vaultId);
}

export function refreshBrowserFolderVault(options?: VaultContinuityOptions): Promise<DesktopVault | null> {
  const handle = activeBrowserHandle;
  const vaultId = activeBrowserVaultId;
  const refresh = browserRefreshQueue.catch(() => null).then(() => (
    handle && vaultId && activeBrowserHandle === handle && activeBrowserVaultId === vaultId
      ? loadBrowserFolderVault(handle, vaultId, options)
      : null
  ));
  browserRefreshQueue = refresh;
  return refresh;
}

export function refreshDesktopVault(options?: VaultContinuityOptions): Promise<DesktopVault | null> {
  const path = activeVaultPath;
  const now = Date.now();
  recentDesktopChanges = recentDesktopChanges.filter((entry) => now - entry.at < 5_000);
  const continuity = {
    ...options,
    changes: [
      ...recentDesktopChanges.map((entry) => entry.change),
      ...(options?.changes ?? []),
    ],
  };
  const refresh = desktopRefreshQueue.catch(() => null).then(() => (
    path && activeVaultPath === path ? loadDesktopVault(path, continuity) : null
  ));
  desktopRefreshQueue = refresh;
  return refresh;
}

async function loadDesktopVault(path: string, options?: VaultContinuityOptions): Promise<DesktopVault> {
  const snapshots = await invoke<VaultFileSnapshot[]>('scan_vault', { vaultPath: path });
  activeVaultPath = path;
  localStorage.setItem(LAST_VAULT_KEY, path);
  await projectFilesIntoCache(path, snapshots, options);
  return {
    path,
    name: path.split(/[\\/]/).filter(Boolean).at(-1) ?? 'Vault',
    noteCount: snapshots.length,
  };
}

async function loadBrowserFolderVault(handle: LeyDirectoryHandle, vaultId: string, options?: VaultContinuityOptions): Promise<DesktopVault> {
  const snapshots = await scanBrowserFolder(handle);
  activeBrowserHandle = handle;
  activeBrowserVaultId = vaultId;
  activeVaultPath = null;
  await projectFilesIntoCache(`browser-folder:${vaultId}`, snapshots, options);
  return { path: vaultId, name: handle.name, noteCount: snapshots.length };
}

async function resolveBrowserVaultId(handle: LeyDirectoryHandle, preferredId?: string): Promise<string> {
  const stored = (await db.settings.get(BROWSER_VAULT_REGISTRY_KEY))?.value;
  const registry = Array.isArray(stored)
    ? stored.filter((entry): entry is BrowserVaultRegistryEntry => Boolean(entry && typeof entry === 'object' && typeof (entry as BrowserVaultRegistryEntry).id === 'string' && (entry as BrowserVaultRegistryEntry).handle?.kind === 'directory'))
    : [];
  for (const entry of registry) {
    const same = handle.isSameEntry
      ? await handle.isSameEntry(entry.handle).catch(() => false)
      : entry.handle.isSameEntry
        ? await entry.handle.isSameEntry(handle).catch(() => false)
        : false;
    if (same) return entry.id;
  }
  const id = preferredId ?? nanoid();
  await db.settings.put({ key: BROWSER_VAULT_REGISTRY_KEY, value: [...registry, { id, handle }] });
  return id;
}

async function scanBrowserFolder(root: LeyDirectoryHandle): Promise<VaultFileSnapshot[]> {
  const snapshots: VaultFileSnapshot[] = [];
  async function walk(directory: LeyDirectoryHandle, prefix: string) {
    for await (const [name, handle] of directory.entries()) {
      if (name.startsWith('.') || name === 'node_modules') continue;
      const path = prefix ? `${prefix}/${name}` : name;
      if (handle.kind === 'directory') await walk(handle, path);
      else if (name.toLowerCase().endsWith('.md')) {
        const file = await handle.getFile();
        const content = await file.text();
        snapshots.push({
          path,
          content,
          createdAt: file.lastModified,
          updatedAt: file.lastModified,
          sourceHash: await hashVaultSource(content),
        });
      }
    }
  }
  await walk(root, '');
  return snapshots.sort((left, right) => left.path.localeCompare(right.path));
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
  const hashedSnapshots = await Promise.all(snapshots.map(async (snapshot) => ({
    ...snapshot,
    sourceHash: snapshot.sourceHash ?? await hashVaultSource(snapshot.content),
  })));
  const snapshotPaths = new Set(hashedSnapshots.map((snapshot) => snapshot.path));
  const matched = new Map<number, Page>();
  const claimedIds = new Set<string>();

  // A stable path is always the strongest identity signal, even if external
  // editing also changed the file contents.
  hashedSnapshots.forEach((snapshot, index) => {
    const page = existingByPath.get(snapshot.path);
    if (page) {
      matched.set(index, page);
      claimedIds.add(page.id);
    }
  });

  // Native notify can report a paired old/new rename. Use it only when both
  // paths are unambiguous and the old path disappeared from this scan.
  const renameSources = new Map<string, string[]>();
  for (const change of options.changes ?? []) {
    if (change.kind !== 'rename' || !change.from) continue;
    const destination = change.path;
    const sources = renameSources.get(destination) ?? [];
    if (!sources.includes(change.from)) sources.push(change.from);
    renameSources.set(destination, sources);
  }
  const originalRenameSource = (destination: string): string | null => {
    let current = destination;
    const visited = new Set<string>();
    while (!visited.has(current)) {
      visited.add(current);
      const sources = renameSources.get(current);
      if (!sources || sources.length !== 1) return current === destination ? null : current;
      current = sources[0];
      if (existingByPath.has(current)) return current;
    }
    return null;
  };
  hashedSnapshots.forEach((snapshot, index) => {
    if (matched.has(index)) return;
    const source = originalRenameSource(snapshot.path);
    if (!source) return;
    const page = existingByPath.get(source);
    if (!page || claimedIds.has(page.id) || snapshotPaths.has(source)) return;
    matched.set(index, page);
    claimedIds.add(page.id);
  });

  // Focus/manual refreshes have no native event. Fall back only when the
  // full-source SHA-256 occurs exactly once on each side; duplicate notes are
  // deliberately left as distinct files rather than guessed into a remap.
  const previousByHash = new Map<string, Page[]>();
  for (const page of existing) {
    if (claimedIds.has(page.id) || snapshotPaths.has(page.path) || !page.sourceHash) continue;
    const candidates = previousByHash.get(page.sourceHash) ?? [];
    candidates.push(page);
    previousByHash.set(page.sourceHash, candidates);
  }
  const snapshotsByHash = new Map<string, number[]>();
  hashedSnapshots.forEach((snapshot, index) => {
    if (matched.has(index) || !snapshot.sourceHash) return;
    const candidates = snapshotsByHash.get(snapshot.sourceHash) ?? [];
    candidates.push(index);
    snapshotsByHash.set(snapshot.sourceHash, candidates);
  });
  for (const [sourceHash, indexes] of snapshotsByHash) {
    const candidates = previousByHash.get(sourceHash);
    if (indexes.length !== 1 || candidates?.length !== 1) continue;
    matched.set(indexes[0], candidates[0]);
    claimedIds.add(candidates[0].id);
  }

  const pages = hashedSnapshots.map((snapshot, index) => pageFromSnapshot(vaultPath, snapshot, matched.get(index)));
  const openPageIds = new Set(options.openPageIds ?? []);
  const removed = existing.filter((page) => !claimedIds.has(page.id) && !snapshotPaths.has(page.path));
  const missing = removed
    .filter((page) => openPageIds.has(page.id))
    .map((page) => ({ ...page, missingFromDisk: true }));
  const discardIds = removed.filter((page) => !openPageIds.has(page.id)).map((page) => page.id);

  await db.transaction('rw', [db.pages, db.blocks, db.links, db.tags, db.assets, db.revisions], async () => {
    await Promise.all([db.blocks.clear(), db.links.clear(), db.tags.clear()]);
    if (!sameVault) {
      await Promise.all([db.pages.clear(), db.assets.clear(), db.revisions.clear()]);
    } else if (discardIds.length > 0) {
      await Promise.all([
        db.pages.bulkDelete(discardIds),
        ...discardIds.map((id) => db.assets.where('pageId').equals(id).delete()),
        ...discardIds.map((id) => db.revisions.where('pageId').equals(id).delete()),
      ]);
    }
    if (pages.length + missing.length > 0) await db.pages.bulkPut([...pages, ...missing]);
  });
  await markActiveDataKind(kind);

  // Resolve links only after every page exists in the cache.
  for (const page of pages) {
    await rebuildPageLinks(page.id, page.content);
    await rebuildPageTags(page.id, page.content, page.frontmatter);
  }
  if (typeof window !== 'undefined') window.dispatchEvent(new CustomEvent('ley:vault-projected'));
}

function pageFromSnapshot(vaultPath: string, snapshot: VaultFileSnapshot, existing?: Page): Page {
  const parsed = parseFrontmatter(snapshot.content);
  const filename = snapshot.path.split('/').at(-1)?.replace(/\.md$/i, '') ?? 'Untitled';
  const title = typeof parsed.frontmatter.title === 'string' && parsed.frontmatter.title.trim()
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
    createdAt: existing?.createdAt ?? (snapshot.createdAt || snapshot.updatedAt || Date.now()),
    updatedAt: snapshot.updatedAt || Date.now(),
    deletedAt: null,
    sourceHash: snapshot.sourceHash,
    missingFromDisk: undefined,
  };
}

/** Local SHA-256 for rename matching. An unavailable WebCrypto implementation
 * simply disables hash fallback rather than risking a weaker heuristic. */
export async function hashVaultSource(content: string): Promise<string | undefined> {
  if (!globalThis.crypto?.subtle) return undefined;
  try {
    const bytes = new TextEncoder().encode(content);
    const digest = await globalThis.crypto.subtle.digest('SHA-256', bytes);
    return `sha256:${Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('')}`;
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

export async function writeActiveVaultFile(relativePath: string, content: string): Promise<void> {
  if (activeVaultPath) {
    await invoke('write_vault_file', { vaultPath: activeVaultPath, relativePath, content });
  } else if (activeBrowserHandle) {
    const file = await browserFileHandle(activeBrowserHandle, relativePath, true, true);
    const writer = await file.createWritable();
    await writer.write(content);
    await writer.close();
  }
}

export async function writeActiveVaultAttachment(relativePath: string, data: ArrayBuffer): Promise<boolean> {
  if (activeVaultPath) {
    await invoke('write_vault_attachment', { vaultPath: activeVaultPath, relativePath, bytes: Array.from(new Uint8Array(data)) });
    return true;
  }
  if (activeBrowserHandle) {
    const file = await browserFileHandle(activeBrowserHandle, relativePath, true, false);
    const writer = await file.createWritable();
    await writer.write(data);
    await writer.close();
    return true;
  }
  return false;
}

export async function readActiveVaultAttachment(relativePath: string): Promise<ArrayBuffer | null> {
  if (activeVaultPath) {
    const bytes = await invoke<number[]>('read_vault_attachment', { vaultPath: activeVaultPath, relativePath });
    return Uint8Array.from(bytes).buffer;
  }
  if (activeBrowserHandle) {
    const handle = await browserFileHandle(activeBrowserHandle, relativePath, false, false);
    return (await handle.getFile()).arrayBuffer();
  }
  return null;
}

export async function listActiveCanvasFiles(): Promise<CanvasFileSnapshot[] | null> {
  if (activeVaultPath) return invoke<CanvasFileSnapshot[]>('scan_canvases', { vaultPath: activeVaultPath });
  if (!activeBrowserHandle) return null;
  try {
    const root = await activeBrowserHandle.getDirectoryHandle('canvases');
    const snapshots: CanvasFileSnapshot[] = [];
    for await (const [name, handle] of root.entries()) {
      if (handle.kind !== 'file' || !name.toLowerCase().endsWith('.canvas')) continue;
      const file = await handle.getFile();
      snapshots.push({ path: `canvases/${name}`, content: await file.text(), updatedAt: file.lastModified });
    }
    return snapshots.sort((left, right) => left.path.localeCompare(right.path));
  } catch {
    return [];
  }
}

export async function writeActiveCanvasFile(relativePath: string, content: string): Promise<boolean> {
  JSON.parse(content);
  if (activeVaultPath) {
    await invoke('write_canvas_file', { vaultPath: activeVaultPath, relativePath, content });
    return true;
  }
  if (activeBrowserHandle) {
    const parts = relativePath.split('/').filter(Boolean);
    const filename = parts.pop();
    if (parts.join('/') !== 'canvases' || !filename?.toLowerCase().endsWith('.canvas')) throw new Error('Canvas files must use canvases/*.canvas');
    const directory = await browserDirectoryHandle(activeBrowserHandle, parts, true);
    const file = await directory.getFileHandle(filename, { create: true });
    const writer = await file.createWritable();
    await writer.write(content);
    await writer.close();
    return true;
  }
  return false;
}

export async function trashActiveCanvasFile(relativePath: string): Promise<boolean> {
  if (activeVaultPath) {
    await invoke('trash_canvas_file', { vaultPath: activeVaultPath, relativePath });
    return true;
  }
  if (activeBrowserHandle) {
    const parts = relativePath.split('/').filter(Boolean);
    const filename = parts.pop();
    if (parts.join('/') !== 'canvases' || !filename?.endsWith('.canvas')) throw new Error('Invalid canvas path');
    const sourceDirectory = await browserDirectoryHandle(activeBrowserHandle, parts, false);
    const source = await sourceDirectory.getFileHandle(filename);
    const content = await (await source.getFile()).text();
    const trash = await browserDirectoryHandle(activeBrowserHandle, ['.trash'], true);
    const stem = filename.replace(/\.canvas$/i, '');
    const target = await trash.getFileHandle(`${stem}-${Date.now()}.canvas`, { create: true });
    const writer = await target.createWritable();
    await writer.write(content);
    await writer.close();
    await sourceDirectory.removeEntry(filename);
    return true;
  }
  return false;
}

export async function renameActiveVaultFile(from: string, to: string): Promise<void> {
  if (activeVaultPath) {
    await invoke('rename_vault_file', { vaultPath: activeVaultPath, from, to });
  } else if (activeBrowserHandle) {
    const source = await browserFileHandle(activeBrowserHandle, from, false, true);
    const content = await (await source.getFile()).text();
    const target = await browserFileHandle(activeBrowserHandle, to, true, true);
    const writer = await target.createWritable();
    await writer.write(content);
    await writer.close();
    await removeBrowserPath(activeBrowserHandle, from);
  }
}

export async function trashActiveVaultFile(relativePath: string): Promise<void> {
  if (activeVaultPath) {
    await invoke('trash_vault_file', { vaultPath: activeVaultPath, relativePath });
  } else if (activeBrowserHandle) {
    const source = await browserFileHandle(activeBrowserHandle, relativePath, false, true);
    const content = await (await source.getFile()).text();
    const filename = relativePath.split('/').at(-1) ?? 'Untitled.md';
    const target = await browserFileHandle(activeBrowserHandle, `.trash/${filename}`, true, true);
    const writer = await target.createWritable();
    await writer.write(content);
    await writer.close();
    await removeBrowserPath(activeBrowserHandle, relativePath);
  }
}

async function browserDirectoryHandle(root: LeyDirectoryHandle, parts: string[], create: boolean): Promise<LeyDirectoryHandle> {
  let directory = root;
  for (const part of parts) {
    if (!part || part === '.' || part === '..') throw new Error('Unsafe vault path');
    directory = await directory.getDirectoryHandle(part, { create });
  }
  return directory;
}

async function browserFileHandle(root: LeyDirectoryHandle, relativePath: string, create: boolean, markdownOnly: boolean): Promise<LeyFileHandle> {
  const parts = relativePath.split('/').filter(Boolean);
  const filename = parts.pop();
  if (!filename || (markdownOnly && !filename.toLowerCase().endsWith('.md'))) throw new Error('Ley note paths must end in .md');
  if (!markdownOnly && parts[0] !== 'attachments') throw new Error('Attachments must be stored inside the attachments folder');
  const directory = await browserDirectoryHandle(root, parts, create);
  return directory.getFileHandle(filename, { create });
}

async function removeBrowserPath(root: LeyDirectoryHandle, relativePath: string): Promise<void> {
  const parts = relativePath.split('/').filter(Boolean);
  const filename = parts.pop();
  if (!filename) throw new Error('Invalid vault path');
  const directory = await browserDirectoryHandle(root, parts, false);
  await directory.removeEntry(filename);
}
