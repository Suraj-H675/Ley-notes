import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { LandingPage } from './LandingPage';

describe('LandingPage', () => {
  it('owns a constrained scroll root while the workspace keeps the document locked', () => {
    const { container } = render(<LandingPage />);
    const page = container.querySelector('[data-page="website"]');
    expect(page).toHaveClass('h-full', 'overflow-y-auto');
    expect(page).not.toHaveClass('min-h-full');
    expect(screen.getByRole('contentinfo')).toHaveTextContent('Local-first by design.');
  });
});
