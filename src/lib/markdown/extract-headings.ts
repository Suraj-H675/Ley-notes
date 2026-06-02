import remarkParse from 'remark-parse';
import { unified } from 'unified';
import remarkGfm from 'remark-gfm';
import { visit } from 'unist-util-visit';
import type { Heading, Root } from 'mdast';

export interface MarkdownHeading {
  level: number;
  text: string;
  offset: number;
}

function plainText(node: any): string {
  if (!node) return '';
  if (typeof node.value === 'string') {
    return node.value;
  }
  if (Array.isArray(node.children)) {
    return node.children.map((c: any) => plainText(c)).join('');
  }
  return '';
}

const parser = unified().use(remarkParse).use(remarkGfm);

export function extractHeadings(markdown: string): MarkdownHeading[] {
  if (!markdown || typeof markdown !== 'string') return [];
  const tree = parser.parse(markdown);
  const headings: MarkdownHeading[] = [];

  visit(tree, 'heading', (node: Heading) => {
    const text = node.children.map((c: any) => plainText(c)).join('');
    const cleaned = text
      .replace(/\[\[([^\]]+)\]\]/g, '$1')
      .replace(/`([^`]+)`/g, '$1')
      .replace(/\*\*([^*]+)\*\*/g, '$1')
      .replace(/\*([^*]+)\*/g, '$1')
      .replace(/_([^_]+)_/g, '$1')
      .trim();
    const offset = node.position?.start?.offset;
    if (typeof offset === 'number') {
      headings.push({
        level: node.depth,
        text: cleaned,
        offset,
      });
    }
  });

  // Setext-style: line === (h1) or --- (h2)
  visit(tree, 'paragraph', (node, index, parent) => {
    if (!parent || index == null) return;
    const next = (parent as Root).children[index + 1];
    if (!next || next.type !== 'thematicBreak') return;
    const endOffset = node.position?.end?.offset;
    const nextStart = next.position?.start?.offset;
    const startOffset = node.position?.start?.offset;
    if (typeof endOffset !== 'number' || typeof nextStart !== 'number' || typeof startOffset !== 'number') return;
    const between = markdown.slice(endOffset, nextStart);
    if (/^=+$/.test(between.trim())) {
      headings.push({ level: 1, text: plainText(node).trim(), offset: startOffset });
    } else if (/^-+$/.test(between.trim())) {
      headings.push({ level: 2, text: plainText(node).trim(), offset: startOffset });
    }
  });

  return headings.sort((a, b) => a.offset - b.offset);
}
