import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { makePage, resetDb } from '@/test/helpers';
import { PropertiesPanel } from './PropertiesPanel';
import { updatePageProperty } from '@/core/vault/pages';

vi.mock('@/core/vault/pages', () => ({
  removePageProperty: vi.fn(),
  updatePageProperty: vi.fn(async () => undefined),
}));

describe('properties panel validation', () => {
  beforeEach(async () => {
    await resetDb();
    vi.mocked(updatePageProperty).mockClear();
    await db.pages.put({
      ...makePage({ id: 'typed', title: 'Typed properties' }),
      frontmatter: { priority: 3, status: 'active', tags: ['one', 'two'] },
    });
  });

  it('blocks invalid numeric edits and does not call the vault write path', async () => {
    render(<PropertiesPanel pageId="typed" frontmatter={{ priority: 3 }} />);

    const input = screen.getByLabelText('priority');
    fireEvent.change(input, { target: { value: 'soon' } });
    fireEvent.blur(input);

    expect(screen.getByRole('alert')).toHaveTextContent(/finite number/);
    await waitFor(() =>
      expect(
        db.pages.where('id').equals('typed').first(),
      ).resolves.toMatchObject({ frontmatter: { priority: 3 } }),
    );
    expect(updatePageProperty).not.toHaveBeenCalled();
  });

  it('commits a valid numeric edit with its original YAML type', async () => {
    render(<PropertiesPanel pageId="typed" frontmatter={{ priority: 3 }} />);

    fireEvent.change(screen.getByLabelText('priority'), {
      target: { value: '18' },
    });
    fireEvent.blur(screen.getByLabelText('priority'));

    await waitFor(() =>
      expect(updatePageProperty).toHaveBeenCalledWith('typed', 'priority', 18),
    );
  });

  it('rejects unsafe new property names before creation', async () => {
    render(<PropertiesPanel pageId="typed" frontmatter={{}} />);

    fireEvent.click(screen.getByRole('button', { name: 'Add property' }));
    const keyInput = screen.getByLabelText('Property name');
    fireEvent.change(keyInput, { target: { value: 'bad:name' } });
    fireEvent.keyDown(keyInput, { key: 'Enter' });

    expect(screen.getByRole('alert')).toHaveTextContent(/portable property/);
    expect(updatePageProperty).not.toHaveBeenCalled();
  });
});
