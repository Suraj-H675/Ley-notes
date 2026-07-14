import type { CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import { db } from '@/infrastructure/database/db';

export interface TagCompletionMatch {
  from: number;
  query: string;
}

export async function tagCompletions(
  context: CompletionContext,
): Promise<CompletionResult | null> {
  const match = findTagCompletion(context.state.doc.toString(), context.pos);
  if (!match) return null;
  const rows = await db.tags.toArray();
  if (context.aborted) return null;
  const counts = new Map<string, number>();
  for (const row of rows) counts.set(row.tag, (counts.get(row.tag) ?? 0) + 1);
  const query = match.query.toLowerCase();
  const options = [...counts.entries()]
    .filter(([tag]) => !query || tag.includes(query))
    .sort(
      ([left, leftCount], [right, rightCount]) =>
        relevance(left, query) - relevance(right, query) ||
        rightCount - leftCount ||
        left.localeCompare(right),
    )
    .slice(0, 50)
    .map(([tag, count]) => ({
      label: `#${tag}`,
      apply: tag,
      detail: `${count} ${count === 1 ? 'note' : 'notes'}`,
      type: 'keyword',
    }));
  if (options.length === 0) return null;
  return { from: match.from, options, validFor: /^[a-z0-9_/-]*$/ };
}

export function findTagCompletion(source: string, position: number): TagCompletionMatch | null {
  if (
    position < 0 ||
    position > source.length ||
    isFrontmatterPosition(source, position) ||
    isFencedPosition(source, position)
  )
    return null;
  const lineStart = source.lastIndexOf('\n', Math.max(0, position - 1)) + 1;
  const before = source.slice(lineStart, position);
  if (isInsideInlineCode(before)) return null;
  const match = /(?:^|[^\w`/#=])#([a-z0-9_/-]*)$/.exec(before);
  if (!match) return null;
  return { from: position - match[1].length, query: match[1] };
}

function relevance(tag: string, query: string): number {
  if (!query || tag === query) return 0;
  if (tag.startsWith(query)) return 1;
  if (tag.split('/').some((segment) => segment.startsWith(query))) return 2;
  return 3;
}

function isFrontmatterPosition(source: string, position: number): boolean {
  if (!source.startsWith('---\n')) return false;
  const closing = source.indexOf('\n---', 4);
  return closing < 0 ? position >= 0 : position <= closing + 4;
}

function isFencedPosition(source: string, position: number): boolean {
  const lines = source.slice(0, position).split('\n');
  let marker: '```' | '~~~' | null = null;
  for (const line of lines) {
    const fence = /^\s*(```|~~~)/.exec(line)?.[1] as '```' | '~~~' | undefined;
    if (!fence) continue;
    if (!marker) marker = fence;
    else if (marker === fence) marker = null;
  }
  return marker !== null;
}

function isInsideInlineCode(before: string): boolean {
  let markerLength = 0;
  for (let index = 0; index < before.length; ) {
    if (before[index] !== '`' || before[index - 1] === '\\') {
      index += 1;
      continue;
    }
    let length = 1;
    while (before[index + length] === '`') length += 1;
    if (markerLength === 0) markerLength = length;
    else if (markerLength === length) markerLength = 0;
    index += length;
  }
  return markerLength > 0;
}
