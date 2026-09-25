import { db } from './db';

const ACTIVE_DATA_KIND = 'active-data-kind';

/** Returns the identity of the filesystem vault currently projected into Dexie. */
export async function activeDataKind(): Promise<string | null> {
  const value = (await db.settings.get(ACTIVE_DATA_KIND))?.value;
  return typeof value === 'string' ? value : null;
}

/** Records which filesystem vault owns the current disposable Dexie projection. */
export async function markActiveDataKind(kind: string): Promise<void> {
  await db.settings.put({ key: ACTIVE_DATA_KIND, value: kind });
}

export function filesystemDataKind(vaultKey: string): string {
  return `filesystem:${vaultKey}`;
}
