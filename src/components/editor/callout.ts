/**
 * Detect Obsidian-style callout blocks in markdown.
 * A callout is a sequence of consecutive `>`-prefixed lines whose first line
 * matches `> [!type] [title]`. Body lines follow on the next `>` lines.
 */

export const CALLOUT_TYPES = [
  'note',
  'tip',
  'info',
  'warning',
  'danger',
  'important',
  'example',
  'question',
  'success',
  'failure',
  'bug',
  'quote',
] as const;

export type CalloutType = (typeof CALLOUT_TYPES)[number];

export interface CalloutBlock {
  /** 1-indexed line number where the callout starts. */
  startLine: number;
  /** 1-indexed line number where the callout ends (inclusive). */
  endLine: number;
  type: CalloutType;
  title: string;
  body: string[];
}

const CALLOUT_SET: Set<string> = new Set(CALLOUT_TYPES);

// Matches `> [!type] [optional title]` at the start of a line. The
// capturing groups: 1 = type, 2 = title (with leading space trimmed).
const CALLOUT_HEADER_RE = /^>\s*\[!(\w+)\](?:\s+(.*))?$/;

function normalizeType(raw: string): CalloutType {
  const lower = raw.toLowerCase();
  return (CALLOUT_SET.has(lower) ? lower : 'note') as CalloutType;
}

function stripBlockquote(line: string): string {
  // `> body` or `>body` → `body`. We don't try to be smart about quoted `>`.
  if (line.startsWith('> ')) return line.slice(2);
  if (line.startsWith('>')) return line.slice(1);
  return line;
}

export function parseCalloutBlocks(markdown: string): CalloutBlock[] {
  if (!markdown) return [];
  const lines = markdown.split('\n');
  const blocks: CalloutBlock[] = [];
  let i = 0;
  while (i < lines.length) {
    const header = CALLOUT_HEADER_RE.exec(lines[i]);
    if (!header) {
      i++;
      continue;
    }
    const type = normalizeType(header[1]);
    const title = (header[2] ?? '').trim();
    const startLine = i + 1; // 1-indexed
    const bodyLines: string[] = [];
    let j = i + 1;
    while (j < lines.length) {
      const body = lines[j];
      if (body.startsWith('>')) {
        bodyLines.push(stripBlockquote(body));
        j++;
      } else if (body === '' && j + 1 < lines.length && lines[j + 1].startsWith('>')) {
        // A blank line within the callout is preserved as a body separator.
        bodyLines.push('');
        j++;
      } else {
        break;
      }
    }
    blocks.push({
      startLine,
      endLine: j,
      type,
      title,
      body: bodyLines,
    });
    i = j;
  }
  return blocks;
}
