/**
 * Vault import — accepts an Obsidian-compatible ZIP and inserts every .md
 * file as a new Page. Existing pages with the same path are skipped (no
 * overwrite — safer default). Returns the count of pages imported.
 */

import JSZip from 'jszip';
import { db } from '@/infrastructure/database/db';
import { nanoid } from '@/shared/lib/nanoid';
import {
  parseFrontmatter,
  getAliases,
} from '@/core/parser/frontmatter';
import { extractInlineTags } from '@/core/parser/tags';
import { rebuildPageLinks } from '@/core/index/backlink';
import { now } from '@/shared/lib/time';
import type { Page } from '@/infrastructure/database/schema';

export async function importVaultFromFile(file: File): Promise<number> {
  const zip = await JSZip.loadAsync(file);
  const mdFiles = Object.values(zip.files).filter(
    (f) => !f.dir && f.name.endsWith('.md'),
  );

  // Build a title→id lookup up front so wiki links can resolve during import.
  const existing = await db.pages.toArray();
  const titleToId = new Map<string, string>();
  for (const p of existing) {
    if (p.deletedAt === null) titleToId.set(p.lcTitle, p.id);
  }

  // Pass 1: insert all pages (without rebuilding cross-links yet).
  const newPages: Page[] = [];
  for (const f of mdFiles) {
    const path = safeZipPath(f.name);
    if (!path) continue;
    const text = await f.async('string');
    const { frontmatter, body } = parseFrontmatter(text);

    const titleFromPath = path
      .replace(/\.md$/, '')
      .split('/')
      .pop()!;
    const title = (frontmatter.title as string) ?? titleFromPath;
    const lc = title.toLowerCase();

    if (titleToId.has(lc)) continue; // skip duplicates

    const ts = now();
    const page: Page = {
      id: nanoid(),
      title,
      lcTitle: lc,
      path,
      content: body,
      frontmatter,
      aliases: getAliases(frontmatter),
      createdAt: ts,
      updatedAt: ts,
      deletedAt: null,
    };
    newPages.push(page);
    titleToId.set(lc, page.id);
  }

  if (newPages.length > 0) await db.pages.bulkAdd(newPages);

  // Pass 2: rebuild link rows + tag rows for each new page.
  for (const p of newPages) {
    await rebuildPageLinks(p.id, p.content);

    const inlineTags = extractInlineTags(p.content);
    const fmTags = Array.isArray(p.frontmatter.tags)
      ? (p.frontmatter.tags as unknown[]).filter((t): t is string => typeof t === 'string')
      : [];
    const tagRows: Array<{ pageId: string; tag: string; source: 'frontmatter' | 'inline' }> = [];
    for (const tag of fmTags) tagRows.push({ pageId: p.id, tag, source: 'frontmatter' });
    for (const tag of inlineTags) tagRows.push({ pageId: p.id, tag, source: 'inline' });
    if (tagRows.length > 0) await db.tags.bulkAdd(tagRows);
  }

  const existingAssets = new Set((await db.assets.toArray()).map((asset) => asset.filename));
  const ownerPageId = newPages[0]?.id ?? existing[0]?.id ?? '';
  const assetFiles = Object.values(zip.files).filter((entry) => !entry.dir && /^attachments\/.+\.(png|jpe?g|gif|webp|pdf|mp3|wav|mp4|webm)$/i.test(entry.name));
  for (const assetFile of assetFiles) {
    const filename = safeZipPath(assetFile.name);
    if (!filename || existingAssets.has(filename)) continue;
    const blob = await assetFile.async('blob');
    await db.assets.add({ id: nanoid(), pageId: ownerPageId, filename, mimeType: mimeTypeForPath(filename), blob, createdAt: now() });
    existingAssets.add(filename);
  }

  return newPages.length;
}

function safeZipPath(path: string): string | null {
  const normalized = path.replace(/\\/g, '/').replace(/^\/+/, '');
  return normalized.split('/').some((part) => !part || part === '.' || part === '..') ? null : normalized;
}

function mimeTypeForPath(path: string): string {
  const extension = path.split('.').at(-1)?.toLowerCase();
  return ({ png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp', pdf: 'application/pdf', mp3: 'audio/mpeg', wav: 'audio/wav', mp4: 'video/mp4', webm: 'video/webm' } as Record<string, string>)[extension ?? ''] ?? 'application/octet-stream';
}
