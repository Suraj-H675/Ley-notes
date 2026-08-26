import { describe, expect, it } from "vitest";
import { makePage } from "@/test/helpers";
import type { Tag } from "@/infrastructure/database/schema";
import {
  buildCollectionRows,
  defaultCollectionColumns,
  discoverCollectionProperties,
  propertyColumn,
  sortCollectionRows,
} from "./collection";

describe("collection projection", () => {
  const pages = [
    makePage({
      id: "a",
      title: "Ley roadmap",
      content: "Desktop polish\n- [ ] Verify desktop package",
      frontmatter: { status: "active", priority: 2, owner: "Suraj" },
    }),
    makePage({
      id: "b",
      title: "Reading list",
      content: "Local-first systems\n- [x] Read CRDT paper",
      frontmatter: { status: "queued", priority: 10 },
    }),
    {
      ...makePage({
        id: "c",
        title: "Archived plan",
        frontmatter: { status: "archived" },
      }),
      deletedAt: Date.now(),
    },
    {
      ...makePage({ id: "missing", title: "External deletion" }),
      missingFromDisk: true,
    },
  ];
  const tags: Tag[] = [
    { pageId: "a", tag: "project/ley", source: "inline" },
    { pageId: "b", tag: "research", source: "frontmatter" },
    { pageId: "c", tag: "project/ley", source: "inline" },
  ];

  it("projects live notes through text, nested tag, property, and exclusion filters", () => {
    expect(
      buildCollectionRows(
        pages,
        tags,
        "tag:project property:status=active desktop",
      ).map((row) => row.page.id),
    ).toEqual(["a"]);
    expect(
      buildCollectionRows(pages, tags, "-property:status=archived").map(
        (row) => row.page.id,
      ),
    ).toEqual(["a", "b"]);
    expect(
      buildCollectionRows(pages, tags, "task-todo:desktop").map(
        (row) => row.page.id,
      ),
    ).toEqual(["a"]);
    expect(
      buildCollectionRows(pages, tags, "task-done:crdt").map(
        (row) => row.page.id,
      ),
    ).toEqual(["b"]);
  });

  it("excludes externally deleted cache projections from collections", () => {
    expect(
      buildCollectionRows(pages, tags, "").map((row) => row.page.id),
    ).toEqual(["a", "b"]);
  });

  it("filters live collection rows by real task state and excludes fenced examples", () => {
    const taskPages = [
      makePage({
        id: "todo",
        title: "Open tasks",
        content: "- [ ] Call Alice\n```md\n- [ ] Hidden Alice example\n```",
      }),
      makePage({
        id: "done",
        title: "Done tasks",
        content: "- [x] Call Alice later",
      }),
    ];
    expect(
      buildCollectionRows(taskPages, [], "task-todo:alice").map((row) => row.page.id),
    ).toEqual(["todo"]);
    expect(
      buildCollectionRows(taskPages, [], "task-done:alice").map((row) => row.page.id),
    ).toEqual(["done"]);
    expect(
      buildCollectionRows(taskPages, [], "task-todo:bob").map((row) => row.page.id),
    ).toEqual([]);
  });

  it("discovers portable YAML columns by coverage", () => {
    const rows = buildCollectionRows(pages, tags, "");
    expect(discoverCollectionProperties(rows)).toEqual([
      { id: "property:priority", key: "priority", count: 2 },
      { id: "property:status", key: "status", count: 2 },
      { id: "property:owner", key: "owner", count: 1 },
    ]);
    expect(defaultCollectionColumns(rows)).toEqual([
      "tags",
      "property:priority",
      "property:status",
      "property:owner",
      "modified",
    ]);
  });

  it("sorts typed property values without lexicographic number errors", () => {
    const rows = buildCollectionRows(
      [...pages, makePage({ id: "d", title: "No priority" })],
      tags,
      "",
    );
    expect(
      sortCollectionRows(rows, {
        column: propertyColumn("priority"),
        direction: "asc",
      }).map((row) => row.page.id),
    ).toEqual(["a", "b", "d"]);
    expect(
      sortCollectionRows(rows, {
        column: propertyColumn("priority"),
        direction: "desc",
      }).map((row) => row.page.id),
    ).toEqual(["b", "a", "d"]);
    expect(
      sortCollectionRows(rows, { column: "title", direction: "desc" }).map(
        (row) => row.page.id,
      ),
    ).toEqual(["b", "d", "a"]);
  });
});
