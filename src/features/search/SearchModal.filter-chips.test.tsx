import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SearchModal } from './SearchModal';

describe('quick switcher filter chips', () => {
  it('discovers task operators and inserts the exact filter', () => {
    const onClose = vi.fn();
    render(
      <SearchModal
        open
        onClose={onClose}
        onOpenCollection={vi.fn()}
      />,
    );

    expect(screen.getByRole('button', { name: /To do task-todo:/ })).toBeVisible();
    expect(screen.getByRole('button', { name: /Done task-done:/ })).toBeVisible();

    fireEvent.click(screen.getByRole('button', { name: /To do task-todo:/ }));
    expect(screen.getByRole('textbox')).toHaveValue('task-todo:');
  });
});
