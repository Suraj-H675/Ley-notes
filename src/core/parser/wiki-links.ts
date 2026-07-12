/**
 * Wiki-link parser. Extracts [[target]], [[target|alias]], [[target#heading]],
 * [[target#^block-id]], and ![[embeds]] from markdown.
 *
 * Pattern is ported from Graphify's extractors/markdown.py (177 LoC) but in
 * TypeScript and with the full Obsidian grammar. Key differences from the
 * Python original:
 *  - We scan a single string (no file I/O) — the caller passes the source.
 *  - We return positions so callers can decorate or replace in-place.
 *  - We skip fenced code blocks (```...```) and inline code (`...`).
 *  - We dedupe outgoing links per target (per Graphify's pattern).
 *
 * The regex is intentionally narrow on what it accepts inside [[ ]]:
 *  - target: any non-bracket, non-newline, non-pipe sequence
 *  - alias: starts with `|`, runs to `]]`
 *  - heading: starts with `#`, then slug-like chars
 *  - block: starts with `#^`, then alphanumeric
 *
 * We do NOT do URL-style escapes here. Obsidian's behavior: `[[foo bar]]` is
 * the title "foo bar" (spaces allowed). We follow that.
 */

export interface WikiLink {
  /** Raw match text from source, e.g. "[[Foo|alias]]". */
  raw: string;
  /** Page title as written, before alias/heading resolution. */
  target: string;
  /** Optional display alias ([[Foo|display as this]]). */
  alias: string | null;
  /** Optional heading anchor. */
  heading: string | null;
  /** Optional block reference id (after #^). */
  blockId: string | null;
  /** Char offset in the source string. */
  position: number;
  /** True if the source was `![[...]]`. */
  isEmbed: boolean;
}

// We do the negation differently: walk the source line by line, tracking
// whether we're inside a fenced code block. This lets us treat markdown
// links with brackets inside prose naturally and skip code blocks
// unambiguously.
const FENCE_RE = /^(```|~~~)/;
const WIKI_RE = /(!?)\[\[([^\]\n]+)\]\]/g;

export function extractWikiLinks(source: string): WikiLink[] {
  const results: WikiLink[] = [];
  const seen = new Set<string>(); // dedupe by raw+position
  let inFence = false;
  let fenceMarker = '';

  const lines = source.split('\n');
  let cursor = 0;

  for (const line of lines) {
    if (FENCE_RE.test(line)) {
      const marker = line.match(FENCE_RE)![1];
      if (!inFence) {
        inFence = true;
        fenceMarker = marker;
      } else if (marker === fenceMarker) {
        inFence = false;
        fenceMarker = '';
      }
    }

    if (!inFence) {
      // Skip inline code spans: replace `...` with spaces so the regex won't match.
      // Cheap approximation: any backtick-delimited span on this line is excluded.
      const stripped = stripInlineCode(line);
      WIKI_RE.lastIndex = 0;
      let m: RegExpExecArray | null;
      while ((m = WIKI_RE.exec(stripped)) !== null) {
        const key = `${cursor + m.index}::${m[0]}`;
        if (seen.has(key)) continue;
        seen.add(key);
        const rawInner = m[2];
        const pipeAt = rawInner.indexOf('|');
        const destination = (pipeAt >= 0 ? rawInner.slice(0, pipeAt) : rawInner).trim();
        const alias = pipeAt >= 0 ? rawInner.slice(pipeAt + 1).trim() || null : null;
        const hashAt = destination.indexOf('#');
        const target = (hashAt >= 0 ? destination.slice(0, hashAt) : destination).trim();
        const anchor = hashAt >= 0 ? destination.slice(hashAt + 1).trim() || null : null;
        if (!target) continue;
        let heading: string | null = null;
        let blockId: string | null = null;
        if (anchor) {
          if (anchor.startsWith('^')) {
            blockId = anchor.slice(1);
          } else {
            heading = anchor;
          }
        }
        results.push({
          raw: m[0],
          target,
          alias,
          heading,
          blockId,
          position: cursor + m.index,
          isEmbed: m[1] === '!',
        });
      }
    }

    cursor += line.length + 1; // +1 for the \n
  }

  return results;
}

/** Retarget links to a renamed page without touching code spans or fences. */
export function retargetWikiLinks(source: string, oldTitle: string, newTitle: string): string {
  const matches = extractWikiLinks(source)
    .filter((link) => link.target.localeCompare(oldTitle, undefined, { sensitivity: 'accent' }) === 0)
    .sort((left, right) => right.position - left.position);
  let output = source;
  for (const link of matches) {
    const anchor = link.blockId ? `#^${link.blockId}` : link.heading ? `#${link.heading}` : '';
    const alias = link.alias ? `|${link.alias}` : '';
    const replacement = `${link.isEmbed ? '!' : ''}[[${newTitle}${anchor}${alias}]]`;
    output = `${output.slice(0, link.position)}${replacement}${output.slice(link.position + link.raw.length)}`;
  }
  return output;
}

/**
 * Replace inline code spans with equal-length whitespace so positions stay
 * aligned but the regex won't match anything inside backticks.
 * Handles `...` and ```...``` spans on the same line (rare but valid).
 */
function stripInlineCode(line: string): string {
  let out = '';
  let i = 0;
  while (i < line.length) {
    if (line[i] === '`') {
      // Count run length of backticks.
      let run = 1;
      while (i + run < line.length && line[i + run] === '`') run++;
      // Find matching closing run of same length.
      const closeIdx = line.indexOf('`'.repeat(run), i + run);
      if (closeIdx === -1) {
        out += line.slice(i);
        break;
      }
      const spanLen = closeIdx - i + run;
      out += ' '.repeat(spanLen);
      i += spanLen;
    } else {
      out += line[i];
      i++;
    }
  }
  return out;
}

/**
 * Just the wiki-link targets (deduped, lowercased). Used for autocomplete.
 */
export function extractWikiLinkTargets(source: string): string[] {
  const set = new Set<string>();
  for (const l of extractWikiLinks(source)) {
    set.add(l.target.toLowerCase());
  }
  return [...set];
}

/**
 * Build the resolver used by autocomplete: given a partial typed query,
 * returns the link text to insert.
 */
export function completeWikiLink(
  partial: string,
  candidates: Array<{ title: string; aliases: string[] }>,
): Array<{ title: string; display: string }> {
  const q = partial.toLowerCase();
  const out: Array<{ title: string; display: string }> = [];
  for (const c of candidates) {
    const titles = [c.title, ...c.aliases];
    for (const t of titles) {
      if (t.toLowerCase().includes(q)) {
        out.push({ title: c.title, display: t });
        break;
      }
    }
  }
  return out.slice(0, 20);
}
