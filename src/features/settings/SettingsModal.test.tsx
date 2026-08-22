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

import { restoreTrashedFilesystemPage as restoreTrashedFilesystemPageMock } from '@/core/vault/pages';

vi.mock('@/core/vault/templates', () => ({
  listVaultTemplates: vi.fn(async () => []),
}));

vi.mock('dexie-react-hooks', async (importOriginal) => ({
  ...(await importOriginal<typeof import('dexie-react-hooks')>()),
  useLiveQuery: (_query: unknown, fallback?: unknown) => fallback ?? [],
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
});
