import { beforeEach, describe, expect, it, vi } from 'vitest';
import { attachmentInsertion, saveAttachment } from './attachments';

const writeAttachment = vi.hoisted(() => vi.fn(async () => true));

vi.mock('@/infrastructure/vault/filesystem-vault', () => ({
  writeActiveVaultAttachment: writeAttachment,
  readActiveVaultAttachment: vi.fn(async () => null),
}));

describe('attachments', () => {
  beforeEach(() => writeAttachment.mockClear());

  it('persists an image to the native vault and returns portable Markdown', async () => {
    const file = new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], 'System diagram.png', { type: 'image/png' });
    const saved = await saveAttachment('page-1', file);

    expect(saved.path).toMatch(/^attachments\/System-diagram-[a-z0-9]{6}\.png$/);
    expect(saved.markdown).toBe(`![System diagram.png](${saved.path})`);
    expect(attachmentInsertion([saved])).toBe(`\n\n${saved.markdown}\n\n`);
    expect(writeAttachment).toHaveBeenCalledWith(
      saved.path,
      expect.any(ArrayBuffer),
    );
  });

  it('rejects executable and oversized attachment types', async () => {
    await expect(saveAttachment('page-1', new File(['bad'], 'script.js'))).rejects.toThrow('not supported');
  });
});
