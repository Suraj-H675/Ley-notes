export interface MarkdownHeading {
  level: number;
  title: string;
  line: number;
}

export interface MarkdownBlockReference {
  id: string;
  line: number;
  preview: string;
}

export interface EnsuredMarkdownBlockReference extends MarkdownBlockReference {
  content: string;
  changed: boolean;
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

export function extractMarkdownBlockReferences(content: string): MarkdownBlockReference[] {
  const references: MarkdownBlockReference[] = [];
  let fence: string | null = null;
  content.split('\n').forEach((line, index) => {
    const marker = /^\s*(```|~~~)/.exec(line)?.[1] ?? null;
    if (marker) {
      fence = fence === null ? marker : fence === marker ? null : fence;
      return;
    }
    if (fence) return;
    const match = /(?:^|\s)\^([\p{L}\p{N}-]+)\s*$/u.exec(line);
    if (!match) return;
    references.push({ id: match[1], line: index + 1, preview: line.slice(0, match.index).trim() || `Block on line ${index + 1}` });
  });
  return references;
}

export function ensureMarkdownBlockReference(content: string, lineNumber: number, generatedId: string): EnsuredMarkdownBlockReference {
  const lines = content.split('\n');
  if (!Number.isInteger(lineNumber) || lineNumber < 1 || lineNumber > lines.length) throw new Error('Place the cursor inside a Markdown block first.');
  if (lines[0]?.trim() === '---') {
    const frontmatterEnd = lines.findIndex((line, index) => index > 0 && line.trim() === '---');
    if (frontmatterEnd >= 0 && lineNumber <= frontmatterEnd + 1) throw new Error('YAML properties cannot be bookmarked as Markdown blocks.');
  }
  let fence: string | null = null;
  for (let index = 0; index < lineNumber; index += 1) {
    const marker = /^\s*(```|~~~)/.exec(lines[index])?.[1] ?? null;
    if (marker) {
      fence = fence === null ? marker : fence === marker ? null : fence;
    }
    if (index === lineNumber - 1 && (marker || fence)) throw new Error('Code fences cannot be bookmarked as Markdown blocks.');
  }
  const index = lineNumber - 1;
  const line = lines[index];
  if (!line.trim()) throw new Error('Blank lines cannot be bookmarked.');
  if (/^\s*#{1,6}\s+/.test(line)) throw new Error('Use the Outline bookmark action for headings.');
  const existing = /(?:^|\s)\^([\p{L}\p{N}-]+)\s*$/u.exec(line);
  if (existing) return { id: existing[1], line: lineNumber, preview: line.slice(0, existing.index).trim() || `Block on line ${lineNumber}`, content, changed: false };
  if (!/^[\p{L}\p{N}-]+$/u.test(generatedId)) throw new Error('Generated block IDs may contain only letters, numbers, and hyphens.');
  const preview = line.trim();
  lines[index] = `${line} ^${generatedId}`;
  return { id: generatedId, line: lineNumber, preview, content: lines.join('\n'), changed: true };
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
