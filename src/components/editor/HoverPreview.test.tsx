import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import 'fake-indexeddb/auto';
import { db } from '@/lib/db';
import { HoverPreview } from './HoverPreview';

describe('HoverPreview', () => {
  beforeEach(async () => {
    await db.nodes.clear();
  });

  it('renders nothing when anchor is null', () => {
    const { container } = render(<HoverPreview anchor={null} title={null} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders the target note title and content preview', async () => {
    await db.nodes.put({
      id: 'n1',
      type: 'document',
      title: 'React patterns',
      content: 'Some content about React patterns and design.\n\nMore text.',
      plainText: 'Some content about React patterns and design. More text.',
      collections: [],
      tags: [],
      properties: {},
      isArchived: 0,
      isPinned: 0,
      createdAt: 0,
      updatedAt: 0,
    });

    const anchor = { x: 100, y: 200, width: 0, height: 0 };
    render(<HoverPreview anchor={anchor} title="React patterns" />);

    await waitFor(() => {
      expect(screen.getByText('React patterns')).toBeInTheDocument();
    });
    expect(screen.getByText(/Some content about React patterns/)).toBeInTheDocument();
  });

  it('shows a "not found" message when the target note does not exist', async () => {
    const anchor = { x: 100, y: 200, width: 0, height: 0 };
    render(<HoverPreview anchor={anchor} title="Nonexistent" />);

    await waitFor(() => {
      expect(screen.getByText(/not found/i)).toBeInTheDocument();
    });
  });

  it('truncates the preview to roughly 200 characters', async () => {
    const long = 'a'.repeat(500);
    await db.nodes.put({
      id: 'n2',
      type: 'document',
      title: 'Long',
      content: long,
      plainText: long,
      collections: [],
      tags: [],
      properties: {},
      isArchived: 0,
      isPinned: 0,
      createdAt: 0,
      updatedAt: 0,
    });

    const anchor = { x: 0, y: 0, width: 0, height: 0 };
    render(<HoverPreview anchor={anchor} title="Long" />);

    await waitFor(() => {
      expect(screen.getByText(/Long/)).toBeInTheDocument();
    });
    // The preview should be truncated; the full 500 'a's should NOT appear.
    expect(document.body.textContent).not.toContain('a'.repeat(300));
  });
});
