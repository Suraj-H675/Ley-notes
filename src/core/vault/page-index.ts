/**
 * Lightweight page index that lives outside React. The React tree pushes
 * pages into this index on every change; non-React code (e.g. CodeMirror
 * view plugins for autocomplete) reads from it synchronously.
 *
 * Why a bridge instead of querying Dexie from the autocomplete:
 *  - Autocomplete runs in a CM6 keydown handler (synchronous).
 *  - Dexie queries are async.
 *  - We refresh the bridge via Dexie's liveQuery in the React app, so the
 *    autocomplete sees consistent, up-to-date data without needing await.
 */

import { liveQuery } from 'dexie';
import { db } from '@/infrastructure/database/db';

interface IndexEntry {
  id: string;
  title: string;
  display: string;
  lcTitle: string;
  aliases: string[];
  updatedAt: number;
}

let entries: IndexEntry[] = [];

const refreshSubs = new Set<() => void>();

function notify() {
  for (const cb of refreshSubs) cb();
}

/** Start the live subscription that keeps the index in sync. Idempotent. */
let sub: import('dexie').Subscription | null = null;
let consumers = 0;
export function startPageIndex(): () => void {
  consumers += 1;
  if (!sub) {
    sub = liveQuery(() => db.pages.toArray()).subscribe({
      next: (pages) => {
        entries = pages
          .filter((p) => p.deletedAt === null && !p.missingFromDisk)
          .map((p) => ({
            id: p.id,
            title: p.title,
            display: p.title,
            lcTitle: p.lcTitle,
            aliases: p.aliases,
            updatedAt: p.updatedAt,
          }));
        notify();
      },
      error: (err) => {
        console.error('[page-index] liveQuery error:', err);
      },
    });
  }

  let stopped = false;
  return () => {
    if (stopped) return;
    stopped = true;
    consumers = Math.max(0, consumers - 1);
    if (consumers === 0) {
      sub?.unsubscribe();
      sub = null;
    }
  };
}

export function getPageIndex(): IndexEntry[] {
  return entries;
}

export function subscribePageIndex(cb: () => void): () => void {
  refreshSubs.add(cb);
  return () => refreshSubs.delete(cb);
}

/**
 * Synchronous resolve by title or alias. Use only for UI suggestions where
 * stale-by-one-render is acceptable.
 */
export function resolveTitleSync(title: string): string | null {
  const lc = title.toLowerCase();
  for (const e of entries) {
    if (e.lcTitle === lc) return e.id;
    if (e.aliases.some((a) => a.toLowerCase() === lc)) return e.id;
  }
  return null;
}

/**
 * Async resolve against the live Dexie DB. Use this when correctness matters
 * (e.g. rebuilding backlink index right after a write — the bridge might be
 * one tick behind).
 */
export async function resolveTitle(title: string): Promise<string | null> {
  const lc = title.toLowerCase();
  const byTitle = await db.pages.where('lcTitle').equals(lc).first();
  if (byTitle && byTitle.deletedAt === null && !byTitle.missingFromDisk) return byTitle.id;
  const all = await db.pages.toArray();
  for (const p of all) {
    if (p.deletedAt !== null || p.missingFromDisk) continue;
    if (p.aliases.some((a) => a.toLowerCase() === lc)) return p.id;
  }
  return null;
}
