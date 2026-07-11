/**
 * CodeMirrorEditor — React wrapper around the imperative mount factory.
 * The wiki-link decoration dispatches a `ley:follow-link` CustomEvent; we
 * listen for it once here and resolve-or-create the target.
 */

import { useEffect, useRef } from 'react';
import { mountEditor, type EditorController } from '@/editor/mount';
import { useNavStore } from '@/store/nav';
import { useDebouncedCallback } from '@/hooks/useDebounce';
import { updatePageContent, createPage } from '@/core/vault/pages';
import { resolveTitle, startPageIndex } from '@/core/vault/page-index';

interface CodeMirrorEditorProps {
  pageId: string;
  initialContent: string;
}

async function followOrCreate(target: string): Promise<string> {
  const resolved = await resolveTitle(target);
  if (resolved) return resolved;
  const created = await createPage({ title: target });
  return created.id;
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
      // Autocomplete-panel Enter/Tab accept. Not called on click — clicks
      // dispatch the `ley:follow-link` event handled below.
      onWikiLinkFollow: (target) => {
        followOrCreate(target).then((id) => {
          openPage(id);
          pushRecent(id);
        });
      },
    });

    controllerRef.current = controller;

    const onFollow = async (e: Event) => {
      const ce = e as CustomEvent<{ target: string }>;
      const target = ce.detail?.target;
      if (!target) return;
      const id = await followOrCreate(target);
      openPage(id);
      pushRecent(id);
    };
    controller.view.contentDOM.addEventListener('ley:follow-link', onFollow);

    return () => {
      controller.view.contentDOM.removeEventListener('ley:follow-link', onFollow);
      controller.destroy();
      controllerRef.current = null;
    };
    // initialContent is the doc snapshot when the editor mounts; we want a
    // Fresh editor for each new pageId rather than re-initialising on content
    // changes (those flow through debouncedSave). debouncedSave / openPage /
    // pushRecent are stable (Zustand actions and a useMemo'd debouncer), so
    // re-binding on every change is unnecessary churn.
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