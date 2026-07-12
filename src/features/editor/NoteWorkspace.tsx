import { lazy, Suspense, useEffect, useRef, useState, type ReactNode } from 'react';
import { BookOpen, Edit3, FileText, Paperclip } from 'lucide-react';
import type { Page } from '@/infrastructure/database/schema';
import { renamePage } from '@/core/vault/pages';
import { PropertiesPanel } from './PropertiesPanel';
import { attachmentInsertion, saveAttachment } from '@/core/vault/attachments';

const MarkdownReadingView = lazy(() => import('./MarkdownReadingView').then((module) => ({ default: module.MarkdownReadingView })));
const CodeMirrorEditor = lazy(() => import('./CodeMirrorEditor').then((module) => ({ default: module.CodeMirrorEditor })));

type EditorMode = 'edit' | 'read';

export function NoteWorkspace({ page }: { page: Page }) {
  const [mode, setMode] = useState<EditorMode>(() => (localStorage.getItem('ley:editor-mode') as EditorMode) || 'edit');
  const [title, setTitle] = useState(page.title);
  const [titleError, setTitleError] = useState<string | null>(null);
  const [attachmentStatus, setAttachmentStatus] = useState<string | null>(null);
  const attachmentInput = useRef<HTMLInputElement>(null);

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
    setMode(next);
    localStorage.setItem('ley:editor-mode', next);
  }

  async function attachFiles(files: File[]) {
    if (files.length === 0) return;
    setAttachmentStatus('Adding…');
    try {
      const saved = await Promise.all(files.map((file) => saveAttachment(page.id, file)));
      setMode('edit');
      localStorage.setItem('ley:editor-mode', 'edit');
      window.setTimeout(() => window.dispatchEvent(new CustomEvent('ley:editor-insert', {
        detail: { text: attachmentInsertion(saved) },
      })), 80);
      setAttachmentStatus(saved.length === 1 ? 'Added' : `${saved.length} added`);
      window.setTimeout(() => setAttachmentStatus(null), 1800);
    } catch (error) {
      setAttachmentStatus(error instanceof Error ? error.message : String(error));
    }
  }

  useEffect(() => {
    const jump = (event: Event) => {
      const detail = (event as CustomEvent<{ line: number }>).detail;
      setMode('edit');
      localStorage.setItem('ley:editor-mode', 'edit');
      window.setTimeout(() => window.dispatchEvent(new CustomEvent('ley:editor-jump', { detail })), 80);
    };
    window.addEventListener('ley:outline-jump', jump);
    return () => window.removeEventListener('ley:outline-jump', jump);
  }, []);

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-background">
      <div className="flex shrink-0 items-center gap-3 border-b border-border px-4 py-2">
        <FileText size={14} className="text-muted-foreground" />
        <div className="min-w-0 flex-1">
          <input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            onBlur={() => void commitTitle()}
            onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); if (event.key === 'Escape') { setTitle(page.title); event.currentTarget.blur(); } }}
            className="w-full bg-transparent text-body font-semibold text-foreground outline-none"
            aria-label="Note title"
          />
          <div className={`truncate font-mono text-micro ${titleError ? 'text-destructive' : 'text-subtle-foreground'}`}>{titleError ?? page.path}</div>
        </div>
        <input ref={attachmentInput} type="file" multiple accept="image/png,image/jpeg,image/gif,image/webp,application/pdf,audio/mpeg,audio/wav,video/mp4,video/webm" className="hidden" onChange={(event) => { void attachFiles(Array.from(event.target.files ?? [])); event.target.value = ''; }} />
        <button type="button" onClick={() => attachmentInput.current?.click()} className="flex items-center gap-1 rounded-md px-2 py-1 text-micro text-muted-foreground hover:bg-surface-2 hover:text-foreground" title="Attach files">
          <Paperclip size={12} /> {attachmentStatus ?? 'Attach'}
        </button>
        <div className="flex rounded-md border border-border bg-surface-1 p-0.5">
          <ModeButton active={mode === 'edit'} onClick={() => changeMode('edit')} icon={<Edit3 size={12} />} label="Edit" />
          <ModeButton active={mode === 'read'} onClick={() => changeMode('read')} icon={<BookOpen size={12} />} label="Read" />
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <PropertiesPanel key={JSON.stringify(page.frontmatter)} pageId={page.id} frontmatter={page.frontmatter} />
        {mode === 'edit'
          ? <div className="mx-auto h-[calc(100vh-175px)] min-h-[500px] w-full max-w-[900px]"><Suspense fallback={<div className="p-10 text-meta text-muted-foreground">Opening editor…</div>}><CodeMirrorEditor pageId={page.id} initialContent={page.content} /></Suspense></div>
          : <Suspense fallback={<div className="p-10 text-meta text-muted-foreground">Rendering note…</div>}><MarkdownReadingView content={page.content} /></Suspense>}
      </div>
    </div>
  );
}

function ModeButton({ active, onClick, icon, label }: { active: boolean; onClick: () => void; icon: ReactNode; label: string }) {
  return <button type="button" onClick={onClick} className={`flex items-center gap-1 rounded px-2 py-1 text-micro ${active ? 'bg-surface-3 text-foreground' : 'text-muted-foreground hover:text-foreground'}`}>{icon}{label}</button>;
}
