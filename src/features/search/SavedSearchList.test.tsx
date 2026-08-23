import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { markActiveDataKind } from '@/infrastructure/database/browser-local-vault';
import { resetDb } from '@/test/helpers';
import { saveSearch } from '@/core/vault/saved-searches';
import { useSavedSearches } from './useSavedSearches';
import { SavedSearchList } from './SavedSearchList';

function TestList() {
  const searches = useSavedSearches();
  return <SavedSearchList searches={searches} onOpen={vi.fn()} onOpenCollection={vi.fn()} />;
}

describe('saved search rows', () => {
  beforeEach(async () => {
    await resetDb();
    await markActiveDataKind('browser-local');
  });

  it('supports rename recovery and reactive deletion', async () => {
    const saved = await saveSearch('Research queue', 'tag:research');

    render(<TestList />);

    fireEvent.click(await screen.findByRole('button', { name: 'Rename Research queue' }));
    const input = screen.getByLabelText('Rename Research queue');
    fireEvent.change(input, { target: { value: '' } });
    fireEvent.blur(input);

    await waitFor(() =>
      expect(
        screen.getByRole('alert'),
      ).toHaveTextContent(/name/i),
    );

    fireEvent.change(screen.getByLabelText('Rename Research queue'), {
      target: { value: 'Active research' },
    });
    fireEvent.keyDown(screen.getByLabelText('Rename Research queue'), {
      key: 'Enter',
    });

    expect(await screen.findByRole('button', { name: 'Active research' })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: 'Delete Active research' }));

    await waitFor(() =>
      expect(screen.queryByRole('button', { name: /Active research|Delete Active research/ })).not.toBeInTheDocument(),
    );
    expect(saved.id).toBeTruthy();
  });
});
