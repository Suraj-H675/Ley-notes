import { afterEach, describe, expect, it } from "vitest";
import { Blob as NodeBlob } from "node:buffer";
import {
  buildLegacyBrowserArchive,
  eraseLegacyBrowserData,
  inspectLegacyBrowserData,
} from "./legacy-browser-recovery";

const DB_NAME = "ley-notes";

afterEach(async () => {
  await deleteDatabase(DB_NAME);
});

describe("legacy browser recovery", () => {
  it("does not create a missing legacy database during inspection", async () => {
    expect(await databaseExists(DB_NAME)).toBe(false);

    const inspection = await inspectLegacyBrowserData();

    expect(inspection.status).toBe("absent");
    expect(await databaseExists(DB_NAME)).toBe(false);
  });

  it("prefers current active tables when they are explicitly browser-local", async () => {
    const database = await createLegacyDatabase();
    await put(database, "settings", { key: "active-data-kind", value: "browser-local" });
    await put(database, "pages", {
      ...page("active", "Active note"),
      sourceHash: "sha256:active",
      frontmatterError: "preserved warning",
    });
    await put(database, "revisions", revision("active-revision", "active"));
    await put(
      database,
      "assets",
      asset("active-asset", "active", "active.bin", [1, 2, 3], "image/png"),
    );
    await put(database, "browserLocalPages", page("snapshot", "Older snapshot"));
    database.close();

    const inspection = await inspectLegacyBrowserData();
    expect(inspection).toMatchObject({
      status: "recoverable",
      source: "active-browser-local",
      pages: 1,
      assets: 1,
      revisions: 1,
      assetBytes: 3,
      ambiguousActiveBackup: false,
    });

    const archive = await buildLegacyBrowserArchive();
    expect(archive.authority).toBe("proven-browser-local");
    expect(archive.dataSets[0].source).toBe("active-browser-local");
    expect(archive.dataSets[0].pages[0]).toMatchObject({
      id: "active",
      title: "Active note",
      sourceHash: "sha256:active",
      frontmatterError: "preserved warning",
    });
    expect(archive.dataSets[0].assets[0]).toMatchObject({
      bytes: 3,
      blobMimeType: "application/octet-stream",
      dataBase64: "AQID",
      metadata: { mimeType: "image/png" },
    });
    expect(archive.dataSets).toHaveLength(1);
  });

  it("uses the stashed browser-local snapshot instead of an active filesystem cache", async () => {
    const database = await createLegacyDatabase();
    await put(database, "settings", {
      key: "active-data-kind",
      value: "filesystem:/project/example",
    });
    await put(database, "pages", page("filesystem", "Filesystem projection"));
    await put(database, "browserLocalPages", page("browser", "Browser note"));
    await put(
      database,
      "browserLocalAssets",
      asset("browser-asset", "browser", "attachments/photo.png", [4, 5]),
    );
    database.close();

    const archive = await buildLegacyBrowserArchive();
    expect(archive.authority).toBe("proven-browser-local");
    expect(archive.dataSets[0].source).toBe("browser-local-snapshot");
    expect(archive.dataSets[0].pages).toHaveLength(1);
    expect(archive.dataSets[0].pages[0]).toMatchObject({ id: "browser" });
    expect(archive.dataSets[0].pages).not.toContainEqual(
      expect.objectContaining({ id: "filesystem" }),
    );
    expect(archive.dataSets).toHaveLength(1);
  });

  it("preserves both copies when old state is ambiguous", async () => {
    const database = await createLegacyDatabase();
    await put(database, "pages", page("active-unknown", "Unknown active note"));
    await put(database, "browserLocalPages", page("snapshot", "Snapshot note"));
    database.close();

    const inspection = await inspectLegacyBrowserData();
    expect(inspection).toMatchObject({
      status: "recoverable",
      source: "browser-local-snapshot",
      ambiguousActiveBackup: true,
    });

    const archive = await buildLegacyBrowserArchive();
    expect(archive.authority).toBe("ambiguous");
    expect(archive.dataSets.map((dataSet) => dataSet.source)).toEqual([
      "browser-local-snapshot",
      "legacy-active-unknown",
    ]);
    expect(archive.dataSets[0].pages[0]).toMatchObject({ id: "snapshot" });
    expect(archive.dataSets[1].pages[0]).toMatchObject({ id: "active-unknown" });
  });

  it("does not mislabel an active filesystem projection when no browser-local snapshot exists", async () => {
    const database = await createLegacyDatabase();
    await put(database, "settings", {
      key: "active-data-kind",
      value: "filesystem:/project/example",
    });
    await put(database, "pages", page("filesystem", "Filesystem projection"));
    database.close();

    const inspection = await inspectLegacyBrowserData();
    expect(inspection.status).toBe("filesystem-cache-only");
    await expect(buildLegacyBrowserArchive()).rejects.toThrow(/filesystem projection/i);
  });

  it("recovers pre-isolation active data and erases only on explicit delete", async () => {
    const database = await createLegacyDatabase({ version: 1 });
    await put(database, "pages", page("legacy", "Legacy note"));
    database.close();

    const inspection = await inspectLegacyBrowserData();
    expect(inspection).toMatchObject({
      status: "recoverable",
      source: "legacy-active-unknown",
      pages: 1,
    });

    await eraseLegacyBrowserData();
    expect(await databaseExists(DB_NAME)).toBe(false);
  });

  it("fails closed on an unrecognized historical authority marker", async () => {
    const database = await createLegacyDatabase();
    await put(database, "settings", {
      key: "active-data-kind",
      value: "future-mode-that-this-recovery-does-not-understand",
    });
    await put(database, "pages", page("unknown", "Unknown authority"));
    database.close();

    await expect(inspectLegacyBrowserData()).rejects.toThrow(/unrecognized active-data-kind/i);
  });

  it("refuses JSON export when a historical record cannot be represented losslessly", async () => {
    const database = await createLegacyDatabase();
    await put(database, "settings", { key: "active-data-kind", value: "browser-local" });
    await put(database, "pages", {
      ...page("unsafe-json", "Unsafe JSON"),
      frontmatter: { exactInteger: 9_007_199_254_740_993n },
    });
    database.close();

    await expect(buildLegacyBrowserArchive()).rejects.toThrow(/losslessly in JSON/i);
  });

  it("keeps a confirmed erase pending when another tab blocks deletion", async () => {
    const database = await createLegacyDatabase();
    let blocked = false;
    const deletion = eraseLegacyBrowserData(() => {
      blocked = true;
    });

    await waitUntil(() => blocked);
    expect(await databaseExists(DB_NAME)).toBe(true);
    database.close();
    await deletion;
    expect(await databaseExists(DB_NAME)).toBe(false);
  });
});

function page(id: string, title: string) {
  return {
    id,
    title,
    lcTitle: title.toLowerCase(),
    path: `${id}.md`,
    content: `# ${title}\n`,
    frontmatter: {},
    aliases: [],
    createdAt: 1,
    updatedAt: 2,
    deletedAt: null,
  };
}

function revision(id: string, pageId: string) {
  return { id, pageId, content: "older body", createdAt: 1 };
}

function asset(
  id: string,
  pageId: string,
  filename: string,
  bytes: number[],
  storedMimeType = "application/octet-stream",
) {
  return {
    id,
    pageId,
    filename,
    mimeType: storedMimeType,
    blob: new NodeBlob([new Uint8Array(bytes)], { type: "application/octet-stream" }),
    createdAt: 1,
  };
}

async function createLegacyDatabase({ version = 2 }: { version?: number } = {}) {
  return new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, version);
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains("pages")) {
        database.createObjectStore("pages", { keyPath: "id" });
        database.createObjectStore("assets", { keyPath: "id" });
        database.createObjectStore("revisions", { keyPath: "id" });
        database.createObjectStore("settings", { keyPath: "key" });
      }
      if (version >= 2 && !database.objectStoreNames.contains("browserLocalPages")) {
        database.createObjectStore("browserLocalPages", { keyPath: "id" });
        database.createObjectStore("browserLocalAssets", { keyPath: "id" });
        database.createObjectStore("browserLocalRevisions", { keyPath: "id" });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function put(database: IDBDatabase, storeName: string, value: unknown) {
  await new Promise<void>((resolve, reject) => {
    const transaction = database.transaction(storeName, "readwrite");
    transaction.objectStore(storeName).put(value);
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error);
  });
}

async function databaseExists(name: string): Promise<boolean> {
  const databases = await indexedDB.databases();
  return databases.some((entry) => entry.name === name);
}

async function deleteDatabase(name: string) {
  await new Promise<void>((resolve, reject) => {
    const request = indexedDB.deleteDatabase(name);
    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
  });
}

async function waitUntil(predicate: () => boolean) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  throw new Error("condition was not reached");
}
