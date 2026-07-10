/**
 * App root. Loads settings from IndexedDB into the UI store, then renders Layout.
 */

import { useEffect, useState } from 'react';
import { Layout } from '@/app/Layout';
import { db } from '@/data/db';
import { seedIfEmpty } from '@/data/seed';
import { seedDemoContent } from '@/data/demo-content';
import { useUIStore, type Theme } from '@/store/ui';
import { useNavStore } from '@/store/nav';
import { useLiveQuery } from 'dexie-react-hooks';

export function App() {
  const setTheme = useUIStore((s) => s.setTheme);
  const [ready, setReady] = useState(false);

  // Auto-open the most recently updated page on first launch, so the user
  // lands somewhere useful. If the vault is empty, the welcome page will be
  // the first result from seedIfEmpty.
  const firstPage = useLiveQuery(async () => {
    const pages = await db.pages
      .filter((p) => p.deletedAt === null)
      .reverse()
      .sortBy('updatedAt');
    return pages[0] ?? null;
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await seedIfEmpty();
        // Demo content: auto-load only when the vault is essentially empty
        // (just the seeded Welcome page). For users with their own notes,
        // we don't surprise them with 25 extra pages — they can use the
        // Settings → "Add demo vault" button instead.
        const pageCount = await db.pages.count();
        if (!cancelled && pageCount <= 1) {
          await seedDemoContent();
        }
        const themeRow = await db.settings.get('theme');
        if (!cancelled && themeRow) {
          setTheme(themeRow.value as Theme);
        }
      } catch (e) {
        console.error('[app] init failed:', e);
      } finally {
        if (!cancelled) setReady(true);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [setTheme]);

  // Open the first page once it loads. Only fires once per app session.
  useEffect(() => {
    if (!firstPage) return;
    const nav = useNavStore.getState();
    if (nav.openTabs.length === 0) {
      nav.openPage(firstPage.id);
      nav.pushRecent(firstPage.id);
    }
    // Intentionally only run when firstPage.id changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [firstPage?.id]);

  if (!ready) return null;
  return <Layout />;
}