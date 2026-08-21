/**
 * Filename helpers. The legacy `slugify` export remains for formats that
 * explicitly need URL-like names (for example Canvas). Markdown notes use
 * `filenameStem` so vaults retain the readable names their authors chose.
 * Pure functions, no I/O.
 */

const SLUG_RE = /[^a-z0-9]+/g;

export function slugify(title: string): string {
  return (
    title
      .toLowerCase()
      .normalize("NFKD")
      // Strip combining marks (diacritics) using Unicode property escapes.
      .replace(/\p{M}+/gu, "")
      .replace(SLUG_RE, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 80) || "untitled"
  );
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

const WINDOWS_RESERVED_BASENAME =
  /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?$/i;
const UNSAFE_FILENAME_CHARACTERS = /[<>:"/\\|?*]/g;
const CONTROL_OR_FORMAT_CHARACTERS = /\p{C}+/gu;
const MAX_FILENAME_STEM_BYTES = 180;

/** Whether a component is reserved on Windows, including an extension variant. */
export function isWindowsReservedFilename(value: string): boolean {
  return WINDOWS_RESERVED_BASENAME.test(value);
}

/**
 * Return a portable, human-readable Markdown filename stem.
 *
 * We deliberately retain spaces, case, and NFC Unicode. Only separators,
 * platform-reserved characters, controls, and trailing Windows-incompatible
 * dots/spaces are changed. This lets a title survive a filesystem refresh
 * without turning `Project Plan – 東京` into a URL slug.
 */
export function filenameStem(title: string): string {
  const cleaned = title
    .normalize("NFC")
    .replace(CONTROL_OR_FORMAT_CHARACTERS, " ")
    .replace(UNSAFE_FILENAME_CHARACTERS, " ")
    .replace(/\s+/gu, " ")
    .trim()
    .replace(/[. ]+$/g, "");

  const portable = isWindowsReservedFilename(cleaned)
    ? `${cleaned} note`
    : cleaned;
  const limited = limitUtf8Bytes(portable, MAX_FILENAME_STEM_BYTES).replace(
    /[. ]+$/g,
    "",
  );

  return limited || "Untitled";
}

/**
 * Make a human-readable filename stem unique under portable, case-insensitive
 * filesystem comparison. Space-separated suffixes read naturally in a vault
 * and avoid overwriting a file when the vault later moves to macOS or Windows.
 */
export function uniqueFilenameStem(
  base: string,
  taken: Iterable<string>,
): string {
  const normalizedTaken = new Set(Array.from(taken, portableFilenameKey));
  const cleanBase = filenameStem(base);
  if (!normalizedTaken.has(portableFilenameKey(cleanBase))) return cleanBase;

  let number = 2;
  while (true) {
    const suffix = ` ${number}`;
    const candidate =
      `${limitUtf8Bytes(cleanBase, MAX_FILENAME_STEM_BYTES - utf8ByteLength(suffix))}${suffix}`.replace(
        /[. ]+$/g,
        "",
      ) || `Untitled${suffix}`;
    if (!normalizedTaken.has(portableFilenameKey(candidate))) return candidate;
    number += 1;
  }
}

/** Portable comparison key for a single path component, not a full path. */
export function portableFilenameKey(value: string): string {
  return value.normalize("NFC").toLowerCase();
}

function limitUtf8Bytes(value: string, maximumBytes: number): string {
  let bytes = 0;
  let result = "";
  for (const character of value) {
    const characterBytes = utf8ByteLength(character);
    if (bytes + characterBytes > maximumBytes) break;
    bytes += characterBytes;
    result += character;
  }
  return result;
}

function utf8ByteLength(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}
