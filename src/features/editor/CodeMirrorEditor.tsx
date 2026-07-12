/**
 * CodeMirrorEditor — React wrapper around the imperative mount factory.
 * The wiki-link decoration dispatches a `ley:follow-link` CustomEvent; we
 * listen for it once here and resolve-or-create the target.
 */

import { useEffect, useRef, useState, type ReactNode, type RefObject } from 'react';
import { EditorView } from '@codemirror/view';
import { Bold, Braces, Italic, Link2, ListChecks } from 'lucide-react';
import { mountEditor, type EditorController } from '@/features/editor/lib/mount';
import { useNavStore } from '@/shared/state/nav';
import { useDebouncedCallback } from '@/shared/hooks/useDebounce';
import { updatePageContent, createPage } from '@/core/vault/pages';
import { resolveTitle } from '@/core/vault/page-index';
import { attachmentInsertion, saveAttachment } from '@/core/vault/attachments';
import type { EditorFormat } from './lib/formatting';

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
    <div className="relative flex h-full w-full flex-col">
      <div ref={containerRef} className="min-h-0 w-full flex-1 overflow-hidden bg-background" data-testid="cm-editor" />
      <div className="flex shrink-0 items-center justify-center gap-0.5 border-t border-border bg-surface-1/95 p-1 backdrop-blur" role="toolbar" aria-label="Markdown formatting">
        <FormatButton label="Bold" shortcut="⌘B" format="bold" controller={controllerRef}><Bold size={13} /></FormatButton>
        <FormatButton label="Italic" shortcut="⌘I" format="italic" controller={controllerRef}><Italic size={13} /></FormatButton>
        <FormatButton label="Link note" shortcut="⌘K" format="wiki-link" controller={controllerRef}><Link2 size={13} /></FormatButton>
        <FormatButton label="Inline code" shortcut="⌘⇧`" format="code" controller={controllerRef}><Braces size={13} /></FormatButton>
        <span className="mx-0.5 h-4 w-px bg-border" />
        <FormatButton label="Cycle task" format="task" controller={controllerRef}><ListChecks size={13} /></FormatButton>
      </div>
      {attachmentStatus && <div className="absolute bottom-12 right-4 max-w-80 rounded-lg border border-border bg-surface-1 px-3 py-2 text-meta text-foreground shadow-menu" role="status">{attachmentStatus}</div>}
    </div>
  );
}

function FormatButton({ label, shortcut, format, controller, children }: { label: string; shortcut?: string; format: EditorFormat; controller: RefObject<EditorController | null>; children: ReactNode }) {
  return <button type="button" onMouseDown={(event) => event.preventDefault()} onClick={() => controller.current?.format(format)} className="flex h-7 items-center gap-1.5 rounded-lg px-2 text-micro text-muted-foreground hover:bg-surface-3 hover:text-foreground" aria-label={label} title={shortcut ? `${label} (${shortcut})` : label}>{children}<span className="hidden sm:inline">{label}</span></button>;
}
