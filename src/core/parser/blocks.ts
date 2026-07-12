/**
 * Block-level markdown splitter. Used when we want to assign stable IDs to
 * each block for [[Page#^block-id]] references.
 *
 * Strategy: split on blank lines, then classify each chunk by its leading
 * marker. This is intentionally simple — a full CommonMark parser would be
 * heavier than we need for block IDs. We re-parse on save, so we don't need
 * to be fast.
 *
 * Indentation in lists is preserved on each block's `content` field; we
 * compute `depth` as the minimum leading-whitespace count of the block's
 * non-blank lines.
 */

import { blockId } from '@/shared/lib/nanoid';

export type BlockType =
  | 'paragraph'
  | 'heading'
  | 'list'
  | 'code'
  | 'quote'
  | 'image'
  | 'divider';

export interface ParsedBlock {
  /** Stable per content; see blockIdFromContent below. */
  id: string;
  content: string;
  type: BlockType;
  /** Indent level for the outliner (0 = top level). */
  depth: number;
}

export function splitBlocks(source: string): ParsedBlock[] {
  if (!source.trim()) return [];

  // Split on blank lines. We don't try to merge split list items — each
  // "paragraph" between blank lines becomes one block.
  const raw = source.split(/\n{2,}/);
  const blocks: ParsedBlock[] = [];

  for (const chunk of raw) {
    const trimmed = chunk.replace(/^\n+|\n+$/g, '');
    if (!trimmed) continue;

    const type = classifyBlock(trimmed);
    const depth = computeDepth(trimmed);
    blocks.push({
      id: blockIdFromContent(trimmed),
      content: trimmed,
      type,
      depth,
    });
  }

  return blocks;
}

function classifyBlock(text: string): BlockType {
  const firstLine = text.split('\n', 1)[0];

  // Divider
  if (/^[-*_]{3,}\s*$/.test(firstLine.trim())) return 'divider';

  // Heading
  if (/^#{1,6}\s+/.test(firstLine)) return 'heading';

  // Code fence — entire block wrapped in ``` or ~~~
  if (/^```/.test(firstLine) || /^~~~/.test(firstLine)) return 'code';

  // Image-only block
  if (/^!\[.*?\]\(.*?\)\s*$/.test(firstLine.trim())) return 'image';

  // Blockquote
  if (text.split('\n').every((l) => l.startsWith('>'))) return 'quote';

  // List (ordered or unordered, all lines)
  if (
    text
      .split('\n')
      .every((l) => /^\s*([-*+]|\d+\.)\s+/.test(l))
  ) {
    return 'list';
  }

  return 'paragraph';
}

function computeDepth(text: string): number {
  let min = Infinity;
  for (const line of text.split('\n')) {
    if (!line.trim()) continue;
    const lead = line.match(/^(\s*)/)![1].length;
    if (lead < min) min = lead;
  }
  return min === Infinity ? 0 : Math.floor(min / 2);
}

/**
 * Block ID derived from content hash, so the ID is stable across renames
 * and re-saves (provided the content itself doesn't change). The leading
 * timestamp part gives it a SiYuan-style "looks like a block ID" feel.
 */
function blockIdFromContent(text: string): string {
  const norm = text.replace(/\s+/g, ' ').trim();
  // Tiny FNV-1a 32-bit hash — good enough for in-vault uniqueness, no crypto needed.
  let hash = 0x811c9dc5;
  for (let i = 0; i < norm.length; i++) {
    hash ^= norm.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  const hex = (hash >>> 0).toString(16).padStart(8, '0');
  const stamp = blockId().split('-')[0]; // YYYYMMDD
  return `${stamp}-${hex}`;
}