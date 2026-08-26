import { useRef, useState } from 'react';
import { Bookmark, ChevronDown, ChevronRight, FileText, Hash, Pencil, TextQuote, Trash2 } from 'lucide-react';
import { deleteDestinationBookmark, renameDestinationBookmark, type DestinationBookmark } from '@/core/vault/bookmarks';
import type { SavedSearch } from '@/core/vault/saved-searches';
import { openPageDestination } from '@/features/editor/lib/open-wiki-destination';
import { useBookmarkedPages } from './useNoteBookmarks';
import { SavedSearchList } from '@/features/search/SavedSearchList';
import { useSavedSearches } from '@/features/search/useSavedSearches';
import { useUIStore } from '@/shared/state/ui';
import { useNavStore } from '@/shared/state/nav';
import { cn } from '@/shared/lib/classnames';
import { useDestinationBookmarks, type ResolvedDestinationBookmark } from './useBookmarks';
import { extractMarkdownBlockReferences } from '@/core/parser/destinations';

export function BookmarksPane({ onOpenSearch, onOpenCollection }: { onOpenSearch: (query: string) => void; onOpenCollection: (search: SavedSearch) => void }) {
  const [expanded, setExpanded] = useState(true);
  const pages = useBookmarkedPages();
  const destinations = useDestinationBookmarks();
  const searches = useSavedSearches();
  const activeTab = useNavStore((state) => state.activeTab);
  const count = pages.length + destinations.length + searches.length;

  function closeMobileSidebar() {
    if (window.matchMedia('(max-width: 767px)').matches) useUIStore.getState().setSidebarOpen(false);
  }

  function openPage(pageId: string) {
    const nav = useNavStore.getState();
    nav.openPage(pageId);
    nav.pushRecent(pageId);
    closeMobileSidebar();
  }

  return (
    <div className="flex flex-col gap-1 px-2">
      <button type="button" onClick={() => setExpanded((value) => !value)} className="flex items-center gap-1.5 rounded-sm px-2 py-1 text-meta font-medium uppercase tracking-[0.08em] text-subtle-foreground hover:bg-surface-2 hover:text-foreground" aria-expanded={expanded}>
        {expanded ? <ChevronDown size={11} /> : <ChevronRight size={11} />}<Bookmark size={12} className="fill-current" /><span>Bookmarks</span><span className="ml-auto text-micro text-subtle-foreground">{count}</span>
      </button>
      {expanded && count === 0 && <p className="px-2 py-1 text-micro leading-relaxed text-muted-foreground">Bookmark a note, heading, block, or useful search.</p>}
      {expanded && pages.length > 0 && <BookmarkSection label="Notes">
        {pages.map((page) => <button key={page.id} type="button" onClick={() => openPage(page.id)} className={cn('flex w-full items-center gap-1.5 rounded-sm px-2 py-1 text-left text-meta', activeTab === page.id ? 'bg-surface-3 text-foreground' : 'text-muted-foreground-strong hover:bg-surface-2 hover:text-foreground')}><FileText size={10} className="shrink-0 text-secondary" /><span className="truncate">{page.title}</span></button>)}
      </BookmarkSection>}
      {expanded && destinations.length > 0 && <BookmarkSection label="Anchors">
        {destinations.map((resolved) => <DestinationRow key={resolved.bookmark.id} resolved={resolved} onOpen={async () => {
          if (!resolved.page || !resolved.destinationAvailable) return;
          await openPageDestination(
            resolved.page.id,
            resolved.bookmark.target.kind === 'heading' ? resolved.bookmark.target.anchor : null,
            resolved.bookmark.target.kind === 'block' ? resolved.bookmark.target.anchor : null,
          );
          closeMobileSidebar();
        }} />)}
      </BookmarkSection>}
      {expanded && searches.length > 0 && <BookmarkSection label="Searches">
        <SavedSearchList searches={searches} onOpen={onOpenSearch} onOpenCollection={onOpenCollection} />
      </BookmarkSection>}
    </div>
  );
}

function BookmarkSection({ label, children }: { label: string; children: React.ReactNode }) {
  return <section className="mt-1"><h3 className="px-2 pb-0.5 text-micro font-medium uppercase tracking-[0.12em] text-subtle-foreground">{label}</h3>{children}</section>;
}

function DestinationRow({ resolved, onOpen }: { resolved: ResolvedDestinationBookmark; onOpen: () => void | Promise<void> }) {
  const { bookmark, page, destinationAvailable } = resolved;
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(bookmark.title ?? '');
  const [error, setError] = useState<string | null>(null);
  const cancelRename = useRef(false);
  const automaticLabel = destinationLabel(bookmark, page);
  const currentLabel = bookmark.title ?? automaticLabel;

  async function commitRename() {
    if (!editing) return;
    if (cancelRename.current) { cancelRename.current = false; return; }
    try {
      await renameDestinationBookmark(bookmark.id, draft);
      setEditing(false);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  if (editing) return <div className="px-2 py-0.5"><input autoFocus value={draft} placeholder={currentLabel} onChange={(event) => setDraft(event.target.value)} onBlur={() => void commitRename()} onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); if (event.key === 'Escape') { cancelRename.current = true; setDraft(bookmark.title ?? ''); setEditing(false); } }} aria-label={`Rename ${currentLabel}`} className="h-7 w-full rounded-md border border-primary bg-background px-2 text-meta text-foreground outline-none" />{error && <p className="pt-1 text-micro text-destructive" role="alert">{error}</p>}</div>;

  const Icon = bookmark.target.kind === 'heading' ? Hash : TextQuote;
  return <div><div className={cn('group flex items-center rounded-sm', destinationAvailable ? 'text-muted-foreground-strong hover:bg-surface-2 hover:text-foreground' : 'text-subtle-foreground')}>
    <button type="button" onClick={() => void onOpen()} disabled={!destinationAvailable} className="flex min-w-0 flex-1 items-center gap-1.5 px-2 py-1 text-left text-meta disabled:cursor-not-allowed" title={destinationAvailable ? `${bookmark.target.path}#${bookmark.target.kind === 'block' ? '^' : ''}${bookmark.target.anchor}` : `Unavailable: ${bookmark.target.path}#${bookmark.target.anchor}`}><Icon size={10} className="shrink-0 text-secondary" /><span className="truncate">{bookmark.title ?? automaticLabel}</span></button>
    <button type="button" onClick={() => { cancelRename.current = false; setEditing(true); }} className="rounded p-1 text-muted-foreground opacity-60 hover:bg-surface-3 hover:text-foreground sm:opacity-0 sm:group-hover:opacity-100 sm:focus:opacity-100" aria-label={`Rename ${currentLabel}`}><Pencil size={10} /></button>
    <button type="button" onClick={() => void deleteDestinationBookmark(bookmark.id).catch((cause) => setError(cause instanceof Error ? cause.message : String(cause)))} className="mr-1 rounded p-1 text-muted-foreground opacity-60 hover:bg-destructive/10 hover:text-destructive sm:opacity-0 sm:group-hover:opacity-100 sm:focus:opacity-100" aria-label={`Delete ${currentLabel}`}><Trash2 size={10} /></button>
  </div>{!destinationAvailable && <p className="px-2 pb-1 text-micro text-subtle-foreground">{page ? 'Destination unavailable' : 'Note unavailable'} · bookmark retained</p>}{error && <p className="px-2 py-1 text-micro text-destructive" role="alert">{error}</p>}</div>;
}

function destinationLabel(bookmark: DestinationBookmark, page: ResolvedDestinationBookmark['page']): string {
  const pageTitle = page?.title ?? bookmark.target.path.replace(/\.md$/i, '');
  if (bookmark.target.kind === 'heading') return `${pageTitle} › ${bookmark.target.anchor}`;
  const preview = page ? extractMarkdownBlockReferences(page.content).find((block) => block.id === bookmark.target.anchor)?.preview : null;
  return `${pageTitle} › ${preview ?? `^${bookmark.target.anchor}`}`;
}
