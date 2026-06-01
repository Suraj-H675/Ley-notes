import FlexSearch from 'flexsearch';
import type { KnowledgeNode, SearchOptions, SearchResult, ParsedQuery } from '@/types';
import { db } from '@/lib/db';

type FlexSearchId = number | string;

interface SearchDocument {
  id: string;
  title: string;
  plainText: string;
  tags: string;
  properties: string;
  type: string;
}

let titleIndex: FlexSearch.Index | null = null;
let contentIndex: FlexSearch.Index | null = null;
let tagIndex: FlexSearch.Index | null = null;
let isInitialized = false;

export async function initializeSearch(nodes: KnowledgeNode[]): Promise<void> {
  titleIndex = new FlexSearch.Index({
    tokenize: 'forward',
    resolution: 9,
  });

  contentIndex = new FlexSearch.Index({
    tokenize: 'forward',
    resolution: 9,
  });

  tagIndex = new FlexSearch.Index({
    tokenize: 'full',
    resolution: 9,
  });

  for (const node of nodes) {
    await indexNode(node);
  }

  isInitialized = true;
}

export async function indexNode(node: KnowledgeNode): Promise<void> {
  const doc: SearchDocument = {
    id: node.id,
    title: node.title,
    plainText: node.plainText,
    tags: node.tags.join(' '),
    properties: Object.values(node.properties).join(' '),
    type: node.type,
  };

  titleIndex?.add(doc.id, doc.title);
  contentIndex?.add(doc.id, `${doc.title} ${doc.plainText} ${doc.properties}`);
  tagIndex?.add(doc.id, doc.tags);
}

export async function updateNodeIndex(node: KnowledgeNode): Promise<void> {
  await removeFromIndex(node.id);
  await indexNode(node);
}

export async function removeFromIndex(nodeId: string): Promise<void> {
  titleIndex?.remove(nodeId);
  contentIndex?.remove(nodeId);
  tagIndex?.remove(nodeId);
}

export function parseSearchQuery(query: string): ParsedQuery {
  const OPERATORS: Record<string, string> = {
    type: 'type',
    is: 'type',
    tag: 'tag',
    collection: 'collection',
    related: 'related',
    depends: 'depends',
    uses: 'uses',
    created: 'created',
    modified: 'modified',
  };

  const operatorRegex = /^(\w+):(\S+)/;
  const match = query.match(operatorRegex);

  if (match && OPERATORS[match[1]]) {
    return {
      raw: query,
      operator: OPERATORS[match[1]] as ParsedQuery['operator'],
      operatorValue: match[2],
      text: query.slice(match[0].length).trim(),
    };
  }

  return { raw: query, text: query };
}

export async function search(
  query: string,
  options: SearchOptions = {}
): Promise<SearchResult[]> {
  if (!isInitialized) {
    const nodes = await db.nodes.where('isArchived').equals(0).toArray();
    await initializeSearch(nodes);
  }

  const { limit = 20 } = options;
  const parsed = parseSearchQuery(query);

  let results: SearchResult[] = [];

  if (parsed.operator && parsed.operatorValue) {
    results = await searchWithOperator(parsed, options);
  } else {
    results = await searchByText(parsed.text, options);
  }

  return results.slice(0, limit);
}

async function searchWithOperator(
  parsed: ParsedQuery,
  options: SearchOptions
): Promise<SearchResult[]> {
  const nodes = await getFilteredNodes(options);

  switch (parsed.operator) {
    case 'type':
      return nodes
        .filter((n) => n.type === parsed.operatorValue)
        .map((n) => ({
          id: n.id,
          node: n,
          score: 1,
          matchedField: 'title' as const,
          highlights: [],
        }));

    case 'tag':
      return nodes
        .filter((n) => n.tags.includes(parsed.operatorValue!))
        .map((n) => ({
          id: n.id,
          node: n,
          score: 1,
          matchedField: 'tags' as const,
          highlights: [],
        }));

    case 'related': {
      // Find nodes that have edges pointing TO the specified node
      const edges = await db.edges
        .where('target')
        .equals(parsed.operatorValue!)
        .toArray();
      const relatedIds = edges.map((e) => e.source);
      return nodes
        .filter((n) => relatedIds.includes(n.id))
        .map((n) => ({
          id: n.id,
          node: n,
          score: 1,
          matchedField: 'title' as const,
          highlights: [],
        }));
    }

    case 'depends': {
      // Find nodes that have 'depends-on' edges pointing to nodes matching the value
      const targetNodes = nodes.filter((n) =>
        n.title.toLowerCase().includes(parsed.operatorValue!.toLowerCase())
      );
      const targetIds = new Set(targetNodes.map((n) => n.id));
      const edges = await db.edges
        .where('type')
        .equals('depends-on' as any)
        .toArray();
      const dependsEdges = edges.filter((e) => targetIds.has(e.target));
      const dependsIds = [...new Set(dependsEdges.map((e) => e.source))];
      return nodes
        .filter((n) => dependsIds.includes(n.id))
        .map((n) => ({
          id: n.id,
          node: n,
          score: 1,
          matchedField: 'title' as const,
          highlights: [],
        }));
    }

    case 'uses': {
      // Find nodes that have 'uses' edges pointing to nodes matching the value
      const targetNodes = nodes.filter((n) =>
        n.title.toLowerCase().includes(parsed.operatorValue!.toLowerCase())
      );
      const targetIds = new Set(targetNodes.map((n) => n.id));
      const edges = await db.edges
        .where('type')
        .equals('uses' as any)
        .toArray();
      const usesEdges = edges.filter((e) => targetIds.has(e.target));
      const usesIds = [...new Set(usesEdges.map((e) => e.source))];
      return nodes
        .filter((n) => usesIds.includes(n.id))
        .map((n) => ({
          id: n.id,
          node: n,
          score: 1,
          matchedField: 'title' as const,
          highlights: [],
        }));
    }

    default:
      return searchByText(parsed.text, options);
  }
}

async function searchByText(
  text: string,
  _options: SearchOptions
): Promise<SearchResult[]> {
  if (!text.trim()) return [];

  const nodes = await db.nodes.where('isArchived').equals(0).toArray();
  const nodeMap = new Map(nodes.map((n) => [n.id, n]));

  const titleResults = flattenSearchResults(
    titleIndex?.search(text, { limit: 50 }) || []
  );
  const contentResults = flattenSearchResults(
    contentIndex?.search(text, { limit: 50 }) || []
  );
  const tagResults = flattenSearchResults(
    tagIndex?.search(text, { limit: 50 }) || []
  );

  const scoreMap = new Map<string, { score: number; matchedField: 'title' | 'plainText' | 'tags' | 'properties' }>();

  for (const { id } of titleResults) {
    const existing = scoreMap.get(id);
    if (existing) {
      existing.score += 10;
      if (existing.matchedField !== 'title') {
        existing.matchedField = 'title';
      }
    } else {
      scoreMap.set(id, { score: 10, matchedField: 'title' });
    }
  }

  for (const { id } of contentResults) {
    const existing = scoreMap.get(id);
    if (existing) {
      existing.score += 5;
    } else {
      scoreMap.set(id, { score: 5, matchedField: 'plainText' });
    }
  }

  for (const { id } of tagResults) {
    const existing = scoreMap.get(id);
    if (existing) {
      existing.score += 8;
      existing.matchedField = 'tags';
    } else {
      scoreMap.set(id, { score: 8, matchedField: 'tags' });
    }
  }

  const results: SearchResult[] = [];

  scoreMap.forEach(({ score, matchedField }, id) => {
    const node = nodeMap.get(id);
    if (node) {
      results.push({
        id: node.id,
        node,
        score,
        matchedField,
        highlights: [],
      });
    }
  });

  results.sort((a, b) => b.score - a.score);

  return results;
}

async function getFilteredNodes(options: SearchOptions): Promise<KnowledgeNode[]> {
  let nodes = options.includeArchived
    ? await db.nodes.toArray()
    : await db.nodes.where('isArchived').equals(0).toArray();

  if (options.type) {
    nodes = nodes.filter((n) => n.type === options.type);
  }

  if (options.tags && options.tags.length > 0) {
    nodes = nodes.filter((n) =>
      options.tags!.some((tag) => n.tags.includes(tag))
    );
  }

  if (options.collections && options.collections.length > 0) {
    nodes = nodes.filter((n) =>
      options.collections!.some((col) => n.collections.includes(col))
    );
  }

  return nodes;
}

function flattenSearchResults(rawResults: FlexSearchId[]): Array<{ id: string }> {
  const seen = new Set<string>();
  const docs: Array<{ id: string }> = [];

  for (const item of rawResults) {
    const id = String(item);
    if (id && !seen.has(id)) {
      seen.add(id);
      docs.push({ id });
    }
  }

  return docs;
}

export function isSearchInitialized(): boolean {
  return isInitialized;
}

export async function rebuildIndex(): Promise<void> {
  isInitialized = false;
  const nodes = await db.nodes.where('isArchived').equals(0).toArray();
  await initializeSearch(nodes);
}
