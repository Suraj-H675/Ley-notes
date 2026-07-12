import { beforeEach, describe, expect, it } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { resetDb } from '@/test/helpers';
import { attachmentInsertion, saveAttachment } from './attachments';

describe('attachments', () => {
  beforeEach(async () => resetDb());

  it('persists a browser-local image and returns portable Markdown', async () => {
    const file = new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], 'System diagram.png', { type: 'image/png' });
    const saved = await saveAttachment('page-1', file);

    expect(saved.path).toMatch(/^attachments\/System-diagram-[a-z0-9]{6}\.png$/);
    expect(saved.markdown).toBe(`![System diagram.png](${saved.path})`);
    expect(attachmentInsertion([saved])).toBe(`\n\n${saved.markdown}\n\n`);
    const stored = await db.assets.where('pageId').equals('page-1').first();
    expect(stored?.filename).toBe(saved.path);
    expect(stored?.mimeType).toBe('image/png');
  });

  it('rejects executable and oversized attachment types', async () => {
    await expect(saveAttachment('page-1', new File(['bad'], 'script.js'))).rejects.toThrow('not supported');
  });
});
