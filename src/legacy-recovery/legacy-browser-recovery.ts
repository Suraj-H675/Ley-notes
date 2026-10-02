const LEGACY_DATABASE_NAME = "ley-notes";
const ACTIVE_DATA_KIND_KEY = "active-data-kind";
const BROWSER_LOCAL_KIND = "browser-local";

const ACTIVE_STORES = {
  pages: "pages",
  assets: "assets",
  revisions: "revisions",
} as const;

const SNAPSHOT_STORES = {
  pages: "browserLocalPages",
  assets: "browserLocalAssets",
  revisions: "browserLocalRevisions",
} as const;

type LegacyRecord = Record<string, unknown>;

export type LegacyRecoverySource =
  | "active-browser-local"
  | "browser-local-snapshot"
  | "legacy-active-unknown";

export type LegacyRecoveryStatus =
  | "absent"
  | "empty"
  | "filesystem-cache-only"
  | "recoverable";

export interface LegacyRecoveryInspection {
  status: LegacyRecoveryStatus;
  databaseVersion?: number;
  activeDataKind?: string | null;
  source?: LegacyRecoverySource;
  pages: number;
  assets: number;
  revisions: number;
  assetBytes: number;
  ambiguousActiveBackup: boolean;
}

interface LegacyDataSet {
  source: LegacyRecoverySource;
  pages: LegacyRecord[];
  revisions: LegacyRecord[];
  assets: LegacyRecord[];
}

export interface LegacyArchiveAsset {
  metadata: LegacyRecord;
  bytes: number;
  blobMimeType: string;
  dataBase64: string;
}

export interface LegacyArchiveDataSet {
  source: LegacyRecoverySource;
  pages: LegacyRecord[];
  revisions: LegacyRecord[];
  assets: LegacyArchiveAsset[];
}

export interface LegacyBrowserArchive {
  schemaVersion: 1;
  kind: "ley-legacy-browser-local-archive";
  databaseName: typeof LEGACY_DATABASE_NAME;
  databaseVersion: number;
  exportedAt: string;
  activeDataKind: string | null;
  authority: "proven-browser-local" | "ambiguous";
  dataSets: LegacyArchiveDataSet[];
  notice: string;
}

export async function inspectLegacyBrowserData(
  factory: IDBFactory = indexedDB,
): Promise<LegacyRecoveryInspection> {
  const database = await openExistingLegacyDatabase(factory);
  if (!database) {
    return {
      status: "absent",
      pages: 0,
      assets: 0,
      revisions: 0,
      assetBytes: 0,
      ambiguousActiveBackup: false,
    };
  }

  try {
    const snapshot = await readLegacyState(database);
    const plan = recoveryPlan(snapshot);
    const selected = plan.dataSets[0];
    return {
      status: plan.status,
      databaseVersion: database.version,
      activeDataKind: snapshot.activeDataKind,
      source: selected?.source,
      pages: selected?.pages.length ?? 0,
      assets: selected?.assets.length ?? 0,
      revisions: selected?.revisions.length ?? 0,
      assetBytes: plan.dataSets.reduce(
        (sum, dataSet) => sum + assetByteLength(dataSet.assets),
        0,
      ),
      ambiguousActiveBackup: plan.dataSets.length > 1,
    };
  } finally {
    database.close();
  }
}

export async function buildLegacyBrowserArchive(
  factory: IDBFactory = indexedDB,
): Promise<LegacyBrowserArchive> {
  const database = await openExistingLegacyDatabase(factory);
  if (!database) {
    throw new Error("No legacy Ley browser database exists on this origin.");
  }

  try {
    const snapshot = await readLegacyState(database);
    const plan = recoveryPlan(snapshot);
    if (plan.status !== "recoverable" || plan.dataSets.length === 0) {
      throw new Error(
        plan.status === "filesystem-cache-only"
          ? "Only a retired filesystem projection is present; there is no distinct browser-local vault to export."
          : "The legacy Ley browser database contains no recoverable browser-local vault data.",
      );
    }

    return {
      schemaVersion: 1,
      kind: "ley-legacy-browser-local-archive",
      databaseName: LEGACY_DATABASE_NAME,
      databaseVersion: database.version,
      exportedAt: new Date().toISOString(),
      activeDataKind: snapshot.activeDataKind,
      authority: plan.authority,
      dataSets: await serializeDataSets(plan.dataSets),
      notice:
        "This archive preserves retired human-authored browser notebook data. It is not Ley continuity and must not be treated as trusted agent memory.",
    };
  } finally {
    database.close();
  }
}

export function downloadLegacyBrowserArchive(archive: LegacyBrowserArchive): string {
  const stamp = archive.exportedAt.replace(/[:.]/g, "-");
  const filename = `ley-legacy-browser-data-${stamp}.json`;
  const blob = new Blob([JSON.stringify(archive, null, 2)], {
    type: "application/json",
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.rel = "noopener";
  anchor.click();
  queueMicrotask(() => URL.revokeObjectURL(url));
  return filename;
}

export async function eraseLegacyBrowserData(
  onBlocked?: () => void,
  factory: IDBFactory = indexedDB,
): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const request = factory.deleteDatabase(LEGACY_DATABASE_NAME);
    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error ?? new Error("IndexedDB deletion failed."));
    request.onblocked = () => onBlocked?.();
  });
}

interface RecoveryPlan {
  status: LegacyRecoveryStatus;
  authority: "proven-browser-local" | "ambiguous";
  dataSets: LegacyDataSet[];
}

interface LegacyState {
  activeDataKind: string | null;
  active: Omit<LegacyDataSet, "source">;
  snapshot: Omit<LegacyDataSet, "source">;
}

function recoveryPlan(state: LegacyState): RecoveryPlan {
  const hasActive = hasData(state.active);
  const hasSnapshot = hasData(state.snapshot);
  const active = (source: LegacyRecoverySource): LegacyDataSet => ({
    source,
    ...state.active,
  });
  const snapshot = (): LegacyDataSet => ({
    source: "browser-local-snapshot",
    ...state.snapshot,
  });

  if (state.activeDataKind === BROWSER_LOCAL_KIND && hasActive) {
    return {
      status: "recoverable",
      authority: "proven-browser-local",
      dataSets: [active("active-browser-local")],
    };
  }

  if (state.activeDataKind === BROWSER_LOCAL_KIND && hasSnapshot) {
    return {
      status: "recoverable",
      authority: "ambiguous",
      dataSets: [snapshot()],
    };
  }

  if (state.activeDataKind?.startsWith("filesystem:")) {
    if (!hasSnapshot) {
      return {
        status: hasActive ? "filesystem-cache-only" : "empty",
        authority: "proven-browser-local",
        dataSets: [],
      };
    }
    return {
      status: "recoverable",
      authority: "proven-browser-local",
      dataSets: [snapshot()],
    };
  }

  if (state.activeDataKind !== null) {
    throw new Error(
      `Legacy Ley browser data uses an unrecognized active-data-kind marker: ${state.activeDataKind}`,
    );
  }

  if (hasSnapshot) {
    return {
      status: "recoverable",
      authority: hasActive ? "ambiguous" : "proven-browser-local",
      dataSets: hasActive
        ? [snapshot(), active("legacy-active-unknown")]
        : [snapshot()],
    };
  }

  if (hasActive) {
    return {
      status: "recoverable",
      authority: "ambiguous",
      dataSets: [active("legacy-active-unknown")],
    };
  }

  return {
    status: "empty",
    authority: "proven-browser-local",
    dataSets: [],
  };
}

async function serializeDataSets(dataSets: LegacyDataSet[]): Promise<LegacyArchiveDataSet[]> {
  const serialized: LegacyArchiveDataSet[] = [];
  for (const dataSet of dataSets) serialized.push(await serializeDataSet(dataSet));
  return serialized;
}

async function serializeDataSet(data: LegacyDataSet): Promise<LegacyArchiveDataSet> {
  for (const [index, page] of data.pages.entries()) {
    assertJsonSafe(page, `pages[${index}]`);
  }
  for (const [index, revision] of data.revisions.entries()) {
    assertJsonSafe(revision, `revisions[${index}]`);
  }
  const assets: LegacyArchiveAsset[] = [];
  for (const record of data.assets) {
    const { blob, ...metadata } = record;
    if (!isBlobLike(blob)) {
      throw new Error("Legacy asset record is missing its binary Blob payload.");
    }
    assertJsonSafe(metadata, `assets[${assets.length}].metadata`);
    assets.push({
      metadata,
      bytes: blob.size,
      blobMimeType: blob.type,
      dataBase64: bytesToBase64(new Uint8Array(await blob.arrayBuffer())),
    });
  }
  return {
    source: data.source,
    pages: data.pages,
    revisions: data.revisions,
    assets,
  };
}

function assertJsonSafe(value: unknown, path: string): void {
  if (value === null || typeof value === "string" || typeof value === "boolean") return;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error(`${path} contains a non-finite number.`);
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertJsonSafe(item, `${path}[${index}]`));
    return;
  }
  if (typeof value !== "object") {
    throw new Error(`${path} contains a value that cannot be represented losslessly in JSON.`);
  }
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) {
    throw new Error(`${path} contains a non-plain value that cannot be represented losslessly in JSON.`);
  }
  for (const [key, child] of Object.entries(value)) {
    assertJsonSafe(child, `${path}.${key}`);
  }
}

function isBlobLike(
  value: unknown,
): value is { size: number; type: string; arrayBuffer(): Promise<ArrayBuffer> } {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as {
    size?: unknown;
    type?: unknown;
    arrayBuffer?: unknown;
  };
  return (
    typeof candidate.size === "number" &&
    typeof candidate.type === "string" &&
    typeof candidate.arrayBuffer === "function"
  );
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + chunkSize));
  }
  return btoa(binary);
}

async function readLegacyState(database: IDBDatabase): Promise<LegacyState> {
  const knownStores = [
    "settings",
    ...Object.values(ACTIVE_STORES),
    ...Object.values(SNAPSHOT_STORES),
  ].filter((name) => database.objectStoreNames.contains(name));
  if (knownStores.length === 0) {
    return {
      activeDataKind: null,
      active: { pages: [], assets: [], revisions: [] },
      snapshot: { pages: [], assets: [], revisions: [] },
    };
  }

  const transaction = database.transaction(knownStores, "readonly");
  const readAll = (storeName: string): Promise<LegacyRecord[]> => {
    if (!database.objectStoreNames.contains(storeName)) return Promise.resolve([]);
    return requestResult(transaction.objectStore(storeName).getAll()).then((values) =>
      values.filter(isLegacyRecord),
    );
  };
  const marker = database.objectStoreNames.contains("settings")
    ? requestResult(transaction.objectStore("settings").get(ACTIVE_DATA_KIND_KEY))
    : Promise.resolve(undefined);
  const requests = {
    marker,
    activePages: readAll(ACTIVE_STORES.pages),
    activeAssets: readAll(ACTIVE_STORES.assets),
    activeRevisions: readAll(ACTIVE_STORES.revisions),
    snapshotPages: readAll(SNAPSHOT_STORES.pages),
    snapshotAssets: readAll(SNAPSHOT_STORES.assets),
    snapshotRevisions: readAll(SNAPSHOT_STORES.revisions),
  };
  const [
    markerRecord,
    activePages,
    activeAssets,
    activeRevisions,
    snapshotPages,
    snapshotAssets,
    snapshotRevisions,
  ] = await Promise.all([
    requests.marker,
    requests.activePages,
    requests.activeAssets,
    requests.activeRevisions,
    requests.snapshotPages,
    requests.snapshotAssets,
    requests.snapshotRevisions,
  ]);

  let activeDataKind: string | null = null;
  if (markerRecord !== undefined) {
    if (!isLegacyRecord(markerRecord) || typeof markerRecord.value !== "string") {
      throw new Error("Legacy Ley browser data has a malformed active-data-kind marker.");
    }
    activeDataKind = markerRecord.value;
  }
  return {
    activeDataKind,
    active: { pages: activePages, assets: activeAssets, revisions: activeRevisions },
    snapshot: {
      pages: snapshotPages,
      assets: snapshotAssets,
      revisions: snapshotRevisions,
    },
  };
}

function isLegacyRecord(value: unknown): value is LegacyRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasData(dataSet: Omit<LegacyDataSet, "source">): boolean {
  return dataSet.pages.length + dataSet.assets.length + dataSet.revisions.length > 0;
}

function assetByteLength(records: LegacyRecord[]): number {
  return records.reduce((sum, record, index) => {
    const blob = record.blob;
    if (!isBlobLike(blob)) {
      throw new Error(`assets[${index}] is missing its binary Blob payload.`);
    }
    return sum + blob.size;
  }, 0);
}

function requestResult<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error("IndexedDB request failed."));
  });
}

async function openExistingLegacyDatabase(factory: IDBFactory): Promise<IDBDatabase | null> {
  const databases = factory.databases?.bind(factory);
  if (databases) {
    try {
      const known = await databases();
      if (!known.some((entry) => entry.name === LEGACY_DATABASE_NAME)) return null;
    } catch {
      // Some privacy modes deny enumeration while ordinary open still works.
      // Fall through to the non-creating open path below.
    }
  }

  return new Promise((resolve, reject) => {
    const request = factory.open(LEGACY_DATABASE_NAME);
    let abortedCreation = false;
    request.onupgradeneeded = (event) => {
      if (event.oldVersion !== 0) return;
      abortedCreation = true;
      request.transaction?.abort();
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => {
      if (abortedCreation && request.error?.name === "AbortError") {
        resolve(null);
        return;
      }
      reject(request.error ?? new Error("Could not open legacy IndexedDB data."));
    };
    request.onblocked = () => {
      reject(new Error("Legacy IndexedDB access is blocked by another open Ley tab."));
    };
  });
}
