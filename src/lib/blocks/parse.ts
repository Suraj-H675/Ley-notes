// Markdown ↔ Block parser.
//
// Block model: each top-level markdown element in a page becomes a row in
// the `blocks` table. Each block has a stable SiYuan-style ID, recorded
// as a hidden HTML comment (`<!-- bid: <id> -->`) at the end of the block's
// markdown.
//
// CommonMark quirk: when an HTML comment appears on its own line, remark
// classifies it as its own `html` block. We post-process to merge bid
// markers into the preceding (or, if leading, the following) non-bid
// block, so the user sees one logical block per markdown section.
//
// Block boundaries:
//   - markdown is split at the byte offsets reported by mdast positions.
//   - We trust mdast to identify top-level blocks (CommonMark rules).
//   - List items INSIDE a list block, paragraphs INSIDE a blockquote, etc.
//     are NOT separate blocks in v2 — they belong to their parent block.
//     Recursive granularity is deferred to v3.

import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import { unified } from 'unified';
import type {
  Root,
  RootContent,
  List,
  ListItem,
  Paragraph,
} from 'mdast';
import type { BlockType } from '@/types';
import { splitFrontmatter } from '../markdown/frontmatter';
import { extractPlainText } from '../markdown/extract-plaintext';
import { parseBlockId } from '../block-id';

export interface ParsedBlock {
  type: BlockType;
  /** Markdown source for this block, INCLUDING the bid marker (if any). */
  markdown: string;
  /** Extracted from the bid marker, or null if none. */
  blockId: string | null;
  /** Plain-text projection (no markdown syntax, no bid marker). */
  textContent: string;
}

/**
 * Regex matching a bid marker on its own line. Used to detect html blocks
 * that are actually bid markers (post-merge with neighbouring blocks).
 *
 * Allows flexible whitespace inside the comment per `parseBlockId`'s rules.
 */
const BID_MARKER_RE =
  /^<!--\s*bid:\s*(\d{14}-[a-z0-9]{7})\s*-->$/;

/** Regex matching any bid marker in any position (for stripping). */
const ANY_BID_MARKER_RE =
  /<!--\s*bid:\s*(\d{14}-[a-z0-9]{7})\s*-->/g;

/** Callout type identifiers accepted in `> [!note]` syntax. */
const CALLOUT_TYPES = [
  'note',
  'info',
  'tip',
  'warning',
  'danger',
  'example',
  'quote',
  'success',
  'failure',
  'bug',
  'question',
];
const CALLOUT_RE = new RegExp(
  `^>\\s*\\[!(${CALLOUT_TYPES.join('|')})\\]`,
  'i',
);

const parser = unified().use(remarkParse).use(remarkGfm);

interface RawBlock {
  type: BlockType;
  markdown: string;
}

export function parseMarkdownToBlocks(markdown: string): ParsedBlock[] {
  if (!markdown || !markdown.trim()) return [];

  // Frontmatter is stored separately on the node; don't produce blocks for it.
  const { body } = splitFrontmatter(markdown);
  if (!body.trim()) return [];

  const tree = parser.parse(body) as Root;

  // 1. Extract one raw block per top-level mdast node.
  const raw: RawBlock[] = [];
  for (const node of tree.children) {
    const rb = extractRawBlock(body, node);
    if (rb) raw.push(rb);
  }

  // 2. Merge bid-marker html blocks into neighbouring blocks.
  const merged = mergeBidMarkers(raw);

  // 3. Promote to ParsedBlock — extract blockId + plain text.
  return merged.map((rb) => {
    const stripped = rb.markdown.replace(ANY_BID_MARKER_RE, '');
    // `code` blocks may legitimately contain `<!-- bid: ... -->` as literal
    // code text — we must not mistake that for an actual block ID marker.
    const blockId = rb.type === 'code' ? null : parseBlockId(rb.markdown);
    return {
      type: rb.type,
      markdown: rb.markdown,
      blockId,
      textContent: extractPlainText(stripped),
    };
  });
}

function extractRawBlock(body: string, node: RootContent): RawBlock | null {
  const start = node.position?.start.offset;
  const end = node.position?.end.offset;
  if (start == null || end == null) return null;
  return {
    type: classifyNode(node, body.slice(start, end)),
    markdown: body.slice(start, end),
  };
}

function classifyNode(node: RootContent, markdown: string): BlockType {
  switch (node.type) {
    case 'heading':
      return 'heading';
    case 'paragraph': {
      // mdast treats a standalone image (e.g. `![alt](url)` on its own line)
      // as a `paragraph` node containing a single `image` child. Detect that
      // pattern and promote to type `'image'`.
      const p = node as Paragraph;
      if (p.children.length === 1 && p.children[0].type === 'image') {
        return 'image';
      }
      return 'paragraph';
    }
    case 'code':
      return 'code';
    case 'list':
      return isTaskList(node as List) ? 'task-list' : 'list';
    case 'blockquote':
      return CALLOUT_RE.test(markdown) ? 'callout' : 'quote';
    case 'thematicBreak':
      return 'divider';
    case 'image':
      return 'image';
    case 'table':
      return 'table';
    case 'html':
      // Both bid markers and other raw HTML use this classification.
      // Bid markers get merged in `mergeBidMarkers` below; non-bid html
      // remains as a standalone 'html' block.
      return 'html';
    case 'definition':
    case 'footnoteDefinition':
      // Treat definitions as paragraphs (rare; preserves content).
      return 'paragraph';
    default:
      // Unknown top-level node — preserve as a paragraph-ish block so we
      // never silently drop content.
      return 'paragraph';
  }
}

function isTaskList(list: List): boolean {
  return list.children.some(
    (item: ListItem) =>
      'checked' in item && item.checked !== null && item.checked !== undefined,
  );
}

/**
 * Merge html blocks whose content is a bid marker into the surrounding
 * non-bid blocks. Leading bids attach to the FIRST following real block;
 * trailing bids attach to the PRECEDING real block. Orphan bids (a bid
 * marker at the start of a document with no following content) are dropped.
 */
function mergeBidMarkers(raw: RawBlock[]): RawBlock[] {
  const result: RawBlock[] = [];
  let leadingBid: RawBlock | null = null;

  for (const block of raw) {
    const isBid = BID_MARKER_RE.test(block.markdown.trim());

    if (isBid && result.length === 0) {
      // Buffer leading bid markers until a real block arrives.
      if (leadingBid) {
        leadingBid = {
          type: leadingBid.type,
          markdown: leadingBid.markdown + '\n' + block.markdown,
        };
      } else {
        leadingBid = block;
      }
    } else if (isBid) {
      // Trailing bid — append to the preceding real block.
      const prev = result[result.length - 1];
      result[result.length - 1] = {
        type: prev.type,
        markdown: prev.markdown + '\n' + block.markdown,
      };
    } else if (leadingBid) {
      // Real block following a buffered leading bid — prepend it.
      result.push({
        type: block.type,
        markdown: leadingBid.markdown + '\n' + block.markdown,
      });
      leadingBid = null;
    } else {
      result.push(block);
    }
  }

  // Orphan trailing leadingBid is dropped (no content to attach to).
  return result;
}

export function blocksToMarkdown(blocks: ParsedBlock[]): string {
  if (blocks.length === 0) return '';
  // Join with a blank line. Each block's markdown already ends with its bid
  // marker (if any), so we don't add or strip anything here. Append a
  // trailing newline to follow the POSIX convention that text files end
  // with a newline — this also matches how editors save markdown files.
  return blocks.map((b) => b.markdown).join('\n\n') + '\n';
}