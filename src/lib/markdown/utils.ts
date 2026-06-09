/** Escape special regex characters in a string so it can be safely used
 * inside `new RegExp(...)` or `String.prototype.replace(..., str)`. */
export function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}