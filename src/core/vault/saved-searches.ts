import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';
import { nanoid } from '@/shared/lib/nanoid';
import type { CollectionColumn, CollectionSort } from '@/core/index/collection';

export interface SavedSearchTable {
  columns: CollectionColumn[];
  sort: CollectionSort;
}

export interface SavedSearch {
  id: string;
  name: string;
  query: string;
  table?: SavedSearchTable;
  createdAt: number;
  updatedAt: number;
}

const SAVED_SEARCHES_PREFIX = 'saved-searches:';
const tableUpdateQueues = new Map<string, Promise<void>>();

export async function savedSearchesKey(): Promise<string> {
  return savedSearchesDataKey(await activeDataKind());
}

export function savedSearchesDataKey(kind: string | null): string {
  return `${SAVED_SEARCHES_PREFIX}${kind ?? 'unselected'}`;
}

export async function listSavedSearches(): Promise<SavedSearch[]> {
  return listSavedSearchesAtKey(await savedSearchesKey());
}

export function parseSavedSearchesSetting(value: unknown): SavedSearch[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter(isSavedSearch)
    .map((item) => ({ ...item, table: isSavedSearchTable(item.table) ? item.table : undefined }))
    .sort((left, right) => right.updatedAt - left.updatedAt);
}

export async function saveSearch(name: string, query: string): Promise<SavedSearch> {
  const cleanName = validateName(name);
  const cleanQuery = validateQuery(query);
  const key = await savedSearchesKey();
  const current = await listSavedSearchesAtKey(key);
  const existing = current.find((item) => item.query === cleanQuery);
  const now = Date.now();
  const saved: SavedSearch = existing
    ? { ...existing, name: cleanName, updatedAt: now }
    : { id: nanoid(), name: cleanName, query: cleanQuery, createdAt: now, updatedAt: now };
  const next = [saved, ...current.filter((item) => item.id !== saved.id)];
  await db.settings.put({ key, value: next });
  return saved;
}

export async function renameSavedSearch(id: string, name: string): Promise<void> {
  const cleanName = validateName(name);
  const key = await savedSearchesKey();
  const current = await listSavedSearchesAtKey(key);
  if (!current.some((item) => item.id === id)) return;
  await db.settings.put({ key, value: current.map((item) => item.id === id ? { ...item, name: cleanName, updatedAt: Date.now() } : item) });
}

export async function deleteSavedSearch(id: string): Promise<void> {
  const key = await savedSearchesKey();
  const current = await listSavedSearchesAtKey(key);
  await db.settings.put({ key, value: current.filter((item) => item.id !== id) });
}

export async function updateSavedSearchTable(id: string, table: SavedSearchTable): Promise<void> {
  const key = await savedSearchesKey();
  const queueKey = `${key}:${id}`;
  const previous = tableUpdateQueues.get(queueKey) ?? Promise.resolve();
  const next = previous.catch(() => undefined).then(() => performSavedSearchTableUpdate(key, id, table));
  tableUpdateQueues.set(queueKey, next);
  return next.finally(() => {
    if (tableUpdateQueues.get(queueKey) === next) tableUpdateQueues.delete(queueKey);
  });
}

async function performSavedSearchTableUpdate(key: string, id: string, table: SavedSearchTable): Promise<void> {
  const columns = [...new Set(table.columns.filter(isSavedCollectionColumn))].slice(0, 16);
  const sortColumn = table.sort.column === 'title' || isSavedCollectionColumn(table.sort.column)
    ? table.sort.column
    : 'title';
  const normalized: SavedSearchTable = {
    columns,
    sort: { column: sortColumn, direction: table.sort.direction === 'asc' ? 'asc' : 'desc' },
  };
  const current = await listSavedSearchesAtKey(key);
  if (!current.some((item) => item.id === id)) return;
  await db.settings.put({
    key,
    value: current.map((item) => item.id === id
      ? { ...item, table: normalized }
      : item),
  });
}

async function listSavedSearchesAtKey(key: string): Promise<SavedSearch[]> {
  return parseSavedSearchesSetting((await db.settings.get(key))?.value);
}

function validateName(name: string): string {
  const value = name.trim();
  if (!value) throw new Error('Give this search a name.');
  if (value.length > 80) throw new Error('Search names must be 80 characters or fewer.');
  return value;
}

function validateQuery(query: string): string {
  const value = query.trim();
  if (!value) throw new Error('Enter a query before saving it.');
  if (value.length > 500) throw new Error('Saved queries must be 500 characters or fewer.');
  return value;
}

function isSavedSearch(value: unknown): value is SavedSearch {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<SavedSearch>;
  return typeof candidate.id === 'string'
    && typeof candidate.name === 'string'
    && typeof candidate.query === 'string'
    && typeof candidate.createdAt === 'number'
    && typeof candidate.updatedAt === 'number';
}

function isSavedSearchTable(value: unknown): value is SavedSearchTable {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<SavedSearchTable>;
  return Array.isArray(candidate.columns)
    && candidate.columns.every(isSavedCollectionColumn)
    && candidate.columns.length <= 16
    && Boolean(candidate.sort)
    && (candidate.sort?.column === 'title' || isSavedCollectionColumn(candidate.sort?.column))
    && (candidate.sort?.direction === 'asc' || candidate.sort?.direction === 'desc');
}

function isSavedCollectionColumn(value: unknown): value is CollectionColumn {
  return typeof value === 'string' && (
    value === 'tags' ||
    value === 'path' ||
    value === 'modified' ||
    (value.startsWith('property:') && value.length > 'property:'.length)
  );
}
