import { describe, it, expect, beforeEach } from 'vitest';
import 'fake-indexeddb/auto';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { db } from '@/lib/db';
import { GroupsSection } from './GroupsSection';

describe('GroupsSection', () => {
  beforeEach(async () => {
    await db.graphSettings.clear();
  });

  it('renders all five color schemes', async () => {
    render(<GroupsSection scope="global" />);
    await waitFor(() => {
      expect(screen.getByText('Untyped')).toBeInTheDocument();
    });
    expect(screen.getByText('Tag')).toBeInTheDocument();
    expect(screen.getByText('Collection')).toBeInTheDocument();
    expect(screen.getByText('Links')).toBeInTheDocument();
    expect(screen.getByText('Community')).toBeInTheDocument();
  });

  it('updates colorScheme in Dexie when a scheme is clicked', async () => {
    const user = userEvent.setup();
    render(<GroupsSection scope="global" />);
    await waitFor(() => {
      expect(screen.getByText('Tag')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Tag'));
    await waitFor(async () => {
      const row = await db.graphSettings.get('global');
      expect(row?.colorScheme).toBe('tag');
    });
  });
});
