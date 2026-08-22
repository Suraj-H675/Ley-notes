import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { SettingsModal } from './SettingsModal';

vi.mock('@/infrastructure/vault/filesystem-vault', () => ({
  getActiveVaultKind: () => 'desktop',
  listActiveVaultTrash: vi.fn(async () => [
    {
      path: '.trash/projects/Recovered note.md',
      content: '# Recovered',
      createdAt: 1,
      updatedAt: 1,
    },
  ]),
}));

vi.mock('@/core/vault/pages', () => ({
  listDeletedPages: vi.fn(async () => []),
  restorePage: vi.fn(),
  permanentlyDeletePage: vi.fn(),
  restoreTrashedFilesystemPage: vi.fn(async () => ({
    id: 'restored-id',
    title: 'Recovered note',
    path: 'projects/Recovered note.md',
  })),
}));

import { restorePage as restorePageMockImport, restoreTrashedFilesystemPage as restoreTrashedFilesystemPageMock } from '@/core/vault/pages';
import { listDeletedPages } from '@/core/vault/pages';

vi.mock('@/core/vault/templates', () => ({
  listVaultTemplates: vi.fn(async () => []),
}));

vi.mock('dexie-react-hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('dexie-react-hooks')>()),
  useLiveQuery: (query?: unknown, fallback?: unknown) => {
    if (query === listDeletedPages) {
      return [
        { id: 'recycled-id', title: 'Recycled note', path: 'Recycled note.md', deletedAt: 1 },
      ];
    }
    return fallback ?? [];
  },
}));

vi.mock('@/infrastructure/database/db', () => ({
  db: {
    settings: {
      get: vi.fn(async () => undefined),
    },
  },
}));

describe('Settings filesystem trash restore', () => {
  beforeEach(() => {
    vi.mocked(restoreTrashedFilesystemPageMock).mockClear();
    vi.mocked(restorePageMockImport).mockClear();
  });

  it('closes settings and opens the restored note without leaving a status behind the modal', async () => {
    const onRefreshVault = vi.fn(async () => ({ noteCount: 1 }));
    const onOpenNote = vi.fn();
    const onClose = vi.fn();

    render(
      <SettingsModal
        open
        vaultMode="desktop"
        vaultName="Vault"
        watcherStatus="watching"
        onRefreshVault={onRefreshVault}
        onSwitchVault={vi.fn()}
        onClose={onClose}
        onOpenNote={onOpenNote}
      />,
    );

    await screen.findByText('.trash/projects/Recovered note.md');
    fireEvent.click(screen.getByRole('button', { name: /Restore \.trash/ }));

    await waitFor(() => expect(restoreTrashedFilesystemPageMock).toHaveBeenCalledWith('.trash/projects/Recovered note.md'));
    await waitFor(() => expect(onOpenNote).toHaveBeenCalledWith('restored-id'));
    expect(onClose).toHaveBeenCalled();
    expect(onRefreshVault).toHaveBeenCalled();
  });

  it('returns to the editor after restoring a browser-local recycled note', async () => {
    const onRefreshVault = vi.fn(async () => ({ noteCount: 1 }));
    const onOpenNote = vi.fn();
    const onClose = vi.fn();
    vi.mocked(restorePageMockImport).mockResolvedValueOnce({
      id: 'recycled-id',
      lcTitle: 'recycled note',
      title: 'Recycled note',
      path: 'Recycled note.md',
      content: '',
      frontmatter: {},
      aliases: [],
      createdAt: 1,
      updatedAt: 1,
      deletedAt: null,
    });

    render(
      <SettingsModal
        open
        vaultMode="browser-local"
        vaultName="Local"
        watcherStatus="inactive"
        onRefreshVault={onRefreshVault}
        onSwitchVault={vi.fn()}
        onClose={onClose}
        onOpenNote={onOpenNote}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Restore Recycled note' }));

    await waitFor(() => expect(restorePageMockImport).toHaveBeenCalledWith('recycled-id'));
    expect(onOpenNote).toHaveBeenCalledWith('recycled-id');
    expect(onClose).toHaveBeenCalled();
  });
});
