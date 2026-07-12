import { useEffect, useState, type ReactNode } from 'react';
import { Paperclip } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { createPage } from '@/core/vault/pages';
import { resolveTitle } from '@/core/vault/page-index';
import { extractWikiLinks } from '@/core/parser/wiki-links';
import { useNavStore } from '@/shared/state/nav';
import { attachmentObjectUrl, isSafeAttachmentPath } from '@/core/vault/attachments';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';

export function MarkdownReadingView({ content }: { content: string }) {
  async function follow(target: string) {
    const id = await resolveTitle(target) ?? (await createPage({ title: target })).id;
    const nav = useNavStore.getState();
    nav.openPage(id);
    nav.pushRecent(id);
  }

  return (
    <article className="markdown-reading mx-auto w-full max-w-[820px] px-4 pb-32 pt-6 sm:px-10 sm:pt-8">
      {splitEmbeddedNotes(content).map((part, index) => part.kind === 'embed'
        ? <EmbeddedNoteCard key={`${part.target}-${index}`} target={part.target} onOpen={follow} />
        : <MarkdownBody key={index} content={part.content} onFollow={follow} />)}
    </article>
  );
}

function MarkdownBody({ content, onFollow }: { content: string; onFollow: (target: string) => Promise<void> }) {
  return <ReactMarkdown remarkPlugins={[remarkGfm]} components={{
    a: ({ href, children }) => {
      if (href && isSafeAttachmentPath(href)) return <AttachmentLink path={href}>{children}</AttachmentLink>;
      if (href?.startsWith('ley:')) {
        const target = decodeURIComponent(href.slice(4).split('#')[0]);
        return <button type="button" className="wiki-reading-link" onClick={() => void onFollow(target)}>{children}</button>;
      }
      return <a href={href} target="_blank" rel="noreferrer">{children}</a>;
    },
    input: ({ type, checked, ...props }) => type === 'checkbox'
      ? <input type="checkbox" checked={checked} readOnly {...props} />
      : <input type={type} {...props} />,
    img: ({ src, alt }) => src && isSafeAttachmentPath(src)
      ? <AttachmentImage path={src} alt={alt ?? ''} />
      : <img src={src} alt={alt ?? ''} loading="lazy" />,
  }}>{renderableMarkdown(content)}</ReactMarkdown>;
}

function AttachmentLink({ path, children }: { path: string; children: ReactNode }) {
  const [busy, setBusy] = useState(false);
  async function openAttachment() {
    setBusy(true);
    try {
      const url = await attachmentObjectUrl(path);
      if (!url) return;
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.target = '_blank';
      anchor.rel = 'noreferrer';
      anchor.click();
      window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
    } finally {
      setBusy(false);
    }
  }
  return <button type="button" disabled={busy} onClick={() => void openAttachment()} className="inline-flex items-center gap-1 rounded bg-surface-2 px-1.5 py-0.5 text-secondary hover:bg-surface-3 disabled:opacity-60"><Paperclip size={11} />{children}</button>;
}

function EmbeddedNoteCard({ target, onOpen }: { target: string; onOpen: (target: string) => Promise<void> }) {
  const page = useLiveQuery(async () => {
    const id = await resolveTitle(target);
    return id ? db.pages.get(id) : undefined;
  }, [target]);

  if (!page) return <div className="my-4 rounded-xl border border-dashed border-border bg-surface-1 p-4 text-meta text-muted-foreground">Embedded note “{target}” does not exist yet.</div>;
  return (
    <section className="my-5 overflow-hidden rounded-xl border border-border bg-surface-1 shadow-sm">
      <button type="button" onClick={() => void onOpen(target)} className="flex w-full items-center justify-between border-b border-border px-4 py-2 text-left text-meta font-medium text-foreground hover:bg-surface-2"><span>{page.title}</span><span className="font-mono text-micro text-muted-foreground">{page.path}</span></button>
      <div className="max-h-[420px] overflow-auto px-4 py-3"><MarkdownBody content={replaceEmbedsWithLinks(page.content)} onFollow={onOpen} /></div>
    </section>
  );
}

function AttachmentImage({ path, alt }: { path: string; alt: string }) {
  const [source, setSource] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let active = true;
    let objectUrl: string | null = null;
    void attachmentObjectUrl(path)
      .then((url) => {
        objectUrl = url;
        if (active) {
          setSource(url);
          setFailed(!url);
        } else if (url) URL.revokeObjectURL(url);
      })
      .catch(() => { if (active) setFailed(true); });
    return () => {
      active = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [path]);

  if (failed) return <span className="inline-flex rounded-md border border-dashed border-border px-3 py-2 text-meta text-muted-foreground">Missing attachment: {alt || path}</span>;
  if (!source) return <span className="inline-block h-36 w-full animate-pulse rounded-lg bg-surface-2" aria-label={`Loading ${alt || path}`} />;
  return <img src={source} alt={alt} loading="lazy" className="max-h-[70vh] rounded-lg border border-border object-contain" />;
}

function renderableMarkdown(content: string): string {
  let output = content;
  const links = extractWikiLinks(content).sort((left, right) => right.position - left.position);
  for (const link of links) {
    const label = link.alias ?? link.target;
    const anchor = link.blockId ? `#^${link.blockId}` : link.heading ? `#${link.heading}` : '';
    if (link.isEmbed) continue;
    const replacement = `[${label}](ley:${encodeURIComponent(link.target)}${anchor})`;
    output = `${output.slice(0, link.position)}${replacement}${output.slice(link.position + link.raw.length)}`;
  }
  return output;
}

type ReadingPart = { kind: 'markdown'; content: string } | { kind: 'embed'; target: string };

function splitEmbeddedNotes(content: string): ReadingPart[] {
  const embeds = extractWikiLinks(content).filter((link) => link.isEmbed).sort((left, right) => left.position - right.position);
  if (embeds.length === 0) return [{ kind: 'markdown', content }];
  const parts: ReadingPart[] = [];
  let cursor = 0;
  for (const embed of embeds) {
    if (embed.position > cursor) parts.push({ kind: 'markdown', content: content.slice(cursor, embed.position) });
    parts.push({ kind: 'embed', target: embed.target });
    cursor = embed.position + embed.raw.length;
  }
  if (cursor < content.length) parts.push({ kind: 'markdown', content: content.slice(cursor) });
  return parts;
}

function replaceEmbedsWithLinks(content: string): string {
  let output = content;
  const embeds = extractWikiLinks(content).filter((link) => link.isEmbed).sort((left, right) => right.position - left.position);
  for (const embed of embeds) {
    const label = embed.alias ?? embed.target;
    const replacement = `Embedded note: [[${label}]]`;
    output = `${output.slice(0, embed.position)}${replacement}${output.slice(embed.position + embed.raw.length)}`;
  }
  return output;
}
