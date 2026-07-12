import { db } from '@/infrastructure/database/db';
import { nanoid } from '@/shared/lib/nanoid';
import {
  getActiveVaultKind,
  readActiveVaultAttachment,
  writeActiveVaultAttachment,
} from '@/infrastructure/vault/filesystem-vault';

const MAX_ATTACHMENT_BYTES = 50 * 1024 * 1024;
const ALLOWED_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'pdf', 'mp3', 'wav', 'mp4', 'webm']);

export interface SavedAttachment {
  path: string;
  markdown: string;
  kind: 'image' | 'file';
}

export function attachmentInsertion(attachments: SavedAttachment[]): string {
  return `\n\n${attachments.map((attachment) => attachment.markdown).join('\n\n')}\n\n`;
}

export async function saveAttachment(pageId: string, file: File): Promise<SavedAttachment> {
  if (file.size > MAX_ATTACHMENT_BYTES) throw new Error('Attachments larger than 50 MB are not supported yet.');
  const extension = file.name.split('.').at(-1)?.toLowerCase() ?? '';
  if (!ALLOWED_EXTENSIONS.has(extension)) throw new Error(`.${extension || 'unknown'} files are not supported as attachments.`);

  const filename = `${safeStem(file.name.replace(/\.[^.]+$/, ''))}-${nanoid().slice(0, 6)}.${extension}`;
  const path = `attachments/${filename}`;
  const data = await file.arrayBuffer();
  const isFilesystemVault = await writeActiveVaultAttachment(path, data);

  if (!isFilesystemVault) {
    await db.assets.add({
      id: nanoid(),
      pageId,
      filename: path,
      mimeType: file.type || mimeTypeForPath(path),
      blob: new Blob([data], { type: file.type || mimeTypeForPath(path) }),
      createdAt: Date.now(),
    });
  }

  const kind = isImagePath(path) ? 'image' : 'file';
  const label = escapeMarkdownLabel(file.name);
  return {
    path,
    kind,
    markdown: kind === 'image' ? `![${label}](${path})` : `[${label}](${path})`,
  };
}

export async function attachmentObjectUrl(path: string): Promise<string | null> {
  if (!isSafeAttachmentPath(path)) return null;
  if (getActiveVaultKind()) {
    const data = await readActiveVaultAttachment(path);
    return data ? URL.createObjectURL(new Blob([data], { type: mimeTypeForPath(path) })) : null;
  }
  const asset = await db.assets.filter((candidate) => candidate.filename === path).first();
  return asset ? URL.createObjectURL(asset.blob) : null;
}

export function isImagePath(path: string): boolean {
  return /\.(png|jpe?g|gif|webp)$/i.test(path);
}

export function isSafeAttachmentPath(path: string): boolean {
  return path.startsWith('attachments/') && !path.split('/').some((part) => !part || part === '.' || part === '..');
}

function safeStem(value: string): string {
  const stem = value.normalize('NFKC').replace(/[^\p{L}\p{N}._-]+/gu, '-').replace(/^-+|-+$/g, '');
  return stem || 'attachment';
}

function escapeMarkdownLabel(value: string): string {
  return value.replaceAll('\\', '\\\\').replaceAll('[', '\\[').replaceAll(']', '\\]');
}

function mimeTypeForPath(path: string): string {
  const extension = path.split('.').at(-1)?.toLowerCase();
  return ({
    png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp',
    pdf: 'application/pdf', mp3: 'audio/mpeg', wav: 'audio/wav', mp4: 'video/mp4', webm: 'video/webm',
  } as Record<string, string>)[extension ?? ''] ?? 'application/octet-stream';
}
