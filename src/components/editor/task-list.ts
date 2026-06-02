/**
 * Detect task-list lines in markdown.
 * A task line starts with optional whitespace, then "- [ ]" or "- [x]".
 */

export interface TaskLineMatch {
  /** Absolute position of the "-" that starts the list marker. */
  lineStart: number;
  /** Position of the "[" character. */
  checkboxFrom: number;
  /** Position just after the "]" character. */
  checkboxTo: number;
  /** Whether the task is checked. */
  checked: boolean;
}

const TASK_RE = /^(\s*)-\s+\[( |x)\]/;

export function findTaskLine(
  markdown: string,
  pos: number
): TaskLineMatch | null {
  // Find the start of the line containing pos.
  const lineStart = markdown.lastIndexOf('\n', pos - 1) + 1;
  const lineEnd = markdown.indexOf('\n', pos);
  const line = markdown.slice(lineStart, lineEnd === -1 ? undefined : lineEnd);

  const m = TASK_RE.exec(line);
  if (!m) return null;

  const leadingWs = m[1];
  const marker = m[2] === 'x';
  return {
    lineStart,
    checkboxFrom: lineStart + leadingWs.length + 2, // "-" + " "
    checkboxTo: lineStart + leadingWs.length + 5, // "-" + " " + "[x]"
    checked: marker,
  };
}
