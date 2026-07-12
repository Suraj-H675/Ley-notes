/**
 * Desktop vault bridge.
 *
 * Markdown files on disk are authoritative. Dexie contains a rebuildable
 * projection used by the React UI, search, backlinks, tags, and graph.
 */

import { invoke, isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { db } from '@/infrastructure/database/db';
import type { Page } from '@/infrastructure/database/schema';
import { parseFrontmatter, getAliases } from '@/core/parser/frontmatter';
import { rebuildPageLinks } from '@/core/index/backlink';
import { rebuildPageTags } from '@/core/index/tag-index';
import { activeDataKind, filesystemDataKind, markActiveDataKind } from '@/infrastructure/database/browser-local-vault';

const LAST_VAULT_KEY = 'ley:last-filesystem-vault';
const BROWSER_HANDLE_KEY = 'browser-directory-handle';

export interface VaultFileSnapshot {
  path: string;
  content: string;
  createdAt: number;
  updatedAt: number;
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
  await db.settings.put({ key: BROWSER_HANDLE_KEY, value: handle });
  return loadBrowserFolderVault(handle);
}

export async function restoreBrowserFolderVault(): Promise<DesktopVault | null> {
  if (!isBrowserFolderSupported()) return null;
  const row = await db.settings.get(BROWSER_HANDLE_KEY);
  const handle = row?.value as LeyDirectoryHandle | undefined;
  if (!handle || handle.kind !== 'directory') return null;
  if (await handle.queryPermission({ mode: 'readwrite' }) !== 'granted') return null;
  return loadBrowserFolderVault(handle);
}

export async function refreshBrowserFolderVault(): Promise<DesktopVault | null> {
  return activeBrowserHandle ? loadBrowserFolderVault(activeBrowserHandle) : null;
}

export async function refreshDesktopVault(): Promise<DesktopVault | null> {
  if (!activeVaultPath) return null;
  return loadDesktopVault(activeVaultPath);
}

async function loadDesktopVault(path: string): Promise<DesktopVault> {
  const snapshots = await invoke<VaultFileSnapshot[]>('scan_vault', { vaultPath: path });
  activeVaultPath = path;
  localStorage.setItem(LAST_VAULT_KEY, path);
  await projectFilesIntoCache(path, snapshots);
  return {
    path,
    name: path.split(/[\\/]/).filter(Boolean).at(-1) ?? 'Vault',
    noteCount: snapshots.length,
  };
}

async function loadBrowserFolderVault(handle: LeyDirectoryHandle): Promise<DesktopVault> {
  const snapshots = await scanBrowserFolder(handle);
  activeBrowserHandle = handle;
  activeVaultPath = null;
  await projectFilesIntoCache(`browser-folder:${handle.name}`, snapshots);
  return { path: handle.name, name: handle.name, noteCount: snapshots.length };
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
        snapshots.push({ path, content: await file.text(), createdAt: file.lastModified, updatedAt: file.lastModified });
      }
    }
  }
  await walk(root, '');
  return snapshots.sort((left, right) => left.path.localeCompare(right.path));
}

export async function projectFilesIntoCache(vaultPath: string, snapshots: VaultFileSnapshot[]): Promise<void> {
  const kind = filesystemDataKind(vaultPath);
  const previousKind = await activeDataKind();
  const sameVault = previousKind === kind;
  const existingByPath = sameVault
    ? new Map((await db.pages.toArray()).map((page) => [page.path.toLowerCase(), page]))
    : new Map<string, Page>();
  const pages = snapshots.map((snapshot) => pageFromSnapshot(vaultPath, snapshot, existingByPath.get(snapshot.path.toLowerCase())));

  await db.transaction('rw', [db.pages, db.blocks, db.links, db.tags, db.assets, db.revisions], async () => {
    await Promise.all([db.pages.clear(), db.blocks.clear(), db.links.clear(), db.tags.clear()]);
    if (!sameVault) await Promise.all([db.assets.clear(), db.revisions.clear()]);
    if (pages.length > 0) await db.pages.bulkPut(pages);
  });
  await markActiveDataKind(kind);

  // Resolve links only after every page exists in the cache.
  for (const page of pages) {
    await rebuildPageLinks(page.id, page.content);
    await rebuildPageTags(page.id, page.content, page.frontmatter);
  }
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
    aliases: getAliases(parsed.frontmatter),
    createdAt: existing?.createdAt ?? (snapshot.createdAt || snapshot.updatedAt || Date.now()),
    updatedAt: snapshot.updatedAt || Date.now(),
    deletedAt: null,
  };
}

function stableFileId(vaultPath: string, path: string): string {
  const input = `${vaultPath}\0${path.toLowerCase()}`;
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
