import { useEffect, useRef, useState } from 'react';
import { FilePlus2, Files, Folder } from 'lucide-react';
import { useLiveQuery } from 'dexie-react-hooks';
import { createPage } from '@/core/vault/pages';
import { useNavStore } from '@/shared/state/nav';
import { Kbd } from '@/shared/components/Kbd';
import { getActiveVaultKind } from '@/infrastructure/vault/filesystem-vault';
import { applyTemplate, listVaultTemplates, templateFrontmatter } from '@/core/vault/templates';

export function NewNoteModal({
  open,
  initialFolder,
  onClose,
}: {
  open: boolean;
  initialFolder: string;
  onClose: () => void;
}) {
  const [title, setTitle] = useState('');
  const [folder, setFolder] = useState(initialFolder);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [templateId, setTemplateId] = useState('');
  const titleRef = useRef<HTMLInputElement>(null);
  const templates = useLiveQuery(listVaultTemplates, [], []);

  useEffect(() => {
    if (!open) return;
    queueMicrotask(() => {
      setTitle('');
      setFolder(initialFolder);
      setError(null);
      setTemplateId('');
      titleRef.current?.focus();
    });
  }, [initialFolder, open]);

  if (!open) return null;

  async function submit() {
    const cleanTitle = title.trim();
    if (!cleanTitle) { setError('Give the note a title.'); return; }
    setBusy(true);
    setError(null);
    try {
      const template = templates.find((candidate) => candidate.id === templateId);
      const page = await createPage({
        title: cleanTitle,
        folder: folder.trim().replace(/^\/+|\/+$/g, '') || undefined,
        content: template ? applyTemplate(template.content, { title: cleanTitle }) : '',
        frontmatter: template ? templateFrontmatter(template) : undefined,
      });
      const nav = useNavStore.getState();
      nav.openPage(page.id);
      nav.pushRecent(page.id);
      onClose();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div role="dialog" aria-modal="true" aria-label="Create new note" className="fixed inset-0 z-[75] flex items-start justify-center bg-background/65 pt-[16vh]" onMouseDown={onClose}>
      <form className="w-[460px] max-w-[calc(100vw-24px)] rounded-xl border border-border bg-surface-1 shadow-menu" onMouseDown={(event) => event.stopPropagation()} onSubmit={(event) => { event.preventDefault(); void submit(); }}>
        <div className="flex items-center gap-2 border-b border-border px-4 py-3 font-medium"><FilePlus2 size={16} className="text-primary" />New note</div>
        <div className="space-y-4 p-4">
          <label className="block text-meta text-muted-foreground-strong">
            Title
            <input ref={titleRef} value={title} onChange={(event) => setTitle(event.target.value)} placeholder="Untitled note" className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 text-foreground outline-none focus:border-primary" />
          </label>
          <label className="block text-meta text-muted-foreground-strong">
            <span className="flex items-center gap-1"><Files size={13} />Template <span className="text-muted-foreground">(optional)</span></span>
            <select value={templateId} onChange={(event) => setTemplateId(event.target.value)} className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 text-meta text-foreground outline-none focus:border-primary">
              <option value="">Blank note</option>
              {templates.map((template) => <option key={template.id} value={template.id}>{template.title}</option>)}
            </select>
            {templates.length === 0 && <span className="mt-1 block text-micro text-muted-foreground">Add Markdown files to the templates folder to use them here.</span>}
          </label>
          <label className="block text-meta text-muted-foreground-strong">
            <span className="flex items-center gap-1"><Folder size={13} />Folder <span className="text-muted-foreground">(optional)</span></span>
            <input value={folder} onChange={(event) => setFolder(event.target.value)} placeholder="e.g. projects/ley" className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 font-mono text-meta text-foreground outline-none focus:border-primary" />
          </label>
          {error && <div className="rounded-md bg-destructive/10 px-3 py-2 text-meta text-destructive">{error}</div>}
        </div>
        <div className="flex items-center justify-between border-t border-border px-4 py-3">
          <span className="text-micro text-muted-foreground">{getActiveVaultKind() ? 'Creates a real Markdown file' : 'Saved in this browser vault'}</span>
          <div className="flex items-center gap-2"><button type="button" onClick={onClose} className="rounded-md px-3 py-1.5 text-meta text-muted-foreground hover:bg-surface-2">Cancel</button><button type="submit" disabled={busy} className="rounded-md bg-primary px-3 py-1.5 text-meta font-medium text-primary-foreground disabled:opacity-50">{busy ? 'Creating…' : 'Create note'} <Kbd>↵</Kbd></button></div>
        </div>
      </form>
    </div>
  );
}
