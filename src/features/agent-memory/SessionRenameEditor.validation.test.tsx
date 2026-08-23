import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { SessionRenameEditor } from './SessionRenameEditor';
import type { SessionContext } from './types';

const session = {
  sessionId: 'ses_test',
  name: 'Implementation session',
  eventCount: 3,
} as unknown as SessionContext;

describe('session rename validation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('explains missing name and reason without submitting', () => {
    const onDirtyChange = vi.fn();
    render(
      <SessionRenameEditor
        projectPath="/projects/ley"
        session={session}
        onCancel={vi.fn()}
        onDirtyChange={onDirtyChange}
        onRenamed={vi.fn()}
      />,
    );

    const name = screen.getByRole('textbox', { name: 'Session name' });
    fireEvent.change(name, { target: { value: '' } });

    expect(screen.getAllByRole('alert')).toHaveLength(2);
    expect(screen.getByRole('button', { name: 'Append rename' })).toBeDisabled();
    expect(onDirtyChange).toHaveBeenLastCalledWith(true);
  });

  it('clears errors once a valid audited rename is possible', () => {
    const onDirtyChange = vi.fn();
    render(
      <SessionRenameEditor
        projectPath="/projects/ley"
        session={session}
        onCancel={vi.fn()}
        onDirtyChange={onDirtyChange}
        onRenamed={vi.fn()}
      />,
    );

    fireEvent.change(
      screen.getByRole('textbox', { name: 'Session name' }),
      { target: { value: 'Release continuity' } },
    );
    fireEvent.change(
      screen.getByRole('textbox', { name: /Why are you renaming/ }),
      { target: { value: 'The release is complete.' } },
    );

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(onDirtyChange).toHaveBeenLastCalledWith(true);
    expect(
      screen.getByRole('button', { name: 'Append rename' }),
    ).toBeEnabled();
  });
});
