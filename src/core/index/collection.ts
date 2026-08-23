import type { Page, Tag } from "@/infrastructure/database/schema";
import {
  comparePropertyValues,
  formatPropertyValue,
} from "@/core/parser/property-values";
import { matchesSearchFilters, parseSearchQuery } from "./search";

export const COLLECTION_SYSTEM_COLUMNS = ["tags", "path", "modified"] as const;
export type CollectionSystemColumn = (typeof COLLECTION_SYSTEM_COLUMNS)[number];
export type CollectionColumn = CollectionSystemColumn | `property:${string}`;
export type CollectionSortDirection = "asc" | "desc";

export interface CollectionSort {
  column: "title" | CollectionColumn;
  direction: CollectionSortDirection;
}

export interface CollectionRow {
  page: Page;
  tags: string[];
}

export interface CollectionPropertyColumn {
  id: `property:${string}`;
  key: string;
  count: number;
}

export function buildCollectionRows(
  pages: Page[],
  tagRows: Tag[],
  query: string,
): CollectionRow[] {
  const tagsByPage = new Map<string, string[]>();
  for (const row of tagRows) {
    const values = tagsByPage.get(row.pageId) ?? [];
    if (!values.includes(row.tag)) values.push(row.tag);
    tagsByPage.set(row.pageId, values);
  }
  const filter = parseSearchQuery(query.trim());
  const terms = filter.terms.toLocaleLowerCase().split(/\s+/).filter(Boolean);

  return pages
    .filter((page) => page.deletedAt === null && !page.missingFromDisk)
    .map((page) => ({ page, tags: (tagsByPage.get(page.id) ?? []).sort() }))
    .filter(({ page, tags }) => {
      const properties = normalizeProperties(page.frontmatter);
      if (
        !matchesSearchFilters(
          {
            id: page.id,
            title: page.title,
            path: page.path,
            tags: tags.join(" "),
            content: page.content,
          },
          filter,
          properties,
        )
      )
        return false;
      if (terms.length === 0) return true;
      const haystack = [
        page.title,
        page.path,
        page.content,
        page.aliases.join(" "),
        tags.join(" "),
        ...Object.entries(page.frontmatter).flatMap(([key, value]) => [
          key,
          formatPropertyValue(value),
        ]),
      ]
        .join(" ")
        .toLocaleLowerCase();
      return terms.every((term) => haystack.includes(term));
    });
}

export function discoverCollectionProperties(
  rows: CollectionRow[],
): CollectionPropertyColumn[] {
  const counts = new Map<string, number>();
  for (const { page } of rows) {
    for (const key of Object.keys(page.frontmatter)) {
      if (
        !key.trim() ||
        key.toLocaleLowerCase() === "title" ||
        key.toLocaleLowerCase() === "tags"
      )
        continue;
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }
  return [...counts.entries()]
    .map(([key, count]) => ({ id: propertyColumn(key), key, count }))
    .sort(
      (left, right) =>
        right.count - left.count || left.key.localeCompare(right.key),
    );
}

export function defaultCollectionColumns(
  rows: CollectionRow[],
): CollectionColumn[] {
  const properties = discoverCollectionProperties(rows)
    .slice(0, 4)
    .map((column) => column.id);
  return ["tags", ...properties, "modified"];
}

export function sortCollectionRows(
  rows: CollectionRow[],
  sort: CollectionSort,
): CollectionRow[] {
  const direction = sort.direction === "asc" ? 1 : -1;
  return [...rows].sort((left, right) => {
    let compared: number;
    if (sort.column === "title")
      compared = left.page.title.localeCompare(right.page.title, undefined, {
        numeric: true,
        sensitivity: "base",
      });
    else if (sort.column === "tags")
      compared = left.tags
        .join(", ")
        .localeCompare(right.tags.join(", "), undefined, {
          sensitivity: "base",
        });
    else if (sort.column === "path")
      compared = left.page.path.localeCompare(right.page.path, undefined, {
        numeric: true,
        sensitivity: "base",
      });
    else if (sort.column === "modified")
      compared = left.page.updatedAt - right.page.updatedAt;
    else {
      const key = propertyKey(sort.column);
      const leftValue = left.page.frontmatter[key];
      const rightValue = right.page.frontmatter[key];
      const leftMissing =
        leftValue === null || leftValue === undefined || leftValue === "";
      const rightMissing =
        rightValue === null || rightValue === undefined || rightValue === "";
      if (leftMissing !== rightMissing) return leftMissing ? 1 : -1;
      compared = comparePropertyValues(leftValue, rightValue);
    }
    return (
      compared * direction || left.page.title.localeCompare(right.page.title)
    );
  });
}

export function propertyColumn(key: string): `property:${string}` {
  return `property:${key}`;
}

export function propertyKey(column: CollectionColumn): string {
  return column.startsWith("property:") ? column.slice("property:".length) : "";
}

export function isCollectionColumn(value: unknown): value is CollectionColumn {
  return (
    typeof value === "string" &&
    (COLLECTION_SYSTEM_COLUMNS.includes(value as CollectionSystemColumn) ||
      (value.startsWith("property:") && value.length > "property:".length))
  );
}

function normalizeProperties(
  frontmatter: Record<string, unknown>,
): Map<string, string[]> {
  const result = new Map<string, string[]>();
  for (const [rawKey, rawValue] of Object.entries(frontmatter)) {
    const key = rawKey.trim().toLocaleLowerCase();
    if (!key) continue;
    const values = flattenPropertyValue(rawValue);
    if (values.length > 0) result.set(key, values);
  }
  return result;
}

function flattenPropertyValue(value: unknown): string[] {
  if (value === null || value === undefined) return [];
  if (Array.isArray(value)) return value.flatMap(flattenPropertyValue);
  if (typeof value === "object") {
    return Object.entries(value as Record<string, unknown>).flatMap(
      ([key, nested]) => [
        key.toLocaleLowerCase(),
        ...flattenPropertyValue(nested),
      ],
    );
  }
  return [String(value).toLocaleLowerCase()];
}
