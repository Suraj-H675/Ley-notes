import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';
import { unified } from 'unified';
import { visit } from 'unist-util-visit';
import type { Root, Emphasis, Strong, InlineCode, Link, Text, Delete } from 'mdast';

export type InlineKind = 'strong' | 'em' | 'code' | 'link' | 'wikilink' | 'strike';

export interface InlineInner {
  from: number;
  to: number;
  text?: string;
}

export interface InlineRange {
  from: number;
  to: number;
  kind: InlineKind;
  inner: InlineInner;
  href?: string;
}

const parser = unified().use(remarkParse).use(remarkGfm);

function getPos(node: any): { start: number; end: number } | null {
  if (
    node?.position?.start?.offset == null ||
    node?.position?.end?.offset == null
  ) {
    return null;
  }
  return { start: node.position.start.offset, end: node.position.end.offset };
}

function getText(node: any): string {
  if (typeof node?.value === 'string') return node.value;
  if (Array.isArray(node?.children)) {
    return node.children.map((c: any) => getText(c)).join('');
  }
  return '';
}

export function parseInlineRanges(markdown: string): InlineRange[] {
  if (!markdown) return [];
  const tree = parser.parse(markdown) as Root;
  const ranges: InlineRange[] = [];

  visit(tree, (node) => {
    if (node.type === 'strong') {
      const pos = getPos(node);
      const inner = (node as Strong).children?.[0];
      const innerPos = inner ? getPos(inner) : null;
      if (pos && innerPos) {
        ranges.push({
          from: pos.start,
          to: pos.end,
          kind: 'strong',
          inner: { from: innerPos.start, to: innerPos.end, text: getText(inner) },
        });
      }
    } else if (node.type === 'emphasis') {
      const pos = getPos(node);
      const inner = (node as Emphasis).children?.[0];
      const innerPos = inner ? getPos(inner) : null;
      if (pos && innerPos) {
        ranges.push({
          from: pos.start,
          to: pos.end,
          kind: 'em',
          inner: { from: innerPos.start, to: innerPos.end, text: getText(inner) },
        });
      }
    } else if (node.type === 'inlineCode') {
      const pos = getPos(node);
      if (pos) {
        ranges.push({
          from: pos.start,
          to: pos.end,
          kind: 'code',
          inner: {
            from: pos.start + 1,
            to: pos.end - 1,
            text: (node as InlineCode).value,
          },
        });
      }
    } else if (node.type === 'link') {
      const pos = getPos(node);
      const inner = (node as Link).children?.[0];
      const innerPos = inner ? getPos(inner) : null;
      if (pos && innerPos) {
        ranges.push({
          from: pos.start,
          to: pos.end,
          kind: 'link',
          inner: { from: innerPos.start, to: innerPos.end, text: getText(inner) },
          href: (node as Link).url,
        });
      }
    } else if (node.type === 'delete') {
      const pos = getPos(node);
      const inner = (node as Delete).children?.[0];
      const innerPos = inner ? getPos(inner) : null;
      if (pos && innerPos) {
        ranges.push({
          from: pos.start,
          to: pos.end,
          kind: 'strike',
          inner: { from: innerPos.start, to: innerPos.end, text: getText(inner) },
        });
      }
    } else if (node.type === 'text') {
      // Detect wikilinks: [[Note Title]] (not a standard mdast node)
      const text = (node as Text).value;
      const wikilinkRe = /\[\[([^\]]+)\]\]/g;
      let m: RegExpExecArray | null;
      const basePos = (node as any).position?.start?.offset ?? 0;
      while ((m = wikilinkRe.exec(text)) !== null) {
        const from = basePos + m.index;
        const to = from + m[0].length;
        const label = m[1].trim();
        ranges.push({
          from,
          to,
          kind: 'wikilink',
          inner: { from: from + 2, to: to - 2, text: label },
          href: label,
        });
      }
    }
  });

  return ranges.sort((a, b) => a.from - b.from);
}
