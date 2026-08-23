import { beforeEach, describe, expect, it } from 'vitest';
import { db } from '@/infrastructure/database/db';
import { makePage, resetDb } from '@/test/helpers';
import { searchPages, startSearchIndex } from './search';

describe('search index projection', () => {
  beforeEach(async () => {
    await resetDb();
  });

  it('excludes deleted and externally missing notes while keeping current notes', async () => {
    await db.pages.bulkPut([
      makePage({ id: 'current', title: 'Current roadmap', content: 'offline sync plan' }),
      { ...makePage({ id: 'deleted', title: 'Deleted roadmap', content: 'offline sync plan' }), deletedAt: 1 },
      { ...makePage({ id: 'missing', title: 'Missing roadmap', content: 'offline sync plan' }), missingFromDisk: true },
    ]);

    const stop = startSearchIndex();
    try {
      const results = await searchPages('roadmap');
      expect(results.map((result) => result.id)).toEqual(['current']);
    } finally {
      stop();
    }
  });
});
