/**
 * Frontmatter parser. Extracts the YAML block between the leading `---` fences
 * and parses it. Returns the body (everything after the closing fence) and the
 * parsed frontmatter object — or `{}` if there's no frontmatter.
 *
 * Edge cases we handle explicitly:
 *  - No frontmatter at all → body is the whole input, frontmatter is {}
 *  - Malformed, unclosed, and non-map frontmatter → leaves the entire source
 *    in `body` verbatim, plus an error. Callers must write that source back
 *    unchanged until it becomes valid.
 *
 * Notes on parsing: we use the `yaml` package (already in deps) — it's the
 * modern YAML 1.2 parser, has good TypeScript types, and supports the
 * Obsidian-style aliases list (`aliases: [Foo, Bar]`) natively.
 */

import YAML from 'yaml';

export interface FrontmatterResult {
  frontmatter: Record<string, unknown>;
  body: string;
  /** Set when the leading frontmatter is malformed or not a YAML map. */
  error?: string;
}

// Recognize a bare leading delimiter as an unclosed frontmatter attempt too;
// otherwise Properties could treat `---` as ordinary body text and rewrite it.
const FENCE = /^---(?:\r?\n|$)/;
// The closing fence may be the first thing after the opening fence (an empty
// properties map) or begin on a later line. Including the preceding newline in
// the match preserves the existing body slicing behavior for non-empty YAML.
const CLOSING_FENCE = /(?:^|\r?\n)---(?:\r?\n|$)/;

export function parseFrontmatter(raw: string): FrontmatterResult {
  if (!FENCE.test(raw)) {
    return { frontmatter: {}, body: raw };
  }

  // Find the closing fence — must come after a newline so we don't match
  // a single `---` thematic break inside the body.
  const afterOpen = raw.replace(FENCE, '');
  const closingMatch = afterOpen.match(CLOSING_FENCE);
  if (!closingMatch || closingMatch.index === undefined) {
    return {
      frontmatter: {},
      body: raw,
      error: 'Frontmatter is missing its closing --- fence',
    };
  }

  const yamlText = afterOpen.slice(0, closingMatch.index);
  const body = afterOpen.slice(closingMatch.index + closingMatch[0].length);

  try {
    // An empty fenced block is the conventional empty YAML map. A literal
    // `null`, list, or scalar is not a properties map and must remain raw.
    const parsed = yamlText.trim() === '' ? {} : YAML.parse(yamlText);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      return { frontmatter: {}, body: raw, error: 'Frontmatter must be a YAML map' };
    }
    return { frontmatter: parsed as Record<string, unknown>, body };
  } catch (e) {
    return {
      frontmatter: {},
      body: raw,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

/**
 * Pull just the `aliases:` field from frontmatter and normalize it to a string[].
 * Obsidian accepts either an inline list (`aliases: [a, b]`) or block-style
 * (`aliases:\n  - a\n  - b`). The `yaml` parser returns both as an array.
 */
export function getAliases(frontmatter: Record<string, unknown>): string[] {
  const v = frontmatter.aliases;
  if (Array.isArray(v)) {
    return v.filter((x): x is string => typeof x === 'string');
  }
  if (typeof v === 'string') return [v];
  return [];
}

/**
 * Pull just the `tags:` field. Obsidian's tags field is a flat array, but we
 * also support nested tags via the inline #tag syntax (handled by tags.ts).
 */
export function getFrontmatterTags(frontmatter: Record<string, unknown>): string[] {
  const v = frontmatter.tags;
  if (Array.isArray(v)) {
    return v.filter((x): x is string => typeof x === 'string');
  }
  if (typeof v === 'string') return [v];
  return [];
}

/**
 * Re-stringify frontmatter + body into the on-disk representation.
 * Body is the raw markdown after the frontmatter; the frontmatter is YAML-
 * serialized with a leading and trailing `---` fence.
 */
export function serializeFrontmatter(
  frontmatter: Record<string, unknown>,
  body: string,
): string {
  const empty = Object.keys(frontmatter).length === 0;
  if (empty) return body;

  // Drop undefined values to keep the YAML clean.
  const cleaned: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(frontmatter)) {
    if (v !== undefined) cleaned[k] = v;
  }

  const yaml = YAML.stringify(cleaned, { lineWidth: 0 }).trimEnd();
  return `---\n${yaml}\n---\n${body}`;
}
