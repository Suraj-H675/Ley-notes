import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import { unified } from 'unified';
import type { Root } from 'mdast';

const parser = unified().use(remarkParse).use(remarkGfm);

/**
 * Parse a Markdown string into a unified/remark AST (mdast Root).
 * The result includes position metadata for each node (used by
 * `extractHeadings` and other tools that need byte offsets).
 */
export function parseMarkdown(markdown: string): Root {
  return parser.parse(markdown) as Root;
}
