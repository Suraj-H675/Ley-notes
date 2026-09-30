import { afterEach, describe, expect, it } from 'vitest';
import { db } from './db';

describe('legacy local-data compatibility schema', () => {
  afterEach(async () => {
    await db.delete();
    await db.open();
  });

  it('keeps retired browser-local stores readable without writing them', async () => {
    expect(db).toBeDefined();
    expect(db.browserLocalPages).toBeDefined();
    expect(db.browserLocalAssets).toBeDefined();
    expect(db.browserLocalRevisions).toBeDefined();
    expect(await db.browserLocalPages.count()).toBe(0);
    expect(await db.browserLocalAssets.count()).toBe(0);
    expect(await db.browserLocalRevisions.count()).toBe(0);
  });
});
