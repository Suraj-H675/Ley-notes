export interface MarkdownHeading {
  level: number;
  title: string;
  line: number;
}

export function extractMarkdownHeadings(content: string): MarkdownHeading[] {
  const headings: MarkdownHeading[] = [];
  let fence: string | null = null;
  content.split('\n').forEach((line, index) => {
    const marker = /^\s*(```|~~~)/.exec(line)?.[1] ?? null;
    if (marker) {
      fence = fence === null ? marker : fence === marker ? null : fence;
      return;
    }
    if (fence) return;
    const match = /^(#{1,6})\s+(.+?)\s*#*\s*$/.exec(line);
    if (match) headings.push({ level: match[1].length, title: match[2], line: index + 1 });
  });
  return headings;
}

export function findMarkdownDestinationLine(content: string, heading?: string | null, blockId?: string | null): number | null {
  if (heading) {
    const wanted = normalizeHeading(heading);
    const match = extractMarkdownHeadings(content).find((candidate) =>
      normalizeHeading(candidate.title) === wanted || headingSlug(candidate.title) === headingSlug(heading),
    );
    if (match) return match.line;
  }
  if (blockId) {
    const escaped = blockId.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const pattern = new RegExp(`(?:^|\\s)\\^${escaped}\\s*$`);
    let fence: string | null = null;
    const lines = content.split('\n');
    for (let index = 0; index < lines.length; index += 1) {
      const marker = /^\s*(```|~~~)/.exec(lines[index])?.[1] ?? null;
      if (marker) { fence = fence === null ? marker : fence === marker ? null : fence; continue; }
      if (!fence && pattern.test(lines[index])) return index + 1;
    }
  }
  return null;
}

export function extractMarkdownDestination(
  content: string,
  heading?: string | null,
  blockId?: string | null,
): string {
  const line = findMarkdownDestinationLine(content, heading, blockId);
  if (!line) return content;
  const lines = content.split('\n');
  if (blockId) return lines[line - 1];
  const headings = extractMarkdownHeadings(content);
  const current = headings.find((candidate) => candidate.line === line);
  if (!current) return content;
  const next = headings.find((candidate) => candidate.line > line && candidate.level <= current.level);
  return lines.slice(line - 1, next ? next.line - 1 : undefined).join('\n').trimEnd();
}

function normalizeHeading(value: string): string {
  return value.trim().replace(/\s+/g, ' ').toLocaleLowerCase();
}

function headingSlug(value: string): string {
  return normalizeHeading(value).replace(/[^\p{L}\p{N}\s-]/gu, '').replace(/\s+/g, '-');
}
