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
  properties: string;
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
        { field: 'properties', tokenize: 'forward' },
      ],
      store: ['title', 'content', 'tags', 'path', 'aliases', 'properties', 'updatedAt'],
    },
    tokenize: 'forward',
  });
}

let index = createIndex();
let docs = new Map<string, SearchDoc>();
let propertiesByPage = new Map<string, Map<string, string[]>>();
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
  const nextProperties = new Map<string, Map<string, string[]>>();
  for (const page of pages) {
    if (page.deletedAt !== null) continue;
    const properties = normalizeProperties(page.frontmatter);
    const doc: SearchDoc = {
      id: page.id,
      title: page.title,
      content: page.content.slice(0, 100_000),
      tags: (tagsByPage.get(page.id) ?? []).join(' '),
      path: page.path,
      aliases: page.aliases.join(' '),
      properties: [...properties.entries()].flatMap(([key, values]) => [key, ...values]).join(' '),
      updatedAt: page.updatedAt,
    };
    nextDocs.set(page.id, doc);
    nextProperties.set(page.id, properties);
    await nextIndex.addAsync(doc);
  }

  // A newer live-query emission won the race; discard this stale rebuild.
  if (version !== rebuildVersion) return;
  index = nextIndex;
  docs = nextDocs;
  propertiesByPage = nextProperties;
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
 * Operators are composable post-filters and work with or without free text:
 * `tag:research`, `-tag:archive`, `path:"project alpha"`,
 * `title:roadmap`, `property:status=active`, and `[status:active]`.
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

export interface SearchValueFilter {
  value: string;
  exclude: boolean;
}

export interface SearchPropertyFilter {
  key: string;
  value?: string;
  exclude: boolean;
}

export interface ParsedFilter {
  terms: string;
  tags: SearchValueFilter[];
  paths: SearchValueFilter[];
  titles: SearchValueFilter[];
  properties: SearchPropertyFilter[];
}

export function parseSearchQuery(query: string): ParsedFilter {
  const terms: string[] = [];
  const tags: SearchValueFilter[] = [];
  const paths: SearchValueFilter[] = [];
  const titles: SearchValueFilter[] = [];
  const properties: SearchPropertyFilter[] = [];
  for (const rawPart of tokenizeQuery(query)) {
    const exclude = rawPart.startsWith('-');
    const part = exclude ? rawPart.slice(1) : rawPart;
    const bracket = /^\[([^:\]]+)(?::([^\]]+))?\]$/.exec(part);
    if (bracket) {
      const key = cleanFilterValue(bracket[1]);
      const value = bracket[2] ? cleanFilterValue(bracket[2]) : undefined;
      if (key) properties.push({ key, value, exclude });
      continue;
    }
    const separator = part.indexOf(':');
    if (separator < 1) { terms.push(cleanTerm(rawPart)); continue; }
    const operator = part.slice(0, separator).toLowerCase();
    const rawValue = part.slice(separator + 1);
    if (operator === 'property') {
      const equals = rawValue.indexOf('=');
      const key = cleanFilterValue(equals >= 0 ? rawValue.slice(0, equals) : rawValue);
      const value = equals >= 0 ? cleanFilterValue(rawValue.slice(equals + 1)) : undefined;
      if (key) properties.push({ key, value, exclude });
    } else {
      const cleaned = cleanFilterValue(rawValue);
      const value = operator === 'tag' ? cleaned.replace(/^#/, '') : cleaned;
      const target = operator === 'tag' ? tags : operator === 'path' ? paths : operator === 'title' || operator === 'file' ? titles : null;
      if (target && value) target.push({ value, exclude });
      else terms.push(cleanTerm(rawPart));
    }
  }
  return { terms: terms.filter(Boolean).join(' '), tags, paths, titles, properties };
}

function parseFilter(query: string): ParsedFilter {
  return parseSearchQuery(query);
}

export function matchesSearchFilters(doc: Pick<SearchDoc, 'id' | 'title' | 'path' | 'tags'>, filter: ParsedFilter, properties = propertiesByPage.get(doc.id) ?? new Map<string, string[]>()): boolean {
  const tags = String(doc.tags).toLowerCase().split(/\s+/);
  if (!matchesEvery(filter.tags, (needle) => tags.some((tag) => tag === needle || tag.startsWith(`${needle}/`)))) return false;
  const path = String(doc.path).toLowerCase();
  if (!matchesEvery(filter.paths, (needle) => path.includes(needle))) return false;
  const title = String(doc.title).toLowerCase();
  if (!matchesEvery(filter.titles, (needle) => title.includes(needle))) return false;
  for (const property of filter.properties) {
    const values = properties.get(property.key);
    const match = Boolean(values && (property.value === undefined || values.some((value) => value.includes(property.value!))));
    if (property.exclude ? match : !match) return false;
  }
  return true;
}

function matchesFilters(doc: SearchDoc, filter: ParsedFilter): boolean {
  return matchesSearchFilters(doc, filter);
}

function matchesEvery(filters: SearchValueFilter[], predicate: (value: string) => boolean): boolean {
  return filters.every((filter) => filter.exclude ? !predicate(filter.value) : predicate(filter.value));
}

function tokenizeQuery(query: string): string[] {
  return query.match(/(?:[^\s"']+|"[^"]*"|'[^']*')+/g) ?? [];
}

function cleanFilterValue(value: string): string {
  return value.trim().replace(/^["']|["']$/g, '').toLowerCase();
}

function cleanTerm(value: string): string {
  return value.replace(/["']/g, '').trim();
}

function normalizeProperties(frontmatter: Record<string, unknown>): Map<string, string[]> {
  const result = new Map<string, string[]>();
  for (const [rawKey, rawValue] of Object.entries(frontmatter)) {
    const key = rawKey.trim().toLowerCase();
    if (!key) continue;
    const values = flattenPropertyValue(rawValue);
    if (values.length > 0) result.set(key, values);
  }
  return result;
}

function flattenPropertyValue(value: unknown): string[] {
  if (value === null || value === undefined) return [];
  if (Array.isArray(value)) return value.flatMap(flattenPropertyValue);
  if (typeof value === 'object') return Object.entries(value as Record<string, unknown>).flatMap(([key, nested]) => [key.toLowerCase(), ...flattenPropertyValue(nested)]);
  return [String(value).toLowerCase()];
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
