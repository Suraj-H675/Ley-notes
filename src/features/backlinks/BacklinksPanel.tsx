import { ArrowDownLeft, ArrowUpRight, FileQuestion, FilePlus2, Link2, Unlink } from 'lucide-react';
import {
  excerptAround,
  useBacklinks,
  useOutgoingLinks,
  useUnlinkedMentions,
} from '@/features/backlinks/useBacklinks';
import { usePageById } from '@/features/notes/usePages';
import { useNavStore } from '@/shared/state/nav';
import { createPage, updatePageContent } from '@/core/vault/pages';

export function BacklinksPanel({ pageId }: { pageId: string | null }) {
  const backlinks = useBacklinks(pageId) ?? [];
  const outgoing = useOutgoingLinks(pageId) ?? [];
  const mentions = useUnlinkedMentions(pageId) ?? [];
  const page = usePageById(pageId);
  const openPage = useNavStore((state) => state.openPage);
  const pushRecent = useNavStore((state) => state.pushRecent);

  if (!pageId || !page) return null;
  const pageTitle = page.title;

  function navigate(id: string) {
    openPage(id);
    pushRecent(id);
  }

  async function createGhost(title: string) {
    const created = await createPage({ title });
    navigate(created.id);
  }

  async function linkMention(sourceId: string, position: number) {
    const mention = mentions.find((entry) => entry.source.id === sourceId && entry.position === position);
    if (!mention) return;
    const content = mention.source.content;
    if (content.slice(position, position + pageTitle.length).toLowerCase() !== pageTitle.toLowerCase()) return;
    const next = `${content.slice(0, position)}[[${content.slice(position, position + pageTitle.length)}]]${content.slice(position + pageTitle.length)}`;
    await updatePageContent(sourceId, next);
  }

  const inboundGroups = new Map<string, { source: (typeof backlinks)[number]['source']; entries: typeof backlinks }>();
  for (const entry of backlinks) {
    const current = inboundGroups.get(entry.source.id);
    if (current) current.entries.push(entry);
    else inboundGroups.set(entry.source.id, { source: entry.source, entries: [entry] });
  }

  return (
    <div className="h-full overflow-y-auto px-3 pb-5">
      <Section title="Linked mentions" count={backlinks.length} icon={<ArrowDownLeft size={13} />}>
        {inboundGroups.size === 0 ? <Hint>Nothing links here yet.</Hint> : [...inboundGroups.values()].map(({ source, entries }) => (
          <button key={source.id} type="button" onClick={() => navigate(source.id)} className="w-full rounded-md border border-border bg-surface-1 p-2.5 text-left hover:bg-surface-2">
            <div className="flex items-center justify-between gap-2 text-meta font-medium"><span className="truncate">{source.title}</span><span className="text-micro text-muted-foreground">{entries.length}</span></div>
            <div className="mt-1 line-clamp-2 text-micro leading-4 text-muted-foreground">{excerptAround(source.content, entries[0].link.position, entries[0].link.targetTitle.length)}</div>
          </button>
        ))}
      </Section>

      <Section title="Outgoing links" count={outgoing.length} icon={<ArrowUpRight size={13} />}>
        {outgoing.length === 0 ? <Hint>This note does not link anywhere yet.</Hint> : outgoing.map(({ link, target }) => target ? (
          <button key={link.id} type="button" onClick={() => navigate(target.id)} className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-meta hover:bg-surface-2">
            <Link2 size={13} className="shrink-0 text-secondary" /><span className="truncate">{target.title}</span>
          </button>
        ) : link.kind === 'markdown' ? (
          <div key={link.id} className="flex w-full items-center gap-2 rounded-sm border border-dashed border-border px-2 py-1.5 text-left text-meta text-muted-foreground" title="The linked Markdown file is not in this vault">
            <FileQuestion size={13} className="shrink-0" /><span className="truncate">{link.targetTitle}</span><span className="ml-auto text-micro">Missing</span>
          </div>
        ) : (
          <button key={link.id} type="button" onClick={() => void createGhost(link.targetTitle)} className="flex w-full items-center gap-2 rounded-sm border border-dashed border-border px-2 py-1.5 text-left text-meta hover:border-secondary hover:bg-surface-2">
            <FilePlus2 size={13} className="shrink-0 text-secondary" /><span className="truncate">{link.targetTitle}</span><span className="ml-auto text-micro text-muted-foreground">Create</span>
          </button>
        ))}
      </Section>

      <Section title="Unlinked mentions" count={mentions.length} icon={<Unlink size={13} />}>
        {mentions.length === 0 ? <Hint>No unlinked title mentions found.</Hint> : mentions.map((mention) => (
          <div key={`${mention.source.id}:${mention.position}`} className="rounded-md border border-border bg-surface-1 p-2.5">
            <button type="button" onClick={() => navigate(mention.source.id)} className="text-meta font-medium hover:text-primary">{mention.source.title}</button>
            <div className="mt-1 line-clamp-2 text-micro leading-4 text-muted-foreground">{mention.excerpt}</div>
            <button type="button" onClick={() => void linkMention(mention.source.id, mention.position)} className="mt-2 rounded bg-surface-3 px-2 py-1 text-micro text-foreground hover:bg-primary hover:text-primary-foreground">Link mention</button>
          </div>
        ))}
      </Section>
    </div>
  );
}

function Section({ title, count, icon, children }: { title: string; count: number; icon: React.ReactNode; children: React.ReactNode }) {
  return <section className="pt-4"><div className="mb-2 flex items-center gap-1.5 text-micro font-medium uppercase tracking-[0.08em] text-subtle-foreground">{icon}<span>{title}</span><span className="ml-auto tabular-nums text-subtle-foreground">{count}</span></div><div className="space-y-2">{children}</div></section>;
}

function Hint({ children }: { children: React.ReactNode }) {
  return <div className="rounded-sm border border-dashed border-border px-3 py-4 text-center text-micro text-muted-foreground">{children}</div>;
}
