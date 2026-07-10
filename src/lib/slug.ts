/**
 * Slug helpers. Title → Obsidian-compatible filename stem.
 * Pure functions, no I/O.
 */

const SLUG_RE = /[^a-z0-9]+/g;

export function slugify(title: string): string {
  return title
    .toLowerCase()
    .normalize('NFKD')
    // Strip combining marks (diacritics) using Unicode property escapes.
    .replace(/\p{M}+/gu, '')
    .replace(SLUG_RE, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 80) || 'untitled';
}

/**
 * Make a slug unique against an existing set. Returns "Foo" if "Foo" not taken,
 * else "Foo 2", "Foo 3", etc.
 */
export function uniqueSlug(base: string, taken: Set<string>): string {
  if (!taken.has(base)) return base;
  let n = 2;
  while (taken.has(`${base}-${n}`)) n++;
  return `${base}-${n}`;
}