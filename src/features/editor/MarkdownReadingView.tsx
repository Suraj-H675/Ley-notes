import { useEffect, useState, type ReactNode } from 'react';
import { Paperclip } from 'lucide-react';
import ReactMarkdown, { defaultUrlTransform } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { updatePageContent } from '@/core/vault/pages';
import { resolveTitle } from '@/core/vault/page-index';
import { extractWikiLinks } from '@/core/parser/wiki-links';
import { attachmentObjectUrl, isSafeAttachmentPath } from '@/core/vault/attachments';
import { useLiveQuery } from 'dexie-react-hooks';
import { db } from '@/infrastructure/database/db';
import { countMarkdownTasks, toggleMarkdownTask } from '@/core/parser/tasks';
import { extractMarkdownDestination } from '@/core/parser/destinations';
import { openWikiDestination, type WikiDestination } from './lib/open-wiki-destination';
import { openMarkdownDestination } from './lib/open-wiki-destination';
import { parseInternalMarkdownDestination } from '@/core/parser/markdown-links';
import type { EditorPane } from '@/shared/state/nav';

export function MarkdownReadingView({ pageId, pagePath, content, pane }: { pageId: string; pagePath: string; content: string; pane: EditorPane }) {
  async function toggleTask(taskIndex: number, checked: boolean) {
    const next = toggleMarkdownTask(content, taskIndex, checked);
    if (next !== content) await updatePageContent(pageId, next);
  }

  const parts = withTaskOffsets(splitEmbeddedNotes(content));
  return (
    <article className="markdown-reading mx-auto w-full max-w-[820px] px-4 pb-32 pt-6 sm:px-10 sm:pt-8">
      {parts.map((part, index) => {
        if (part.kind === 'embed') {
          return <EmbeddedNoteCard key={`${part.target}-${part.heading ?? part.blockId ?? ''}-${index}`} destination={part} pane={pane} />;
        }
        return <MarkdownBody key={`markdown-${index}`} content={part.content} sourcePath={pagePath} taskOffset={part.taskOffset} onToggleTask={toggleTask} pane={pane} />;
      })}
    </article>
  );
}

function MarkdownBody({ content, sourcePath, taskOffset = 0, onToggleTask, pane }: { content: string; sourcePath: string; taskOffset?: number; onToggleTask: (taskIndex: number, checked: boolean) => Promise<void>; pane: EditorPane }) {
  return <ReactMarkdown remarkPlugins={[remarkGfm]} urlTransform={(url) => url.startsWith('ley:') ? url : defaultUrlTransform(url)} components={{
    a: ({ href, children }) => {
      if (href && isSafeAttachmentPath(href)) return <AttachmentLink path={href}>{children}</AttachmentLink>;
      if (href?.startsWith('ley:')) {
        const destination = parseLeyDestination(href);
        return <button type="button" className="wiki-reading-link" onClick={() => void openWikiDestination(destination, pane)}>{children}</button>;
      }
      if (href) {
        const destination = parseInternalMarkdownDestination(href);
        if (destination) return <button type="button" className="wiki-reading-link" onClick={() => void openMarkdownDestination(sourcePath, destination.path, destination.heading, destination.blockId, pane)}>{children}</button>;
      }
      return <a href={href} target="_blank" rel="noreferrer">{children}</a>;
    },
    input: ({ type, checked, node, disabled: _disabled, ...props }) => {
      if (type !== 'checkbox') return <input type={type} {...props} />;
      const sourceLine = node?.position?.start.line ?? 1;
      const localIndex = Math.max(0, countMarkdownTasks(content.split('\n').slice(0, sourceLine).join('\n')) - 1);
      return <input type="checkbox" checked={checked} onChange={(event) => void onToggleTask(taskOffset + localIndex, event.currentTarget.checked)} {...props} />;
    },
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

function EmbeddedNoteCard({ destination, pane }: { destination: Extract<ReadingPart, { kind: 'embed' }>; pane: EditorPane }) {
  const { target, heading, blockId } = destination;
  const page = useLiveQuery(async () => {
    const id = await resolveTitle(target);
    return id ? db.pages.get(id) : undefined;
  }, [target]);

  if (!page) return <div className="my-4 rounded-sm border border-dashed border-border bg-surface-1 p-4 text-meta text-muted-foreground">Embedded note “{target}” does not exist yet.</div>;
  const embeddedPage = page;
  const embeddedContent = extractMarkdownDestination(embeddedPage.content, heading, blockId);
  async function toggleEmbeddedTask(taskIndex: number, checked: boolean) {
    const next = toggleMarkdownTask(embeddedContent, taskIndex, checked);
    if (next === embeddedContent) return;
    const updated = heading || blockId ? embeddedPage.content.replace(embeddedContent, next) : next;
    await updatePageContent(embeddedPage.id, updated);
  }
  return (
    <section className="my-5 overflow-hidden rounded-md border border-border bg-surface-1 shadow-sm">
      <button type="button" onClick={() => void openWikiDestination(destination, pane)} className="flex w-full items-center justify-between border-b border-border px-4 py-2 text-left text-meta font-medium text-foreground hover:bg-surface-2"><span>{page.title}{heading ? ` › ${heading}` : blockId ? ` › ^${blockId}` : ''}</span><span className="font-mono text-micro text-muted-foreground">{page.path}</span></button>
      <div className="max-h-[420px] overflow-auto px-4 py-3"><MarkdownBody content={replaceEmbedsWithLinks(embeddedContent)} sourcePath={embeddedPage.path} onToggleTask={toggleEmbeddedTask} pane={pane} /></div>
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

  if (failed) return <span className="inline-flex rounded-sm border border-dashed border-border px-3 py-2 text-meta text-muted-foreground">Missing attachment: {alt || path}</span>;
  if (!source) return <span className="inline-block h-36 w-full animate-pulse rounded-md bg-surface-2" aria-label={`Loading ${alt || path}`} />;
  return <img src={source} alt={alt} loading="lazy" className="max-h-[70vh] rounded-md border border-border object-contain" />;
}

function renderableMarkdown(content: string): string {
  let output = content;
  const links = extractWikiLinks(content).sort((left, right) => right.position - left.position);
  for (const link of links) {
    const label = link.alias ?? link.target;
    const anchor = link.blockId ? `#^${encodeURIComponent(link.blockId)}` : link.heading ? `#${encodeURIComponent(link.heading)}` : '';
    if (link.isEmbed) continue;
    const replacement = `[${label}](ley:${encodeURIComponent(link.target)}${anchor})`;
    output = `${output.slice(0, link.position)}${replacement}${output.slice(link.position + link.raw.length)}`;
  }
  return output;
}

type ReadingPart = { kind: 'markdown'; content: string } | { kind: 'embed'; target: string; heading: string | null; blockId: string | null };
type OffsetReadingPart = Exclude<ReadingPart, { kind: 'markdown' }> | (Extract<ReadingPart, { kind: 'markdown' }> & { taskOffset: number });

function splitEmbeddedNotes(content: string): ReadingPart[] {
  const embeds = extractWikiLinks(content).filter((link) => link.isEmbed).sort((left, right) => left.position - right.position);
  if (embeds.length === 0) return [{ kind: 'markdown', content }];
  const parts: ReadingPart[] = [];
  let cursor = 0;
  for (const embed of embeds) {
    if (embed.position > cursor) parts.push({ kind: 'markdown', content: content.slice(cursor, embed.position) });
    parts.push({ kind: 'embed', target: embed.target, heading: embed.heading, blockId: embed.blockId });
    cursor = embed.position + embed.raw.length;
  }
  if (cursor < content.length) parts.push({ kind: 'markdown', content: content.slice(cursor) });
  return parts;
}

function withTaskOffsets(parts: ReadingPart[]): OffsetReadingPart[] {
  let offset = 0;
  return parts.map((part) => {
    if (part.kind === 'embed') return part;
    const result = { ...part, taskOffset: offset };
    offset += countMarkdownTasks(part.content);
    return result;
  });
}

function parseLeyDestination(href: string): WikiDestination {
  const [encodedTarget, encodedAnchor] = href.slice(4).split('#', 2);
  const anchor = encodedAnchor ? decodeURIComponent(encodedAnchor) : null;
  return {
    target: decodeURIComponent(encodedTarget),
    heading: anchor && !anchor.startsWith('^') ? anchor : null,
    blockId: anchor?.startsWith('^') ? anchor.slice(1) : null,
  };
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
