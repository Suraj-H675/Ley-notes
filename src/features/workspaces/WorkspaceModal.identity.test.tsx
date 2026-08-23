import { act } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  markActiveDataKind,
} from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';
import { resetDb } from '@/test/helpers';
import type { NavigationLayout } from '@/core/vault/navigation-session';
import {
  saveWorkspaceLayout,
  workspaceLayoutsDataKey,
} from '@/core/vault/workspace-layouts';

const navigation: NavigationLayout = {
  openTabs: [{ id: 'note', path: 'Note.md' }],
  activeTab: { id: 'note', path: 'Note.md' },
  primaryTab: { id: 'note', path: 'Note.md' },
  secondaryTab: null,
  activePane: 'primary',
};

vi.mock('@/shared/state/ui', () => ({
  useUIStore: {
    getState: () => ({
      sidebarOpen: true,
      rightDockOpen: false,
      rightDockTab: 'outline',
    }),
  },
}));

describe('workspace layout vault identity', () => {
  beforeEach(async () => {
    await resetDb();
    await markActiveDataKind('browser-local');
    await saveWorkspaceLayout('Local workspace', navigation, {
      sidebarOpen: true,
      rightDockOpen: false,
      rightDockTab: 'outline',
      splitPercent: 50,
    });
  });

  it('isolates saved layouts under a new active identity', async () => {
    const local = await db.settings.get(workspaceLayoutsDataKey('browser-local'));
    expect(local?.value).toHaveLength(1);

    await act(async () => markActiveDataKind('filesystem:/vault/next'));

    const next = await db.settings.get(
      workspaceLayoutsDataKey('filesystem:/vault/next'),
    );
    expect(next?.value ?? []).toEqual([]);
  });
});
