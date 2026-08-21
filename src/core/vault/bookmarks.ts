import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';
import { nanoid } from '@/shared/lib/nanoid';

export type BookmarkAnchorKind = 'heading' | 'block';

export interface BookmarkTarget {
  kind: BookmarkAnchorKind;
  pageId: string;
  path: string;
  anchor: string;
}

export interface DestinationBookmark {
  id: string;
  title: string | null;
  target: BookmarkTarget;
  createdAt: number;
  updatedAt: number;
}

const BOOKMARKS_PREFIX = 'bookmarks:';
const MAX_BOOKMARKS = 200;

export function bookmarksDataKey(kind: string | null): string {
  return `${BOOKMARKS_PREFIX}${kind ?? 'unselected'}`;
}

export async function bookmarksKey(): Promise<string> {
  return bookmarksDataKey(await activeDataKind());
}

export async function listDestinationBookmarks(): Promise<DestinationBookmark[]> {
  return listAtKey(await bookmarksKey());
}

export function parseBookmarksSetting(value: unknown): DestinationBookmark[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter(isDestinationBookmark)
    .map((bookmark) => ({ ...bookmark, title: normalizeOptionalTitle(bookmark.title) }))
    .sort((left, right) => right.updatedAt - left.updatedAt)
    .slice(0, MAX_BOOKMARKS);
}

export async function addDestinationBookmark(target: BookmarkTarget, title: string | null = null): Promise<DestinationBookmark> {
  const normalizedTarget = normalizeTarget(target);
  const cleanTitle = normalizeOptionalTitle(title);
  const key = await bookmarksKey();
  const current = await listAtKey(key);
  const existing = current.find((bookmark) => sameTarget(bookmark.target, normalizedTarget));
  if (existing) return existing;
  const now = Date.now();
  const bookmark: DestinationBookmark = {
    id: nanoid(),
    title: cleanTitle,
    target: normalizedTarget,
    createdAt: now,
    updatedAt: now,
  };
  await db.settings.put({ key, value: [bookmark, ...current].slice(0, MAX_BOOKMARKS) });
  return bookmark;
}

export async function toggleDestinationBookmark(target: BookmarkTarget): Promise<boolean> {
  const normalizedTarget = normalizeTarget(target);
  const key = await bookmarksKey();
  const current = await listAtKey(key);
  const existing = current.find((bookmark) => sameTarget(bookmark.target, normalizedTarget));
  if (existing) {
    await db.settings.put({ key, value: current.filter((bookmark) => bookmark.id !== existing.id) });
    return false;
  }
  const now = Date.now();
  const bookmark: DestinationBookmark = { id: nanoid(), title: null, target: normalizedTarget, createdAt: now, updatedAt: now };
  await db.settings.put({ key, value: [bookmark, ...current].slice(0, MAX_BOOKMARKS) });
  return true;
}

export async function renameDestinationBookmark(id: string, title: string | null): Promise<void> {
  const cleanTitle = normalizeOptionalTitle(title);
  const key = await bookmarksKey();
  const current = await listAtKey(key);
  if (!current.some((bookmark) => bookmark.id === id)) return;
  await db.settings.put({
    key,
    value: current.map((bookmark) => bookmark.id === id
      ? { ...bookmark, title: cleanTitle, updatedAt: Date.now() }
      : bookmark),
  });
}

export async function deleteDestinationBookmark(id: string): Promise<void> {
  const key = await bookmarksKey();
  const current = await listAtKey(key);
  await db.settings.put({ key, value: current.filter((bookmark) => bookmark.id !== id) });
}

export async function removeDestinationBookmarksForPage(pageId: string): Promise<void> {
  const key = await bookmarksKey();
  const current = await listAtKey(key);
  const next = current.filter((bookmark) => bookmark.target.pageId !== pageId);
  if (next.length !== current.length) await db.settings.put({ key, value: next });
}

async function listAtKey(key: string): Promise<DestinationBookmark[]> {
  return parseBookmarksSetting((await db.settings.get(key))?.value);
}

function normalizeTarget(target: BookmarkTarget): BookmarkTarget {
  const pageId = target.pageId.trim();
  const path = target.path.trim();
  const anchor = target.anchor.trim();
  if (!pageId || !path || !anchor) throw new Error('Bookmark targets need a note and a destination.');
  if (anchor.length > 500) throw new Error('Bookmark destinations must be 500 characters or fewer.');
  return { kind: target.kind === 'block' ? 'block' : 'heading', pageId, path, anchor };
}

function normalizeOptionalTitle(title: unknown): string | null {
  if (title === null || title === undefined) return null;
  if (typeof title !== 'string') return null;
  const value = title.trim();
  if (!value) return null;
  if (value.length > 80) throw new Error('Bookmark titles must be 80 characters or fewer.');
  return value;
}

function sameTarget(left: BookmarkTarget, right: BookmarkTarget): boolean {
  return left.kind === right.kind
    && (left.pageId === right.pageId || left.path.toLowerCase() === right.path.toLowerCase())
    && left.anchor.toLowerCase() === right.anchor.toLowerCase();
}

function isDestinationBookmark(value: unknown): value is DestinationBookmark {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<DestinationBookmark>;
  return typeof candidate.id === 'string'
    && (candidate.title === null || candidate.title === undefined || (typeof candidate.title === 'string' && candidate.title.length <= 80))
    && typeof candidate.createdAt === 'number'
    && typeof candidate.updatedAt === 'number'
    && isBookmarkTarget(candidate.target);
}

function isBookmarkTarget(value: unknown): value is BookmarkTarget {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<BookmarkTarget>;
  return (candidate.kind === 'heading' || candidate.kind === 'block')
    && typeof candidate.pageId === 'string'
    && candidate.pageId.length > 0
    && typeof candidate.path === 'string'
    && candidate.path.length > 0
    && typeof candidate.anchor === 'string'
    && candidate.anchor.length > 0
    && candidate.anchor.length <= 500;
}
