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

  it('filters real Markdown tasks by state, composes filters, excludes fenced examples, and returns task context', async () => {
    await db.pages.bulkPut([
      makePage({
        id: 'todo',
        title: 'Open work',
        content: 'Plan\n- [ ] Call Alice about launch\n```md\n- [ ] Fenced example\n```',
      }),
      makePage({
        id: 'done',
        title: 'Completed work',
        content: 'Ship log\n- [x] Ship release candidate\n- [ ] Follow up with Bob',
      }),
      makePage({
        id: 'plain',
        title: 'No tasks',
        content: '- Call Alice in prose, not a task',
      }),
    ]);

    const stop = startSearchIndex();
    try {
      await new Promise((resolve) => setTimeout(resolve, 20));
      expect(
        (await searchPages('task-todo:alice')).map(({ id, snippet }) => ({ id, snippet })),
      ).toEqual([
        { id: 'todo', snippet: 'To do · Call Alice about launch' },
      ]);
      expect(
        (await searchPages('task-done:ship')).map(({ id, snippet }) => ({ id, snippet })),
      ).toEqual([{ id: 'done', snippet: 'Done · Ship release candidate' }]);
      expect((await searchPages('task:call')).map(({ id }) => id).sort()).toEqual(['todo']);
      expect((await searchPages('task-todo:bob -title:open')).map(({ id }) => id)).toEqual(['done']);
    } finally {
      stop();
    }
  });
});
