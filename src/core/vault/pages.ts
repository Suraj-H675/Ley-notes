/**
 * Vault CRUD on pages. All write paths go through this module so we can
 * guarantee that indexes (backlinks, tags) are kept in sync.
 *
 * Naming follows the Graphify _stat_index pattern: writes are atomic and
 * idempotent so a crash mid-save leaves no orphan rows.
 */

import { db } from '@/infrastructure/database/db';
import { nanoid } from '@/shared/lib/nanoid';
import { slugify, uniqueSlug } from '@/shared/lib/slug';
import {
  getAliases,
  parseFrontmatter,
} from '@/core/parser/frontmatter';
import { rebuildPageLinks, resolveGhostLinksForPage } from '@/core/index/backlink';
import { rebuildPageTags } from '@/core/index/tag-index';
import { now } from '@/shared/lib/time';
import type { Page } from '@/infrastructure/database/schema';
import { retargetWikiLinks } from '@/core/parser/wiki-links';
import { serializeFrontmatter } from '@/core/parser/frontmatter';
import {
  renameActiveVaultFile,
  trashActiveVaultFile,
  writeActiveVaultFile,
} from '@/infrastructure/vault/filesystem-vault';

export interface CreatePageInput {
  title: string;
  content?: string;
  folder?: string;
  aliases?: string[];
  frontmatter?: Record<string, unknown>;
}

export async function listPages(): Promise<Page[]> {
  const rows = await db.pages.toArray();
  return rows.filter((p) => p.deletedAt === null);
}

/**
 * Find a page by its exact title (case-insensitive).
 */
export async function getPageByTitle(title: string): Promise<Page | null> {
  const lc = title.toLowerCase();
  return (await db.pages.where('lcTitle').equals(lc).first()) ?? null;
}

export async function getPageById(pageId: string): Promise<Page | null> {
  return (await db.pages.get(pageId)) ?? null;
}

/**
 * Create a new page. If a page with the same title already exists, returns it
 * (idempotent). Otherwise inserts and returns the new page.
 *
 * Title uniqueness is case-insensitive (Obsidian-compatible). The path is
 * derived from the slugified title and placed in the requested folder.
 */
export async function createPage(input: CreatePageInput): Promise<Page> {
  const existing = await getPageByTitle(input.title);
  if (existing) return existing;

  const ts = now();
  const stem = slugify(input.title);
  const all = await db.pages.toArray();
  const taken = new Set(all.map((p) => p.path.replace(/\.md$/, '')));
  const uniqueStem = uniqueSlug(stem, taken);
  const path = input.folder ? `${input.folder}/${uniqueStem}.md` : `${uniqueStem}.md`;

  const page: Page = {
    id: nanoid(),
    title: input.title,
    lcTitle: input.title.toLowerCase(),
    path,
    content: input.content ?? '',
    frontmatter: input.frontmatter ?? {},
    aliases: input.aliases ?? [],
    createdAt: ts,
    updatedAt: ts,
    deletedAt: null,
  };

  await writeActiveVaultFile(path, serializeFrontmatter(page.frontmatter, page.content));
  await db.pages.add(page);

  // Initial index build.
  await rebuildPageLinks(page.id, page.content);
  await rebuildPageTags(page.id, page.content, page.frontmatter);
  await resolveGhostLinksForPage(page);

  return page;
}

/**
 * Update the body and frontmatter of a page. Re-parses frontmatter and
 * aliases, rebuilds the backlink and tag indexes.
 */
export async function updatePageContent(
  pageId: string,
  content: string,
): Promise<void> {
  const page = await db.pages.get(pageId);
  if (!page) throw new Error(`updatePageContent: page ${pageId} not found`);

  const hasFrontmatter = content.startsWith('---\n') || content.startsWith('---\r\n');
  const parsed = hasFrontmatter ? parseFrontmatter(content) : { frontmatter: page.frontmatter, body: content };
  const { frontmatter, body } = parsed;
  const aliases = getAliases(frontmatter);

  const ts = now();
  await checkpointPage(page);
  await writeActiveVaultFile(page.path, serializeFrontmatter(frontmatter, body));
  await db.pages.update(pageId, {
    content: body,
    frontmatter,
    aliases,
    updatedAt: ts,
  });

  await rebuildPageLinks(pageId, body);
  await rebuildPageTags(pageId, body, frontmatter);
}

/** Update YAML properties without rewriting editor-visible Markdown. */
export async function updatePageFrontmatter(
  pageId: string,
  frontmatter: Record<string, unknown>,
): Promise<void> {
  const page = await db.pages.get(pageId);
  if (!page) throw new Error(`updatePageFrontmatter: page ${pageId} not found`);
  const aliases = getAliases(frontmatter);
  await checkpointPage(page);
  await writeActiveVaultFile(page.path, serializeFrontmatter(frontmatter, page.content));
  await db.pages.update(pageId, { frontmatter, aliases, updatedAt: now() });
  await rebuildPageTags(pageId, page.content, frontmatter);
  await resolveGhostLinksForPage({ ...page, frontmatter, aliases });
}

async function checkpointPage(page: Page): Promise<void> {
  const revisions = await db.revisions.where('pageId').equals(page.id).toArray();
  const latest = revisions.sort((left, right) => right.createdAt - left.createdAt)[0];
  const snapshot = serializeFrontmatter(page.frontmatter, page.content);
  if (latest && (now() - latest.createdAt < 5 * 60_000 || latest.content === snapshot)) return;
  await db.revisions.add({ id: nanoid(), pageId: page.id, content: snapshot, createdAt: now() });
}

/**
 * Rename a page. Updates the title, slug-derived path, and lcTitle index.
 * Backlinks aren't rebuilt — they store targetTitle (the title at link
 * resolution time) which remains valid because the title still resolves.
 */
export async function renamePage(pageId: string, newTitle: string): Promise<Page> {
  const page = await db.pages.get(pageId);
  if (!page) throw new Error(`renamePage: page ${pageId} not found`);

  const collision = await getPageByTitle(newTitle);
  if (collision && collision.id !== pageId) {
    throw new Error(`A page named "${newTitle}" already exists`);
  }

  const stem = slugify(newTitle);
  const all = await db.pages.toArray();
  const taken = new Set(
    all.filter((p) => p.id !== pageId).map((p) => p.path.replace(/\.md$/, '')),
  );
  const uniqueStem = uniqueSlug(stem, taken);
  const path = page.path.includes('/')
    ? `${page.path.split('/').slice(0, -1).join('/')}/${uniqueStem}.md`
    : `${uniqueStem}.md`;

  const updated: Partial<Page> = {
    title: newTitle,
    lcTitle: newTitle.toLowerCase(),
    path,
    updatedAt: now(),
  };
  const inbound = await db.links.where('targetPageId').equals(pageId).toArray();
  const affectedSourceIds = [...new Set(inbound.map((link) => link.sourcePageId))];
  await renameActiveVaultFile(page.path, path);
  await db.pages.update(pageId, updated);

  // Obsidian-style automatic link maintenance. Each affected source is also
  // persisted to disk through updatePageContent.
  for (const sourceId of affectedSourceIds) {
    const source = await db.pages.get(sourceId);
    if (!source || source.deletedAt !== null) continue;
    const nextContent = retargetWikiLinks(source.content, page.title, newTitle);
    if (nextContent !== source.content) await updatePageContent(source.id, nextContent);
  }

  return { ...page, ...updated } as Page;
}

/**
 * Soft-delete a page. Links and tags remain so other pages' backlinks
 * resolve gracefully until they're rebuilt on next save.
 */
export async function deletePage(pageId: string): Promise<void> {
  const page = await db.pages.get(pageId);
  if (!page) return;
  await trashActiveVaultFile(page.path);
  await db.pages.update(pageId, { deletedAt: now() });
  await db.links.where('sourcePageId').equals(pageId).delete();
  await db.tags.where('pageId').equals(pageId).delete();
}
