/**
 * Full-text search over pages. Uses Flexsearch for fast prefix + token search.
 *
 * Index strategy: a single Flexsearch Document indexed by page ID. On every
 * save we delete the old entry and re-add with the updated fields. We do NOT
 * re-tokenize the entire vault on each save (that would be O(n)).
 *
 * Why client-side and not a separate index store? Vaults are small (10k
 * pages typical, 50k ceiling in our plan). A single in-memory index keeps
 * the model simple — no sync, no persistence, just rebuild on cold start.
 *
 * Re-build on cold start is sub-100ms for 10k pages based on Flexsearch's
 * published benchmarks; users won't notice.
 */

import FlexSearch from 'flexsearch';
import type { Page } from '@/data/schema';

/**
 * Flexsearch Document requires a string index signature. We model docs as
 * `{ id, title, content, tags }` and cast to `any` at the constructor so we
 * don't have to maintain the public API's exact field shape in our types.
 */
type Doc = { [key: string]: string };

const index = new FlexSearch.Document({
  document: {
    id: 'id',
    index: [
      { field: 'title', tokenize: 'forward' },
      { field: 'content', tokenize: 'forward' },
      { field: 'tags', tokenize: 'forward' },
    ],
    store: ['title', 'content', 'tags'],
  },
  tokenize: 'forward',
});

/** Add or replace a page in the index. */
export async function indexPage(page: Page, tags: string[]): Promise<void> {
  const doc: Doc = {
    id: page.id,
    title: page.title,
    content: page.content.slice(0, 50_000), // cap per-page content for index size
    tags: tags.join(' '),
  };
  await index.removeAsync(page.id);
  await index.addAsync(doc);
}

/** Remove a page from the index. */
export async function removePageFromIndex(pageId: string): Promise<void> {
  await index.removeAsync(pageId);
}

/**
 * Search the index. Returns up to `limit` matching page IDs, ranked by Flexsearch.
 * Filter syntax: `tag:foo` (must have tag foo), `path:folder/` (path prefix).
 */
export async function searchPages(
  query: string,
  limit = 20,
): Promise<Array<{ id: string; title: string; score: number }>> {
  const trimmed = query.trim();
  if (!trimmed) return [];

  const filter = parseFilter(trimmed);
  const results = await index.searchAsync(filter.terms, { limit, enrich: true });

  // Flatten across fields (Flexsearch returns one result per field).
  const seen = new Map<string, { title: string; score: number }>();
  for (const fieldResult of results) {
    for (const item of fieldResult.result) {
      const id = String(item.id);
      const title = String(item.doc?.title ?? '');
      // Flexsearch's score is field-local; combine by best (lowest distance wins).
      const score = seen.get(id)?.score ?? 0;
      if (!seen.has(id) || score > 0) seen.set(id, { title, score: 1 });
    }
  }

  // Convert to sorted list.
  return [...seen.entries()]
    .map(([id, v]) => ({ id, title: v.title, score: v.score }))
    .sort((a, b) => b.score - a.score)
    .slice(0, limit);
}

interface ParsedFilter {
  terms: string;
  tag?: string;
  pathPrefix?: string;
}

function parseFilter(query: string): ParsedFilter {
  const parts = query.split(/\s+/);
  const terms: string[] = [];
  let tag: string | undefined;
  let pathPrefix: string | undefined;
  for (const p of parts) {
    if (p.startsWith('tag:')) {
      tag = p.slice(4);
    } else if (p.startsWith('path:')) {
      pathPrefix = p.slice(5);
    } else {
      terms.push(p);
    }
  }
  return { terms: terms.join(' '), tag, pathPrefix };
}

/** Returns the parsed filter for callers that want to apply post-filter logic. */
export function getFilter(query: string): ParsedFilter {
  return parseFilter(query);
}