import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { makePage, resetDb } from '@/test/helpers';
import { addDestinationBookmark } from '@/core/vault/bookmarks';
import { ensureMarkdownBlockReference } from '@/core/parser/destinations';
import { togglePageBookmark } from '@/core/vault/note-bookmarks';
import { BookmarksPane } from './BookmarksPane';

vi.mock('@/features/search/useSavedSearches', () => ({
  useSavedSearches: () => [],
}));

vi.mock('@/features/search/SavedSearchList', () => ({
  SavedSearchList: () => null,
}));

vi.mock('@/shared/state/ui', () => ({
  useUIStore: { getState: () => ({ setSidebarOpen: vi.fn() }) },
}));

describe('BookmarksPane', () => {
  beforeEach(async () => {
    await resetDb();
    await markActiveDataKind('browser-local');
  });

  it('shows note and anchor bookmarks with custom titles, availability, navigation, rename, and deletion', async () => {
    const page = makePage({
      id: 'fixture-page',
      title: 'Fixture',
      content: '# Alpha\n\nAlpha prose.\n- list item ^alpha-block\n',
    });
    await db.pages.put(page);
    await togglePageBookmark(page.id);
    const heading = await addDestinationBookmark({ kind: 'heading', pageId: page.id, path: page.path, anchor: 'Alpha' }, 'Custom heading');
    await addDestinationBookmark({ kind: 'block', pageId: page.id, path: page.path, anchor: 'alpha-block' });
    const blank = ensureMarkdownBlockReference(page.content, 3, 'blank-id');
    expect(blank.changed).toBe(true);

    let refusal: unknown;
    try {
      ensureMarkdownBlockReference(page.content, 1, 'heading-id');
    } catch (cause) {
      refusal = cause;
    }
    expect(refusal).toBeInstanceOf(Error);

    render(<BookmarksPane onOpenSearch={vi.fn()} onOpenCollection={vi.fn()} />);

    await waitFor(() => expect(screen.getByText('Fixture')).toBeInTheDocument());
    expect(screen.getByText('Custom heading')).toBeInTheDocument();
    expect(await screen.findByText('Fixture › - list item')).toBeInTheDocument();

    const openHeading = screen.getByRole('button', { name: 'Custom heading' });
    await userEvent.click(openHeading);

    const rename = screen.getByRole('button', { name: 'Rename Custom heading' });
    await userEvent.click(rename);
    const input = screen.getByRole('textbox');
    await userEvent.clear(input);
    await userEvent.type(input, 'Renamed heading{Enter}');
    await waitFor(() => expect(screen.getByText('Renamed heading')).toBeInTheDocument());

    await userEvent.click(screen.getByRole('button', { name: 'Delete Renamed heading' }));
    await waitFor(() => expect(screen.queryByText('Renamed heading')).not.toBeInTheDocument());
    expect(heading.id).toBeDefined();
  });
});
