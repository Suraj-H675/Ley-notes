/**
 * CodeMirror 6 autocomplete source for [[wikilink]] picker.
 *
 * Triggers when the user types `[[`. Offers matching node titles.
 * Selecting an option inserts `[[Title]]` at the cursor, with the
 * cursor placed between the brackets and the title.
 */

export interface WikilinkNodeOption {
  id: string;
  title: string;
}

export interface WikilinkCompletion {
  from: number;
  to: number;
  options: Array<{
    label: string;
    apply: string;
    detail?: string;
  }>;
  validFor?: { text: RegExp };
}

const TRIGGER = '[[';

export function wikilinkSource(
  context: {
    textBefore: string;
    textAfter: string;
    pos: number;
    explicit: boolean;
    state: { doc: { toString: () => string; length: number } };
  },
  nodes: WikilinkNodeOption[]
): WikilinkCompletion | null {
  const before = context.textBefore;
  const triggerIdx = before.lastIndexOf(TRIGGER);
  if (triggerIdx === -1) return null;

  // No newline allowed in the wikilink.
  const query = before.slice(triggerIdx + TRIGGER.length);
  if (query.includes('\n')) return null;

  const from = triggerIdx;
  const to = context.pos;
  const queryLower = query.toLowerCase();

  const options = nodes
    .filter((n) => n.title.toLowerCase().includes(queryLower))
    .slice(0, 20)
    .map((n) => ({
      label: n.title,
      apply: n.title,
      detail: 'page',
    }));

  return {
    from,
    to,
    options,
  };
}
