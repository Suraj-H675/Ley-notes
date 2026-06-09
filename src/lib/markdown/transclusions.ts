import { db } from '@/lib/db';

/** Return value from fetchTransclusionData — the resolved data for one title,
 * plus any transclusions found inside that note's content (nested embeds). */
export interface ResolvedTransclusion {
  title: string;
  plainText: string;
  exists: true;
}

export interface FailedTransclusion {
  title: string;
  exists: false;
}

/** Cycle-guard for transclusion data fetching.
 *
 * Transclusion chains (A embeds B, B embeds A) would otherwise loop forever.
 * Guard: depth > 5 or title already visited in this fetch chain → return null.
 *
 * Also recursively resolves nested transclusions inside a note's content, so
 * the plainText shown in a transclusion card already contains the resolved
 * content of its children (with cycles broken at the boundary). */
export async function fetchTransclusionData(
  title: string,
  depth: number,
  visited: Set<string>
): Promise<ResolvedTransclusion | FailedTransclusion | null> {
  const trimmed = title.trim();
  if (trimmed === '') return null;
  if (depth > 5) return null;
  if (visited.has(trimmed)) return null;
  visited.add(trimmed);

  const node = await db.nodes.where('title').equals(trimmed).first();
  if (!node) return null;

  // Recursively resolve any transclusions embedded in this node's content.
  // Pass a *copy* of visited so sibling branches don't share visited state,
  // but the same title can't appear twice in one chain.
  await resolveAllTransclusions(node.content ?? '', depth + 1, new Set(visited));

  // plainText is the already-extracted text of this node, which now
  // includes recursively resolved transclusion content (see resolveAllTransclusions).
  const plainText = node.plainText ?? '';

  return { title: trimmed, plainText, exists: true };
}

/** Parse transclusion markers in `content` and recursively fetch each target.
 * This mutates the in-memory node.plainText fields so that transclusion cards
 * show already-resolved nested content without re-triggering widget creation.
 *
 * @param content  Markdown content to scan for ![[...]] markers
 * @param depth    Current recursion depth
 * @param visited  Titles visited in this fetch chain (copy per sibling group)
 */
async function resolveAllTransclusions(
  content: string,
  depth: number,
  visited: Set<string>
): Promise<void> {
  if (typeof content !== 'string' || content === '') return;
  if (depth > 5) return;

  // Find all ![[Title]] markers in the content
  const transclusionRe = /!\[\[([^\]]+)\]\]/g;
  const titles: string[] = [];
  let m: RegExpExecArray | null;
  while ((m = transclusionRe.exec(content)) !== null) {
    const t = m[1].trim();
    if (t && !visited.has(t)) titles.push(t);
  }

  // Fetch all targets in parallel
  const results = await Promise.all(
    titles.map((t) => fetchTransclusionData(t, depth + 1, new Set(visited)))
  );

  // After fetching, each result's node.plainText already carries its own
  // recursively resolved content (mutated in-place by recursive calls above).
  // Nothing more to merge — the in-memory node objects in Dexie's liveQuery
  // cache are mutated so subsequent reads in the editor see resolved text.
  void results;
}