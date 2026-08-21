/**
 * Domain types — single source of truth for every table in Dexie.
 * Designed for additive evolution: schema migrations add optional fields,
 * never remove or rename existing ones.
 */

export interface Page {
  /** nanoid */
  id: string;
  /** Human-readable title, unique within the vault. Used as the [[wiki]] target. */
  title: string;
  /** Pre-computed lowercase title for case-insensitive lookup. */
  lcTitle: string;
  /** Obsidian-compatible path, e.g. "folder/note.md". Slug-derived from title. */
  path: string;
  /** Raw markdown body (excluding frontmatter). */
  content: string;
  /** Parsed YAML frontmatter (the keys `aliases` and `tags` are also mirrored below). */
  frontmatter: Record<string, unknown>;
  /**
   * Present when the source begins with malformed, unclosed, or non-map YAML.
   * In that state `content` is the complete, verbatim source rather than just
   * the body, so a save cannot silently rewrite the invalid YAML.
   */
  frontmatterError?: string;
  /** Mirrored from frontmatter.aliases for fast autocomplete. */
  aliases: string[];
  /** Unix ms. */
  createdAt: number;
  /** Unix ms. */
  updatedAt: number;
  /** Soft delete marker — null = live. */
  deletedAt: number | null;
  /** SHA-256 of the complete on-disk source, used only for safe rename matching. */
  sourceHash?: string;
  /**
   * A transient filesystem-vault projection retained only while its tab is
   * open after an external deletion. It is intentionally not indexed.
   */
  missingFromDisk?: boolean;
}

export type BlockType =
  | 'paragraph'
  | 'heading'
  | 'list'
  | 'code'
  | 'quote'
  | 'image'
  | 'divider';

export interface Block {
  /** YYYYMMDD-xxxxxx, like SiYuan. Stable across saves if content unchanged. */
  id: string;
  pageId: string;
  parentId: string | null;
  order: number;
  content: string;
  type: BlockType;
  /** Indent level for the outliner (0-based). */
  depth: number;
  createdAt: number;
  updatedAt: number;
}

export interface Link {
  id: string;
  sourcePageId: string;
  sourceBlockId: string | null;
  /** Title as written in the source, before resolution. */
  targetTitle: string;
  /** null when the link points to an uncreated page (ghost link). */
  targetPageId: string | null;
  kind: LinkKind;
  /** Char offset in the source page's raw content. */
  position: number;
}

export type LinkKind = 'wiki' | 'embed' | 'markdown';

export interface Tag {
  pageId: string;
  /** Full nested path, e.g. "project/ley/architecture". */
  tag: string;
  source: 'frontmatter' | 'inline';
}

export interface Asset {
  id: string;
  pageId: string;
  filename: string;
  mimeType: string;
  blob: Blob;
  createdAt: number;
}

export interface Revision {
  id: string;
  pageId: string;
  content: string;
  /** Sparse — only written on user-initiated checkpoints, not every keystroke. */
  createdAt: number;
}

export interface Setting {
  key: string;
  value: unknown;
}
