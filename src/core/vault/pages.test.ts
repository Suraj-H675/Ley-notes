import { describe, expect, it, beforeEach, vi } from "vitest";
import { restoreTrashedFilesystemPage } from "./pages";
import { db } from "@/infrastructure/database/db";
import {
  createPage,
  deletePage,
  duplicatePage,
  getPageByTitle,
  listDeletedPages,
  listPages,
  movePage,
  permanentlyDeletePage,
  renamePage,
  restorePage,
  restoreMissingPage,
  updatePageContent,
  updatePageFrontmatter,
  updatePageProperty,
} from "./pages";
import { resetDb } from "@/test/helpers";

vi.mock("@/infrastructure/vault/filesystem-vault", async (importOriginal) => ({
  ...(await importOriginal()),
  trashActiveVaultFile: vi.fn(async () => undefined),
  renameActiveVaultFile: vi.fn(async () => undefined),
  writeActiveVaultFile: vi.fn(async () => undefined),
  restoreActiveVaultTrashFile: vi.fn(async (trashedPath: string) =>
    trashedPath.replace(/^\.trash\//, ""),
  ),
  readActiveVaultFile: vi.fn(async (_path: string) => "See [[Target]].\n\n#restored"),
}));

describe("pages CRUD", () => {
  beforeEach(async () => {
    await resetDb();
  });

  it("creates a page with derived path", async () => {
    const p = await createPage({ title: "Hello World" });
    expect(p.path).toBe("Hello World.md");
    expect(p.lcTitle).toBe("hello world");
  });

  it("keeps meaningful Unicode, case, and spaces in new filenames", async () => {
    const p = await createPage({ title: "Project Plan – 東京" });
    expect(p.path).toBe("Project Plan – 東京.md");
  });

  it("rejects unsafe or non-portable note names instead of changing them silently", async () => {
    await expect(createPage({ title: "???" })).rejects.toThrow(/Note names/);
    await expect(createPage({ title: "Research: phase/one" })).rejects.toThrow(/Note names/);
    await expect(createPage({ title: "CON" })).rejects.toThrow(/Note names/);
  });

  it("returns the existing page on duplicate title (idempotent)", async () => {
    const a = await createPage({ title: "Foo" });
    const b = await createPage({ title: "Foo" });
    expect(a.id).toBe(b.id);
  });

  it("makes human-readable filenames unique with case-insensitive collision protection", async () => {
    const timestamp = Date.now();
    await db.pages.add({
      id: "external-filename-collision",
      title: "Imported note",
      lcTitle: "imported note",
      path: "Notes/existing.md",
      content: "",
      frontmatter: {},
      aliases: [],
      createdAt: timestamp,
      updatedAt: timestamp,
      deletedAt: null,
    });
    const p2 = await createPage({ title: "Existing", folder: "notes" });
    expect(p2.path).toBe("Notes/Existing 2.md");
  });

  it("places in folder when specified", async () => {
    const p = await createPage({ title: "Inbox", folder: "daily" });
    expect(p.path).toBe("daily/Inbox.md");
  });

  it("is idempotent on title collision", async () => {
    const a = await createPage({ title: "Foo" });
    const b = await createPage({ title: "foo" });
    expect(a.id).toBe(b.id);
  });

  it("updates content and rebuilds backlink index", async () => {
    const foo = await createPage({ title: "Foo" });
    const bar = await createPage({ title: "Bar", content: "initial" });
    await updatePageContent(bar.id, "see [[Foo]] for context");
    const links = await db.links.where("sourcePageId").equals(bar.id).toArray();
    expect(links).toHaveLength(1);
    expect(links[0].targetPageId).toBe(foo.id);
    expect(links[0].targetTitle).toBe("Foo");
  });

  it("keeps the newest content and derived indexes during rapid saves", async () => {
    const page = await createPage({ title: "Rapid capture" });
    await Promise.all([
      updatePageContent(page.id, "#project/ley"),
      updatePageContent(
        page.id,
        "#project/ley #project/research #status/active",
      ),
    ]);
    expect((await db.pages.get(page.id))?.content).toBe(
      "#project/ley #project/research #status/active",
    );
    expect(
      (await db.tags.where("pageId").equals(page.id).toArray())
        .map((row) => row.tag)
        .sort(),
    ).toEqual(["project/ley", "project/research", "status/active"]);
  });

  it("serializes content and property edits without losing either revision", async () => {
    const page = await createPage({ title: "Concurrent edits" });
    await Promise.all([
      updatePageContent(page.id, "New body #active"),
      updatePageFrontmatter(page.id, { status: "active", priority: 2 }),
    ]);
    expect(await db.pages.get(page.id)).toMatchObject({
      content: "New body #active",
      frontmatter: { status: "active", priority: 2 },
    });
    expect(await db.tags.get([page.id, "active"])).toBeTruthy();
  });

  it("merges rapid cell-level property edits against the newest frontmatter", async () => {
    const page = await createPage({ title: "Property table" });
    await Promise.all([
      updatePageProperty(page.id, "status", "active"),
      updatePageProperty(page.id, "priority", 2),
    ]);
    expect((await db.pages.get(page.id))?.frontmatter).toEqual({
      status: "active",
      priority: 2,
    });
  });

  it("indexes relative Markdown links by resolved vault path", async () => {
    const target = await createPage({ title: "Design", folder: "docs" });
    const source = await createPage({
      title: "Source",
      folder: "projects",
      content: "[Design](../docs/design.md#API)",
    });
    const link = await db.links.where("sourcePageId").equals(source.id).first();
    expect(link).toMatchObject({
      targetPageId: target.id,
      targetTitle: "Design",
      kind: "markdown",
    });
  });

  it("resolves a missing relative Markdown link when its file is later created at that path", async () => {
    const source = await createPage({
      title: "Source",
      folder: "projects",
      content: "[Future](../docs/future.md)",
    });
    expect(
      (await db.links.where("sourcePageId").equals(source.id).first())
        ?.targetPageId,
    ).toBeNull();
    const target = await createPage({ title: "Future", folder: "docs" });
    expect(
      (await db.links.where("sourcePageId").equals(source.id).first())
        ?.targetPageId,
    ).toBe(target.id);
  });

  it("extracts frontmatter aliases on save", async () => {
    const p = await createPage({ title: "Foo" });
    await updatePageContent(p.id, "---\naliases: [FooBar, The F]\n---\nbody");
    const reloaded = await db.pages.get(p.id);
    expect(reloaded?.aliases).toEqual(["FooBar", "The F"]);
  });

  it("renames and updates path", async () => {
    const p = await createPage({ title: "Old Name" });
    const renamed = await renamePage(p.id, "New Name");
    expect(renamed.title).toBe("New Name");
    expect(renamed.path).toBe("New Name.md");
  });

  it("rename rejects collision", async () => {
    await createPage({ title: "Foo" });
    const bar = await createPage({ title: "Bar" });
    await expect(renamePage(bar.id, "Foo")).rejects.toThrow(/already exists/);
  });

  it("moves a page between folders without changing its identity", async () => {
    const page = await createPage({ title: "Roadmap", folder: "projects/ley" });
    const moved = await movePage(page.id, "archive/2026");
    expect(moved.id).toBe(page.id);
    expect(moved.title).toBe("Roadmap");
    expect(moved.path).toBe("archive/2026/Roadmap.md");
    expect((await db.pages.get(page.id))?.path).toBe("archive/2026/Roadmap.md");
  });

  it("retargets incoming Markdown links when their target moves or is renamed", async () => {
    const target = await createPage({ title: "Design", folder: "docs" });
    const source = await createPage({
      title: "Source",
      folder: "projects",
      content: "[Design](../docs/design.md#API)",
    });
    await movePage(target.id, "archive");
    expect((await db.pages.get(source.id))?.content).toBe(
      "[Design](../archive/Design.md#API)",
    );
    await renamePage(target.id, "Design System");
    expect((await db.pages.get(source.id))?.content).toBe(
      "[Design](../archive/Design%20System.md#API)",
    );
  });

  it("rebases outgoing Markdown links when their source moves", async () => {
    await createPage({ title: "Design", folder: "docs" });
    const source = await createPage({
      title: "Source",
      folder: "projects",
      content: "[Design](../docs/design.md)",
    });
    await movePage(source.id, "projects/active");
    expect((await db.pages.get(source.id))?.content).toBe(
      "[Design](../../docs/design.md)",
    );
    expect(
      (await db.links.where("sourcePageId").equals(source.id).first())
        ?.targetTitle,
    ).toBe("Design");
  });

  it("moves a nested page back to the vault root", async () => {
    const page = await createPage({ title: "Inbox", folder: "capture" });
    expect((await movePage(page.id, "")).path).toBe("Inbox.md");
  });

  it("keeps an imported legacy filename when the note is moved", async () => {
    const timestamp = Date.now();
    await db.pages.add({
      id: "legacy-path",
      title: "Release notes",
      lcTitle: "release notes",
      path: "release-notes.md",
      content: "",
      frontmatter: {},
      aliases: [],
      createdAt: timestamp,
      updatedAt: timestamp,
      deletedAt: null,
    });

    expect((await movePage("legacy-path", "Archive")).path).toBe(
      "Archive/release-notes.md",
    );
  });

  it("rejects unsafe move destinations and path collisions", async () => {
    await createPage({ title: "One", folder: "target" });
    const another = await createPage({ title: "Two", folder: "source" });
    await expect(movePage(another.id, "../outside")).rejects.toThrow(
      /safe folder/,
    );
    await expect(movePage(another.id, "CON")).rejects.toThrow(/safe folder/);
    const timestamp = Date.now();
    await db.pages.add({
      id: "external-move-collision",
      title: "Imported collision",
      lcTitle: "imported collision",
      path: "target/two.md",
      content: "",
      frontmatter: {},
      aliases: [],
      createdAt: timestamp,
      updatedAt: timestamp,
      deletedAt: null,
    });
    await expect(movePage(another.id, "target")).rejects.toThrow(/already exists/);
  });

  it("duplicates content and properties but removes ambiguous aliases", async () => {
    const page = await createPage({
      title: "Project brief",
      folder: "projects",
      content: "Links to [[Welcome]].",
      frontmatter: { status: "active", aliases: ["Brief"] },
    });
    const copy = await duplicatePage(page.id);
    expect(copy.title).toBe("Project brief copy");
    expect(copy.path).toBe("projects/Project brief copy.md");
    expect(copy.content).toBe(page.content);
    expect(copy.frontmatter).toEqual({ status: "active" });
  });

  it("soft-deletes a page and removes its link/tag rows", async () => {
    const p = await createPage({
      title: "Foo",
      content: "see [[Bar]]",
      folder: "x",
    });
    await createPage({ title: "Bar" });
    await updatePageContent(p.id, "see [[Bar]] and #tag");
    await deletePage(p.id);
    const reloaded = await db.pages.get(p.id);
    expect(reloaded?.deletedAt).not.toBeNull();
    const links = await db.links.where("sourcePageId").equals(p.id).toArray();
    expect(links).toHaveLength(0);
    const tags = await db.tags.where("pageId").equals(p.id).toArray();
    expect(tags).toHaveLength(0);
  });

  it("clears stale filesystem continuity metadata when trashing a note", async () => {
    const page = await createPage({ title: "Continuity" });
    await updatePageContent(page.id, "Current disk content.");
    await db.pages.update(page.id, { missingFromDisk: true });

    await deletePage(page.id);

    const deleted = await db.pages.get(page.id);
    expect(deleted).toMatchObject({
      deletedAt: expect.any(Number),
    });
    expect(deleted?.missingFromDisk).toBeUndefined();
    expect(deleted?.sourceHash).toBeUndefined();
  });

  it("listPages excludes soft-deleted", async () => {
    const a = await createPage({ title: "Keep" });
    const b = await createPage({ title: "Drop" });
    await deletePage(b.id);
    const live = await listPages();
    expect(live.map((p) => p.id).sort()).toEqual([a.id].sort());
  });

  it("lists and restores browser-local deleted notes with rebuilt indexes", async () => {
    const target = await createPage({ title: "Target" });
    const page = await createPage({
      title: "Recover me",
      content: "See [[Target]] and #recovered.",
    });
    await deletePage(page.id);
    expect((await listDeletedPages()).map((candidate) => candidate.id)).toEqual(
      [page.id],
    );
    const restored = await restorePage(page.id);
    expect(restored.deletedAt).toBeNull();
    expect(
      (await db.links.where("sourcePageId").equals(page.id).first())
        ?.targetPageId,
    ).toBe(target.id);
    expect(await db.tags.get([page.id, "recovered"])).toBeTruthy();
  });

  it("allows a title to be recreated but prevents restoring over it", async () => {
    const deleted = await createPage({ title: "Reusable title" });
    await deletePage(deleted.id);
    const replacement = await createPage({ title: "Reusable title" });
    expect(replacement.id).not.toBe(deleted.id);
    await expect(restorePage(deleted.id)).rejects.toThrow(/current note/);
  });

  it("restores filesystem trash with its original identity and links", async () => {
    const target = await createPage({ title: "Target" });
    const page = await createPage({
      title: "Trashed note",
      folder: "projects",
      content: "See [[Target]].",
    });
    await updatePageContent(page.id, "See [[Target]].\n\n#restored");
    await deletePage(page.id);
    expect((await db.pages.get(page.id))?.deletedAt).not.toBeNull();

    const restored = await restoreTrashedFilesystemPage(
      "projects/Trashed note.md",
    );
    expect(restored.content).toBe("See [[Target]].\n\n#restored");
    expect(restored.frontmatter).toEqual({});

    expect(restored).toMatchObject({
      id: page.id,
      title: "Trashed note",
      path: "projects/Trashed note.md",
      deletedAt: null,
      sourceHash: expect.stringMatching(/^sha256:/),
    });
    expect((await db.pages.get(page.id))?.deletedAt).toBeNull();
    const links = await db.links.where("sourcePageId").equals(page.id).toArray();
    expect(links).toHaveLength(1);
    expect(
      (await db.links.where("sourcePageId").equals(page.id).first())
        ?.targetPageId,
    ).toBe(target.id);
    expect(await db.tags.get([page.id, "restored"])).toBeTruthy();
  });

  it("restores a missing recovery projection from filesystem trash", async () => {
    const readActiveVaultFile = vi.mocked(
      (await import("@/infrastructure/vault/filesystem-vault"))
        .readActiveVaultFile,
    );
    readActiveVaultFile.mockResolvedValueOnce(
      "Recovery buffer.\n\n #recovered",
    );
    await createPage({ title: "Target" });
    const page = await createPage({
      title: "Missing note",
      folder: "projects",
      content: "Recovery buffer.",
    });
    await db.pages.update(page.id, { missingFromDisk: true });

    const restored = await restoreTrashedFilesystemPage(
      "projects/Missing note.md",
    );

    expect(restored).toMatchObject({
      id: page.id,
      title: "Missing note",
      path: "projects/Missing note.md",
      deletedAt: null,
      missingFromDisk: undefined,
    });
    expect(await db.tags.get([page.id, "recovered"])).toBeTruthy();
  });

  it("restores an externally deleted open note with its editor buffer and continuity hash", async () => {
    const readActiveVaultFile = vi.mocked(
      (await import("@/infrastructure/vault/filesystem-vault"))
        .readActiveVaultFile,
    );
    readActiveVaultFile.mockResolvedValueOnce(null);
    await createPage({ title: "Target" });
    const page = await createPage({
      title: "Missing note",
      content: "Old disk body.",
    });
    await updatePageContent(page.id, "Recovery buffer.\n\n #recovered");
    const before = (await db.pages.get(page.id))?.sourceHash;
    await db.pages.update(page.id, { missingFromDisk: true });

    const restored = await restoreMissingPage(
      page.id,
      "---\ntitle: Missing note\n---\nRecovered buffer.\n\n #recovered",
    );

    expect(restored).toMatchObject({
      id: page.id,
      title: "Missing note",
      path: page.path,
      content: "Recovered buffer.\n\n #recovered",
      frontmatter: { title: "Missing note" },
      missingFromDisk: undefined,
    });
    expect(restored.sourceHash).toMatch(/^sha256:/);
    expect(restored.sourceHash).not.toBe(before);
    expect((await db.pages.get(page.id))?.missingFromDisk).toBeUndefined();
    expect(await db.tags.get([page.id, "recovered"])).toBeTruthy();
  });

  it("refuses to overwrite a missing note that reappeared externally before recovery", async () => {
    const readActiveVaultFile = vi.mocked(
      (await import("@/infrastructure/vault/filesystem-vault"))
        .readActiveVaultFile,
    );
    const page = await createPage({
      title: "Reappeared",
      content: "Old disk body.",
    });
    await db.pages.update(page.id, { missingFromDisk: true });
    readActiveVaultFile.mockResolvedValueOnce("External replacement.");

    await expect(
      restoreMissingPage(page.id, "Editor buffer."),
    ).rejects.toThrow(/reappeared on disk/);

    const unchanged = await db.pages.get(page.id);
    expect(unchanged).toMatchObject({
      content: "Old disk body.",
      missingFromDisk: true,
    });
  });

  it("permanently deletes only recycled notes and their private data", async () => {
    const page = await createPage({ title: "Disposable" });
    const source = await createPage({
      title: "Source",
      content: "Still references [[Disposable]].",
    });
    await db.revisions.add({
      id: "revision",
      pageId: page.id,
      content: "old",
      createdAt: Date.now(),
    });
    await deletePage(page.id);
    await permanentlyDeletePage(page.id);
    expect(await db.pages.get(page.id)).toBeUndefined();
    expect(await db.revisions.where("pageId").equals(page.id).count()).toBe(0);
    expect(
      (await db.links.where("sourcePageId").equals(source.id).first())
        ?.targetPageId,
    ).toBeNull();
  });

  it("getPageByTitle is case-insensitive", async () => {
    await createPage({ title: "CamelCase" });
    const got = await getPageByTitle("camelcase");
    expect(got).not.toBeNull();
    expect(got?.title).toBe("CamelCase");
  });
});
