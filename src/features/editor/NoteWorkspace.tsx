import { lazy, Suspense, useEffect, useRef, useState, type ReactNode } from 'react';
import { BookOpen, Bookmark, Code2, Eye, FileText, Paperclip, TriangleAlert } from 'lucide-react';
import type { Page } from '@/infrastructure/database/schema';
import { renamePage } from '@/core/vault/pages';
import { PropertiesPanel } from './PropertiesPanel';
import { attachmentInsertion, saveAttachment } from '@/core/vault/attachments';
import { togglePageBookmark } from '@/core/vault/note-bookmarks';
import { useIsPageBookmarked } from '@/features/bookmarks/useNoteBookmarks';
import type { EditorPane } from '@/shared/state/nav';

const MarkdownReadingView = lazy(() => import('./MarkdownReadingView').then((module) => ({ default: module.MarkdownReadingView })));
const CodeMirrorEditor = lazy(() => import('./CodeMirrorEditor').then((module) => ({ default: module.CodeMirrorEditor })));

type EditorMode = 'edit' | 'read';
type EditingStyle = 'live' | 'source';

function storedEditorMode(): EditorMode {
  return localStorage.getItem('ley:editor-mode') === 'read' ? 'read' : 'edit';
}

function storedEditingStyle(): EditingStyle {
  return localStorage.getItem('ley:editor-style') === 'source' ? 'source' : 'live';
}

export function NoteWorkspace({ page, pane }: { page: Page; pane: EditorPane }) {
  const [mode, setMode] = useState<EditorMode>(storedEditorMode);
  const [editingStyle, setEditingStyle] = useState<EditingStyle>(storedEditingStyle);
  const [title, setTitle] = useState(page.title);
  const [titleError, setTitleError] = useState<string | null>(null);
  const [attachmentStatus, setAttachmentStatus] = useState<string | null>(null);
  const attachmentInput = useRef<HTMLInputElement>(null);
  const bookmarked = useIsPageBookmarked(page.id);
  const invalidFrontmatter = Boolean(page.frontmatterError);
  const missingFromDisk = Boolean(page.missingFromDisk);
  const visibleMode = invalidFrontmatter || missingFromDisk ? 'edit' : mode;
  const visibleEditingStyle = invalidFrontmatter ? 'source' : editingStyle;

  async function commitTitle() {
    const next = title.trim();
    if (!next || next === page.title) { setTitle(page.title); return; }
    try {
      await renamePage(page.id, next);
      setTitleError(null);
    } catch (error) {
      setTitle(page.title);
      setTitleError(error instanceof Error ? error.message : String(error));
    }
  }

  function changeMode(next: EditorMode) {
    if (invalidFrontmatter && next !== 'edit') return;
    setMode(next);
    localStorage.setItem('ley:editor-mode', next);
  }

  function changeEditingStyle(next: EditingStyle) {
    if (invalidFrontmatter && next !== 'source') return;
    setEditingStyle(next);
    localStorage.setItem('ley:editor-style', next);
    changeMode('edit');
  }

  // A filesystem rename can update the visible filename title without changing
  // this workspace's page ID. Mirror that projection after paint so a stale
  // input value cannot accidentally rename the file back on blur.
  useEffect(() => {
    const timer = window.setTimeout(() => setTitle(page.title), 0);
    return () => window.clearTimeout(timer);
  }, [page.id, page.title]);

  async function attachFiles(files: File[]) {
    if (files.length === 0) return;
    setAttachmentStatus('Adding…');
    try {
      const saved = await Promise.all(files.map((file) => saveAttachment(page.id, file)));
      setMode('edit');
      localStorage.setItem('ley:editor-mode', 'edit');
      window.setTimeout(() => window.dispatchEvent(new CustomEvent('ley:editor-insert', {
        detail: { text: attachmentInsertion(saved), pageId: page.id },
      })), 80);
      setAttachmentStatus(saved.length === 1 ? 'Added' : `${saved.length} added`);
      window.setTimeout(() => setAttachmentStatus(null), 1800);
    } catch (error) {
      setAttachmentStatus(error instanceof Error ? error.message : String(error));
    }
  }

  useEffect(() => {
    const jump = (event: Event) => {
      const detail = (event as CustomEvent<{ line: number; pageId?: string }>).detail;
      if (detail.pageId && detail.pageId !== page.id) return;
      setMode('edit');
      localStorage.setItem('ley:editor-mode', 'edit');
      window.setTimeout(() => window.dispatchEvent(new CustomEvent('ley:editor-jump', { detail: { ...detail, pageId: page.id } })), 80);
    };
    window.addEventListener('ley:outline-jump', jump);
    return () => window.removeEventListener('ley:outline-jump', jump);
  }, [page.id]);

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-background">
      <div className="flex shrink-0 items-center gap-1.5 border-b border-border px-2 py-2 sm:gap-3 sm:px-4">
        <FileText size={14} className="hidden shrink-0 text-muted-foreground sm:block" />
        <div className="min-w-0 flex-1">
          <input
            value={title}
            disabled={missingFromDisk}
            onChange={(event) => setTitle(event.target.value)}
            onBlur={() => void commitTitle()}
            onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); if (event.key === 'Escape') { setTitle(page.title); event.currentTarget.blur(); } }}
            className="w-full bg-transparent text-body font-semibold text-foreground outline-none disabled:cursor-not-allowed disabled:text-muted-foreground"
            aria-label="Note title"
          />
          <div className={`truncate font-mono text-micro ${titleError ? 'text-destructive' : 'text-subtle-foreground'}`}>{titleError ?? page.path}</div>
        </div>
        <input ref={attachmentInput} type="file" multiple accept="image/png,image/jpeg,image/gif,image/webp,application/pdf,audio/mpeg,audio/wav,video/mp4,video/webm" className="hidden" onChange={(event) => { void attachFiles(Array.from(event.target.files ?? [])); event.target.value = ''; }} />
        <button type="button" onClick={() => void togglePageBookmark(page.id)} disabled={missingFromDisk} className={`rounded-md p-1.5 disabled:cursor-not-allowed disabled:opacity-50 ${bookmarked ? 'text-secondary' : 'text-muted-foreground hover:bg-surface-2 hover:text-foreground'}`} aria-label={missingFromDisk ? 'Restore this note before bookmarking it' : (bookmarked ? 'Remove note bookmark' : 'Bookmark note')} title={missingFromDisk ? 'Restore this note before bookmarking it' : (bookmarked ? 'Remove note bookmark' : 'Bookmark note')} aria-pressed={bookmarked}>
          <Bookmark size={13} className={bookmarked ? 'fill-current' : undefined} />
        </button>
        <button type="button" onClick={() => attachmentInput.current?.click()} disabled={missingFromDisk} className="flex shrink-0 items-center gap-1 rounded-md p-1.5 text-micro text-muted-foreground hover:bg-surface-2 hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50 sm:px-2 sm:py-1" title={missingFromDisk ? 'Restore this note before adding attachments' : (attachmentStatus ?? 'Attach files')} aria-label={missingFromDisk ? 'Restore this note before adding attachments' : (attachmentStatus ?? 'Attach files')}>
          <Paperclip size={12} /> <span className="hidden sm:inline">{attachmentStatus ?? 'Attach'}</span>
        </button>
        <div className="flex rounded-md border border-border bg-surface-1 p-0.5">
          <ModeButton active={visibleMode === 'edit' && visibleEditingStyle === 'live'} onClick={() => changeEditingStyle('live')} icon={<Eye size={12} />} label="Live preview" />
          <ModeButton active={visibleMode === 'edit' && visibleEditingStyle === 'source'} onClick={() => changeEditingStyle('source')} icon={<Code2 size={12} />} label="Source" />
          <ModeButton active={visibleMode === 'read'} onClick={() => changeMode('read')} icon={<BookOpen size={12} />} label="Read" />
        </div>
      </div>
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
        {invalidFrontmatter || missingFromDisk
          ? <div className="mx-auto w-full max-w-[820px] px-4 pt-4 sm:px-10 sm:pt-5"><div className="flex gap-2 rounded-md border border-warning/35 bg-warning/10 px-3 py-2 text-meta text-muted-foreground-strong" role="alert"><TriangleAlert size={15} className="mt-0.5 shrink-0 text-warning" aria-hidden="true" /><div>{invalidFrontmatter ? <><p className="font-medium text-foreground">Properties are unavailable until this frontmatter is fixed.</p><p>Its original YAML is shown verbatim in Source mode and will not be rewritten by Ley.</p><p className="sr-only">Parser detail: {page.frontmatterError}</p></> : <><p className="font-medium text-foreground">Properties are unavailable while this file is missing.</p><p>Restore the note to disk or close and discard its recovery copy.</p></>}</div></div></div>
          : <PropertiesPanel key={JSON.stringify(page.frontmatter)} pageId={page.id} frontmatter={page.frontmatter} />}
        {visibleMode === 'edit'
          ? <div className="mx-auto min-h-[240px] w-full max-w-[900px] flex-1"><Suspense fallback={<div className="p-10 text-meta text-muted-foreground">Opening editor…</div>}><CodeMirrorEditor pageId={page.id} pagePath={page.path} initialContent={page.content} pane={pane} livePreview={visibleEditingStyle === 'live'} missingFromDisk={missingFromDisk} frontmatterError={page.frontmatterError} /></Suspense></div>
          : <Suspense fallback={<div className="p-10 text-meta text-muted-foreground">Rendering note…</div>}><MarkdownReadingView pageId={page.id} pagePath={page.path} content={page.content} pane={pane} /></Suspense>}
      </div>
    </div>
  );
}

function ModeButton({ active, onClick, icon, label }: { active: boolean; onClick: () => void; icon: ReactNode; label: string }) {
  return <button type="button" onClick={onClick} aria-label={`${label} mode`} title={`${label} mode`} className={`flex items-center gap-1 rounded p-1.5 text-micro sm:px-2 sm:py-1 ${active ? 'bg-surface-3 text-foreground' : 'text-muted-foreground hover:text-foreground'}`}>{icon}<span className="hidden sm:inline">{label}</span></button>;
}
