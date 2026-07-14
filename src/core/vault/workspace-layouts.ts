import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';
import { nanoid } from '@/shared/lib/nanoid';
import type { NavigationLayout, PageReference } from './navigation-session';

export type WorkspaceDockTab = 'graph' | 'backlinks' | 'outline' | 'history';

export interface WorkspaceShellLayout {
  sidebarOpen: boolean;
  rightDockOpen: boolean;
  rightDockTab: WorkspaceDockTab;
  splitPercent: number;
}

export interface NamedWorkspace {
  id: string;
  name: string;
  navigation: NavigationLayout;
  shell: WorkspaceShellLayout;
  createdAt: number;
  updatedAt: number;
}

const WORKSPACES_PREFIX = 'workspace-layouts:';
const MAX_WORKSPACES = 30;

export function workspaceLayoutsDataKey(kind: string | null): string {
  return `${WORKSPACES_PREFIX}${kind ?? 'unselected'}`;
}

export async function workspaceLayoutsKey(): Promise<string> {
  return workspaceLayoutsDataKey(await activeDataKind());
}

export async function listWorkspaceLayouts(): Promise<NamedWorkspace[]> {
  return listAtKey(await workspaceLayoutsKey());
}

export function parseWorkspaceLayoutsSetting(value: unknown): NamedWorkspace[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter(isNamedWorkspace)
    .map((workspace) => ({
      ...workspace,
      navigation: normalizeNavigation(workspace.navigation),
      shell: normalizeShell(workspace.shell),
    }))
    .sort((left, right) => right.updatedAt - left.updatedAt)
    .slice(0, MAX_WORKSPACES);
}

export async function saveWorkspaceLayout(
  name: string,
  navigation: NavigationLayout,
  shell: WorkspaceShellLayout,
): Promise<NamedWorkspace> {
  const cleanName = validateName(name);
  const key = await workspaceLayoutsKey();
  const current = await listAtKey(key);
  const existing = current.find((item) => item.name.localeCompare(cleanName, undefined, { sensitivity: 'accent' }) === 0);
  const now = Date.now();
  const workspace: NamedWorkspace = existing
    ? { ...existing, name: cleanName, navigation: normalizeNavigation(navigation), shell: normalizeShell(shell), updatedAt: now }
    : { id: nanoid(), name: cleanName, navigation: normalizeNavigation(navigation), shell: normalizeShell(shell), createdAt: now, updatedAt: now };
  await writeAtKey(key, [workspace, ...current.filter((item) => item.id !== workspace.id)].slice(0, MAX_WORKSPACES));
  return workspace;
}

export async function replaceWorkspaceLayout(
  id: string,
  navigation: NavigationLayout,
  shell: WorkspaceShellLayout,
): Promise<void> {
  const key = await workspaceLayoutsKey();
  const current = await listAtKey(key);
  if (!current.some((item) => item.id === id)) return;
  await writeAtKey(key, current.map((item) => item.id === id
    ? { ...item, navigation: normalizeNavigation(navigation), shell: normalizeShell(shell), updatedAt: Date.now() }
    : item));
}

export async function renameWorkspaceLayout(id: string, name: string): Promise<void> {
  const cleanName = validateName(name);
  const key = await workspaceLayoutsKey();
  const current = await listAtKey(key);
  if (!current.some((item) => item.id === id)) return;
  const duplicate = current.some((item) => item.id !== id && item.name.localeCompare(cleanName, undefined, { sensitivity: 'accent' }) === 0);
  if (duplicate) throw new Error('A workspace with this name already exists.');
  await writeAtKey(key, current.map((item) => item.id === id ? { ...item, name: cleanName, updatedAt: Date.now() } : item));
}

export async function deleteWorkspaceLayout(id: string): Promise<void> {
  const key = await workspaceLayoutsKey();
  const current = await listAtKey(key);
  await writeAtKey(key, current.filter((item) => item.id !== id));
}

async function listAtKey(key: string): Promise<NamedWorkspace[]> {
  return parseWorkspaceLayoutsSetting((await db.settings.get(key))?.value);
}

async function writeAtKey(key: string, workspaces: NamedWorkspace[]): Promise<void> {
  await db.settings.put({ key, value: workspaces });
}

function validateName(name: string): string {
  const value = name.trim();
  if (!value) throw new Error('Give this workspace a name.');
  if (value.length > 80) throw new Error('Workspace names must be 80 characters or fewer.');
  return value;
}

function normalizeNavigation(navigation: NavigationLayout): NavigationLayout {
  return {
    openTabs: navigation.openTabs.filter(isPageReference),
    activeTab: isPageReference(navigation.activeTab) ? navigation.activeTab : null,
    primaryTab: isPageReference(navigation.primaryTab) ? navigation.primaryTab : null,
    secondaryTab: isPageReference(navigation.secondaryTab) ? navigation.secondaryTab : null,
    activePane: navigation.activePane === 'secondary' ? 'secondary' : 'primary',
  };
}

function normalizeShell(shell: WorkspaceShellLayout): WorkspaceShellLayout {
  const splitPercent = Number.isFinite(shell.splitPercent) ? Math.max(28, Math.min(72, shell.splitPercent)) : 50;
  return {
    sidebarOpen: Boolean(shell.sidebarOpen),
    rightDockOpen: Boolean(shell.rightDockOpen),
    rightDockTab: isDockTab(shell.rightDockTab) ? shell.rightDockTab : 'backlinks',
    splitPercent,
  };
}

function isNamedWorkspace(value: unknown): value is NamedWorkspace {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<NamedWorkspace>;
  return typeof candidate.id === 'string'
    && typeof candidate.name === 'string'
    && candidate.name.trim().length > 0
    && candidate.name.length <= 80
    && typeof candidate.createdAt === 'number'
    && typeof candidate.updatedAt === 'number'
    && isNavigationLayout(candidate.navigation)
    && isShellLayout(candidate.shell);
}

function isNavigationLayout(value: unknown): value is NavigationLayout {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<NavigationLayout>;
  return Array.isArray(candidate.openTabs)
    && candidate.openTabs.every(isPageReference)
    && (candidate.activeTab === null || isPageReference(candidate.activeTab))
    && (candidate.primaryTab === null || isPageReference(candidate.primaryTab))
    && (candidate.secondaryTab === null || isPageReference(candidate.secondaryTab))
    && (candidate.activePane === 'primary' || candidate.activePane === 'secondary');
}

function isShellLayout(value: unknown): value is WorkspaceShellLayout {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<WorkspaceShellLayout>;
  return typeof candidate.sidebarOpen === 'boolean'
    && typeof candidate.rightDockOpen === 'boolean'
    && isDockTab(candidate.rightDockTab)
    && typeof candidate.splitPercent === 'number';
}

function isPageReference(value: unknown): value is PageReference {
  return Boolean(value && typeof value === 'object'
    && typeof (value as PageReference).id === 'string'
    && typeof (value as PageReference).path === 'string');
}

function isDockTab(value: unknown): value is WorkspaceDockTab {
  return value === 'graph' || value === 'backlinks' || value === 'outline' || value === 'history';
}
