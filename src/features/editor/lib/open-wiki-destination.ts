import { findMarkdownDestinationLine } from '@/core/parser/destinations';
import { resolveTitle } from '@/core/vault/page-index';
import { createPage, getPageById } from '@/core/vault/pages';
import { useNavStore } from '@/shared/state/nav';
import { resolveInternalMarkdownPath } from '@/core/parser/markdown-links';
import { db } from '@/infrastructure/database/db';

export interface WikiDestination {
  target: string;
  heading?: string | null;
  blockId?: string | null;
}

export async function openWikiDestination({ target, heading, blockId }: WikiDestination): Promise<void> {
  const id = await resolveTitle(target) ?? (await createPage({ title: target })).id;
  await openPageDestination(id, heading, blockId);
}

export async function openMarkdownDestination(sourcePath: string, path: string, heading?: string | null, blockId?: string | null): Promise<boolean> {
  const targetPath = resolveInternalMarkdownPath(sourcePath, path);
  if (!targetPath) return false;
  const page = await db.pages.filter((candidate) => candidate.deletedAt === null && candidate.path.toLowerCase() === targetPath.toLowerCase()).first();
  if (!page) return false;
  await openPageDestination(page.id, heading, blockId);
  return true;
}

async function openPageDestination(id: string, heading?: string | null, blockId?: string | null): Promise<void> {
  const nav = useNavStore.getState();
  nav.openPage(id);
  nav.pushRecent(id);
  if (!heading && !blockId) return;
  const page = await getPageById(id);
  const line = page ? findMarkdownDestinationLine(page.content, heading, blockId) : null;
  if (!line) return;
  window.setTimeout(() => {
    window.dispatchEvent(new CustomEvent('ley:outline-jump', { detail: { line } }));
  }, 100);
}
