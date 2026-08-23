import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { makePage, resetDb } from '@/test/helpers';
import { SearchModal } from './SearchModal';

const openPage = vi.fn();
const pushRecent = vi.fn();

vi.mock('@/shared/state/nav', () => ({
  useNavStore: (selector: (state: { openPage: unknown; openInSplit: unknown; pushRecent: unknown }) => unknown) =>
    selector({ openPage, openInSplit: vi.fn(), pushRecent }),
}));

vi.mock('@/core/vault/saved-searches', () => ({
  saveSearch: vi.fn(),
}));

describe('quick switcher missing-note guard', () => {
  beforeEach(async () => {
    await resetDb();
    openPage.mockClear();
    pushRecent.mockClear();
    await db.pages.put({
      ...makePage({ id: 'missing-id', title: 'Missing roadmap' }),
      missingFromDisk: true,
    });
  });

  it('does not open a cache-only projection deleted outside Ley', async () => {
    render(
      <SearchModal
        open
        initialQuery="Missing roadmap"
        onClose={vi.fn()}
        onOpenCollection={vi.fn()}
      />,
    );

    const input = await screen.findByPlaceholderText(/Quick switcher/);
    await waitFor(() =>
      expect(screen.queryByText('Missing roadmap')).not.toBeInTheDocument(),
    );
    expect(input).toHaveFocus();

    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() =>
      expect(db.pages.get('missing-id')).resolves.toMatchObject({
        missingFromDisk: true,
      }),
    );
    await waitFor(() => expect(openPage).not.toHaveBeenCalled());
    await waitFor(() => expect(pushRecent).not.toHaveBeenCalled());
  });
});
