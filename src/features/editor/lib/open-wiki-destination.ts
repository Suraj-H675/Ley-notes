import { findMarkdownDestinationLine } from '@/core/parser/destinations';
import { resolveTitle } from '@/core/vault/page-index';
import { createPage, getPageById } from '@/core/vault/pages';
import { useNavStore } from '@/shared/state/nav';

export interface WikiDestination {
  target: string;
  heading?: string | null;
  blockId?: string | null;
}

export async function openWikiDestination({ target, heading, blockId }: WikiDestination): Promise<void> {
  const id = await resolveTitle(target) ?? (await createPage({ title: target })).id;
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
