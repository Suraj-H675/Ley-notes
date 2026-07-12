import { Hash, ListTree } from 'lucide-react';
import type { Page } from '@/infrastructure/database/schema';
import { extractMarkdownHeadings } from '@/core/parser/destinations';

export function OutlinePanel({ page }: { page: Page | undefined }) {
  const headings = page ? extractMarkdownHeadings(page.content) : [];
  return (
    <div className="h-full overflow-y-auto px-3 py-4">
      <div className="mb-3 flex items-center gap-1.5 text-meta font-medium text-muted-foreground"><ListTree size={13} />Outline<span className="ml-auto text-micro">{headings.length}</span></div>
      {headings.length === 0 ? (
        <div className="rounded-md border border-dashed border-border px-3 py-5 text-center text-micro text-muted-foreground">Add Markdown headings to navigate this note.</div>
      ) : headings.map((heading) => (
        <button
          key={`${heading.line}:${heading.title}`}
          type="button"
          onClick={() => window.dispatchEvent(new CustomEvent('ley:outline-jump', { detail: heading }))}
          className="flex w-full items-center gap-1.5 rounded py-1.5 pr-2 text-left text-meta text-muted-foreground-strong hover:bg-surface-2 hover:text-foreground"
          style={{ paddingLeft: 4 + (heading.level - 1) * 12 }}
        >
          <Hash size={11} className="shrink-0 text-subtle-foreground" /><span className="truncate">{heading.title}</span>
        </button>
      ))}
    </div>
  );
}
