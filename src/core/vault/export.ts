/**
 * Vault export — produces an Obsidian-compatible ZIP. Each page becomes one
 * .md file at `path`. Frontmatter is included as YAML fences. The .obsidian
 * folder is optional and not generated here (it's per-user config).
 *
 * Why a ZIP rather than a folder pick? Browser File System Access API is
 * not universally available; ZIP works in every modern browser.
 */

import JSZip from 'jszip';
import { db } from '@/data/db';
import { serializeFrontmatter } from '@/core/parser/frontmatter';

export async function exportVault(): Promise<Blob> {
  const zip = new JSZip();
  const pages = await db.pages.toArray();
  const live = pages.filter((p) => p.deletedAt === null);

  for (const p of live) {
    const body = serializeFrontmatter(p.frontmatter, p.content);
    zip.file(p.path, body);
  }

  // Drop a manifest so the exporter knows it was a Ley vault.
  const manifest = {
    generator: 'Ley Notes',
    version: '1.0',
    exportedAt: new Date().toISOString(),
    pageCount: live.length,
  };
  zip.file('.ley-manifest.json', JSON.stringify(manifest, null, 2));

  return zip.generateAsync({ type: 'blob' });
}