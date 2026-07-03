// Block-level data model — see /home/suraj/.claude/plans/abstract-wishing-falcon.md
// for the full architecture plan.
//
// A block is a top-level markdown element within a page. Each block has a
// stable SiYuan-style ID (`YYYYMMDDHHMMSS-xxxxxxx`) and lives as a row in
// the `blocks` table, derived from `nodes.content` markdown on every save.
// Blocks are NOT the source of truth — markdown is. See lib/block-id.ts
// for the ID format and lib/blocks/parse.ts for the markdown parser.

/**
 * The block types we recognise. Top-level only in v2 — list items inside a
 * list block, paragraphs inside a blockquote, etc. are part of the parent
 * block's markdown. We split recursively in v3 if user demand justifies it.
 */
export type BlockType =
  | 'paragraph'
  | 'heading'
  | 'code'
  | 'list'
  | 'task-list'
  | 'quote'
  | 'divider'
  | 'image'
  | 'callout'
  | 'table'
  | 'html'
  | 'math'
  | 'embed';

/**
 * A single top-level block within a page.
 *
 * `markdown` is the source-of-truth representation of the block. It MUST end
 * with a `<!-- bid: <id> -->` marker; the parser/serializer maintain this
 * invariant via `lib/block-id.ts`. `textContent` is a pre-computed plain-text
 * projection used for search and link previews.
 */
export interface KnowledgeBlock {
  /** Block ID — format enforced by `lib/block-id.ts`. */
  id: string;
  /** FK to `nodes.id` — the page this block belongs to. */
  nodeId: string;
  /** Sibling order within the page (low = first). Sparse integers allow cheap insert. */
  order: number;
  type: BlockType;
  /**
   * The markdown source for this block, ending with a `<!-- bid: <id> -->`
   * marker. May be empty for divider blocks.
   */
  markdown: string;
  /** Plain-text projection (no markdown syntax, no block-id marker). */
  textContent: string;
  createdAt: number;
  updatedAt: number;
}

/**
 * Reference kinds:
 * - `page-ref`:    a `[[Page Title]]` link (target resolves to a node)
 * - `block-ref`:   a `((block-id))` link (target resolves to a specific block)
 * - `embed`:       a `![[Title]]` or `!((block-id))` transclusion/embed
 */
export type RefLinkType = 'page-ref' | 'block-ref' | 'embed';

/**
 * Denormalized backlink index row. One per `[[Title]]` / `((id))` /
 * `![[Title]]` / `!((id))` reference in a block's markdown.
 *
 * For `page-ref` and `embed` (when target is a page), either `targetNodeId`
 * or `targetNodeTitle` is populated:
 *   - `targetNodeId` is set when the title resolves to an existing page.
 *   - `targetNodeTitle` is always set (the user's typed title), so unresolved
 *     links can be surfaced ("Create page 'New Idea'?").
 *
 * For `block-ref` and `embed` (when target is a block), `targetBlockId` is set.
 */
export interface RefRecord {
  id: string;
  /** The block containing the link. */
  sourceBlockId: string;
  /** Resolved target node (when the link points to an existing page). */
  targetNodeId: string | null;
  /** Original title typed by the user (always set for `page-ref`/`embed`-to-page). */
  targetNodeTitle: string | null;
  /** Resolved target block (when the link points to a specific block). */
  targetBlockId: string | null;
  linkType: RefLinkType;
  /**
   * First ~200 chars of the source block, used for backlink previews.
   * Pre-computed so the backlinks panel doesn't re-parse markdown on every render.
   */
  context: string;
  createdAt: number;
}

/**
 * Per-block typed key-value metadata. Powers inline syntax like
 * `tag:: research`, `due:: 2026-07-04`, `lang:: js` (parsed in Phase 9).
 *
 * Keys are case-insensitive at query time (use `[name+value]` index with
 * lowercased names). Values are strings — complex values should be JSON-
 * encoded by the caller.
 */
export interface BlockAttribute {
  id: string;
  blockId: string;
  /** Lowercase, kebab-case. E.g., 'tag', 'due', 'lang', 'priority'. */
  name: string;
  value: string;
  createdAt: number;
}