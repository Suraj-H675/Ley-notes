import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import { unified } from 'unified';
import { visit } from 'unist-util-visit';
import type { Root, Code } from 'mdast';

function getNodeText(node: any): string {
  if (!node) return '';
  if (typeof node.value === 'string') return node.value;
  if (Array.isArray(node.children)) {
    return node.children.map((c: any) => getNodeText(c)).join('');
  }
  return '';
}

const parser = unified().use(remarkParse).use(remarkGfm);

export function extractPlainText(markdown: string): string {
  if (!markdown || typeof markdown !== 'string') return '';
  const tree = parser.parse(markdown);
  const out: string[] = [];

  visit(tree as Root, (node, _index, parent) => {
    // Skip code block content (handled by 'code' node below).
    if (parent && (parent as any).type === 'code') return;

    switch (node.type) {
      case 'heading': {
        out.push(getNodeText(node));
        out.push('\n\n');
        break;
      }
      case 'paragraph': {
        // Skip paragraphs nested inside listItem — already collected via listItem.
        let p: any = parent;
        let inList = false;
        while (p) {
          if (p.type === 'listItem') {
            inList = true;
            break;
          }
          p = p.parent ?? null;
        }
        if (inList) break;
        out.push(getNodeText(node));
        out.push('\n\n');
        break;
      }
      case 'list': {
        // items are handled recursively; do nothing here
        break;
      }
      case 'listItem': {
        out.push(getNodeText(node));
        out.push('\n');
        break;
      }
      case 'code': {
        out.push((node as Code).value);
        out.push('\n\n');
        break;
      }
      case 'blockquote': {
        // body content is in children; visited recursively
        break;
      }
      case 'thematicBreak': {
        out.push('\n\n');
        break;
      }
      case 'image': {
        out.push((node as any).alt ?? '');
        break;
      }
      case 'html': {
        // drop raw HTML
        break;
      }
      // Inline nodes: skip; they are gathered by getNodeText on their parent
      case 'text':
      case 'inlineCode':
      case 'link':
      case 'strong':
      case 'emphasis':
      case 'break':
        break;
      default:
        break;
    }
  });

  return out
    .join('')
    .replace(/\[\[([^\]]+)\]\]/g, '$1') // safety: residual wikilink brackets
    .replace(/`/g, '')                  // safety: residual backticks
    .replace(/[ \t]+\n/g, '\n')         // trim trailing ws
    .replace(/\n{3,}/g, '\n\n')         // collapse 3+ blank lines to 2
    .trim();
}
