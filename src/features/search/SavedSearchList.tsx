import { useRef, useState } from 'react';
import { Pencil, Search, TableProperties, Trash2 } from 'lucide-react';
import { deleteSavedSearch, renameSavedSearch, type SavedSearch } from '@/core/vault/saved-searches';
import { useUIStore } from '@/shared/state/ui';

export function SavedSearchList({ searches, onOpen, onOpenCollection }: { searches: SavedSearch[]; onOpen: (query: string) => void; onOpenCollection: (search: SavedSearch) => void }) {
  function closeMobileSidebar() {
    if (window.matchMedia('(max-width: 767px)').matches) useUIStore.getState().setSidebarOpen(false);
  }
  return <>{searches.map((search) => <SavedSearchRow key={search.id} search={search} onOpen={() => { onOpen(search.query); closeMobileSidebar(); }} onOpenCollection={() => { onOpenCollection(search); closeMobileSidebar(); }} />)}</>;
}

function SavedSearchRow({ search, onOpen, onOpenCollection }: { search: SavedSearch; onOpen: () => void; onOpenCollection: () => void }) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(search.name);
  const [error, setError] = useState<string | null>(null);
  const cancelRename = useRef(false);

  async function commitRename() {
    if (!editing) return;
    if (cancelRename.current) { cancelRename.current = false; return; }
    try {
      await renameSavedSearch(search.id, draft);
      setEditing(false);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  if (editing) {
    return <div className="px-2"><input autoFocus value={draft} onChange={(event) => setDraft(event.target.value)} onBlur={() => void commitRename()} onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); if (event.key === 'Escape') { cancelRename.current = true; setDraft(search.name); setEditing(false); } }} aria-label={`Rename ${search.name}`} className="h-7 w-full rounded-md border border-primary bg-background px-2 text-meta text-foreground outline-none" />{error && <p className="pt-1 text-micro text-destructive" role="alert">{error}</p>}</div>;
  }

  return (
    <div>
      <div className="group flex items-center rounded-sm text-muted-foreground-strong hover:bg-surface-2 hover:text-foreground">
        <button type="button" onClick={onOpen} className="flex min-w-0 flex-1 items-center gap-1.5 px-2 py-1 text-left text-meta" title={search.query}><Search size={10} className="shrink-0 text-secondary" /><span className="truncate">{search.name}</span></button>
        <button type="button" onClick={onOpenCollection} className="rounded p-1 text-muted-foreground opacity-60 hover:bg-surface-3 hover:text-foreground sm:opacity-0 sm:group-hover:opacity-100 sm:focus:opacity-100" aria-label={`Open ${search.name} as table`} title="Open as property table"><TableProperties size={10} /></button>
        <button type="button" onClick={() => { cancelRename.current = false; setEditing(true); }} className="rounded p-1 text-muted-foreground opacity-60 hover:bg-surface-3 hover:text-foreground sm:opacity-0 sm:group-hover:opacity-100 sm:focus:opacity-100" aria-label={`Rename ${search.name}`} title="Rename saved search"><Pencil size={10} /></button>
        <button type="button" onClick={() => { void deleteSavedSearch(search.id).catch((cause) => setError(cause instanceof Error ? cause.message : String(cause))); }} className="mr-1 rounded p-1 text-muted-foreground opacity-60 hover:bg-destructive/10 hover:text-destructive sm:opacity-0 sm:group-hover:opacity-100 sm:focus:opacity-100" aria-label={`Delete ${search.name}`} title="Delete saved search"><Trash2 size={10} /></button>
      </div>
      {error && <p className="px-2 py-1 text-micro text-destructive" role="alert">{error}</p>}
    </div>
  );
}
