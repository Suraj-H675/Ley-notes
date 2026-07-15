import { Bookmark, Hash, ListTree } from 'lucide-react';
import type { Page } from '@/infrastructure/database/schema';
import { extractMarkdownHeadings } from '@/core/parser/destinations';
import { toggleDestinationBookmark } from '@/core/vault/bookmarks';
import { useDestinationBookmarks } from '@/features/bookmarks/useBookmarks';

export function OutlinePanel({ page }: { page: Page | undefined }) {
  const headings = page ? extractMarkdownHeadings(page.content) : [];
  const bookmarks = useDestinationBookmarks();
  const bookmarkedHeadings = new Set(bookmarks
    .filter(({ bookmark, page: resolvedPage }) => bookmark.target.kind === 'heading' && resolvedPage?.id === page?.id)
    .map(({ bookmark }) => bookmark.target.anchor.toLowerCase()));
  return (
    <div className="h-full overflow-y-auto px-3 py-4">
      <div className="mb-3 flex items-center gap-1.5 text-meta font-medium text-muted-foreground"><ListTree size={13} />Outline<span className="ml-auto text-micro">{headings.length}</span></div>
      {headings.length === 0 ? (
        <div className="rounded-md border border-dashed border-border px-3 py-5 text-center text-micro text-muted-foreground">Add Markdown headings to navigate this note.</div>
      ) : headings.map((heading) => {
        const bookmarked = bookmarkedHeadings.has(heading.title.toLowerCase());
        return <div key={`${heading.line}:${heading.title}`} className="group flex items-center rounded hover:bg-surface-2">
          <button type="button" onClick={() => window.dispatchEvent(new CustomEvent('ley:outline-jump', { detail: { ...heading, pageId: page?.id } }))} className="flex min-w-0 flex-1 items-center gap-1.5 py-1.5 pr-1 text-left text-meta text-muted-foreground-strong hover:text-foreground" style={{ paddingLeft: 4 + (heading.level - 1) * 12 }}>
            <Hash size={11} className="shrink-0 text-subtle-foreground" /><span className="truncate">{heading.title}</span>
          </button>
          {page && <button type="button" onClick={() => void toggleDestinationBookmark({ kind: 'heading', pageId: page.id, path: page.path, anchor: heading.title })} aria-label={bookmarked ? `Remove ${heading.title} bookmark` : `Bookmark ${heading.title}`} aria-pressed={bookmarked} title={bookmarked ? 'Remove bookmark' : 'Bookmark heading'} className={`mr-1 rounded p-1 opacity-70 hover:bg-surface-3 sm:opacity-0 sm:group-hover:opacity-100 sm:focus:opacity-100 ${bookmarked ? 'text-secondary sm:opacity-100' : 'text-muted-foreground'}`}><Bookmark size={10} className={bookmarked ? 'fill-current' : undefined} /></button>}
        </div>;
      })}
    </div>
  );
}
