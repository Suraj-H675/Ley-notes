/**
 * CodeMirrorEditor — React wrapper around the imperative mount factory.
 * The wiki-link decoration dispatches a `ley:follow-link` CustomEvent; we
 * listen for it once here and resolve-or-create the target.
 */

import { useEffect, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from 'react';
import { EditorView } from '@codemirror/view';
import { Bold, BookmarkPlus, Braces, Hash, Italic, Link2, ListChecks, Search } from 'lucide-react';
import { startCompletion } from '@codemirror/autocomplete';
import { mountEditor, type EditorController } from '@/features/editor/lib/mount';
import { useDebouncedCallback } from '@/shared/hooks/useDebounce';
import { discardMissingPage, restoreMissingPage, updatePageContent } from '@/core/vault/pages';
import { attachmentInsertion, saveAttachment } from '@/core/vault/attachments';
import type { EditorFormat } from './lib/formatting';
import { openWikiDestination, type WikiDestination } from './lib/open-wiki-destination';
import { openMarkdownDestination } from './lib/open-wiki-destination';
import type { InternalMarkdownLink } from '@/core/parser/markdown-links';
import type { EditorPane } from '@/shared/state/nav';
import { useNavStore } from '@/shared/state/nav';
import { db } from '@/infrastructure/database/db';
import { ensureMarkdownBlockReference } from '@/core/parser/destinations';
import { addDestinationBookmark } from '@/core/vault/bookmarks';
import { nanoid } from '@/shared/lib/nanoid';
import { shortcutLabel } from '@/shared/lib/shortcut';

interface CodeMirrorEditorProps {
  pageId: string;
  pagePath: string;
  initialContent: string;
  pane: EditorPane;
  livePreview: boolean;
  missingFromDisk: boolean;
  frontmatterError?: string;
}

export function CodeMirrorEditor({ pageId, pagePath, initialContent, pane, livePreview, missingFromDisk, frontmatterError }: CodeMirrorEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const controllerRef = useRef<EditorController | null>(null);
  const dirtyRef = useRef(false);
  const syncingRef = useRef(false);
  const externalContentRef = useRef<string | null>(null);
  const acceptedDiskContentRef = useRef(initialContent);
  const pagePathRef = useRef(pagePath);
  const missingFromDiskRef = useRef(missingFromDisk);
  const externalRemovalPendingRef = useRef(false);
  const frontmatterErrorRef = useRef(frontmatterError);
  const [attachmentStatus, setAttachmentStatus] = useState<string | null>(null);
  const [externalContent, setExternalContent] = useState<string | null>(null);
  const [syncStatus, setSyncStatus] = useState<string | null>(null);
  const [recoveryBusy, setRecoveryBusy] = useState(false);

  useLayoutEffect(() => {
    pagePathRef.current = pagePath;
  }, [pagePath]);

  // Save handler — debounced to coalesce keystrokes.
  const debouncedSave = useDebouncedCallback((value: string) => {
    if (externalContentRef.current !== null || missingFromDiskRef.current || externalRemovalPendingRef.current) return;
    updatePageContent(pageId, value).then(() => {
      if (controllerRef.current?.getValue() === value) dirtyRef.current = false;
    }).catch((err) => {
      console.error('[editor] save failed:', err);
      setSyncStatus(err instanceof Error ? err.message : String(err));
    });
  }, 600);

  useEffect(() => {
    if (!containerRef.current) return;

    const controller = mountEditor(containerRef.current, {
      initialDoc: initialContent,
      livePreview,
      onChange: (value) => {
        if (syncingRef.current) return;
        dirtyRef.current = true;
        debouncedSave(value);
      },
    });

    controllerRef.current = controller;

    const onFollow = async (e: Event) => {
      const destination = (e as CustomEvent<WikiDestination>).detail;
      if (!destination?.target) return;
      await openWikiDestination(destination, pane);
    };
    controller.view.contentDOM.addEventListener('ley:follow-link', onFollow);
    const onFollowMarkdown = async (event: Event) => {
      const link = (event as CustomEvent<InternalMarkdownLink>).detail;
      if (!link) return;
      await openMarkdownDestination(pagePathRef.current, link.path, link.heading, link.blockId, pane);
    };
    controller.view.contentDOM.addEventListener('ley:follow-markdown-link', onFollowMarkdown);

    const insertFiles = async (files: File[]) => {
      if (files.length === 0) return;
      if (missingFromDiskRef.current || externalRemovalPendingRef.current) {
        setAttachmentStatus('Restore this note before adding attachments.');
        return;
      }
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
      const detail = (event as CustomEvent<{ text?: string; pageId?: string }>).detail;
      if (detail?.pageId && detail.pageId !== pageId) return;
      const text = detail?.text;
      if (text) controller.insertText(text);
    };
    controller.view.contentDOM.addEventListener('paste', onPaste);
    controller.view.contentDOM.addEventListener('drop', onDrop);
    window.addEventListener('ley:editor-insert', onInsert);
    window.addEventListener('blur', debouncedSave.flush);

    return () => {
      controller.view.contentDOM.removeEventListener('ley:follow-link', onFollow);
      controller.view.contentDOM.removeEventListener('ley:follow-markdown-link', onFollowMarkdown);
      controller.view.contentDOM.removeEventListener('paste', onPaste);
      controller.view.contentDOM.removeEventListener('drop', onDrop);
      window.removeEventListener('ley:editor-insert', onInsert);
      window.removeEventListener('blur', debouncedSave.flush);
      debouncedSave.flush();
      controller.destroy();
      controllerRef.current = null;
    };
    // initialContent is the doc snapshot when the editor mounts; we want a
    // Fresh editor for each new pageId rather than re-initialising on content
    // changes (those flow through debouncedSave). debouncedSave / openPage /
    // navigation and debouncing are stable, so
    // re-binding on every change is unnecessary churn.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pageId]);

  useEffect(() => {
    controllerRef.current?.setLivePreview(livePreview);
  }, [livePreview]);

  useEffect(() => {
    const controller = controllerRef.current;
    if (!controller) return;
    const wasInvalidFrontmatter = Boolean(frontmatterErrorRef.current);
    frontmatterErrorRef.current = frontmatterError;
    // Saving a repaired raw frontmatter source transforms the projection from
    // complete source to body-only. That is a local format transition, not an
    // external conflict; replace the view deliberately and resume normal
    // parsed-frontmatter behavior.
    if (wasInvalidFrontmatter && !frontmatterError) {
      acceptedDiskContentRef.current = initialContent;
      syncingRef.current = true;
      controller.setValue(initialContent);
      syncingRef.current = false;
      dirtyRef.current = false;
      externalContentRef.current = null;
      queueMicrotask(() => setExternalContent(null));
      return;
    }
    const current = controller.getValue();
    if (current === initialContent) {
      acceptedDiskContentRef.current = initialContent;
      dirtyRef.current = false;
      externalContentRef.current = null;
      queueMicrotask(() => setExternalContent(null));
      return;
    }
    if (dirtyRef.current) {
      acceptedDiskContentRef.current = initialContent;
      debouncedSave.cancel();
      externalContentRef.current = initialContent;
      queueMicrotask(() => setExternalContent(initialContent));
      return;
    }
    acceptedDiskContentRef.current = initialContent;
    syncingRef.current = true;
    controller.setValue(initialContent);
    syncingRef.current = false;
    queueMicrotask(() => {
      setSyncStatus('Updated from disk');
      window.setTimeout(() => setSyncStatus(null), 1600);
    });
  }, [debouncedSave, frontmatterError, initialContent]);

  // A filesystem event can arrive before the debounced save is due. Stop
  // autosave immediately so an external edit is never overwritten and an old
  // path is never recreated, then let the authoritative scan resolve it.
  useEffect(() => {
    const onVaultChange = (event: Event) => {
      const detail = (event as CustomEvent<{ fullRescan?: boolean; changes?: Array<{ kind?: string; path?: string; from?: string }> }>).detail;
      const path = pagePathRef.current.toLowerCase();
      const affectsCurrentPath = detail?.fullRescan || detail?.changes?.some((change) => (
        ((change.kind === 'remove' || change.kind === 'modify') && change.path?.toLowerCase() === path)
        || (change.kind === 'rename' && change.from?.toLowerCase() === path)
      ));
      if (!affectsCurrentPath) return;
      externalRemovalPendingRef.current = true;
      debouncedSave.cancel();
    };
    const onProjected = () => {
      void db.pages.get(pageId).then((page) => {
        if (!page || page.missingFromDisk) return;
        externalRemovalPendingRef.current = false;
        // A safe rename or metadata-only filesystem event canceled the pending
        // timer. Resume it for the still-dirty buffer at the now-current path.
        const value = controllerRef.current?.getValue();
        if (
          dirtyRef.current
          && value !== undefined
          && externalContentRef.current === null
          && page.content === acceptedDiskContentRef.current
        ) {
          debouncedSave(value);
        }
      });
    };
    window.addEventListener('ley:vault-files-changed', onVaultChange);
    window.addEventListener('ley:vault-projected', onProjected);
    const flushOnUnload = () => debouncedSave.flush();
    window.addEventListener('beforeunload', flushOnUnload);
    return () => {
      window.removeEventListener('ley:vault-files-changed', onVaultChange);
      window.removeEventListener('ley:vault-projected', onProjected);
      window.removeEventListener('beforeunload', flushOnUnload);
    };
  }, [debouncedSave, pageId]);

  useEffect(() => {
    missingFromDiskRef.current = missingFromDisk;
    if (!missingFromDisk) return;
    externalRemovalPendingRef.current = true;
    debouncedSave.cancel();
    externalContentRef.current = null;
  }, [debouncedSave, missingFromDisk]);

  useEffect(() => {
    const jump = (event: Event) => {
      const { line, pageId: targetPageId } = (event as CustomEvent<{ line: number; pageId?: string }>).detail;
      if (targetPageId && targetPageId !== pageId) return;
      const view = controllerRef.current?.view;
      if (!view || !line || line > view.state.doc.lines) return;
      const target = view.state.doc.line(line);
      view.dispatch({ selection: { anchor: target.from }, effects: EditorView.scrollIntoView(target.from, { y: 'center' }) });
      view.focus();
    };
    window.addEventListener('ley:editor-jump', jump);
    return () => window.removeEventListener('ley:editor-jump', jump);
  }, [pageId]);

  function reloadExternalContent() {
    const controller = controllerRef.current;
    if (!controller || externalContentRef.current === null) return;
    syncingRef.current = true;
    controller.setValue(externalContentRef.current);
    syncingRef.current = false;
    dirtyRef.current = false;
    externalContentRef.current = null;
    setExternalContent(null);
    setSyncStatus('Reloaded external version');
  }

  async function keepLocalContent() {
    const value = controllerRef.current?.getValue();
    if (value === undefined) return;
    externalContentRef.current = null;
    setExternalContent(null);
    setSyncStatus('Saving your version…');
    try {
      await updatePageContent(pageId, value);
      dirtyRef.current = false;
      setSyncStatus('Your version saved');
    } catch (error) {
      setSyncStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function restoreMissingFile() {
    const value = controllerRef.current?.getValue();
    if (value === undefined) return;
    setRecoveryBusy(true);
    try {
      await restoreMissingPage(pageId, value);
      missingFromDiskRef.current = false;
      externalRemovalPendingRef.current = false;
      dirtyRef.current = false;
      setSyncStatus('Restored to disk');
    } catch (error) {
      setSyncStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setRecoveryBusy(false);
    }
  }

  async function discardMissingFile() {
    setRecoveryBusy(true);
    try {
      await discardMissingPage(pageId);
      useNavStore.getState().closeTab(pageId);
    } catch (error) {
      setSyncStatus(error instanceof Error ? error.message : String(error));
      setRecoveryBusy(false);
    }
  }

  async function bookmarkBlockAtCursor() {
    if (missingFromDiskRef.current) {
      setSyncStatus('Restore this note before bookmarking a block.');
      return;
    }
    const view = controllerRef.current?.view;
    if (!view) return;
    try {
      const line = view.state.doc.lineAt(view.state.selection.main.head);
      const result = ensureMarkdownBlockReference(view.state.doc.toString(), line.number, nanoid().slice(0, 8));
      if (result.changed) {
        const nextLine = result.content.split('\n')[line.number - 1];
        view.dispatch({ changes: { from: line.to, insert: nextLine.slice(line.text.length) } });
      }
      await addDestinationBookmark({ kind: 'block', pageId, path: pagePathRef.current, anchor: result.id });
      setSyncStatus(result.changed ? `Block bookmarked · added ^${result.id} to Markdown` : `Block ^${result.id} bookmarked`);
      window.setTimeout(() => setSyncStatus(null), 2400);
    } catch (cause) {
      setSyncStatus(cause instanceof Error ? cause.message : String(cause));
    }
  }

  return (
    <div className="relative flex h-full w-full flex-col">
      <div ref={containerRef} className="min-h-0 w-full flex-1 overflow-hidden bg-background" data-testid="cm-editor" />
      {missingFromDisk && <div className="flex shrink-0 flex-wrap items-center gap-2 border-t border-warning/30 bg-warning/8 px-3 py-2 text-micro text-muted-foreground-strong" role="alert"><span className="flex-1"><strong className="font-medium text-foreground">This file was deleted outside Ley.</strong> The text in this open editor has not been written back. Restore it at its previous path, or close and discard this missing projection.</span><button type="button" onClick={() => void restoreMissingFile()} disabled={recoveryBusy} className="rounded-md bg-primary px-2 py-1 font-medium text-primary-foreground disabled:opacity-60">Restore to disk</button><button type="button" onClick={() => void discardMissingFile()} disabled={recoveryBusy} className="rounded-md border border-border bg-background px-2 py-1 hover:bg-surface-2 disabled:opacity-60">Close and discard</button></div>}
      {externalContent !== null && !missingFromDisk && <div className="flex shrink-0 flex-wrap items-center gap-2 border-t border-warning/30 bg-warning/8 px-3 py-2 text-micro text-muted-foreground-strong" role="alert"><span className="flex-1">This note changed outside Ley while you had unsaved edits.</span><button type="button" onClick={reloadExternalContent} className="rounded-md border border-border bg-background px-2 py-1 hover:bg-surface-2">Reload disk</button><button type="button" onClick={() => void keepLocalContent()} className="rounded-md bg-primary px-2 py-1 font-medium text-primary-foreground">Keep mine</button></div>}
      <div className="app-chrome flex shrink-0 items-center justify-center gap-0.5 p-1" role="toolbar" aria-label="Markdown formatting">
        <FormatButton label="Bold" shortcut={shortcutLabel('B')} format="bold" controller={controllerRef}><Bold size={13} /></FormatButton>
        <FormatButton label="Italic" shortcut={shortcutLabel('I')} format="italic" controller={controllerRef}><Italic size={13} /></FormatButton>
        <FormatButton label="Link note" shortcut={shortcutLabel('K')} format="wiki-link" controller={controllerRef}><Link2 size={13} /></FormatButton>
        <button type="button" onMouseDown={(event) => event.preventDefault()} onClick={() => { const controller = controllerRef.current; if (!controller) return; controller.insertText('#'); startCompletion(controller.view); }} className="flex h-7 items-center gap-1.5 rounded-md px-2 text-micro text-muted-foreground hover:bg-surface-3 hover:text-foreground" aria-label="Add tag" title="Add tag"><Hash size={13} /><span className="hidden sm:inline">Tag</span></button>
        <FormatButton label="Inline code" shortcut={shortcutLabel('Shift `')} format="code" controller={controllerRef}><Braces size={13} /></FormatButton>
        <span className="mx-0.5 h-4 w-px bg-border" />
        <FormatButton label="Cycle task" format="task" controller={controllerRef}><ListChecks size={13} /></FormatButton>
        <button type="button" onMouseDown={(event) => event.preventDefault()} onClick={() => void bookmarkBlockAtCursor()} disabled={missingFromDisk} className="flex h-7 items-center gap-1.5 rounded-md px-2 text-micro text-muted-foreground hover:bg-surface-3 hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50" aria-label="Bookmark block at cursor" title={missingFromDisk ? 'Restore this note before bookmarking a block' : 'Bookmark block at cursor'}><BookmarkPlus size={13} /><span className="hidden lg:inline">Bookmark</span></button>
        <button type="button" onMouseDown={(event) => event.preventDefault()} onClick={() => controllerRef.current?.openSearch()} className="flex h-7 items-center gap-1.5 rounded-md px-2 text-micro text-muted-foreground hover:bg-surface-3 hover:text-foreground" aria-label="Find and replace" title={`Find and replace (${shortcutLabel('F')})`}><Search size={13} /><span className="hidden sm:inline">Find</span></button>
      </div>
      {attachmentStatus && <div className="absolute bottom-12 right-4 max-w-80 rounded-md border border-border bg-surface-1 px-3 py-2 text-meta text-foreground shadow-menu" role="status">{attachmentStatus}</div>}
      {syncStatus && <button type="button" onClick={() => setSyncStatus(null)} className="absolute bottom-12 left-4 max-w-80 rounded-md border border-border bg-surface-1 px-3 py-2 text-left text-meta text-foreground shadow-menu" role="status">{syncStatus}</button>}
    </div>
  );
}

function FormatButton({ label, shortcut, format, controller, children }: { label: string; shortcut?: string; format: EditorFormat; controller: RefObject<EditorController | null>; children: ReactNode }) {
  return <button type="button" onMouseDown={(event) => event.preventDefault()} onClick={() => controller.current?.format(format)} className="flex h-7 items-center gap-1.5 rounded-md px-2 text-micro text-muted-foreground hover:bg-surface-3 hover:text-foreground" aria-label={label} title={shortcut ? `${label} (${shortcut})` : label}>{children}<span className="hidden sm:inline">{label}</span></button>;
}
