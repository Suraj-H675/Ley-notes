import { beforeEach, describe, expect, it } from 'vitest';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';
import { resetDb } from '@/test/helpers';
import type { NavigationLayout } from './navigation-session';
import {
  deleteWorkspaceLayout,
  listWorkspaceLayouts,
  parseWorkspaceLayoutsSetting,
  renameWorkspaceLayout,
  replaceWorkspaceLayout,
  saveWorkspaceLayout,
  workspaceLayoutsDataKey,
} from './workspace-layouts';

const firstNavigation: NavigationLayout = {
  openTabs: [{ id: 'first', path: 'First.md' }],
  activeTab: { id: 'first', path: 'First.md' },
  primaryTab: { id: 'first', path: 'First.md' },
  secondaryTab: null,
  activePane: 'primary',
};

const splitNavigation: NavigationLayout = {
  openTabs: [{ id: 'first', path: 'First.md' }, { id: 'second', path: 'Second.md' }],
  activeTab: { id: 'second', path: 'Second.md' },
  primaryTab: { id: 'first', path: 'First.md' },
  secondaryTab: { id: 'second', path: 'Second.md' },
  activePane: 'secondary',
};

const shell = { sidebarOpen: true, rightDockOpen: false, rightDockTab: 'outline' as const, splitPercent: 50 };

describe('named workspace layouts', () => {
  beforeEach(async () => {
    await resetDb();
    await markActiveDataKind('browser-local');
  });

  it('creates a workspace and updates the same normalized name in place', async () => {
    const created = await saveWorkspaceLayout(' Writing ', firstNavigation, shell);
    const updated = await saveWorkspaceLayout('writing', splitNavigation, { ...shell, splitPercent: 61 });
    expect(updated.id).toBe(created.id);
    expect(updated.createdAt).toBe(created.createdAt);
    expect(await listWorkspaceLayouts()).toEqual([expect.objectContaining({
      id: created.id,
      name: 'writing',
      navigation: splitNavigation,
      shell: expect.objectContaining({ splitPercent: 61 }),
      updatedAt: expect.any(Number),
    })]);
  });

  it('replaces, renames, and deletes a saved layout', async () => {
    const workspace = await saveWorkspaceLayout('Research', firstNavigation, shell);
    await replaceWorkspaceLayout(workspace.id, splitNavigation, { ...shell, rightDockOpen: true });
    await renameWorkspaceLayout(workspace.id, 'Deep research');
    expect(await listWorkspaceLayouts()).toEqual([expect.objectContaining({
      name: 'Deep research',
      navigation: splitNavigation,
      shell: expect.objectContaining({ rightDockOpen: true }),
    })]);
    await deleteWorkspaceLayout(workspace.id);
    expect(await listWorkspaceLayouts()).toEqual([]);
  });

  it('rejects empty and duplicate rename targets', async () => {
    const first = await saveWorkspaceLayout('Writing', firstNavigation, shell);
    await saveWorkspaceLayout('Research', splitNavigation, shell);
    await expect(renameWorkspaceLayout(first.id, ' research ')).rejects.toThrow('already exists');
    await expect(saveWorkspaceLayout('  ', firstNavigation, shell)).rejects.toThrow('Give this workspace');
  });

  it('isolates layouts by vault identity', async () => {
    await markActiveDataKind('filesystem:/vault/a');
    await saveWorkspaceLayout('Vault A', firstNavigation, shell);
    await markActiveDataKind('filesystem:/vault/b');
    expect(await listWorkspaceLayouts()).toEqual([]);
    expect((await db.settings.get(workspaceLayoutsDataKey('filesystem:/vault/a')))?.value).toHaveLength(1);
  });

  it('ignores malformed records and clamps saved shell dimensions', async () => {
    const workspace = await saveWorkspaceLayout('Writing', firstNavigation, { ...shell, splitPercent: 100 });
    expect(workspace.shell.splitPercent).toBe(72);
    expect(parseWorkspaceLayoutsSetting([workspace, { id: 'broken' }, null])).toEqual([workspace]);
    expect(parseWorkspaceLayoutsSetting('broken')).toEqual([]);
  });
});
