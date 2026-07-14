import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { LandingPage } from './LandingPage';

describe('LandingPage', () => {
  it('owns a constrained scroll root while the workspace keeps the document locked', () => {
    const { container } = render(<LandingPage />);
    const page = container.querySelector('[data-page="website"]');
    expect(page).toHaveClass('min-h-screen');
    expect(page).not.toHaveClass('h-full', 'overflow-y-auto');
    expect(screen.getByRole('contentinfo')).toHaveTextContent('Local-first by design.');
  });
});
