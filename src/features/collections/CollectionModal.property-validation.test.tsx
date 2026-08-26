import userEvent from '@testing-library/user-event';
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

    const cell = await screen.findByLabelText('Edit priority property for row typed');
    fireEvent.change(cell, { target: { value: 'soon' } });
    fireEvent.blur(cell);

    expect(screen.getByRole('alert')).toHaveTextContent(/finite number/);
    await waitFor(() =>
      expect(screen.queryByText('priority saved')).not.toBeInTheDocument(),
    );
    expect(updatePageProperty).not.toHaveBeenCalled();
  });

  it('commits two rapid property edits without losing either value or note content', async () => {
    const user = userEvent.setup();
    await db.pages.bulkPut([
      {
        ...makePage({ id: 'first', title: 'First note', content: 'First body' }),
        frontmatter: { status: 'draft' },
      },
      {
        ...makePage({ id: 'second', title: 'Second note', content: 'Second body' }),
        frontmatter: { status: 'review' },
      },
    ]);

    render(
      <CollectionModal
        request={{ query: '', title: 'All notes' }}
        onClose={vi.fn()}
      />,
    );

    const firstCell = await screen.findByLabelText('Edit status property for row second');
    const secondCell = screen.getByLabelText('Edit status property for row first');
    await user.clear(firstCell);
    await user.type(firstCell, 'published{Enter}');
    await user.clear(secondCell);
    await user.type(secondCell, 'approved{Enter}');

    await waitFor(() => {
      expect(updatePageProperty).toHaveBeenNthCalledWith(1, 'second', 'status', 'published');
      expect(updatePageProperty).toHaveBeenNthCalledWith(2, 'first', 'status', 'approved');
    });
    expect(await db.pages.get('first')).toMatchObject({ content: 'First body' });
    expect(await db.pages.get('second')).toMatchObject({ content: 'Second body' });
  });
});
