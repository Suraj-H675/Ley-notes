import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';
import { nanoid } from '@/shared/lib/nanoid';

export interface SavedSearch {
  id: string;
  name: string;
  query: string;
  createdAt: number;
  updatedAt: number;
}

const SAVED_SEARCHES_PREFIX = 'saved-searches:';

export async function savedSearchesKey(): Promise<string> {
  return `${SAVED_SEARCHES_PREFIX}${await activeDataKind() ?? 'unselected'}`;
}

export async function listSavedSearches(): Promise<SavedSearch[]> {
  const value = (await db.settings.get(await savedSearchesKey()))?.value;
  if (!Array.isArray(value)) return [];
  return value.filter(isSavedSearch).sort((left, right) => right.updatedAt - left.updatedAt);
}

export async function saveSearch(name: string, query: string): Promise<SavedSearch> {
  const cleanName = validateName(name);
  const cleanQuery = validateQuery(query);
  const key = await savedSearchesKey();
  const current = await listSavedSearches();
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
  const current = await listSavedSearches();
  if (!current.some((item) => item.id === id)) return;
  await db.settings.put({ key, value: current.map((item) => item.id === id ? { ...item, name: cleanName, updatedAt: Date.now() } : item) });
}

export async function deleteSavedSearch(id: string): Promise<void> {
  const key = await savedSearchesKey();
  const current = await listSavedSearches();
  await db.settings.put({ key, value: current.filter((item) => item.id !== id) });
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
