/**
 * Time helpers. Centralized so we can mock in tests and avoid `Date.now()` everywhere.
 */

export function now(): number {
  return Date.now();
}