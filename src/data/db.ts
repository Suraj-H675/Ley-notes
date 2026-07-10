/**
 * Dexie singleton. One versioned schema; migrations append.
 *
 * Indexes are picked for the queries we actually run:
 *  - backlinks by source and target
 *  - file tree by path
 *  - tag autocomplete by tag (with secondary pageId for the tag pane)
 *  - search by title prefix (via lcTitle)
 *  - graph layout by pageId
 */

import Dexie, { type Table } from 'dexie';
import type { Asset, Block, Link, Page, Revision, Setting, Tag } from './schema';

export class LeyDB extends Dexie {
  pages!: Table<Page, string>;
  blocks!: Table<Block, string>;
  links!: Table<Link, string>;
  tags!: Table<Tag, [string, string]>; // composite key [pageId, tag]
  assets!: Table<Asset, string>;
  revisions!: Table<Revision, string>;
  settings!: Table<Setting, string>;

  constructor() {
    super('ley-notes');
    this.version(1).stores({
      pages: 'id, lcTitle, path, updatedAt, deletedAt',
      blocks: 'id, pageId, parentId, [pageId+order]',
      links: 'id, sourcePageId, targetPageId, targetTitle, [sourcePageId+targetPageId]',
      tags: '[pageId+tag], pageId, tag',
      assets: 'id, pageId',
      revisions: 'id, pageId, createdAt',
      settings: 'key',
    });
  }
}

export const db = new LeyDB();

// Surface open errors loudly — silent failures here lead to "my notes disappeared" reports.
db.open().catch((err) => {
  console.error('[db] Failed to open IndexedDB:', err);
});