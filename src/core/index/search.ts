/**
 * Reactive full-text index for the vault.
 *
 * Pages remain the source of truth. This module keeps a disposable in-memory
 * FlexSearch index in sync with Dexie and can rebuild it at any time. Search
 * never depends on callers remembering to manually index a write.
 */

import FlexSearch from 'flexsearch';
import { liveQuery, type Subscription } from 'dexie';
import { db } from '@/infrastructure/database/db';
import type { Page, Tag } from '@/infrastructure/database/schema';

interface SearchDoc {
  [key: string]: string | number;
  id: string;
  title: string;
  content: string;
  tags: string;
  path: string;
  aliases: string;
  updatedAt: number;
}

export interface PageSearchResult {
  id: string;
  title: string;
  path: string;
  snippet: string;
  score: number;
}

function createIndex() {
  return new FlexSearch.Document({
    document: {
      id: 'id',
      index: [
        { field: 'title', tokenize: 'forward' },
        { field: 'aliases', tokenize: 'forward' },
        { field: 'content', tokenize: 'forward' },
        { field: 'tags', tokenize: 'forward' },
        { field: 'path', tokenize: 'forward' },
      ],
      store: ['title', 'content', 'tags', 'path', 'aliases', 'updatedAt'],
    },
    tokenize: 'forward',
  });
}

let index = createIndex();
let docs = new Map<string, SearchDoc>();
let subscription: Subscription | null = null;
let consumers = 0;
let rebuildVersion = 0;
let ready = false;

/** Start the Dexie → search-index bridge. Safe under React Strict Mode. */
export function startSearchIndex(): () => void {
  consumers += 1;
  if (!subscription) {
    subscription = liveQuery(async () => {
      const [pages, tags] = await Promise.all([db.pages.toArray(), db.tags.toArray()]);
      return { pages, tags };
    }).subscribe({
      next: ({ pages, tags }) => {
        void rebuildIndex(pages, tags);
      },
      error: (error) => console.error('[search-index] live query failed:', error),
    });
  }

  let stopped = false;
  return () => {
    if (stopped) return;
    stopped = true;
    consumers = Math.max(0, consumers - 1);
    if (consumers === 0) {
      subscription?.unsubscribe();
      subscription = null;
    }
  };
}

async function rebuildIndex(pages: Page[], tags: Tag[]): Promise<void> {
  const version = ++rebuildVersion;
  const tagsByPage = new Map<string, string[]>();
  for (const row of tags) {
    const values = tagsByPage.get(row.pageId) ?? [];
    values.push(row.tag);
    tagsByPage.set(row.pageId, values);
  }

  const nextIndex = createIndex();
  const nextDocs = new Map<string, SearchDoc>();
  for (const page of pages) {
    if (page.deletedAt !== null) continue;
    const doc: SearchDoc = {
      id: page.id,
      title: page.title,
      content: page.content.slice(0, 100_000),
      tags: (tagsByPage.get(page.id) ?? []).join(' '),
      path: page.path,
      aliases: page.aliases.join(' '),
      updatedAt: page.updatedAt,
    };
    nextDocs.set(page.id, doc);
    await nextIndex.addAsync(doc);
  }

  // A newer live-query emission won the race; discard this stale rebuild.
  if (version !== rebuildVersion) return;
  index = nextIndex;
  docs = nextDocs;
  ready = true;
}

async function ensureReady(): Promise<void> {
  if (ready) return;
  const [pages, tags] = await Promise.all([db.pages.toArray(), db.tags.toArray()]);
  await rebuildIndex(pages, tags);
}

/**
 * Search titles, aliases, content, tags, and paths.
 *
 * Operators are post-filters and work with or without free text:
 * `tag:research`, `path:projects/`.
 */
export async function searchPages(query: string, limit = 20): Promise<PageSearchResult[]> {
  await ensureReady();
  const filter = parseFilter(query.trim());

  if (!filter.terms) {
    return [...docs.values()]
      .filter((doc) => matchesFilters(doc, filter))
      .sort((a, b) => b.updatedAt - a.updatedAt)
      .slice(0, limit)
      .map((doc) => toResult(doc, '', 1));
  }

  const raw = await index.searchAsync(filter.terms, { limit: Math.max(limit * 4, 40), enrich: true });
  const ranked = new Map<string, number>();
  const queryLc = filter.terms.toLowerCase();

  for (const fieldResult of raw) {
    const fieldBoost = fieldResult.field === 'title' ? 50 : fieldResult.field === 'aliases' ? 35 : 10;
    fieldResult.result.forEach((item, position) => {
      const id = String(item.id);
      const doc = docs.get(id);
      if (!doc || !matchesFilters(doc, filter)) return;
      let score = fieldBoost + Math.max(0, 30 - position);
      const title = doc.title.toLowerCase();
      if (title === queryLc) score += 100;
      else if (title.startsWith(queryLc)) score += 60;
      ranked.set(id, Math.max(ranked.get(id) ?? 0, score));
    });
  }

  return [...ranked.entries()]
    .map(([id, score]) => toResult(docs.get(id)!, filter.terms, score))
    .sort((a, b) => b.score - a.score || a.title.localeCompare(b.title))
    .slice(0, limit);
}

interface ParsedFilter {
  terms: string;
  tag?: string;
  pathPrefix?: string;
}

function parseFilter(query: string): ParsedFilter {
  const terms: string[] = [];
  let tag: string | undefined;
  let pathPrefix: string | undefined;
  for (const part of query.split(/\s+/).filter(Boolean)) {
    if (part.toLowerCase().startsWith('tag:')) tag = part.slice(4).replace(/^#/, '').toLowerCase();
    else if (part.toLowerCase().startsWith('path:')) pathPrefix = part.slice(5).toLowerCase();
    else terms.push(part);
  }
  return { terms: terms.join(' '), tag, pathPrefix };
}

function matchesFilters(doc: SearchDoc, filter: ParsedFilter): boolean {
  const tags = String(doc.tags).toLowerCase().split(/\s+/);
  if (filter.tag && !tags.some((tag) => tag === filter.tag || tag.startsWith(`${filter.tag}/`))) return false;
  if (filter.pathPrefix && !String(doc.path).toLowerCase().startsWith(filter.pathPrefix)) return false;
  return true;
}

function toResult(doc: SearchDoc, terms: string, score: number): PageSearchResult {
  return {
    id: doc.id,
    title: String(doc.title),
    path: String(doc.path),
    snippet: makeSnippet(String(doc.content), terms),
    score,
  };
}

function makeSnippet(content: string, terms: string): string {
  const compact = content.replace(/[#>*_`[\]]/g, '').replace(/\s+/g, ' ').trim();
  if (!compact) return 'Empty note';
  const needle = terms.split(/\s+/).find(Boolean)?.toLowerCase();
  const at = needle ? compact.toLowerCase().indexOf(needle) : -1;
  const start = at > 50 ? at - 40 : 0;
  const prefix = start > 0 ? '…' : '';
  const suffix = start + 150 < compact.length ? '…' : '';
  return `${prefix}${compact.slice(start, start + 150)}${suffix}`;
}

export function getFilter(query: string): ParsedFilter {
  return parseFilter(query);
}
