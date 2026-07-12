/**
 * CodeMirrorEditor — React wrapper around the imperative mount factory.
 * The wiki-link decoration dispatches a `ley:follow-link` CustomEvent; we
 * listen for it once here and resolve-or-create the target.
 */

import { useEffect, useRef, useState } from 'react';
import { EditorView } from '@codemirror/view';
import { mountEditor, type EditorController } from '@/features/editor/lib/mount';
import { useNavStore } from '@/shared/state/nav';
import { useDebouncedCallback } from '@/shared/hooks/useDebounce';
import { updatePageContent, createPage } from '@/core/vault/pages';
import { resolveTitle } from '@/core/vault/page-index';
import { attachmentInsertion, saveAttachment } from '@/core/vault/attachments';

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
  const [attachmentStatus, setAttachmentStatus] = useState<string | null>(null);

  const openPage = useNavStore((s) => s.openPage);
  const pushRecent = useNavStore((s) => s.pushRecent);

  // Save handler — debounced to coalesce keystrokes.
  const debouncedSave = useDebouncedCallback((value: string) => {
    updatePageContent(pageId, value).catch((err) => {
      console.error('[editor] save failed:', err);
    });
  }, 600);

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

    const insertFiles = async (files: File[]) => {
      if (files.length === 0) return;
      setAttachmentStatus(files.length === 1 ? 'Adding attachment…' : `Adding ${files.length} attachments…`);
      try {
        const saved = await Promise.all(files.map((file) => saveAttachment(pageId, file)));
        controller.insertText(attachmentInsertion(saved));
        setAttachmentStatus(saved.length === 1 ? 'Attachment added' : `${saved.length} attachments added`);
        window.setTimeout(() => setAttachmentStatus(null), 1800);
      } catch (error) {
        setAttachmentStatus(error instanceof Error ? error.message : String(error));
      }
    };
    const onPaste = (event: ClipboardEvent) => {
      const files = Array.from(event.clipboardData?.files ?? []);
      if (files.length === 0) return;
      event.preventDefault();
      void insertFiles(files);
    };
    const onDrop = (event: DragEvent) => {
      const files = Array.from(event.dataTransfer?.files ?? []);
      if (files.length === 0) return;
      event.preventDefault();
      void insertFiles(files);
    };
    const onInsert = (event: Event) => {
      const text = (event as CustomEvent<{ text?: string }>).detail?.text;
      if (text) controller.insertText(text);
    };
    controller.view.contentDOM.addEventListener('paste', onPaste);
    controller.view.contentDOM.addEventListener('drop', onDrop);
    window.addEventListener('ley:editor-insert', onInsert);
    window.addEventListener('blur', debouncedSave.flush);

    return () => {
      controller.view.contentDOM.removeEventListener('ley:follow-link', onFollow);
      controller.view.contentDOM.removeEventListener('paste', onPaste);
      controller.view.contentDOM.removeEventListener('drop', onDrop);
      window.removeEventListener('ley:editor-insert', onInsert);
      window.removeEventListener('blur', debouncedSave.flush);
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

  useEffect(() => {
    const jump = (event: Event) => {
      const { line } = (event as CustomEvent<{ line: number }>).detail;
      const view = controllerRef.current?.view;
      if (!view || !line || line > view.state.doc.lines) return;
      const target = view.state.doc.line(line);
      view.dispatch({ selection: { anchor: target.from }, effects: EditorView.scrollIntoView(target.from, { y: 'center' }) });
      view.focus();
    };
    window.addEventListener('ley:editor-jump', jump);
    return () => window.removeEventListener('ley:editor-jump', jump);
  }, []);

  return (
    <div className="relative h-full w-full">
      <div ref={containerRef} className="h-full w-full overflow-auto bg-background" data-testid="cm-editor" />
      {attachmentStatus && <div className="absolute bottom-4 right-4 max-w-80 rounded-lg border border-border bg-surface-1 px-3 py-2 text-meta text-foreground shadow-menu" role="status">{attachmentStatus}</div>}
    </div>
  );
}
