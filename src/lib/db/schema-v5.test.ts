// Tests for the Dexie schema additions in v5 (block-level tables).
// Uses fake-indexeddb so the test doesn't depend on a real browser IndexedDB.

// IMPORTANT: fake-indexeddb/auto must be imported BEFORE `db` is imported,
// otherwise Dexie will try to use a stub IndexedDB at module load time.
import 'fake-indexeddb/auto';

import { describe, it, expect, beforeAll } from 'vitest';
import { db } from './index';

describe('Dexie schema v5 — block-level tables', () => {
  beforeAll(async () => {
    // Open forces Dexie to run pending migrations and create v5 tables.
    await db.open();
  });

  it('declares the blocks, refs, and blockAttributes tables', () => {
    const tableNames = db.tables.map((t) => t.name);
    expect(tableNames).toContain('blocks');
    expect(tableNames).toContain('refs');
    expect(tableNames).toContain('blockAttributes');
  });

  describe('blocks table', () => {
    it('has id as primary key', () => {
      expect(db.blocks.schema.primKey.keyPath).toBe('id');
    });

    it('has the expected secondary indexes', () => {
      const indexNames = db.blocks.schema.indexes.map((i) => i.name);
      expect(indexNames).toContain('nodeId');
      expect(indexNames).toContain('type');
      expect(indexNames).toContain('updatedAt');
    });

    it('has the compound index [nodeId+order] for ordered page reads', () => {
      const compound = db.blocks.schema.indexes.find(
        (i) => i.name === '[nodeId+order]',
      );
      expect(compound).toBeDefined();
      expect(compound!.keyPath).toEqual(['nodeId', 'order']);
    });

    it('accepts a basic insert and round-trip read', async () => {
      await db.blocks.put({
        id: '20200812220555-lj3enxa',
        nodeId: 'node-1',
        order: 0,
        type: 'paragraph',
        markdown: 'Hello\n<!-- bid: 20200812220555-lj3enxa -->',
        textContent: 'Hello',
        createdAt: 1,
        updatedAt: 1,
      });
      const got = await db.blocks.get('20200812220555-lj3enxa');
      expect(got).toBeDefined();
      expect(got!.nodeId).toBe('node-1');
      expect(got!.type).toBe('paragraph');
      await db.blocks.delete('20200812220555-lj3enxa');
    });
  });

  describe('refs table', () => {
    it('has id as primary key', () => {
      expect(db.refs.schema.primKey.keyPath).toBe('id');
    });

    it('has the expected secondary indexes', () => {
      const indexNames = db.refs.schema.indexes.map((i) => i.name);
      expect(indexNames).toContain('sourceBlockId');
      expect(indexNames).toContain('targetNodeId');
      expect(indexNames).toContain('targetBlockId');
    });

    it('has the backlink compound index [targetNodeId+sourceBlockId]', () => {
      const compound = db.refs.schema.indexes.find(
        (i) => i.name === '[targetNodeId+sourceBlockId]',
      );
      expect(compound).toBeDefined();
      expect(compound!.keyPath).toEqual(['targetNodeId', 'sourceBlockId']);
    });

    it('has the outbound compound index [sourceBlockId+targetNodeId]', () => {
      const compound = db.refs.schema.indexes.find(
        (i) => i.name === '[sourceBlockId+targetNodeId]',
      );
      expect(compound).toBeDefined();
      expect(compound!.keyPath).toEqual(['sourceBlockId', 'targetNodeId']);
    });

    it('accepts a backlink insert and is queryable by targetNodeId', async () => {
      await db.refs.put({
        id: 'ref-1',
        sourceBlockId: 'block-A',
        targetNodeId: 'node-B',
        targetNodeTitle: 'Note B',
        targetBlockId: null,
        linkType: 'page-ref',
        context: 'See [[Note B]]',
        createdAt: 1,
      });
      const backlinks = await db.refs
        .where('targetNodeId')
        .equals('node-B')
        .toArray();
      expect(backlinks).toHaveLength(1);
      expect(backlinks[0].sourceBlockId).toBe('block-A');
      await db.refs.delete('ref-1');
    });
  });

  describe('blockAttributes table', () => {
    it('has id as primary key', () => {
      expect(db.blockAttributes.schema.primKey.keyPath).toBe('id');
    });

    it('has the expected secondary indexes', () => {
      const indexNames = db.blockAttributes.schema.indexes.map((i) => i.name);
      expect(indexNames).toContain('blockId');
      expect(indexNames).toContain('name');
      expect(indexNames).toContain('value');
    });

    it('has the [blockId+name] and [name+value] compound indexes', () => {
      const blockCompound = db.blockAttributes.schema.indexes.find(
        (i) => i.name === '[blockId+name]',
      );
      expect(blockCompound).toBeDefined();
      expect(blockCompound!.keyPath).toEqual(['blockId', 'name']);

      const attrCompound = db.blockAttributes.schema.indexes.find(
        (i) => i.name === '[name+value]',
      );
      expect(attrCompound).toBeDefined();
      expect(attrCompound!.keyPath).toEqual(['name', 'value']);
    });
  });
});