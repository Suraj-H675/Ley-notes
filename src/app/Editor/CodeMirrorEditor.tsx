/**
 * CodeMirrorEditor — React wrapper around the imperative mount factory.
 * Manages the editor instance lifecycle and routes the `ley:follow-link`
 * custom event from the wiki-link decoration into the nav store.
 */

import { useEffect, useRef } from 'react';
import { mountEditor, type EditorController } from '@/editor/mount';
import { useNavStore } from '@/store/nav';
import { useDebouncedCallback } from '@/hooks/useDebounce';
import { updatePageContent } from '@/core/vault/pages';
import { resolveTitle, startPageIndex } from '@/core/vault/page-index';
import { createPage } from '@/core/vault/pages';

interface CodeMirrorEditorProps {
  pageId: string;
  initialContent: string;
}

export function CodeMirrorEditor({ pageId, initialContent }: CodeMirrorEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const controllerRef = useRef<EditorController | null>(null);

  const openPage = useNavStore((s) => s.openPage);
  const pushRecent = useNavStore((s) => s.pushRecent);

  // Save handler — debounced to coalesce keystrokes.
  const debouncedSave = useDebouncedCallback((value: string) => {
    updatePageContent(pageId, value).catch((err) => {
      console.error('[editor] save failed:', err);
    });
  }, 600);

  // Start the page index bridge once. The WikiLink autocomplete reads from
  // it synchronously inside CM's keydown handler.
  useEffect(() => {
    return startPageIndex();
  }, []);

  useEffect(() => {
    if (!containerRef.current) return;

    const controller = mountEditor(containerRef.current, {
      initialDoc: initialContent,
      onChange: debouncedSave,
      onWikiLinkFollow: async (target) => {
        // If the target resolves, navigate; otherwise create the page first.
        const resolved = await resolveTitle(target);
        if (resolved) {
          openPage(resolved);
          pushRecent(resolved);
        } else {
          const created = await createPage({ title: target });
          openPage(created.id);
          pushRecent(created.id);
        }
      },
    });

    controllerRef.current = controller;

    // Listen for the `ley:follow-link` event the decoration plugin dispatches.
    const onFollow = async (e: Event) => {
      const ce = e as CustomEvent<{ target: string }>;
      const target = ce.detail?.target;
      if (!target) return;
      const resolved = await resolveTitle(target);
      if (resolved) {
        openPage(resolved);
        pushRecent(resolved);
      } else {
        const created = await createPage({ title: target });
        openPage(created.id);
        pushRecent(created.id);
      }
    };
    controller.view.contentDOM.addEventListener('ley:follow-link', onFollow);

    return () => {
      controller.view.contentDOM.removeEventListener('ley:follow-link', onFollow);
      controller.destroy();
      controllerRef.current = null;
    };
    // initialContent is the doc snapshot when the editor mounts; we want a
    // fresh editor for each new pageId rather than re-initialising on content
    // changes (those flow through debouncedSave).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pageId]);

  return (
    <div
      ref={containerRef}
      className="h-full w-full overflow-auto bg-background"
      data-testid="cm-editor"
    />
  );
}