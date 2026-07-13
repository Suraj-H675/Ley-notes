import { beforeEach, describe, expect, it } from 'vitest';
import JSZip from 'jszip';
import { db } from '@/infrastructure/database/db';
import { resetDb } from '@/test/helpers';
import { importVaultFromFile } from './import';

describe('vault import links', () => {
  beforeEach(() => resetDb());

  it('indexes portable relative Markdown links after all files are imported', async () => {
    const zip = new JSZip();
    zip.file('docs/design.md', '# Design');
    zip.file('projects/source.md', '[Design](../docs/design.md)');
    const blob = await zip.generateAsync({ type: 'blob' });
    await importVaultFromFile(new File([blob], 'vault.zip', { type: 'application/zip' }));
    const design = await db.pages.where('path').equals('docs/design.md').first();
    const source = await db.pages.where('path').equals('projects/source.md').first();
    const link = source ? await db.links.where('sourcePageId').equals(source.id).first() : null;
    expect(link).toMatchObject({ kind: 'markdown', targetPageId: design?.id, targetTitle: 'design' });
  });
});
