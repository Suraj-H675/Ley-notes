import YAML from 'yaml';

export interface SplitResult {
  frontmatter: string | null;
  body: string;
}

/**
 * Split a markdown document into its YAML frontmatter (if any) and body.
 * The frontmatter must appear at the very top of the document, enclosed
 * in `---` delimiters on their own lines.
 */
export function splitFrontmatter(markdown: string): SplitResult {
  if (!markdown) return { frontmatter: null, body: '' };
  // Normalize line endings for matching, but preserve them in output.
  const normalized = markdown.replace(/\r\n/g, '\n');
  // Frontmatter: ^---\n...\n---\n at the start.
  const match = normalized.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!match) return { frontmatter: null, body: markdown };
  // Re-insert the original line endings if they were CRLF.
  let body = normalized.slice(match[0].length);
  let frontmatter = match[1] + '\n';
  if (markdown.includes('\r\n')) {
    body = body.replace(/\n/g, '\r\n');
    frontmatter = frontmatter.replace(/\n/g, '\r\n');
  }
  return { frontmatter, body };
}

export type FrontmatterValue =
  | string
  | number
  | boolean
  | Date
  | null
  | FrontmatterValue[]
  | { [key: string]: FrontmatterValue };

export type FrontmatterRecord = Record<string, FrontmatterValue>;

const ISO_DATE = /^\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?)?$/;

function coerceDates(value: FrontmatterValue): FrontmatterValue {
  if (value == null) return value;
  if (Array.isArray(value)) {
    return value.map((v) => coerceDates(v) as FrontmatterValue);
  }
  if (typeof value === 'object') {
    const out: Record<string, FrontmatterValue> = {};
    for (const [k, v] of Object.entries(value)) {
      out[k] = coerceDates(v as FrontmatterValue);
    }
    return out;
  }
  if (typeof value === 'string' && ISO_DATE.test(value)) {
    const d = new Date(value);
    if (!isNaN(d.getTime())) return d;
  }
  return value;
}

/**
 * Parse a YAML frontmatter string into a typed record.
 * ISO-format date strings are converted to Date objects.
 */
export function parseFrontmatter(yaml: string | null): FrontmatterRecord {
  if (!yaml || !yaml.trim()) return {};
  const parsed = YAML.parse(yaml);
  if (parsed == null) return {};
  if (typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Frontmatter must be a YAML mapping (key-value pairs)');
  }
  return coerceDates(parsed) as FrontmatterRecord;
}

/**
 * Serialize a frontmatter record and body back into a complete markdown
 * document. If `properties` is empty, returns just the body.
 */
export function stringifyFrontmatter(
  properties: FrontmatterRecord,
  body: string
): string {
  if (!properties || Object.keys(properties).length === 0) return body;
  const yaml = YAML.stringify(properties, { lineWidth: 0 }).trimEnd();
  return `---\n${yaml}\n---\n${body}`;
}
