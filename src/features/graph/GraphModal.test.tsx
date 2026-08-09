import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GraphModal } from './GraphModal';

vi.mock('./useGraphData', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./useGraphData')>();
  return { ...actual, useGraphData: () => null };
});

describe('GraphModal narrow controls', () => {
  beforeEach(() => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn((query: string) => ({
        matches: query === '(max-width: 767px)',
        media: query,
        onchange: null,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
        addListener: () => undefined,
        removeListener: () => undefined,
        dispatchEvent: () => true,
      })),
    );
  });

  afterEach(() => vi.unstubAllGlobals());

  it('keeps closed controls out of the focus tree and gives Escape a layered dismissal', async () => {
    const onClose = vi.fn();
    render(<GraphModal open onClose={onClose} />);

    const toggle = screen.getByRole('button', { name: 'Controls' });
    const panel = document.getElementById('graph-controls-panel');
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(panel).toHaveAttribute('aria-hidden', 'true');
    expect(panel).toHaveAttribute('inert');
    expect(screen.queryByRole('textbox', { name: 'Search' })).not.toBeInTheDocument();

    fireEvent.click(toggle);
    const search = await screen.findByRole('textbox', { name: 'Search' });
    expect(toggle).toHaveAttribute('aria-expanded', 'true');
    expect(panel).toHaveAttribute('aria-hidden', 'false');
    expect(panel).not.toHaveAttribute('inert');
    await waitFor(() => expect(search).toHaveFocus());

    fireEvent.keyDown(search, { key: 'Escape', code: 'Escape' });
    await waitFor(() => expect(toggle).toHaveAttribute('aria-expanded', 'false'));
    expect(onClose).not.toHaveBeenCalled();
    expect(toggle).toHaveFocus();

    fireEvent.keyDown(toggle, { key: 'Escape', code: 'Escape' });
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
  });
});
