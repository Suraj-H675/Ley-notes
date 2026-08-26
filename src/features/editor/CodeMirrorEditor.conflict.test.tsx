import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { makePage, resetDb } from '@/test/helpers';
import { updatePageContent } from '@/core/vault/pages';
import { CodeMirrorEditor } from './CodeMirrorEditor';

let typeInEditor: (value: string) => void;

vi.mock('./lib/mount', async (importOriginal) => {
  const original = await importOriginal<typeof import('./lib/mount')>();
  let onChange: ((value: string) => void) | null = null;
  let value = '';

  const controller = {
    view: {
      contentDOM: document.createElement('div'),
      state: { doc: { lines: 1, line: () => ({ from: 0 }) } },
      dispatch: () => undefined,
      scrollIntoView: () => undefined,
    },
    getValue: () => value,
    setValue: (nextValue: string) => {
      value = nextValue;
      onChange?.(nextValue);
    },
    insertText: (nextValue: string) => {
      value += nextValue;
      onChange?.(value);
    },
    format: () => undefined,
    openSearch: () => undefined,
    setLivePreview: () => undefined,
    focus: () => undefined,
    destroy: () => undefined,
  };

  return {
    ...original,
    mountEditor: vi.fn((_parent: HTMLElement, options: { initialDoc: string; onChange: (value: string) => void }) => {
      value = options.initialDoc;
      onChange = options.onChange;
      return controller;
    }),
    typeInEditor: (nextValue: string) => {
      value = nextValue;
      onChange?.(nextValue);
    },
  };
});

vi.mock('@/core/vault/pages', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/core/vault/pages')>()),
  updatePageContent: vi.fn(async () => undefined),
}));

describe('editor external conflict handling', () => {
  beforeEach(async () => {
    await resetDb();
    vi.mocked(updatePageContent).mockClear();
    await db.pages.put(makePage({ id: 'conflict-page', title: 'Conflict note' }));
    ({ typeInEditor } = (await import('./lib/mount')) as typeof import('./lib/mount') & {
      typeInEditor: (value: string) => void;
    });
  });

  it('autosaves normal edits through the debounced save path', async () => {
    render(
      <CodeMirrorEditor
        pageId="conflict-page"
        pagePath="Conflict note.md"
        initialContent="original"
        pane="primary"
        livePreview={false}
        missingFromDisk={false}
        frontmatterError={undefined}
      />,
    );

    const textarea = document.querySelector('textarea');
    if (textarea) throw new Error('Unexpected fallback editor');
    await act(async () => {
      typeInEditor('my local edit');
    });

    await waitFor(() =>
      expect(updatePageContent).toHaveBeenCalledWith('conflict-page', 'my local edit'),
      { timeout: 2000 },
    );
  });

  it('blocks a pending autosave when the note changes outside Ley', async () => {
    const view = render(
      <CodeMirrorEditor
        pageId="conflict-page"
        pagePath="Conflict note.md"
        initialContent="original"
        pane="primary"
        livePreview={false}
        missingFromDisk={false}
        frontmatterError={undefined}
      />,
    );

    await act(async () => {
      typeInEditor('my unsaved local edit');
    });

    await db.pages.update('conflict-page', { content: 'changed outside Ley' });
    await act(async () => {
      view.rerender(
        <CodeMirrorEditor
          pageId="conflict-page"
          pagePath="Conflict note.md"
          initialContent="changed outside Ley"
          pane="primary"
          livePreview={false}
          missingFromDisk={false}
          frontmatterError={undefined}
        />,
      );
    });

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('This note changed outside Ley while you had unsaved edits.');

    await new Promise((resolve) => setTimeout(resolve, 700));
    expect(updatePageContent).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Keep mine' }));
    await waitFor(() =>
      expect(updatePageContent).toHaveBeenCalledWith('conflict-page', 'my unsaved local edit'),
    );
    await waitFor(() => {
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });
  });

  it('blocks autosave when file is missing from disk', async () => {
    await db.pages.update('conflict-page', { missingFromDisk: true });
    render(
      <CodeMirrorEditor
        pageId="conflict-page"
        pagePath="Conflict note.md"
        initialContent="buffer text"
        pane="primary"
        livePreview={false}
        missingFromDisk
        frontmatterError={undefined}
      />,
    );

    expect(screen.getByText(/deleted outside Ley/i)).toBeVisible();
    expect(screen.getByRole('button', { name: 'Restore to disk' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Close and discard' })).toBeVisible();
  });

  it('replaces a clean editor with disk content and preserves disk on Reload disk', async () => {
    const view = render(
      <CodeMirrorEditor
        pageId="conflict-page"
        pagePath="Conflict note.md"
        initialContent="original"
        pane="primary"
        livePreview={false}
        missingFromDisk={false}
        frontmatterError={undefined}
      />,
    );

    await act(async () => {
      view.rerender(
        <CodeMirrorEditor
          pageId="conflict-page"
          pagePath="Conflict note.md"
          initialContent="clean external update"
          pane="primary"
          livePreview={false}
          missingFromDisk={false}
          frontmatterError={undefined}
        />,
      );
    });

    expect(await screen.findByText('Updated from disk')).toBeInTheDocument();
    expect(updatePageContent).not.toHaveBeenCalled();

    await act(async () => {
      typeInEditor('my local edit');
    });

    await db.pages.update('conflict-page', { content: 'dirty external update' });
    await act(async () => {
      view.rerender(
        <CodeMirrorEditor
          pageId="conflict-page"
          pagePath="Conflict note.md"
          initialContent="dirty external update"
          pane="primary"
          livePreview={false}
          missingFromDisk={false}
          frontmatterError={undefined}
        />,
      );
    });

    await screen.findByRole('alert');

    fireEvent.click(screen.getByRole('button', { name: 'Reload disk' }));

    await waitFor(() => expect(screen.getByText('Reloaded external version')).toBeInTheDocument());
    expect(updatePageContent).not.toHaveBeenCalled();

  });
});
