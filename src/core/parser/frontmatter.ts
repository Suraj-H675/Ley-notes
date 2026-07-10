/**
 * Frontmatter parser. Extracts the YAML block between the leading `---` fences
 * and parses it. Returns the body (everything after the closing fence) and the
 * parsed frontmatter object — or `{}` if there's no frontmatter.
 *
 * Edge cases we handle explicitly:
 *  - No frontmatter at all → body is the whole input, frontmatter is {}
 *  - Frontmatter with no closing fence → treated as if it doesn't exist
 *    (avoids swallowing the entire document on a typo)
 *  - Invalid YAML → returns empty frontmatter, body unchanged, plus the error
 *    in the result so callers can warn the user
 *
 * Notes on parsing: we use the `yaml` package (already in deps) — it's the
 * modern YAML 1.2 parser, has good TypeScript types, and supports the
 * Obsidian-style aliases list (`aliases: [Foo, Bar]`) natively.
 */

import YAML from 'yaml';

export interface FrontmatterResult {
  frontmatter: Record<string, unknown>;
  body: string;
  /** Set when YAML parsing failed. The body still contains the raw block. */
  error?: string;
}

const FENCE = /^---\r?\n/;
// Closing fence is `\n---` followed by an optional newline. We slice on the
// match position so anything after the fence is body. Markdown `---` thematic
// breaks inside the body are uncommon in practice and produce a YAML parse
// error which we catch and surface.
const CLOSING_FENCE = /\n---\r?\n?/;

export function parseFrontmatter(raw: string): FrontmatterResult {
  if (!FENCE.test(raw)) {
    return { frontmatter: {}, body: raw };
  }

  // Find the closing fence — must come after a newline so we don't match
  // a single `---` thematic break inside the body.
  const afterOpen = raw.replace(FENCE, '');
  const closingMatch = afterOpen.match(CLOSING_FENCE);
  if (!closingMatch || closingMatch.index === undefined) {
    // Unclosed frontmatter — bail out, treat as no frontmatter.
    return { frontmatter: {}, body: raw };
  }

  const yamlText = afterOpen.slice(0, closingMatch.index);
  const body = afterOpen.slice(closingMatch.index + closingMatch[0].length);

  try {
    const parsed = YAML.parse(yamlText) ?? {};
    if (typeof parsed !== 'object' || Array.isArray(parsed)) {
      return { frontmatter: {}, body, error: 'Frontmatter must be a YAML map' };
    }
    return { frontmatter: parsed as Record<string, unknown>, body };
  } catch (e) {
    return {
      frontmatter: {},
      body,
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