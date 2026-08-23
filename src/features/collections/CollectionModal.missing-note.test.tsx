import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { makePage, resetDb } from '@/test/helpers';
import { CollectionModal } from './CollectionModal';

const openPage = vi.fn();
const openInSplit = vi.fn();
const pushRecent = vi.fn();

vi.mock('@/shared/state/nav', () => ({
  useNavStore: (
    selector: (state: {
      openPage: unknown;
      openInSplit: unknown;
      pushRecent: unknown;
    }) => unknown,
  ) => selector({ openPage, openInSplit, pushRecent }),
}));

vi.mock('@/core/vault/pages', () => ({
  updatePageProperty: vi.fn(),
}));

describe('collection missing-note guard', () => {
  beforeEach(async () => {
    await resetDb();
    for (const mock of [openPage, openInSplit, pushRecent]) mock.mockClear();
    await db.pages.put({
      ...makePage({ id: 'missing-id', title: 'Missing roadmap' }),
      missingFromDisk: true,
    });
  });

  it('does not navigate to an externally deleted projection', async () => {
    render(
      <CollectionModal
        request={{ query: 'Missing roadmap', title: 'Query collection' }}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByText('Missing roadmap'));

    await waitFor(() => expect(openPage).not.toHaveBeenCalled());
    await waitFor(() => expect(openInSplit).not.toHaveBeenCalled());
    await waitFor(() => expect(pushRecent).not.toHaveBeenCalled());
  });
});
