/**
 * Thin wrapper over nanoid for stable import shape.
 * nanoid ships ESM/CJS dual — we use the default 21-char alphabet at length 12
 * for IDs that are short enough to be readable in dev tools but unique enough
 * to never collide in a single-user vault.
 */

import { customAlphabet } from 'nanoid';

// URL-safe alphabet minus visually-ambiguous chars (0/O, 1/l/I).
const ALPHABET = '23456789abcdefghijkmnpqrstuvwxyz';
const LENGTH = 12;

const generator = customAlphabet(ALPHABET, LENGTH);

export function nanoid(): string {
  return generator();
}

/**
 * SiYuan-style block ID: YYYYMMDD-xxxxxx where x is a nanoid-derived suffix.
 * Stable per content via a content-hashed variant — see core/parser/blocks.ts.
 */
export function blockId(): string {
  const d = new Date();
  const stamp =
    d.getUTCFullYear().toString() +
    String(d.getUTCMonth() + 1).padStart(2, '0') +
    String(d.getUTCDate()).padStart(2, '0');
  return `${stamp}-${generator()}`;
}