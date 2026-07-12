import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { WebVaultLauncher } from './WebVaultLauncher';
import { VaultLauncher } from './VaultLauncher';

describe('vault launcher scroll roots', () => {
  it('keeps the web launcher reachable in short browser viewports', () => {
    const { container } = render(<WebVaultLauncher folderSupported busy={false} error={null} onOpenFolder={vi.fn()} onBrowserLocal={vi.fn()} />);
    const page = container.querySelector('[data-page="web-vault-launcher"]');
    expect(page).toHaveClass('h-full', 'overflow-y-auto');
    expect(page).not.toHaveClass('min-h-full');
  });

  it('keeps native vault onboarding reachable in short windows', () => {
    const { container } = render(<VaultLauncher busy={false} error={null} onOpen={vi.fn()} />);
    const page = container.querySelector('[data-page="desktop-vault-launcher"]');
    expect(page).toHaveClass('h-full', 'overflow-y-auto');
    expect(page).not.toHaveClass('overflow-hidden');
  });
});
