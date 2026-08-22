/**
 * Vault CRUD on pages. All write paths go through this module so we can
 * guarantee that indexes (backlinks, tags) are kept in sync.
 *
 * Naming follows the Graphify _stat_index pattern: writes are atomic and
 * idempotent so a crash mid-save leaves no orphan rows.
 */

import { db } from "@/infrastructure/database/db";
import { nanoid } from "@/shared/lib/nanoid";
import {
  filenameStem,
  isWindowsReservedFilename,
  portableFilenameKey,
  uniqueFilenameStem,
} from "@/shared/lib/slug";
import { getAliases, parseFrontmatter } from "@/core/parser/frontmatter";
import {
  rebuildPageLinks,
  resolveGhostLinksForPage,
} from "@/core/index/backlink";
import { rebuildPageTags } from "@/core/index/tag-index";
import { now } from "@/shared/lib/time";
import type { Page } from "@/infrastructure/database/schema";
import { retargetWikiLinks } from "@/core/parser/wiki-links";
import { retargetInternalMarkdownLinks } from "@/core/parser/markdown-links";
import { serializeFrontmatter } from "@/core/parser/frontmatter";
import { removeDestinationBookmarksForPage } from "@/core/vault/bookmarks";
import { removePageBookmarkReference } from "@/core/vault/note-bookmarks";
import {
  hashVaultSource,
  readActiveVaultFile,
  restoreActiveVaultTrashFile,
  renameActiveVaultFile,
  trashActiveVaultFile,
  writeActiveVaultFile,
} from "@/infrastructure/vault/filesystem-vault";
export interface CreatePageInput {
  title: string;
  content?: string;
  folder?: string;
  aliases?: string[];
  frontmatter?: Record<string, unknown>;
}

const pageMutationQueues = new Map<string, Promise<unknown>>();

function queuePageMutation<T>(
  pageId: string,
  mutation: () => Promise<T>,
): Promise<T> {
  const previous = pageMutationQueues.get(pageId) ?? Promise.resolve();
  const next = previous.catch(() => undefined).then(mutation);
  pageMutationQueues.set(pageId, next);
  return next.finally(() => {
    if (pageMutationQueues.get(pageId) === next)
      pageMutationQueues.delete(pageId);
  });
}

export async function listPages(): Promise<Page[]> {
  const rows = await db.pages.toArray();
  return rows.filter((p) => p.deletedAt === null && !p.missingFromDisk);
}

/**
 * Find a page by its exact title (case-insensitive).
 */
export async function getPageByTitle(title: string): Promise<Page | null> {
  const lc = title.toLowerCase();
  return (
    (await db.pages
      .where("lcTitle")
      .equals(lc)
      .filter((page) => page.deletedAt === null && !page.missingFromDisk)
      .first()) ?? null
  );
}

export async function listDeletedPages(): Promise<Page[]> {
  return (await db.pages.where("deletedAt").above(0).toArray()).sort(
    (left, right) => (right.deletedAt ?? 0) - (left.deletedAt ?? 0),
  );
}

export async function getPageById(pageId: string): Promise<Page | null> {
  return (await db.pages.get(pageId)) ?? null;
}

/**
 * Create a new page. If a page with the same title already exists, returns it
 * (idempotent). Otherwise inserts and returns the new page.
 *
 * Title uniqueness is case-insensitive (Obsidian-compatible). The path keeps
 * the title's readable case, spaces, and Unicode in the requested folder.
 */
export async function createPage(input: CreatePageInput): Promise<Page> {
  const title = requirePortablePageTitle(input.title);
  const existing = await getPageByTitle(title);
  if (existing) return existing;

  const ts = now();
  const all = await db.pages.toArray();
  const folder = canonicalFolderCase(normalizeFolder(input.folder ?? ""), all);
  const path = pagePathForTitle(title, folder, all);

  const page: Page = {
    id: nanoid(),
    title,
    lcTitle: title.toLowerCase(),
    path,
    content: input.content ?? "",
    frontmatter: input.frontmatter ?? {},
    aliases: input.aliases ?? [],
    createdAt: ts,
    updatedAt: ts,
    deletedAt: null,
  };

  const source = serializeFrontmatter(page.frontmatter, page.content);
  await writeActiveVaultFile(path, source);
  page.sourceHash = await hashVaultSource(source);
  await db.pages.add(page);

  // Initial index build.
  await rebuildPageLinks(page.id, page.content);
  await rebuildPageTags(page.id, page.content, page.frontmatter);
  await resolveGhostLinksForPage(page);

  return page;
}

/**
 * Update the body and frontmatter of a page. Re-parses frontmatter and
 * aliases, rebuilds the backlink and tag indexes.
 */
export function updatePageContent(
  pageId: string,
  content: string,
): Promise<void> {
  return queuePageMutation(pageId, () =>
    performPageContentUpdate(pageId, content),
  );
}

async function performPageContentUpdate(
  pageId: string,
  content: string,
): Promise<void> {
  const page = await db.pages.get(pageId);
  if (!page) throw new Error(`updatePageContent: page ${pageId} not found`);
  if (page.missingFromDisk)
    throw new Error(
      "This file was deleted outside Ley. Restore it or discard this open tab before saving.",
    );

  const next = projectEditorContent(page, content);

  const ts = now();
  await checkpointPage(page);
  await writeActiveVaultFile(page.path, next.source);
  await db.pages.update(pageId, {
    content: next.content,
    frontmatter: next.frontmatter,
    aliases: next.aliases,
    frontmatterError: next.frontmatterError,
    missingFromDisk: undefined,
    sourceHash: await hashVaultSource(next.source),
    updatedAt: ts,
  });

  await rebuildPageLinks(pageId, next.content);
  await rebuildPageTags(pageId, next.content, next.frontmatter);
}

/** Update YAML properties without rewriting editor-visible Markdown. */
export function updatePageFrontmatter(
  pageId: string,
  frontmatter: Record<string, unknown>,
): Promise<void> {
  return queuePageMutation(pageId, () =>
    performPageFrontmatterUpdate(pageId, frontmatter),
  );
}

export function updatePageProperty(
  pageId: string,
  key: string,
  value: unknown,
): Promise<void> {
  return queuePageMutation(pageId, async () => {
    const page = await db.pages.get(pageId);
    if (!page) throw new Error(`updatePageProperty: page ${pageId} not found`);
    await performPageFrontmatterUpdate(pageId, {
      ...page.frontmatter,
      [key]: value,
    });
  });
}

export function removePageProperty(pageId: string, key: string): Promise<void> {
  return queuePageMutation(pageId, async () => {
    const page = await db.pages.get(pageId);
    if (!page) throw new Error(`removePageProperty: page ${pageId} not found`);
    const next = { ...page.frontmatter };
    delete next[key];
    await performPageFrontmatterUpdate(pageId, next);
  });
}

async function performPageFrontmatterUpdate(
  pageId: string,
  frontmatter: Record<string, unknown>,
): Promise<void> {
  const page = await db.pages.get(pageId);
  if (!page) throw new Error(`updatePageFrontmatter: page ${pageId} not found`);
  if (page.frontmatterError)
    throw new Error(
      "Fix the invalid frontmatter in Source mode before editing Properties.",
    );
  if (page.missingFromDisk)
    throw new Error(
      "This file was deleted outside Ley. Restore it or discard this open tab before editing Properties.",
    );
  const aliases = getAliases(frontmatter);
  await checkpointPage(page);
  const source = serializeFrontmatter(frontmatter, page.content);
  await writeActiveVaultFile(page.path, source);
  await db.pages.update(pageId, {
    frontmatter,
    aliases,
    sourceHash: await hashVaultSource(source),
    updatedAt: now(),
  });
  await rebuildPageTags(pageId, page.content, frontmatter);
  await resolveGhostLinksForPage({ ...page, frontmatter, aliases });
}

async function checkpointPage(page: Page): Promise<void> {
  const revisions = await db.revisions
    .where("pageId")
    .equals(page.id)
    .toArray();
  const latest = revisions.sort(
    (left, right) => right.createdAt - left.createdAt,
  )[0];
  const snapshot = sourceForPage(page);
  if (
    latest &&
    (now() - latest.createdAt < 5 * 60_000 || latest.content === snapshot)
  )
    return;
  await db.revisions.add({
    id: nanoid(),
    pageId: page.id,
    content: snapshot,
    createdAt: now(),
  });
}

interface EditorContentProjection {
  content: string;
  frontmatter: Record<string, unknown>;
  aliases: string[];
  frontmatterError?: string;
  source: string;
}

/**
 * Convert editor text back into the cache projection without ever serializing
 * malformed leading YAML. Valid notes retain the existing body-only editor
 * model; invalid notes expose and persist their complete raw source instead.
 */
function projectEditorContent(
  page: Page,
  content: string,
): EditorContentProjection {
  const beginsWithFrontmatter = /^---(?:\r?\n|$)/.test(content);
  if (!beginsWithFrontmatter && !page.frontmatterError) {
    return {
      content,
      frontmatter: page.frontmatter,
      aliases: getAliases(page.frontmatter),
      source: serializeFrontmatter(page.frontmatter, content),
    };
  }

  const parsed = parseFrontmatter(content);
  if (parsed.error) {
    return {
      content,
      frontmatter: {},
      aliases: [],
      frontmatterError: parsed.error,
      source: content,
    };
  }

  return {
    content: parsed.body,
    frontmatter: parsed.frontmatter,
    aliases: getAliases(parsed.frontmatter),
    source: serializeFrontmatter(parsed.frontmatter, parsed.body),
  };
}

function sourceForPage(page: Page): string {
  return page.frontmatterError
    ? page.content
    : serializeFrontmatter(page.frontmatter, page.content);
}

/**
 * Rename a page. Updates the title, human-readable path, and lcTitle index.
 * Backlinks aren't rebuilt — they store targetTitle (the title at link
 * resolution time) which remains valid because the title still resolves.
 */
export function renamePage(pageId: string, newTitle: string): Promise<Page> {
  return queuePageMutation(pageId, () => performRenamePage(pageId, newTitle));
}

async function performRenamePage(
  pageId: string,
  newTitle: string,
): Promise<Page> {
  const page = await db.pages.get(pageId);
  if (!page) throw new Error(`renamePage: page ${pageId} not found`);
  if (page.missingFromDisk)
    throw new Error(
      "This file was deleted outside Ley. Restore it or discard this open tab before renaming.",
    );

  const title = requirePortablePageTitle(newTitle);
  const collision = await getPageByTitle(title);
  if (collision && collision.id !== pageId) {
    throw new Error(`A page named "${newTitle}" already exists`);
  }

  const all = await db.pages.toArray();
  const folder = canonicalFolderCase(folderForPath(page.path), all, pageId);
  const path = pagePathForTitle(title, folder, all, pageId);

  const updatesTitleProperty =
    !page.frontmatterError &&
    Object.prototype.hasOwnProperty.call(page.frontmatter, "title");
  const frontmatter = updatesTitleProperty
    ? { ...page.frontmatter, title }
    : page.frontmatter;
  const updated: Partial<Page> = {
    title,
    lcTitle: title.toLowerCase(),
    path,
    updatedAt: now(),
  };
  if (updatesTitleProperty) updated.frontmatter = frontmatter;
  const inbound = await db.links.where("targetPageId").equals(pageId).toArray();
  const affectedSourceIds = [
    ...new Set(inbound.map((link) => link.sourcePageId)),
  ];

  // Move first so the renamed path is authoritative. If changing a title
  // property cannot be persisted afterwards, restore the prior source and
  // best-effort move it back rather than leaving stale YAML at the new path.
  const originalSource = sourceForPage(page);
  let renamed = false;
  try {
    if (path !== page.path) {
      await renameActiveVaultFile(page.path, path);
      renamed = true;
    }
    if (updatesTitleProperty) {
      const source = serializeFrontmatter(frontmatter, page.content);
      await writeActiveVaultFile(path, source);
      updated.sourceHash = await hashVaultSource(source);
    }
    await db.pages.update(pageId, updated);
  } catch (error) {
    if (updatesTitleProperty) {
      await writeActiveVaultFile(path, originalSource).catch(() => undefined);
    }
    if (renamed)
      await renameActiveVaultFile(path, page.path).catch(() => undefined);
    throw error;
  }

  await maintainLinksAfterPathChange(
    page.id,
    page.path,
    path,
    affectedSourceIds,
    page.title,
    title,
  );

  return { ...page, ...updated } as Page;
}

/** Restore an externally deleted, still-open filesystem note using its current editor buffer. */
export function restoreMissingPage(
  pageId: string,
  editorContent: string,
): Promise<Page> {
  return queuePageMutation(pageId, async () => {
    const page = await db.pages.get(pageId);
    if (!page?.missingFromDisk)
      throw new Error("This open note is no longer waiting for recovery.");
    const next = projectEditorContent(page, editorContent);
    const updatedAt = now();
    const sourceHash = await hashVaultSource(next.source);
    await checkpointPage(page);
    await writeActiveVaultFile(page.path, next.source);
    await db.pages.update(pageId, {
      content: next.content,
      frontmatter: next.frontmatter,
      aliases: next.aliases,
      frontmatterError: next.frontmatterError,
      missingFromDisk: undefined,
      sourceHash,
      updatedAt,
    });
    const restored = {
      ...page,
      content: next.content,
      frontmatter: next.frontmatter,
      aliases: next.aliases,
      frontmatterError: next.frontmatterError,
      missingFromDisk: undefined,
      sourceHash,
      updatedAt,
    };
    await rebuildPageLinks(pageId, next.content);
    await rebuildPageTags(pageId, next.content, next.frontmatter);
    await resolveGhostLinksForPage(restored);
    return restored;
  });
}

/** Close and discard the cache-only recovery projection for an externally deleted note. */
export function discardMissingPage(pageId: string): Promise<void> {
  return queuePageMutation(pageId, async () => {
    const page = await db.pages.get(pageId);
    if (!page?.missingFromDisk)
      throw new Error(
        "Only an externally deleted open note can be discarded here.",
      );
    await db.transaction(
      "rw",
      db.pages,
      db.links,
      db.tags,
      db.revisions,
      db.assets,
      async () => {
        await db.links
          .where("targetPageId")
          .equals(pageId)
          .modify({ targetPageId: null });
        await Promise.all([
          db.links.where("sourcePageId").equals(pageId).delete(),
          db.tags.where("pageId").equals(pageId).delete(),
          db.revisions.where("pageId").equals(pageId).delete(),
          db.assets.where("pageId").equals(pageId).delete(),
          db.pages.delete(pageId),
        ]);
      },
    );
    await Promise.all([
      removePageBookmarkReference(pageId),
      removeDestinationBookmarksForPage(pageId),
    ]);
  });
}

/** Move a note without changing its title or breaking wiki links. */
export function movePage(
  pageId: string,
  destinationFolder: string,
): Promise<Page> {
  return queuePageMutation(pageId, () =>
    performMovePage(pageId, destinationFolder),
  );
}

async function performMovePage(
  pageId: string,
  destinationFolder: string,
): Promise<Page> {
  const page = await db.pages.get(pageId);
  if (!page || page.deletedAt !== null)
    throw new Error(`movePage: page ${pageId} not found`);

  const all = await db.pages.toArray();
  const folder = canonicalFolderCase(
    normalizeFolder(destinationFolder),
    all,
    pageId,
  );
  const filename =
    page.path.split("/").at(-1) ?? `${filenameStem(page.title)}.md`;
  const path = folder ? `${folder}/${filename}` : filename;
  if (path === page.path) return page;

  const collision = all.find(
    (candidate) =>
      candidate.id !== pageId &&
      candidate.deletedAt === null &&
      portablePathKey(candidate.path) === portablePathKey(path),
  );
  if (collision) {
    throw new Error(
      `A note named "${filename.replace(/\.md$/i, "")}" already exists in that folder`,
    );
  }

  const inbound = await db.links.where("targetPageId").equals(pageId).toArray();
  const affectedSourceIds = [
    ...new Set([...inbound.map((link) => link.sourcePageId), pageId]),
  ];
  await renameActiveVaultFile(page.path, path);
  const updatedAt = now();
  await db.pages.update(pageId, { path, updatedAt });
  await maintainLinksAfterPathChange(
    page.id,
    page.path,
    path,
    affectedSourceIds,
  );
  return { ...page, path, updatedAt };
}

async function maintainLinksAfterPathChange(
  pageId: string,
  oldPath: string,
  newPath: string,
  sourceIds: string[],
  oldTitle?: string,
  newTitle?: string,
): Promise<void> {
  const pathChanges = new Map([[oldPath.toLowerCase(), newPath]]);
  for (const sourceId of new Set([...sourceIds, pageId])) {
    const source = await db.pages.get(sourceId);
    if (!source || source.deletedAt !== null) continue;
    const oldSourcePath = sourceId === pageId ? oldPath : source.path;
    let nextContent = retargetInternalMarkdownLinks(
      source.content,
      oldSourcePath,
      source.path,
      pathChanges,
    );
    if (oldTitle && newTitle)
      nextContent = retargetWikiLinks(nextContent, oldTitle, newTitle);
    if (nextContent !== source.content) {
      if (source.id === pageId)
        await performPageContentUpdate(source.id, nextContent);
      else await updatePageContent(source.id, nextContent);
    }
  }
}

/** Create an independent Markdown copy while avoiding duplicate aliases. */
export async function duplicatePage(pageId: string): Promise<Page> {
  const page = await db.pages.get(pageId);
  if (!page || page.deletedAt !== null)
    throw new Error(`duplicatePage: page ${pageId} not found`);

  let suffix = 1;
  let title = `${page.title} copy`;
  while (await getPageByTitle(title)) {
    suffix += 1;
    title = `${page.title} copy ${suffix}`;
  }
  const folder = page.path.includes("/")
    ? page.path.split("/").slice(0, -1).join("/")
    : undefined;
  const frontmatter = { ...page.frontmatter };
  delete frontmatter.alias;
  delete frontmatter.aliases;
  return createPage({ title, folder, content: page.content, frontmatter });
}

function normalizeFolder(value: string): string {
  const folder = value
    .trim()
    .replace(/\\/g, "/")
    .replace(/^\/+|\/+$/g, "");
  if (!folder) return "";
  const segments = folder.split("/");
  if (
    segments.some(
      (segment) =>
        !segment ||
        segment === "." ||
        segment === ".." ||
        /[<>:"\\|?*\p{C}]/u.test(segment) ||
        isWindowsReservedFilename(segment) ||
        /[. ]$/.test(segment),
    )
  ) {
    throw new Error("Choose a safe folder inside the vault");
  }
  return segments.join("/");
}

function requirePortablePageTitle(value: string): string {
  const title = value.normalize("NFC").trim();
  if (!title) throw new Error("A note name is required");
  if (filenameStem(title) !== title) {
    throw new Error("Note names cannot contain path separators, control characters, Windows-reserved names, or trailing dots/spaces.");
  }
  return title;
}

function pagePathForTitle(
  title: string,
  folder: string,
  pages: Page[],
  excludedPageId?: string,
): string {
  const taken = pages
    .filter(
      (page) =>
        page.id !== excludedPageId &&
        page.deletedAt === null &&
        portablePathKey(folderForPath(page.path)) === portablePathKey(folder),
    )
    .map((page) => page.path.split("/").at(-1)?.replace(/\.md$/i, "") ?? "");
  const stem = uniqueFilenameStem(filenameStem(title), taken);
  return folder ? `${folder}/${stem}.md` : `${stem}.md`;
}

/** Use an existing folder's spelling when it differs only by case or NFC form. */
function canonicalFolderCase(
  folder: string,
  pages: Page[],
  excludedPageId?: string,
): string {
  const key = portablePathKey(folder);
  const existing = pages.find(
    (page) =>
      page.id !== excludedPageId &&
      page.deletedAt === null &&
      portablePathKey(folderForPath(page.path)) === key,
  );
  return existing ? folderForPath(existing.path) : folder;
}

function folderForPath(path: string): string {
  const slash = path.lastIndexOf("/");
  return slash < 0 ? "" : path.slice(0, slash);
}

function portablePathKey(path: string): string {
  return path.split("/").filter(Boolean).map(portableFilenameKey).join("/");
}

/**
 * Soft-delete a page. Links and tags remain so other pages' backlinks
 * resolve gracefully until they're rebuilt on next save.
 */
export function deletePage(pageId: string): Promise<void> {
  return queuePageMutation(pageId, async () => {
    const page = await db.pages.get(pageId);
    if (!page) return;
    await trashActiveVaultFile(page.path);
    await db.pages.update(pageId, {
      deletedAt: now(),
      sourceHash: undefined,
      missingFromDisk: undefined,
    });
    await db.links.where("sourcePageId").equals(pageId).delete();
    await db.tags.where("pageId").equals(pageId).delete();
  });
}

/** Restore a browser-local soft-deleted note and rebuild all derived indexes. */
export async function restorePage(pageId: string): Promise<Page> {
  const page = await db.pages.get(pageId);
  if (!page || page.deletedAt === null)
    throw new Error("That deleted note no longer exists");
  const collision = await getPageByTitle(page.title);
  if (collision)
    throw new Error(`A current note named "${page.title}" already exists`);
  const pathCollision = await db.pages
    .where("path")
    .equals(page.path)
    .filter((candidate) => candidate.deletedAt === null)
    .first();
  if (pathCollision)
    throw new Error(`A current note already uses ${page.path}`);
  const updatedAt = now();
  await db.pages.update(pageId, { deletedAt: null, updatedAt });
  const restored = { ...page, deletedAt: null, updatedAt };
  await rebuildPageLinks(pageId, page.content);
  await rebuildPageTags(pageId, page.content, page.frontmatter);
  await resolveGhostLinksForPage(restored);
  return restored;
}

/**
 * Restore a filesystem `.trash` note to its original folder. If the original
 * path is occupied, create an independent sibling copy rather than overwrite
 * authoritative Markdown; retarget links only for the actual restored path.
 */
export async function restoreTrashedFilesystemPage(
  trashedPath: string,
): Promise<Page> {
  const allBefore = await db.pages.toArray();
  const missingProjection = allBefore.find(
    (page) =>
      page.path.toLowerCase() === trashedPath.toLowerCase() &&
      page.deletedAt === null &&
      Boolean(page.missingFromDisk),
  );
  const restoredPath = await restoreActiveVaultTrashFile(trashedPath);
  if (!restoredPath)
    throw new Error("This vault cannot restore trashed notes.");

  const all = await db.pages.toArray();
  const previousProjection = all.find(
    (page) =>
      page.path.toLowerCase() === trashedPath.toLowerCase() &&
      page.deletedAt !== null,
  );
  const source = await readActiveVaultFile(restoredPath);
  if (source === null)
    throw new Error("This vault cannot read the restored note.");
  const sourceHash = await hashVaultSource(source);
  const parsed = parseFrontmatter(source);
  const filenameTitle =
    restoredPath.split("/").at(-1)?.replace(/\.md$/i, "") ?? "Untitled";
  const title =
    typeof parsed.frontmatter.title === "string" &&
    parsed.frontmatter.title.trim()
      ? parsed.frontmatter.title.trim()
      : filenameTitle;

  const id =
    previousProjection?.id ?? missingProjection?.id ?? stableTrashPageId(trashedPath);
  if (
    !previousProjection &&
    !(missingProjection && missingProjection.id === id) &&
    (await db.pages.get(id))
  ) {
    throw new Error(
      "This trashed note cannot be restored because Ley already has its recovery record.",
    );
  }
  const titleCollision = await getPageByTitle(title);
  if (titleCollision && titleCollision.id !== id)
    throw new Error(`A current note named "${title}" already exists`);

  const updatedAt = now();
  const restored: Page = {
    ...(previousProjection ?? missingProjection ?? ({} as Page)),
    id,
    title,
    lcTitle: title.toLowerCase(),
    path: restoredPath,
    content: parsed.body,
    frontmatter: parsed.frontmatter,
    frontmatterError: parsed.error,
    aliases: getAliases(parsed.frontmatter),
    createdAt:
      previousProjection?.createdAt ??
      missingProjection?.createdAt ??
      Date.now(),
    updatedAt,
    deletedAt: null,
    sourceHash,
    missingFromDisk: undefined,
  };
  await db.pages.put(restored);
  await maintainLinksAfterPathChange(
    id,
    trashedPath,
    restoredPath,
    [id],
    previousProjection?.title,
    previousProjection && title !== previousProjection.title ? title : undefined,
  );
  await rebuildPageLinks(id, restored.content);
  await rebuildPageTags(id, restored.content, restored.frontmatter);
  await resolveGhostLinksForPage(restored);
  return restored;
}

function stableTrashPageId(trashedPath: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < trashedPath.length; index += 1) {
    hash ^= trashedPath.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return `file_${(hash >>> 0).toString(36)}_restored`;
}

/** Irreversibly remove a browser-local deleted note and its private history/assets. */
export async function permanentlyDeletePage(pageId: string): Promise<void> {
  const page = await db.pages.get(pageId);
  if (!page || page.deletedAt === null)
    throw new Error("Only notes in the recycle bin can be permanently deleted");
  await db.transaction(
    "rw",
    db.pages,
    db.links,
    db.tags,
    db.revisions,
    db.assets,
    async () => {
      await db.links
        .where("targetPageId")
        .equals(pageId)
        .modify({ targetPageId: null });
      await Promise.all([
        db.links.where("sourcePageId").equals(pageId).delete(),
        db.tags.where("pageId").equals(pageId).delete(),
        db.revisions.where("pageId").equals(pageId).delete(),
        db.assets.where("pageId").equals(pageId).delete(),
        db.pages.delete(pageId),
      ]);
    },
  );
}
