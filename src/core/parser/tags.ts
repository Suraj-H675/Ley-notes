/**
 * Inline-tag extractor. Handles #tag, #nested/tag, and #kebab-case-tag.
 *
 * Rules:
 *  - Must follow a non-word character (or start of line/string)
 *  - Must contain only lowercase letters, digits, hyphens, underscores, slashes
 *  - Length 1-100 characters
 *  - Skips inline code (`...`) and fenced code (```...```)
 *
 * Does NOT match `#` inside URLs or HTML attributes. The boundary excludes
 * URL slashes, assignments, quotes, and a preceding hash so headings,
 * URLs, and quoted attribute values stay out of the tag index.
 *
 * Returns a deduped set of full tag paths.
 */

const TAG_RE = /(?:^|[^\w`/#="'])#([a-z0-9_][a-z0-9_\-/]{0,99})/g;

export function extractInlineTags(source: string): string[] {
  const out = new Set<string>();
  let inFence = false;
  let fenceMarker = '';

  for (const line of source.split('\n')) {
    if (/^(```|~~~)/.test(line)) {
      const marker = line.match(/^(```|~~~)/)![1];
      if (!inFence) {
        inFence = true;
        fenceMarker = marker;
      } else if (marker === fenceMarker) {
        inFence = false;
        fenceMarker = '';
      }
    }

    if (!inFence) {
      const stripped = stripInlineCode(line);
      TAG_RE.lastIndex = 0;
      let m: RegExpExecArray | null;
      while ((m = TAG_RE.exec(stripped)) !== null) {
        const tag = m[1].replace(/\/+$/, ''); // strip trailing slashes
        if (tag) out.add(tag);
      }
    }
  }

  return [...out];
}

function stripInlineCode(line: string): string {
  let out = '';
  let i = 0;
  while (i < line.length) {
    if (line[i] === '`') {
      let run = 1;
      while (i + run < line.length && line[i + run] === '`') run++;
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
 * Split a nested tag path into segments, used for the tag pane tree.
 */
export function tagSegments(tag: string): string[] {
  return tag.split('/').filter(Boolean);
}
