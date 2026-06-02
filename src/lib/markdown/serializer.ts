import remarkParse from 'remark-parse';
import remarkStringify from 'remark-stringify';
import remarkGfm from 'remark-gfm';
import { unified } from 'unified';

const processor = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkStringify);

/**
 * Normalize a Markdown string so that a fresh parse + stringify round-trip
 * is stable. This is useful for tests, diffing, and storage normalization.
 */
export function normalizeMarkdown(markdown: string): string {
  return String(processor.processSync(markdown));
}
