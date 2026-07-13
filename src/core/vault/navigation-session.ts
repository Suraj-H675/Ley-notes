import { activeDataKind } from '@/infrastructure/database/browser-local-vault';
import { db } from '@/infrastructure/database/db';
import type { Page } from '@/infrastructure/database/schema';
import { useNavStore } from '@/shared/state/nav';
import type { EditorPane } from '@/shared/state/nav';

interface PageReference {
  id: string;
  path: string;
}

interface NavigationSession {
  openTabs: PageReference[];
  activeTab: PageReference | null;
  primaryTab: PageReference | null;
  secondaryTab: PageReference | null;
  activePane: EditorPane;
  recentPages: PageReference[];
}

const SESSION_PREFIX = 'navigation-session:';
let activeStop: (() => Promise<void>) | null = null;

export async function restoreNavigationSession(isCurrent: () => boolean = () => true): Promise<boolean> {
  const key = await activeSessionKey();
  const pages = (await db.pages.filter((page) => page.deletedAt === null).toArray()).sort((left, right) => right.updatedAt - left.updatedAt);
  const session = parseSession((await db.settings.get(key))?.value);
  if (!isCurrent()) return false;
  const byId = new Map(pages.map((page) => [page.id, page]));
  const byPath = new Map(pages.map((page) => [page.path.toLowerCase(), page]));
  const resolve = (reference: PageReference | null): Page | null => reference ? byId.get(reference.id) ?? byPath.get(reference.path.toLowerCase()) ?? null : null;
  const openTabs = uniquePages(session?.openTabs.map(resolve).filter((page): page is Page => Boolean(page)) ?? []);
  const primary = resolve(session?.primaryTab ?? session?.activeTab ?? null);
  const secondary = resolve(session?.secondaryTab ?? null);
  const recent = uniquePages(session?.recentPages.map(resolve).filter((page): page is Page => Boolean(page)) ?? []);
  if (openTabs.length === 0 && pages[0]) openTabs.push(pages[0]);
  const primaryTab = primary && openTabs.some((page) => page.id === primary.id) ? primary.id : openTabs.at(-1)?.id ?? null;
  const secondaryTab = secondary && secondary.id !== primaryTab && openTabs.some((page) => page.id === secondary.id) ? secondary.id : null;
  const activePane = secondaryTab && session?.activePane === 'secondary' ? 'secondary' : 'primary';
  const activeTab = activePane === 'secondary' ? secondaryTab : primaryTab;
  const recentPages = recent.length > 0 ? recent.map((page) => page.id) : activeTab ? [activeTab] : [];
  useNavStore.getState().hydrate({ openTabs: openTabs.map((page) => page.id), activeTab, primaryTab, secondaryTab, activePane, recentPages });
  return Boolean(session);
}

export async function saveNavigationSession(key?: string): Promise<void> {
  const sessionKey = key ?? await activeSessionKey();
  const state = useNavStore.getState();
  const ids = [...new Set([...state.openTabs, ...state.recentPages, ...[state.activeTab, state.primaryTab, state.secondaryTab].filter((id): id is string => Boolean(id))])];
  const pages = ids.length > 0 ? await db.pages.where('id').anyOf(ids).toArray() : [];
  const byId = new Map(pages.map((page) => [page.id, page]));
  const reference = (id: string): PageReference | null => {
    const page = byId.get(id);
    return page ? { id: page.id, path: page.path } : null;
  };
  const openTabs = state.openTabs.map(reference).filter((item): item is PageReference => Boolean(item));
  const recentPages = state.recentPages.map(reference).filter((item): item is PageReference => Boolean(item));
  await db.settings.put({ key: sessionKey, value: {
    openTabs,
    activeTab: state.activeTab ? reference(state.activeTab) : null,
    primaryTab: state.primaryTab ? reference(state.primaryTab) : null,
    secondaryTab: state.secondaryTab ? reference(state.secondaryTab) : null,
    activePane: state.activePane,
    recentPages,
  } satisfies NavigationSession });
}

export async function startNavigationSession(isCurrent: () => boolean = () => true): Promise<() => void> {
  if (activeStop) await activeStop();
  const key = await activeSessionKey();
  await restoreNavigationSession(isCurrent);
  if (!isCurrent()) return () => undefined;
  let writes = Promise.resolve();
  let stopped = false;
  const unsubscribe = useNavStore.subscribe(() => {
    writes = writes.then(() => saveNavigationSession(key)).catch((error) => console.error('[navigation] Could not save workspace session', error));
  });
  const stop = async () => {
    if (stopped) return;
    stopped = true;
    unsubscribe();
    await writes;
    await saveNavigationSession(key);
  };
  activeStop = stop;
  return () => {
    if (activeStop === stop) activeStop = null;
    void stop().catch((error) => console.error('[navigation] Could not flush workspace session', error));
  };
}

export async function stopNavigationSession(): Promise<void> {
  const stop = activeStop;
  activeStop = null;
  if (stop) await stop();
}

async function activeSessionKey(): Promise<string> {
  return `${SESSION_PREFIX}${await activeDataKind() ?? 'unselected'}`;
}

function parseSession(value: unknown): NavigationSession | null {
  if (!value || typeof value !== 'object') return null;
  const candidate = value as Partial<NavigationSession>;
  if (!Array.isArray(candidate.openTabs) || !Array.isArray(candidate.recentPages)) return null;
  const valid = (reference: unknown): reference is PageReference => Boolean(reference && typeof reference === 'object' && typeof (reference as PageReference).id === 'string' && typeof (reference as PageReference).path === 'string');
  return {
    openTabs: candidate.openTabs.filter(valid),
    activeTab: valid(candidate.activeTab) ? candidate.activeTab : null,
    primaryTab: valid(candidate.primaryTab) ? candidate.primaryTab : (valid(candidate.activeTab) ? candidate.activeTab : null),
    secondaryTab: valid(candidate.secondaryTab) ? candidate.secondaryTab : null,
    activePane: candidate.activePane === 'secondary' ? 'secondary' : 'primary',
    recentPages: candidate.recentPages.filter(valid),
  };
}

function uniquePages(pages: Page[]): Page[] {
  const seen = new Set<string>();
  return pages.filter((page) => seen.has(page.id) ? false : (seen.add(page.id), true));
}
