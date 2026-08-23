import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { makePage, resetDb } from '@/test/helpers';
import { updatePageProperty } from '@/core/vault/pages';
import { CollectionModal } from './CollectionModal';

const nav = {
  openPage: vi.fn(),
  openInSplit: vi.fn(),
  pushRecent: vi.fn(),
};

vi.mock('@/shared/state/nav', () => ({
  useNavStore: (
    selector: (state: {
      openPage: unknown;
      openInSplit: unknown;
      pushRecent: unknown;
    }) => unknown,
  ) => selector(nav),
}));

vi.mock('@/core/vault/pages', () => ({
  updatePageProperty: vi.fn(async () => undefined),
}));

describe('collection property validation', () => {
  beforeEach(async () => {
    await resetDb();
    for (const mock of Object.values(nav)) mock.mockClear();
    await db.pages.put({
      ...makePage({ id: 'typed', title: 'Typed note' }),
      frontmatter: { priority: 4 },
    });
  });

  it('blocks invalid numeric edits before the YAML write path', async () => {
    render(
      <CollectionModal
        request={{ query: '', title: 'All notes' }}
        onClose={vi.fn()}
      />,
    );

    const cell = await screen.findByLabelText('Edit priority property');
    fireEvent.change(cell, { target: { value: 'soon' } });
    fireEvent.blur(cell);

    expect(screen.getByRole('alert')).toHaveTextContent(/finite number/);
    await waitFor(() =>
      expect(screen.queryByText('priority saved')).not.toBeInTheDocument(),
    );
    expect(updatePageProperty).not.toHaveBeenCalled();
  });
});
