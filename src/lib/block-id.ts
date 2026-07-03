// Block ID utilities.
//
// Block ID format: 'YYYYMMDDHHMMSS-xxxxxxx'
//   - 14 digits: UTC timestamp (year, month, day, hour, minute, second)
//   - dash separator
//   - 7 chars: random hash from [a-z0-9]
//
// Examples: '20200812220555-lj3enxa', '20260703041255-m0z9a4k'
//
// Block markers in markdown: `<!-- bid: <id> -->`
//   - Hidden HTML comment at end of a block's markdown
//   - Whitespace inside the comment is flexible (e.g., `<!--bid:foo-->` works)
//   - When multiple markers appear, the LAST one is the authoritative ID
//     (allows the earlier markers to be stale without breaking parsing)
//
// The 7-char random suffix gives 36^7 ≈ 78 billion combinations. Collision
// is only possible if two IDs are generated in the same UTC second AND the
// random draws collide. With 1000 IDs/sec the birthday-collision probability
// is ~6.4×10⁻⁶ over an hour. Safe for personal-scale workloads.

const HASH_CHARS = 'abcdefghijklmnopqrstuvwxyz0123456789';

/** Pattern that matches a valid block ID exactly. */
export const BLOCK_ID_PATTERN = /^[0-9]{14}-[a-z0-9]{7}$/;

/** Non-global regex for `.test()` and `.replace()` (avoid lastIndex state). */
const BLOCK_ID_MARKER_RE =
  /<!--\s*bid:\s*([0-9]{14}-[a-z0-9]{7})\s*-->/;

/** Global regex for `.matchAll()` (we want every match). */
const BLOCK_ID_MARKER_RE_GLOBAL =
  /<!--\s*bid:\s*([0-9]{14}-[a-z0-9]{7})\s*-->/g;

function pad(n: number, width: number): string {
  return n.toString().padStart(width, '0');
}

/**
 * Generate a new block ID using the current (or supplied) UTC time.
 * The 7-character random suffix makes collisions effectively impossible
 * for personal-scale workloads.
 */
export function generateBlockId(now: Date = new Date()): string {
  const ts =
    now.getUTCFullYear().toString() +
    pad(now.getUTCMonth() + 1, 2) +
    pad(now.getUTCDate(), 2) +
    pad(now.getUTCHours(), 2) +
    pad(now.getUTCMinutes(), 2) +
    pad(now.getUTCSeconds(), 2);

  let hash = '';
  for (let i = 0; i < 7; i++) {
    hash += HASH_CHARS[Math.floor(Math.random() * HASH_CHARS.length)];
  }
  return `${ts}-${hash}`;
}

/** Returns true iff the string is a syntactically valid block ID. */
export function isValidBlockId(id: string): boolean {
  return BLOCK_ID_PATTERN.test(id);
}

/**
 * Extract the LAST block ID marker from a markdown string.
 * Returns null if no marker is present.
 *
 * The "last marker wins" rule lets earlier markers be stale or duplicated
 * without breaking parsing — only the final marker carries authority.
 */
export function parseBlockId(md: string): string | null {
  if (!md) return null;
  const matches = md.matchAll(BLOCK_ID_MARKER_RE_GLOBAL);
  const arr = Array.from(matches);
  if (arr.length === 0) return null;
  return arr[arr.length - 1][1];
}

/**
 * Set the block ID marker on a markdown string.
 * - If a marker exists, replace it in place.
 * - If no marker exists, append one on a new line (collapsing extra
 *   trailing newlines so we don't end up with blank lines before the marker).
 */
export function setBlockId(md: string, id: string): string {
  const marker = `<!-- bid: ${id} -->`;
  if (BLOCK_ID_MARKER_RE.test(md)) {
    return md.replace(BLOCK_ID_MARKER_RE, marker);
  }
  // Collapse any trailing newlines, then add exactly one before the marker.
  return md.replace(/\n+$/, '') + '\n' + marker;
}

/**
 * Strip the LAST block ID marker from a markdown string.
 * Earlier markers (if any) are left untouched.
 * A trailing newline at end of file is preserved.
 */
export function stripBlockId(md: string): string {
  if (!md) return md;
  const matches = md.matchAll(BLOCK_ID_MARKER_RE_GLOBAL);
  const arr = Array.from(matches);
  if (arr.length === 0) return md;
  const last = arr[arr.length - 1];
  const before = md.slice(0, last.index);
  const after = md.slice(last.index + last[0].length);
  // Trim newlines immediately preceding the marker so we don't leave a
  // dangling blank line where the marker used to be.
  return before.replace(/\n+$/, '') + after;
}