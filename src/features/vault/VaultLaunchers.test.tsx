import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { VaultLauncher } from './VaultLauncher';

describe('native vault launcher scroll root', () => {
  it('keeps native vault onboarding reachable in short windows', () => {
    const { container } = render(<VaultLauncher busy={false} error={null} onOpen={vi.fn()} />);
    const page = container.querySelector('[data-page="desktop-vault-launcher"]');
    expect(page).toHaveClass('h-full', 'overflow-y-auto');
    expect(page).not.toHaveClass('overflow-hidden');
  });
});
