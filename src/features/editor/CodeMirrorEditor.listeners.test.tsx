import { render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { CodeMirrorEditor } from './CodeMirrorEditor';

const contentDOM = document.createElement('div');

vi.mock('./lib/mount', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/mount')>()),
  mountEditor: vi.fn(() => {
    return {
      view: {
        contentDOM,
        state: { doc: { lines: 1, line: () => ({ from: 0 }) } },
        dispatch: () => undefined,
      },
      getValue: () => '',
      setValue: () => undefined,
      insertText: () => undefined,
      format: () => undefined,
      openSearch: () => undefined,
      setLivePreview: () => undefined,
      focus: () => undefined,
      destroy: () => undefined,
    };
  }),
}));

describe('editor follow-link listeners', () => {
  beforeEach(() => {
    contentDOM.replaceChildren();
    vi.restoreAllMocks();
  });

  it('removes both wiki and Markdown follow-link listeners on unmount', () => {
    const removedEvents: string[] = [];
    const originalRemove = contentDOM.removeEventListener.bind(contentDOM);
    vi.spyOn(contentDOM, 'removeEventListener').mockImplementation((type, listener, options) => {
      removedEvents.push(type);
      originalRemove(type, listener, options);
    });

    const { unmount } = render(
      <CodeMirrorEditor
        pageId="listener-page"
        pagePath="Listener.md"
        initialContent=""
        pane="primary"
        livePreview={false}
        missingFromDisk={false}
        frontmatterError={undefined}
      />,
    );

    unmount();
    expect(removedEvents).toContain('ley:follow-link');
    expect(removedEvents).toContain('ley:follow-markdown-link');
  });
});
